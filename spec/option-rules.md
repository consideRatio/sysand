# Option Design Rules

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
form a shared type in the library and bindings. Example: `IndexOptions`
groups `--index`, `--default-index`, `--index-mode`.

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

## Rationale

**Why no `--no-*` flags.** Negative flags create double-negation
confusion (`--no-verify=false`). Positive enums are unambiguous and
project cleanly to all binding surfaces — each variant maps to an
enum value, not a boolean with inverted semantics.

**Why semver is mandatory.** Non-semver version strings break the
solver's ability to order versions, evaluate range constraints, and
filter pre-releases. An `--allow-non-semver` flag was originally part
of the design but was removed because it would require every
constraint-evaluation code path to handle unorderable strings as a
special case. Rejecting non-semver at every entry point (init,
manifest read, index query) keeps the version resolution pipeline
simple and correct.
