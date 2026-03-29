// SPDX-FileCopyrightText: © 2026 Sysand contributors <opensource@sensmetry.com>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Workspace facade functions.

use camino::Utf8PathBuf;

use crate::error::{ErrorCode, SysandError};
use crate::project::ProjectRead;

/// Find the workspace root by walking up from the given path.
///
/// Returns the absolute path to the directory containing `.workspace.json`.
/// Errors if no workspace root is found.
pub fn locate(path: &camino::Utf8Path) -> Result<Utf8PathBuf, SysandError> {
    let ws = crate::discover::discover_workspace(path)
        .map_err(SysandError::from)?;

    ws.map(|w| w.root_path().to_owned())
        .ok_or_else(|| {
            SysandError::new(
                ErrorCode::WorkspaceNotFound,
                "no workspace found in parent directories",
            )
        })
}

/// Build KPAR archives for all projects in a workspace.
pub fn build(
    workspace: &crate::workspace::Workspace,
    output_path: &camino::Utf8Path,
    opts: crate::types::options::BuildOptions,
) -> Result<Vec<crate::types::output::BuildOutput>, SysandError> {
    let compression = crate::facade::build::compression_to_internal(opts.compression)?;

    let kpars =
        crate::commands::build::do_build_workspace_kpars(workspace, output_path, compression, true, false)
            .map_err(SysandError::from)?;

    kpars
        .into_iter()
        .map(|kpar| {
            let (info, _meta) = kpar
                .get_project()
                .map_err(|e| SysandError::from(e))?;
            let info =
                info.ok_or_else(|| SysandError::new(ErrorCode::FieldRequired, "built KPAR missing .project.json"))?;
            Ok(crate::types::output::BuildOutput {
                path: Utf8PathBuf::from(output_path),
                name: info.name,
                version: info.version,
            })
        })
        .collect()
}
