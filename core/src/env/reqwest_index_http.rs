// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: © 2026 Sysand contributors <opensource@sensmetry.com>

//! HTTP client for the sysand index protocol.
//!
//! This environment reads `index.json` for IRI enumeration and
//! per-project `versions.json` for candidate enumeration. A `versions.json`
//! entry carries the data needed to decide which version to lock plus the
//! artifact metadata needed for lockfile population and archive verification.
//!
//! Once a concrete version has been selected, the returned project wrapper
//! lazily fetches that version's real `.project.json` and `.meta.json` to
//! materialize the locked project record, while continuing to defer the kpar
//! download itself until archive contents are actually needed.

use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use semver::Version;
use serde::Deserialize;
use sha2::Sha256;
use thiserror::Error;
use tokio::sync::OnceCell;

use crate::{
    auth::{HTTPAuthentication, StandardHTTPAuthentication},
    env::{AsSyncEnvironmentTokio, ReadEnvironmentAsync, segment_uri_generic},
    model::InterchangeProjectUsageRaw,
    project::indexed_remote::{IndexedRemoteProject, IndexedRemoteProjectError},
    resolve::net_utils::json_get_request,
};

/// Blocking wrapper around [`HTTPIndexEnvironmentAsync`] that drives the
/// async implementation on a Tokio runtime. Use this from synchronous call
/// sites (e.g. the CLI) where an `Environment`/`ReadEnvironment` is required;
/// all real HTTP work happens on the wrapped async implementation. The type
/// parameter is fixed to [`StandardHTTPAuthentication`] — construct the
/// async form directly if a custom auth policy is needed.
pub type HTTPIndexEnvironment =
    AsSyncEnvironmentTokio<HTTPIndexEnvironmentAsync<StandardHTTPAuthentication>>;

/// Async HTTP client for the sysand index protocol. This is the
/// authoritative implementation; [`HTTPIndexEnvironment`] is just a
/// blocking wrapper around it.
///
/// Resolves IRIs as follows:
///
/// - `pkg:sysand/<publisher>/<name>` (two valid, normalized segments) ->
///   `<publisher>/<name>/` under `base_url`.
/// - Any other IRI -> `_iri/<sha256_hex(iri)>/` under `base_url`. A
///   non-normalized `pkg:sysand` IRI falls through to this branch so that
///   malformed or non-canonical IRIs in a dependency tree cannot traverse
///   out of the configured index base.
///
/// A per-IRI `versions.json` document holds the advertised versions plus
/// the five per-entry fields (`version`, `usage`, `project_digest`,
/// `kpar_size`, `kpar_digest`) needed to enumerate candidates and verify
/// later-materialized archives without downloading anything heavier.
/// Fetched documents are validated (semver + digest shape) and cached in
/// `versions_cache` so concurrent solver paths share a single fetch.
#[derive(Debug)]
pub struct HTTPIndexEnvironmentAsync<Policy> {
    pub(crate) client: reqwest_middleware::ClientWithMiddleware,
    pub(crate) auth_policy: Arc<Policy>,
    pub(crate) base_url: reqwest::Url,
    /// Intra-run cache of parsed `versions.json` documents, keyed by IRI.
    /// Avoids the duplicate fetches that otherwise occur because
    /// `versions_async` and `get_project_async` each independently hit the
    /// endpoint (and `get_project_async` is called once per candidate during
    /// solving). The cache is scoped to one env lifetime and never invalidates
    /// — there is no freshness signal in the protocol yet, and a single
    /// `sysand lock` run should see a stable view of the index.
    ///
    /// Each entry is a per-IRI `OnceCell`, so concurrent callers requesting
    /// the same IRI share a single fetch even under a parallel solver. A
    /// cached `None` (project missing) is stored just like a cached hit, so
    /// 404s are not re-queried within one env lifetime either.
    ///
    /// Cached entries are **validated** on ingest (semver + digest shape +
    /// uniqueness). The wire order is preserved verbatim — the server is
    /// contractually required to emit entries newest-first by parsed
    /// `semver::Version`, and downstream code relies on that without
    /// re-sorting.
    pub(crate) versions_cache: Mutex<HashMap<String, VersionsCacheEntry>>,
}

