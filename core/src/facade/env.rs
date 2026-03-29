// SPDX-FileCopyrightText: © 2026 Sysand contributors <opensource@sensmetry.com>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Environment management facade functions.

// TODO: pub fn sync(ctx, opts) -> Result<(), SysandError>
// Needs: resolver chain, HTTP client, tokio runtime, auth policy.
// Same infrastructure question as lock::update.

// TODO: pub fn install(ctx, iri, opts) -> Result<(), SysandError>
// Needs: same infrastructure as sync + lock.

use crate::env::{ReadEnvironment, WriteEnvironment};
use crate::error::SysandError;
use crate::types::output::EnvEntry;

/// Create a new environment directory.
#[cfg(feature = "filesystem")]
pub fn create(path: &camino::Utf8Path) -> Result<(), SysandError> {
    crate::commands::env::do_env_local_dir(path)
        .map(|_| ()) // discard the LocalDirectoryEnvironment
        .map_err(SysandError::from)
}

/// List installed projects in an environment.
pub fn list<E: ReadEnvironment>(env: E) -> Result<Vec<EnvEntry>, SysandError>
where
    E::ReadError: Into<SysandError>,
{
    let entries = crate::commands::env::do_env_list(env)
        .map_err(|e| e.into())?;

    Ok(entries
        .into_iter()
        .map(|(iri, version)| EnvEntry { iri, version })
        .collect())
}

/// Uninstall a project from the environment.
pub fn uninstall<E: WriteEnvironment>(
    env: E,
    iri: &str,
    version: Option<&str>,
) -> Result<(), SysandError>
where
    E::WriteError: Into<SysandError>,
{
    crate::commands::env::do_env_uninstall(iri, version, env)
        .map_err(|e| e.into())
}
