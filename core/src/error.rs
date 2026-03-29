// SPDX-FileCopyrightText: © 2026 Sysand contributors <opensource@sensmetry.com>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Unified error type for the sysand public API.
//!
//! `SysandError` is the single error type returned by all facade functions.
//! Internal code keeps its own error types; conversion to `SysandError`
//! happens at the facade boundary via `From` implementations.

use std::fmt;

/// Error codes for all sysand operations. Flat enum — no hierarchy.
///
/// These codes are projected to all binding surfaces:
/// - Rust: `ErrorCode` enum
/// - Java: `ErrorCode` enum
/// - JS/WASM: kebab-case string (e.g., `"project-not-found"`)
/// - Python: `ErrorCode` enum
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ErrorCode {
    // Discovery / path
    ProjectNotFound,
    WorkspaceNotFound,
    PathNotFound,
    PathNotAProject,
    PathNotAWorkspace,

    // Config
    ConfigNotFound,
    ConfigInvalid,

    // Schema / validation
    SchemaInvalid,
    FieldRequired,
    FieldInvalid,
    VersionInvalid,
    LicenseInvalid,

    // Usages
    UsageNotFound,
    UsageAlreadyExists,
    UsageCycle,

    // Environment
    EnvNotFound,
    EnvCorrupted,
    EnvConflict,

    // Index / network
    IndexUnreachable,
    ProjectNotInIndex,
    VersionNotInIndex,

    // Build
    BuildFailed,

    // Lock
    LockStale,

    // Resolution
    ResolutionFailed,
    IriInvalid,

    // Auth
    AuthFailed,

    // Generic
    IoError,
    Internal,
}

impl ErrorCode {
    /// Returns the kebab-case string representation used by JS/WASM bindings.
    pub fn as_kebab_str(self) -> &'static str {
        match self {
            Self::ProjectNotFound => "project-not-found",
            Self::WorkspaceNotFound => "workspace-not-found",
            Self::PathNotFound => "path-not-found",
            Self::PathNotAProject => "path-not-a-project",
            Self::PathNotAWorkspace => "path-not-a-workspace",
            Self::ConfigNotFound => "config-not-found",
            Self::ConfigInvalid => "config-invalid",
            Self::SchemaInvalid => "schema-invalid",
            Self::FieldRequired => "field-required",
            Self::FieldInvalid => "field-invalid",
            Self::VersionInvalid => "version-invalid",
            Self::LicenseInvalid => "license-invalid",
            Self::UsageNotFound => "usage-not-found",
            Self::UsageAlreadyExists => "usage-already-exists",
            Self::UsageCycle => "usage-cycle",
            Self::EnvNotFound => "env-not-found",
            Self::EnvCorrupted => "env-corrupted",
            Self::EnvConflict => "env-conflict",
            Self::IndexUnreachable => "index-unreachable",
            Self::ProjectNotInIndex => "project-not-in-index",
            Self::VersionNotInIndex => "version-not-in-index",
            Self::BuildFailed => "build-failed",
            Self::LockStale => "lock-stale",
            Self::ResolutionFailed => "resolution-failed",
            Self::IriInvalid => "iri-invalid",
            Self::AuthFailed => "auth-failed",
            Self::IoError => "io-error",
            Self::Internal => "internal",
        }
    }
}

impl fmt::Display for ErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_kebab_str())
    }
}

/// The unified error type for all sysand facade operations.
///
/// All facade functions return `Result<T, SysandError>`. Internal error
/// types are converted to `SysandError` at the facade boundary.
#[derive(Debug)]
pub struct SysandError {
    pub code: ErrorCode,
    pub message: String,
    pub context: Option<String>,
}

impl SysandError {
    pub fn new(code: ErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            context: None,
        }
    }

    pub fn with_context(
        code: ErrorCode,
        message: impl Into<String>,
        context: impl Into<String>,
    ) -> Self {
        Self {
            code,
            message: message.into(),
            context: Some(context.into()),
        }
    }
}

