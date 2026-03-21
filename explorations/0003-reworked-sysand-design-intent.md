# 0003: Design Intent from reworked-sysand Exploration

**Status: Distilled into ADRs 0001–0005**

## What Was Explored

A separate design workspace (`reworked-sysand`) produced a complete design package:
a CLI spec, canonical Rust API, Python/Java projections, 6 ADRs, a 79-row
cross-language mapping matrix, and 10 design rules. This document captures the
**ideas and intent** worth carrying forward, not the specific details.

## Core Insight

**If you understand the CLI, you can understand everything else.** The CLI command
tree is the most intuitive surface — it's what users interact with directly. The
Rust library, Python bindings, Java bindings, and JS/WASM bindings should all be
mechanically derivable from the CLI shape. Not generated from it literally, but
_predictable_ from it.

This means:

- Knowing one column of the mapping matrix mostly/fully implies the others
- There are clear, regular patterns — not a lookup table of special cases
- A developer who reads the CLI help can predict the Python function name,
  the Rust module path, and the Java method chain without documentation

## Design Principles Worth Preserving

### 1. One command = one operation = one return shape

No overloaded commands that return different types depending on flags. Each CLI
command maps to exactly one Rust function with one return type.

### 2. Command path encodes meaning

```
sysand <namespace> [<resource>...] <verb> [OPERANDS] [OPTIONS]
```

The path _is_ the API structure:

- CLI: `sysand project info name set "foo"`
- Rust: `sysand_api::project::info::name::set(...)`
- Python: `sysand.project.info.name.set(...)`
- Java: `client.project().info().name().set(...)`
- JS/WASM: follows the same structural pattern

### 3. Explicit over implicit

- Selectors (`--project`, `--env`, `--workspace`) have fixed meanings everywhere
- Side-effects are explicit (`--update manifest|lock|sync`) not negated (`--no-lock`)
- No boolean inversions

### 4. Projections are mechanical, not designed

Each binding surface follows its language's idioms but the _mapping rules_ are
regular:

- **Rust**: module path mirrors command path, snake_case
- **Python**: module path mirrors command path, snake_case, keyword-only options
- **Java**: fluent namespace chain, PascalCase types, options objects
- **JS/WASM**: TBD but same structural principle

The projection rules are few and consistent. If you need a per-command override,
that's a design smell.

## What the Matrix Should Look Like

The rework should produce a mapping matrix where every operation is one row:

| CLI command           | Rust operation       | Python                     | Java                         | JS/WASM                    | Return shape        |
| --------------------- | -------------------- | -------------------------- | ---------------------------- | -------------------------- | ------------------- |
| `project init <PATH>` | `project::init(...)` | `sysand.project.init(...)` | `client.project().init(...)` | `sysand.project.init(...)` | `ProjectInitResult` |
| ...                   | ...                  | ...                        | ...                          | ...                        | ...                 |

The point is not that we maintain this table manually — it's that the patterns
are so regular that the table is _implied by the CLI spec plus the projection rules_.
If the table ever needs a footnote, something is wrong.

## What Changed from the Reference Codebase

The reference codebase had these problems that the rework addresses:

- **`info` was overloaded** — local project info and remote resolution shared one
  command with different flags changing the return shape. Split into `project ...`
  (local) vs `resolve ...` (remote).
- **Negative flags** (`--no-lock`, `--no-sync`, `--no-deps`) — replaced with
  explicit mode enums (`--update`, `--dependency-mode`).
- **Path options were ambiguous** — `--path` meant different things in different
  commands. Each selector now has a fixed name and meaning.
- **Bindings diverged** — Python and Java APIs had ad-hoc shapes because the CLI
  was irregular. Regular CLI → regular projections.

## Binding Surfaces

Five targets, one source of truth:

1. **Rust CLI** — thin clap adapter over the Rust library
2. **Rust library** — canonical implementation, async where needed
3. **Python bindings** — via PyO3/UniFFI, module structure mirrors Rust
4. **Java bindings** — via JNI/UniFFI, fluent namespace pattern
5. **JS/WASM bindings** — via wasm-bindgen, structure TBD but same principle

## Open Questions

1. **Schema format**: Should the CLI spec + projection rules be captured in a
   machine-readable schema (for codegen), or is the regularity sufficient that
   hand-written code following the rules is simpler to maintain?

2. **Where does behavior live?** The schema/CLI describes _what_ operations exist
   and their inputs/outputs. The _implementation_ lives in Rust. Is there a middle
   layer (e.g., operation descriptions for documentation generation)?

3. **JS/WASM projection rules**: Not yet defined. Likely follows the Python
   pattern (module path, camelCase) but needs to account for async/Promise
   semantics and wasm-bindgen constraints.

4. **How much of the reference codebase's internals survive?** The trait
   architecture (ProjectRead, environments, resolvers) is an implementation
   concern below the operation layer. The rework may simplify or replace it
   entirely — the schema doesn't care.
