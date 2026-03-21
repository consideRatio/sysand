# ADR-0004: Command Tree

- **Status**: Accepted
- **Date**: 2026-03-19
- **Applies**: ADR-0001 (discovery/config), ADR-0002 (noun-verb grammar),
  ADR-0003 (option rules)

## Context

The current CLI (v0.0.10) has a flat command structure with issues documented
in exploration 0005: overloaded `info` command (local + remote), flags-as-verbs,
`--path` meaning five different things, implicit side effects, and no namespace
grouping.

This ADR defines the complete reworked command tree.

## Decision

### Global Options

```
--config <auto|none|PATH>                        Config mode (ADR-0001)
--log-level <error|warn|info|debug|trace>
--format <text|json>                             Rendering only; does not change result type
```

### Namespaces

| Namespace   | Purpose                                                        |
| ----------- | -------------------------------------------------------------- |
| `project`   | Local project lifecycle, sources, info fields, metadata fields |
| `usage`     | Usage management (add/remove/list)                             |
| `lock`      | Lockfile operations                                            |
| `env`       | Environment creation, sync, install/uninstall, listing         |
| `workspace` | Workspace operations                                           |
| `resolve`   | Remote package queries (read-only, no config)                  |

### Command Tree

```
sysand
  project
    init <PATH>
      [--name <NAME>]
      [--publisher <PUBLISHER>]
      [--version <VERSION>]
      [--license <SPDX_EXPR>]
      [--allow-non-spdx]

    show [--project <PATH>]
    locate [--project <PATH>]

    clone <LOCATOR>
      [--target <PATH>]
      [--version <VERSION>]
      [--deps all|none]
      [--source-kind <KIND> --source <VALUE>]
      [resolve options]

    source
      add <PATH>...
        [--project <PATH>]
        [--checksum none|sha256]
        [--index-symbols on|off]
        [--language auto|sysml|kerml]
      remove <PATH>...
        [--project <PATH>]
      list
        [--project <PATH>]
        [--deps all|none]
        [--env <PATH>]
        [--include-std]

    info
      name
        get [--project <PATH>]
        set <NAME> [--project <PATH>]
      publisher
        get [--project <PATH>]
        set <PUBLISHER> [--project <PATH>]
        clear [--project <PATH>]
      description
        get [--project <PATH>]
        set <DESCRIPTION> [--project <PATH>]
        clear [--project <PATH>]
      version
        get [--project <PATH>]
        set <VERSION> [--project <PATH>]
      license
        get [--project <PATH>]
        set <SPDX_EXPR> [--project <PATH>] [--allow-non-spdx]
        clear [--project <PATH>]
      website
        get [--project <PATH>]
        set <URI> [--project <PATH>]
        clear [--project <PATH>]
      maintainer
        list [--project <PATH>]
        set <VALUE>... [--project <PATH>]
        add <VALUE> [--project <PATH>]
        remove <VALUE> [--project <PATH>]
        clear [--project <PATH>]
      topic
        list [--project <PATH>]
        set <VALUE>... [--project <PATH>]
        add <VALUE> [--project <PATH>]
        remove <VALUE> [--project <PATH>]
        clear [--project <PATH>]

    metadata
      created
        get [--project <PATH>]
      index
        list [--project <PATH>]
      checksum
        list [--project <PATH>]
      metamodel
        get [--project <PATH>]
        set-standard <sysml|kerml> [--release <YYYYMMDD>] [--project <PATH>]
        set-custom <IRI> [--project <PATH>]
        clear [--project <PATH>]
      includes-derived
        get [--project <PATH>]
        set <true|false> [--project <PATH>]
        clear [--project <PATH>]
      includes-implied
        get [--project <PATH>]
        set <true|false> [--project <PATH>]
        clear [--project <PATH>]

    build
      [--project <PATH>]
      [--target <PATH>]
      [--compression stored|deflated|bzip2|zstd|xz|ppmd]

  usage
    add <IRI> [<VERSION_REQ>]
      [--project <PATH>]
      [--source-kind <KIND> --source <VALUE>]
      [--update manifest|lock|sync]
      [resolve options]
    remove <IRI>
      [--project <PATH>]
    list
      [--project <PATH>]

  lock
    update
      [--project <PATH>]
      [resolve options]

  env
    create [--env <PATH>]
    sync
      [--project <PATH>]
      [--env <PATH>]
      [resolve options]
    install <IRI> [<VERSION_REQ>]
      [--env <PATH>]
      [--source-kind <KIND> --source <VALUE>]
      [--allow-overwrite]
      [--allow-multiple]
      [--deps all|none]
      [resolve options]
    uninstall <IRI> [<VERSION_REQ>]
      [--env <PATH>]
    list [--env <PATH>]
    source
      list <IRI> [<VERSION_REQ>]
        [--env <PATH>]
        [--deps all|none]
        [--include-std]

  workspace
    locate [--workspace <PATH>]
    build
      [--workspace <PATH>]
      [--target <PATH>]
      [--compression stored|deflated|bzip2|zstd|xz|ppmd]

  resolve
    show <IRI> [<VERSION_CONSTRAINT>]
      [resolve options]
    info
      name get <IRI> [<VERSION_CONSTRAINT>] [resolve options]
      description get <IRI> [<VERSION_CONSTRAINT>] [resolve options]
      version get <IRI> [<VERSION_CONSTRAINT>] [resolve options]
      license get <IRI> [<VERSION_CONSTRAINT>] [resolve options]
      website get <IRI> [<VERSION_CONSTRAINT>] [resolve options]
      maintainer list <IRI> [<VERSION_CONSTRAINT>] [resolve options]
      topic list <IRI> [<VERSION_CONSTRAINT>] [resolve options]
      usage list <IRI> [<VERSION_CONSTRAINT>] [resolve options]
    metadata
      created get <IRI> [<VERSION_CONSTRAINT>] [resolve options]
      index list <IRI> [<VERSION_CONSTRAINT>] [resolve options]
      checksum list <IRI> [<VERSION_CONSTRAINT>] [resolve options]
      metamodel get <IRI> [<VERSION_CONSTRAINT>] [resolve options]
      includes-derived get <IRI> [<VERSION_CONSTRAINT>] [resolve options]
      includes-implied get <IRI> [<VERSION_CONSTRAINT>] [resolve options]
```

