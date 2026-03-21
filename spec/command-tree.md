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

| Namespace   | Purpose                                                |
| ----------- | ------------------------------------------------------ |
| `project`   | Local project lifecycle, sources, usages               |
| `lock`      | Lockfile operations                                    |
| `env`       | Environment creation, sync, install/uninstall, listing |
| `workspace` | Workspace operations                                   |

## Grammar

```
sysand [GLOBAL_OPTIONS] <namespace> [<resource>...] <verb> [OPERANDS...] [OPTIONS...]
```

- Namespace and resource tokens are nouns (`project`, `source`,
  `usage`, `env`, `workspace`)
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

    locate [--project <PATH>]

    clone <LOCATOR>
      [--target <PATH>]
      [--version <VERSION>]
      [--deps all|none]
      [--include-std]
      [index options]

    source
      add <PATH>...
        [--project <PATH>]
        [--checksum none|sha256]
        [--index-symbols on|off]
        [--language auto|sysml|kerml]
      remove <PATH>...
        [--project <PATH>]

    usage
      add <IRI> [<VERSION_REQ>]
        [--project <PATH>]
        [--source-kind <KIND> --source <VALUE>]
        [--update manifest|lock|sync]
        [--include-std]
        [index options]
      remove <IRI>
        [--project <PATH>]

    build
      [--project <PATH>]
      [--target <PATH>]
      [--compression stored|deflated|bzip2|zstd|xz|ppmd]

  lock
    update
      [--project <PATH>]
      [--include-std]
      [index options]

  env
    create [--env <PATH>]
    sync
      [--project <PATH>]
      [--env <PATH>]
      [--include-std]
      [index options]
    install <IRI> [<VERSION_REQ>]
      [--env <PATH>]
      [--source-kind <KIND> --source <VALUE>]
      [--allow-overwrite]
      [--allow-multiple]
      [--deps all|none]
      [--include-std]
      [index options]
    uninstall <IRI> [<VERSION_REQ>]
      [--env <PATH>]
    list [--env <PATH>]

  workspace
    locate [--workspace <PATH>]
    build
      [--workspace <PATH>]
      [--target <PATH>]
      [--compression stored|deflated|bzip2|zstd|xz|ppmd]

```

Where `[index options]` expands to:

```
[--index <URL>]...
[--default-index <URL>]...
[--index-mode default|none]
```

## Build Compression

`stored` and `deflated` available by default. `bzip2`, `zstd`, `xz`,
`ppmd` behind feature flags. Default is `deflated`.
