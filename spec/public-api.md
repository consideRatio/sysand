# Public API

The complete public API across all five binding surfaces. Types are
organized by category; functions are listed once with their signatures
derived mechanically from the projection rules (`projection-rules.md`).

## Context Objects

### ProjectContext

| Surface | Shape                                                                               |
| ------- | ----------------------------------------------------------------------------------- |
| CLI     | `--project <PATH>` + `--config <auto\|none\|PATH>` (implicit from CWD when omitted) |
| Rust    | `ProjectContext { path: Utf8PathBuf, config: ConfigMode }`                          |
| Java    | `new ProjectContext(path, configMode)`                                              |
| JS/WASM | `new ProjectContext(path, configMode?)`                                             |
| Python  | `ProjectContext(path, config=ConfigMode.AUTO)`                                      |

### WorkspaceContext

| Surface | Shape                                                 |
| ------- | ----------------------------------------------------- |
| CLI     | `--workspace <PATH>` (implicit from CWD when omitted) |
| Rust    | `WorkspaceContext { path: Utf8PathBuf }`              |
| Java    | `new WorkspaceContext(path)`                          |
| JS/WASM | `new WorkspaceContext(path)`                          |
| Python  | `WorkspaceContext(path)`                              |

## Enums

### ConfigMode

Controls how `sysand.toml` is loaded.

| Surface | Shape                                                         |
| ------- | ------------------------------------------------------------- |
| CLI     | `--config <auto\|none\|PATH>`                                 |
| Rust    | `enum ConfigMode { Auto, File(Utf8PathBuf), None }`           |
| Java    | `ConfigMode.AUTO`, `ConfigMode.NONE`, `ConfigMode.file(path)` |
| JS/WASM | `"auto" \| "none" \| { file: string }`                        |
| Python  | `ConfigMode.AUTO`, `ConfigMode.NONE`, `ConfigMode.file(path)` |

### UpdateMode

Controls side-effects of `usage add`.

| Surface | Shape                                                       |
| ------- | ----------------------------------------------------------- |
| CLI     | `--update <manifest\|lock\|sync>`                           |
| Rust    | `enum UpdateMode { Manifest, Lock, Sync }`                  |
| Java    | `UpdateMode.MANIFEST`, `UpdateMode.LOCK`, `UpdateMode.SYNC` |
| JS/WASM | `"manifest" \| "lock" \| "sync"`                            |
| Python  | `UpdateMode.MANIFEST`, `UpdateMode.LOCK`, `UpdateMode.SYNC` |

Default: `Sync`

### DepsMode

Controls whether transitive usages are fetched.

| Surface | Shape                           |
| ------- | ------------------------------- |
| CLI     | `--deps <all\|none>`            |
| Rust    | `enum DepsMode { All, None }`   |
| Java    | `DepsMode.ALL`, `DepsMode.NONE` |
| JS/WASM | `"all" \| "none"`               |
| Python  | `DepsMode.ALL`, `DepsMode.NONE` |

Default: `All`

### IndexModeOption

Controls whether default indexes are used.

| Surface | Shape                                             |
| ------- | ------------------------------------------------- |
| CLI     | `--index-mode <default\|none>`                    |
| Rust    | `enum IndexModeOption { Default, None }`          |
| Java    | `IndexModeOption.DEFAULT`, `IndexModeOption.NONE` |
| JS/WASM | `"default" \| "none"`                             |
| Python  | `IndexModeOption.DEFAULT`, `IndexModeOption.NONE` |

Default: `Default`

### ChecksumMode

Controls checksum generation for source files.

| Surface | Shape                                      |
| ------- | ------------------------------------------ |
| CLI     | `--checksum <none\|sha256>`                |
| Rust    | `enum ChecksumMode { None, Sha256 }`       |
| Java    | `ChecksumMode.NONE`, `ChecksumMode.SHA256` |
| JS/WASM | `"none" \| "sha256"`                       |
| Python  | `ChecksumMode.NONE`, `ChecksumMode.SHA256` |

### IndexSymbols

Controls whether symbols are indexed when adding sources.

