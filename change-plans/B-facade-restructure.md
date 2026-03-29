# B. Facade & Crate Restructure

Reorganize `core/src/` into `facade/`, `types/`, `internal/` per
`spec/crate-structure.md`. Rename functions to match spec command tree.
Introduce options structs and simplify generic signatures.

Depends on: A (error model).

## Current State

All modules live at the top level of `core/src/` with everything `pub`.
Command functions use `do_` prefix, positional params, boolean flags,
and heavily generic signatures.

**Function naming:**

- `do_init` / `do_init_ext` / `do_init_local_file` / `do_init_memory`
- `do_add`
- `do_remove`
- `do_include`
- `do_exclude`
- `do_build_kpar` / `do_build_workspace_kpars`
- `do_lock_projects` / `do_lock_extend` / `do_lock_local_editable`
- `do_sync`
- `do_env_local_dir` / `do_env_memory`
- `do_env_install_project`
- `do_env_uninstall`
- `do_env_list`
- `do_info` / `do_info_project`
- `do_sources_project_no_deps` / `do_sources_local_src_project_no_deps`
- `do_root`

**Signature complexity ranges:**

- Simple: `do_add(project, usage)` — 2 params, 1 generic
- Complex: `do_sync(...)` — 13 type params, 7 closure factories
- Complex: `do_lock_projects(...)` — 4 type params with associated
  type constraints

## Target State

Per `spec/crate-structure.md` and `spec/public-api.md`:

```
core/src/
  lib.rs              pub use facade::*; pub use types::*;
  facade/
    mod.rs            re-exports submodules
    init.rs           pub fn init(project, InitOptions) -> Result<(), SysandError>
    locate.rs         pub fn locate(path) -> Result<Utf8PathBuf, SysandError>
    clone.rs          pub fn clone(locator, CloneOptions) -> Result<(), SysandError>
    build.rs          pub fn build(ctx, BuildOptions) -> Result<BuildOutput, SysandError>
    source.rs         pub mod source { pub fn add(...), pub fn remove(...) }
    usage.rs          pub mod usage { pub fn add(...), pub fn remove(...) }
    lock.rs           pub mod lock { pub fn update(...) }
    env.rs            pub mod env { pub fn create(...), sync(...), install(...), uninstall(...), list(...) }
    workspace.rs      pub mod workspace { pub fn locate(...), build(...) }
  types/
    mod.rs            re-exports
    context.rs        ProjectContext, WorkspaceContext
    enums.rs          ConfigMode, UpdateMode, DepsMode, Compression, ...
    options.rs        InitOptions, BuildOptions, UsageAddOptions, ...
    output.rs         BuildOutput, EnvEntry
    error.rs          SysandError, ErrorCode (from plan A)
  internal/           pub(crate)
    commands/         existing command logic, unchanged signatures
    project/          traits + implementations
    env/              traits + implementations
    resolve/          resolver chain
    solve/            PubGrub solver
    lock.rs           lockfile parsing
    config/           config loading
    model.rs          raw interchange types
    auth.rs           HTTP auth
    discover.rs       filesystem walk
    stdlib.rs         standard library
    workspace.rs      workspace metadata
```

## Steps

### Step 1: Create `types/` module

New files, no changes to existing code yet.

**`types/context.rs`:**

```rust
pub struct ProjectContext {
    pub path: Utf8PathBuf,
    pub config: ConfigMode,
}

pub struct WorkspaceContext {
    pub path: Utf8PathBuf,
}
```

Note: the reference `ProjectContext` holds live `Option<Workspace>` and
`Option<LocalSrcProject>`. The new one holds path + config mode only.
The facade functions open the project internally.

**`types/enums.rs`:**

```rust
pub enum ConfigMode { Auto, File(Utf8PathBuf), None }
pub enum UpdateMode { Manifest, Lock, Sync }
pub enum DepsMode { All, None }
pub enum IndexModeOption { Default, None }
pub enum ChecksumMode { None, Sha256 }
pub enum IndexSymbols { On, Off }
pub enum Language { Auto, Sysml, Kerml }
pub enum Compression { Stored, Deflated, Bzip2, Zstd, Xz, Ppmd }
pub enum SourceKind { Editable, LocalSrc, LocalKpar, Registry, RemoteSrc, RemoteKpar, RemoteGit, RemoteApi }
```

**`types/options.rs`:**

