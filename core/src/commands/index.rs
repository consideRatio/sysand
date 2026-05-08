// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: © 2026 Sysand contributors <opensource@sensmetry.com>

use std::io::ErrorKind;

use camino::{Utf8Path, Utf8PathBuf};
use fluent_uri::Iri;
use semver::Version;
use serde::{Deserialize, Serialize};
use sha2::Digest as _;
use thiserror::Error;

use crate::{
    env::index::{IndexEnvironmentError, iri_path_segments},
    model::{InterchangeProjectInfoRaw, InterchangeProjectUsageRaw},
    project::{
        CanonicalizationError, ProjectRead,
        local_kpar::{LocalKParError, LocalKParProject},
        utils::{FsIoError, wrapfs},
    },
    purl::{
        PKG_SYSAND_PREFIX, SysandPurlError, is_valid_unnormalized_name,
        is_valid_unnormalized_publisher, normalize_field, parse_sysand_purl,
    },
};

const INDEX_JSON: &str = "index.json";
const VERSIONS_JSON: &str = "versions.json";
const KPAR_FILE: &str = "project.kpar";
const PROJECT_JSON: &str = ".project.json";
const META_JSON: &str = ".meta.json";

#[derive(Debug, Clone)]
pub struct IndexManager {
    root: Utf8PathBuf,
}

impl IndexManager {
    pub fn new(root: Utf8PathBuf) -> Self {
        Self { root }
    }

    pub fn init(&self) -> Result<(), IndexCommandError> {
        let index_path = self.index_path();
        if index_path.exists() {
            return Err(IndexCommandError::AlreadyExists(index_path));
        }
        if !self.root.exists() {
            wrapfs::create_dir_all(&self.root)?;
        }
        write_json_atomic(&index_path, &IndexJson::default())?;
        Ok(())
    }

    pub fn add(
        &self,
        kpar_path: &Utf8Path,
        explicit_iri: Option<Iri<String>>,
    ) -> Result<String, IndexCommandError> {
        if !wrapfs::is_file(kpar_path)? {
            return Err(IndexCommandError::KparNotFound(kpar_path.to_path_buf()));
        }

        let kpar_project = LocalKParProject::new_guess_root(kpar_path).map_err(|source| {
            IndexCommandError::KparOpen {
                path: kpar_path.to_path_buf(),
                source,
            }
        })?;
        let (info, meta) =
            kpar_project
                .get_project()
                .map_err(|source| IndexCommandError::KparRead {
                    path: kpar_path.to_path_buf(),
                    source,
                })?;
        let info = info.ok_or(IndexCommandError::MissingInfo {
            path: kpar_path.to_path_buf(),
        })?;
        let meta = meta.ok_or(IndexCommandError::MissingMeta {
            path: kpar_path.to_path_buf(),
        })?;

        let iri = match explicit_iri {
            Some(iri) => {
                validate_explicit_iri(&iri)?;
                iri.to_string()
            }
            None => infer_sysand_purl(&info)?,
        };

        parse_version(&info.version)?;
        for usage in &info.usage {
            usage
                .validate()
                .map_err(|source| IndexCommandError::InvalidUsage {
                    resource: usage.resource.clone(),
                    source,
                })?;
        }

        let project_dir = self.project_dir(&iri)?;
        let versions_path = project_dir.join(VERSIONS_JSON);
        let mut versions = if versions_path.exists() {
            load_versions(&versions_path)?
        } else {
            VersionsJson::default()
        };
        if versions.versions.iter().any(|v| v.version == info.version) {
            return Err(IndexCommandError::DuplicateVersion {
                iri,
                version: info.version,
            });
        }

        let mut index = self.load_index()?;
        match index.project_mut(&iri) {
            Some(project) if project.status == ProjectStatus::Removed => {
                return Err(IndexCommandError::ProjectRemoved { iri });
            }
            Some(project) => project.status = ProjectStatus::Available,
            None => index.projects.push(IndexProject {
                iri: iri.clone(),
                status: ProjectStatus::Available,
            }),
        }

        wrapfs::create_dir_all(&project_dir)?;
        let version_dir = project_dir.join(&info.version);
        wrapfs::create_dir_all(&version_dir)?;

        let kpar_bytes = std::fs::read(kpar_path)
            .map_err(|source| IndexCommandError::ReadFile(kpar_path.to_path_buf(), source))?;
        let kpar_size = u64::try_from(kpar_bytes.len()).expect("usize fits into u64");
        let kpar_digest = format!("{:x}", sha2::Sha256::digest(&kpar_bytes));
        let project_digest = kpar_project
            .checksum_canonical_hex()
            .map_err(IndexCommandError::CanonicalProjectDigest)?
            .ok_or(IndexCommandError::MissingDigestInput {
                path: kpar_path.to_path_buf(),
            })?;

        wrapfs::write(version_dir.join(KPAR_FILE), kpar_bytes)?;
        write_json_atomic(&version_dir.join(PROJECT_JSON), &info)?;
        write_json_atomic(&version_dir.join(META_JSON), &meta)?;

        versions.versions.push(VersionEntry {
            version: info.version.clone(),
            usage: info.usage,
            project_digest: format!("sha256:{project_digest}"),
            kpar_size,
            kpar_digest: format!("sha256:{kpar_digest}"),
            status: VersionStatus::Available,
        });
        versions.sort_newest_first()?;
        write_json_atomic(&versions_path, &versions)?;
        write_json_atomic(&self.index_path(), &index)?;

        Ok(iri)
    }

