// SPDX-FileCopyrightText: © 2026 Sysand contributors <opensource@sensmetry.com>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Clone facade function.
//!
//! `clone` is a complex orchestration: resolve a project by IRI,
//! fetch it to a target directory, then optionally lock + sync deps.
//!
//! The full orchestration involves CLI-specific concerns (directory
//! validation, error recovery, locator parsing). The facade exposes
//! the building blocks — callers compose `lock::update` + `env::sync`
//! after cloning the project files.

#[cfg(feature = "filesystem")]
use crate::error::{ErrorCode, SysandError};
#[cfg(feature = "filesystem")]
use crate::project::ProjectRead;

/// Clone a resolved project's files into a target directory.
///
/// The caller resolves the project first (via a resolver), then passes
/// the resolved `ProjectRead` here to copy files to `target`. This
/// does not handle dependency resolution — use `lock::update` +
/// `env::sync` after cloning for that.
#[cfg(feature = "filesystem")]
pub fn clone_project<P: ProjectRead>(
    source: &P,
    target: &camino::Utf8Path,
) -> Result<(), SysandError>
where
    P::Error: Into<SysandError>,
{
    use crate::project::local_src::LocalSrcProject;

    let mut dest = LocalSrcProject {
        nominal_path: None,
        project_path: target.to_path_buf(),
    };

    crate::env::utils::clone_project(source, &mut dest, true)
        .map(|_| ())
        .map_err(|e| SysandError::new(ErrorCode::IoError, e.to_string()))
}