/// Per-IRI cache slot: a `OnceCell` shared by all concurrent callers
/// requesting the same IRI, holding either the validated `versions.json`
/// entries (`Some`) or a "project missing" marker (`None`). Cached entries
/// are already validated and normalized (see [`AdvertisedVersion`]); the
/// raw wire format is not retained.
pub(crate) type VersionsCacheEntry = Arc<OnceCell<Option<Arc<Vec<AdvertisedVersion>>>>>;

/// A validated sha256 hex digest — 64 lowercase hex characters, with the
/// `"sha256:"` prefix already stripped. Constructed via `TryFrom<&str>`
/// which performs the only digest validation pass in this module: both
/// ingest (`validate_versions`) and every downstream site use the
/// result of that parse, so there is no second "is this hex?" check hiding
/// at the point of use.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Sha256HexDigest(String);

impl Sha256HexDigest {
    pub(crate) fn as_hex(&self) -> &str {
        &self.0
    }
}

impl TryFrom<&str> for Sha256HexDigest {
    type Error = ();

    fn try_from(raw: &str) -> Result<Self, Self::Error> {
        let hex = raw.strip_prefix("sha256:").ok_or(())?;
        if hex.len() != 64 || !hex.bytes().all(|c| c.is_ascii_hexdigit()) {
            return Err(());
        }
        Ok(Sha256HexDigest(hex.to_ascii_lowercase()))
    }
}

/// A `versions.json` entry after ingest-time validation: `version` is parsed
/// into `semver::Version`, digests are validated lowercase hex without the
/// wire prefix, and `usage`/`kpar_size` are carried through verbatim. This
/// is the representation the cache stores and the rest of the crate sees —
/// the raw [`VersionEntry`] is only alive briefly during deserialization.
#[derive(Debug, Clone)]
pub(crate) struct AdvertisedVersion {
    pub(crate) version: semver::Version,
    pub(crate) usage: Vec<InterchangeProjectUsageRaw>,
    pub(crate) project_digest: Sha256HexDigest,
    pub(crate) kpar_size: u64,
    pub(crate) kpar_digest: Sha256HexDigest,
}

#[derive(Error, Debug)]
pub enum HTTPIndexEnvironmentError {
    #[error("failed to extend URL `{0}` with path `{1}`: {2}")]
    JoinURL(Box<str>, String, url::ParseError),
    #[error(transparent)]
    Fetch(#[from] HttpFetchError),
    #[error(
        "versions.json at `{url}` has entry for version `{version}` with \
         malformed `{field}` = `{value}` (expected `sha256:<64-hex>`)"
    )]
    InvalidVersionEntry {
        url: Box<str>,
        version: String,
        field: &'static str,
        value: String,
    },
    #[error("versions.json at `{url}` has entry with non-semver version `{value}`: {source}")]
    InvalidSemverVersion {
        url: Box<str>,
        value: String,
        #[source]
        source: semver::Error,
    },
    #[error("versions.json at `{url}` does not list version `{version}`")]
    VersionNotInIndex { url: Box<str>, version: String },
    #[error("versions.json at `{url}` lists version `{version}` more than once")]
    DuplicateVersion { url: Box<str>, version: String },
    #[error(transparent)]
    Project(#[from] Box<IndexedRemoteProjectError>),
}

