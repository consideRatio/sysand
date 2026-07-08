// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: © 2026 Sysand contributors <opensource@sensmetry.com>

//! Discovery of `index_root` and `api_root` via
//! `<discovery-root>/sysand-index-config.json`. The wire contract lives in
//! `design/index-protocol.md`; this module implements the client-side fetch
//! and URL-shape validation.
//!
//! The underlying `reqwest` middleware applies its default redirect policy
//! to the discovery fetch (see the comment next to
//! [`crate::resolve::net_utils::create_reqwest_client`]).

use serde::Deserialize;
use thiserror::Error;

use crate::{
    auth::HTTPAuthentication,
    env::index::{HttpFetchError, IndexEnvironmentError, MissingPolicy, fetch_json},
    index::iri::parse_iri,
    index_location::{IndexLocation, IndexLocationError, is_template_syntax, with_trailing_slash},
};

const INDEX_PATH: &str = "index.json";
const VERSIONS_PATH: &str = "versions.json";
const KPAR_FILE: &str = "project.kpar";
const PROJECT_JSON_FILE: &str = ".project.json";
const META_JSON_FILE: &str = ".meta.json";

/// Resolved view of a sysand index server's two roots, as produced by the
/// discovery step.
#[derive(Debug, Clone)]
pub struct ResolvedEndpoints {
    /// Location of the sysand index (where `index.json` lives): a plain
    /// base URL or a `{path}` URL template.
    pub index_root: IndexLocation,
    /// Base URL of the sysand index API (where `v1/upload` lives). `None`
    /// when the discovery root is a URL template and the discovery
    /// document does not supply an `api_root` — such an index is
    /// read-only from this client's point of view.
    pub api_root: Option<url::Url>,
}

impl ResolvedEndpoints {
    /// Build a `ResolvedEndpoints` that routes index traffic (and, for a
    /// plain-URL discovery root, API traffic) at the discovery root
    /// itself. Used when the discovery document is absent (HTTP 404).
    pub fn flat(discovery_root: IndexLocation) -> Self {
        let api_root = discovery_root.as_root().cloned();
        Self {
            index_root: discovery_root,
            api_root,
        }
    }

    fn resolve(&self, rel_path: &str) -> Result<url::Url, IndexEnvironmentError> {
        self.index_root
            .resolve(rel_path)
            .map_err(IndexEnvironmentError::ResolveUrl)
    }

    pub(crate) fn index_url(&self) -> Result<url::Url, IndexEnvironmentError> {
        self.resolve(INDEX_PATH)
    }

    /// Relative index path of the project directory for `iri`, without a
    /// trailing slash (`<publisher>/<name>` or `_iri/<sha256hex>`).
    fn project_rel_path<S: AsRef<str>>(iri: S) -> Result<String, IndexEnvironmentError> {
        Ok(parse_iri(iri.as_ref())?.get_path())
    }

    pub(crate) fn kpar_url<S: AsRef<str>, T: AsRef<str>>(
        &self,
        iri: S,
        version: T,
    ) -> Result<url::Url, IndexEnvironmentError> {
        let project = Self::project_rel_path(iri)?;
        self.resolve(&format!("{project}/{}/{KPAR_FILE}", version.as_ref()))
    }

    pub(crate) fn project_json_url<S: AsRef<str>, T: AsRef<str>>(
        &self,
        iri: S,
        version: T,
    ) -> Result<url::Url, IndexEnvironmentError> {
        let project = Self::project_rel_path(iri)?;
        self.resolve(&format!(
            "{project}/{}/{PROJECT_JSON_FILE}",
            version.as_ref()
        ))
    }

    pub(crate) fn meta_json_url<S: AsRef<str>, T: AsRef<str>>(
        &self,
        iri: S,
        version: T,
    ) -> Result<url::Url, IndexEnvironmentError> {
        let project = Self::project_rel_path(iri)?;
        self.resolve(&format!("{project}/{}/{META_JSON_FILE}", version.as_ref()))
    }

    pub(crate) fn versions_url<S: AsRef<str>>(
        &self,
        iri: S,
    ) -> Result<url::Url, IndexEnvironmentError> {
        let project = Self::project_rel_path(iri)?;
        self.resolve(&format!("{project}/{VERSIONS_PATH}"))
    }
}

#[derive(Debug, Deserialize)]
struct IndexConfigRaw {
    #[serde(default)]
    index_root: Option<String>,
    #[serde(default)]
    api_root: Option<String>,
}

