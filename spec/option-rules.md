# Option Design Rules

Sources: ADR-0003, ADR-0007

## Stable Option Names

The same option name means the same thing in every command where it
appears.

| Option                        | Meaning                  | Always refers to                            |
| ----------------------------- | ------------------------ | ------------------------------------------- |
| `--project <PATH>`            | Project root path        | The project being operated on               |
| `--workspace <PATH>`          | Workspace root path      | The workspace being operated on             |
| `--env <PATH>`                | Environment directory    | The sysand_env being operated on            |
| `--target <PATH>`             | Output target path       | Where to write output artifacts             |
| `--config <auto\|none\|PATH>` | Config mode              | How to load sysand.toml                     |
| `--index <URL>`               | Additional index URL     | Extra package index (repeatable)            |
| `--default-index <URL>`       | Default index URL        | Override default package index (repeatable) |
| `--include-std`               | Include standard library | Include KerML/SysML v2 stdlib entries       |

If a command needs a path that doesn't fit one of these, it gets its
own dedicated name.

## Positive Options

No `--no-*` flags. Use positive enums or positive flags instead.

Side-effect control: `--update manifest|lock|sync`

Usage control: `--deps all|none`

Validation relaxation: `--allow-non-spdx`

Symbol control: `--index-symbols on|off`

## Shared Option Groups

When multiple commands accept the same set of options, those options
form a shared type in the library and bindings. Example: `LookupOptions`
groups `--index`, `--default-index`, `--index-mode`, `--include-std`.

## Defaults

| Option            | Default                                                            |
| ----------------- | ------------------------------------------------------------------ |
| `--project`       | `"."` (CLI provides from CWD discovery; library requires explicit) |
| `--env`           | `"sysand_env"`                                                     |
| `--config`        | `auto`                                                             |
| `--update`        | `sync` (when applicable)                                           |
| `--deps`          | `all`                                                              |
| `--index-mode`    | `default`                                                          |
| `--index-symbols` | `on`                                                               |

Defaults are the same in every surface.

## Semver Required

All project versions must be valid semver (Semantic Versioning 2.0.0).
There is no `--allow-non-semver` flag. Sysand errors if a non-semver
version is encountered anywhere.
