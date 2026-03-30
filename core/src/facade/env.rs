// SPDX-FileCopyrightText: © 2026 Sysand contributors <opensource@sensmetry.com>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Environment management facade functions.


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

/// Install a single already-resolved project into the environment.
///
/// The caller is responsible for resolving the project first (via
/// a resolver or by opening a local project directly). For the full
/// resolve + lock + sync orchestration, use `lock::update` followed
/// by `env::sync`.
pub fn install_project<P, E>(
    iri: &str,
    project: &P,
    env: &mut E,
    allow_overwrite: bool,
    allow_multiple: bool,
) -> Result<(), SysandError>
where
    P: crate::project::ProjectRead,
    E: crate::env::WriteEnvironment + crate::env::ReadEnvironment,
    P::Error: Into<SysandError>,
    E::ReadError: Into<SysandError>,
    E::WriteError: Into<SysandError>,
    <E::InterchangeProjectMut as crate::project::ProjectRead>::Error: Into<SysandError>,
{
    crate::commands::env::do_env_install_project(
        iri,
        project,
        env,
        allow_overwrite,
        allow_multiple,
    )
    .map_err(|e| SysandError::new(crate::error::ErrorCode::EnvConflict, e.to_string()))
}

/// Install a project from a local path into the environment.
///
/// If `no_deps` is false, resolves and installs dependencies too.
#[cfg(all(feature = "filesystem", feature = "networking"))]
pub fn install_from_path<Policy: crate::auth::HTTPAuthentication>(
    iri: &str,
    path: &camino::Utf8Path,
    version_check: Option<&str>,
    project_root: &camino::Utf8Path,
    env: &mut crate::env::local_directory::LocalDirectoryEnvironment,
    net: &crate::types::network::NetworkContext<Policy>,
    index_urls: Option<Vec<url::Url>>,
    provided_iris: &std::collections::HashMap<String, Vec<crate::project::memory::InMemoryProject>>,
    no_deps: bool,
    allow_overwrite: bool,
    allow_multiple: bool,
) -> Result<(), SysandError> {
    use crate::project::ProjectRead;
    use crate::project::utils::wrapfs;
    use crate::resolve::file::FileResolverProject;

    let metadata = wrapfs::metadata(path).map_err(SysandError::from)?;
    let project: FileResolverProject = if metadata.is_dir() {
        FileResolverProject::LocalSrcProject(crate::project::local_src::LocalSrcProject {
            nominal_path: None,
            project_path: path.to_string().into(),
        })
    } else {
        FileResolverProject::LocalKParProject(
            crate::project::local_kpar::LocalKParProject::new_guess_root(path)
                .map_err(SysandError::from)?,
        )
    };

    // Version check
    if let Some(vr_str) = version_check {
        let project_version = project
            .get_info()
            .map_err(|e| SysandError::new(crate::error::ErrorCode::Internal, e.to_string()))?
            .and_then(|info| semver::Version::parse(&info.version).ok());
        if let Some(pv) = project_version {
            let vr = semver::VersionReq::parse(vr_str)
                .map_err(|e| SysandError::new(crate::error::ErrorCode::VersionInvalid, e.to_string()))?;
            if !vr.matches(&pv) {
                return Err(SysandError::new(
                    crate::error::ErrorCode::VersionInvalid,
                    format!("project at `{path}` has version `{pv}` which does not match `{vr}`"),
                ));
            }
        }
    }

    if no_deps {
        crate::commands::env::do_env_install_project(
            iri,
            &project,
            env,
            allow_overwrite,
            allow_multiple,
        )
        .map_err(|e| SysandError::new(crate::error::ErrorCode::EnvConflict, e.to_string()))
    } else {
        use std::str::FromStr;
        use crate::commands::lock::do_lock_projects;
        use crate::project::editable::EditableProject;

        let resolver = crate::facade::resolver::build_resolver(
            project_root,
            net,
            index_urls,
            provided_iris.clone(),
        )?;

        let iri_parsed = fluent_uri::Iri::from_str(iri)
            .map_err(|e| SysandError::new(crate::error::ErrorCode::IriInvalid, e.to_string()))?;
        let project_with_editable = EditableProject::new(
            path.as_str().into(),
            project,
        );

        let internal_ctx = crate::context::ProjectContext::default();

        let outcome = do_lock_projects(
            [(Some(vec![iri_parsed]), &project_with_editable)],
            resolver,
            provided_iris,
            &internal_ctx,
        )
        .map_err(|e| SysandError::new(crate::error::ErrorCode::ResolutionFailed, e.to_string()))?;

        sync(&outcome.lock, project_root, env, net, provided_iris)
    }
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

/// Install a project by IRI into the environment, resolving dependencies.
///
/// Full orchestration: resolve the IRI via the provided resolver, lock
/// dependencies, then sync the environment. This is what
/// `sysand env install <IRI>` does end-to-end.
///
/// If `no_deps` is true, installs only the specified project without
/// resolving or installing dependencies.
#[cfg(all(feature = "filesystem", feature = "networking"))]
pub fn install<Policy: crate::auth::HTTPAuthentication>(
    iri: &str,
    version_req: Option<&str>,
    project_root: &camino::Utf8Path,
    env: &mut crate::env::local_directory::LocalDirectoryEnvironment,
    net: &crate::types::network::NetworkContext<Policy>,
    index_urls: Option<Vec<url::Url>>,
    provided_iris: &std::collections::HashMap<String, Vec<crate::project::memory::InMemoryProject>>,
    no_deps: bool,
    allow_overwrite: bool,
    allow_multiple: bool,
) -> Result<(), SysandError> {
    use std::str::FromStr;

    let resolver = crate::facade::resolver::build_resolver(
        project_root,
        net,
        index_urls,
        provided_iris.clone(),
    )?;

    if no_deps {
        // Resolve project and install directly
        let (_version, storage) = resolve_project_version(iri, version_req, &resolver)?;
        crate::commands::env::do_env_install_project(
            iri,
            &storage,
            env,
            allow_overwrite,
            allow_multiple,
        )
        .map_err(|e| SysandError::new(crate::error::ErrorCode::EnvConflict, e.to_string()))
    } else {
        // Resolve, lock, and sync
        let iri_parsed = fluent_uri::Iri::from_str(iri)
            .map_err(|e| SysandError::new(crate::error::ErrorCode::IriInvalid, e.to_string()))?;
        let version_constraint = version_req
            .map(|v| semver::VersionReq::parse(v))
            .transpose()
            .map_err(|e| SysandError::new(crate::error::ErrorCode::VersionInvalid, e.to_string()))?;

        let usages = vec![crate::model::InterchangeProjectUsage {
            resource: iri_parsed,
            version_constraint,
        }];

        let internal_ctx = crate::context::ProjectContext::default();

        let outcome = crate::commands::lock::do_lock_extend(
            crate::lock::Lock::default(),
            usages,
            resolver,
            provided_iris,
            &internal_ctx,
        )
        .map_err(|e| SysandError::new(crate::error::ErrorCode::ResolutionFailed, e.to_string()))?;

        sync(&outcome.lock, project_root, env, net, provided_iris)
    }
}

/// Resolve a project by IRI and optional version constraint.
#[cfg(all(feature = "filesystem", feature = "networking"))]
fn resolve_project_version<R>(
    iri: &str,
    version_req: Option<&str>,
    resolver: &R,
) -> Result<(Option<String>, <R as crate::resolve::ResolveRead>::ProjectStorage), SysandError>
where
    R: crate::resolve::ResolveRead + std::fmt::Debug,
    R::ProjectStorage: crate::project::ProjectRead + std::fmt::Debug,
{
    use crate::resolve::ResolutionOutcome;
    use crate::project::ProjectRead;

    let iri_parsed = fluent_uri::Iri::parse(iri.to_string())
        .map_err(|(e, _)| SysandError::new(crate::error::ErrorCode::IriInvalid, e.to_string()))?;

    let candidates = match resolver.resolve_read(&iri_parsed)
        .map_err(|e| SysandError::new(crate::error::ErrorCode::ResolutionFailed, e.to_string()))?
    {
        ResolutionOutcome::Resolved(storages) => storages,
        ResolutionOutcome::UnsupportedIRIType(msg) => {
            return Err(SysandError::new(crate::error::ErrorCode::IriInvalid, msg));
        }
        ResolutionOutcome::Unresolvable(msg) => {
            return Err(SysandError::new(crate::error::ErrorCode::ProjectNotInIndex, msg));
        }
    };

    let version_req = version_req
        .map(|v| semver::VersionReq::parse(v))
        .transpose()
        .map_err(|e| SysandError::new(crate::error::ErrorCode::VersionInvalid, e.to_string()))?;

    for candidate in candidates {
        let storage = candidate
            .map_err(|e| SysandError::new(crate::error::ErrorCode::ResolutionFailed, e.to_string()))?;
        let version = storage.version()
            .map_err(|e| SysandError::new(crate::error::ErrorCode::Internal, e.to_string()))?;

        if let Some(ref vr) = version_req {
            if let Some(ref v) = version {
                if let Ok(parsed) = semver::Version::parse(v) {
                    if vr.matches(&parsed) {
                        return Ok((version, storage));
                    }
                }
            }
        } else {
            return Ok((version, storage));
        }
    }

    Err(SysandError::new(
        crate::error::ErrorCode::VersionNotInIndex,
        format!("no matching version found for `{iri}`"),
    ))
}
