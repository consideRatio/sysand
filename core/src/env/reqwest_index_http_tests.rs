// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: © 2026 Sysand contributors <opensource@sensmetry.com>

use std::sync::Arc;

use crate::{
    auth::Unauthenticated,
    context::ProjectContext,
    env::{ReadEnvironment, ReadEnvironmentAsync},
    lock::Source,
    model::{InterchangeProjectInfoRaw, InterchangeProjectMetadataRaw, project_hash_raw},
    project::{
        ProjectRead, canonical_project_digest_inline, indexed_remote::IndexedRemoteProjectError,
        reqwest_kpar_download::ReqwestKparDownloadedError,
    },
    resolve::net_utils::create_reqwest_client,
};

/// Placeholder sha256 value acceptable by `parse_sha256_digest` — used in
/// tests that exercise flow but don't care about the specific digest bytes.
/// All-`a`s so it's visibly distinct from real-digest tests below.
const FILLER_DIGEST: &str =
    "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

/// Render a minimal-but-valid `versions.json` body for the given (version,
/// usage) pairs. The three required artifact fields are populated with
/// placeholder values; tests that need specific digest or size semantics
/// construct the body inline instead.
fn versions_json_body<const N: usize>(entries: [(&str, &str); N]) -> String {
    let parts: Vec<String> = entries
        .iter()
        .map(|(version, usage)| {
            format!(
                r#"{{"version":"{version}","usage":{usage},"project_digest":"{FILLER_DIGEST}","kpar_size":42,"kpar_digest":"{FILLER_DIGEST}"}}"#
            )
        })
        .collect();
    format!(r#"{{"versions":[{}]}}"#, parts.join(","))
}

fn versions_json_body_with_project_digest<const N: usize>(
    entries: [(&str, &str, &str); N],
) -> String {
    let parts: Vec<String> = entries
        .iter()
        .map(|(version, usage, project_digest)| {
            format!(
                r#"{{"version":"{version}","usage":{usage},"project_digest":"{project_digest}","kpar_size":42,"kpar_digest":"{FILLER_DIGEST}"}}"#
            )
        })
        .collect();
    format!(r#"{{"versions":[{}]}}"#, parts.join(","))
}

/// Render a minimal `.project.json` body for the given fields. `usage` is
/// inlined as a raw JSON fragment so callers can pass `"[]"` or
/// `r#"[{"resource":"..."}]"#` without escaping.
fn project_json_body(name: &str, publisher: Option<&str>, version: &str, usage: &str) -> String {
    match publisher {
        Some(p) => format!(
            r#"{{"name":"{name}","publisher":"{p}","version":"{version}","usage":{usage}}}"#
        ),
        None => format!(r#"{{"name":"{name}","version":"{version}","usage":{usage}}}"#),
    }
}

/// Render a minimal-but-valid `.meta.json` body. The fixed timestamp keeps
/// any test that hashes the body reproducible.
fn meta_json_body() -> &'static str {
    r#"{"index":{},"created":"2026-01-01T00:00:00.000000000Z"}"#
}

fn project_digest(info_json: &str, meta_json: &str) -> Result<String, Box<dyn std::error::Error>> {
    let info: InterchangeProjectInfoRaw = serde_json::from_str(info_json)?;
    let meta: InterchangeProjectMetadataRaw = serde_json::from_str(meta_json)?;
    Ok(format!("sha256:{:x}", project_hash_raw(&info, &meta)))
}

/// Compute the canonical project digest — matches what the server would
/// advertise in `versions.json`'s `project_digest`. Equivalent to
/// `project_digest` when `meta` has no checksum entries or only lowercase
/// SHA256 entries, but differs when entries require canonicalization
/// (mixed-case SHA256 hex values).
fn canonical_project_digest(
    info_json: &str,
    meta_json: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    let info: InterchangeProjectInfoRaw = serde_json::from_str(info_json)?;
    let meta: InterchangeProjectMetadataRaw = serde_json::from_str(meta_json)?;
    let hash = canonical_project_digest_inline(&info, &meta)
        .expect("canonical digest should be computable inline for this fixture");
    Ok(format!("sha256:{:x}", hash))
}

fn make_runtime() -> Result<Arc<tokio::runtime::Runtime>, Box<dyn std::error::Error>> {
    Ok(Arc::new(
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?,
    ))
}

/// Build an unauthenticated async index environment rooted at `base_url`.
/// Used by URL-construction tests that never touch a mock server.
fn test_env_async(
    base_url: &str,
) -> Result<super::HTTPIndexEnvironmentAsync<Unauthenticated>, Box<dyn std::error::Error>> {
    Ok(super::HTTPIndexEnvironmentAsync {
        client: create_reqwest_client()?,
        base_url: url::Url::parse(base_url)?,
        auth_policy: Arc::new(Unauthenticated {}),
        versions_cache: Default::default(),
    })
}

/// Build a sync-facing unauthenticated index environment rooted at `server`,
/// with a runtime owned by the test. This is the shape most tests want — a
/// mock `server` plus a single blocking handle to call `.get_project(...)` /
/// `.uris()` / `.versions(...)` against it.
fn test_env_sync(
    server: &mockito::Server,
) -> Result<
    crate::env::AsSyncEnvironmentTokio<super::HTTPIndexEnvironmentAsync<Unauthenticated>>,
    Box<dyn std::error::Error>,
> {
    Ok(test_env_async(&server.url())?.to_tokio_sync(make_runtime()?))
}

/// Register a mock for `{method} {path}` asserting it must never be called.
/// Body/status are immaterial — any match would fail `mock.assert()`.
fn expect_untouched(server: &mut mockito::Server, method: &str, path: &str) -> mockito::Mock {
    server.mock(method, path).expect(0).create()
}

#[test]
fn test_uri_examples() -> Result<(), Box<dyn std::error::Error>> {
    let env = test_env_async("https://www.example.com/index/")?;

    assert_eq!(
        env.index_url()?.to_string(),
        "https://www.example.com/index/index.json"
    );

    // pkg:sysand/<publisher>/<name> routes under publisher/name/
    assert_eq!(
        env.kpar_url("pkg:sysand/admin/proj0", "0.3.0")?.to_string(),
        "https://www.example.com/index/admin/proj0/0.3.0/project.kpar"
    );

    // Non-pkg:sysand IRIs go under _iri/<sha256(iri)>/
    assert_eq!(
        env.kpar_url("urn:kpar:b", "1.0.0")?.to_string(),
        "https://www.example.com/index/_iri/621a5fdf587a3ecc878a98c8be2240dd5bbe561860d11f4da1ece4a4fe2fb8b5/1.0.0/project.kpar"
    );

    // Four-segment pkg:sysand falls through to _iri (full sha256 pinned so a
    // regression in parse_pkg_sysand_iri returning e.g. Some(("a", "")) would
    // be caught rather than hidden behind a prefix match).
    assert_eq!(
        env.kpar_url("pkg:sysand/a/b/c", "1.0.0")?.to_string(),
        "https://www.example.com/index/_iri/\
         d3f517f8f0d4750ec01cd1eb2d80bfdb6b5a204c0e2101b478beda809fccc6f6/1.0.0/project.kpar"
    );

    // pkg:sysand with trailing empty segment also falls through.
    assert_eq!(
        env.kpar_url("pkg:sysand/a/", "1.0.0")?.to_string(),
        "https://www.example.com/index/_iri/\
         789d4dde087cf2996b8728f911b3807f22e671d2061a2dd47ecc42baf97528ba/1.0.0/project.kpar"
    );

    // Per-version `.project.json` lives in the same version directory; this
    // also exercises the `version_dir_url` trailing-slash invariant.
    assert_eq!(
        env.project_json_url("pkg:sysand/admin/proj0", "0.3.0")?
            .to_string(),
        "https://www.example.com/index/admin/proj0/0.3.0/.project.json"
    );

    Ok(())
}

#[test]
fn test_invalid_or_non_normalized_pkg_sysand_falls_through()
-> Result<(), Box<dyn std::error::Error>> {
    // IRIs whose publisher/name segments don't pass the purl validation +
    // normalization check must be rerouted to `_iri/<sha256>/`, preventing
    // path traversal, non-canonical casing, or otherwise invalid segments
    // from being spliced into URL paths.
    let env = test_env_async("https://www.example.com/index/")?;

    for iri in [
        // traversal / URL-syntax attacks
        "pkg:sysand/../attacker",
        "pkg:sysand/..%2Fattacker/proj",
        "pkg:sysand/./proj",
        "pkg:sysand/.hidden/proj",
        "pkg:sysand/pub/.hidden",
        // non-ASCII
        "pkg:sysand/Åcme/proj",
        // valid but not normalized (uppercase, spaces)
        "pkg:sysand/Admin/proj0",
        "pkg:sysand/admin/My Project",
        // too short (min 3 chars)
        "pkg:sysand/ab/proj0",
        // control characters
        "pkg:sysand/pub\t/proj",
        // URL-syntax characters
        "pkg:sysand/pub#frag/proj",
        "pkg:sysand/pub?q/proj",
    ] {
        let url = env.kpar_url(iri, "1.0.0")?.to_string();
        assert!(
            url.starts_with("https://www.example.com/index/_iri/"),
            "IRI `{iri}` produced URL `{url}` outside the _iri/ route"
        );
        assert!(
            !url.contains(".."),
            "`..` leaked into URL from `{iri}`: {url}"
        );
    }

    Ok(())
}

#[test]
fn test_base_url_without_trailing_slash() -> Result<(), Box<dyn std::error::Error>> {
    let env = test_env_async("https://www.example.com/index")?;

    assert_eq!(
        env.index_url()?.to_string(),
        "https://www.example.com/index/index.json"
    );
    assert_eq!(
        env.kpar_url("pkg:sysand/admin/proj0", "0.3.0")?.to_string(),
        "https://www.example.com/index/admin/proj0/0.3.0/project.kpar"
    );

    Ok(())
}

#[test]
fn test_uris_from_index_json() -> Result<(), Box<dyn std::error::Error>> {
    let mut server = mockito::Server::new();

    let env = test_env_sync(&server)?;

    let index_mock = server
        .mock("GET", "/index.json")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"{
                "projects": [
                    { "iri": "pkg:sysand/admin/proj0" },
                    { "iri": "urn:kpar:b" }
                ]
            }"#,
        )
        .expect_at_least(1)
        .create();

    let uris: Result<Vec<_>, _> = env.uris()?.collect();
    let uris = uris?;

    assert_eq!(uris.len(), 2);
    assert!(uris.contains(&"pkg:sysand/admin/proj0".to_string()));
    assert!(uris.contains(&"urn:kpar:b".to_string()));

    index_mock.assert();

    Ok(())
}

