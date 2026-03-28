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

## Internal Error Handling

`SysandError` is the only error type — internally and externally.
There are no per-module or per-command error enums.

Rust code constructs a `SysandError` at the point of failure using
the appropriate `ErrorCode`, a human-readable message, and optional
context. Propagation uses `Result<T, SysandError>` and `?`
throughout.

```rust
// At the point of failure — not at a module boundary
fn read_project(path: &Path) -> Result<Project, SysandError> {
    let content = fs::read_to_string(path).map_err(|e| SysandError {
        code: ErrorCode::IoError,
        message: format!("failed to read .project.json: {e}"),
        context: Some(path.display().to_string()),
    })?;
    // ...
}
```

**No error type hierarchies.** Do not define domain-specific error
enums (`LockError`, `SyncError`, `ResolverError`, etc.) or
parameterize errors by storage backend types. The `ErrorCode` enum
is the only discriminant. The `message` and `context` fields carry
diagnostic detail.

**No matching on errors in business logic.** Core commands propagate
errors with `?` and do not branch on `ErrorCode`. The only places
that inspect the code are:

- **Binding layers** — to map `ErrorCode` to the surface's exception
  type (e.g., `IoError` → `PyIOError`, `SchemaInvalid` →
  `PyValueError`).
- **Infrastructure code** — `std::io::ErrorKind` matches for
  expected conditions (file-not-found → return `Ok(false)`,
  cross-device rename → fallback to copy). These happen *before*
  constructing a `SysandError`, not after.

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

**Why no internal error hierarchies.** Domain-specific error enums
(per-module, per-command, generic over storage backends) add
maintenance cost without enabling control flow — in practice, errors
are propagated with `?` and never inspected within core business
logic. The diagnostic value comes from the message string at the
point of failure, not from the type. A single `SysandError`
constructed at the failure site carries the same information with
less machinery, and the binding translation layer becomes a simple
match on `ErrorCode` rather than an exhaustive match on dozens of
nested variants.
