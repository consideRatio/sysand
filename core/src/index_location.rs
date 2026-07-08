// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: © 2026 Sysand contributors <opensource@sensmetry.com>

//! User-configurable index locations: either a plain base URL that relative
//! index paths are appended to (the historical behavior), or a URL template
//! containing a `{path}` placeholder that the relative index path is
//! substituted into, percent-encoded as a single path segment. Templates
//! make it possible to reach indexes served through file-access APIs whose
//! URL structure is not "base + path", such as the GitLab repository files
//! API (`.../repository/files/{path}/raw?ref=<branch>`).
//!
//! Two placeholder spellings are supported, differing only in how `/` is
//! treated when the relative index path (for example
//! `some-publisher/some-project/1.0.0/project.kpar`) is substituted:
//!
//! - `{path}` — every byte outside RFC 3986 *unreserved* is
//!   percent-encoded, including `/` as `%2F`. This is what the GitLab
//!   repository files API expects.
//! - `{path_raw}` — `/` stays literal; each path segment is
//!   percent-encoded individually. For hosts that take the file path as
//!   ordinary URL path segments but need a suffix after it (for example
//!   Gitea's `.../raw/<path>?ref=<branch>`).
//!
//! The syntax is a deliberately restricted subset of [RFC 6570] URI
//! Templates: `{path}` behaves like RFC 6570 simple string expansion of
//! a single value and `{path_raw}` like reserved expansion (`{+path}`),
//! but only these two fixed names are accepted so that typos fail at
//! parse time instead of producing wrong URLs. Fixed custom markers in a
//! package-index template follow crates.io's `config.json` `dl` field
//! (`{crate}`, `{version}`).
//!
//! [RFC 6570]: https://www.rfc-editor.org/rfc/rfc6570

use core::fmt;
use core::str::FromStr;

use percent_encoding::{AsciiSet, NON_ALPHANUMERIC, utf8_percent_encode};
use thiserror::Error;

/// Placeholder that expands to the fully percent-encoded relative index
/// path (`/` becomes `%2F`).
const PATH_PLACEHOLDER: &str = "{path}";

/// Placeholder that expands to the relative index path with literal `/`
/// separators (each segment percent-encoded individually).
const PATH_RAW_PLACEHOLDER: &str = "{path_raw}";

/// Percent-encode everything outside RFC 3986 unreserved
/// (`A-Z a-z 0-9 - . _ ~`). Notably `/` becomes `%2F`, which is what
/// file-access APIs like GitLab's expect for a path used as one URL
/// segment.
const PATH_ENCODE_SET: &AsciiSet = &NON_ALPHANUMERIC
    .remove(b'-')
    .remove(b'.')
    .remove(b'_')
    .remove(b'~');

#[derive(Error, Debug)]
pub enum IndexLocationError {
    #[error("invalid index URL `{url}`")]
    InvalidUrl {
        url: Box<str>,
        #[source]
        source: url::ParseError,
    },
    #[error(
        "index URL `{url}` is relative; \
         index locations must be absolute HTTP(S) URLs"
    )]
    RelativeUrl { url: Box<str> },
    #[error(
        "index URL `{url}` contains `%7B`/`%7D` (percent-encoded braces) — \
         did you mean a `{{path}}` template? Placeholders must be written \
         literally, e.g. `.../repository/files/{{path}}/raw?ref=main`"
    )]
    PreEncodedPlaceholder { url: Box<str> },
    #[error(
        "index URL template `{url}` contains unknown placeholder `{placeholder}`{hint}; \
         supported placeholders: `{{path}}` (percent-encoded, `/` becomes `%2F`) and \
         `{{path_raw}}` (literal `/` separators)"
    )]
    UnknownPlaceholder {
        url: Box<str>,
        placeholder: Box<str>,
        hint: &'static str,
    },
    #[error(
        "index URL template `{url}` contains a stray `{{` or `}}`; \
         supported placeholders are `{{path}}` and `{{path_raw}}`"
    )]
    StrayBrace { url: Box<str> },
    #[error(
        "index URL template `{url}` must contain a `{{path}}` or `{{path_raw}}` placeholder \
         exactly once, found {count} occurrences"
    )]
    PlaceholderCount { url: Box<str>, count: usize },
    #[error(
        "index URL template `{url}` expands to an invalid URL; note that a \
         placeholder may only appear in the path or query of an absolute \
         HTTP(S) URL"
    )]
    InvalidTemplate {
        url: Box<str>,
        #[source]
        source: url::ParseError,
    },
    #[error("index URL template `{url}` is relative; templates must be absolute HTTP(S) URLs")]
    RelativeTemplate { url: Box<str> },
    #[error("index URL `{url}` uses scheme `{scheme}`; only `http` and `https` are supported")]
    UnsupportedScheme { url: Box<str>, scheme: Box<str> },
    #[error("index URL `{url}` includes username or password; URL userinfo is not allowed")]
    Userinfo { url: Box<str> },
    #[error(
        "index URL template `{url}` includes a `#` fragment; \
         fragments are never sent to the server and are not allowed in templates"
    )]
    Fragment { url: Box<str> },
}

