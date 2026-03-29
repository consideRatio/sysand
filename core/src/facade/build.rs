// SPDX-FileCopyrightText: © 2026 Sysand contributors <opensource@sensmetry.com>
// SPDX-License-Identifier: MIT OR Apache-2.0

use crate::commands::build::{do_build_kpar, KparCompressionMethod};
use crate::error::{ErrorCode, SysandError};
use crate::project::ProjectRead;
use crate::types::enums::Compression;
use crate::types::options::BuildOptions;
use crate::types::output::BuildOutput;

/// Build a KPAR archive from a project.
///
/// The caller provides a `ProjectRead` implementation. Returns
/// metadata about the created archive.
pub fn build<P: ProjectRead>(
    project: &P,
    output_path: &camino::Utf8Path,
    opts: BuildOptions,
) -> Result<BuildOutput, SysandError>
where
    P::Error: Into<SysandError>,
{
    let compression = compression_to_internal(opts.compression)?;

    let kpar = do_build_kpar(project, output_path, compression, true, false)
        .map_err(SysandError::from)?;

    // Extract project info from the built KPAR
    let (info, _meta) = kpar
        .get_project()
        .map_err(|e| SysandError::new(ErrorCode::Internal, e.to_string()))?;

    let info = info.ok_or_else(|| {
        SysandError::new(ErrorCode::FieldRequired, "built KPAR missing .project.json")
    })?;

    Ok(BuildOutput {
        path: camino::Utf8PathBuf::from(output_path),
        name: info.name,
        version: info.version,
    })
}

pub(crate) fn compression_to_internal(c: Compression) -> Result<KparCompressionMethod, SysandError> {
    match c {
        Compression::Stored => Ok(KparCompressionMethod::Stored),
        Compression::Deflated => Ok(KparCompressionMethod::Deflated),
        #[cfg(feature = "kpar-bzip2")]
        Compression::Bzip2 => Ok(KparCompressionMethod::Bzip2),
        #[cfg(not(feature = "kpar-bzip2"))]
        Compression::Bzip2 => Err(SysandError::new(
            ErrorCode::FieldInvalid,
            "bzip2 compression requires the kpar-bzip2 feature",
        )),
        #[cfg(feature = "kpar-zstd")]
        Compression::Zstd => Ok(KparCompressionMethod::Zstd),
        #[cfg(not(feature = "kpar-zstd"))]
        Compression::Zstd => Err(SysandError::new(
            ErrorCode::FieldInvalid,
            "zstd compression requires the kpar-zstd feature",
        )),
        #[cfg(feature = "kpar-xz")]
        Compression::Xz => Ok(KparCompressionMethod::Xz),
        #[cfg(not(feature = "kpar-xz"))]
        Compression::Xz => Err(SysandError::new(
            ErrorCode::FieldInvalid,
            "xz compression requires the kpar-xz feature",
        )),
        #[cfg(feature = "kpar-ppmd")]
        Compression::Ppmd => Ok(KparCompressionMethod::Ppmd),
        #[cfg(not(feature = "kpar-ppmd"))]
        Compression::Ppmd => Err(SysandError::new(
            ErrorCode::FieldInvalid,
            "ppmd compression requires the kpar-ppmd feature",
        )),
    }
}