#[test]
fn test_versions_from_versions_json() -> Result<(), Box<dyn std::error::Error>> {
    let mut server = mockito::Server::new();

    let env = test_env_sync(&server)?;

    let pkg_versions_mock = server
        .mock("GET", "/admin/proj0/versions.json")
        .with_status(200)
        .with_header("content-type", "application/json")
        // newest-first per server contract
        .with_body(versions_json_body([("0.3.0", "[]"), ("0.2.0", "[]")]))
        .expect_at_least(1)
        .create();

    let iri_versions_mock = server
        .mock(
            "GET",
            "/_iri/621a5fdf587a3ecc878a98c8be2240dd5bbe561860d11f4da1ece4a4fe2fb8b5/versions.json",
        )
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(versions_json_body([("1.0.0", "[]")]))
        .expect_at_least(1)
        .create();

    let pkg_versions: Result<Vec<_>, _> = env.versions("pkg:sysand/admin/proj0")?.collect();
    let pkg_versions = pkg_versions?;

    assert_eq!(pkg_versions.len(), 2);
    assert!(pkg_versions.contains(&"0.3.0".to_string()));
    assert!(pkg_versions.contains(&"0.2.0".to_string()));

    let iri_versions: Result<Vec<_>, _> = env.versions("urn:kpar:b")?.collect();
    assert_eq!(iri_versions?, vec!["1.0.0"]);

    pkg_versions_mock.assert();
    iri_versions_mock.assert();

    Ok(())
}

#[test]
fn test_versions_json_preserves_server_order() -> Result<(), Box<dyn std::error::Error>> {
    // Server contract: entries are emitted newest-first by parsed semver.
    // The client does not re-sort; `versions_async` streams through the
    // wire order verbatim. Semver-tricky versions in the fixture make this
    // visible — a lexicographic-sort regression would reorder them.
    let mut server = mockito::Server::new();

    let env = test_env_sync(&server)?;

    let versions_mock = server
        .mock("GET", "/admin/proj0/versions.json")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(versions_json_body([
            ("10.0.0", "[]"),
            ("10.0.0-beta.1", "[]"),
            ("2.0.0", "[]"),
        ]))
        .expect_at_least(1)
        .create();

    let versions: Vec<_> = env
        .versions("pkg:sysand/admin/proj0")?
        .collect::<Result<_, _>>()?;
    assert_eq!(versions, vec!["10.0.0", "10.0.0-beta.1", "2.0.0"]);

    versions_mock.assert();

    Ok(())
}

#[test]
fn test_versions_json_non_semver_version_errors() -> Result<(), Box<dyn std::error::Error>> {
    // A `version` value that doesn't parse as semver rejects the whole
    // document with InvalidSemverVersion — the client cannot order the
    // entries without it.
    let mut server = mockito::Server::new();

    let env = test_env_sync(&server)?;

    let versions_mock = server
        .mock("GET", "/admin/proj0/versions.json")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(versions_json_body([("not-a-version", "[]")]))
        .expect_at_least(1)
        .create();

    let err = env
        .versions("pkg:sysand/admin/proj0")
        .expect_err("non-semver version must surface as a protocol error");
    match err {
        super::HTTPIndexEnvironmentError::InvalidSemverVersion { value, .. } => {
            assert_eq!(value, "not-a-version");
        }
        other => panic!("expected InvalidSemverVersion, got {other:?}"),
    }

    versions_mock.assert();

    Ok(())
}

#[test]
fn test_versions_missing_project_is_empty() -> Result<(), Box<dyn std::error::Error>> {
    let mut server = mockito::Server::new();

    let env = test_env_sync(&server)?;

    let versions_mock = server
        .mock("GET", "/nope/nope/versions.json")
        .with_status(404)
        .create();

    let versions: Result<Vec<_>, _> = env.versions("pkg:sysand/nope/nope")?.collect();
    assert!(versions?.is_empty());

    versions_mock.assert();

    Ok(())
}

