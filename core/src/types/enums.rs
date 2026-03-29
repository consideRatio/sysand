// SPDX-FileCopyrightText: © 2026 Sysand contributors <opensource@sensmetry.com>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Enums used in facade options and context types.

/// Controls how `sysand.toml` configuration is loaded.
#[derive(Debug, Clone, Default)]
pub enum ConfigMode {
    /// Discover and load config automatically.
    #[default]
    Auto,
    /// Load config from a specific file path.
    File(camino::Utf8PathBuf),
    /// Do not load any config.
    None,
}

/// Controls side-effects of `usage add`.
#[derive(Debug, Clone, Copy, Default)]
pub enum UpdateMode {
    /// Only update `.project.json` manifest.
    Manifest,
    /// Update manifest and re-run solver to update lockfile.
    Lock,
    /// Update manifest, lockfile, and sync environment (default).
    #[default]
    Sync,
}

/// Controls whether transitive usages are fetched.
#[derive(Debug, Clone, Copy, Default)]
pub enum DepsMode {
    /// Fetch all transitive dependencies (default).
    #[default]
    All,
    /// Do not fetch dependencies.
    None,
}

/// Controls whether default indexes are queried.
#[derive(Debug, Clone, Copy, Default)]
pub enum IndexModeOption {
    /// Use default indexes (default).
    #[default]
    Default,
    /// Do not use any indexes.
    None,
}

/// Controls checksum generation for source files.
#[derive(Debug, Clone, Copy, Default)]
pub enum ChecksumMode {
    /// No checksum (default).
    #[default]
    None,
    /// SHA-256 checksum.
    Sha256,
}

/// Controls whether symbols are indexed when adding sources.
#[derive(Debug, Clone, Copy, Default)]
pub enum IndexSymbols {
    /// Index symbols (default).
    #[default]
    On,
    /// Do not index symbols.
    Off,
}

/// Controls language detection for source files.
#[derive(Debug, Clone, Copy, Default)]
pub enum Language {
    /// Auto-detect from file extension (default).
    #[default]
    Auto,
    /// SysML v2.
    Sysml,
    /// KerML.
    Kerml,
}

/// Controls KPAR archive compression.
#[derive(Debug, Clone, Copy, Default)]
pub enum Compression {
    /// No compression.
    Stored,
    /// Deflate compression (default).
    #[default]
    Deflated,
    /// Bzip2 compression (feature-gated).
    Bzip2,
    /// Zstandard compression (feature-gated).
    Zstd,
    /// XZ/LZMA compression (feature-gated).
    Xz,
    /// PPMd compression (feature-gated).
    Ppmd,
}

/// Identifies the type of source location.
#[derive(Debug, Clone, Copy)]
pub enum SourceKind {
    /// Local project, changes picked up live.
    Editable,
    /// Local source directory.
    LocalSrc,
    /// Local `.kpar` archive.
    LocalKpar,
    /// Package index.
    Registry,
    /// Remote source directory (URL).
    RemoteSrc,
    /// Remote `.kpar` archive (URL).
    RemoteKpar,
    /// Git repository (URL).
    RemoteGit,
    /// API endpoint (URL).
    RemoteApi,
}
