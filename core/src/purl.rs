// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: © 2026 Sysand contributors <opensource@sensmetry.com>

//! PURL (Package URL) validation and normalization for `pkg:sysand` IRIs.
//!
//! Sysand uses the `pkg:sysand/<publisher>/<name>` scheme as its canonical
//! project identifier, following the [Package URL specification][purl-spec].
//! This module defines the rules that publisher and name segments must
//! satisfy and provides the normalization function that maps valid
//! human-supplied values to their canonical form.
//!
//! [purl-spec]: https://github.com/package-url/purl-spec

/// Which kind of `pkg:sysand` segment to validate. Publishers disallow dots
/// (they would collide with reverse-DNS-shaped identifiers elsewhere in the
/// toolchain); names permit dots so that dotted product names (e.g.
/// `foo.bar`) are expressible.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FieldKind {
    Publisher,
    Name,
}

impl FieldKind {
    fn dot_is_separator(self) -> bool {
        matches!(self, FieldKind::Name)
    }
}

/// Validates a publisher or name field for `pkg:sysand` project IDs.
///
/// Rules: 3-50 ASCII alphanumeric characters, with single separators (space,
/// hyphen, and — for `FieldKind::Name` — dot) allowed between words. Must
/// start and end with an alphanumeric character.
pub fn is_valid_field(s: &str, kind: FieldKind) -> bool {
    if !s.is_ascii() {
        return false;
    }
    let bytes = s.as_bytes();

    if !(3..=50).contains(&bytes.len()) {
        return false;
    }

    if !bytes[0].is_ascii_alphanumeric() || !bytes[bytes.len() - 1].is_ascii_alphanumeric() {
        return false;
    }

    for i in 1..(bytes.len() - 1) {
        let b = bytes[i];

        if b.is_ascii_alphanumeric() {
            continue;
        }

        let is_separator = b == b'-' || b == b' ' || (kind.dot_is_separator() && b == b'.');
        if !is_separator {
            return false;
        }

        // only isolated separators — knowing first/last is alphanumeric,
        // this is sufficient
        if !bytes[i - 1].is_ascii_alphanumeric() {
            return false;
        }
    }

    true
}

/// Whether `s` is a valid publisher segment.
pub fn is_valid_publisher(s: &str) -> bool {
    is_valid_field(s, FieldKind::Publisher)
}

/// Whether `s` is a valid project name segment.
pub fn is_valid_name(s: &str) -> bool {
    is_valid_field(s, FieldKind::Name)
}

/// Canonicalizes a publisher or name by lowercasing ASCII and replacing spaces
/// with hyphens. The result is what ends up embedded in a `pkg:sysand` IRI;
/// callers should validate with [`is_valid_field`] before or after calling.
pub fn normalize_field(s: &str) -> String {
    s.to_ascii_lowercase().replace(' ', "-")
}

/// Whether `s` is a valid and *already-normalized* publisher or name segment
/// for a `pkg:sysand` IRI. This combines validation with a normalization
/// check — the IRI must have been through `normalize_field` before being
/// stored; a non-normalized IRI (uppercase, spaces) is rejected even if
/// valid pre-normalization.
pub fn is_normalized_field(s: &str, kind: FieldKind) -> bool {
    is_valid_field(s, kind) && normalize_field(s) == s
}

#[cfg(test)]
#[path = "./purl_tests.rs"]
mod tests;
