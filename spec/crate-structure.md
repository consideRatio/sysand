# Crate Structure

How the Rust core library is organized internally. The public API
(`public-api.md`) defines _what_ is exposed; this file defines _where_
it lives in the crate and how the internals are isolated.

## Boundary Rule

The core crate (`sysand-core`) has exactly two public module trees:

- **`facade/`** — functions callers invoke
- **`types/`** — types callers pass and receive

Everything else is `pub(crate)`. Bindings, CLI, and library consumers
import only from `facade` and `types`.

## Module Layout

```
core/src/
  lib.rs                  re-exports facade/ and types/
  facade/
    mod.rs                re-exports all submodules
    init.rs               init
    locate.rs             locate
    clone.rs              clone_project (building block — copies files)
    build.rs              build
    source.rs             source::add, source::remove
    usage.rs              usage::add, usage::remove
    lock.rs               lock::update
    env.rs                env::create, sync, install_project, uninstall, list
    workspace.rs          workspace::locate, workspace::build
  error.rs                SysandError, ErrorCode (top-level, not in types/)
  types/
    mod.rs                re-exports everything below
    context.rs            ProjectContext, WorkspaceContext
    enums.rs              ConfigMode, UpdateMode, DepsMode, Compression, ...
    options.rs            InitOptions, BuildOptions, UsageAddOptions, ...
    output.rs             BuildOutput, EnvEntry
    network.rs            NetworkContext<Policy> (infrastructure bundle)
  internal/               pub(crate) — not visible outside the crate
    commands/             command implementations (generic, trait-bounded)
    project/              ProjectRead, ProjectMut traits + implementations
    env/                  ReadEnvironment, WriteEnvironment + implementations
    resolve/              ResolveRead, resolver chain, priority logic
    solve/                PubGrub solver
    lock.rs               lockfile parsing, serialization
    config/               config loading, merging
    model.rs              raw interchange types (InfoRaw, MetadataRaw)
    auth.rs               HTTP authentication
    discover.rs           filesystem walk for .project.json / .workspace.json
    stdlib.rs             standard library definitions
    workspace.rs          workspace metadata
```

## Facade Functions

Each facade function is a thin wrapper that:

1. Calls the corresponding internal command function
2. Maps the internal error type to `SysandError`

Facade functions are re-exported from `lib.rs`, so callers write
`sysand_core::init(...)` not `sysand_core::facade::init(...)`. The
`facade/` directory is organizational, not part of the public path.

```rust
// core/src/facade/mod.rs
pub fn init(
    project: &mut impl ProjectMut,
    opts: InitOptions,
) -> Result<(), SysandError> {
    crate::internal::commands::init::do_init(project, opts)
        .map_err(SysandError::from)
}

// core/src/lib.rs
mod facade;
mod types;
mod internal;

pub use facade::*;
pub use types::*;
```

Callers see a flat namespace for root commands and nested modules for
namespaced commands:

```rust
use sysand_core::{init, build, locate, clone, SysandError, InitOptions};

init(&mut project, opts)?;
sysand_core::source::add(&ctx, paths, opts)?;
sysand_core::env::sync(&ctx, opts)?;
```

Facade functions take `impl ProjectMut` / `impl ProjectRead` — the
caller provides the concrete storage backend. The facade does not know
which backend is in use.

For commands that need environments or resolvers (e.g., `env::sync`,
`lock::update`), the facade takes concrete assembled objects rather
than the current approach of closure factories with many generic
parameters. The CLI and bindings construct these objects and pass
them in.

## Types

Types in `types/` are the public vocabulary shared between the facade
and its callers. They fall into four categories:

| Category | Examples                                         | Notes                                    |
| -------- | ------------------------------------------------ | ---------------------------------------- |
| Context  | `ProjectContext`, `WorkspaceContext`             | Passed as first arg to most functions    |
| Options  | `InitOptions`, `BuildOptions`, `UsageAddOptions` | Per-command configuration                |
| Output   | `BuildOutput`, `EnvEntry`                        | Return values from facade functions      |
| Enums    | `UpdateMode`, `Compression`, `ConfigMode`        | Used in options and context              |
| Error    | `SysandError`, `ErrorCode`                       | Single error type for all facade returns |

Internal types (`InterchangeProjectInfoRaw`, `Lock`, resolver traits,
etc.) stay in `internal/` and do not appear in facade signatures.

## Migration from Reference

The reference implementation has all modules at the top level of
`core/src/`, with everything `pub`. The migration:

| Reference location                  | Target location             | Visibility |
| ----------------------------------- | --------------------------- | ---------- |
| `commands/init.rs` (do_init)        | `facade/init.rs`            | pub        |
| `commands/add.rs` (do_add)          | `facade/usage.rs` (add)     | pub        |
| `commands/include.rs` (do_include)  | `facade/source.rs` (add)    | pub        |
| `commands/exclude.rs` (do_exclude)  | `facade/source.rs` (remove) | pub        |
| `commands/build.rs` (do_build_kpar) | `facade/build.rs`           | pub        |
| `commands/lock.rs` (do_lock)        | `facade/lock.rs` (update)   | pub        |
| `commands/sync.rs` (do_sync)        | `facade/env.rs` (sync)      | pub        |
| `commands/env.rs`                   | `facade/env.rs`             | pub        |
| `commands/info.rs`                  | removed                     | —          |
| `commands/sources.rs`               | removed                     | —          |
| `commands/root.rs`                  | `facade/locate.rs`          | pub        |
| `context.rs` (ProjectContext)       | `types/context.rs`          | pub        |
| `model.rs`                          | `internal/model.rs`         | pub(crate) |
| `project/`                          | `internal/project/`         | pub(crate) |
| `env/`                              | `internal/env/`             | pub(crate) |
| `resolve/`                          | `internal/resolve/`         | pub(crate) |
| `solve/`                            | `internal/solve/`           | pub(crate) |
| `lock.rs`                           | `internal/lock.rs`          | pub(crate) |
| `config/`                           | `internal/config/`          | pub(crate) |
| `auth.rs`                           | `internal/auth.rs`          | pub(crate) |
| `discover.rs`                       | `internal/discover.rs`      | pub(crate) |
| `stdlib.rs`                         | `internal/stdlib.rs`        | pub(crate) |
| `workspace.rs`                      | `internal/workspace.rs`     | pub(crate) |

New types not in reference (to be created):

- `types/options.rs` — `InitOptions`, `BuildOptions`, etc. (replace positional params + booleans)
- `types/output.rs` — `BuildOutput`, `EnvEntry` (replace raw model types in returns)
- `types/error.rs` — `SysandError`, `ErrorCode` (replace per-module error enums)
- `types/enums.rs` — `UpdateMode`, `ConfigMode`, etc. (replace boolean flags)

## Rationale

**Why two internal module trees (`facade/` + `types/`), not one.**
Both are re-exported flat from `lib.rs`, so callers don't see the
split. Internally, separating them keeps function definitions and type
definitions from growing into one large module. Types are shared across
facade functions and imported independently by bindings for argument
construction and return value handling.

**Why `pub(crate)` instead of a separate crate.** A single crate with
visibility control is simpler than a multi-crate dependency graph.
`pub(crate)` is enforced by the compiler — internal types cannot
accidentally leak into public signatures. If the crate grows large
enough to warrant splitting, the `facade/` + `types/` boundary makes
the split point obvious.

**Why facade wraps internal commands rather than replacing them.** The
internal command functions remain generic and testable with in-memory
backends. The facade adds the public API contract (options structs,
`SysandError`) on top. This keeps unit testing of command logic
independent of the public API shape.
