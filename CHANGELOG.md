# Changelog

Decisions made and their outcomes, in reverse chronological order.

## 2026-03-22

- **"package" → "project" in error codes** — `PackageNotFound` renamed
  to `ProjectNotInIndex`, `VersionNotFound` renamed to
  `VersionNotInIndex`. User-facing text uses "project" consistently
  per KerML terminology. "Package index" remains acceptable as a
  concept name.

- **spec/public-api.md added** — Complete public API across all five
  surfaces (CLI, Rust, Java, JS/WASM, Python). Types, enums, options
  structs, function signatures, CLI grammar. `command-tree.md` removed
  (absorbed into public-api). `discovery-and-config.md` trimmed to
  behavioral content only.

- **`env install` and `env uninstall` take `ProjectContext`** — Both
  need config for source overrides and index URLs, consistent with
  the reference implementation.

- **`locate` goes through client in Java** — No static methods.
  `client.project().locate()` and `client.workspace().locate()`,
  consistent with all other operations.

- **`project clone` has no `ProjectContext`** — Bootstrapping operation.
  Uses `IndexOptions` from arguments directly, no config file.

## 2026-03-21

- **`project info`, `project metadata`, `project show`, and `lookup`
  removed** — Not worth the CLI surface area. Users edit `.project.json`
  and `.meta.json` directly. Index queries remain internal to the solver.
  `[lookup options]` renamed to `[index options]`, `LookupOptions` type
  renamed to `IndexOptions`.

- **`usage` moved under `project`** — Usages live in `.project.json`,
  same as sources. Now `project usage add/remove` for consistency.

- **`project usage list`, `project source list` removed** — Thin
  wrappers over reading `.project.json` and the env directory. `env
list` kept for inspecting actual installed state.

- **`--include-std` separated from `IndexOptions`** — Controls
  standard library inclusion regardless of source type, not an index
  concern. Standalone flag on all resolution commands.

- **spec/ directory populated** — Living specification created with 6
  files: command-tree, projection-rules, option-rules,
  discovery-and-config, error-model, version-resolution. These are
  the single source of truth for the current design.

- **`resolve` renamed to `lookup`** — Public command namespace for
  querying indexes. "Resolve" reserved for internal solver concept.

- **ADR-0005 updated: Natural return types** — Dropped the four-wrapper
  taxonomy (`ScalarFieldResult`, `ListFieldResult`, `MutationResult`,
  `LookupFieldResult`). Every operation now returns `Result<T, SysandError>`
  where `T` is the natural type. Per exploration 0009.

- **ADR-0008: Single-version lookup** — Lookup always returns one
  package. Takes IRI + optional version constraint. Return types
  mirror local project info/metadata commands. `--relative-root` and
  `<IRI_OR_URL>` dropped.

- **ADR-0007: Semver required** — All project versions must be valid
  semver. `--allow-non-semver` removed. Simplifies resolution,
  constraint matching, and pre-release filtering.

- **ADR-0006: Locate** — `project::locate` and `workspace::locate`
  are library operations. Returns a path, not a context. Implicit
  locate from CWD remains CLI-only. `project root` renamed to
  `project locate`, `workspace locate` added.

- **Build namespace fix** — `build` moved from a top-level namespace
  to a verb under `project` and `workspace`: `sysand project build`,
  `sysand workspace build`.

- **Return types direction** (exploration 0009) — Drop the four-wrapper
  taxonomy (`ScalarFieldResult`, `ListFieldResult`, `MutationResult`,
  `LookupFieldResult`). Each operation returns `Result<T, SysandError>`
  where `T` is the natural type. ADR-0005 update still pending.

- **Lookup design direction** (exploration 0010) — Single-version lookup
  resolution, pre-release opt-in via constraint string only, no flags.
  Distilled into ADR-0008.

- **IRI variants explored** (exploration 0011) — Identified 8 source
  types in reference implementation, proposed simplifying to 4 kinds
  (index, path, kpar, git). Fundamental question parked: do
  --source-kind/--source belong on commands at all?