/// Failure to construct a concrete file URL from an [`IndexLocation`] and a
/// relative index path.
#[derive(Error, Debug)]
pub enum ResolveUrlError {
    #[error("cannot construct URL from `{root}` and `{path}`")]
    Join {
        root: Box<str>,
        path: Box<str>,
        #[source]
        source: url::ParseError,
    },
    #[error("expanding index URL template `{template}` with `{path}` produced an invalid URL")]
    Expand {
        template: Box<str>,
        path: Box<str>,
        #[source]
        source: url::ParseError,
    },
    #[error("index URL `{root}` cannot serve as a base for relative paths")]
    NotABase { root: Box<str> },
}

/// How a relative index path is encoded when substituted into a template
/// placeholder.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PathEncoding {
    /// `{path}`: the whole path is one percent-encoded unit, `/` → `%2F`.
    Encoded,
    /// `{path_raw}`: `/` stays literal, each segment percent-encoded.
    Raw,
}

impl PathEncoding {
    fn placeholder(self) -> &'static str {
        match self {
            Self::Encoded => PATH_PLACEHOLDER,
            Self::Raw => PATH_RAW_PLACEHOLDER,
        }
    }

    fn encode(self, rel_path: &str) -> String {
        match self {
            Self::Encoded => utf8_percent_encode(rel_path, PATH_ENCODE_SET).to_string(),
            Self::Raw => {
                let mut encoded = String::with_capacity(rel_path.len());
                for (i, segment) in rel_path.split('/').enumerate() {
                    if i > 0 {
                        encoded.push('/');
                    }
                    encoded.extend(utf8_percent_encode(segment, PATH_ENCODE_SET));
                }
                encoded
            }
        }
    }
}

/// A URL template with exactly one `{path}` or `{path_raw}` placeholder,
/// validated at construction. Carried as a string because `url::Url`
/// percent-encodes `{`/`}` on parse, which would corrupt the placeholder.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IndexUrlTemplate {
    raw: String,
    encoding: PathEncoding,
}

impl IndexUrlTemplate {
    /// Validate `raw` as an index URL template. `raw` must contain exactly
    /// one `{path}` or `{path_raw}` placeholder, no other `{...}` tokens
    /// or stray braces, no fragment, and must expand to an absolute
    /// HTTP(S) URL without userinfo, with the placeholder in the path or
    /// query.
    fn parse(raw: &str) -> Result<Self, IndexLocationError> {
        let boxed_url = || -> Box<str> { raw.into() };

        let encoding = validate_placeholders(raw)?;

        // Fragments are never sent to the server; in a template one is
        // always a mistake (and would swallow the placeholder if it
        // followed `#`).
        if raw.contains('#') {
            return Err(IndexLocationError::Fragment { url: boxed_url() });
        }

        let template = Self {
            raw: raw.to_owned(),
            encoding,
        };

        // Expand a probe that encodes to a percent-escape: hosts and
        // ports cannot contain `%`, so a placeholder in the authority
        // component fails to parse rather than validating silently.
        let expanded = match template.expand("pr obe/probe") {
            Ok(expanded) => expanded,
            Err(ResolveUrlError::Expand {
                source: url::ParseError::RelativeUrlWithoutBase,
                ..
            }) => {
                return Err(IndexLocationError::RelativeTemplate { url: boxed_url() });
            }
            Err(ResolveUrlError::Expand { source, .. }) => {
                return Err(IndexLocationError::InvalidTemplate {
                    url: boxed_url(),
                    source,
                });
            }
            Err(ResolveUrlError::Join { .. } | ResolveUrlError::NotABase { .. }) => {
                unreachable!("BUG: IndexUrlTemplate::expand only returns Expand errors")
            }
        };
        validate_expanded_shape(raw, &expanded)?;

        Ok(template)
    }

