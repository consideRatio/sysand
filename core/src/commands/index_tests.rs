// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: © 2026 Sysand contributors <opensource@sensmetry.com>

use camino::{Utf8Path, Utf8PathBuf};
use camino_tempfile::Utf8TempDir;
use fluent_uri::Iri;
use indexmap::IndexMap;

use crate::{
    commands::index::{IndexCommandError, IndexManager},
    env::index::{VersionsJson as ReaderVersionsJson, iri_path_segments, validate_versions},
    model::{
        InterchangeProjectInfoRaw, InterchangeProjectMetadataRaw, InterchangeProjectUsageRaw,
        format_created_now,
    },
    project::{local_kpar::LocalKParProject, memory::InMemoryProject},
};

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn temp_index() -> Result<(Utf8TempDir, IndexManager, Utf8PathBuf), Box<dyn std::error::Error>> {
    let dir = camino_tempfile::Utf8TempDir::with_prefix("sysand_index_test_")?;
    let root = dir.path().to_path_buf();
    Ok((dir, IndexManager::new(root.clone()), root))
}

fn write_kpar(
    dir: &Utf8Path,
    name: &str,
    publisher: Option<&str>,
    version: &str,
    usage: Vec<InterchangeProjectUsageRaw>,
) -> Result<Utf8PathBuf, Box<dyn std::error::Error>> {
    let info = InterchangeProjectInfoRaw {
        name: name.to_string(),
        publisher: publisher.map(str::to_string),
        description: None,
        version: version.to_string(),
        license: Some("MIT".to_string()),
        maintainer: vec![],
        website: None,
        topic: vec![],
        usage,
    };
    let meta = InterchangeProjectMetadataRaw {
        index: IndexMap::default(),
        created: format_created_now(),
        metamodel: None,
        includes_derived: None,
        includes_implied: None,
        checksum: None,
    };
    let project = InMemoryProject::from_info_meta(info, meta);
    let path = dir.join(format!("{name}-{version}.kpar"));
    LocalKParProject::from_project(&project, &path, zip::CompressionMethod::Stored, None)?;
    Ok(path)
}

fn project_dir(root: &Utf8Path, iri: &str) -> Result<Utf8PathBuf, Box<dyn std::error::Error>> {
    let mut path = root.to_path_buf();
    for segment in iri_path_segments(iri)? {
        path.push(segment);
    }
    Ok(path)
}

fn read_json(path: &Utf8Path) -> serde_json::Value {
    serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap()
}

#[test]
fn init_writes_empty_index_json() -> TestResult {
    let (_dir, manager, root) = temp_index()?;

    manager.init()?;

    assert_eq!(
        read_json(&root.join("index.json")),
        serde_json::json!({ "projects": [] })
    );
    Ok(())
}

#[test]
fn add_infers_sysand_purl_and_writes_reader_compatible_versions_json() -> TestResult {
    let (_dir, manager, root) = temp_index()?;
    manager.init()?;
    let kpar_path = write_kpar(
        &root,
        "Project One",
        Some("Test Publisher"),
        "1.0.0",
        vec![],
    )?;

    let iri = manager.add(&kpar_path, None)?;

    assert_eq!(iri, "pkg:sysand/test-publisher/project-one");
    let project_dir = project_dir(&root, &iri)?;
    let versions_path = project_dir.join("versions.json");
    let reader_versions: ReaderVersionsJson =
        serde_json::from_slice(&std::fs::read(&versions_path)?)?;
    let parsed = validate_versions(
        &url::Url::parse("https://example.com/versions.json")?,
        reader_versions,
    )?;
    assert_eq!(parsed.len(), 1);
    assert_eq!(parsed[0].version.to_string(), "1.0.0");
    assert!(project_dir.join("1.0.0/project.kpar").is_file());
    assert!(project_dir.join("1.0.0/.project.json").is_file());
    assert!(project_dir.join("1.0.0/.meta.json").is_file());
    Ok(())
}

#[test]
fn add_accepts_explicit_arbitrary_iri() -> TestResult {
    let (_dir, manager, root) = temp_index()?;
    manager.init()?;
    let kpar_path = write_kpar(
        &root,
        "Project One",
        Some("Test Publisher"),
        "1.0.0",
        vec![],
    )?;
    let iri = Iri::parse("https://example.com/project.kpar".to_string()).unwrap();

    let added_iri = manager.add(&kpar_path, Some(iri))?;

    assert_eq!(added_iri, "https://example.com/project.kpar");
    assert!(
        project_dir(&root, &added_iri)?
            .join("versions.json")
            .is_file()
    );
    Ok(())
}

