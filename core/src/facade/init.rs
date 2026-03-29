// SPDX-FileCopyrightText: © 2026 Sysand contributors <opensource@sensmetry.com>
// SPDX-License-Identifier: MIT OR Apache-2.0

use crate::error::SysandError;
use crate::project::ProjectMut;
use crate::types::options::InitOptions;

/// Initialize a new project.
///
/// Creates `.project.json` and `.meta.json` in the project storage.
/// The caller provides the storage backend (e.g., `LocalSrcProject`
/// for filesystem, `ProjectLocalBrowserStorage` for browser).
pub fn init<P: ProjectMut>(
    project: &mut P,
    opts: InitOptions,
) -> Result<(), SysandError>
where
    P::Error: Into<SysandError>,
{
    let version = opts.version.unwrap_or_else(|| "0.0.1".into());
    let no_spdx = !opts.allow_non_spdx;

    crate::commands::init::do_init_ext(
        opts.name.unwrap_or_default(),
        opts.publisher,
        version,
        false, // no_semver: always enforce semver
        opts.license,
        no_spdx,
        project,
    )
    .map_err(SysandError::from)
}
