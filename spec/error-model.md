# Error Model

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

| Surface | Shape                                                                 |
| ------- | --------------------------------------------------------------------- |
| Rust    | `Result<T, SysandError>` with `ErrorCode` enum                        |
| Java    | throws `SysandException` with `ErrorCode` field                       |
| JS/WASM | throws `SysandError` with `code` property (`"kebab-case"` string)     |
| Python  | raises `SysandError` subclass per code (`ProjectNotFoundError`, etc.) |

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

    // Index / network
    IndexUnreachable,
    ProjectNotInIndex,
    VersionNotInIndex,

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

## Rationale

**Why a flat enum, not per-command exceptions.** Per-command exception
classes (e.g., `ProjectInitError`, `UsageAddError`) multiply the error
surface across four binding surfaces and make catch-site code fragile —
callers need to know which exception each command throws. A single
`SysandError` with a flat `ErrorCode` enum projects cleanly everywhere:
one exception class in Java, one error type in JS, subclasses per code
in Python. Callers match on the code, not the command.

**Why the same codes everywhere.** The error code enum is identical
across all surfaces so that documentation, logging, and user-facing
messages are consistent regardless of how sysand was invoked.