/// Shared error surface for both env-level and project-level JSON fetches.
/// Introduced to collapse the previously-parallel enums with mismatched
/// casing (`HTTPRequest` vs `HttpRequest`, `HttpIo` vs `ResponseBody`): one
/// `HttpFetchError` represents "something went wrong fetching a JSON doc
/// over HTTP" regardless of which caller issued the request.
#[derive(Error, Debug)]
pub enum HttpFetchError {
    #[error("HTTP request to `{url}` failed: {source}")]
    Request {
        url: Box<str>,
        #[source]
        source: reqwest_middleware::Error,
    },
    #[error("HTTP request to `{url}` returned status {status}")]
    BadHttpStatus {
        url: Box<str>,
        status: reqwest::StatusCode,
    },
    #[error("failed to read HTTP response body from `{url}`: {source}")]
    Body {
        url: Box<str>,
        #[source]
        source: reqwest::Error,
    },
    #[error("failed to parse JSON from `{url}`: {source}")]
    JsonParse {
        url: Box<str>,
        #[source]
        source: serde_json::Error,
    },
}

/// Whether a 404 on the requested URL is a successful "no such document"
/// signal (`AllowNotFound` — e.g. an optional `versions.json`) or a hard
/// error (`RequirePresent` — e.g. the per-version `.project.json` that must
/// exist once a version has been selected). This is the only policy
/// difference between the two callers of [`fetch_json`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MissingPolicy {
    AllowNotFound,
    RequirePresent,
}

/// Fetch and JSON-parse one document from `url` through `client`+`auth`. A
/// 404 returns `Ok(None)` under [`MissingPolicy::AllowNotFound`] and a
/// [`HttpFetchError::BadHttpStatus`] under [`MissingPolicy::RequirePresent`];
/// any other non-success status is always an error.
pub(crate) async fn fetch_json<T: for<'de> serde::Deserialize<'de>, P: HTTPAuthentication>(
    client: &reqwest_middleware::ClientWithMiddleware,
    auth: &P,
    url: &url::Url,
    missing: MissingPolicy,
) -> Result<Option<T>, HttpFetchError> {
    let response = auth
        .with_authentication(client, &json_get_request(url.clone()))
        .await
        .map_err(|source| HttpFetchError::Request {
            url: url.as_str().into(),
            source,
        })?;

    let status = response.status();

    if status == reqwest::StatusCode::NOT_FOUND && missing == MissingPolicy::AllowNotFound {
        return Ok(None);
    }

    if !status.is_success() {
        return Err(HttpFetchError::BadHttpStatus {
            url: url.as_str().into(),
            status,
        });
    }

    let bytes = response
        .bytes()
        .await
        .map_err(|source| HttpFetchError::Body {
            url: url.as_str().into(),
            source,
        })?;

    serde_json::from_slice::<T>(&bytes)
        .map(Some)
        .map_err(|source| HttpFetchError::JsonParse {
            url: url.as_str().into(),
            source,
        })
}

const INDEX_PATH: &str = "index.json";
const VERSIONS_PATH: &str = "versions.json";
const KPAR_FILE: &str = "project.kpar";
const PROJECT_JSON_FILE: &str = ".project.json";
const META_JSON_FILE: &str = ".meta.json";
const IRI_HASH_SEGMENT: &str = "_iri";
const PKG_SYSAND_PREFIX: &str = "pkg:sysand/";

// Note on forward compatibility: none of the types below set
// `#[serde(deny_unknown_fields)]`. Unknown fields are silently ignored so
// servers can add new optional fields without breaking clients.
//
// The protocol currently has no schema-version signal. When a mechanism is
// chosen (URL prefix, media type, or a single in-document field) it should
// be added in one place — not duplicated across documents.

/// Top-level `index.json` — the list of every project IRI the index knows
/// about. Used by `uris_async` for list-all enumeration. Per-project version
/// data lives in `versions.json`.
#[derive(Debug, Deserialize)]
struct IndexJson {
    projects: Vec<IndexProject>,
}

#[derive(Debug, Deserialize)]
struct IndexProject {
    iri: String,
}