impl fmt::Display for SysandError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}] {}", self.code, self.message)?;
        if let Some(ctx) = &self.context {
            write!(f, " ({})", ctx)?;
        }
        Ok(())
    }
}

impl std::error::Error for SysandError {}

// ---------------------------------------------------------------------------
// From implementations for internal error types
//
// These convert internal errors to SysandError at the facade boundary.
// Organized by source module.
// ---------------------------------------------------------------------------

// -- Project I/O errors --

impl From<Box<crate::project::utils::FsIoError>> for SysandError {
    fn from(err: Box<crate::project::utils::FsIoError>) -> Self {
        let context = err.path_context();
        Self {
            code: ErrorCode::IoError,
            message: err.to_string(),
            context,
        }
    }
}

impl From<crate::project::utils::ProjectDeserializationError> for SysandError {
    fn from(err: crate::project::utils::ProjectDeserializationError) -> Self {
        Self::new(ErrorCode::SchemaInvalid, err.to_string())
    }
}

impl From<crate::project::utils::ProjectSerializationError> for SysandError {
    fn from(err: crate::project::utils::ProjectSerializationError) -> Self {
        Self::new(ErrorCode::Internal, err.to_string())
    }
}

#[cfg(feature = "filesystem")]
impl From<crate::project::utils::ZipArchiveError> for SysandError {
    fn from(err: crate::project::utils::ZipArchiveError) -> Self {
        Self::new(ErrorCode::BuildFailed, err.to_string())
    }
}

impl From<crate::project::utils::RelativizePathError> for SysandError {
    fn from(err: crate::project::utils::RelativizePathError) -> Self {
        Self::new(ErrorCode::PathNotFound, err.to_string())
    }
}

#[cfg(feature = "filesystem")]
impl From<crate::project::local_src::PathError> for SysandError {
    fn from(err: crate::project::local_src::PathError) -> Self {
        Self::new(ErrorCode::FieldInvalid, err.to_string())
    }
}

#[cfg(feature = "filesystem")]
impl From<crate::project::local_src::LocalSrcError> for SysandError {
    fn from(err: crate::project::local_src::LocalSrcError) -> Self {
        use crate::project::local_src::LocalSrcError;
        match err {
            LocalSrcError::AlreadyExists(path) => {
                Self::with_context(ErrorCode::EnvConflict, "project already exists", path)
            }
            LocalSrcError::MissingMeta => {
                Self::new(ErrorCode::FieldRequired, "missing .meta.json")
            }
            LocalSrcError::Deserialize(e) => Self::from(e),
            LocalSrcError::Serialize(e) => Self::from(e),
            LocalSrcError::Path(e) => Self::from(e),
            LocalSrcError::Io(e) => Self::from(e),
            LocalSrcError::ImpossibleRelativePath(e) => Self::from(e),
        }
    }
}

#[cfg(feature = "filesystem")]
impl From<crate::project::local_kpar::LocalKParError> for SysandError {
    fn from(err: crate::project::local_kpar::LocalKParError) -> Self {
        use crate::project::local_kpar::LocalKParError;
        match err {
            LocalKParError::Zip(e) => Self::from(e),
            LocalKParError::NotFound(path) => {
                Self::with_context(ErrorCode::PathNotFound, "file not found in archive", path.to_string())
            }
            LocalKParError::Deserialize(e) => Self::from(e),
            LocalKParError::Io(e) => Self::from(e),
            LocalKParError::ImpossibleRelativePath(e) => Self::from(e),
        }
    }
}

// -- Environment errors --

#[cfg(feature = "filesystem")]
impl From<crate::env::local_directory::LocalReadError> for SysandError {
    fn from(err: crate::env::local_directory::LocalReadError) -> Self {
        Self::new(ErrorCode::IoError, err.to_string())
    }
}

