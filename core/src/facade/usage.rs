// SPDX-FileCopyrightText: © 2026 Sysand contributors <opensource@sensmetry.com>
// SPDX-License-Identifier: MIT OR Apache-2.0

use crate::error::SysandError;
use crate::model::InterchangeProjectUsageRaw;
use crate::project::ProjectMut;

/// Add a usage (dependency) to the project.
///
/// Only updates the manifest (`.project.json`). Lock and sync
/// side-effects are controlled by `UpdateMode` in the caller
/// (CLI or binding layer).
pub fn add<P: ProjectMut>(
    project: &mut P,
    iri: &str,
    version_req: Option<&str>,
) -> Result<bool, SysandError>
where
    P::Error: Into<SysandError>,
{
    let usage = InterchangeProjectUsageRaw {
        resource: iri.to_string(),
        version_constraint: version_req.map(|s| s.to_string()),
    };
    crate::commands::add::do_add(project, &usage).map_err(SysandError::from)
}

/// Remove a usage (dependency) from the project.
/// Returns the list of removed usages.
pub fn remove<P: ProjectMut>(
    project: &mut P,
    iri: &str,
) -> Result<Vec<InterchangeProjectUsageRaw>, SysandError>
where
    P::Error: Into<SysandError>,
{
    crate::commands::remove::do_remove(project, iri).map_err(SysandError::from)
}