    pub fn yank(&self, iri: &Iri<String>, version: &str) -> Result<(), IndexCommandError> {
        let iri = validated_iri_string(iri)?;
        let version = parse_version(version)?;
        let version_string = version.to_string();
        let versions_path = self.versions_path(&iri)?;
        let mut versions = load_versions(&versions_path)?;
        let entry =
            versions
                .version_mut(&version)
                .ok_or_else(|| IndexCommandError::VersionNotFound {
                    iri: iri.clone(),
                    version: version_string.clone(),
                })?;
        match entry.status {
            VersionStatus::Available => entry.status = VersionStatus::Yanked,
            VersionStatus::Yanked => {}
            VersionStatus::Removed => {
                return Err(IndexCommandError::VersionRemoved {
                    iri,
                    version: version_string,
                });
            }
        }
        write_json_atomic(&versions_path, &versions)?;
        Ok(())
    }

    pub fn remove_project(&self, iri: &Iri<String>) -> Result<(), IndexCommandError> {
        let iri = validated_iri_string(iri)?;
        let mut index = self.load_index()?;
        let project = index
            .project_mut(&iri)
            .ok_or_else(|| IndexCommandError::ProjectNotFound { iri: iri.clone() })?;
        project.status = ProjectStatus::Removed;

        let versions_path = self.versions_path(&iri)?;
        let mut versions = load_versions(&versions_path)?;
        for entry in &mut versions.versions {
            entry.status = VersionStatus::Removed;
            remove_release_files(&self.version_dir(&iri, &entry.version)?)?;
        }

        write_json_atomic(&versions_path, &versions)?;
        write_json_atomic(&self.index_path(), &index)?;
        Ok(())
    }

    pub fn remove_version(
        &self,
        iri: &Iri<String>,
        version: &str,
    ) -> Result<(), IndexCommandError> {
        let iri = validated_iri_string(iri)?;
        let version = parse_version(version)?;
        let version_string = version.to_string();
        let versions_path = self.versions_path(&iri)?;
        let mut versions = load_versions(&versions_path)?;
        let entry =
            versions
                .version_mut(&version)
                .ok_or_else(|| IndexCommandError::VersionNotFound {
                    iri: iri.clone(),
                    version: version_string.clone(),
                })?;
        entry.status = VersionStatus::Removed;
        remove_release_files(&self.version_dir(&iri, &version_string)?)?;
        write_json_atomic(&versions_path, &versions)?;
        Ok(())
    }