    /// Substitute `rel_path`, encoded according to the placeholder
    /// spelling, into the placeholder and parse the result.
    pub fn expand(&self, rel_path: &str) -> Result<url::Url, ResolveUrlError> {
        let encoded = self.encoding.encode(rel_path);
        let expanded = self.raw.replacen(self.encoding.placeholder(), &encoded, 1);
        url::Url::parse(&expanded).map_err(|source| ResolveUrlError::Expand {
            template: self.raw.as_str().into(),
            path: rel_path.into(),
            source,
        })
    }

    /// The template string as configured by the user.
    pub fn as_str(&self) -> &str {
        &self.raw
    }
}

impl fmt::Display for IndexUrlTemplate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.raw)
    }
}

/// Where an index lives: a plain base URL (relative index paths are
/// RFC 3986-joined onto it, the historical behavior) or a URL template
/// (relative index paths are percent-encoded into its `{path}`
/// placeholder).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IndexLocation {
    /// A plain base URL. Constructed through [`Self::parse`], its path is
    /// normalized to end with `/` so that relative index paths append
    /// rather than replace the last segment; direct constructions must
    /// uphold the same invariant.
    Root(url::Url),
    Template(IndexUrlTemplate),
}

impl IndexLocation {
    /// Parse a user-supplied index location string. Strings containing `{`
    /// or `}` are treated as URL templates; anything else is parsed as a
    /// plain base URL. Braces are outside the URI character set
    /// ([RFC 3986 §2]), so no conformant URL is misclassified. (The `url`
    /// crate previously tolerated literal braces by percent-encoding them
    /// in paths and keeping them in queries; such non-conformant index
    /// URLs are now template-syntax errors.)
    ///
    /// Plain base URLs are deliberately not shape-validated here (unlike
    /// templates): index resolvers are constructed lazily so that a
    /// configured-but-unused index never fails unrelated commands, and
    /// scheme/userinfo checks happen at discovery time as before.
    ///
    /// [RFC 3986 §2]: https://www.rfc-editor.org/rfc/rfc3986#section-2
    pub fn parse(s: &str) -> Result<Self, IndexLocationError> {
        if is_template_syntax(s) {
            return Ok(Self::Template(IndexUrlTemplate::parse(s)?));
        }
        // A pre-encoded `%7Bpath%7D` would silently behave as a plain
        // append-mode URL and 404 on every fetch; catch the paste
        // accident early instead. Only the exact placeholder spellings
        // are rejected, so URLs that legitimately contain encoded braces
        // elsewhere keep working. Gated on `%` so the common case does
        // not allocate.
        if s.contains('%') {
            let lower = s.to_ascii_lowercase();
            if lower.contains("%7bpath%7d") || lower.contains("%7bpath_raw%7d") {
                return Err(IndexLocationError::PreEncodedPlaceholder { url: s.into() });
            }
        }
        match url::Url::parse(s) {
            // Normalize the trailing slash once here so `resolve` can
            // join without cloning. Non-hierarchical URLs (`mailto:`)
            // cannot be normalized and are rejected later by `resolve`.
            Ok(url) if !url.cannot_be_a_base() => Ok(Self::Root(with_trailing_slash(url))),
            Ok(url) => Ok(Self::Root(url)),
            Err(url::ParseError::RelativeUrlWithoutBase) => {
                Err(IndexLocationError::RelativeUrl { url: s.into() })
            }
            Err(source) => Err(IndexLocationError::InvalidUrl {
                url: s.into(),
                source,
            }),
        }
    }

    /// Construct the URL for the index file at `rel_path` (a relative path
    /// such as `index.json` or `<publisher>/<name>/versions.json`).
    /// `rel_path` is trusted: callers pass paths composed from validated
    /// components (charset-checked publisher/name, `_iri/<sha256hex>`,
    /// parsed semver versions, fixed file names — see
    /// `design/index-protocol.md` §5), so it cannot smuggle `..` segments
    /// or URL syntax into the result.
    pub fn resolve(&self, rel_path: &str) -> Result<url::Url, ResolveUrlError> {
        match self {
            Self::Root(root) => {
                // Non-hierarchical schemes (`mailto:`, `data:`) parse as
                // plain URLs but cannot anchor relative paths; error
                // rather than panic in `with_trailing_slash`.
                if root.cannot_be_a_base() {
                    return Err(ResolveUrlError::NotABase {
                        root: root.as_str().into(),
                    });
                }
                root.join(rel_path).map_err(|source| ResolveUrlError::Join {
                    root: root.as_str().into(),
                    path: rel_path.into(),
                    source,
                })
            }
            Self::Template(template) => template.expand(rel_path),
        }
    }