| Surface | Shape                                 |
| ------- | ------------------------------------- |
| CLI     | `--index-symbols <on\|off>`           |
| Rust    | `enum IndexSymbols { On, Off }`       |
| Java    | `IndexSymbols.ON`, `IndexSymbols.OFF` |
| JS/WASM | `"on" \| "off"`                       |
| Python  | `IndexSymbols.ON`, `IndexSymbols.OFF` |

Default: `On`

### Language

Controls language detection for source files.

| Surface | Shape                                               |
| ------- | --------------------------------------------------- |
| CLI     | `--language <auto\|sysml\|kerml>`                   |
| Rust    | `enum Language { Auto, Sysml, Kerml }`              |
| Java    | `Language.AUTO`, `Language.SYSML`, `Language.KERML` |
| JS/WASM | `"auto" \| "sysml" \| "kerml"`                      |
| Python  | `Language.AUTO`, `Language.SYSML`, `Language.KERML` |

Default: `Auto`

### Compression

Controls KPAR archive compression.

| Surface | Shape                                                                |
| ------- | -------------------------------------------------------------------- |
| CLI     | `--compression <stored\|deflated\|bzip2\|zstd\|xz\|ppmd>`            |
| Rust    | `enum Compression { Stored, Deflated, Bzip2, Zstd, Xz, Ppmd }`       |
| Java    | `Compression.STORED`, `.DEFLATED`, `.BZIP2`, `.ZSTD`, `.XZ`, `.PPMD` |
| JS/WASM | `"stored" \| "deflated" \| "bzip2" \| "zstd" \| "xz" \| "ppmd"`      |
| Python  | `Compression.STORED`, `.DEFLATED`, `.BZIP2`, `.ZSTD`, `.XZ`, `.PPMD` |

Default: `Deflated`. `Bzip2`, `Zstd`, `Xz`, `Ppmd` behind feature flags.

### SourceKind

Identifies the type of source location.

| Variant      | CLI value     | Description                   |
| ------------ | ------------- | ----------------------------- |
| `Editable`   | `editable`    | Local project, changes live   |
| `LocalSrc`   | `src-path`    | Local source directory        |
| `LocalKpar`  | `kpar-path`   | Local `.kpar` archive         |
| `Registry`   | `registry`    | Package index                 |
| `RemoteSrc`  | `remote-src`  | Remote source directory (URL) |
| `RemoteKpar` | `remote-kpar` | Remote `.kpar` archive (URL)  |
| `RemoteGit`  | `remote-git`  | Git repository (URL)          |
| `RemoteApi`  | `remote-api`  | API endpoint (URL)            |

| Surface | Shape                                                                                                                        |
| ------- | ---------------------------------------------------------------------------------------------------------------------------- |
| Rust    | `enum SourceKind { Editable, LocalSrc, LocalKpar, Registry, RemoteSrc, RemoteKpar, RemoteGit, RemoteApi }`                   |
| Java    | `SourceKind.EDITABLE`, `.LOCAL_SRC`, `.LOCAL_KPAR`, `.REGISTRY`, `.REMOTE_SRC`, `.REMOTE_KPAR`, `.REMOTE_GIT`, `.REMOTE_API` |
| JS/WASM | `"editable" \| "src-path" \| "kpar-path" \| "registry" \| "remote-src" \| "remote-kpar" \| "remote-git" \| "remote-api"`     |
| Python  | `SourceKind.EDITABLE`, `.LOCAL_SRC`, `.LOCAL_KPAR`, `.REGISTRY`, `.REMOTE_SRC`, `.REMOTE_KPAR`, `.REMOTE_GIT`, `.REMOTE_API` |

### ErrorCode

See `error-model.md` for the full enum and binding projections.

## Options Structs

### IndexOptions

Shared across commands that query indexes.

| Surface | Shape                                                                                          |
| ------- | ---------------------------------------------------------------------------------------------- |
| CLI     | `--index <URL>` (repeated), `--default-index <URL>` (repeated), `--index-mode <default\|none>` |
| Rust    | `IndexOptions { indexes: Vec<Url>, default_indexes: Vec<Url>, index_mode: IndexModeOption }`   |
| Java    | `IndexOptions.builder().index(url).defaultIndex(url).indexMode(mode).build()`                  |
| JS/WASM | `{ indexes?: string[], defaultIndexes?: string[], indexMode?: "default" \| "none" }`           |
| Python  | `IndexOptions(indexes=[], default_indexes=[], index_mode=IndexModeOption.DEFAULT)`             |