    fn load_index(&self) -> Result<IndexJson, IndexCommandError> {
        load_json(&self.index_path())
    }

    fn index_path(&self) -> Utf8PathBuf {
        self.root.join(INDEX_JSON)
    }

    fn project_dir(&self, iri: &str) -> Result<Utf8PathBuf, IndexCommandError> {
        let mut path = self.root.clone();
        for segment in iri_path_segments(iri)? {
            path.push(segment);
        }
        Ok(path)
    }

    fn versions_path(&self, iri: &str) -> Result<Utf8PathBuf, IndexCommandError> {
        Ok(self.project_dir(iri)?.join(VERSIONS_JSON))
    }

    fn version_dir(&self, iri: &str, version: &str) -> Result<Utf8PathBuf, IndexCommandError> {
        Ok(self.project_dir(iri)?.join(version))
    }
}

#[derive(Debug, Default, Deserialize, Serialize)]
struct IndexJson {
    projects: Vec<IndexProject>,
}

impl IndexJson {
    fn project_mut(&mut self, iri: &str) -> Option<&mut IndexProject> {
        self.projects.iter_mut().find(|project| project.iri == iri)
    }
}

#[derive(Debug, Deserialize, Serialize)]
struct IndexProject {
    iri: String,
    #[serde(default, skip_serializing_if = "ProjectStatus::is_available")]
    status: ProjectStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
enum ProjectStatus {
    #[default]
    Available,
    Removed,
}

impl ProjectStatus {
    fn is_available(&self) -> bool {
        matches!(self, ProjectStatus::Available)
    }
}

#[derive(Debug, Default, Deserialize, Serialize)]
struct VersionsJson {
    versions: Vec<VersionEntry>,
}

impl VersionsJson {
    fn sort_newest_first(&mut self) -> Result<(), IndexCommandError> {
        for entry in &self.versions {
            parse_version(&entry.version)?;
        }
        self.versions.sort_by(|a, b| {
            parse_version(&b.version)
                .expect("versions parsed above")
                .cmp(&parse_version(&a.version).expect("versions parsed above"))
        });
        self.validate_order()
    }

    fn validate_order(&self) -> Result<(), IndexCommandError> {
        for [previous, current] in self.versions.as_slice().array_windows() {
            if parse_version(&previous.version)? <= parse_version(&current.version)? {
                return Err(IndexCommandError::VersionsOutOfOrder {
                    previous: previous.version.clone(),
                    current: current.version.clone(),
                });
            }
        }
        Ok(())
    }

    fn validate_existing(&self) -> Result<(), IndexCommandError> {
        let mut seen = std::collections::HashSet::new();
        for entry in &self.versions {
            parse_version(&entry.version)?;
            if !seen.insert(entry.version.clone()) {
                return Err(IndexCommandError::DuplicateExistingVersion {
                    version: entry.version.clone(),
                });
            }
        }
        self.validate_order()
    }

