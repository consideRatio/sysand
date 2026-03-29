// SPDX-FileCopyrightText: © 2026 Sysand contributors <opensource@sensmetry.com>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Return types from facade functions.

use camino::Utf8PathBuf;

/// Returned by `build`. Workspace build returns `Vec<BuildOutput>`.
#[derive(Debug, Clone)]
pub struct BuildOutput {
    /// Absolute path to the created `.kpar` file.
    pub path: Utf8PathBuf,
    /// Project name.
    pub name: String,
    /// Project version (semver).
    pub version: String,
}

/// Returned by `env::list`. One entry per installed project.
#[derive(Debug, Clone)]
pub struct EnvEntry {
    /// Project identifier (IRI).
    pub iri: String,
    /// Installed version, if available.
    pub version: Option<String>,
}