#[test]
fn test_get_project_pkg_sysand_route() -> Result<(), Box<dyn std::error::Error>> {
    let mut server = mockito::Server::new();

    let env = test_env_sync(&server)?;

    let info_json = project_json_body("proj0", Some("admin"), "0.3.0", "[]");
    let meta_json = meta_json_body();
    let project_digest = project_digest(&info_json, meta_json)?;

    let versions_mock = server
        .mock("GET", "/admin/proj0/versions.json")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(versions_json_body_with_project_digest([(
            "0.3.0",
            "[]",
            &project_digest,
        )]))
        .expect_at_least(1)
        .create();

    let project_json_mock = server
        .mock("GET", "/admin/proj0/0.3.0/.project.json")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(info_json)
        .expect_at_least(1)
        .create();

    let meta_json_mock = server
        .mock("GET", "/admin/proj0/0.3.0/.meta.json")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(meta_json)
        .expect_at_least(1)
        .create();

    let project = env.get_project("pkg:sysand/admin/proj0", "0.3.0")?;

    let inner = &project.inner;
    assert_eq!(
        inner.archive.url.as_str(),
        format!("{}/admin/proj0/0.3.0/project.kpar", server.url())
    );

    // get_project should return the prefetched info without hitting the
    // kpar URL (no mock for the kpar body).
    let (info, meta) = project.get_project()?;
    let info = info.expect("info should be prefetched");
    assert_eq!(info.name, "proj0");
    assert_eq!(info.publisher.as_deref(), Some("admin"));
    assert_eq!(info.version, "0.3.0");
    assert!(info.usage.is_empty());
    assert!(meta.is_some());

    versions_mock.assert();
    project_json_mock.assert();
    meta_json_mock.assert();

    Ok(())
}

#[test]
fn test_get_project_iri_hash_route() -> Result<(), Box<dyn std::error::Error>> {
    let mut server = mockito::Server::new();

    let env = test_env_sync(&server)?;

    let info_json = project_json_body("b", None, "1.0.0", "[]");
    let meta_json = meta_json_body();
    let project_digest = project_digest(&info_json, meta_json)?;

    let versions_mock = server
        .mock(
            "GET",
            "/_iri/621a5fdf587a3ecc878a98c8be2240dd5bbe561860d11f4da1ece4a4fe2fb8b5/versions.json",
        )
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(versions_json_body_with_project_digest([(
            "1.0.0",
            "[]",
            &project_digest,
        )]))
        .expect_at_least(1)
        .create();

    let project_json_mock = server
        .mock(
            "GET",
            "/_iri/621a5fdf587a3ecc878a98c8be2240dd5bbe561860d11f4da1ece4a4fe2fb8b5/1.0.0/.project.json",
        )
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(info_json)
        .expect_at_least(1)
        .create();

    let meta_json_mock = server
        .mock(
            "GET",
            "/_iri/621a5fdf587a3ecc878a98c8be2240dd5bbe561860d11f4da1ece4a4fe2fb8b5/1.0.0/.meta.json",
        )
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(meta_json)
        .expect_at_least(1)
        .create();

    let project = env.get_project("urn:kpar:b", "1.0.0")?;

    let inner = &project.inner;
    assert_eq!(
        inner.archive.url.as_str(),
        format!(
            "{}/_iri/621a5fdf587a3ecc878a98c8be2240dd5bbe561860d11f4da1ece4a4fe2fb8b5/1.0.0/project.kpar",
            server.url()
        )
    );

    let (info, _) = project.get_project()?;
    let info = info.expect("info should be prefetched");
    assert_eq!(info.name, "b");
    assert_eq!(info.publisher, None);

    versions_mock.assert();
    project_json_mock.assert();
    meta_json_mock.assert();

    Ok(())
}

#[test]
fn test_get_project_carries_usage() -> Result<(), Box<dyn std::error::Error>> {
    let mut server = mockito::Server::new();

    let env = test_env_sync(&server)?;

    let usage_json = r#"[
        {"resource":"pkg:sysand/admin/dep","versionConstraint":"<2"},
        {"resource":"pkg:sysand/admin/other"}
    ]"#;
    let info_json = project_json_body("proj0", Some("admin"), "0.3.0", usage_json);
    let meta_json = meta_json_body();
    let project_digest = project_digest(&info_json, meta_json)?;

    let versions_mock = server
        .mock("GET", "/admin/proj0/versions.json")
        .with_status(200)
        .with_header("content-type", "application/json")
        // versions.json `usage` is required by the protocol but `usage` shown
        // to the caller comes from `.project.json`. Carry the same payload in
        // both to verify they're consistent and there's no double-merge.
        .with_body(versions_json_body_with_project_digest([(
            "0.3.0",
            usage_json,
            &project_digest,
        )]))
        .expect_at_least(1)
        .create();

    let project_json_mock = server
        .mock("GET", "/admin/proj0/0.3.0/.project.json")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(info_json)
        .expect_at_least(1)
        .create();

    let meta_json_mock = server
        .mock("GET", "/admin/proj0/0.3.0/.meta.json")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(meta_json)
        .expect_at_least(1)
        .create();

    let project = env.get_project("pkg:sysand/admin/proj0", "0.3.0")?;
    let (info, _) = project.get_project()?;
    let info = info.expect("info should be prefetched");

    assert_eq!(info.usage.len(), 2);
    assert_eq!(info.usage[0].resource, "pkg:sysand/admin/dep");
    assert_eq!(info.usage[0].version_constraint.as_deref(), Some("<2"));
    assert_eq!(info.usage[1].resource, "pkg:sysand/admin/other");
    assert_eq!(info.usage[1].version_constraint, None);

    versions_mock.assert();
    project_json_mock.assert();
    meta_json_mock.assert();

    Ok(())
}

#[test]
fn test_get_project_rejects_usage_drift_from_versions_json()
-> Result<(), Box<dyn std::error::Error>> {
    let mut server = mockito::Server::new();

    let env = test_env_sync(&server)?;

    let advertised_usage = r#"[
        {"resource":"pkg:sysand/admin/dep","versionConstraint":"<2"}
    ]"#;
    let drifted_usage = r#"[
        {"resource":"pkg:sysand/admin/dep","versionConstraint":"<3"}
    ]"#;

    let versions_mock = server
        .mock("GET", "/admin/proj0/versions.json")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(versions_json_body([("0.3.0", advertised_usage)]))
        .expect_at_least(1)
        .create();

    let project_json_mock = server
        .mock("GET", "/admin/proj0/0.3.0/.project.json")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(project_json_body(
            "proj0",
            Some("admin"),
            "0.3.0",
            drifted_usage,
        ))
        .expect_at_least(1)
        .create();

    let meta_json_mock = server
        .mock("GET", "/admin/proj0/0.3.0/.meta.json")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(meta_json_body())
        .expect_at_least(1)
        .create();

    let project = env.get_project("pkg:sysand/admin/proj0", "0.3.0")?;
    let err = project
        .get_info()
        .expect_err("selection drift must reject the selected version");

    match err {
        IndexedRemoteProjectError::AdvertisedMetadataDrift { field, .. } => {
            assert_eq!(field, "usage");
        }
        other => panic!("expected AdvertisedMetadataDrift, got {other:?}"),
    }

    versions_mock.assert();
    project_json_mock.assert();
    meta_json_mock.assert();

    Ok(())
}

