// SPDX-FileCopyrightText: © 2025 Sysand contributors <opensource@sensmetry.com>
// SPDX-License-Identifier: MIT OR Apache-2.0

use anyhow::{Result, bail};
use camino::Utf8Path;
use sysand_core::{
    ErrorCode,
    project::local_src::LocalSrcProject,
    types::{enums::Compression, options::BuildOptions},
    workspace::Workspace,
};

pub fn command_build_for_project<P: AsRef<Utf8Path>>(
    path: P,
    compression: Compression,
    current_project: LocalSrcProject,
    allow_path_usage: bool,
) -> Result<()> {
    match sysand_core::facade::build::build(
        &current_project,
        path.as_ref(),
        BuildOptions {
            compression,
            allow_path_usage,
            ..Default::default()
        },
    ) {
        Ok(_) => Ok(()),
        Err(err) if err.code == ErrorCode::FieldInvalid && err.message.contains("path usage") => {
            bail!(
                "{}\nto build anyway, pass `--allow-path-usage`",
                err.message
            )
        }
        Err(err) => bail!(err),
    }
}

pub fn command_build_for_workspace<P: AsRef<Utf8Path>>(
    path: P,
    compression: Compression,
    workspace: Workspace,
    allow_path_usage: bool,
) -> Result<()> {
    log::warn!(
        "Workspaces are an experimental feature\n\
        and their behavior may change even with minor\n\
        releases. For the status of this feature, see\n\
        https://github.com/sensmetry/sysand/issues/101."
    );
    sysand_core::facade::workspace::build(
        &workspace,
        path.as_ref(),
        BuildOptions {
            compression,
            allow_path_usage,
            ..Default::default()
        },
    )?;
    Ok(())
}
