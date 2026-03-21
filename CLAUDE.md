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
adr/            Architectural Decision Records (numbered, commitments)
explorations/   Working exploration documents (numbered, cheap)
reference/      Reference implementation (existing codebase, read-only)
TODO.md         Open work items only
CHANGELOG.md    Decisions made and their dates
```

## Working Style

We use an explore-and-distill workflow:

1. Explore topics broadly, capture in `explorations/NNNN-*.md`
2. Iterate, resolve open questions one at a time
3. Distill into `adr/NNNN-*.md` when decisions crystallize

Ask clarifying questions rather than assuming. Deliberate on naming.
One question at a time, not a list of 10.

### Exploration lifecycle

Every exploration has a status line at the top (after the title):

- `Status: Open` — actively being worked on
- `Status: Parked` — has unresolved questions, not actively progressing
- `Status: Distilled into ADR-NNNN` — decisions captured, exploration is historical
- `Status: Superseded by NNNN-*` — replaced by a later exploration

### Scenarios first

When exploring a design, write concrete end-to-end usage scenarios
across all binding surfaces (Rust, Java, JS/WASM, Python) *before*
proposing abstractions. Scenarios catch problems that abstract
reasoning misses.

### ADR amendments

When an ADR is amended by a later ADR, add a dated entry to the
amendment log at the bottom of the amended ADR. This makes evolution
traceable without digging through git history.

### TODO and CHANGELOG

`TODO.md` contains only open work items. When an item is resolved,
move it to `CHANGELOG.md` with the date and outcome. This keeps the
active work list focused.