/// Per-project `versions.json`. Each entry carries the data needed to decide
/// which version to lock (`version` and `usage`) plus the publish-time
/// artifact metadata (`project_digest`, `kpar_size`, `kpar_digest`) that lets
/// the client populate the lockfile and verify archive integrity without
/// downloading first.
///
/// Protocol contract: `versions.json` is sufficient for candidate
/// enumeration/version selection. Once a specific version has been chosen,
/// the client may fetch that version's `.project.json` and `.meta.json` to
/// materialize the locked project record and reconcile it against the
/// digests advertised here. All five per-entry fields are required by the
/// protocol — a missing field on any entry makes the whole document reject
/// as malformed.
#[derive(Debug, Clone, Deserialize)]
pub(crate) struct VersionsJson {
    versions: Vec<VersionEntry>,
}

#[derive(Debug, Clone, Deserialize)]
struct VersionEntry {
    version: String,
    // `usage` is a required protocol field because `versions.json` is
    // sufficient for candidate enumeration/version selection, so the solver
    // must be able to consume it without fetching `.project.json`.
    usage: Vec<InterchangeProjectUsageRaw>,
    /// Canonical project digest (sha256 over canonicalized info+meta),
    /// used to populate the lockfile checksum without downloading the kpar.
    /// Format: `"sha256:<lowercase-hex>"`.
    project_digest: String,
    /// Byte length of the kpar archive, used by `sources_async` in lieu of
    /// a HEAD request.
    kpar_size: u64,
    /// Digest of the kpar archive bytes, verified against the streamed body
    /// when the archive is downloaded. Format: `"sha256:<lowercase-hex>"`.
    kpar_digest: String,
}

/// Parse `pkg:sysand/<publisher>/<name>` into `(publisher, name)` when the IRI
/// matches exactly that shape (two slash-separated components after the
/// scheme) and both segments are valid, normalized `pkg:sysand` identifiers
/// per [`crate::purl`]. An IRI that fails any of these checks falls through
/// to the `_iri/<sha256>/` route, so a malicious or non-canonical IRI in a
/// dependency tree cannot traverse out of the configured index base.
fn parse_pkg_sysand_iri(iri: &str) -> Option<(&str, &str)> {
    use crate::purl::{FieldKind, is_normalized_field};

    let rest = iri.strip_prefix(PKG_SYSAND_PREFIX)?;
    let parts: Vec<&str> = rest.split('/').collect();
    match parts.as_slice() {
        [publisher, name]
            if is_normalized_field(publisher, FieldKind::Publisher)
                && is_normalized_field(name, FieldKind::Name) =>
        {
            Some((publisher, name))
        }
        _ => None,
    }
}

/// Map an IRI to the index path segments that locate its project directory.
///
/// `pkg:sysand/<publisher>/<name>` resolves under `<publisher>/<name>/`.
/// Any other IRI resolves under `_iri/<sha256_hex(iri)>/`.
fn iri_path_segments(iri: &str) -> Vec<String> {
    if let Some((publisher, name)) = parse_pkg_sysand_iri(iri) {
        return vec![publisher.to_string(), name.to_string()];
    }

    let hash = segment_uri_generic::<_, Sha256>(iri)
        .next()
        .expect("segment_uri_generic always yields one segment");
    vec![IRI_HASH_SEGMENT.to_string(), hash]
}

impl<Policy: HTTPAuthentication> HTTPIndexEnvironmentAsync<Policy> {
    /// Return `base_url` with a guaranteed trailing slash on its path, so that
    /// `Url::join` treats the base as a directory.
    fn root_url(&self) -> url::Url {
        let mut result = self.base_url.clone();

        if result.path().is_empty() {
            result.set_path("/");
        } else if !result.path().ends_with('/') {
            let new_path = format!("{}/", result.path());
            result.set_path(&new_path);
        }

        result
    }

    fn url_join(url: &url::Url, join: &str) -> Result<url::Url, HTTPIndexEnvironmentError> {
        url.join(join)
            .map_err(|e| HTTPIndexEnvironmentError::JoinURL(url.as_str().into(), join.into(), e))
    }

