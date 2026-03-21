# Error Model

Sources: ADR-0005, exploration 0009

## SysandError

```rust
pub struct SysandError {
    pub code: ErrorCode,
    pub message: String,
    pub context: Option<String>,
}
```

`context` provides location info — which file, field, or IRI caused
the error. Optional because some errors are context-free.

## Binding Projections

| Surface | Shape |
| ------- | ----- |
| Rust | `Result<T, SysandError>` with `ErrorCode` enum |
| Java | throws `SysandException` with `ErrorCode` field |
| JS/WASM | throws `SysandError` with `code` property (`"kebab-case"` string) |
| Python | raises `SysandError` subclass per code (`ProjectNotFoundError`, etc.) |

Error codes are the same enum everywhere. No per-command exception
classes.

## ErrorCode — Draft

```rust
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

    // Lookup / network
    IndexUnreachable,
    PackageNotFound,
    VersionNotFound,

    // Build
    BuildFailed,

    // Lock
    LockStale,

    // Generic
    IoError,
    Internal,
}
```

This is a flat enum — no hierarchy.
