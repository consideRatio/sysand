# Claude Code Guide

See `README.md` for project overview, architecture, and design decisions.

## Terminology

- **Usage** (not "dependency") — the KerML/SysML term for project dependencies
- **Interchange project** — the unit of packaging
- **KPAR** — KerML Project Archive

## Conventions

- Binding surface order in docs/examples: Rust, Java, JS/WASM, Python
- Argument order: context first, required args, then options
- Every operation returns `Result<T, SysandError>` where `T` is the natural type
- No `--no-*` flags — use positive enums
- Option names are stable across commands (`--project` always means project root)
- Semver is required for all project versions (ADR-0007)

## Directory Structure

```
spec/           Living specification — the current state of all decisions
adr/            Architectural Decision Records (numbered, immutable)
explorations/   Working exploration documents (numbered, immutable once done)
reference/      Reference implementation (existing codebase, read-only)
TODO.md         Open work items only
CHANGELOG.md    Decisions made and their dates
```

## Working Style

We use an explore → decide → specify workflow:

1. **Explore** — investigate topics broadly in `explorations/NNNN-*.md`
2. **Decide** — when a decision crystallizes, record it in `adr/NNNN-*.md`
3. **Specify** — update the relevant `spec/` file(s) to reflect the
   cumulative current state

Ask clarifying questions rather than assuming. Deliberate on naming.
One question at a time, not a list of 10.

### Three layers, three purposes

**`spec/`** is the living specification. It reflects the current state
of all cumulative decisions. When you want to know "what is the command
tree right now?", you read `spec/command-tree.md`. When a new decision
is made, the relevant spec file is updated. Spec files are the single
source of truth for the current design.

**`adr/`** records are immutable. Each ADR captures a decision, its
context, and its reasoning at a specific point in time. ADRs are never
edited after acceptance — they are historical artifacts. If a later
decision supersedes an earlier one, the later ADR notes what it
supersedes, but the earlier ADR is not modified. The spec files reflect
the cumulative effect.

**`explorations/`** are immutable once they reach a terminal status.
They capture working-out and analysis that led to decisions. They may
contain ideas that were rejected — that's valuable context.

### Exploration lifecycle

Every exploration has a status line at the top (after the title):

- `Status: Open` — actively being worked on
- `Status: Parked` — has unresolved questions, not actively progressing
- `Status: Distilled into ADR-NNNN` — decisions captured, exploration is historical
- `Status: Superseded by NNNN-*` — replaced by a later exploration

### Scenarios first

When exploring a design, write concrete end-to-end usage scenarios
across all binding surfaces (Rust, Java, JS/WASM, Python) _before_
proposing abstractions. Scenarios catch problems that abstract
reasoning misses.

### TODO and CHANGELOG

`TODO.md` contains only open work items. When an item is resolved,
move it to `CHANGELOG.md` with the date and outcome. This keeps the
active work list focused.