```rust
pub struct InitOptions {
    pub name: Option<String>,
    pub publisher: Option<String>,
    pub version: Option<String>,
    pub license: Option<String>,
    pub allow_non_spdx: bool,
}

pub struct BuildOptions {
    pub target: Option<Utf8PathBuf>,
    pub compression: Compression,
}

pub struct SourceAddOptions {
    pub checksum: ChecksumMode,
    pub index_symbols: IndexSymbols,
    pub language: Language,
}

pub struct UsageAddOptions {
    pub version_req: Option<String>,
    pub source: Option<SourceSpec>,
    pub update: UpdateMode,
    pub include_std: bool,
    pub index: IndexOptions,
}

pub struct IndexOptions {
    pub indexes: Vec<Url>,
    pub default_indexes: Vec<Url>,
    pub index_mode: IndexModeOption,
}

pub struct SourceSpec {
    pub kind: SourceKind,
    pub value: String,
}

// ... remaining options structs per spec/public-api.md
```

**`types/output.rs`:**

```rust
pub struct BuildOutput {
    pub path: Utf8PathBuf,
    pub name: String,
    pub version: String,
}

pub struct EnvEntry {
    pub iri: String,
    pub version: Option<String>,
}
```

### Step 2: Move existing code to `internal/`

Mechanical move — no code changes, just path changes + `pub(crate)`:

- `commands/` → `internal/commands/`
- `project/` → `internal/project/`
- `env/` → `internal/env/`
- `resolve/` → `internal/resolve/`
- `solve/` → `internal/solve/`
- `lock.rs` → `internal/lock.rs`
- `config/` → `internal/config/`
- `model.rs` → `internal/model.rs`
- `auth.rs` → `internal/auth.rs`
- `discover.rs` → `internal/discover.rs`
- `stdlib.rs` → `internal/stdlib.rs`
- `workspace.rs` → `internal/workspace.rs`
- `context.rs` → `internal/context.rs` (old context, still used internally)

Update `mod` declarations. Change `pub` to `pub(crate)` on `internal`.
Fix import paths throughout. This is a large but mechanical change.

### Step 3: Create `facade/` functions

Each facade function: unpack options struct, call internal function,
convert error.

**`facade/init.rs`:**

```rust
use crate::internal::commands::init::do_init_ext;
use crate::types::*;

pub fn init(
    project: &mut impl ProjectMut,
    opts: InitOptions,
) -> Result<(), SysandError> {
    let version = opts.version.unwrap_or_else(|| "0.0.1".into());
    let allow_non_spdx = opts.allow_non_spdx;
    do_init_ext(
        opts.name.unwrap_or_default(),
        opts.publisher,
        version,
        false, // no_semver removed per spec
        opts.license,
        !allow_non_spdx, // invert: allow_non_spdx → !no_spdx
        project,
    ).map_err(SysandError::from)
}
```

**`facade/build.rs`:**

```rust
pub fn build(
    ctx: &ProjectContext,
    opts: BuildOptions,
) -> Result<BuildOutput, SysandError> {
    let project = open_project(ctx)?;
    let compression = opts.compression.into(); // Convert to internal KparCompressionMethod
    let target = opts.target.unwrap_or_else(|| /* default */);
    let kpar = do_build_kpar(&project, &target, compression, true, false)
        .map_err(SysandError::from)?;
    Ok(BuildOutput {
        path: /* absolute path */,
        name: /* from kpar */,
        version: /* from kpar */,
    })
}
```

**`facade/usage.rs`:**

```rust
pub mod usage {
    pub fn add(
        ctx: &ProjectContext,
        iri: &str,
        opts: UsageAddOptions,
    ) -> Result<(), SysandError> {
        let mut project = open_project_mut(ctx)?;
        let usage_raw = InterchangeProjectUsageRaw {
            resource: iri.to_string(),
            version_constraint: opts.version_req,
        };
        do_add(&mut project, &usage_raw).map_err(SysandError::from)?;

        match opts.update {
            UpdateMode::Manifest => Ok(()),
            UpdateMode::Lock => { lock_update_internal(ctx, &opts.index)?; Ok(()) }
            UpdateMode::Sync => { lock_update_internal(ctx, &opts.index)?; env_sync_internal(ctx)?; Ok(()) }
        }
    }

    pub fn remove(
        ctx: &ProjectContext,
        iri: &str,
    ) -> Result<(), SysandError> {
        let mut project = open_project_mut(ctx)?;
        do_remove(&mut project, iri).map_err(SysandError::from)?;
        Ok(())
    }
}
```

