// SPDX-FileCopyrightText: © 2026 Sysand contributors <opensource@sensmetry.com>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Options structs for facade functions. Each command has its own
//! options struct containing optional parameters.

use camino::Utf8PathBuf;

use super::enums::*;

/// Options for `init`.
#[derive(Debug, Clone, Default)]
pub struct InitOptions {
    pub name: Option<String>,
    pub publisher: Option<String>,
    pub version: Option<String>,
    pub license: Option<String>,
    pub allow_non_spdx: bool,
}

/// Options for `clone`.
#[derive(Debug, Clone, Default)]
pub struct CloneOptions {
    pub target: Option<Utf8PathBuf>,
    pub version: Option<String>,
    pub deps: DepsMode,
    pub include_std: bool,
    pub index: IndexOptions,
}

/// Options for `source::add`.
#[derive(Debug, Clone, Default)]
pub struct SourceAddOptions {
    pub checksum: ChecksumMode,
    pub index_symbols: IndexSymbols,
    pub language: Language,
}

/// Options for `usage::add`.
#[derive(Debug, Clone, Default)]
pub struct UsageAddOptions {
    pub version_req: Option<String>,
    pub source: Option<SourceSpec>,
    pub update: UpdateMode,
    pub include_std: bool,
    pub index: IndexOptions,
}

/// Options for `build` and `workspace::build`.
#[derive(Debug, Clone, Default)]
pub struct BuildOptions {
    pub target: Option<Utf8PathBuf>,
    pub compression: Compression,
    pub allow_path_usage: bool,
}

/// Options for `lock::update`.
#[derive(Debug, Clone, Default)]
pub struct LockUpdateOptions {
    pub include_std: bool,
    pub index: IndexOptions,
}

/// Options for `env::create`.
#[derive(Debug, Clone, Default)]
pub struct EnvCreateOptions {
    pub env: Option<Utf8PathBuf>,
}

/// Options for `env::sync`.
#[derive(Debug, Clone, Default)]
pub struct EnvSyncOptions {
    pub env: Option<Utf8PathBuf>,
    pub include_std: bool,
    pub index: IndexOptions,
}

/// Options for `env::install`.
#[derive(Debug, Clone, Default)]
pub struct EnvInstallOptions {
    pub version_req: Option<String>,
    pub env: Option<Utf8PathBuf>,
    pub source: Option<SourceSpec>,
    pub allow_overwrite: bool,
    pub allow_multiple: bool,
    pub deps: DepsMode,
    pub include_std: bool,
    pub index: IndexOptions,
}

/// Options for `env::uninstall`.
#[derive(Debug, Clone, Default)]
pub struct EnvUninstallOptions {
    pub version_req: Option<String>,
    pub env: Option<Utf8PathBuf>,
}

/// Options for `env::list`.
#[derive(Debug, Clone, Default)]
pub struct EnvListOptions {
    pub env: Option<Utf8PathBuf>,
}

/// Shared options for commands that query indexes.
#[derive(Debug, Clone, Default)]
pub struct IndexOptions {
    pub indexes: Vec<String>,
    pub default_indexes: Vec<String>,
    pub index_mode: IndexModeOption,
}

/// Paired source kind + value for source overrides.
#[derive(Debug, Clone)]
pub struct SourceSpec {
    pub kind: SourceKind,
    pub value: String,
}