### SourceSpec

Paired `--source-kind` + `--source` flags.

| Surface | Shape                                            |
| ------- | ------------------------------------------------ |
| CLI     | `--source-kind <KIND> --source <VALUE>`          |
| Rust    | `SourceSpec { kind: SourceKind, value: String }` |
| Java    | `new SourceSpec(kind, value)`                    |
| JS/WASM | `{ sourceKind: string, source: string }`         |
| Python  | `SourceSpec(kind, value)`                        |

## Return Types

### SysandError

See `error-model.md`. All operations return errors through this type.

| Surface | Shape                                           |
| ------- | ----------------------------------------------- |
| CLI     | Exit code + stderr message                      |
| Rust    | `Result<T, SysandError>`                        |
| Java    | throws `SysandException` with `ErrorCode` field |
| JS/WASM | throws `SysandError` with `code` property       |
| Python  | raises `SysandError` subclass per code          |

### BuildOutput

Returned by `build`. Workspace build returns a list of these.

| Field     | Type   | Notes                                     |
| --------- | ------ | ----------------------------------------- |
| `path`    | Path   | Absolute path to the created `.kpar` file |
| `name`    | String | Project name                              |
| `version` | String | Project version (semver)                  |

| Surface | Shape                                                              |
| ------- | ------------------------------------------------------------------ |
| CLI     | `--format text\|json` rendering of the output                      |
| Rust    | `BuildOutput { path: Utf8PathBuf, name: String, version: String }` |
| Java    | `BuildOutput` with `getPath()`, `getName()`, `getVersion()`        |
| JS/WASM | `{ path: string, name: string, version: string }`                  |
| Python  | `BuildOutput(path, name, version)`                                 |

### EnvEntry

Returned by `env list`. One entry per installed project.

| Field     | Type             | Notes                           |
| --------- | ---------------- | ------------------------------- |
| `iri`     | String           | Project identifier (IRI)        |
| `version` | `Option<String>` | Installed version, if available |

| Surface | Shape                                               |
| ------- | --------------------------------------------------- |
| CLI     | `--format text\|json` rendering of the list         |
| Rust    | `EnvEntry { iri: String, version: Option<String> }` |
| Java    | `EnvEntry` with `getIri()`, `getVersion()`          |
| JS/WASM | `{ iri: string, version?: string }`                 |
| Python  | `EnvEntry(iri, version)`                            |

## Functions

Every function takes context as its first argument, required operands
next, then an options struct. Projection rules (`projection-rules.md`)
map CLI command paths to namespaces mechanically.

### Root Commands

Project operations live at the root level — no `project` namespace.

| Command | CLI                                | Rust                                              | Java                              | JS/WASM                          | Python                            |
| ------- | ---------------------------------- | ------------------------------------------------- | --------------------------------- | -------------------------------- | --------------------------------- |
| init    | `sysand init <PATH> [opts]`        | `init(path, InitOptions) → Result<()>`            | `client.init(path, opts)`         | `sysand.init(path, opts?)`       | `sysand.init(path, **opts)`       |
| locate  | `sysand locate [--project PATH]`   | `locate(path) → Result<Utf8PathBuf>`              | `client.locate(path)`             | `sysand.locate(path)`            | `sysand.locate(path)`             |
| clone   | `sysand clone <LOCATOR> [opts]`    | `clone(locator, CloneOptions) → Result<()>`       | `client.clone(locator, opts)`     | `sysand.clone(locator, opts?)`   | `sysand.clone(locator, **opts)`   |
| build   | `sysand build [opts]`              | `build(&ctx, BuildOptions) → Result<BuildOutput>` | `client.build(ctx, opts)`         | `sysand.build(ctx, opts?)`       | `sysand.build(ctx, **opts)`       |

### source

