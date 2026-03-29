# D. CLI Restructure

Restructure the CLI command tree, rename flags to match spec, remove
dropped commands. Can be done in parallel with C (bindings) once B
(facade) is complete.

Depends on: B (facade restructure).

## Current State

### Command Tree (reference)

```
sysand init [PATH]
sysand add <LOCATOR> [VERSION]
sysand remove <LOCATOR>
sysand clone <LOCATOR>
sysand include <PATHS...>
sysand exclude <PATHS...>
sysand build [PATH]
sysand lock
sysand sync
sysand env [install|uninstall|list|sources]
sysand info [name|publisher|version|license|maintainer|topic|usage|index|...]
sysand sources
sysand print-root
```

### Flags with `--no-*` pattern

- `--no-semver` (init) — removed entirely per spec
- `--no-spdx` (init) → `--allow-non-spdx`
- `--no-lock` (add) → part of `--update manifest|lock|sync`
- `--no-sync` (add) → part of `--update manifest|lock|sync`
- `--no-deps` (clone, env install) → `--deps all|none`
- `--no-index` (resolution) → `--index-mode default|none`
- `--no-index-symbols` (include) → `--index-symbols on|off`
- `--no-config` (global) → `--config auto|none|PATH`

### Dispatch Architecture

`lib_main()` → `Args::try_parse_from()` → `run_cli()` which is a
~440-line match block that:

1. Sets up logging, discovery, config, auth, HTTP client, tokio runtime
2. Matches on `Command` enum variant
3. Calls `command_*()` functions with 8-12+ params each

## Target State

### Command Tree (spec)

```
sysand init <PATH> [opts]
sysand locate [--project PATH]
sysand clone <LOCATOR> [opts]
sysand build [opts]
sysand source add <PATH>... [opts]
sysand source remove <PATH>... [opts]
sysand usage add <IRI> [VERSION_REQ] [opts]
sysand usage remove <IRI> [opts]
sysand lock update [opts]
sysand env create [opts]
sysand env sync [opts]
sysand env install <IRI> [VERSION_REQ] [opts]
sysand env uninstall <IRI> [VERSION_REQ] [opts]
sysand env list [opts]
sysand workspace locate [--workspace PATH]
sysand workspace build [opts]
```

### Flag Changes

| Old                             | New                                           | Notes                     |
| ------------------------------- | --------------------------------------------- | ------------------------- |
| `--no-semver`                   | (removed)                                     | Semver always required    |
| `--no-spdx`                     | `--allow-non-spdx`                            | Positive flag             |
| `--no-lock` + `--no-sync`       | `--update manifest\|lock\|sync`               | Single enum, default sync |
| `--no-deps`                     | `--deps all\|none`                            | Positive enum             |
| `--no-index`                    | `--index-mode default\|none`                  | Positive enum             |
| `--no-index-symbols`            | `--index-symbols on\|off`                     | Positive enum             |
| `--no-config` + `--config-file` | `--config auto\|none\|PATH`                   | Unified into ConfigMode   |
| `--verbose` / `--quiet`         | `--log-level error\|warn\|info\|debug\|trace` | Positive enum             |
| `--compute-checksum`            | `--checksum none\|sha256`                     | Positive enum             |

### Removed Commands

- `info` (all subcommands) — users edit `.project.json` directly
- `sources` — users read files directly
- `print-root` — replaced by `locate`

### Simplified Dispatch

With the facade handling resolver creation, HTTP client, tokio runtime,
etc., the CLI dispatch becomes thin:

```rust
match command {
    Command::Init { path, opts } => {
        let mut project = LocalSrcProject::open(&path)?;
        sysand_core::init(&mut project, opts.into())?;
    }
    Command::Build { opts } => {
        let ctx = discover_project_context(&args)?;
        let output = sysand_core::build(&ctx, opts.into())?;
        render_build_output(&output, args.format);
    }
    Command::Source(SourceCommand::Add { paths, opts }) => {
        let ctx = discover_project_context(&args)?;
        sysand_core::source::add(&ctx, &paths, opts.into())?;
    }
    // ...
}
```

## Steps