#[test]
fn test_get_project_returns_real_info_meta_from_per_version_files()
-> Result<(), Box<dyn std::error::Error>> {
    // The per-version `.project.json` and `.meta.json` are the source of
    // truth for info/meta — pick values that no IRI-derived heuristic could
    // invent so a regression that re-introduces synthesis would fail.
    let mut server = mockito::Server::new();

    let env = test_env_sync(&server)?;

    let usage_json = r#"[{"resource":"pkg:sysand/x/y","versionConstraint":">=1"}]"#;
    let info_json = project_json_body(
        "real_name_from_server",
        Some("real_publisher"),
        "0.3.0",
        usage_json,
    );
    let meta_json = r#"{"index":{},"created":"2026-04-17T00:00:00.000000000Z","metamodel":"https://www.omg.org/spec/KerML/20250201"}"#;
    let project_digest = project_digest(&info_json, meta_json)?;

    let versions_mock = server
        .mock("GET", "/admin/proj0/versions.json")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(versions_json_body_with_project_digest([(
            "0.3.0",
            usage_json,
            &project_digest,
        )]))
        .expect_at_least(1)
        .create();

    let project_json_mock = server
        .mock("GET", "/admin/proj0/0.3.0/.project.json")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(info_json)
        .expect_at_least(1)
        .create();

    let meta_json_mock = server
        .mock("GET", "/admin/proj0/0.3.0/.meta.json")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(meta_json)
        .expect_at_least(1)
        .create();

    let project = env.get_project("pkg:sysand/admin/proj0", "0.3.0")?;
    let (info, meta) = project.get_project()?;
    let info = info.expect("info should be prefetched");
    let meta = meta.expect("meta should be prefetched");

    assert_eq!(info.name, "real_name_from_server");
    assert_eq!(info.publisher.as_deref(), Some("real_publisher"));
    assert_eq!(info.version, "0.3.0");
    assert_eq!(info.usage.len(), 1);
    assert_eq!(info.usage[0].resource, "pkg:sysand/x/y");
    assert_eq!(info.usage[0].version_constraint.as_deref(), Some(">=1"));

    assert_eq!(
        meta.metamodel.as_deref(),
        Some("https://www.omg.org/spec/KerML/20250201")
    );

    versions_mock.assert();
    project_json_mock.assert();
    meta_json_mock.assert();

    Ok(())
}

#[test]
fn test_get_project_missing_per_version_info_errors() -> Result<(), Box<dyn std::error::Error>> {
    // The protocol guarantees `.project.json` whenever `versions.json` lists
    // the version. A 404 must surface as a hard error rather than silently
    // proceeding without info.
    let mut server = mockito::Server::new();

    let env = test_env_sync(&server)?;

    let versions_mock = server
        .mock("GET", "/admin/proj0/versions.json")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(versions_json_body([("0.3.0", "[]")]))
        .expect_at_least(1)
        .create();

    let project_json_mock = server
        .mock("GET", "/admin/proj0/0.3.0/.project.json")
        .with_status(404)
        .expect_at_least(1)
        .create();

    let _meta_json_mock = server
        .mock("GET", "/admin/proj0/0.3.0/.meta.json")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(meta_json_body())
        .create();

    // `env.get_project` returns a lazy wrapper; forcing `get_project()` on it
    // triggers the per-version `.project.json` fetch that must surface the
    // 404 as a hard error.
    let project = env.get_project("pkg:sysand/admin/proj0", "0.3.0")?;
    let err = project
        .get_project()
        .expect_err("missing .project.json must surface as a hard error");
    let text = format!("{err:?}");
    assert!(
        text.contains("BadHttpStatus") && text.contains("/admin/proj0/0.3.0/.project.json"),
        "expected BadHttpStatus on .project.json, got: {text}"
    );

    versions_mock.assert();
    project_json_mock.assert();

    Ok(())
}

#[test]
fn test_get_project_version_not_in_versions_json_errors() -> Result<(), Box<dyn std::error::Error>>
{
    // versions.json is the source of truth for which versions exist. A
    // request for a version it doesn't list must surface VersionNotInIndex
    // — there is no kpar-only fallback in the new protocol.
    let mut server = mockito::Server::new();

    let env = test_env_sync(&server)?;

    let versions_mock = server
        .mock("GET", "/admin/proj0/versions.json")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(versions_json_body([("0.3.0", "[]")]))
        .expect_at_least(1)
        .create();

    let err = env
        .get_project("pkg:sysand/admin/proj0", "9.9.9")
        .expect_err("requesting an absent version must error");
    match err {
        super::HTTPIndexEnvironmentError::VersionNotInIndex { url, version } => {
            assert!(
                url.contains("/admin/proj0/versions.json"),
                "url carried: {url}"
            );
            assert_eq!(version, "9.9.9");
        }
        other => panic!("expected VersionNotInIndex, got {other:?}"),
    }

    versions_mock.assert();

    Ok(())
}

#[test]
fn test_checksum_uses_inline_project_digest_and_skips_kpar_download()
-> Result<(), Box<dyn std::error::Error>> {
    // When versions.json carries project_digest, checksum_canonical_hex must
    // return that value without fetching the kpar body. The kpar endpoint is
    // marked expect(0) to catch a regression that would silently start
    // downloading archives during resolution.
    let mut server = mockito::Server::new();

    let env = test_env_sync(&server)?;

    let expected_hex = "b".repeat(64);
    let advertised_digest = format!("sha256:{expected_hex}");

    let versions_mock = server
        .mock("GET", "/admin/proj0/versions.json")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(format!(
            r#"{{"versions":[{{"version":"0.3.0","usage":[],"project_digest":"{advertised_digest}","kpar_size":42,"kpar_digest":"{FILLER_DIGEST}"}}]}}"#,
        ))
        .expect_at_least(1)
        .create();

    // The short-circuit in `checksum_canonical_hex` returns the advertised
    // digest before any per-version read, so the `.project.json` / `.meta.json`
    // endpoints must not be touched either — `expect(0)` catches a regression
    // that would re-introduce a materialization step here.
    let project_json_mock =
        expect_untouched(&mut server, "GET", "/admin/proj0/0.3.0/.project.json");

    let meta_json_mock = expect_untouched(&mut server, "GET", "/admin/proj0/0.3.0/.meta.json");

    let kpar_mock = expect_untouched(&mut server, "GET", "/admin/proj0/0.3.0/project.kpar");

    let project = env.get_project("pkg:sysand/admin/proj0", "0.3.0")?;
    let digest = project
        .checksum_canonical_hex()?
        .expect("prefetched digest should propagate");
    assert_eq!(digest, expected_hex);

    versions_mock.assert();
    project_json_mock.assert();
    meta_json_mock.assert();
    kpar_mock.assert();

    Ok(())
}

#[test]
fn test_malformed_project_digest_errors() -> Result<(), Box<dyn std::error::Error>> {
    // `project_digest` is a required protocol field; a non-`sha256:<hex>`
    // value is a server bug and must surface as an error rather than be
    // silently replaced with a locally-computed (and lock/sync-incompatible)
    // hash. The kpar endpoint must never be touched.
    let mut server = mockito::Server::new();

    let env = test_env_sync(&server)?;

    let versions_mock = server
        .mock("GET", "/admin/proj0/versions.json")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(format!(
            r#"{{"versions":[{{"version":"0.3.0","usage":[],"project_digest":"md5:abc","kpar_size":42,"kpar_digest":"{FILLER_DIGEST}"}}]}}"#,
        ))
        .expect_at_least(1)
        .create();

    let kpar_mock = expect_untouched(&mut server, "GET", "/admin/proj0/0.3.0/project.kpar");

    let err = env
        .get_project("pkg:sysand/admin/proj0", "0.3.0")
        .expect_err("malformed project_digest must surface as a protocol error");
    match err {
        super::HTTPIndexEnvironmentError::InvalidVersionEntry {
            version,
            field,
            value,
            ..
        } => {
            assert_eq!(version, "0.3.0");
            assert_eq!(field, "project_digest");
            assert_eq!(value, "md5:abc");
        }
        other => panic!("expected InvalidVersionEntry, got {other:?}"),
    }

    versions_mock.assert();
    kpar_mock.assert();

    Ok(())
}