/// Errors that can occur during the discovery step. Surface as
/// [`crate::env::index::IndexEnvironmentError::Discovery`] at the env
/// boundary.
#[derive(Error, Debug)]
pub enum DiscoveryError {
    #[error(transparent)]
    Fetch(#[from] HttpFetchError),
    #[error(
        "discovery document at `{url}` supplied a relative URL `{value}` for `{field}`; \
         absolute HTTP(S) URLs are required"
    )]
    RelativeUrl {
        url: Box<str>,
        field: &'static str,
        value: String,
    },
    #[error(
        "discovery document at `{url}` supplied an invalid URL `{value}` for `{field}`: {source}"
    )]
    InvalidUrl {
        url: Box<str>,
        field: &'static str,
        value: String,
        #[source]
        source: url::ParseError,
    },
    #[error(
        "discovery document at `{url}` supplied a non-HTTP(S) URL `{value}` for `{field}`; \
         only `http` and `https` are supported"
    )]
    UnsupportedScheme {
        url: Box<str>,
        field: &'static str,
        value: String,
    },
    #[error(
        "discovery document at `{url}` supplied URL userinfo in `{value}` for `{field}`; \
         username and password are not allowed"
    )]
    Userinfo {
        url: Box<str>,
        field: &'static str,
        value: String,
    },
    #[error("discovery document at `{url}` supplied an invalid `{field}`")]
    InvalidLocation {
        url: Box<str>,
        field: &'static str,
        #[source]
        source: IndexLocationError,
    },
    #[error("index location `{location}` is not a valid discovery root")]
    InvalidDiscoveryRoot {
        location: Box<str>,
        #[source]
        source: crate::index_location::ResolveUrlError,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum HttpBaseUrlShapeError {
    UnsupportedScheme,
    Userinfo,
}

pub(crate) fn validate_http_base_url_shape(url: &url::Url) -> Result<(), HttpBaseUrlShapeError> {
    if !matches!(url.scheme(), "http" | "https") {
        return Err(HttpBaseUrlShapeError::UnsupportedScheme);
    }
    if !url.username().is_empty() || url.password().is_some() {
        return Err(HttpBaseUrlShapeError::Userinfo);
    }
    Ok(())
}

fn discovery_shape_error(
    source_url: &url::Url,
    field: &'static str,
    value: &url::Url,
    error: HttpBaseUrlShapeError,
) -> DiscoveryError {
    match error {
        HttpBaseUrlShapeError::UnsupportedScheme => DiscoveryError::UnsupportedScheme {
            url: source_url.as_str().into(),
            field,
            value: value.as_str().to_owned(),
        },
        HttpBaseUrlShapeError::Userinfo => DiscoveryError::Userinfo {
            url: source_url.as_str().into(),
            field,
            value: value.as_str().to_owned(),
        },
    }
}

/// Fetch the discovery document from
/// `<discovery_root>/sysand-index-config.json` and produce the resolved
/// `(index_root, api_root)` pair. See module docs for the protocol-level
/// semantics.
pub async fn fetch_index_config<P: HTTPAuthentication>(
    client: &reqwest_middleware::ClientWithMiddleware,
    auth: &P,
    discovery_root: &IndexLocation,
) -> Result<ResolvedEndpoints, DiscoveryError> {
    // Normalize a plain discovery root so relative-path resolution treats
    // it as a directory. Validate its shape before issuing any request;
    // templates were already shape-validated when parsed.
    let discovery_location = match discovery_root {
        IndexLocation::Root(root) => {
            validate_http_base_url_shape(root)
                .map_err(|error| discovery_shape_error(root, "<discovery_root>", root, error))?;
            IndexLocation::Root(with_trailing_slash(root.clone()))
        }
        template @ IndexLocation::Template(_) => template.clone(),
    };

    let config_url = discovery_location
        .resolve("sysand-index-config.json")
        .map_err(|source| DiscoveryError::InvalidDiscoveryRoot {
            location: discovery_location.to_string().into(),
            source,
        })?;

    let parsed: Option<IndexConfigRaw> =
        fetch_json(client, auth, &config_url, MissingPolicy::AllowNotFound).await?;

    let Some(raw) = parsed else {
        let endpoints = ResolvedEndpoints::flat(discovery_location);
        log_resolved(&endpoints);
        return Ok(endpoints);
    };

    // Parse a supplied field value as a plain base URL.
    // `url::Url::parse` on a relative input (e.g. `"/index/"`) returns
    // `Err(RelativeUrlWithoutBase)` — map that specifically to
    // `RelativeUrl` so the error is actionable.
    let parse_base_url = |field: &'static str, s: String| -> Result<url::Url, DiscoveryError> {
        let parsed = match url::Url::parse(&s) {
            Ok(parsed) => parsed,
            Err(url::ParseError::RelativeUrlWithoutBase) => {
                return Err(DiscoveryError::RelativeUrl {
                    url: config_url.as_str().into(),
                    field,
                    value: s,
                });
            }
            Err(source) => {
                return Err(DiscoveryError::InvalidUrl {
                    url: config_url.as_str().into(),
                    field,
                    value: s,
                    source,
                });
            }
        };
        validate_http_base_url_shape(&parsed)
            .map_err(|error| discovery_shape_error(&config_url, field, &parsed, error))?;
        Ok(with_trailing_slash(parsed))
    };

    // `index_root` may itself be a URL template; `api_root` may not
    // (uploads are not file fetches, so templating it is meaningless).
    let index_root = match raw.index_root {
        None => discovery_location.clone(),
        Some(s) if is_template_syntax(&s) => {
            IndexLocation::parse(&s).map_err(|source| DiscoveryError::InvalidLocation {
                url: config_url.as_str().into(),
                field: "index_root",
                source,
            })?
        }
        Some(s) => IndexLocation::Root(parse_base_url("index_root", s)?),
    };

    let api_root = match (raw.api_root, discovery_location) {
        (Some(s), _) => Some(parse_base_url("api_root", s)?),
        (None, IndexLocation::Root(directory_root)) => Some(directory_root),
        (None, IndexLocation::Template(_)) => None,
    };

    let endpoints = ResolvedEndpoints {
        index_root,
        api_root,
    };
    log_resolved(&endpoints);
    Ok(endpoints)
}

/// Discovery decides where every later index fetch goes, so record the
/// outcome for debugging.
fn log_resolved(endpoints: &ResolvedEndpoints) {
    match &endpoints.api_root {
        Some(api_root) => log::debug!(
            "resolved index endpoints: index_root `{}`, api_root `{api_root}`",
            endpoints.index_root
        ),
        None => log::debug!(
            "resolved index endpoints: index_root `{}`, no api_root (read-only index)",
            endpoints.index_root
        ),
    }
}

#[cfg(test)]
#[path = "./discovery_tests.rs"]
mod tests;
