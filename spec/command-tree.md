# Command Tree

The complete CLI command tree. Every command maps structurally to
library functions in all binding surfaces (see `projection-rules.md`).

Sources: ADR-0002, ADR-0004, ADR-0006, ADR-0007, ADR-0008

## Global Options

```
--config <auto|none|PATH>                        Config mode
--log-level <error|warn|info|debug|trace>
--format <text|json>                             Rendering only; does not change result type
```

## Namespaces

| Namespace   | Purpose                                                        |
| ----------- | -------------------------------------------------------------- |
| `project`   | Local project lifecycle, sources, info fields, metadata fields |
| `usage`     | Usage management (add/remove/list)                             |
| `lock`      | Lockfile operations                                            |
| `env`       | Environment creation, sync, install/uninstall, listing         |
| `workspace` | Workspace operations                                           |
| `lookup`    | Index package queries (read-only, no config)                   |

## Grammar

```
sysand [GLOBAL_OPTIONS] <namespace> [<resource>...] <verb> [OPERANDS...] [OPTIONS...]
```

- Namespace and resource tokens are nouns (`project`, `info`, `source`,
  `env`, `usage`, `workspace`)
- The final token in the command path is always the verb (`get`, `set`,
  `add`, `remove`, `list`, `show`, `clear`, `update`, `sync`, `create`,
  `init`, `build`, `locate`)
- Required data is positional operands
- Optional behavior is named options

## Full Tree

```
sysand
  project
    init <PATH>
      [--name <n>]
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
      [lookup options]

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
        set <n> [--project <PATH>]
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
      [lookup options]
    remove <IRI>
      [--project <PATH>]
    list
      [--project <PATH>]

  lock
    update
      [--project <PATH>]
      [lookup options]

  env
    create [--env <PATH>]
    sync
      [--project <PATH>]
      [--env <PATH>]
      [lookup options]
    install <IRI> [<VERSION_REQ>]
      [--env <PATH>]
      [--source-kind <KIND> --source <VALUE>]
      [--allow-overwrite]
      [--allow-multiple]
      [--deps all|none]
      [lookup options]
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

  lookup
    show <IRI> [<VERSION_CONSTRAINT>]
      [lookup options]
    info
      name get <IRI> [<VERSION_CONSTRAINT>] [lookup options]
      description get <IRI> [<VERSION_CONSTRAINT>] [lookup options]
      version get <IRI> [<VERSION_CONSTRAINT>] [lookup options]
      license get <IRI> [<VERSION_CONSTRAINT>] [lookup options]
      website get <IRI> [<VERSION_CONSTRAINT>] [lookup options]
      maintainer list <IRI> [<VERSION_CONSTRAINT>] [lookup options]
      topic list <IRI> [<VERSION_CONSTRAINT>] [lookup options]
      usage list <IRI> [<VERSION_CONSTRAINT>] [lookup options]
    metadata
      created get <IRI> [<VERSION_CONSTRAINT>] [lookup options]
      index list <IRI> [<VERSION_CONSTRAINT>] [lookup options]
      checksum list <IRI> [<VERSION_CONSTRAINT>] [lookup options]
      metamodel get <IRI> [<VERSION_CONSTRAINT>] [lookup options]
      includes-derived get <IRI> [<VERSION_CONSTRAINT>] [lookup options]
      includes-implied get <IRI> [<VERSION_CONSTRAINT>] [lookup options]
```

Where `[lookup options]` expands to:

```
[--index <URL>]...
[--default-index <URL>]...
[--index-mode default|none]
[--include-std]
```

## Build Compression

`stored` and `deflated` available by default. `bzip2`, `zstd`, `xz`,
`ppmd` behind feature flags. Default is `deflated`.