### Step 1: Update clap parser (`cli.rs`)

Replace the `Command` enum:

```rust
#[derive(Parser)]
struct Args {
    #[command(subcommand)]
    command: Command,

    /// Config mode
    #[arg(long, default_value = "auto")]
    config: ConfigModeArg,

    /// Log level
    #[arg(long, default_value = "warn")]
    log_level: LogLevel,

    /// Output format
    #[arg(long, default_value = "text")]
    format: Format,
}

#[derive(Subcommand)]
enum Command {
    /// Initialize a new project
    Init {
        path: Utf8PathBuf,
        #[command(flatten)]
        opts: InitArgs,
    },
    /// Find project root
    Locate {
        #[arg(long)]
        project: Option<Utf8PathBuf>,
    },
    /// Clone a project
    Clone {
        locator: String,
        #[command(flatten)]
        opts: CloneArgs,
    },
    /// Build KPAR archive
    Build {
        #[command(flatten)]
        opts: BuildArgs,
    },
    /// Source file management
    #[command(subcommand)]
    Source(SourceCommand),
    /// Usage management
    #[command(subcommand)]
    Usage(UsageCommand),
    /// Lockfile operations
    #[command(subcommand)]
    Lock(LockCommand),
    /// Environment operations
    #[command(subcommand)]
    Env(EnvCommand),
    /// Workspace operations
    #[command(subcommand)]
    Workspace(WorkspaceCommand),
}

#[derive(Subcommand)]
enum SourceCommand {
    Add {
        paths: Vec<Utf8PathBuf>,
        #[command(flatten)]
        opts: SourceAddArgs,
    },
    Remove {
        paths: Vec<Utf8PathBuf>,
    },
}

#[derive(Subcommand)]
enum UsageCommand {
    Add {
        iri: String,
        version_req: Option<String>,
        #[command(flatten)]
        opts: UsageAddArgs,
    },
    Remove {
        iri: String,
    },
}
// ... similarly for Lock, Env, Workspace
```

### Step 2: Update flag definitions

```rust
#[derive(Args)]
struct InitArgs {
    #[arg(long)]
    name: Option<String>,
    #[arg(long)]
    publisher: Option<String>,
    #[arg(long)]
    version: Option<String>,
    #[arg(long)]
    license: Option<String>,
    #[arg(long)]
    allow_non_spdx: bool,
}

#[derive(Args)]
struct UsageAddArgs {
    #[arg(long, default_value = "sync")]
    update: UpdateModeArg,  // manifest|lock|sync
    #[arg(long)]
    include_std: bool,
    #[command(flatten)]
    index: IndexArgs,
    // source-kind/source if we keep them (see TODO.md)
}

#[derive(Args)]
struct IndexArgs {
    #[arg(long = "index", num_args = 1)]
    indexes: Vec<String>,
    #[arg(long = "default-index", num_args = 1)]
    default_indexes: Vec<String>,
    #[arg(long, default_value = "default")]
    index_mode: IndexModeArg,  // default|none
}
```

### Step 3: Simplify dispatch (`lib.rs`)

Replace the ~440-line match block. Each arm becomes 3-10 lines:

1. Discover context (if needed)
2. Convert CLI args to facade options struct
3. For network commands: build `NetworkContext` + resolver
4. Call facade function
5. Render output (if any)

**Important nuance from implementation:** The CLI still owns:

- `NetworkContext` construction (config + auth from env vars + HTTP
  client + tokio runtime)
- Resolver assembly for `lock::update` (the `get_overrides` +
  `create_resolver` logic stays in CLI for now)
- Complex orchestrations like "install with deps" (resolve → lock →
  sync)

So dispatch isn't *quite* as thin as originally planned for network
commands. Tier-1 commands (init, build, source, usage) become truly
3-5 lines. Tier-2 commands (lock, sync, clone, env install) are
10-20 lines with infrastructure setup.

The CLI handles:

- Implicit CWD discovery → `ProjectContext`
- `SYSAND_CRED_*` env vars → auth policy → `NetworkContext`
- Config loading → `Config` → `NetworkContext`
- `--log-level` → logging setup
- `--format` → output rendering
- Exit codes

