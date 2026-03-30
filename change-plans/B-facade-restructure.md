# B. Facade & Crate Restructure

Add `facade/` and `types/` modules alongside existing code. Do NOT
move existing code into `internal/` yet — that's a separate mechanical
step that risks breaking all consumers simultaneously. The additive
approach keeps everything compiling.

Depends on: A (error model).

## Current State

All modules live at the top level of `core/src/` with everything `pub`.
Command functions use `do_` prefix, positional params, boolean flags,
and heavily generic signatures.

**Signature complexity ranges:**

- Simple: `do_add(project, usage)` — 2 params, 1 generic
- Complex: `do_sync(...)` — 13 type params, 7 closure factories
- Complex: `do_lock_projects(...)` — 4 type params with associated
  type constraints

## Target State

Per `spec/crate-structure.md` and `spec/public-api.md`:

```
core/src/
  lib.rs              pub mod facade; pub mod types; (existing modules unchanged)
  error.rs            SysandError, ErrorCode (from plan A)
  facade/
    mod.rs            re-exports submodules
    init.rs           init(project, InitOptions) -> Result<(), SysandError>
    locate.rs         locate(path) -> Result<Utf8PathBuf, SysandError>
    clone.rs          clone_project(source, target) -> Result<(), SysandError>
    build.rs          build(project, path, BuildOptions) -> Result<BuildOutput, SysandError>
    source.rs         add(project, path, opts), remove(project, path)
    usage.rs          add(project, iri, version_req), remove(project, iri)
    lock.rs           update(ctx, resolver, provided_iris)
    env.rs            create, sync, install_project, list, uninstall
    workspace.rs      locate(path), build(workspace, path, opts)
  types/
    mod.rs            re-exports
    context.rs        ProjectContext, WorkspaceContext
    enums.rs          ConfigMode, UpdateMode, DepsMode, Compression, ...
    options.rs        InitOptions, BuildOptions, UsageAddOptions, ...
    output.rs         BuildOutput, EnvEntry
    network.rs        NetworkContext<Policy>
  (existing modules)  commands/, project/, env/, resolve/, etc. — unchanged
```

## Steps

### Step 1: Create `types/` module (additive)

New files alongside existing code. No moves, no breakage.

**`types/mod.rs`** — re-exports submodules:

- `types/context.rs` — `ProjectContext { path, config }`, `WorkspaceContext { path }`
- `types/enums.rs` — `ConfigMode`, `UpdateMode`, `DepsMode`, `Compression`, etc.
- `types/options.rs` — `InitOptions`, `BuildOptions`, `UsageAddOptions`, etc.
  All options structs derive `Default` for ergonomic construction.
- `types/output.rs` — `BuildOutput`, `EnvEntry`
- `types/network.rs` — `NetworkContext<Policy>` (see Step 3)

### Step 2: Create `facade/` functions (additive)

New files alongside existing code. Each facade function calls existing
command functions and converts errors via `SysandError::from`.

**Critical lesson: generic bounds.** Facade functions that take
`impl ProjectMut` need `where P::Error: Into<SysandError>` on the
generic param. This is invisible to binding authors (the concrete
type satisfies it automatically) but must be in the signature:

```rust
pub fn init<P: ProjectMut>(
    project: &mut P,
    opts: InitOptions,
) -> Result<(), SysandError>
where
    P::Error: Into<SysandError>,
{
    crate::commands::init::do_init_ext(/* ... */)
        .map_err(SysandError::from)
}
```

**Feature gates required.** Many facade functions must be gated:

- `#[cfg(feature = "filesystem")]` — `build`, `locate`, `workspace::*`,
  `env::create`, `lock::update`, `clone::clone_project`
- `#[cfg(all(feature = "filesystem", feature = "networking"))]` —
  `env::sync`

Put imports inside the function body (after the `#[cfg]`) to avoid
unresolved import errors when the feature is disabled.

### Step 3: NetworkContext for network commands

**Design decision (resolved):** `NetworkContext<Policy>` bundles
config + auth + HTTP client + tokio runtime.

Generic over `Policy` because `HTTPAuthentication` uses RPITIT
(`impl Future<...>` in trait methods), making it not object-safe.
`Arc<dyn HTTPAuthentication>` does not compile.