Where `[resolve options]` expands to:

```
[--index <URL>]...
[--default-index <URL>]...
[--index-mode default|none]
[--include-std]
```

### Key Design Decisions

**Local vs remote split:** The current `info` command is split into `project`
(local operations) and `resolve` (remote queries). They have different inputs,
different return shapes, and different config behavior. `resolve` commands
don't accept `--config` — they use explicit `--index`/`--default-index` flags
only.

**Info vs metadata split:** Project fields are split between `info` (from
`.project.json`: name, publisher, description, version, license, website,
maintainer, topic) and `metadata` (from `.meta.json`: created, index,
checksum, metamodel, includes-derived, includes-implied).

**Explicit verbs:** Every field operation uses a verb subcommand (`get`, `set`,
`add`, `remove`, `clear`, `list`). No mutually exclusive flag-verbs.

**List removal by value:** `maintainer remove` and `topic remove` take a value,
not an index. Duplicates are prevented at `add` time. Matches universal CLI
convention.

**Build compression:** `stored` and `deflated` available by default. `bzip2`,
`zstd`, `xz`, `ppmd` behind feature flags. Default is `deflated`.

**Build mode explicit:** `project build` and `workspace build` are separate
commands under their respective namespaces. No auto-detection of project vs
workspace context.

## What Changed from Current CLI

| Change                                                                    | Reason                                  | ADR  |
| ------------------------------------------------------------------------- | --------------------------------------- | ---- |
| `init` → `project init`                                                   | Namespace grouping                      | 0002 |
| `add`/`remove` → `usage add`/`remove`                                     | Domain term; namespace grouping         | 0002 |
| `include`/`exclude` → `project source add`/`remove`                       | Namespace grouping                      | 0002 |
| `sources` → `project source list`                                         | Namespace grouping                      | 0002 |
| `print-root` → `project locate`                                           | Namespace grouping; proper verb          | 0002, 0006 |
| `sync` → `env sync`                                                       | Belongs under env                       | 0002 |
| `info` → `project show` + `project info` + `project metadata` + `resolve` | Split local/remote; split info/metadata | 0002 |
| `info name --set` → `project info name set`                               | Verbs as subcommands                    | 0002 |
| `build` auto-detect → `project build` / `workspace build`                 | Noun-verb grammar; explicit subcommands | 0002 |
| `env` (no subcmd) → `env create`                                          | Explicit verb                           | 0002 |
| `--no-lock`/`--no-sync` → `--update manifest\|lock\|sync`                 | Positive enum                           | 0003 |
| `--no-deps` → `--deps all\|none`                                          | Positive enum                           | 0003 |
| `--no-semver` → semver always required                                    | Semver required (ADR-0007)              | 0007 |
| `--no-index-symbols` → `--index-symbols on\|off`                          | Positive enum                           | 0003 |
| `--path` (5 meanings) → `--project`/`--env`/`--target`                    | Stable names                            | 0003 |
| `--no-config` → `--config none`                                           | Positive enum                           | 0001 |
| `maintainer remove <INDEX>` → `remove <VALUE>`                            | By-value removal                        | —    |

## Consequences

- Every command has a clear namespace, verb, and return shape
- The command tree maps structurally to Rust modules, Java method chains,
  JS/WASM namespaces, and Python module paths (per ADR-0002)
- All options follow stable naming and positive framing (per ADR-0003)
- The CLI crate becomes a thin parser over the library API
- Deeper command paths mean more typing but consistent discoverability
