# 0005: Current CLI — Mapping Analysis Against Design Principles

## Current Command Tree (v0.0.10)

```
sysand
  init [PATH]
  new                              (deprecated alias for init)
  add <IRI|--path PATH> [VERSION]
  remove <IRI|--path PATH>
  clone <LOCATOR|--iri|--path>
  include [PATHS]...
  exclude [PATHS]...
  build [PATH]
  lock
  sync
  info [--path|--iri|--auto-location]
    name         [--set]
    publisher    [--set] [--clear]
    description  [--set] [--clear]
    version      [--set] [--no-semver]
    license      [--set] [--clear] [--no-spdx]
    website      [--set] [--clear]
    maintainer   [--set] [--add] [--remove] [--clear] [--numbered]
    topic        [--set] [--add] [--remove] [--clear] [--numbered]
    usage        [--numbered]
    metamodel    [--set] [--set-custom] [--release] [--release-custom] [--clear]
    index        [--numbered]
    created
    includes-derived  [--set] [--clear]
    includes-implied  [--set] [--clear]
    checksum     [--numbered]
  sources [--no-deps] [--include-std]
  env
    install <IRI> [VERSION]
    uninstall <IRI> [VERSION]
    list
    sources <IRI> [VERSION]
  print-root
```

---

## Issue-by-Issue Analysis

### Issue 1: `info` is overloaded (local vs remote)

**Current:** `sysand info` operates on local projects by default, but `--iri`
and `--auto-location` make it resolve remote projects. The return shape changes:
local returns one project, remote can return multiple matches.

```
sysand info --path .         →  local project
sysand info --iri urn:pkg    →  resolved remote (possibly multiple)
sysand info --auto-location  →  tries IRI, falls back to path
```

**Principle violated:** Rule 2 (one command = one return shape), Rule 3 (stable
option names — `--path` here means "project to inspect" not "project root").

**Proposed fix:** Split into `project show` (local) and `resolve show` (remote).

### Issue 2: Flags as verbs in `info` subcommands

**Current:** Every `info` field uses flags to encode the action:

```
sysand info name --set "foo"
sysand info maintainer --add "Alice"
sysand info maintainer --remove 2
sysand info maintainer --clear
```

These flags are mutually exclusive verbs disguised as options. The "get" case
is the absence of all flags — implicit default behavior.

**Principle violated:** Rule 1 (command path encodes operation), Rule 4 (verbs
as subcommands not mode flags).

**Projection problem:**

```python
# What the current shape forces:
sysand.info.maintainer(set=["Alice"])       # awkward
sysand.info.maintainer(add="Alice")         # which flag is active?
sysand.info.maintainer(remove=2)            # mutually exclusive kwargs

# What the reworked shape gives:
sysand.project.info.maintainer.add("Alice")  # clear
sysand.project.info.maintainer.remove(2)     # clear
sysand.project.info.maintainer.list()        # explicit
```

### Issue 3: Negative boolean flags

**Current:**

| Command        | Flag                 | Meaning                          |
| -------------- | -------------------- | -------------------------------- |
| `add`          | `--no-lock`          | Don't generate lockfile          |
| `add`          | `--no-sync`          | Don't install dependencies       |
| `clone`        | `--no-deps`          | Don't resolve dependencies       |
| `include`      | `--no-index-symbols` | Don't extract symbols            |
| `sources`      | `--no-deps`          | Don't include dependency sources |
| `env install`  | `--no-deps`          | Don't install dependencies       |
| `init`         | `--no-semver`        | Don't validate as SemVer         |
| `init`         | `--no-spdx`          | Don't validate as SPDX           |
| `info version` | `--no-semver`        | Don't validate as SemVer         |
| `info license` | `--no-spdx`          | Don't validate as SPDX           |

**Principle violated:** Rule 5 (positive enums over negative booleans).

**Proposed fix:** Two categories:

- Side-effect control: `--no-lock` + `--no-sync` → `--update manifest|lock|sync`
- Usage control: `--no-deps` → `--deps all|none`
- Validation relaxation: `--no-semver`, `--no-spdx` → `--allow-non-semver`,
  `--allow-non-spdx` (still booleans but positive framing)
- Symbol control: `--no-index-symbols` → `--index-symbols on|off`

### Issue 4: `--path` means different things

**Current:**

| Command              | `--path` means                              |
| -------------------- | ------------------------------------------- |
| `add --path`         | Path to project being _added as a usage_    |
| `remove --path`      | Path to project being _removed from usages_ |
| `clone --path`       | Path to project being _cloned from_         |
| `info --path`        | Path to project being _inspected_           |
| `env install --path` | Path to interchange project to install      |

All semantically different uses of the same option name.

**Principle violated:** Rule 3 (stable option names).

**Proposed fix:** Dedicated names: `--project` (project root being operated on),
`--source` (source location), `--target` (output), `--env` (environment path).

### Issue 5: Flat top-level commands mixing concerns

**Current:** All commands live at the top level:

```
sysand init          (project lifecycle)
sysand add           (usage management)
sysand remove        (usage management)
sysand include       (source file management)
sysand exclude       (source file management)
sysand build         (build)
sysand lock          (lock management)
sysand sync          (env management)
sysand sources       (source listing)
sysand print-root    (project query)
sysand info          (project info + remote resolve)
sysand env           (env management)
```

No namespace grouping. `sync` is top-level but conceptually belongs under `env`.
`sources` is top-level but duplicated under `env sources`. `include`/`exclude`
are about project sources but not grouped.

**Principle violated:** Rule 1 (command path encodes operation — namespaces
should group related operations).