| Command | CLI                                       | Rust                                                      | Java                                    | JS/WASM                                  | Python                                    |
| ------- | ----------------------------------------- | --------------------------------------------------------- | --------------------------------------- | ---------------------------------------- | ----------------------------------------- |
| add     | `sysand source add <PATH>... [opts]`      | `source::add(&ctx, paths, SourceAddOptions) → Result<()>` | `client.source().add(ctx, paths, opts)` | `sysand.source.add(ctx, paths, opts?)`   | `sysand.source.add(ctx, paths, **opts)`   |
| remove  | `sysand source remove <PATH>... [opts]`   | `source::remove(&ctx, paths) → Result<()>`                | `client.source().remove(ctx, paths)`    | `sysand.source.remove(ctx, paths)`       | `sysand.source.remove(ctx, paths)`        |

### usage

| Command | CLI                                             | Rust                                                      | Java                                     | JS/WASM                                | Python                                  |
| ------- | ----------------------------------------------- | --------------------------------------------------------- | ---------------------------------------- | -------------------------------------- | --------------------------------------- |
| add     | `sysand usage add <IRI> [VERSION_REQ] [opts]`   | `usage::add(&ctx, iri, UsageAddOptions) → Result<()>`     | `client.usage().add(ctx, iri, opts)`     | `sysand.usage.add(ctx, iri, opts?)`    | `sysand.usage.add(ctx, iri, **opts)`    |
| remove  | `sysand usage remove <IRI> [opts]`              | `usage::remove(&ctx, iri) → Result<()>`                   | `client.usage().remove(ctx, iri)`        | `sysand.usage.remove(ctx, iri)`        | `sysand.usage.remove(ctx, iri)`         |

### lock

| Command | CLI                         | Rust                                                 | Java                              | JS/WASM                          | Python                            |
| ------- | --------------------------- | ---------------------------------------------------- | --------------------------------- | -------------------------------- | --------------------------------- |
| update  | `sysand lock update [opts]` | `lock::update(&ctx, LockUpdateOptions) → Result<()>` | `client.lock().update(ctx, opts)` | `sysand.lock.update(ctx, opts?)` | `sysand.lock.update(ctx, **opts)` |

### env

| Command   | CLI                                               | Rust                                                          | Java                                     | JS/WASM                                 | Python                                   |
| --------- | ------------------------------------------------- | ------------------------------------------------------------- | ---------------------------------------- | --------------------------------------- | ---------------------------------------- |
| create    | `sysand env create [opts]`                        | `env::create(EnvCreateOptions) → Result<()>`                  | `client.env().create(opts)`              | `sysand.env.create(opts?)`              | `sysand.env.create(**opts)`              |
| sync      | `sysand env sync [opts]`                          | `env::sync(&ctx, EnvSyncOptions) → Result<()>`                | `client.env().sync(ctx, opts)`           | `sysand.env.sync(ctx, opts?)`           | `sysand.env.sync(ctx, **opts)`           |
| install   | `sysand env install <IRI> [VERSION_REQ] [opts]`   | `env::install(&ctx, iri, EnvInstallOptions) → Result<()>`     | `client.env().install(ctx, iri, opts)`   | `sysand.env.install(ctx, iri, opts?)`   | `sysand.env.install(ctx, iri, **opts)`   |
| uninstall | `sysand env uninstall <IRI> [VERSION_REQ] [opts]` | `env::uninstall(&ctx, iri, EnvUninstallOptions) → Result<()>` | `client.env().uninstall(ctx, iri, opts)` | `sysand.env.uninstall(ctx, iri, opts?)` | `sysand.env.uninstall(ctx, iri, **opts)` |
| list      | `sysand env list [opts]`                          | `env::list(EnvListOptions) → Result<Vec<EnvEntry>>`           | `client.env().list(opts)`                | `sysand.env.list(opts?)`                | `sysand.env.list(**opts)`                |

### workspace

