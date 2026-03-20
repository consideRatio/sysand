# Sysand

A package manager for SysML v2 and KerML, similar to Cargo for Rust or npm
for JavaScript. Manages interchange projects defined by the KerML spec
(clause 10.3).

## Project State

This is a **rework** of an existing codebase. The reference implementation
lives in `reference/`. We are designing the reworked version from first
principles, guided by ADRs and explorations.

No implementation code exists yet — only design documents.

## Architecture

Five binding surfaces, one source of truth:

1. **Rust CLI** — thin clap adapter over the Rust library
2. **Rust library** — canonical implementation
3. **Java bindings** — via JNI/UniFFI
4. **JS/WASM bindings** — via wasm-bindgen
5. **Python bindings** — via PyO3/UniFFI

The CLI command tree maps structurally to all binding surfaces. If you know
the CLI, you can predict the Rust module path, Java method chain, JS namespace,
and Python module path.

## Key Design Decisions (ADRs)

Read `adr/` for the full decisions. Summary:

- **ADR-0001**: Discovery (walking up to find `.project.json`) is CLI-only.
  The library takes explicit paths via `ProjectContext` and `WorkspaceContext`.
  Config (`sysand.toml`) is project-level only, loaded automatically,
  overridable via `ConfigMode` (`--config auto|none|<PATH>`).

- **ADR-0002**: CLI follows noun-verb grammar:
  `sysand <namespace> [<resource>...] <verb> [OPERANDS] [OPTIONS]`.
  Command path segments map directly to namespaces in all surfaces.

- **ADR-0003**: Option names are stable across commands (`--project` always
  means project root). No `--no-*` flags — use positive enums
  (`--update manifest|lock|sync`, `--deps all|none`). Shared option groups
  become shared types (`ResolveOptions`).

- **ADR-0004**: Complete command tree with 6 namespaces: `project`, `usage`,
  `lock`, `env`, `workspace`, `resolve`. Every command has one return shape.

- **ADR-0005**: Projection rules for all surfaces. Context objects
  (`ProjectContext`) are explicit everywhere. Every operation returns a typed
  result object (no unwrapping). Errors use a shared `ErrorCode` enum.

## Terminology

- **Usage** (not "dependency") — the KerML/SysML term for a project's
  dependencies. The CLI namespace is `sysand usage add|remove|list`.
- **Interchange project** — the unit of packaging (`.project.json` +
  `.meta.json` + source files).
- **KPAR** — KerML Project Archive, a zip-based archive format.
- **Environment** — local `sysand_env/` directory where usages are installed.

## CLI Command Namespaces

```
sysand project ...    Local project lifecycle, sources, info, metadata, building
sysand usage ...      Usage management (add/remove/list)
sysand lock ...       Lockfile operations
sysand env ...        Environment creation, sync, install/uninstall
sysand workspace ...  Workspace operations
sysand resolve ...    Remote package queries (read-only)
```

See `adr/0004-command-tree.md` for the full tree.

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

Every operation returns a typed result object:

- `ScalarFieldResult<T>` — single value queries
- `ListFieldResult<T>` — list queries
- `MutationResult` — mutations
- `ResolveFieldResult<T>` — remote queries

No unwrapping in any surface.

## Directory Structure

```
adr/            Architectural Decision Records (numbered)
explorations/   Working exploration documents (numbered)
reference/      Reference implementation (existing codebase, read-only)
```

## Working Style

We use an explore-and-distill workflow:

1. Explore topics broadly, capture findings in `explorations/NNNN-*.md`
2. Iterate through discussion, resolving open questions
3. Distill into ADRs in `adr/NNNN-*.md` when decisions crystallize

Explorations are cheap working memory. ADRs are commitments.
