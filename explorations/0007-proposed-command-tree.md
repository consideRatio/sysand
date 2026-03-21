# 0007: Proposed Command Tree

**Status: Distilled into ADRs 0001–0005**

## Governing ADRs

- ADR-0001: Discovery and config boundaries (ProjectContext, WorkspaceContext)
- ADR-0002: CLI noun-verb grammar (namespace-resource-verb structure)
- ADR-0003: Option design rules (stable names, positive enums, shared groups)

## Global Options

```
--config <auto|none|PATH>           Config mode (see ADR-0001)
--log-level <error|warn|info|debug|trace>
--format <text|json>                Rendering only; does not change result type
```

## Proposed Command Tree

```
sysand
  project
    init <PATH>
      [--name <NAME>]
      [--publisher <PUBLISHER>]
      [--version <VERSION>]
      [--allow-non-semver]
      [--license <SPDX_EXPR>]
      [--allow-non-spdx]

    show
      [--project <PATH>]

    root
      [--project <PATH>]

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
        set <VERSION> [--project <PATH>] [--allow-non-semver]
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
    create
      [--env <PATH>]
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
    list
      [--env <PATH>]
    source
      list <IRI> [<VERSION_REQ>]
        [--env <PATH>]
        [--deps all|none]
        [--include-std]

  build
    project
      [--project <PATH>]
      [--target <PATH>]
      [--compression stored|deflated|bzip2|zstd|xz|ppmd]
    workspace
      [--workspace <PATH>]
      [--target <PATH>]
      [--compression stored|deflated|bzip2|zstd|xz|ppmd]

  resolve
    show <IRI_OR_URL>
      [--relative-root <PATH>]
      [resolve options]
    info
      name get <IRI_OR_URL> [--relative-root <PATH>] [resolve options]
      description get <IRI_OR_URL> [--relative-root <PATH>] [resolve options]
      version get <IRI_OR_URL> [--relative-root <PATH>] [resolve options]
      license get <IRI_OR_URL> [--relative-root <PATH>] [resolve options]
      website get <IRI_OR_URL> [--relative-root <PATH>] [resolve options]
      maintainer list <IRI_OR_URL> [--relative-root <PATH>] [resolve options]
      topic list <IRI_OR_URL> [--relative-root <PATH>] [resolve options]
      usage list <IRI_OR_URL> [--relative-root <PATH>] [resolve options]
    metadata
      created get <IRI_OR_URL> [--relative-root <PATH>] [resolve options]
      index list <IRI_OR_URL> [--relative-root <PATH>] [resolve options]
      checksum list <IRI_OR_URL> [--relative-root <PATH>] [resolve options]
      metamodel get <IRI_OR_URL> [--relative-root <PATH>] [resolve options]
      includes-derived get <IRI_OR_URL> [--relative-root <PATH>] [resolve options]
      includes-implied get <IRI_OR_URL> [--relative-root <PATH>] [resolve options]
```

Where `[resolve options]` expands to:

```
[--index <URL>]...
[--default-index <URL>]...
[--index-mode default|none]
[--include-std]
```

## What Changed from Current CLI

| Change                                                                    | Reason                                  | ADR      |
| ------------------------------------------------------------------------- | --------------------------------------- | -------- |
| `init` → `project init`                                                   | Namespace grouping                      | ADR-0002 |
| `add`/`remove` → `usage add`/`usage remove`                               | Domain term; namespace grouping         | ADR-0002 |
| `include`/`exclude` → `project source add`/`remove`                       | Namespace grouping                      | ADR-0002 |
| `sources` → `project source list`                                         | Namespace grouping                      | ADR-0002 |
| `print-root` → `project root`                                             | Namespace grouping                      | ADR-0002 |
| `sync` → `env sync`                                                       | Belongs under env                       | ADR-0002 |
| `info` → `project show` + `project info` + `project metadata` + `resolve` | Split local/remote; split info/metadata | ADR-0002 |
| `info name --set` → `project info name set`                               | Verbs as subcommands                    | ADR-0002 |
| `build` auto-detect → `build project` / `build workspace`                 | Explicit subcommands                    | ADR-0002 |
| `env` (no subcmd) → `env create`                                          | Explicit verb                           | ADR-0002 |
| `--no-lock`/`--no-sync` → `--update manifest\|lock\|sync`                 | Positive enum                           | ADR-0003 |
| `--no-deps` → `--deps all\|none`                                          | Positive enum                           | ADR-0003 |
| `--no-semver` → `--allow-non-semver`                                      | Positive framing                        | ADR-0003 |
| `--no-index-symbols` → `--index-symbols on\|off`                          | Positive enum                           | ADR-0003 |
| `--path` (5 meanings) → `--project`/`--env`/`--target`                    | Stable names                            | ADR-0003 |
| `--no-config` → `--config none`                                           | Positive enum                           | ADR-0001 |

## Open Questions

1. ~~**`resolve` and config**~~ **Resolved**: `resolve` commands don't accept
   `--config`. Everything they need (indexes) is expressible via explicit
   flags (`--index`, `--default-index`). No config fields exist that aren't
   already covered by resolve option flags. If repetitive flag typing becomes
   a problem, that's solved by shell aliases or a future user-level config —
   not by shoehorning project config into a project-less context.

2. ~~**`build` compression options**~~ **Resolved**: `stored` and `deflated`
   are available by default (no extra deps). `bzip2`, `zstd`, `xz`, `ppmd`
   remain behind feature flags. Default is `deflated`. The CLI lists
   available methods; users enable additional ones via build features.

3. ~~**`project show` vs `project info`**~~ **Resolved**: `show` is the verb
   for full project snapshot display. `info` is the noun namespace for
   field-level access. No conflict.

4. ~~**List removal indexing**~~ **Resolved**: Remove by value, not by index.
   `maintainer remove <VALUE>` and `topic remove <VALUE>`. Matches universal
   CLI convention (npm, cargo, git, brew, etc.). Duplicates are prevented at
   `add` time. If an index escape hatch is ever needed, it can be added later
   as a `--index N` flag.

5. ~~**`env source list` vs `project source list`**~~ **Resolved**: Clear
   distinction. `project source list` lists sources for the local project
   (no IRI required). `env source list <IRI>` lists sources for a specific
   installed package (IRI required). Different inputs, different context.
