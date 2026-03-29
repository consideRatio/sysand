// SPDX-FileCopyrightText: © 2026 Sysand contributors <opensource@sensmetry.com>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Context objects passed as the first argument to most facade functions.

use camino::Utf8PathBuf;

use super::enums::ConfigMode;

/// Context for project operations. Holds the project root path and
/// configuration mode. Facade functions that operate on a project
/// take this as their first argument.
#[derive(Debug, Clone)]
pub struct ProjectContext {
    pub path: Utf8PathBuf,
    pub config: ConfigMode,
}

impl ProjectContext {
    pub fn new(path: impl Into<Utf8PathBuf>) -> Self {
        Self {
            path: path.into(),
            config: ConfigMode::Auto,
        }
    }

    pub fn with_config(path: impl Into<Utf8PathBuf>, config: ConfigMode) -> Self {
        Self {
            path: path.into(),
            config,
        }
    }
}

/// Context for workspace operations. Holds the workspace root path.
#[derive(Debug, Clone)]
pub struct WorkspaceContext {
    pub path: Utf8PathBuf,
}

impl WorkspaceContext {
    pub fn new(path: impl Into<Utf8PathBuf>) -> Self {
        Self { path: path.into() }
    }
}
