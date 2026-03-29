// SPDX-FileCopyrightText: © 2025 Sysand contributors <opensource@sensmetry.com>
// SPDX-License-Identifier: MIT OR Apache-2.0

use anyhow::Result;
use camino::{Utf8Path, Utf8PathBuf};
use sysand_core::{
    project::{local_src::LocalSrcProject, utils::wrapfs},
    types::options::InitOptions,
};

use crate::CliError;

pub fn command_init(
    name: Option<String>,
    publisher: Option<String>,
    version: Option<String>,
    _no_semver: bool,
    license: Option<String>,
    no_spdx: bool,
    path: Option<String>,
) -> Result<()> {
    let path = match path {
        Some(p) => {
            wrapfs::create_dir_all(&p)?;
            Utf8PathBuf::from(p)
        }
        None => Utf8PathBuf::from("."),
    };
    let name = match name {
        Some(n) => Some(n),
        None => Some(default_name_from_path(&path)?),
    };

    let mut project = LocalSrcProject {
        nominal_path: None,
        project_path: path,
    };
    sysand_core::facade::init::init(
        &mut project,
        InitOptions {
            name,
            publisher,
            version,
            license,
            allow_non_spdx: !no_spdx,
        },
    )?;
    Ok(())
}

fn default_name_from_path<P: AsRef<Utf8Path>>(path: P) -> Result<String> {
    Ok(wrapfs::canonicalize(&path)?
        .file_name()
        .ok_or_else(|| {
            CliError::InvalidDirectory(format!("path `{}` is not a directory", path.as_ref()))
        })?
        .to_string())
}