#[test]
fn test_missing_required_field_errors() -> Result<(), Box<dyn std::error::Error>> {
    // Each of `project_digest`, `kpar_size`, `kpar_digest` is required; an
    // entry omitting any of them must reject the whole document at parse
    // time rather than silently degrade.
    let mut server = mockito::Server::new();

    let env = test_env_sync(&server)?;

    // No `kpar_digest`.
    let versions_mock = server
        .mock("GET", "/admin/proj0/versions.json")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(format!(
            r#"{{"versions":[{{"version":"0.3.0","usage":[],"project_digest":"{FILLER_DIGEST}","kpar_size":42}}]}}"#,
        ))
        .expect_at_least(1)
        .create();

    let err = env
        .versions("pkg:sysand/admin/proj0")
        .expect_err("missing required field must reject the document");
    assert!(
        matches!(
            err,
            super::HTTPIndexEnvironmentError::Fetch(super::HttpFetchError::JsonParse { .. })
        ),
        "expected Fetch(JsonParse), got {err:?}"
    );

    versions_mock.assert();

    Ok(())
}

#[test]
fn test_duplicate_versions_are_rejected() -> Result<(), Box<dyn std::error::Error>> {
    // A server listing the same version twice violates the protocol. The
    // client rejects the whole document at ingest rather than silently
    // preserving both and letting first-match select one.
    let mut server = mockito::Server::new();

    let env = test_env_sync(&server)?;

    let versions_mock = server
        .mock("GET", "/admin/proj0/versions.json")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(versions_json_body([("1.0.0", "[]"), ("1.0.0", "[]")]))
        .expect_at_least(1)
        .create();

    let err = env
        .versions("pkg:sysand/admin/proj0")
        .expect_err("duplicate versions must reject the document");
    match err {
        super::HTTPIndexEnvironmentError::DuplicateVersion { version, .. } => {
            assert_eq!(version, "1.0.0");
        }
        other => panic!("expected DuplicateVersion, got {other:?}"),
    }

    versions_mock.assert();

    Ok(())
}

#[test]
fn test_kpar_digest_mismatch_surfaces_error() -> Result<(), Box<dyn std::error::Error>> {
    // versions.json advertises a kpar_digest that doesn't match the served
    // body. The first call that forces `ensure_downloaded` (here, read_source)
    // must surface DigestMismatch rather than silently accepting the
    // mismatched archive.
    let mut server = mockito::Server::new();

    let env = test_env_sync(&server)?;

    // An advertised digest that doesn't match the body bytes below.
    let advertised_digest_hex = "0".repeat(64);
    let advertised = format!("sha256:{advertised_digest_hex}");

    let versions_mock = server
        .mock("GET", "/admin/proj0/versions.json")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(format!(
            r#"{{"versions":[{{"version":"0.3.0","usage":[],"project_digest":"{FILLER_DIGEST}","kpar_size":42,"kpar_digest":"{advertised}"}}]}}"#,
        ))
        .expect_at_least(1)
        .create();

    // `read_source` only needs the kpar archive; per-version `.project.json` /
    // `.meta.json` fetches would be a regression. Mock them as `expect(0)` so
    // such a regression fails loudly.
    let project_json_mock =
        expect_untouched(&mut server, "GET", "/admin/proj0/0.3.0/.project.json");

    let meta_json_mock = expect_untouched(&mut server, "GET", "/admin/proj0/0.3.0/.meta.json");

    let kpar_mock = server
        .mock("GET", "/admin/proj0/0.3.0/project.kpar")
        .with_status(200)
        .with_header("content-type", "application/zip")
        .with_body(b"not really a kpar")
        .expect_at_least(1)
        .create();

    let project = env.get_project("pkg:sysand/admin/proj0", "0.3.0")?;
    let err = project
        .read_source("anything.sysml")
        .err()
        .expect("mismatched kpar digest must error");
    match err {
        IndexedRemoteProjectError::Downloaded(ReqwestKparDownloadedError::DigestMismatch {
            expected,
            ..
        }) => {
            assert_eq!(expected, advertised_digest_hex);
        }
        other => panic!("expected DigestMismatch, got {other:?}"),
    }

    versions_mock.assert();
    project_json_mock.assert();
    meta_json_mock.assert();
    kpar_mock.assert();

    Ok(())
}

#[test]
fn test_kpar_digest_mismatch_does_not_persist_archive() -> Result<(), Box<dyn std::error::Error>> {
    // Defense in depth around DigestMismatch: a failed verification must
    // never leave a usable archive at the project's `archive_path`. A retry
    // therefore re-downloads (and re-verifies) rather than short-circuiting
    // on a stale, tampered file. The invariant is that the atomic-rename
    // path never installs a mismatched body at `archive_path`.
    let mut server = mockito::Server::new();

    let env = test_env_sync(&server)?;

    let advertised_digest_hex = "0".repeat(64);
    let advertised = format!("sha256:{advertised_digest_hex}");

    let versions_mock = server
        .mock("GET", "/admin/proj0/versions.json")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(format!(
            r#"{{"versions":[{{"version":"0.3.0","usage":[],"project_digest":"{FILLER_DIGEST}","kpar_size":42,"kpar_digest":"{advertised}"}}]}}"#,
        ))
        .expect_at_least(1)
        .create();

    // `read_source` is exercised here for its kpar-download effect only; it
    // must not reach out to the per-version JSON endpoints.
    let project_json_mock =
        expect_untouched(&mut server, "GET", "/admin/proj0/0.3.0/.project.json");

    let meta_json_mock = expect_untouched(&mut server, "GET", "/admin/proj0/0.3.0/.meta.json");

    let kpar_mock = server
        .mock("GET", "/admin/proj0/0.3.0/project.kpar")
        .with_status(200)
        .with_header("content-type", "application/zip")
        .with_body(b"not really a kpar")
        .expect_at_least(1)
        .create();

    let project = env.get_project("pkg:sysand/admin/proj0", "0.3.0")?;
    let _ = project
        .read_source("anything.sysml")
        .err()
        .expect("mismatched kpar digest must error");

    assert!(
        !project.inner.archive.is_downloaded(),
        "tampered archive must not persist at `{}` after DigestMismatch",
        project.inner.archive.inner.archive_path
    );

    versions_mock.assert();
    project_json_mock.assert();
    meta_json_mock.assert();
    kpar_mock.assert();

    Ok(())
}

/// Build a minimal kpar (ZIP) archive carrying `root/.project.json`,
/// `root/.meta.json`, and a single source file, returning the archive bytes
/// alongside the exact info/meta JSON strings written into it. Tests that
/// also mock the per-version `.project.json` / `.meta.json` endpoints reuse
/// those strings so the index-served content matches the in-archive content
/// — the only deliberate drift remains in the advertised `project_digest`.
fn build_minimal_kpar(
    name: &str,
    version: &str,
    src_path: &str,
    src_body: &str,
) -> (Vec<u8>, String, &'static str) {
    use std::io::Write as _;
    let info_json = format!(r#"{{"name":"{name}","version":"{version}","usage":[]}}"#);
    let meta_json: &'static str = r#"{"index":{},"created":"0000-00-00T00:00:00.123456789Z"}"#;
    let mut buf: Vec<u8> = Vec::new();
    {
        let mut zip = zip::ZipWriter::new(std::io::Cursor::new(&mut buf));
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Stored)
            .unix_permissions(0o755);
        zip.start_file("root/.project.json", options).unwrap();
        zip.write_all(info_json.as_bytes()).unwrap();
        zip.start_file("root/.meta.json", options).unwrap();
        zip.write_all(meta_json.as_bytes()).unwrap();
        zip.start_file(format!("root/{src_path}"), options).unwrap();
        zip.write_all(src_body.as_bytes()).unwrap();
        zip.finish().unwrap();
    }
    (buf, info_json, meta_json)
}

