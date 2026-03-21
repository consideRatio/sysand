# ADR-0009: Minimal Public API

- **Status**: Accepted
- **Date**: 2026-03-21
- **Supersedes**: ADR-0008 (lookup removed entirely)
- **Updates**: ADR-0004 (command tree: `project info`, `project metadata`, `project show`, `lookup` namespace removed; `usage` moved under `project`; `project usage list`, `project source list` removed; `--include-std` separated from `IndexOptions`)

## Context

The command tree in ADR-0004 includes field-level accessors (`project
info name get/set`, `project metadata metamodel get/set`, etc.) and
index query commands (`lookup show`, `lookup info`, `lookup metadata`).
These map to all binding surfaces via the projection rules (ADR-0005),
meaning every field accessor and lookup command becomes a public
function in Rust, Java, JS/WASM, and Python.

Surveying other package managers: `cargo metadata`, `npm view`, `pip
show` exist but are lightly used — humans read manifests directly,
and tooling that needs structured access tends to parse the manifest
file. Index query CLIs (`cargo search`, `npm search`) see even less
use; people use web UIs.

The field-level accessor commands alone account for ~50 commands
across info and metadata. Each one must be implemented, projected to
four binding surfaces, tested, and documented. The value doesn't
justify the cost at this stage.

## Decision

### 1. Remove field-level accessors

The `project info` subtree (name, publisher, description, version,
license, website, maintainer, topic — each with get/set/clear/add/
remove verbs) and `project metadata` subtree (created, index,
checksum, metamodel, includes-derived, includes-implied) are removed
from the public API.

Users read and edit `.project.json` and `.meta.json` directly. The
files are the interface for field-level operations.

`project show` is also removed — it was a read-only dump of project
info that adds no value over reading `.project.json`.

### 2. Remove lookup namespace

The `lookup` namespace (show, info, metadata subcommands) is removed.
Index queries remain internal to the dependency solver (`pub(crate)`).
The solver continues to query indexes for `lock update`, `env sync`,
`env install`, and `project usage add --update lock|sync`.

ADR-0008's version constraint rules (latest stable, range, pre-release
opt-in, exact) still apply — they govern internal resolution. The
rules are specified in `spec/version-resolution.md`.

### 3. Move usage under project

The top-level `usage` namespace moves to `project usage`. Usages are
defined in `.project.json`, the same file that `project source`
operates on. Having `usage` at the top level while `source` is nested
under `project` is inconsistent.

### 4. Remove thin list/display commands

`project usage list` is removed — usages are in `.project.json`,
same argument as field-level accessors. `project source list` is
removed — listing source files across a project and its dependencies
is useful but not essential; users can read `.project.json` and the
environment directory directly.

`env list` is kept. It shows what is actually installed in
`sysand_env/`, which can diverge from the lockfile after partial
syncs, manual edits, or failures. This is analogous to `pip list` —
lightweight but useful for quick inspection.

### 5. Rename LookupOptions to IndexOptions

The shared options group (`--index`, `--default-index`, `--index-mode`)
is renamed from `LookupOptions` to `IndexOptions`, since the `lookup`
namespace no longer exists. These options still appear on commands
that query indexes: `project usage add`, `lock update`, `env sync`,
`env install`, `project clone`.

### 6. Separate --include-std from IndexOptions

`--include-std` controls whether the standard library is included as
an implicit usage. This applies regardless of source type — a direct
git fetch still needs to know whether to pull in the standard library
for transitive resolution. It is not an index-specific option and
appears as a standalone flag, not part of `IndexOptions`.

## Consequences

- The public API surface drops significantly
- `.project.json` and `.meta.json` become the primary interface for
  field-level reads and writes — no CLI or library abstraction layer
- Index queries are purely internal; if user-facing lookup is needed
  later, it can be added without breaking changes
- `project` namespace groups all project-scoped resources: `init`,
  `locate`, `clone`, `source`, `usage`, `build`
- `IndexOptions` type name reflects its purpose (configuring index
  access) rather than a removed command namespace
- `--include-std` is orthogonal to index configuration and can appear
  on commands that don't use indexes (e.g. `project source list`)
