// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: © 2026 Sysand contributors <opensource@sensmetry.com>

use anyhow::Result;
use camino::Utf8PathBuf;
use fluent_uri::Iri;
use sysand_core::commands::index::IndexManager;

pub fn command_index_init(root: Utf8PathBuf) -> Result<()> {
    IndexManager::new(root).init()?;
    Ok(())
}

pub fn command_index_add(
    root: Utf8PathBuf,
    kpar_path: Utf8PathBuf,
    iri: Option<Iri<String>>,
) -> Result<()> {
    let added_iri = IndexManager::new(root).add(&kpar_path, iri)?;
    let header = sysand_core::style::get_style_config().header;
    log::info!("{header}{:>12}{header:#} `{added_iri}`", "Added");
    Ok(())
}

pub fn command_index_yank(root: Utf8PathBuf, iri: Iri<String>, version: String) -> Result<()> {
    IndexManager::new(root).yank(&iri, &version)?;
    Ok(())
}

pub fn command_index_remove(
    root: Utf8PathBuf,
    iri: Iri<String>,
    version: Option<String>,
) -> Result<()> {
    let manager = IndexManager::new(root);
    if let Some(version) = version {
        manager.remove_version(&iri, &version)?;
    } else {
        manager.remove_project(&iri)?;
    }
    Ok(())
}
