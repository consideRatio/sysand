# Claude Code Guide

See `README.md` for project overview, architecture, and design decisions.

## Terminology

- **Usage** (not "dependency") — the KerML/SysML term for project dependencies
- **Interchange project** — the unit of packaging
- **KPAR** — KerML Project Archive

## Conventions

- Binding surface order in docs/examples: Rust, Java, JS/WASM, Python
- Argument order: context first, required args, then options
- Every operation returns a typed result object — no unwrapping
- No `--no-*` flags — use positive enums
- Option names are stable across commands (`--project` always means project root)

## Directory Structure

```
adr/            Architectural Decision Records (numbered, commitments)
explorations/   Working exploration documents (numbered, cheap)
reference/      Reference implementation (existing codebase, read-only)
```

## Working Style

We use an explore-and-distill workflow (see `/explore-and-distill` skill):

1. Explore topics broadly, capture in `explorations/NNNN-*.md`
2. Iterate, resolve open questions one at a time
3. Distill into `adr/NNNN-*.md` when decisions crystallize

Ask clarifying questions rather than assuming. Deliberate on naming.
One question at a time, not a list of 10.
