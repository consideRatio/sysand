# Specification

This directory is the **living specification** for sysand. Each file
reflects the current state of its topic, including a Rationale section
that captures _why_ the design is what it is.

When you want to know "how does X work right now?" or "why was it
designed this way?", look here.

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