#[cfg(feature = "filesystem")]
impl From<crate::env::local_directory::LocalWriteError> for SysandError {
    fn from(err: crate::env::local_directory::LocalWriteError) -> Self {
        use crate::env::local_directory::LocalWriteError;
        match err {
            LocalWriteError::AlreadyExists(path) => {
                Self::with_context(ErrorCode::EnvConflict, "already exists in environment", path)
            }
            LocalWriteError::MissingMeta => {
                Self::new(ErrorCode::FieldRequired, "missing .meta.json")
            }
            LocalWriteError::Deserialize(e) => Self::from(e),
            LocalWriteError::Serialize(e) => Self::from(e),
            LocalWriteError::Path(e) => Self::from(e),
            LocalWriteError::Io(e) => Self::from(e),
            LocalWriteError::TryMove(e) => Self::new(ErrorCode::IoError, e.to_string()),
            LocalWriteError::LocalRead(e) => Self::from(e),
            LocalWriteError::ImpossibleRelativePath(e) => Self::from(e),
        }
    }
}

impl From<crate::env::memory::MemoryReadError> for SysandError {
    fn from(err: crate::env::memory::MemoryReadError) -> Self {
        use crate::env::memory::MemoryReadError;
        match err {
            MemoryReadError::MissingProject(iri) => {
                Self::with_context(ErrorCode::ProjectNotInIndex, "project not found", iri)
            }
            MemoryReadError::MissingVersion(iri, ver) => Self::with_context(
                ErrorCode::VersionNotInIndex,
                format!("version {ver} not found"),
                iri,
            ),
        }
    }
}

// -- Config errors --

#[cfg(feature = "filesystem")]
impl From<crate::config::local_fs::ConfigReadError> for SysandError {
    fn from(err: crate::config::local_fs::ConfigReadError) -> Self {
        use crate::config::local_fs::ConfigReadError;
        match err {
            ConfigReadError::Toml(path, e) => Self::with_context(
                ErrorCode::ConfigInvalid,
                e.to_string(),
                path.to_string(),
            ),
            ConfigReadError::Io(e) => Self::from(e),
        }
    }
}

// -- Lock errors --

impl From<crate::lock::ParseError> for SysandError {
    fn from(err: crate::lock::ParseError) -> Self {
        Self::new(ErrorCode::SchemaInvalid, format!("lockfile parse error: {err}"))
    }
}

impl From<crate::lock::ValidationError> for SysandError {
    fn from(err: crate::lock::ValidationError) -> Self {
        use crate::lock::ValidationError;
        match err {
            ValidationError::NameCollision(_) => {
                Self::new(ErrorCode::UsageCycle, err.to_string())
            }
            ValidationError::UnsatisfiedUsage { .. } => {
                Self::new(ErrorCode::VersionNotInIndex, err.to_string())
            }
            _ => Self::new(ErrorCode::SchemaInvalid, err.to_string()),
        }
    }
}

// -- Workspace errors --

#[cfg(feature = "filesystem")]
impl From<crate::workspace::WorkspaceReadError> for SysandError {
    fn from(err: crate::workspace::WorkspaceReadError) -> Self {
        use crate::workspace::WorkspaceReadError;
        match err {
            WorkspaceReadError::Io(e) => Self::from(e),
            WorkspaceReadError::Deserialize(e) => {
                Self::new(ErrorCode::SchemaInvalid, e.to_string())
            }
            WorkspaceReadError::Validation(path, e) => Self::with_context(
                ErrorCode::SchemaInvalid,
                e.to_string(),
                path.to_string(),
            ),
        }
    }
}

// -- Symbol extraction errors --

impl From<crate::symbols::ExtractError> for SysandError {
    fn from(err: crate::symbols::ExtractError) -> Self {
        Self::new(ErrorCode::BuildFailed, err.to_string())
    }
}

// -- Command errors --

impl<P: std::error::Error + Send + Sync + 'static + Into<SysandError>> From<crate::commands::init::InitError<P>> for SysandError {
    fn from(err: crate::commands::init::InitError<P>) -> Self {
        use crate::commands::init::InitError;
        match err {
            InitError::SemVerParse(input, e) => Self::with_context(
                ErrorCode::VersionInvalid,
                e.to_string(),
                input.to_string(),
            ),
            InitError::SPDXLicenseParse(input, e) => Self::with_context(
                ErrorCode::LicenseInvalid,
                e.to_string(),
                input.to_string(),
            ),
            InitError::Project(e) => e.into(),
        }
    }
}

