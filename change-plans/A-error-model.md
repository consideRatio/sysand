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
| `RemoveError::MissingInfo`                | `FieldRequired`                               |
| `IncludeError::Extract(path, err)`        | `BuildFailed` (2-field variant)               |
| `IncludeError::UnknownFormat`             | `FieldInvalid`                                |
| `ExcludeError::SourceNotFound`            | `PathNotFound` (not `NotFound`)               |
| `KParBuildError::Validation`              | `SchemaInvalid`                               |
| `KParBuildError::PathUsage`               | `FieldInvalid`                                |
| `KParBuildError::IncompleteSource`        | `FieldRequired`                               |
| `KParBuildError::MissingInfo`             | `FieldRequired`                               |
| `KParBuildError::MissingMeta`             | `FieldRequired`                               |
| `KParBuildError::Extract`                 | `BuildFailed`                                 |
| `KParBuildError::UnknownFormat`           | `FieldInvalid`                                |
| `KParBuildError::Serialize`               | `Internal`                                    |
| `KParBuildError::WorkspaceMetamodelConflict` | `ConfigInvalid`                            |
| `KParBuildError::Zip`                     | `BuildFailed`                                 |
| `KParBuildError::Io`                      | `IoError`                                     |
| `KParBuildError::ProjectRead`             | delegates to `P::into()`                      |
| `KParBuildError::WorkspaceRead`           | delegates to `WorkspaceReadError` impl        |
| `KParBuildError::LocalSrc`                | delegates to `LocalSrcError` impl             |
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
| `LocalSrcError::Path`                     | `FieldInvalid`                                |
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
| `GixDownloadedError::Fetch`               | `IndexUnreachable`                            |
| `GixDownloadedError::Checkout`            | `IndexUnreachable`                            |
| `GixDownloadedError::UrlParse`            | `IriInvalid`                                  |
| `GixDownloadedError::Other`               | `Internal`                                    |
| `ReqwestKparDownloadedError::BadHttpStatus` | `AuthFailed` / `ProjectNotInIndex` / `IndexUnreachable` (by status code) |
| `ReqwestKparDownloadedError::ParseUrl`    | `IriInvalid`                                  |
| `ReqwestKparDownloadedError::Reqwest`     | `IndexUnreachable`                            |
| `ReqwestKparDownloadedError::ReqwestMiddleware` | `IndexUnreachable`                       |
| `ReqwestKparDownloadedError::KPar`        | delegates to `LocalKParError` impl            |
| `SyncError::InvalidRemoteSource`          | `IriInvalid`                                  |
| `ReqwestSrcError::Reqwest` (401/403)      | `AuthFailed`                                  |
| `ReqwestSrcError::Reqwest` (5xx)          | `IndexUnreachable`                            |
| `HTTPEnvironmentError::Request` (401/403) | `AuthFailed`                                  |
| `ExtractError::SyntaxError`               | `BuildFailed`                                 |
| `CliError::*`                             | Various (ProjectNotFound, FieldInvalid, etc.) |

## Steps

### Step 1: Define `SysandError` and `ErrorCode`

Create `core/src/error.rs` (not `types/error.rs` — crate restructure
is plan B; for now it lives at the top level). Implement `Display`,
`Error`, `From` conversions. Add helper constructors:

```rust
impl SysandError {
    pub fn new(code: ErrorCode, message: impl Into<String>) -> Self { ... }
    pub fn with_context(code: ErrorCode, message: impl Into<String>, context: impl Into<String>) -> Self { ... }
}
```

Also add `FsIoError::path_context() -> Option<String>` helper to
extract path info from I/O errors for the `context` field.

### Step 2: Add `From` impls for existing error types

For each existing error type, implement `From<ExistingError> for
SysandError`. This lets the facade layer use `?` to convert. Start
with the leaf types (no generics):

- `From<Box<FsIoError>>` — map all variants to `IoError` with path context
  (note: `Box<FsIoError>`, not `FsIoError` — that's how it's stored)
- `From<PathError>` — map to `FieldInvalid` (path safety violations)
- `From<ConfigReadError>` — map to `ConfigInvalid` / `IoError`
- `From<WorkspaceReadError>` — map to `SchemaInvalid` / `IoError`
- `From<ValidationError>` — map per variant
- `From<ParseError>` — map to `SchemaInvalid`
- `From<ExtractError>` — map to `BuildFailed`
- `From<LocalSrcError>` — map per variant (delegates to leaf impls)
- `From<LocalKParError>` — map per variant
- `From<LocalReadError>` — map to `IoError`
- `From<LocalWriteError>` — map per variant (delegates to leaf impls)
- `From<MemoryReadError>` — map per variant
- `From<InterchangeProjectValidationError>` — map to `SchemaInvalid`
- `From<ReqwestKparDownloadedError>` — map per variant (HTTP status
  code inspection for `AuthFailed` vs `IndexUnreachable`)
- `From<GixDownloadedError>` — map per variant

Then the generic command error types. **Critical:** these need
`std::error::Error + Send + Sync + 'static` bounds on the generic
param, not just `Into<SysandError>`, because the error enums require
`ErrorBound` on their type params:

```rust
impl<P: std::error::Error + Send + Sync + 'static + Into<SysandError>>
    From<InitError<P>> for SysandError { ... }
```

Generic command impls:
- `From<InitError<P>>` — map per variant
- `From<AddError<P>>` — map per variant
- `From<RemoveError<P>>` — map per variant
- `From<IncludeError<P>>` — map per variant
- `From<ExcludeError<P>>` — map per variant
- `From<KParBuildError<P>>` — map all 14 variants explicitly (no wildcard)
- `From<EnvError<W>>` — map per variant

**Feature gates:** Many types are behind `#[cfg(feature = "...")]`:
- `filesystem`: `LocalSrcError`, `PathError`, `LocalKParError`,
  `ZipArchiveError`, `LocalReadError`, `LocalWriteError`,
  `ConfigReadError`, `WorkspaceReadError`, `KParBuildError`
- `filesystem + networking`: `ReqwestKparDownloadedError`,
  `GixDownloadedError`

Every `From` impl must have the matching `#[cfg(...)]` gate.

**Variant name gotchas** (differ from what you might expect):
- `ExcludeError::SourceNotFound` (not `NotFound`)
- `RemoveError::MissingInfo` (not `ProjectInfoNotFound`)
- `IncludeError::Extract(Box<str>, ExtractError)` — 2 fields, not 1
- `KParBuildError::WorkspaceMetamodelConflict { .. }` — struct variant
  with 3 named fields

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

- **Feature gate combinatorics:** From impls must match the exact
  `#[cfg(...)]` of the error type they convert. Test with both
  `cargo check --package sysand-core` (default features = std only)
  and `cargo check --package sysand-core --features filesystem,networking`
  to catch mismatched gates.

## Size Estimate

- New code: ~590 lines (SysandError + ErrorCode + From impls — much
  larger than initially estimated because each From impl with
  per-variant matching is 10-20 lines)
- Helper: `FsIoError::path_context()` ~25 lines added to
  `project/utils.rs`
- Modified: facade functions (thin wrappers), all 3 bindings
- Deleted: ~500 lines of error boilerplate across bindings
- Net: roughly neutral on step 1-2; net reduction comes in steps 3-5
