// SPDX-FileCopyrightText: © 2026 Sysand contributors <opensource@sensmetry.com>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Network infrastructure bundle for commands that need HTTP/Git access.
//!
//! Commands that resolve dependencies, sync environments, or clone
//! projects need an HTTP client, auth policy, and async runtime.
//! `NetworkContext` bundles these so callers construct them once and
//! pass them to each command.
//!
//! Non-network commands (init, build, source add/remove, etc.) do not
//! take a `NetworkContext`.

use std::sync::Arc;

use crate::config::Config;

/// Network infrastructure for commands that access remote resources.
///
/// Generic over `Policy` (the auth implementation) because
/// `HTTPAuthentication` is not object-safe (uses RPITIT).
///
/// Construct once at startup (or on first use) and pass to any facade
/// function that needs network access: `lock::update`, `env::sync`,
/// `env::install`, `clone`.
///
/// # Example
///
/// ```ignore
/// let net = NetworkContext::new(config, auth)?;
/// sysand_core::facade::env::sync(lock, root, &mut env, &net, &iris)?;
/// ```
#[cfg(feature = "networking")]
pub struct NetworkContext<Policy: crate::auth::HTTPAuthentication> {
    pub config: Config,
    pub auth: Arc<Policy>,
    pub client: reqwest_middleware::ClientWithMiddleware,
    pub runtime: Arc<tokio::runtime::Runtime>,
}

#[cfg(feature = "networking")]
impl<Policy: crate::auth::HTTPAuthentication> NetworkContext<Policy> {
    /// Create a new `NetworkContext` with the given config and auth policy.
    ///
    /// Builds an HTTP client and single-threaded tokio runtime internally.
    pub fn new(
        config: Config,
        auth: Arc<Policy>,
    ) -> Result<Self, crate::error::SysandError> {
        use crate::error::{ErrorCode, SysandError};

        let client = crate::resolve::net_utils::create_reqwest_client()
            .map_err(|e| SysandError::new(ErrorCode::Internal, e.to_string()))?;

        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|e| SysandError::new(ErrorCode::Internal, format!("failed to create async runtime: {e}")))?;

        Ok(Self {
            config,
            auth,
            client,
            runtime: Arc::new(runtime),
        })
    }

    /// Create with a pre-built HTTP client and runtime (for testing or
    /// custom setups).
    pub fn with_client(
        config: Config,
        auth: Arc<Policy>,
        client: reqwest_middleware::ClientWithMiddleware,
        runtime: Arc<tokio::runtime::Runtime>,
    ) -> Self {
        Self {
            config,
            auth,
            client,
            runtime,
        }
    }
}