impl<P: std::error::Error + Send + Sync + 'static + Into<SysandError>> From<crate::commands::add::AddError<P>> for SysandError {
    fn from(err: crate::commands::add::AddError<P>) -> Self {
        use crate::commands::add::AddError;
        match err {
            AddError::Project(e) => e.into(),
            AddError::Validation(e) => {
                Self::new(ErrorCode::SchemaInvalid, e.to_string())
            }
            AddError::MissingInfo(msg) => {
                Self::new(ErrorCode::FieldRequired, msg.to_string())
            }
        }
    }
}

impl<P: std::error::Error + Send + Sync + 'static + Into<SysandError>> From<crate::commands::remove::RemoveError<P>> for SysandError {
    fn from(err: crate::commands::remove::RemoveError<P>) -> Self {
        use crate::commands::remove::RemoveError;
        match err {
            RemoveError::Project(e) => e.into(),
            RemoveError::UsageNotFound(iri) => {
                Self::with_context(ErrorCode::UsageNotFound, "usage not found", iri.to_string())
            }
            RemoveError::MissingInfo(msg) => {
                Self::new(ErrorCode::FieldRequired, msg.to_string())
            }
        }
    }
}

impl<P: std::error::Error + Send + Sync + 'static + Into<SysandError>> From<crate::commands::include::IncludeError<P>> for SysandError {
    fn from(err: crate::commands::include::IncludeError<P>) -> Self {
        use crate::commands::include::IncludeError;
        match err {
            IncludeError::Project(e) => e.into(),
            IncludeError::Io(e) => Self::from(e),
            IncludeError::Extract(path, e) => Self::with_context(
                ErrorCode::BuildFailed,
                e.to_string(),
                path.to_string(),
            ),
            IncludeError::UnknownFormat(path) => Self::with_context(
                ErrorCode::FieldInvalid,
                "unknown file format",
                path.to_string(),
            ),
        }
    }
}

impl<P: std::error::Error + Send + Sync + 'static + Into<SysandError>> From<crate::commands::exclude::ExcludeError<P>> for SysandError {
    fn from(err: crate::commands::exclude::ExcludeError<P>) -> Self {
        use crate::commands::exclude::ExcludeError;
        match err {
            ExcludeError::Project(e) => e.into(),
            ExcludeError::Io(e) => Self::from(e),
            ExcludeError::SourceNotFound(path) => Self::with_context(
                ErrorCode::PathNotFound,
                "file not found in metadata",
                path.to_string(),
            ),
        }
    }
}

#[cfg(feature = "filesystem")]
impl<P: std::error::Error + Send + Sync + 'static + Into<SysandError>> From<crate::commands::build::KParBuildError<P>> for SysandError {
    fn from(err: crate::commands::build::KParBuildError<P>) -> Self {
        use crate::commands::build::KParBuildError;
        match err {
            KParBuildError::ProjectRead(e) => e.into(),
            KParBuildError::WorkspaceRead(e) => Self::from(e),
            KParBuildError::LocalSrc(e) => Self::from(e),
            KParBuildError::Io(e) => Self::from(e),
            KParBuildError::Validation(e) => Self::from(e),
            KParBuildError::Zip(e) => Self::from(e),
            KParBuildError::PathUsage(msg) => {
                Self::new(ErrorCode::FieldInvalid, msg)
            }
            KParBuildError::IncompleteSource(msg) => {
                Self::new(ErrorCode::FieldRequired, msg)
            }
            KParBuildError::MissingInfo => {
                Self::new(ErrorCode::FieldRequired, "missing .project.json")
            }
            KParBuildError::MissingMeta => {
                Self::new(ErrorCode::FieldRequired, "missing .meta.json")
            }
            KParBuildError::Extract(msg) => {
                Self::new(ErrorCode::BuildFailed, msg)
            }
            KParBuildError::UnknownFormat(path) => {
                Self::with_context(ErrorCode::FieldInvalid, "unknown file format", path.to_string())
            }
            KParBuildError::Serialize(msg, e) => {
                Self::with_context(ErrorCode::Internal, e.to_string(), msg)
            }
            e @ KParBuildError::WorkspaceMetamodelConflict { .. } => {
                Self::new(ErrorCode::ConfigInvalid, e.to_string())
            }
        }
    }
}

