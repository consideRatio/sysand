// SPDX-FileCopyrightText: © 2025 Sysand contributors <opensource@sensmetry.com>
// SPDX-License-Identifier: MIT OR Apache-2.0

use std::collections::HashMap;

use anyhow::Result;
use camino::Utf8Path;

use sysand_core::{
    auth::HTTPAuthentication,
    commands::lock::{DEFAULT_LOCKFILE_NAME, LockOutcome, do_lock_local_editable},
    context::ProjectContext,
    facade::resolver::{build_resolver, resolve_index_urls},
    project::utils::wrapfs,
    stdlib::known_std_libs,
    types::network::NetworkContext,
};

use crate::{DEFAULT_INDEX_URL, cli::ResolutionOptions};

/// Generate a lockfile for `current_project`.
#[expect(clippy::too_many_arguments)]
pub fn command_lock<P: AsRef<Utf8Path>, Policy: HTTPAuthentication, R: AsRef<Utf8Path>>(
    path: P,
    resolution_opts: ResolutionOptions,
    net: &NetworkContext<Policy>,
    project_root: R,
    ctx: ProjectContext,
) -> Result<sysand_core::lock::Lock> {
    assert!(path.as_ref().is_relative(), "{}", path.as_ref());

    let provided_iris = if !resolution_opts.include_std {
        known_std_libs()
    } else {
        HashMap::default()
    };

    let index_urls = resolve_index_urls(
        &net.config,
        resolution_opts.index,
        resolution_opts.default_index,
        &resolution_opts.index_mode,
        DEFAULT_INDEX_URL,
    )?;

    let wrapped_resolver = build_resolver(
        path.as_ref(),
        net,
        index_urls,
        provided_iris.clone(),
    )?;

    let alias_iris = if let Some(w) = &ctx.current_workspace {
        w.projects()
            .iter()
            .find(|p| Utf8Path::new(&p.path) == path.as_ref())
            .map(|p| p.iris.clone())
    } else {
        None
    };
    let LockOutcome {
        lock,
        dependencies: _dependencies,
    } = do_lock_local_editable(
        &path,
        &project_root,
        alias_iris,
        &provided_iris,
        wrapped_resolver,
        &ctx,
    )?;

    let canonical = lock.canonicalize();
    wrapfs::write(
        path.as_ref().join(DEFAULT_LOCKFILE_NAME),
        canonical.to_string(),
    )?;

    Ok(canonical)
}