| Command | CLI                                          | Rust                                                              | Java                                  | JS/WASM                              | Python                                |
| ------- | -------------------------------------------- | ----------------------------------------------------------------- | ------------------------------------- | ------------------------------------ | ------------------------------------- |
| locate  | `sysand workspace locate [--workspace PATH]` | `workspace::locate(path) → Result<Utf8PathBuf>`                   | `client.workspace().locate(path)`     | `sysand.workspace.locate(path)`      | `sysand.workspace.locate(path)`       |
| build   | `sysand workspace build [opts]`              | `workspace::build(&ctx, BuildOptions) → Result<Vec<BuildOutput>>` | `client.workspace().build(ctx, opts)` | `sysand.workspace.build(ctx, opts?)` | `sysand.workspace.build(ctx, **opts)` |

## Per-Command Options

Options that are unique to a single command live in that command's
options struct. Options shared across commands are composed in via
`IndexOptions` or `SourceSpec`.

### InitOptions

| Field            | Type             | Default | CLI                |
| ---------------- | ---------------- | ------- | ------------------ |
| `name`           | `Option<String>` | None    | `--name`           |
| `publisher`      | `Option<String>` | None    | `--publisher`      |
| `version`        | `Option<String>` | None    | `--version`        |
| `license`        | `Option<String>` | None    | `--license`        |
| `allow_non_spdx` | `bool`           | `false` | `--allow-non-spdx` |

### CloneOptions

| Field         | Type             | Default  | CLI               |
| ------------- | ---------------- | -------- | ----------------- |
| `target`      | `Option<Path>`   | None     | `--target`        |
| `version`     | `Option<String>` | None     | `--version`       |
| `deps`        | `DepsMode`       | `All`    | `--deps`          |
| `include_std` | `bool`           | `false`  | `--include-std`   |
| `index`       | `IndexOptions`   | defaults | `[index options]` |

### SourceAddOptions

| Field           | Type           | Default | CLI               |
| --------------- | -------------- | ------- | ----------------- |
| `checksum`      | `ChecksumMode` | `None`  | `--checksum`      |
| `index_symbols` | `IndexSymbols` | `On`    | `--index-symbols` |
| `language`      | `Language`     | `Auto`  | `--language`      |

### UsageAddOptions

| Field         | Type                 | Default  | CLI                          |
| ------------- | -------------------- | -------- | ---------------------------- |
| `version_req` | `Option<String>`     | None     | positional `[VERSION_REQ]`   |
| `source`      | `Option<SourceSpec>` | None     | `--source-kind` + `--source` |
| `update`      | `UpdateMode`         | `Sync`   | `--update`                   |
| `include_std` | `bool`               | `false`  | `--include-std`              |
| `index`       | `IndexOptions`       | defaults | `[index options]`            |

### BuildOptions

| Field         | Type           | Default    | CLI             |
| ------------- | -------------- | ---------- | --------------- |
| `target`      | `Option<Path>` | None       | `--target`      |
| `compression` | `Compression`  | `Deflated` | `--compression` |

### LockUpdateOptions

| Field         | Type           | Default  | CLI               |
| ------------- | -------------- | -------- | ----------------- |
| `include_std` | `bool`         | `false`  | `--include-std`   |
| `index`       | `IndexOptions` | defaults | `[index options]` |

### EnvCreateOptions

| Field | Type           | Default      | CLI     |
| ----- | -------------- | ------------ | ------- |
| `env` | `Option<Path>` | `sysand_env` | `--env` |

### EnvSyncOptions

| Field         | Type           | Default      | CLI               |
| ------------- | -------------- | ------------ | ----------------- |
| `env`         | `Option<Path>` | `sysand_env` | `--env`           |
| `include_std` | `bool`         | `false`      | `--include-std`   |
| `index`       | `IndexOptions` | defaults     | `[index options]` |

### EnvInstallOptions

| Field             | Type                 | Default      | CLI                          |
| ----------------- | -------------------- | ------------ | ---------------------------- |
| `version_req`     | `Option<String>`     | None         | positional `[VERSION_REQ]`   |
| `env`             | `Option<Path>`       | `sysand_env` | `--env`                      |
| `source`          | `Option<SourceSpec>` | None         | `--source-kind` + `--source` |
| `allow_overwrite` | `bool`               | `false`      | `--allow-overwrite`          |
| `allow_multiple`  | `bool`               | `false`      | `--allow-multiple`           |
| `deps`            | `DepsMode`           | `All`        | `--deps`                     |
| `include_std`     | `bool`               | `false`      | `--include-std`              |
| `index`           | `IndexOptions`       | defaults     | `[index options]`            |