    /// The plain base URL, if this location is not a template.
    pub fn as_root(&self) -> Option<&url::Url> {
        match self {
            Self::Root(url) => Some(url),
            Self::Template(_) => None,
        }
    }
}

impl fmt::Display for IndexLocation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Root(url) => f.write_str(url.as_str()),
            Self::Template(template) => f.write_str(template.as_str()),
        }
    }
}

impl FromStr for IndexLocation {
    type Err = IndexLocationError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s)
    }
}

/// Check that every `{...}` token in `raw` is `{path}` or `{path_raw}`,
/// that braces are balanced and non-nested, and that exactly one
/// placeholder occurs. Returns the encoding the placeholder selects.
fn validate_placeholders(raw: &str) -> Result<PathEncoding, IndexLocationError> {
    let boxed_url = || -> Box<str> { raw.into() };
    let mut found: Vec<PathEncoding> = Vec::new();
    let mut rest = raw;
    while let Some(open) = rest.find(['{', '}']) {
        if rest.as_bytes()[open] == b'}' {
            return Err(IndexLocationError::StrayBrace { url: boxed_url() });
        }
        let after_open = &rest[open + 1..];
        let Some(close) = after_open.find(['{', '}']) else {
            return Err(IndexLocationError::StrayBrace { url: boxed_url() });
        };
        if after_open.as_bytes()[close] == b'{' {
            return Err(IndexLocationError::StrayBrace { url: boxed_url() });
        }
        let name = &after_open[..close];
        match name {
            "path" => found.push(PathEncoding::Encoded),
            "path_raw" => found.push(PathEncoding::Raw),
            _ => {
                let hint = if name.eq_ignore_ascii_case("path") {
                    " (did you mean `{path}`?)"
                } else if name.eq_ignore_ascii_case("path_raw") {
                    " (did you mean `{path_raw}`?)"
                } else {
                    ""
                };
                return Err(IndexLocationError::UnknownPlaceholder {
                    url: boxed_url(),
                    placeholder: format!("{{{name}}}").into(),
                    hint,
                });
            }
        }
        rest = &after_open[close + 1..];
    }
    match found.as_slice() {
        [encoding] => Ok(*encoding),
        _ => Err(IndexLocationError::PlaceholderCount {
            url: boxed_url(),
            count: found.len(),
        }),
    }
}

fn validate_expanded_shape(raw: &str, expanded: &url::Url) -> Result<(), IndexLocationError> {
    if !matches!(expanded.scheme(), "http" | "https") {
        return Err(IndexLocationError::UnsupportedScheme {
            url: raw.into(),
            scheme: expanded.scheme().into(),
        });
    }
    if !expanded.username().is_empty() || expanded.password().is_some() {
        return Err(IndexLocationError::Userinfo { url: raw.into() });
    }
    Ok(())
}

/// Whether `s` uses index URL template syntax (contains a brace). Owned
/// here so parsing and the discovery-document field dispatch cannot
/// diverge.
pub(crate) fn is_template_syntax(s: &str) -> bool {
    s.contains(['{', '}'])
}

/// Return `url` with a guaranteed trailing slash on its path so that
/// `Url::join` treats it as a directory. Operates via `path_segments_mut`
/// rather than touching the serialized path string, so percent-encoded
/// segments survive the round-trip unchanged.
///
/// Callers must pass an HTTP(S) URL. Such URLs can be a base, so the
/// `path_segments_mut` call should not fail after endpoint-shape
/// validation.
pub(crate) fn with_trailing_slash(mut url: url::Url) -> url::Url {
    {
        let mut segments = url
            .path_segments_mut()
            .expect("caller passes a URL that can be a base");
        segments.pop_if_empty();
        segments.push("");
    }
    url
}

#[cfg(test)]
#[path = "./index_location_tests.rs"]
mod tests;