    fn index_url(&self) -> Result<url::Url, HTTPIndexEnvironmentError> {
        Self::url_join(&self.root_url(), INDEX_PATH)
    }

    fn project_url<S: AsRef<str>>(&self, iri: S) -> Result<url::Url, HTTPIndexEnvironmentError> {
        let mut result = self.root_url();
        for mut segment in iri_path_segments(iri.as_ref()) {
            segment.push('/');
            result = Self::url_join(&result, &segment)?;
        }
        Ok(result)
    }

    /// Per-version directory URL ending with a trailing slash, so that
    /// `Url::join` treats it as a directory when composing leaf URLs
    /// (`project.kpar`, `.project.json`, `.meta.json`).
    fn version_dir_url<S: AsRef<str>, T: AsRef<str>>(
        &self,
        iri: S,
        version: T,
    ) -> Result<url::Url, HTTPIndexEnvironmentError> {
        let base = self.project_url(iri)?;
        Self::url_join(&base, &format!("{}/", version.as_ref()))
    }

    fn kpar_url<S: AsRef<str>, T: AsRef<str>>(
        &self,
        iri: S,
        version: T,
    ) -> Result<url::Url, HTTPIndexEnvironmentError> {
        Self::url_join(&self.version_dir_url(iri, version)?, KPAR_FILE)
    }

    fn project_json_url<S: AsRef<str>, T: AsRef<str>>(
        &self,
        iri: S,
        version: T,
    ) -> Result<url::Url, HTTPIndexEnvironmentError> {
        Self::url_join(&self.version_dir_url(iri, version)?, PROJECT_JSON_FILE)
    }

    fn meta_json_url<S: AsRef<str>, T: AsRef<str>>(
        &self,
        iri: S,
        version: T,
    ) -> Result<url::Url, HTTPIndexEnvironmentError> {
        Self::url_join(&self.version_dir_url(iri, version)?, META_JSON_FILE)
    }

    fn versions_url<S: AsRef<str>>(&self, iri: S) -> Result<url::Url, HTTPIndexEnvironmentError> {
        let base = self.project_url(iri)?;
        Self::url_join(&base, VERSIONS_PATH)
    }

    async fn fetch_optional_json<T: for<'de> serde::Deserialize<'de>>(
        &self,
        url: &url::Url,
    ) -> Result<Option<T>, HTTPIndexEnvironmentError> {
        Ok(fetch_json(
            &self.client,
            &*self.auth_policy,
            url,
            MissingPolicy::AllowNotFound,
        )
        .await?)
    }

    async fn fetch_index(&self) -> Result<IndexJson, HTTPIndexEnvironmentError> {
        // `index.json` is the root document that identifies a URL as a sysand
        // index. A 404 means "this URL is not a sysand index" (misconfigured
        // base URL, server down, wrong deployment), not "a working index with
        // no projects". Empty indices serve `{"projects": []}` with 200 OK.
        //
        // Propagate the 404 as a hard error so the resolver chain can give up
        // on this index and try the next source, with a signal that points at
        // the actual problem instead of a later "no resolver resolved the
        // IRI" that hides the misconfiguration.
        let url = self.index_url()?;
        self.fetch_optional_json::<IndexJson>(&url)
            .await?
            .ok_or_else(|| {
                HTTPIndexEnvironmentError::Fetch(HttpFetchError::BadHttpStatus {
                    url: url.as_str().into(),
                    status: reqwest::StatusCode::NOT_FOUND,
                })
            })
    }