    fn version_mut(&mut self, version: &Version) -> Option<&mut VersionEntry> {
        self.versions
            .iter_mut()
            .find(|entry| parse_version(&entry.version).is_ok_and(|v| v == *version))
    }
}

#[derive(Debug, Deserialize, Serialize)]
struct VersionEntry {
    version: String,
    usage: Vec<InterchangeProjectUsageRaw>,
    project_digest: String,
    kpar_size: u64,
    kpar_digest: String,
    #[serde(default, skip_serializing_if = "VersionStatus::is_available")]
    status: VersionStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
enum VersionStatus {
    #[default]
    Available,
    Yanked,
    Removed,
}

impl VersionStatus {
    fn is_available(&self) -> bool {
        matches!(self, VersionStatus::Available)
    }
}

fn infer_sysand_purl(info: &InterchangeProjectInfoRaw) -> Result<String, IndexCommandError> {
    let publisher = info
        .publisher
        .as_deref()
        .ok_or(IndexCommandError::MissingPublisher)?;
    if !is_valid_unnormalized_publisher(publisher) {
        return Err(IndexCommandError::InvalidPublisher(publisher.to_string()));
    }
    if !is_valid_unnormalized_name(&info.name) {
        return Err(IndexCommandError::InvalidName(info.name.clone()));
    }
    Ok(format!(
        "{PKG_SYSAND_PREFIX}{}/{}",
        normalize_field(publisher),
        normalize_field(&info.name)
    ))
}

fn validate_explicit_iri(iri: &Iri<String>) -> Result<(), IndexCommandError> {
    parse_sysand_purl(iri.as_str())
        .map(|_| ())
        .map_err(IndexCommandError::MalformedSysandPurl)
}

fn validated_iri_string(iri: &Iri<String>) -> Result<String, IndexCommandError> {
    validate_explicit_iri(iri)?;
    Ok(iri.to_string())
}

fn parse_version(version: &str) -> Result<Version, IndexCommandError> {
    let parsed = Version::parse(version).map_err(|source| IndexCommandError::InvalidVersion {
        version: version.to_string(),
        source,
    })?;
    if !parsed.build.is_empty() {
        return Err(IndexCommandError::VersionBuildMetadata {
            version: version.to_string(),
        });
    }
    Ok(parsed)
}

fn load_json<T: serde::de::DeserializeOwned>(path: &Utf8Path) -> Result<T, IndexCommandError> {
    let bytes = std::fs::read(path)
        .map_err(|source| IndexCommandError::ReadFile(path.to_path_buf(), source))?;
    serde_json::from_slice(&bytes).map_err(|source| IndexCommandError::JsonRead {
        path: path.to_path_buf(),
        source,
    })
}

fn load_versions(path: &Utf8Path) -> Result<VersionsJson, IndexCommandError> {
    let versions = load_json::<VersionsJson>(path)?;
    versions.validate_existing()?;
    Ok(versions)
}

fn write_json_atomic<T: Serialize>(path: &Utf8Path, value: &T) -> Result<(), IndexCommandError> {
    let mut bytes = serde_json::to_vec_pretty(value).map_err(IndexCommandError::JsonWrite)?;
    bytes.push(b'\n');
    write_atomic(path, bytes)
}

fn write_atomic(path: &Utf8Path, contents: Vec<u8>) -> Result<(), IndexCommandError> {
    let parent = path
        .parent()
        .ok_or_else(|| IndexCommandError::MissingParent(path.to_path_buf()))?;
    wrapfs::create_dir_all(parent)?;
    let tmp_path = parent.join(format!(
        ".{}.{}.tmp",
        path.file_name().unwrap_or("index"),
        std::process::id()
    ));
    std::fs::write(&tmp_path, contents)
        .map_err(|source| IndexCommandError::WriteFile(tmp_path.clone(), source))?;
    std::fs::rename(&tmp_path, path).map_err(|source| {
        let _ = std::fs::remove_file(&tmp_path);
        IndexCommandError::RenameFile {
            from: tmp_path,
            to: path.to_path_buf(),
            source,
        }
    })
}

fn remove_release_files(version_dir: &Utf8Path) -> Result<(), IndexCommandError> {
    for filename in [KPAR_FILE, PROJECT_JSON, META_JSON] {
        let path = version_dir.join(filename);
        match std::fs::remove_file(&path) {
            Ok(()) => {}
            Err(source) if source.kind() == ErrorKind::NotFound => {}
            Err(source) => return Err(IndexCommandError::RemoveFile(path, source)),
        }
    }
    Ok(())
}

#[derive(Debug, Error)]
pub enum IndexCommandError {
    #[error("index file already exists at `{0}`")]
    AlreadyExists(Utf8PathBuf),
    #[error("KPAR file not found at `{0}`")]
    KparNotFound(Utf8PathBuf),
    #[error("failed to open KPAR file at `{path}`: {source}")]
    KparOpen {
        path: Utf8PathBuf,
        #[source]
        source: Box<FsIoError>,
    },
    #[error("failed to read KPAR project at `{path}`: {source}")]
    KparRead {
        path: Utf8PathBuf,
        #[source]
        source: LocalKParError,
    },
    #[error("missing project info in KPAR file at `{path}`")]
    MissingInfo { path: Utf8PathBuf },
    #[error("missing project metadata in KPAR file at `{path}`")]
    MissingMeta { path: Utf8PathBuf },
    #[error("missing publisher in project info, required when `--iri` is not provided")]
    MissingPublisher,
    #[error("publisher field `{0}` cannot be converted to a valid `pkg:sysand` publisher")]
    InvalidPublisher(String),
    #[error("name field `{0}` cannot be converted to a valid `pkg:sysand` name")]
    InvalidName(String),
    #[error("malformed `pkg:sysand` IRI: {0}")]
    MalformedSysandPurl(#[source] SysandPurlError),
    #[error("version `{version}` is invalid for an index: {source}")]
    InvalidVersion {
        version: String,
        #[source]
        source: semver::Error,
    },
    #[error("version `{version}` is invalid for an index: build metadata (`+...`) is forbidden")]
    VersionBuildMetadata { version: String },
    #[error("usage of `{resource}` is invalid for an index: {source}")]
    InvalidUsage {
        resource: String,
        #[source]
        source: crate::model::InterchangeProjectValidationError,
    },
    #[error("project `{iri}` has already been removed from this index")]
    ProjectRemoved { iri: String },
    #[error("project `{iri}` is not in this index")]
    ProjectNotFound { iri: String },
    #[error("version `{version}` of `{iri}` is not in this index")]
    VersionNotFound { iri: String, version: String },
    #[error("version `{version}` of `{iri}` has already been removed")]
    VersionRemoved { iri: String, version: String },
    #[error("version `{version}` of `{iri}` already exists in this index")]
    DuplicateVersion { iri: String, version: String },
    #[error("versions.json lists version `{version}` more than once")]
    DuplicateExistingVersion { version: String },
    #[error("versions.json is not in descending semver order: `{previous}` precedes `{current}`")]
    VersionsOutOfOrder { previous: String, current: String },
    #[error("failed to compute canonical project digest: {0}")]
    CanonicalProjectDigest(#[source] CanonicalizationError<LocalKParError>),
    #[error("missing digest input in KPAR file at `{path}`")]
    MissingDigestInput { path: Utf8PathBuf },
    #[error("failed to read file `{0}`: {1}")]
    ReadFile(Utf8PathBuf, std::io::Error),
    #[error("failed to write file `{0}`: {1}")]
    WriteFile(Utf8PathBuf, std::io::Error),
    #[error("failed to rename `{from}` to `{to}`: {source}")]
    RenameFile {
        from: Utf8PathBuf,
        to: Utf8PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to remove file `{0}`: {1}")]
    RemoveFile(Utf8PathBuf, std::io::Error),
    #[error("failed to parse JSON file `{path}`: {source}")]
    JsonRead {
        path: Utf8PathBuf,
        #[source]
        source: serde_json::Error,
    },
    #[error("failed to serialize index JSON: {0}")]
    JsonWrite(#[source] serde_json::Error),
    #[error("path `{0}` has no parent directory")]
    MissingParent(Utf8PathBuf),
    #[error(transparent)]
    IndexPath(#[from] IndexEnvironmentError),
    #[error(transparent)]
    Io(#[from] Box<FsIoError>),
}

#[cfg(test)]
#[path = "./index_tests.rs"]
mod tests;