#[test]
fn add_rejects_duplicate_version_before_overwriting_release_files() -> TestResult {
    let (_dir, manager, root) = temp_index()?;
    manager.init()?;
    let kpar_path = write_kpar(
        &root,
        "Project One",
        Some("Test Publisher"),
        "1.0.0",
        vec![],
    )?;
    let iri = manager.add(&kpar_path, None)?;
    let release_kpar = project_dir(&root, &iri)?.join("1.0.0/project.kpar");
    let original = std::fs::read(&release_kpar)?;

    let err = manager.add(&kpar_path, None).unwrap_err();

    assert!(matches!(err, IndexCommandError::DuplicateVersion { .. }));
    assert_eq!(std::fs::read(release_kpar)?, original);
    Ok(())
}

#[test]
fn add_sorts_versions_by_semver_precedence() -> TestResult {
    let (_dir, manager, root) = temp_index()?;
    manager.init()?;
    let beta = write_kpar(
        &root,
        "Project One",
        Some("Test Publisher"),
        "10.0.0-beta.1",
        vec![],
    )?;
    let release = write_kpar(
        &root,
        "Project One",
        Some("Test Publisher"),
        "10.0.0",
        vec![],
    )?;

    let iri = manager.add(&beta, None)?;
    manager.add(&release, None)?;

    let versions = read_json(&project_dir(&root, &iri)?.join("versions.json"));
    assert_eq!(versions["versions"][0]["version"], "10.0.0");
    assert_eq!(versions["versions"][1]["version"], "10.0.0-beta.1");
    Ok(())
}

#[test]
fn yank_marks_version_and_keeps_release_files() -> TestResult {
    let (_dir, manager, root) = temp_index()?;
    manager.init()?;
    let kpar_path = write_kpar(
        &root,
        "Project One",
        Some("Test Publisher"),
        "1.0.0",
        vec![],
    )?;
    let iri = manager.add(&kpar_path, None)?;
    let iri = Iri::parse(iri).unwrap();

    manager.yank(&iri, "1.0.0")?;

    let project_dir = project_dir(&root, iri.as_str())?;
    let versions = read_json(&project_dir.join("versions.json"));
    assert_eq!(versions["versions"][0]["status"], "yanked");
    assert!(project_dir.join("1.0.0/project.kpar").is_file());
    assert!(project_dir.join("1.0.0/.project.json").is_file());
    assert!(project_dir.join("1.0.0/.meta.json").is_file());
    Ok(())
}

#[test]
fn remove_version_marks_removed_and_deletes_release_files() -> TestResult {
    let (_dir, manager, root) = temp_index()?;
    manager.init()?;
    let kpar_path = write_kpar(
        &root,
        "Project One",
        Some("Test Publisher"),
        "1.0.0",
        vec![],
    )?;
    let iri = manager.add(&kpar_path, None)?;
    let iri = Iri::parse(iri).unwrap();

    manager.remove_version(&iri, "1.0.0")?;

    let project_dir = project_dir(&root, iri.as_str())?;
    let versions = read_json(&project_dir.join("versions.json"));
    assert_eq!(versions["versions"][0]["status"], "removed");
    assert!(!project_dir.join("1.0.0/project.kpar").exists());
    assert!(!project_dir.join("1.0.0/.project.json").exists());
    assert!(!project_dir.join("1.0.0/.meta.json").exists());
    Ok(())
}

#[test]
fn remove_project_cascades_versions_and_marks_project_removed() -> TestResult {
    let (_dir, manager, root) = temp_index()?;
    manager.init()?;
    let kpar_1 = write_kpar(
        &root,
        "Project One",
        Some("Test Publisher"),
        "1.0.0",
        vec![],
    )?;
    let kpar_2 = write_kpar(
        &root,
        "Project One",
        Some("Test Publisher"),
        "2.0.0",
        vec![],
    )?;
    let iri = manager.add(&kpar_1, None)?;
    manager.add(&kpar_2, None)?;
    let parsed_iri = Iri::parse(iri.clone()).unwrap();

    manager.remove_project(&parsed_iri)?;

    let index = read_json(&root.join("index.json"));
    assert_eq!(index["projects"][0]["status"], "removed");
    let project_dir = project_dir(&root, &iri)?;
    let versions = read_json(&project_dir.join("versions.json"));
    assert_eq!(versions["versions"][0]["version"], "2.0.0");
    assert_eq!(versions["versions"][0]["status"], "removed");
    assert_eq!(versions["versions"][1]["status"], "removed");
    assert!(!project_dir.join("1.0.0/project.kpar").exists());
    assert!(!project_dir.join("2.0.0/project.kpar").exists());
    Ok(())
}
