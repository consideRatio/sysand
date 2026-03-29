// SPDX-FileCopyrightText: © 2026 Sysand contributors <opensource@sensmetry.com>
// SPDX-License-Identifier: MIT OR Apache-2.0

use typed_path::Utf8UnixPath;

use crate::error::SysandError;
use crate::project::ProjectMut;
use crate::types::enums::{IndexSymbols, ChecksumMode, Language};
use crate::types::options::SourceAddOptions;

/// Add source files to the project metadata.
pub fn add<P: ProjectMut>(
    project: &mut P,
    path: &Utf8UnixPath,
    opts: SourceAddOptions,
) -> Result<(), SysandError>
where
    P::Error: Into<SysandError>,
{
    let compute_checksum = matches!(opts.checksum, ChecksumMode::Sha256);
    let index_symbols = matches!(opts.index_symbols, IndexSymbols::On);
    let force_format = match opts.language {
        Language::Auto => None,
        Language::Sysml => Some(crate::symbols::Language::SysML),
        Language::Kerml => Some(crate::symbols::Language::KerML),
    };

    crate::commands::include::do_include(
        project,
        path,
        compute_checksum,
        index_symbols,
        force_format,
    )
    .map_err(SysandError::from)
}

/// Remove source files from the project metadata.
pub fn remove<P: ProjectMut>(
    project: &mut P,
    path: &Utf8UnixPath,
) -> Result<(), SysandError>
where
    P::Error: Into<SysandError>,
{
    crate::commands::exclude::do_exclude(project, path)
        .map(|_| ()) // discard SourceExclusionOutcome
        .map_err(SysandError::from)
}
