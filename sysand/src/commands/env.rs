// SPDX-FileCopyrightText: © 2025 Sysand contributors <opensource@sensmetry.com>
// SPDX-License-Identifier: MIT OR Apache-2.0

use std::collections::HashMap;

use anyhow::Result;

use camino::{Utf8Path, Utf8PathBuf};

use sysand_core::{
    auth::HTTPAuthentication,
    commands::env::do_env_local_dir,
    env::local_directory::LocalDirectoryEnvironment,
    project::utils::wrapfs,
    stdlib::known_std_libs,
    types::network::NetworkContext,
};

use crate::{
    DEFAULT_INDEX_URL,
    cli::{InstallOptions, ResolutionOptions},
};

pub fn command_env<P: AsRef<Utf8Path>>(path: P) -> Result<LocalDirectoryEnvironment> {
    Ok(do_env_local_dir(path)?)
}

pub fn command_env_install<Policy: HTTPAuthentication>(
    iri: fluent_uri::Iri<String>,
    version: Option<String>,
    install_opts: InstallOptions,
    resolution_opts: ResolutionOptions,
    net: &NetworkContext<Policy>,
    project_root: Option<Utf8PathBuf>,
) -> Result<()> {
    let project_root = project_root.unwrap_or(wrapfs::current_dir()?);
    let mut env = crate::get_or_create_env(project_root.as_path())?;

    let no_deps = install_opts.deps == "none";

    let provided_iris = if !resolution_opts.include_std {
        let sysml_std = known_std_libs();
        if sysml_std.contains_key(iri.as_ref()) {
            crate::logger::warn_std(iri.as_ref());
            return Ok(());
        }
        sysml_std
    } else {
        HashMap::default()
    };

    let index_urls = sysand_core::facade::resolver::resolve_index_urls(
        &net.config,
        resolution_opts.index,
        resolution_opts.default_index,
        &resolution_opts.index_mode,
        DEFAULT_INDEX_URL,
    )?;

    sysand_core::facade::env::install(
        iri.as_ref(),
        version.as_deref(),
        &project_root,
        &mut env,
        net,
        index_urls,
        &provided_iris,
        no_deps,
        install_opts.allow_overwrite,
        install_opts.allow_multiple,
    )?;

    Ok(())
}

pub fn command_env_install_path<Policy: HTTPAuthentication>(
    iri: fluent_uri::Iri<String>,
    version: Option<String>,
    path: Utf8PathBuf,
    install_opts: InstallOptions,
    resolution_opts: ResolutionOptions,
    net: &NetworkContext<Policy>,
    project_root: Option<Utf8PathBuf>,
) -> Result<()> {
    let project_root = project_root.unwrap_or(wrapfs::current_dir()?);
    let mut env = crate::get_or_create_env(project_root.as_path())?;

    let no_deps = install_opts.deps == "none";

    let provided_iris = if !resolution_opts.include_std {
        let sysml_std = known_std_libs();
        if sysml_std.contains_key(iri.as_ref()) {
            crate::logger::warn_std(iri.as_ref());
            return Ok(());
        }
        sysml_std
    } else {
        HashMap::default()
    };

    let index_urls = sysand_core::facade::resolver::resolve_index_urls(
        &net.config,
        resolution_opts.index,
        resolution_opts.default_index,
        &resolution_opts.index_mode,
        DEFAULT_INDEX_URL,
    )?;

    sysand_core::facade::env::install_from_path(
        iri.as_ref(),
        &path,
        version.as_deref(),
        &project_root,
        &mut env,
        net,
        index_urls,
        &provided_iris,
        no_deps,
        install_opts.allow_overwrite,
        install_opts.allow_multiple,
    )?;

    Ok(())
}

pub fn command_env_uninstall<S: AsRef<str>, Q: AsRef<str>>(
    iri: S,
    version: Option<Q>,
    env: LocalDirectoryEnvironment,
) -> Result<()> {
    sysand_core::facade::env::uninstall(env, iri.as_ref(), version.as_ref().map(|s| s.as_ref()))?;
    Ok(())
}

pub fn command_env_list(env: Option<LocalDirectoryEnvironment>) -> Result<()> {
    match env {
        Some(env) => {
            let entries = sysand_core::facade::env::list(env)?;
            for entry in entries {
                match entry.version {
                    Some(v) => println!("{} {}", entry.iri, v),
                    None => println!("{}", entry.iri),
                }
            }
        }
        None => {
            log::warn!("no environment found");
        }
    }
    Ok(())
}