#[test]
fn test_project_digest_drift_after_download_errors() -> Result<(), Box<dyn std::error::Error>> {
    // versions.json advertises a deliberately-wrong `project_digest` but a
    // correct `kpar_digest`, so the kpar download succeeds and the post-
    // download `checksum_canonical_hex` reaches the reconciliation step.
    // The locally-computed canonical digest will not match the advertised
    // value, and the mismatch must surface as `AdvertisedDigestDrift` — server
    // and client disagreeing on canonicalization cannot silently corrupt
    // the lockfile.
    use sha2::{Digest as _, Sha256};

    let mut server = mockito::Server::new();

    let env = test_env_sync(&server)?;

    // `.project.json` / `.meta.json` are served only from inside the kpar
    // for this test; the per-version JSON mocks below are `expect_untouched`,
    // so the destructured info/meta strings from `build_minimal_kpar` are
    // unused.
    let (kpar_bytes, _info_json, _meta_json) =
        build_minimal_kpar("proj0", "0.3.0", "foo.sysml", "// hi");
    let kpar_digest_hex = format!("{:x}", Sha256::digest(&kpar_bytes));
    let advertised_kpar = format!("sha256:{kpar_digest_hex}");

    // `bbb…b` is not the canonical project digest of the archive above,
    // which is what forces the drift branch post-download.
    let wrong_project_digest_hex = "b".repeat(64);
    let advertised_project = format!("sha256:{wrong_project_digest_hex}");

    let versions_mock = server
        .mock("GET", "/admin/proj0/versions.json")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(format!(
            r#"{{"versions":[{{"version":"0.3.0","usage":[],"project_digest":"{advertised_project}","kpar_size":{kpar_size},"kpar_digest":"{advertised_kpar}"}}]}}"#,
            kpar_size = kpar_bytes.len(),
        ))
        .expect_at_least(1)
        .create();

    // Per-version `.project.json` / `.meta.json` endpoints carry the same
    // content the kpar embeds, but neither `read_source` nor the post-download
    // branch of `checksum_canonical_hex` touches them — reconciliation runs
    // against the in-archive copies. `expect(0)` catches a regression that
    // would fall back to the per-version JSON during drift checks.
    let project_json_mock =
        expect_untouched(&mut server, "GET", "/admin/proj0/0.3.0/.project.json");

    let meta_json_mock = expect_untouched(&mut server, "GET", "/admin/proj0/0.3.0/.meta.json");

    let kpar_mock = server
        .mock("GET", "/admin/proj0/0.3.0/project.kpar")
        .with_status(200)
        .with_header("content-type", "application/zip")
        .with_body(&kpar_bytes)
        .expect_at_least(1)
        .create();

    let project = env.get_project("pkg:sysand/admin/proj0", "0.3.0")?;

    // Force a download so `checksum_canonical_hex` reaches the
    // post-download (authoritative local) branch rather than the
    // pre-download shortcut.
    let mut reader = project.read_source("foo.sysml")?;
    let mut buf = String::new();
    std::io::Read::read_to_string(&mut reader, &mut buf)?;
    drop(reader);

    let err = project
        .checksum_canonical_hex()
        .expect_err("drift between advertised and locally-computed digest must error");

    // Surface path: CanonicalizationError::ProjectRead ->
    // IndexedRemoteProjectError::AdvertisedDigestDrift. Traverse whatever
    // wrappers the display-side adds by matching on the debug text.
    let text = format!("{err:?}");
    assert!(
        text.contains("AdvertisedDigestDrift"),
        "expected AdvertisedDigestDrift, got: {text}"
    );
    assert!(
        text.contains(&wrong_project_digest_hex),
        "advertised digest should appear in error: {text}"
    );

    versions_mock.assert();
    project_json_mock.assert();
    meta_json_mock.assert();
    kpar_mock.assert();

    Ok(())
}

#[test]
fn test_kpar_digest_match_allows_download() -> Result<(), Box<dyn std::error::Error>> {
    // When kpar_digest matches the served body, verification passes and
    // control flow reaches the kpar parser. The served body isn't a valid
    // zip, so the eventual error is a KPar-layer error, not DigestMismatch —
    // which is the signal that the digest check passed.
    use sha2::{Digest as _, Sha256};

    let mut server = mockito::Server::new();

    let env = test_env_sync(&server)?;

    let body: &[u8] = b"not really a kpar either";
    let actual_hex = format!("{:x}", Sha256::digest(body));
    let advertised = format!("sha256:{actual_hex}");

    let versions_mock = server
        .mock("GET", "/admin/proj0/versions.json")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(format!(
            r#"{{"versions":[{{"version":"0.3.0","usage":[],"project_digest":"{FILLER_DIGEST}","kpar_size":42,"kpar_digest":"{advertised}"}}]}}"#,
        ))
        .expect_at_least(1)
        .create();

    // Per-version JSON endpoints aren't on the `read_source` path — `expect(0)`
    // locks that in.
    let project_json_mock =
        expect_untouched(&mut server, "GET", "/admin/proj0/0.3.0/.project.json");

    let meta_json_mock = expect_untouched(&mut server, "GET", "/admin/proj0/0.3.0/.meta.json");

    let kpar_mock = server
        .mock("GET", "/admin/proj0/0.3.0/project.kpar")
        .with_status(200)
        .with_header("content-type", "application/zip")
        .with_body(body)
        .expect_at_least(1)
        .create();

    let project = env.get_project("pkg:sysand/admin/proj0", "0.3.0")?;
    let err = project
        .read_source("anything.sysml")
        .err()
        .expect("invalid zip will still error, but not with DigestMismatch");
    if matches!(
        err,
        IndexedRemoteProjectError::Downloaded(ReqwestKparDownloadedError::DigestMismatch { .. })
    ) {
        panic!("digest matched; DigestMismatch should not surface");
    }

    versions_mock.assert();
    project_json_mock.assert();
    meta_json_mock.assert();
    kpar_mock.assert();

    Ok(())
}

#[test]
fn test_versions_json_fetched_once_per_env_lifetime() -> Result<(), Box<dyn std::error::Error>> {
    // `versions_async` and `get_project_async` both read versions.json; with
    // caching in place the document is fetched at most once per env lifetime
    // regardless of how many candidates the solver touches. The mock is set
    // to expect(1) so any regression that re-fetches would fail the
    // assertion.
    let mut server = mockito::Server::new();

    let env = test_env_sync(&server)?;

    let versions_mock = server
        .mock("GET", "/admin/proj0/versions.json")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(versions_json_body([("0.3.0", "[]")]))
        .expect(1)
        .create();

    // get_project also reaches `.project.json` / `.meta.json`; mock them
    // so the call succeeds, but they're orthogonal to the versions.json
    // cache assertion (no expectation on count).
    let _project_json_mock = server
        .mock("GET", "/admin/proj0/0.3.0/.project.json")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(project_json_body("proj0", Some("admin"), "0.3.0", "[]"))
        .create();
    let _meta_json_mock = server
        .mock("GET", "/admin/proj0/0.3.0/.meta.json")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(meta_json_body())
        .create();

    // Three independent calls into paths that consult versions.json.
    let _ = env.versions("pkg:sysand/admin/proj0")?.collect::<Vec<_>>();
    let _ = env.versions("pkg:sysand/admin/proj0")?.collect::<Vec<_>>();
    let _ = env.get_project("pkg:sysand/admin/proj0", "0.3.0")?;

    versions_mock.assert();

    Ok(())
}