impl<W: std::error::Error + Send + Sync + 'static + Into<SysandError>> From<crate::commands::env::EnvError<W>> for SysandError {
    fn from(err: crate::commands::env::EnvError<W>) -> Self {
        use crate::commands::env::EnvError;
        match err {
            EnvError::AlreadyExists(path) => Self::with_context(
                ErrorCode::EnvConflict,
                "environment already exists",
                path.to_string(),
            ),
            EnvError::Write(e) => e.into(),
        }
    }
}

// -- Model validation errors --

impl From<crate::model::InterchangeProjectValidationError> for SysandError {
    fn from(err: crate::model::InterchangeProjectValidationError) -> Self {
        Self::new(ErrorCode::SchemaInvalid, err.to_string())
    }
}

// -- Networking errors --

#[cfg(all(feature = "filesystem", feature = "networking"))]
impl From<crate::project::reqwest_kpar_download::ReqwestKparDownloadedError> for SysandError {
    fn from(err: crate::project::reqwest_kpar_download::ReqwestKparDownloadedError) -> Self {
        use crate::project::reqwest_kpar_download::ReqwestKparDownloadedError;
        match err {
            ReqwestKparDownloadedError::BadHttpStatus { url, status } => {
                let code = if status.as_u16() == 401 || status.as_u16() == 403 {
                    ErrorCode::AuthFailed
                } else if status.is_client_error() {
                    ErrorCode::ProjectNotInIndex
                } else {
                    ErrorCode::IndexUnreachable
                };
                Self::with_context(code, format!("HTTP {status}"), url.to_string())
            }
            ReqwestKparDownloadedError::ParseUrl(url, e) => {
                Self::with_context(ErrorCode::IriInvalid, e.to_string(), url.to_string())
            }
            ReqwestKparDownloadedError::Reqwest(e) => {
                Self::new(ErrorCode::IndexUnreachable, e.to_string())
            }
            ReqwestKparDownloadedError::ReqwestMiddleware(e) => {
                Self::new(ErrorCode::IndexUnreachable, e.to_string())
            }
            ReqwestKparDownloadedError::KPar(e) => Self::from(e),
            ReqwestKparDownloadedError::Io(e) => Self::from(e),
        }
    }
}

#[cfg(all(feature = "filesystem", feature = "networking"))]
impl From<crate::project::gix_git_download::GixDownloadedError> for SysandError {
    fn from(err: crate::project::gix_git_download::GixDownloadedError) -> Self {
        use crate::project::gix_git_download::GixDownloadedError;
        match err {
            GixDownloadedError::Clone(ref url, _) | GixDownloadedError::Fetch(ref url, _) => {
                Self::with_context(ErrorCode::IndexUnreachable, err.to_string(), url.clone())
            }
            GixDownloadedError::Checkout(ref path, _) => {
                Self::with_context(ErrorCode::IndexUnreachable, err.to_string(), path.to_string())
            }
            GixDownloadedError::UrlParse(ref url, _) => {
                Self::with_context(ErrorCode::IriInvalid, err.to_string(), url.to_string())
            }
            GixDownloadedError::Io(e) => Self::from(e),
            GixDownloadedError::Path(e) => Self::from(e),
            GixDownloadedError::Deserialize(e) => Self::from(e),
            GixDownloadedError::Serialize(e) => Self::from(e),
            GixDownloadedError::ImpossibleRelativePath(e) => Self::from(e),
            GixDownloadedError::Other(msg) => Self::new(ErrorCode::Internal, msg),
        }
    }
}
