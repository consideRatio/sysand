# Specification

This directory is the **living specification** for sysand. It reflects
the cumulative current state of all design decisions.

When you want to know "how does X work right now?", look here.

Files in this directory are updated when new ADRs are accepted. Unlike
ADRs and explorations, these files are mutable — they always reflect
the latest state.

## Files

- `public-api.md` — complete public API: types, functions, options, CLI grammar across all five surfaces
- `data-model.md` — file schemas: .project.json, .meta.json, sysand.toml, lockfile, workspace
- `projection-rules.md` — mechanical rules for projecting CLI to Rust, Java, JS/WASM, Python
- `option-rules.md` — option naming, defaults, positive flags, semver requirement
- `discovery-and-config.md` — locate behavior, config loading, source overrides, resolver priority
- `error-model.md` — SysandError, ErrorCode enum
- `version-resolution.md` — semver, version constraints
- `dependency-resolution.md` — lock, sync, solver pipeline, lockfile contents
- `binding-architecture.md` — facade pattern, storage backends, binding tools, maintenance strategy

## Relationship to Other Directories

- **`spec/`** (here) — what the design _is_ right now
- **`adr/`** — _why_ each decision was made (immutable, historical)
- **`explorations/`** — _how_ we arrived at decisions (immutable, historical)
