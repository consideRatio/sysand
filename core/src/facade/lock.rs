// SPDX-FileCopyrightText: © 2026 Sysand contributors <opensource@sensmetry.com>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Lock facade functions.

/// Resolve dependencies and write the lockfile.
///
/// Takes a pre-built resolver (constructed by the CLI or binding layer
/// from `NetworkContext` + config). The resolver encapsulates index
/// URLs, source overrides, auth, and HTTP transport.
///
/// `provided_iris` contains IRIs to treat as pre-resolved (typically
/// stdlib exclusions from `known_std_libs()`).
#[cfg(feature = "filesystem")]
pub fn update<PD, R>(
    ctx: &crate::types::context::ProjectContext,
    resolver: R,
    provided_iris: &std::collections::HashMap<String, Vec<crate::project::memory::InMemoryProject>>,
) -> Result<(), crate::error::SysandError>
where
    PD: crate::project::ProjectRead + std::fmt::Debug + 'static,
    R: crate::resolve::ResolveRead<ProjectStorage = PD> + std::fmt::Debug + 'static,
{
    use crate::commands::lock::{DEFAULT_LOCKFILE_NAME, do_lock_local_editable};
    use crate::context::ProjectContext as InternalProjectContext;
    use crate::error::{ErrorCode, SysandError};
    use crate::project::utils::wrapfs;

    let project_root = &ctx.path;

    let internal_ctx = InternalProjectContext {
        current_workspace: None,
        current_project: None,
    };

    let outcome = do_lock_local_editable(
        project_root,
        project_root,
        None,
        provided_iris,
        resolver,
        &internal_ctx,
    )
    .map_err(|e| SysandError::new(ErrorCode::ResolutionFailed, e.to_string()))?;

    let canonical = outcome.lock.canonicalize();
    wrapfs::write(
        project_root.join(DEFAULT_LOCKFILE_NAME),
        canonical.to_string(),
    )
    .map_err(SysandError::from)?;

    Ok(())
}