### Step 4: Implement `From<CliArgs> for FacadeOptions`

Conversion from clap args structs to facade options:

```rust
impl From<InitArgs> for InitOptions {
    fn from(args: InitArgs) -> Self {
        InitOptions {
            name: args.name,
            publisher: args.publisher,
            version: args.version,
            license: args.license,
            allow_non_spdx: args.allow_non_spdx,
        }
    }
}

impl From<UsageAddArgs> for UsageAddOptions {
    fn from(args: UsageAddArgs) -> Self {
        UsageAddOptions {
            version_req: None, // set separately from positional
            source: None,
            update: args.update.into(),
            include_std: args.include_std,
            index: args.index.into(),
        }
    }
}
```

### Step 5: Remove CLI commands (DONE)

Removed from the **CLI crate**:

- `commands/info.rs` (679 lines)
- `commands/sources.rs` (122 lines)
- `commands/print_root.rs` (21 lines)
- `Command::Info` variant + ~900 lines of `InfoCommand` enum, impl,
  verb types, and dispatch logic in `cli.rs`
- `Command::Sources` variant + dispatch
- `Command::PrintRoot` variant + dispatch
- `Command::New` (hidden alias)
- `EnvCommand::Sources` variant + dispatch
- `SourcesOptions` struct
- Imports for all removed command functions

**Lessons from deletion:**

1. **`InfoCommand` was ~900 lines alone.** The enum had 15 variants
   (name, publisher, description, version, license, maintainer,
   website, topic, usage, index, created, metamodel, includes-derived,
   includes-implied, checksum), each with set/clear/add/remove sub-
   operations, plus a 400-line `as_verb()` impl and a `numbered()`
   method. This was by far the largest chunk of deleted code.

2. **`sed` line deletion is fragile for Rust.** Deleting ranges by line
   number left stray `#[derive(...)]` attributes and doc comments that
   caused compilation errors (conflicting trait impls, orphaned doc
   comments). Manual cleanup was needed after sed. For future large
   deletions, prefer reading the exact boundaries and using Edit tool
   with precise old_string matching.

3. **No test changes needed.** The CLI crate had no integration tests
   for `info` or `sources` commands — those were only tested through
   the Python binding (already updated in plan C).

4. **Dispatch simplification is immediate.** Removing ~160 lines of
   Info dispatch and ~15 lines of Sources dispatch from `lib.rs` was
   the easy part. The `cli.rs` enum + types were the bulk.

### Step 6: Restructure command tree (remaining)

Not yet done. The remaining changes:

| Old                     | New                           | Status  |
| ----------------------- | ----------------------------- | ------- |
| `sysand add <IRI>`      | `sysand usage add <IRI>`      | pending |
| `sysand remove <IRI>`   | `sysand usage remove <IRI>`   | pending |
| `sysand include <PATH>` | `sysand source add <PATH>`    | pending |
| `sysand exclude <PATH>` | `sysand source remove <PATH>` | pending |
| `sysand lock`           | `sysand lock update`          | pending |
| `sysand sync`           | `sysand env sync`             | pending |
| `sysand print-root`     | `sysand locate`               | done    |
| `sysand info`           | removed                       | done    |
| `sysand sources`        | removed                       | done    |

### Step 7: Rename flags (remaining)

| Old               | New                  | Status  |
| ----------------- | -------------------- | ------- |
| `--no-semver`     | (removed)            | pending |
| `--no-spdx`       | `--allow-non-spdx`   | pending |
| `--no-lock/sync`  | `--update manifest…` | pending |
| `--no-deps`       | `--deps all\|none`   | pending |
| `--no-index`      | `--index-mode …`     | pending |
| `--no-index-syms` | `--index-symbols …`  | pending |

## Actual Size (after step 5)

- `cli.rs`: 1634 → 652 lines (~60% deleted)
- `lib.rs`: 779 → 578 lines (~25% deleted)
- Deleted files: info.rs (679), sources.rs (122), print_root.rs (21)
- Total: ~2000 lines deleted
- Remaining steps 6-7 are structural renames, not deletions
