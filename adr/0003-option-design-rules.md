# ADR-0003: Option Design Rules

- **Status**: Accepted
- **Date**: 2026-03-19

## Context

The current CLI reuses option names with different meanings across commands
(`--path` means five different things) and relies on negative boolean flags
(`--no-lock`, `--no-sync`, `--no-deps`) to control behavior. Both patterns
project poorly to bindings and make the API harder to learn.

We need rules for how options behave across all commands.

## Decision

### 1. Stable option names

The same option name means the same thing in every command where it appears.

| Option                        | Meaning                  | Always refers to                            |
| ----------------------------- | ------------------------ | ------------------------------------------- |
| `--project <PATH>`            | Project root path        | The project being operated on               |
| `--workspace <PATH>`          | Workspace root path      | The workspace being operated on             |
| `--env <PATH>`                | Environment directory    | The sysand_env being operated on            |
| `--target <PATH>`             | Output target path       | Where to write output artifacts             |
| `--config <auto\|none\|PATH>` | Config mode              | How to load sysand.toml (see ADR-0001)      |
| `--index <URL>`               | Additional index URL     | Extra package index (repeatable)            |
| `--default-index <URL>`       | Default index URL        | Override default package index (repeatable) |
| `--include-std`               | Include standard library | Include KerML/SysML v2 stdlib entries       |

If a command needs a path that doesn't fit one of these, it gets its own
dedicated name — never reuse an existing name with a different meaning.

### 2. Positive options over negative flags

No `--no-*` flags. Use positive enums or positive flags instead.

**Side-effect control:**

```
Current:   sysand add <IRI> --no-lock --no-sync
Reworked:  sysand usage add <IRI> --update manifest|lock|sync
```

`--update` controls how far side effects propagate:

- `manifest` — update project info only
- `lock` — update project info + regenerate lockfile
- `sync` — update project info + regenerate lockfile + sync environment

**Usage control:**

```
Current:   sysand clone <LOC> --no-deps
Reworked:  sysand project clone <LOC> --deps all|none
```

**Validation relaxation:**

```
Current:   sysand project init --no-spdx
Reworked:  sysand project init --allow-non-spdx
```

Still a boolean, but positive framing — the flag says what it _allows_,
not what it _disables_.

Note: `--allow-non-semver` was originally planned here but removed —
semver is now always required (ADR-0007).

**Symbol control:**

```
Current:   sysand include [PATHS] --no-index-symbols
Reworked:  sysand project source add [PATHS] --index-symbols on|off
```

### 3. Shared option groups become shared types

When multiple commands accept the same set of options, those options form
a shared type in the library and bindings:

**Resolve options** (`--index`, `--default-index`, `--index-mode`, `--include-std`):

| Surface | Type                                       |
| ------- | ------------------------------------------ |
| Rust    | `ResolveOptions` struct                    |
| Java    | `ResolveOptions` builder                   |
| JS/WASM | `ResolveOptions` object/interface          |
| Python  | `ResolveOptions` dataclass or keyword args |

This avoids repeating the same fields in every operation's input type.

### 4. Options carry defaults

Every option has a clear default that the library applies when the option is
omitted:

| Option            | Default                                                            |
| ----------------- | ------------------------------------------------------------------ |
| `--project`       | `"."` (CLI provides from CWD discovery; library requires explicit) |
| `--env`           | `"sysand_env"`                                                     |
| `--config`        | `auto`                                                             |
| `--update`        | `sync` (when applicable)                                           |
| `--deps`          | `all`                                                              |
| `--index-mode`    | `default`                                                          |
| `--index-symbols` | `on`                                                               |

Defaults are the same in every surface. The CLI may fill in defaults from
discovery before calling the library, but the library's own defaults are
explicit and documented.

## Consequences

- Option names are learnable — once you know `--project` in one command, you
  know it everywhere
- No negative booleans means binding APIs read naturally:
  `update="manifest"` instead of `no_lock=True, no_sync=True`
- Enum options scale better when new modes are added
- Shared option groups reduce type proliferation in the library
- Every option's default is explicit, making library behavior predictable
  without CLI context
