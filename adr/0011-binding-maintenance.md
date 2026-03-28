# ADR-0011: Binding Maintenance Strategy

- **Status**: Accepted
- **Date**: 2026-03-28
- **Applies**: ADR-0010 (binding strategy), ADR-0005 (projection rules)

## Context

ADR-0010 established a facade in the Rust core (generic over storage)
and kept existing binding tools (JNI, PyO3, wasm-bindgen). With the
facade, every binding surface follows the same pattern per command:
construct the storage backend, call the facade, convert errors. This
is ~5–10 lines per command per surface.

This mechanical nature raises the question: how do we maintain
bindings with near-zero manual effort?

Two approaches were considered:

1. **Code generation** — a proc macro or build script reads the
   facade and emits binding code. Guarantees correctness by
   construction but requires building and maintaining a tool.
   Conflicts with the project's "predictable, not generated"
   philosophy (ADR-0005).

2. **AI-assisted generation** — the projection rules (ADR-0005),
   error model, and facade signatures are sufficient for Claude to
   produce correct binding code. No tooling to build. The rules
   live in CLAUDE.md so they're always in context.

## Decision

### 1. All binding surfaces are AI-generated from the facade

When a facade function is added or changed, Claude updates the
Java, Python, and JS/WASM command wrappers by applying the
projection rules mechanically. The binding patterns are documented
in CLAUDE.md with concrete examples for each surface.

All three surfaces follow the same pattern: construct storage,
call facade, convert errors. JS/WASM is no longer a special case
for command wrappers — only the storage construction line differs.

No build-time code generation tooling is introduced.

### 2. CLAUDE.md contains the binding generation rules

The rules cover:

- **Naming**: ADR-0005's casing table (snake_case → camelCase for
  Java/JS, etc.)
- **Argument conversion**: a fixed mapping table per surface
  (e.g., `String` → `JString` extraction for Java, direct for PyO3)
- **Error mapping**: `SysandError` → `SysandException` (Java),
  `PyResult` via `into_py_err` (Python), `JsValue` (JS/WASM)
- **Project construction**: `LocalSrcProject::open(path)` for
  Java/Python, `ProjectLocalBrowserStorage::open(prefix, root_path)`
  for JS/WASM
- **Module structure**: facade module path → binding namespace
  (e.g., `project::init` → `sysand.project.init()` for Python,
  `com.sensmetry.sysand.Project.init()` for Java)
- **Concrete examples**: at least one complete command showing the
  pattern for each surface

### 3. Storage backends require manual engineering

The JS/WASM storage backend (`ProjectLocalBrowserStorage`,
`LocalBrowserStorageEnvironment`, `LocalStorageVFS` — ~390 lines)
implements core traits on browser `localStorage`. This is
irreducible platform-specific code.

Changes to core traits (`ProjectRead`, `ProjectMut`,
`ReadEnvironment`, `WriteEnvironment`) may require corresponding
changes in the storage backend. These changes require understanding
the browser storage model and cannot be reduced to a projection
rule.

CLAUDE.md documents this distinction: command wrappers are
mechanical (all surfaces), storage backends are not (JS/WASM only).

## Consequences

1. **Near-zero binding effort for all surfaces.** Adding a new
   command means writing the facade function; the bindings across
   all three surfaces are generated in the same change by Claude.

2. **No codegen tooling.** No proc macros, build scripts, or
   external tools to maintain. The "tool" is the projection rules
   in CLAUDE.md applied by Claude.

3. **JS/WASM command wrappers are now mechanical too.** Only the
   storage backend (~390 lines) requires manual engineering. Command
   wrappers follow the same pattern as Java and Python.

4. **Binding correctness depends on review.** Unlike codegen, AI-
   generated bindings can have errors. The projection rules are
   simple enough that review is quick — the patterns are highly
   repetitive and deviations are obvious.

5. **Rules must stay current.** If the binding patterns evolve
   (e.g., new error mapping approach, new argument types),
   CLAUDE.md must be updated or Claude will generate stale code.
