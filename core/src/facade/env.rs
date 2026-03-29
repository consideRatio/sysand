// SPDX-FileCopyrightText: © 2026 Sysand contributors <opensource@sensmetry.com>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Environment management facade functions.

// TODO: pub fn install(ctx, iri, net, opts) -> Result<(), SysandError>
// Needs: resolver + lock + sync combined. Complex orchestration.

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

/// Sync the environment to match the lockfile.
///
/// Reads `sysand-lock.toml` and ensures `sysand_env/` matches it:
/// missing projects are fetched and installed, mismatched versions
/// are replaced.
/// Sync the environment to match the lockfile.
///
/// Reads `sysand-lock.toml` and ensures `sysand_env/` matches it:
/// missing projects are fetched and installed, mismatched versions
/// are replaced.
#[cfg(all(feature = "filesystem", feature = "networking"))]
pub fn sync<Policy: crate::auth::HTTPAuthentication>(
    lock: &crate::lock::Lock,
    project_root: &camino::Utf8Path,
    env: &mut crate::env::local_directory::LocalDirectoryEnvironment,
    net: &crate::types::network::NetworkContext<Policy>,
    provided_iris: &std::collections::HashMap<String, Vec<crate::project::memory::InMemoryProject>>,
) -> Result<(), SysandError> {
    use crate::project::{
        AsSyncProjectTokio, ProjectReadAsync,
        gix_git_download::{GixDownloadedError, GixDownloadedProject},
        local_kpar::LocalKParProject,
        local_src::LocalSrcProject,
        reqwest_kpar_download::ReqwestKparDownloadedProject,
        reqwest_src::ReqwestSrcProjectAsync,
    };

    let client = net.client.clone();
    let runtime = net.runtime.clone();
    let auth = net.auth.clone();

    crate::commands::sync::do_sync(
        lock,
        env,
        Some(|src_path: &camino::Utf8Path| LocalSrcProject {
            nominal_path: Some(src_path.to_path_buf()),
            project_path: project_root.join(src_path),
        }),
        Some({
            let client = client.clone();
            let runtime = runtime.clone();
            let auth = auth.clone();
            move |remote_src: String| -> Result<AsSyncProjectTokio<ReqwestSrcProjectAsync<Policy>>, url::ParseError> {
                Ok(ReqwestSrcProjectAsync {
                    client: client.clone(),
                    url: reqwest::Url::parse(&remote_src)?,
                    auth_policy: auth.clone(),
                }
                .to_tokio_sync(runtime.clone()))
            }
        }),
        Some(|kpar_path: &camino::Utf8Path| {
            LocalKParProject::new_guess_root_nominal(
                project_root.join(kpar_path),
                kpar_path,
            )
            .expect("failed to open local KPAR")
        }),
        Some({
            let client = client.clone();
            let runtime = runtime.clone();
            let auth = auth.clone();
            move |remote_kpar: String| -> Result<AsSyncProjectTokio<ReqwestKparDownloadedProject<Policy>>, url::ParseError> {
                Ok(
                    ReqwestKparDownloadedProject::new_guess_root(
                        reqwest::Url::parse(&remote_kpar)?,
                        client.clone(),
                        auth.clone(),
                    )
                    .expect("failed to download remote KPAR")
                    .to_tokio_sync(runtime.clone()),
                )
            }
        }),
        Some(|remote_git: String| -> Result<GixDownloadedProject, GixDownloadedError> {
            GixDownloadedProject::new(remote_git)
        }),
        provided_iris,
    )
    .map_err(|e| SysandError::new(crate::error::ErrorCode::IoError, e.to_string()))
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