```rust
#[cfg(feature = "networking")]
pub struct NetworkContext<Policy: HTTPAuthentication> {
    pub config: Config,
    pub auth: Arc<Policy>,
    pub client: reqwest_middleware::ClientWithMiddleware,
    pub runtime: Arc<tokio::runtime::Runtime>,
}
```

Provides `new(config, auth)` (builds client + runtime) and
`with_client(config, auth, client, runtime)` for testing.

### Step 4: Two tiers of facade functions

**Tier 1: Self-contained** — caller provides storage + options only:

- `init`, `source::add/remove`, `usage::add/remove` (generic over
  `ProjectMut`, with `where P::Error: Into<SysandError>`)
- `build`, `locate`, `workspace::locate/build` (filesystem)
- `env::create`, `env::list`, `env::uninstall` (environment)

**Tier 2: Network/orchestration** — caller provides infrastructure:

- `lock::update` — takes pre-built resolver + provided_iris. Resolver
  assembly stays in CLI/binding layer because it needs config override
  logic (`get_overrides`) that currently lives in the CLI crate.
  Requires `'static` bounds on both `PD` and `R` type params.
- `env::sync` — takes `NetworkContext<Policy>` + lock + env. Hides
  the 13-generic-param `do_sync` by constructing closure factories
  internally. The closures must clone `Arc<Policy>`, `Arc<Runtime>`,
  and `ClientWithMiddleware` for each factory.
- `env::install_project` — installs a single resolved project.
  Full orchestration (resolve + lock + sync) stays in CLI layer.
- `clone::clone_project` — copies resolved project files to target.
  Full orchestration stays in CLI layer.

**Key insight:** Complex orchestrations (`env install` with deps,
`clone` with deps) compose tier-2 building blocks. The CLI does:

1. Build resolver from `NetworkContext` + config
2. Resolve project
3. `lock::update` or `do_lock_extend` for deps
4. `env::sync` to install

This stays in the CLI because it involves CLI-specific concerns
(directory validation, error recovery, user warnings).

### Step 5: Wire up `lib.rs`

```rust
pub mod types;
pub mod facade;
// ... existing modules unchanged
```

No re-exports yet — consumers access via `sysand_core::facade::init::init`
or `sysand_core::types::options::InitOptions`. The spec's flat re-export
(`sysand_core::init(...)`) happens when existing modules move to
`internal/` (a later step).

### Step 6: Handle removed commands

`info` and `sources` stay in existing `commands/` with
`#[allow(dead_code)]` doc comments. `enumerate_projects_lock` is dead
code — delete it. `do_root` is superseded by `facade/locate.rs`.

### Step 7: Document `internal/` boundary (DONE)

Rather than physically moving files (high risk, breaks all consumers),
we created `core/src/internal.rs` as documentation of the intended
boundary. It lists all modules that belong in `internal/` and explains
that the physical move happens after consumers fully migrate to the
facade.

**Lesson:** After migrating simple commands and moving resolver assembly
to the facade, the CLI still has ~20 direct imports from core internals
used by complex orchestration commands (env install, clone, add config
source matching). The physical move requires:

1. `facade::env::install_with_deps` — env install orchestration (~170 lines)
2. `facade::clone::clone_with_deps` — clone orchestration (~180 lines)
3. Move `Lock`, `Source` types to `types/` (public vocabulary)
4. Move `default_kpar_file_name` to facade
5. Expose old `ProjectContext` via facade for discovery
6. Move `style` config to types or facade

~500 lines of additional migration. The boundary is communicated via
`internal.rs` doc file for now.

## Resolved Questions

1. **Auth handling:** `NetworkContext<Policy>` is generic over auth.
   Caller provides auth policy. Facade never reads env vars.

2. **Sync closure factories:** `env::sync` facade hides them by
   constructing closures internally from `NetworkContext`.

3. **Object safety:** `HTTPAuthentication` uses RPITIT so
   `dyn HTTPAuthentication` doesn't work. `NetworkContext` must be
   generic over `Policy`.

4. **Resolver assembly for lock::update:** The `get_overrides` logic
   (config → override resolver) currently lives in the CLI crate.
   The facade's `lock::update` takes a pre-built resolver rather than
   trying to absorb this logic. Moving `get_overrides` to core is a
   future improvement.

## Size Estimate

- New code: ~1100 lines (facade functions ~550, types ~550)
- No code moved or deleted (additive only)
- Existing consumers unchanged — they continue using old paths
- Net addition, not reduction (reduction comes when consumers migrate)