#[test]
fn test_versions_json_negative_result_is_cached() -> Result<(), Box<dyn std::error::Error>> {
    // 404s must be cached for the env's lifetime too, not just successful
    // hits. Otherwise a `sysand lock` run that probes many absent IRIs across
    // several resolvers pays for the same 404 repeatedly. The mock is capped
    // at `expect(1)` so any regression in negative caching fails loudly.
    let mut server = mockito::Server::new();

    let env = test_env_sync(&server)?;

    let versions_mock = server
        .mock("GET", "/nope/nope/versions.json")
        .with_status(404)
        .expect(1)
        .create();

    let _ = env.versions("pkg:sysand/nope/nope")?.collect::<Vec<_>>();
    let _ = env.versions("pkg:sysand/nope/nope")?.collect::<Vec<_>>();
    let _ = env.versions("pkg:sysand/nope/nope")?.collect::<Vec<_>>();

    versions_mock.assert();

    Ok(())
}

#[test]
fn test_sources_async_uses_inline_kpar_size_and_skips_head()
-> Result<(), Box<dyn std::error::Error>> {
    // When versions.json carries kpar_size, sources_async must take it from
    // there and never issue a HEAD. The HEAD mock is set to expect(0) and
    // would return a different size if accidentally called, so either guard
    // catches a regression.
    let mut server = mockito::Server::new();

    let env = test_env_sync(&server)?;

    let versions_mock = server
        .mock("GET", "/admin/proj0/versions.json")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(format!(
            r#"{{"versions":[{{"version":"0.3.0","usage":[],"project_digest":"{FILLER_DIGEST}","kpar_size":42,"kpar_digest":"{FILLER_DIGEST}"}}]}}"#,
        ))
        .expect_at_least(1)
        .create();

    // `sources` pulls from the inline kpar_size so the per-version JSON
    // endpoints shouldn't be touched either.
    let project_json_mock =
        expect_untouched(&mut server, "GET", "/admin/proj0/0.3.0/.project.json");

    let meta_json_mock = expect_untouched(&mut server, "GET", "/admin/proj0/0.3.0/.meta.json");

    let head_mock = expect_untouched(&mut server, "HEAD", "/admin/proj0/0.3.0/project.kpar");

    let project = env.get_project("pkg:sysand/admin/proj0", "0.3.0")?;
    let sources = project.sources(&ProjectContext::default())?;

    assert_eq!(sources.len(), 1);
    match &sources[0] {
        Source::RemoteKpar {
            remote_kpar_size, ..
        } => assert_eq!(*remote_kpar_size, Some(42)),
        other => panic!("expected Source::RemoteKpar, got {:?}", other),
    }

    versions_mock.assert();
    project_json_mock.assert();
    meta_json_mock.assert();
    head_mock.assert();

    Ok(())
}

#[test]
fn test_missing_index_is_hard_error() -> Result<(), Box<dyn std::error::Error>> {
    // `index.json` is the root document that identifies a URL as a sysand
    // index. A 404 means "this URL is not a sysand index" — propagate as a
    // BadHttpStatus rather than silently pretending the index is empty, so
    // a misconfigured base URL surfaces clearly. Empty-but-live indices
    // serve `{"projects": []}` with 200 OK.
    let mut server = mockito::Server::new();

    let env = test_env_sync(&server)?;

    let index_mock = server
        .mock("GET", "/index.json")
        .with_status(404)
        .with_body("not found")
        .create();

    let err = env
        .uris()
        .expect_err("missing index.json must error, not be treated as empty");
    match err {
        super::HTTPIndexEnvironmentError::Fetch(super::HttpFetchError::BadHttpStatus {
            url,
            status,
        }) => {
            assert_eq!(status, reqwest::StatusCode::NOT_FOUND);
            assert!(url.contains("/index.json"), "url carried: {url}");
        }
        other => panic!("expected Fetch(BadHttpStatus 404), got {other:?}"),
    }
    index_mock.assert();

    Ok(())
}

#[test]
fn test_empty_but_live_index_yields_no_uris() -> Result<(), Box<dyn std::error::Error>> {
    // A registry with no projects serves `{"projects": []}` with 200 OK;
    // `uris_async` returns an empty stream. This is distinct from the 404
    // path above, which surfaces as a hard error.
    let mut server = mockito::Server::new();

    let env = test_env_sync(&server)?;

    let index_mock = server
        .mock("GET", "/index.json")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"projects": []}"#)
        .create();

    let uris: Result<Vec<_>, _> = env.uris()?.collect();
    assert!(uris?.is_empty());
    index_mock.assert();

    Ok(())
}

#[test]
fn test_server_error_surfaces() -> Result<(), Box<dyn std::error::Error>> {
    let mut server = mockito::Server::new();

    let env = test_env_sync(&server)?;

    let index_mock = server
        .mock("GET", "/index.json")
        .with_status(500)
        .with_body("server error")
        .create();

    assert!(env.uris().is_err());
    index_mock.assert();

    Ok(())
}

#[test]
fn test_versions_json_with_artifact_metadata_parses() -> Result<(), Box<dyn std::error::Error>> {
    // Versions.json carrying the required artifact fields (project_digest,
    // kpar_size, kpar_digest) and an unknown extra field must parse cleanly.
    // The ignore-unknowns behavior is load-bearing for forward compatibility,
    // so exercise it here.
    let mut server = mockito::Server::new();

    let env = test_env_sync(&server)?;

    let versions_mock = server
        .mock("GET", "/admin/proj0/versions.json")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"{
                "versions": [
                    {
                        "version": "0.3.0",
                        "project_digest": "sha256:00000000000000000000000000000000000000000000000000000000deadbeef",
                        "kpar_size": 4096,
                        "kpar_digest": "sha256:00000000000000000000000000000000000000000000000000000000cafef00d",
                        "usage": [],
                        "some_future_field": "ignored"
                    }
                ],
                "another_future_field": ["also", "ignored"]
            }"#,
        )
        .expect_at_least(1)
        .create();

    let versions: Vec<_> = env
        .versions("pkg:sysand/admin/proj0")?
        .collect::<Result<_, _>>()?;
    assert_eq!(versions, vec!["0.3.0"]);
    versions_mock.assert();

    Ok(())
}

#[test]
fn test_malformed_index_json_surfaces_parse_error() -> Result<(), Box<dyn std::error::Error>> {
    // A misconfigured reverse proxy may serve an HTML error page with 200 OK.
    // The client must surface that as a JsonParse error with the URL preserved
    // rather than silently treating the index as empty.
    let mut server = mockito::Server::new();

    let env = test_env_sync(&server)?;

    let index_mock = server
        .mock("GET", "/index.json")
        .with_status(200)
        .with_header("content-type", "text/html")
        .with_body("<html><body>not json</body></html>")
        .create();

    let err = env.uris().expect_err("malformed JSON must error");
    match err {
        super::HTTPIndexEnvironmentError::Fetch(super::HttpFetchError::JsonParse {
            url, ..
        }) => {
            assert!(url.contains("/index.json"), "url carried: {url}");
        }
        other => panic!("expected Fetch(JsonParse), got {other:?}"),
    }
    index_mock.assert();

    Ok(())
}

#[test]
fn test_malformed_versions_json_surfaces_parse_error() -> Result<(), Box<dyn std::error::Error>> {
    let mut server = mockito::Server::new();

    let env = test_env_sync(&server)?;

    let versions_mock = server
        .mock("GET", "/admin/proj0/versions.json")
        .with_status(200)
        .with_header("content-type", "text/html")
        .with_body("<html><body>not json</body></html>")
        .create();

    let err = env
        .versions("pkg:sysand/admin/proj0")
        .expect_err("malformed JSON must error");
    match err {
        super::HTTPIndexEnvironmentError::Fetch(super::HttpFetchError::JsonParse {
            url, ..
        }) => {
            assert!(
                url.contains("/admin/proj0/versions.json"),
                "url carried: {url}"
            );
        }
        other => panic!("expected Fetch(JsonParse), got {other:?}"),
    }
    versions_mock.assert();

    Ok(())
}