    /// Fetch and cache `versions.json` for the given IRI. Within a single env
    /// lifetime the document is fetched at most once per IRI — concurrent
    /// callers share a single in-flight fetch via a per-IRI `OnceCell` and
    /// later callers see the same cached `Option<Arc<...>>` (so both hits and
    /// 404s are deduplicated).
    ///
    /// The cached entries are **validated at ingest**: every `version` is
    /// parsed with `semver::Version` (a parse failure rejects the document
    /// with `InvalidSemverVersion`), both digest fields are shape-checked,
    /// and duplicate versions are rejected. The server is required to emit
    /// entries newest-first, so the cache preserves wire order —
    /// `versions_async` streams entries in that order, and
    /// `get_project_async`'s linear scan is `O(n)` regardless.
    async fn fetch_versions_json<S: AsRef<str>>(
        &self,
        iri: S,
    ) -> Result<Option<Arc<Vec<AdvertisedVersion>>>, HTTPIndexEnvironmentError> {
        let iri_key = iri.as_ref();
        let cell = {
            let mut cache = self
                .versions_cache
                .lock()
                .expect("versions_cache mutex poisoned");
            Arc::clone(
                cache
                    .entry(iri_key.to_string())
                    .or_insert_with(|| Arc::new(OnceCell::new())),
            )
        };

        let cached = cell
            .get_or_try_init(|| async {
                let url = self.versions_url(iri_key)?;
                let parsed = self.fetch_optional_json::<VersionsJson>(&url).await?;
                let validated = match parsed {
                    Some(vs) => Some(Arc::new(validate_versions(&url, vs)?)),
                    None => None,
                };
                Ok::<_, HTTPIndexEnvironmentError>(validated)
            })
            .await?;

        Ok(cached.clone())
    }
}

/// Validate every entry's protocol-required fields and emit the list of
/// `AdvertisedVersion`s that the rest of the crate consumes. Every
/// `version` is semver-parsed; both digest fields (`project_digest`,
/// `kpar_digest`) are parsed into [`Sha256HexDigest`]s. A malformed entry
/// rejects the whole document at fetch/cache time rather than only when
/// that specific version is materialized — the cache is shared across
/// candidate enumeration and lockfile assembly, so catching the protocol
/// violation before the document is cached means all downstream operations
/// see the same "this index is broken" error, not a delayed surprise.
///
/// Order is preserved from the wire verbatim: the server is contractually
/// required to emit entries newest-first by parsed `semver::Version`.
/// `versions_async` streams that order straight through, and pubgrub
/// re-sorts internally during solving, so correctness does not depend on
/// client-side re-sorting.
fn validate_versions(
    url: &url::Url,
    vs: VersionsJson,
) -> Result<Vec<AdvertisedVersion>, HTTPIndexEnvironmentError> {
    let validated: Vec<AdvertisedVersion> =
        vs.versions
            .into_iter()
            .map(|entry| {
                let version = Version::parse(&entry.version).map_err(|source| {
                    HTTPIndexEnvironmentError::InvalidSemverVersion {
                        url: url.as_str().into(),
                        value: entry.version.clone(),
                        source,
                    }
                })?;
                let project_digest = Sha256HexDigest::try_from(entry.project_digest.as_str())
                    .map_err(|_| HTTPIndexEnvironmentError::InvalidVersionEntry {
                        url: url.as_str().into(),
                        version: entry.version.clone(),
                        field: "project_digest",
                        value: entry.project_digest.clone(),
                    })?;
                let kpar_digest =
                    Sha256HexDigest::try_from(entry.kpar_digest.as_str()).map_err(|_| {
                        HTTPIndexEnvironmentError::InvalidVersionEntry {
                            url: url.as_str().into(),
                            version: entry.version.clone(),
                            field: "kpar_digest",
                            value: entry.kpar_digest.clone(),
                        }
                    })?;
                Ok::<_, HTTPIndexEnvironmentError>(AdvertisedVersion {
                    version,
                    usage: entry.usage,
                    project_digest,
                    kpar_size: entry.kpar_size,
                    kpar_digest,
                })
            })
            .collect::<Result<_, _>>()?;
    // Duplicate versions are a server-side protocol violation: "pick the
    // best duplicate" has no principled answer, and letting duplicates
    // reach `resolve_candidates` would leak non-determinism into pubgrub
    // (two indexable candidates with the same semver but potentially
    // different digests). Reject here instead.
    let mut seen = std::collections::HashSet::new();
    for v in &validated {
        if !seen.insert(v.version.clone()) {
            return Err(HTTPIndexEnvironmentError::DuplicateVersion {
                url: url.as_str().into(),
                version: v.version.to_string(),
            });
        }
    }
    Ok(validated)
}

