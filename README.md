# Sysand

A package manager for SysML v2 and KerML, similar to Cargo for Rust or npm
for JavaScript. Manages interchange projects defined by the KerML spec
(clause 10.3).

## Project State

This is a **rework** of an existing codebase. The reference implementation
lives in `reference/`. We are designing the reworked version from first
principles.

No implementation code exists yet — only design documents.

## Architecture

Five binding surfaces, one source of truth:

1. **Rust CLI** — thin clap adapter over the Rust library
2. **Rust library** — canonical implementation
3. **Java bindings** — via JNI
4. **JS/WASM bindings** — via wasm-bindgen
5. **Python bindings** — via PyO3

The CLI command tree maps structurally to all binding surfaces. If you know
the CLI, you can predict the Rust module path, Java method chain, JS namespace,
and Python module path.

## Key Design Decisions

Each `spec/` file includes a Rationale section explaining why the design
is what it is — rejected alternatives, tradeoffs, and constraints. Key
decisions:

- Explicit paths via `ProjectContext` / `WorkspaceContext`; config is
  project-level only; implicit CWD discovery is CLI-only
- Noun-verb CLI grammar where command path segments map directly to
  namespaces in all binding surfaces
- Stable option names, no `--no-*` flags, semver required
- Minimal API: field-level accessors and index queries removed; users
  edit manifests directly, index queries are internal to the solver
- Facade pattern in Rust core, generic over storage; binding command
  wrappers are AI-generated from the facade using projection rules
- PubGrub solver is internal; three-stage pipeline (lock, lockfile,
  env sync)

## Terminology

- **Usage** (not "dependency") — the KerML/SysML term for a project's
  dependencies. The CLI commands are `sysand project usage add|remove`.
- **Interchange project** (not "package") — the unit of packaging
  (`.project.json` + `.meta.json` + source files).
- **IRI** — unique identity of a project (e.g., `urn:kpar:sensors`). A
  project can have multiple IRIs. Name is a human-readable label, not
  unique, not used for resolution.
- **KPAR** — KerML Project Archive, a zip-based archive format.
- **Index** (not "registry") — a package index queried by IRI during
  dependency resolution.
- **Environment** — local `sysand_env/` directory where usages are installed.

## CLI Command Namespaces

```
sysand project ...    Local project lifecycle, sources, usages, building
sysand lock ...       Lockfile operations
sysand env ...        Environment creation, sync, install/uninstall
sysand workspace ...  Workspace operations
```

See `spec/public-api.md` for the full API across all surfaces.

## Conventions

### Argument order in all surfaces

1. Context object (`&ProjectContext` / `&WorkspaceContext`) — first
2. Required positional args (IRI, name, path, etc.)
3. Options struct or keyword args

### Naming

- Rust: `snake_case` modules and functions, `PascalCase` types
- Java: `camelCase` methods, `PascalCase` types, `UPPER_SNAKE` enum variants
- JS/WASM: `camelCase` namespaces and functions, `PascalCase` types
- Python: `snake_case` modules and functions, `PascalCase` types, `UPPER_SNAKE` enum variants

### Return types

Every operation returns `Result<T, SysandError>` where `T` is the natural
type for that operation — `()` for mutations, `String` for a name,
`Vec<UsageEntry>` for a list, `BuildOutput` for a build. No universal
wrapper types.

## Directory Structure

```
spec/           Living specification with rationale — single source of truth
reference/      Reference implementation (existing codebase, read-only)
TODO.md         Open work items
CHANGELOG.md    Completed decisions and their dates
```
