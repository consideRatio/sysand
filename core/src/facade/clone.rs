// SPDX-FileCopyrightText: © 2026 Sysand contributors <opensource@sensmetry.com>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Clone facade functions.

/// Clone a resolved project's files into a target directory.
#[cfg(feature = "filesystem")]
pub fn clone_project<P: crate::project::ProjectRead>(
    source: &P,
    target: &camino::Utf8Path,
) -> Result<(), crate::error::SysandError>
where
    P::Error: Into<crate::error::SysandError>,
{
    use crate::project::local_src::LocalSrcProject;

    let mut dest = LocalSrcProject {
        nominal_path: None,
        project_path: target.to_path_buf(),
    };

    crate::env::utils::clone_project(source, &mut dest, true)
        .map(|_| ())
        .map_err(|e| crate::error::SysandError::new(crate::error::ErrorCode::IoError, e.to_string()))
}

/// Clone a project and resolve + install its dependencies.
///
/// Full orchestration:
/// 1. Lock dependencies from the cloned project using the resolver
/// 2. Write lockfile to the project directory
/// 3. Create environment and sync dependencies
///
/// The caller has already cloned the project files to `project_path`.
/// This function handles the dependency resolution and installation.
#[cfg(all(feature = "filesystem", feature = "networking"))]
pub fn clone_with_deps<Policy: crate::auth::HTTPAuthentication>(
    project_path: &camino::Utf8Path,
    identifiers: Option<Vec<fluent_uri::Iri<String>>>,
    net: &crate::types::network::NetworkContext<Policy>,
    provided_iris: &std::collections::HashMap<String, Vec<crate::project::memory::InMemoryProject>>,
) -> Result<(), crate::error::SysandError> {
    use crate::commands::lock::{DEFAULT_LOCKFILE_NAME, do_lock_projects};
    use crate::env::local_directory::DEFAULT_ENV_NAME;
    use crate::error::{ErrorCode, SysandError};
    use crate::project::editable::EditableProject;
    use crate::project::local_src::LocalSrcProject;
    use crate::project::utils::wrapfs;

    let local_project = LocalSrcProject {
        nominal_path: None,
        project_path: project_path.to_path_buf(),
    };
    let project = EditableProject::new(".".into(), local_project);

    // Build resolver for dependency resolution
    let index_urls = net.config.indexes.iter()
        .map(|idx| url::Url::parse(&idx.url))
        .collect::<Result<Vec<_>, _>>()
        .ok();

    let resolver = crate::facade::resolver::build_resolver(
        project_path,
        net,
        index_urls,
        provided_iris.clone(),
    )?;

    let internal_ctx = crate::context::ProjectContext::default();

    let outcome = do_lock_projects(
        [(identifiers, &project)],
        resolver,
        provided_iris,
        &internal_ctx,
    )
    .map_err(|e| SysandError::new(ErrorCode::ResolutionFailed, e.to_string()))?;

    // Write lockfile
    let lock = outcome.lock.canonicalize();
    wrapfs::write(
        project_path.join(DEFAULT_LOCKFILE_NAME),
        lock.to_string(),
    )
    .map_err(SysandError::from)?;

    // Create env and sync
    let env_path = project_path.join(DEFAULT_ENV_NAME);
    if !wrapfs::is_dir(&env_path).map_err(SysandError::from)? {
        crate::facade::env::create(&env_path)?;
    }
    let mut env = crate::env::local_directory::LocalDirectoryEnvironment {
        environment_path: env_path,
    };

    crate::facade::env::sync(&lock, project_path, &mut env, net, provided_iris)
}
