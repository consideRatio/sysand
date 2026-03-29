// SPDX-FileCopyrightText: © 2026 Sysand contributors <opensource@sensmetry.com>
// SPDX-License-Identifier: MIT OR Apache-2.0

use camino::Utf8PathBuf;

use crate::error::{ErrorCode, SysandError};

/// Find the project root by walking up from the given path.
///
/// Returns the absolute path to the directory containing `.project.json`.
/// Errors if no project root is found.
pub fn locate(path: &camino::Utf8Path) -> Result<Utf8PathBuf, SysandError> {
    crate::commands::root::do_root(path)
        .map_err(|e| SysandError::from(e))?
        .ok_or_else(|| SysandError::new(ErrorCode::ProjectNotFound, "no project found in parent directories"))
}