**Proposed grouping:**

```
sysand project init|show|root|clone
sysand project source add|remove|list
sysand project info <field> get|set|add|remove|clear|list
sysand project metadata <field> get|set|clear|list
sysand usage add|remove|list
sysand lock update
sysand env create|sync|install|uninstall|list
sysand env source list
sysand build project|workspace
sysand resolve show|info|metadata
```

### Issue 6: `build` mode is implicit

**Current:** `sysand build` auto-detects whether to build a single project or
a workspace based on context (inside project → single; inside workspace but
outside project → workspace).

**Principle violated:** Rule 4 (explicit subcommands over mode flags), Rule 8
(selectors over ambient discovery).

**Proposed fix:** `sysand build project` and `sysand build workspace` as
explicit subcommands.

### Issue 7: `add` source specification is complex

**Current:** `add` has 8 `--as-*` flags for specifying the source kind:

```
--from-path, --from-url, --as-editable, --as-local-src,
--as-local-kpar, --as-remote-src, --as-remote-kpar, --as-remote-git
```

These are mutually exclusive flags encoding a type discriminator.

**Principle violated:** Effectively flags-as-verbs; maps poorly to bindings.

**Proposed fix:** `--source-kind <KIND> --source <VALUE>` — one enum + one value.

### Issue 8: `info` with no subcommand shows full project

**Current:** `sysand info` (no subcommand) shows the full project info.
`sysand info name` (no `--set`) shows just the name. The "get" verb is implicit.

**Projection problem:** In bindings, is `sysand.info()` a different operation
than `sysand.info.name()`? The current design conflates "show everything" with
the namespace for field access.

**Proposed fix:** `sysand project show` for full display, `sysand project info
name get` for field access. Separate commands, separate operations.

### Issue 9: Resolution options appear everywhere (even where irrelevant)

**Current:** Every `info` subcommand carries resolution options (`--index`,
`--default-index`, `--no-index`, `--include-std`) because `info` can operate
on remote projects. For local-only fields like `info name --set`, resolution
options are meaningless.

**Principle violated:** Options should be relevant to the operation.

**Proposed fix:** When `info` is split into local (`project info`) and remote
(`resolve info`), resolution options only appear on `resolve` commands.

### Issue 10: `env` creates environment implicitly

**Current:** Running `sysand env` with no subcommand creates the environment
directory. This is a side-effect of the command existing.

**Proposed fix:** Explicit `sysand env create`.

---

## Current → Proposed Mapping Overview

| Current CLI                            | Issues   | Proposed CLI                                           |
| -------------------------------------- | -------- | ------------------------------------------------------ |
| `sysand init [PATH]`                   | —        | `sysand project init <PATH>`                           |
| `sysand add <IRI> [VER]`               | #3 #4 #7 | `sysand usage add <IRI> [VER] --update sync`           |
| `sysand remove <IRI>`                  | #4       | `sysand usage remove <IRI>`                            |
| `sysand clone <LOC>`                   | #3 #4    | `sysand project clone <LOC>`                           |
| `sysand include [PATHS]`               | #3 #5    | `sysand project source add <PATHS>`                    |
| `sysand exclude [PATHS]`               | #5       | `sysand project source remove <PATHS>`                 |
| `sysand build [PATH]`                  | #6       | `sysand build project` / `sysand build workspace`      |
| `sysand lock`                          | #5       | `sysand lock update`                                   |
| `sysand sync`                          | #5       | `sysand env sync`                                      |
| `sysand info`                          | #1 #8    | `sysand project show`                                  |
| `sysand info --iri X`                  | #1       | `sysand resolve show <IRI>`                            |
| `sysand info name`                     | #2 #8    | `sysand project info name get`                         |
| `sysand info name --set X`             | #2       | `sysand project info name set <NAME>`                  |
| `sysand info maintainer --add X`       | #2       | `sysand project info maintainer add <VAL>`             |
| `sysand info maintainer --remove N`    | #2       | `sysand project info maintainer remove <IDX>`          |
| `sysand info maintainer --clear`       | #2       | `sysand project info maintainer clear`                 |
| `sysand info metamodel --set sysml`    | #2       | `sysand project metadata metamodel set-standard sysml` |
| `sysand info metamodel --set-custom X` | #2       | `sysand project metadata metamodel set-custom <IRI>`   |
| `sysand sources`                       | #3 #5    | `sysand project source list`                           |
| `sysand print-root`                    | #5       | `sysand project root`                                  |
| `sysand env` (no subcmd)               | #10      | `sysand env create`                                    |
| `sysand env install`                   | #3       | `sysand env install` (with `--deps`)                   |
| `sysand env sources <IRI>`             | #5       | `sysand env source list <IRI>`                         |

---

## Summary of Violations by Principle

| Principle                                | Violations                                                   |
| ---------------------------------------- | ------------------------------------------------------------ |
| **Path = operation** (Rule 1)            | Flat top-level; `info` mixes local/remote; flags-as-verbs    |
| **One return shape** (Rule 2)            | `info` with `--path` vs `--iri`                              |
| **Stable option names** (Rule 3)         | `--path` means 5 different things                            |
| **Subcommands over mode flags** (Rule 4) | `build` auto-detects; `info` uses `--set`/`--add`/`--remove` |
| **Positive enums** (Rule 5)              | 10 `--no-*` flags                                            |
| **Side effects explicit** (Rule 7)       | `add` (usage add) implicitly locks + syncs                   |
| **Selectors over discovery** (Rule 8)    | `build`, `lock`, `sync`, `sources` all rely on cwd           |
