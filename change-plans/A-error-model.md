# A. Unify Error Model

Replace ~57 per-module error enums with a single `SysandError` + flat
`ErrorCode` enum. This is foundational — the facade layer (B) and
bindings (C) depend on it.

## Current State

The reference implementation has error types at every layer:

**Command errors** (generic over project/env error types):

- `InitError<P>` — SemVerParse, SPDXLicenseParse, Project(P)
- `AddError<P>` — Project(P), Validation, MissingInfo
- `RemoveError<P>` — Project(P), UsageNotFound, ProjectInfoNotFound
- `IncludeError<P>` — Project(P), Io, Extract, UnknownFormat
- `ExcludeError<P>` — Project(P), Io, NotFound
- `KParBuildError<P>` — 12 variants including ProjectRead, Zip, Io,
  Validation, Serialize, PathUsage
- `SyncError<U, G>` — BadChecksum, BadProject, NoKnownSources,
  SourceResolution, ProjectDownload
- `LockError<PD, R>` — 7 variants, deeply nested generics
- `LockProjectError<PI, PD, R>` — wraps LockError with input project
  errors
- `InfoError<E>` — NoResolve, UnsupportedIri, Resolution
- `EnvError<W>` — AlreadyExists, Write(W)
- `EnvInstallError<ER, PR, IE>` — 3 generic params, deeply nested
  associated types

**Project errors:**

- `LocalSrcError` — 7 variants
- `LocalKParError` — 5 variants
- `IntoKparError<R>` — 6 variants
- `ReqwestSrcError` — 4 variants
- `ReqwestKparDownloadedError` — 3 variants
- `GixDownloadedError` — 4 variants
- `InMemoryError` — 3 variants
- `FsIoError` — 15 variants (detailed path context)

**Environment errors:**

- `LocalReadError` — 3 variants
- `LocalWriteError` — 8 variants
- `HTTPEnvironmentError` — 3 variants
- `MemoryReadError` — 2 variants
- `MemoryWriteError` — empty

**Resolver errors:**

- `FileResolverError` — 4 variants
- `FileResolverProjectError` — 7 variants
- `GitResolverError` — transparent wrapper
- `RemoteResolverError<H, G>` — 2 variants
- `CombinedResolverError<F, L, R, Reg>` — 4 generic params

**Other:**

- `ConfigReadError` — 2 variants
- `ValidationError` (lock) — 6 variants
- `ParseError` (lock) — 3 variants
- `ExtractError` (symbols) — 5 variants
- `WorkspaceReadError` — 3 variants
- `PathError` — 2 variants
- `CliError` — 5 variants

## Target State

```rust
// types/error.rs
pub struct SysandError {
    pub code: ErrorCode,
    pub message: String,
    pub context: Option<String>,
}

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
```

## Migration Mapping

How current error variants map to ErrorCode:

| Current variant                           | ErrorCode                                     |
| ----------------------------------------- | --------------------------------------------- |
| `InitError::SemVerParse`                  | `VersionInvalid`                              |
| `InitError::SPDXLicenseParse`             | `LicenseInvalid`                              |
| `InitError::Project(AlreadyExists)`       | `FieldInvalid` (project exists)               |
| `AddError::Validation`                    | `SchemaInvalid`                               |
| `AddError::MissingInfo`                   | `FieldRequired`                               |
| `RemoveError::UsageNotFound`              | `UsageNotFound`                               |
| `RemoveError::ProjectInfoNotFound`        | `ProjectNotFound`                             |
| `IncludeError::Extract`                   | `BuildFailed`                                 |
| `IncludeError::UnknownFormat`             | `FieldInvalid`                                |
| `ExcludeError::NotFound`                  | `PathNotFound`                                |
| `KParBuildError::*` (most)                | `BuildFailed`                                 |
| `KParBuildError::Validation`              | `SchemaInvalid`                               |
| `KParBuildError::PathUsage`               | `FieldInvalid`                                |
| `SyncError::BadChecksum`                  | `EnvCorrupted`                                |
| `SyncError::BadProject`                   | `SchemaInvalid`                               |
| `SyncError::NoKnownSources`               | `ProjectNotInIndex`                           |
| `LockError::Solver(NoSolution)`           | `ResolutionFailed`                            |
| `LockError::Solver(NotFound)`             | `ProjectNotInIndex`                           |
| `LockError::Solver(NoValidCandidates)`    | `VersionNotInIndex`                           |
| `LockError::Solver(UnsupportedIriType)`   | `IriInvalid`                                  |
| `LockError::NameCollision`                | `UsageCycle`                                  |
| `LockError::Validation`                   | `SchemaInvalid`                               |
| `InfoError::NoResolve`                    | `ProjectNotFound`                             |
| `InfoError::UnsupportedIri`               | `FieldInvalid`                                |
| `EnvError::AlreadyExists`                 | `EnvConflict`                                 |
| `LocalSrcError::AlreadyExists`            | `EnvConflict`                                 |
| `LocalSrcError::MissingMeta`              | `FieldRequired`                               |
| `LocalSrcError::Io`                       | `IoError`                                     |
| `LocalSrcError::Path`                     | `PathNotFound`                                |
| `LocalSrcError::Deserialize`              | `SchemaInvalid`                               |
| `LocalSrcError::Serialize`                | `Internal`                                    |
| `FsIoError::*` (all)                      | `IoError`                                     |
| `ConfigReadError::Toml`                   | `ConfigInvalid`                               |
| `ConfigReadError::Io`                     | `ConfigNotFound` or `IoError`                 |
| `ValidationError::NameCollision`          | `UsageCycle`                                  |
| `ValidationError::UnsatisfiedUsage`       | `VersionNotInIndex`                           |
| `ParseError::*`                           | `SchemaInvalid`                               |
| `WorkspaceReadError::*`                   | `WorkspaceNotFound` or `SchemaInvalid`        |
| `HTTPEnvironmentError::NotFound`          | `ProjectNotInIndex`                           |
| `HTTPEnvironmentError::Request`           | `IndexUnreachable`                            |
| `ReqwestSrcError::Reqwest`                | `IndexUnreachable`                            |
| `GixDownloadedError::Clone`               | `IndexUnreachable`                            |
| `GixDownloadedError::UrlParse`            | `IriInvalid`                                  |
| `SyncError::InvalidRemoteSource`          | `IriInvalid`                                  |
| `ReqwestSrcError::Reqwest` (401/403)      | `AuthFailed`                                  |
| `ReqwestSrcError::Reqwest` (5xx)          | `IndexUnreachable`                            |
| `HTTPEnvironmentError::Request` (401/403) | `AuthFailed`                                  |
| `ExtractError::SyntaxError`               | `BuildFailed`                                 |
| `CliError::*`                             | Various (ProjectNotFound, FieldInvalid, etc.) |