**`facade/env.rs` — the hard one (sync):**

The current `do_sync` takes 13 generic type params and closure
factories. The facade must hide this complexity.

```rust
pub mod env {
    pub fn sync(
        ctx: &ProjectContext,
        opts: EnvSyncOptions,
    ) -> Result<(), SysandError> {
        let config = load_config(ctx)?;
        let lock = read_lockfile(ctx)?;
        let env_path = opts.env.unwrap_or_else(default_env_path);
        let mut env = LocalDirectoryEnvironment::open(&env_path)?;

        // Construct concrete closures internally — caller never sees them
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all().build().map_err(/* ... */)?;
        let client = create_reqwest_client()?;
        let auth = build_auth_from_config(&config)?;

        do_sync(
            &lock,
            &mut env,
            Some(|path: &Utf8Path| LocalSrcProject { /* ... */ }),
            Some(|url: String| -> Result<_, _> { /* reqwest */ }),
            Some(|path: &Utf8Path| LocalKParProject::new_guess_root(path)),
            Some(|url: String| -> Result<_, _> { /* reqwest kpar */ }),
            Some(|url: String| -> Result<_, _> { /* gix */ }),
            &HashMap::new(),
        ).map_err(SysandError::from)
    }
}
```

Key insight: the facade for `sync` and `lock::update` must internally
create the HTTP client, tokio runtime, and auth policy. The caller
just passes `ProjectContext` + options. This is where the facade
absorbs complexity that currently leaks to CLI/bindings.

For library users who need custom HTTP clients or auth, we can add
`sync_with(ctx, opts, client, auth)` variants later.

### Step 4: Wire up `lib.rs`

```rust
mod facade;
mod types;
pub(crate) mod internal;

pub use facade::*;
pub use types::*;
```

### Step 5: Update CLI to use facade

The CLI currently does significant assembly work (creating resolvers,
runtimes, clients). Most of this moves into the facade. The CLI
becomes:

```rust
// Before (reference)
pub fn command_sync<P, Policy>(lock, project_root, env, client, provided_iris, runtime, auth_policy) -> Result<()> { ... }

// After
fn run_sync(args: &SyncArgs, ctx: ProjectContext) -> Result<()> {
    sysand_core::env::sync(&ctx, EnvSyncOptions {
        env: args.env.clone(),
        include_std: args.include_std,
        index: args.index_options(),
    })?;
    Ok(())
}
```

### Step 6: Handle removed commands

**`info` and `sources` stay in `internal/commands/`** — they are not
exposed via the facade, but are retained in case they become useful
internally later. Since no internal code currently calls them (all
call sites are CLI/bindings), they will become dead code. Mark them:

```rust
// internal/commands/info.rs

/// Retained for potential internal use. Not exposed via the facade.
/// Previously served the `sysand info` CLI command (removed in spec).
/// All public call sites (CLI, Java, Python) have been removed.
#[allow(dead_code)]
pub(crate) fn do_info<S: AsRef<str>, R: ResolveRead>(/* ... */) { /* ... */ }
```

Same treatment for `do_info_project`, `do_sources_project_no_deps`,
`do_sources_local_src_project_no_deps`, and `find_project_dependencies`.

**`enumerate_projects_lock` is already dead code** (never called
anywhere). Delete it.

**`print-root` / `do_root`** — replaced by `facade/locate.rs`. The
internal `do_root` can be deleted since `locate` reimplements it with
the new context types.

## Open Questions

1. **How does facade handle auth?** The spec says the facade takes auth
   as a parameter (discovery-and-config.md). But for library simplicity,
   the facade could also build auth from config automatically. Needs
   decision: always explicit, or auto from config with override?

2. **Sync's closure factories:** The current design lets callers inject
   custom storage backends via closures. The facade hides this. If a
   library user needs a custom storage backend, they'd call internal
   APIs directly. Is this acceptable, or do we need a public extension
   point?

## Size Estimate

- New code: ~600 lines (facade functions + types)
- Moved code: ~3000 lines (into internal/, mechanical)
- Modified: CLI commands (simplified significantly)
- Deleted: ~400 lines (removed commands: info, sources, print-root)
