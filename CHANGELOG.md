# Changelog

Decisions made and their outcomes, in reverse chronological order.

## 2026-03-21

- **ADR-0008: Single-version resolve** — Resolve always returns one
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
  `ResolveFieldResult`). Each operation returns `Result<T, SysandError>`
  where `T` is the natural type. ADR-0005 update still pending.

- **Resolve design direction** (exploration 0010) — Single-version
  resolution, pre-release opt-in via constraint string only, no flags.
  Distilled into ADR-0008.

- **IRI variants explored** (exploration 0011) — Identified 8 source
  types in reference implementation, proposed simplifying to 4 kinds
  (index, path, kpar, git). Fundamental question parked: do
  --source-kind/--source belong on commands at all?