### EnvUninstallOptions

| Field         | Type             | Default      | CLI                        |
| ------------- | ---------------- | ------------ | -------------------------- |
| `version_req` | `Option<String>` | None         | positional `[VERSION_REQ]` |
| `env`         | `Option<Path>`   | `sysand_env` | `--env`                    |

### EnvListOptions

| Field | Type           | Default      | CLI     |
| ----- | -------------- | ------------ | ------- |
| `env` | `Option<Path>` | `sysand_env` | `--env` |

## CLI

### Grammar

```
sysand [GLOBAL_OPTIONS] [<namespace>] <verb> [OPERANDS...] [OPTIONS...]
```

- Project operations (`init`, `locate`, `clone`, `build`) are root
  verbs — no namespace prefix
- `source` and `usage` are root-level namespaces for project manifest
  operations
- `lock`, `env`, and `workspace` are namespaces for non-project
  operations
- The final token in the command path is always the verb (`add`,
  `remove`, `update`, `sync`, `create`, `init`, `build`, `locate`)
- Required data is positional operands
- Optional behavior is named options

### Global Options

```
--config <auto|none|PATH>                        Config mode
--log-level <error|warn|info|debug|trace>        Terminal logging
--format <text|json>                             Rendering only; does not change result type
```

### Namespaces

| Namespace   | Purpose                                                |
| ----------- | ------------------------------------------------------ |
| (root)      | Project lifecycle: `init`, `locate`, `clone`, `build`  |
| `source`    | Project source file management                         |
| `usage`     | Project usage (dependency) management                  |
| `lock`      | Lockfile operations                                    |
| `env`       | Environment creation, sync, install/uninstall, listing |
| `workspace` | Workspace operations                                   |

### CLI-Only Concerns

These exist only in the CLI surface:

- `--log-level` and `--format` (global options above)
- Implicit CWD discovery when `--project`/`--workspace` is omitted
- Exit codes (0 success, non-zero error)
- Color and terminal formatting

## Rationale

**Why project operations are root-level.** Project operations (`init`,
`build`, `clone`, `locate`) are the most common commands. A `project`
namespace prefix adds typing cost without aiding disambiguation — these
verbs don't collide with `env`, `lock`, or `workspace` commands.
`source` and `usage` remain namespaced because their verbs (`add`,
`remove`) would otherwise collide with each other. `workspace` stays
namespaced because `build` and `locate` overlap with the root verbs,
and workspace operations are less frequent.

**Why noun-verb grammar.** Verbs as subcommands (not flags) guarantee
one command = one return shape, which means bindings can be generated
mechanically. The alternative — flags that change behavior (e.g.,
`--set` vs `--get` on the same command) — would require per-flag
return type logic in every surface.

**Why field-level accessors were removed.** An earlier design had
`project info name get`, `project info name set`, `project metadata`
commands, and a `project show` aggregate. These created ~20 commands
that were thin wrappers over reading/writing `.project.json` fields.
Across four binding surfaces with testing and documentation, the
maintenance cost was high for low value — users can edit the JSON
file directly. Removing them cut the API surface significantly.

**Why lookup was removed.** The original design had a `lookup`
namespace for querying package indexes (name, version, usages, etc.).
This created ~50 commands across all query fields. In practice, index
queries are only needed internally by the solver during dependency
resolution. Exposing them as public API would mean maintaining those
~50 commands across 4 binding surfaces with testing and docs, for a
use case that doesn't justify the cost. The version constraint rules
still apply — they're used internally by the solver.

**Why thin list commands were cut.** `project usage list` and
`project source list` were wrappers over reading `.project.json`.
`env list` was kept because it inspects actual installed state in
`sysand_env/`, which isn't a simple file read.

**Why `source` and `usage` are namespaces, not root verbs.** Both have
`add` and `remove` verbs that would collide at the root level.
Namespacing them as `source add` / `usage add` keeps the commands
parallel and unambiguous.