#[test]
fn test_versions_async_rejects_document_with_malformed_digest()
-> Result<(), Box<dyn std::error::Error>> {
    // Digest-field shape is checked eagerly during `versions.json` parsing,
    // not only when a specific version is requested for materialization
    // (merge-list #5). This matters because `versions_async` and
    // `get_project_async` share the same cache: if validation were deferred,
    // `versions_async` would happily stream version strings from a protocol-
    // invalid document and the matching `get_project_async` call would
    // surprise the caller with an error much later.
    let mut server = mockito::Server::new();

    let env = test_env_sync(&server)?;

    let versions_mock = server
        .mock("GET", "/admin/proj0/versions.json")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(format!(
            r#"{{"versions":[{{"version":"0.3.0","usage":[],"project_digest":"{FILLER_DIGEST}","kpar_size":42,"kpar_digest":"md5:abc"}}]}}"#,
        ))
        .expect_at_least(1)
        .create();

    let err = env
        .versions("pkg:sysand/admin/proj0")
        .expect_err("malformed kpar_digest must reject the document at parse time");
    match err {
        super::HTTPIndexEnvironmentError::InvalidVersionEntry {
            version,
            field,
            value,
            ..
        } => {
            assert_eq!(version, "0.3.0");
            assert_eq!(field, "kpar_digest");
            assert_eq!(value, "md5:abc");
        }
        other => panic!("expected InvalidVersionEntry, got {other:?}"),
    }

    versions_mock.assert();

    Ok(())
}

#[test]
fn test_get_project_accepts_canonical_digest_with_mixed_case_sha256_meta()
-> Result<(), Box<dyn std::error::Error>> {
    // Regression guard for the digest-canonicalization unification fix
    // (review1 #2 / merge-list #1). A `.meta.json` whose `checksum` entries
    // carry mixed-case SHA256 hex values is well-formed but the raw-hash
    // reconciliation that used to live in `fetched_project_async` would hash
    // the uppercase-as-written meta and compare that to the server's
    // canonical digest (which is computed after lowercasing), producing a
    // spurious `AdvertisedDigestDrift`. With the fix, reconciliation uses the
    // canonical-inline digest and this happy path succeeds without
    // downloading the kpar.
    let mut server = mockito::Server::new();

    let env = test_env_sync(&server)?;

    let info_json = project_json_body("proj0", Some("admin"), "0.3.0", "[]");
    // 64-char hex with uppercase digits — still a legal SHA256 value, but
    // canonicalization lowercases it before hashing.
    let meta_json = r#"{"index":{"Sym":"foo.sysml"},"created":"2026-01-01T00:00:00.000000000Z","checksum":{"foo.sysml":{"value":"ABCDEF0123456789ABCDEF0123456789ABCDEF0123456789ABCDEF0123456789","algorithm":"SHA256"}}}"#;
    let advertised_digest = canonical_project_digest(&info_json, meta_json)?;
    // Sanity-check: the canonical digest differs from the raw-hash digest;
    // otherwise this test would pass even with the old buggy code.
    assert_ne!(
        advertised_digest,
        project_digest(&info_json, meta_json)?,
        "fixture must exercise canonicalization — raw and canonical digests should differ"
    );

    let versions_mock = server
        .mock("GET", "/admin/proj0/versions.json")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(versions_json_body_with_project_digest([(
            "0.3.0",
            "[]",
            &advertised_digest,
        )]))
        .expect_at_least(1)
        .create();

    let project_json_mock = server
        .mock("GET", "/admin/proj0/0.3.0/.project.json")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(info_json)
        .expect_at_least(1)
        .create();

    let meta_json_mock = server
        .mock("GET", "/admin/proj0/0.3.0/.meta.json")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(meta_json)
        .expect_at_least(1)
        .create();

    // The kpar must not be fetched — reconciliation is against the canonical
    // digest of the JSON pair, not the archive.
    let kpar_mock = expect_untouched(&mut server, "GET", "/admin/proj0/0.3.0/project.kpar");

    let project = env.get_project("pkg:sysand/admin/proj0", "0.3.0")?;
    let (info, meta) = project
        .get_project()
        .expect("canonical digest reconciliation must succeed for mixed-case SHA256 meta");
    assert_eq!(info.as_ref().map(|i| i.name.as_str()), Some("proj0"));
    assert!(meta.is_some());

    versions_mock.assert();
    project_json_mock.assert();
    meta_json_mock.assert();
    kpar_mock.assert();

    Ok(())
}

#[test]
fn test_get_project_defers_digest_check_for_non_sha256_meta_checksums()
-> Result<(), Box<dyn std::error::Error>> {
    // When `.meta.json` carries a non-SHA256 checksum entry, the canonical
    // project digest can't be computed without reading the corresponding
    // source file from the kpar. The pre-download reconciliation must not
    // fail in that case — the `kpar_digest` check on the archive body and
    // the post-download `checksum_canonical_hex` path remain the
    // authoritative checks. Before the fix, `fetched_project_async` hashed
    // the raw meta and would reject any server advertising the (correct)
    // canonical digest.
    let mut server = mockito::Server::new();

    let env = test_env_sync(&server)?;

    let info_json = project_json_body("proj0", Some("admin"), "0.3.0", "[]");
    // Non-SHA256 algorithm: canonicalizing this entry requires reading the
    // actual source to compute its SHA256. Inline canonicalization returns
    // `None` and reconciliation must defer.
    let meta_json = r#"{"index":{"Sym":"foo.sysml"},"created":"2026-01-01T00:00:00.000000000Z","checksum":{"foo.sysml":{"value":"dabe95d26be5d1c68a80fae65d12ae056e8fc8ab","algorithm":"SHA1"}}}"#;

    // Use an arbitrary valid-format digest. The server's canonical digest
    // can't be verified pre-download (sources are in the kpar), so whatever
    // the server advertises must be accepted here.
    let advertised_digest = format!("sha256:{}", "a".repeat(64));

    let versions_mock = server
        .mock("GET", "/admin/proj0/versions.json")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(versions_json_body_with_project_digest([(
            "0.3.0",
            "[]",
            &advertised_digest,
        )]))
        .expect_at_least(1)
        .create();

    let project_json_mock = server
        .mock("GET", "/admin/proj0/0.3.0/.project.json")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(info_json)
        .expect_at_least(1)
        .create();

    let meta_json_mock = server
        .mock("GET", "/admin/proj0/0.3.0/.meta.json")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(meta_json)
        .expect_at_least(1)
        .create();

    // Pre-download `get_project()` must not reach for the kpar; that check
    // belongs to the post-download canonical path.
    let kpar_mock = expect_untouched(&mut server, "GET", "/admin/proj0/0.3.0/project.kpar");

    let project = env.get_project("pkg:sysand/admin/proj0", "0.3.0")?;
    let (info, meta) = project
        .get_project()
        .expect("non-SHA256 meta should defer digest verification, not fail pre-download");
    assert_eq!(info.as_ref().map(|i| i.name.as_str()), Some("proj0"));
    assert!(meta.is_some());

    versions_mock.assert();
    project_json_mock.assert();
    meta_json_mock.assert();
    kpar_mock.assert();

    Ok(())
}