## Steps

### Step 1: Define `SysandError` and `ErrorCode`

Create `core/src/types/error.rs` with the types above. Implement
`Display`, `Error`, `From` conversions. Add helper constructors:

```rust
impl SysandError {
    pub fn new(code: ErrorCode, message: impl Into<String>) -> Self { ... }
    pub fn with_context(code: ErrorCode, message: impl Into<String>, context: impl Into<String>) -> Self { ... }
}
```

### Step 2: Add `From` impls for existing error types

For each existing error type, implement `From<ExistingError> for
SysandError`. This lets the facade layer use `?` to convert. Start
with the leaf types (no generics):

- `From<FsIoError>` — map all variants to `IoError` with path context
- `From<PathError>` — map to `PathNotFound`
- `From<ConfigReadError>` — map to `ConfigInvalid` / `ConfigNotFound`
- `From<WorkspaceReadError>` — map to `WorkspaceNotFound` / `SchemaInvalid`
- `From<ValidationError>` — map per variant
- `From<ParseError>` — map to `SchemaInvalid`
- `From<ExtractError>` — map to `BuildFailed`

Then the generic types (parameterized by project error):

- `From<InitError<impl Into<SysandError>>>` — map per variant
- `From<AddError<impl Into<SysandError>>>` — map per variant
- etc.

### Step 3: Update facade functions to return `Result<T, SysandError>`

Each facade function wraps the internal call with `.map_err(SysandError::from)`:

```rust
pub fn init(project: &mut impl ProjectMut, opts: InitOptions) -> Result<(), SysandError> {
    internal::commands::init::do_init_ext(/* ... */)
        .map_err(SysandError::from)
}
```

The internal functions keep their existing error types — only the
facade boundary converts.

### Step 4: Update binding error mapping

**Java:** Replace 10 exception classes with single `SysandException`:

```java
public class SysandException extends Exception {
    private final ErrorCode code;
    public ErrorCode getCode() { return code; }
}
```

The Rust JNI code becomes a single `throw_sysand_exception(env, error)`
helper instead of per-variant matching.

**Python:** Replace stdlib exception mapping with single `SysandError`:

```rust
fn into_py_err(err: SysandError) -> PyErr {
    let code = err.code.as_str(); // e.g., "version-invalid"
    PyErr::new::<SysandErrorType, _>((code, err.message, err.context))
}
```

**JS/WASM:** Already close — just ensure thrown object has `code`,
`message`, `context` fields.

### Step 5: Clean up

- Remove old error enums from `internal/` (or leave them if internal
  code still uses them for logic — they just don't cross the facade)
- Remove Java exception hierarchy classes
- Remove Python per-exception mapping
- Update tests to catch `SysandError`/`SysandException` with code checks

## Risks

- **Information loss:** Some current error variants carry structured
  data (e.g., `FsIoError` has path fields, `NameCollisionError` has
  symbol + 2 project names). This becomes string `message` + `context`.
  Acceptable — callers don't inspect these fields programmatically.

- **Internal error matching breaks:** If any internal code matches on
  command-level error variants for control flow, those matches need
  updating. The spec says this shouldn't happen, but verify by grep.

## Size Estimate

- New code: ~200 lines (SysandError + ErrorCode + From impls)
- Modified: facade functions (thin wrappers), all 3 bindings
- Deleted: ~500 lines of error boilerplate across bindings
- Net reduction in code