type ResultStream<T> =
    futures::stream::Iter<std::vec::IntoIter<Result<T, HTTPIndexEnvironmentError>>>;

impl<Policy: HTTPAuthentication> ReadEnvironmentAsync for HTTPIndexEnvironmentAsync<Policy> {
    type ReadError = HTTPIndexEnvironmentError;

    type UriStream = ResultStream<String>;

    async fn uris_async(&self) -> Result<Self::UriStream, Self::ReadError> {
        let index = self.fetch_index().await?;
        let items: Vec<Result<String, HTTPIndexEnvironmentError>> =
            index.projects.into_iter().map(|p| Ok(p.iri)).collect();
        Ok(futures::stream::iter(items))
    }

    type VersionStream = ResultStream<String>;

    async fn versions_async<S: AsRef<str>>(
        &self,
        uri: S,
    ) -> Result<Self::VersionStream, Self::ReadError> {
        let versions: Vec<Result<String, HTTPIndexEnvironmentError>> =
            match self.fetch_versions_json(uri.as_ref()).await? {
                Some(vs) => vs.iter().map(|e| Ok(e.version.to_string())).collect(),
                None => vec![],
            };

        Ok(futures::stream::iter(versions))
    }

    type InterchangeProjectRead = IndexedRemoteProject<Policy>;

    async fn get_project_async<S: AsRef<str>, T: AsRef<str>>(
        &self,
        uri: S,
        version: T,
    ) -> Result<Self::InterchangeProjectRead, Self::ReadError> {
        let kpar_url = self.kpar_url(&uri, &version)?;
        let project_json_url = self.project_json_url(&uri, &version)?;
        let meta_json_url = self.meta_json_url(&uri, &version)?;
        let versions_url = self.versions_url(uri.as_ref())?;

        // `versions.json` is the source of truth for version selection: a 404
        // on the document, or a parsed document that doesn't list the
        // requested version, surfaces as a hard error. Once the caller has
        // asked for one concrete version, return a project wrapper seeded with
        // the inline solver data from `versions.json` plus the per-version
        // `.project.json` / `.meta.json` URLs. Those files are fetched lazily
        // only when the selected version is materialized for locking.
        let versions = self
            .fetch_versions_json(uri.as_ref())
            .await?
            .ok_or_else(|| {
                HTTPIndexEnvironmentError::Fetch(HttpFetchError::BadHttpStatus {
                    url: versions_url.as_str().into(),
                    status: reqwest::StatusCode::NOT_FOUND,
                })
            })?;
        // `AdvertisedVersion` stores `version` as parsed `semver::Version`, so
        // compare via its `Display` — the caller's `version` is a free-form
        // string and we only commit to the semver invariant for the cached
        // entries themselves.
        let advertised = versions
            .iter()
            .find(|e| e.version.to_string() == version.as_ref())
            .cloned()
            .ok_or_else(|| HTTPIndexEnvironmentError::VersionNotInIndex {
                url: versions_url.as_str().into(),
                version: version.as_ref().to_string(),
            })?;

        let project = IndexedRemoteProject::new(
            kpar_url,
            project_json_url,
            meta_json_url,
            advertised,
            self.client.clone(),
            self.auth_policy.clone(),
        )
        .map_err(Box::new)?;

        Ok(project)
    }
}

#[cfg(test)]
#[path = "./reqwest_index_http_tests.rs"]
mod tests;
