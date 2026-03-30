// SPDX-FileCopyrightText: © 2025 Sysand contributors <opensource@sensmetry.com>
// SPDX-License-Identifier: MIT OR Apache-2.0

use std::collections::HashMap;

use anyhow::Result;
use camino::Utf8Path;

use sysand_core::{
    auth::HTTPAuthentication,
    env::local_directory::LocalDirectoryEnvironment,
    lock::Lock,
    project::memory::InMemoryProject,
    types::network::NetworkContext,
};

pub fn command_sync<P: AsRef<Utf8Path>, Policy: HTTPAuthentication>(
    lock: &Lock,
    project_root: P,
    env: &mut LocalDirectoryEnvironment,
    net: &NetworkContext<Policy>,
    provided_iris: &HashMap<String, Vec<InMemoryProject>>,
) -> Result<()> {
    sysand_core::facade::env::sync(lock, project_root.as_ref(), env, net, provided_iris)?;
    Ok(())
}
