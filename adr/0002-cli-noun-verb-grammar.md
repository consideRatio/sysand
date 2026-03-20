# ADR-0002: CLI Noun-Verb Grammar

- **Status**: Accepted
- **Date**: 2026-03-19

## Context

Sysand's CLI needs to project mechanically to a Rust library, Java bindings,
JS/WASM bindings, and Python bindings. The current CLI has a flat command
structure where most commands live at the top level (`sysand add`, `sysand init`,
`sysand include`, `sysand sync`) with no namespace grouping. This makes the
mapping to library module paths ad hoc.

We need a grammar that:

- Encodes the operation in the command path (not in flags)
- Maps structurally to module/namespace paths in every binding surface
- Groups related operations under shared namespaces
- Is predictable enough that knowing the CLI implies knowing the library API

## Decision

### Grammar

```
sysand [GLOBAL_OPTIONS] <namespace> [<resource>...] <verb> [OPERANDS...] [OPTIONS...]
```

- `<namespace>` and `<resource>` tokens are **nouns** (`project`, `info`,
  `source`, `env`, `usage`)
- The **final token** in the command path is always the **verb** (`get`, `set`,
  `add`, `remove`, `list`, `show`, `clear`, `update`, `sync`, `create`, `init`)
- Required data is **positional operands**
- Optional behavior is **named options**

### Namespaces group related operations

```
sysand project ...       project lifecycle, metadata, and building
sysand usage ...         usage management (add/remove/list)
sysand lock ...          lockfile operations
sysand env ...           environment management
sysand workspace ...     workspace operations
sysand resolve ...       remote package queries (read-only)
```

### The command path is the API path

Every segment in the command path becomes a namespace level in bindings.
No renaming, no special cases:

| CLI                            | Rust                         | Java                                   | JS/WASM                          | Python                           |
| ------------------------------ | ---------------------------- | -------------------------------------- | -------------------------------- | -------------------------------- |
| `sysand project init`          | `project::init()`            | `client.project().init()`              | `sysand.project.init()`          | `sysand.project.init()`          |
| `sysand project info name set` | `project::info::name::set()` | `client.project().info().name().set()` | `sysand.project.info.name.set()` | `sysand.project.info.name.set()` |
| `sysand env install`           | `env::install()`             | `client.env().install()`               | `sysand.env.install()`           | `sysand.env.install()`           |
| `sysand usage add`             | `usage::add()`               | `client.usage().add()`                 | `sysand.usage.add()`             | `sysand.usage.add()`             |

If this structural mapping feels awkward for a command, the command is
designed wrong.

### Verbs are subcommands, not flags

The action is always expressed in the command path:

```
Good:  sysand project info name set "foo"
Bad:   sysand project info name --set "foo"

Good:  sysand project info maintainer add "Alice"
Bad:   sysand project info maintainer --add "Alice"

Good:  sysand project build
       sysand workspace build
Bad:   sysand build --workspace
```

Flags modify behavior. They do not encode the primary action.

### One command = one return shape

No command returns different types depending on which flags are passed.
If different inputs produce structurally different outputs, they are
different commands:

```
Good:  sysand project show      →  ProjectSnapshot
       sysand resolve show      →  ResolveMatchesResult

Bad:   sysand info --path .     →  ProjectSnapshot
       sysand info --iri x      →  Vec<ResolveMatch>
```

### Required data is positional, modifiers are options

```
sysand usage add <IRI> [<VERSION_REQ>] --update sync --project .
                 ^^^^^ ^^^^^^^^^^^^^^^ ^^^^^^^^^^^^^^^^^^^^^^^^^^^^
                 required  optional     modifiers (named options)
```

Keep positional args to 1–2 at most.

## Naming Rules

- Singular nouns for namespaces (`project`, not `projects`)
- Imperative verbs for actions (`add`, `remove`, `show`, `set`, `clear`)
- No aliases that hide operation meaning
- No negatives for main behavior switches (see future ADR on positive options)

## Design Checklist

Before adding a CLI command, verify:

1. What Rust function does this map to?
2. Does it have exactly one return shape?
3. Can the CLI impl just parse args and call the Rust function?
4. Can Java expose it as `client.namespace().verb(required, options)`?
5. Can JS expose it as `await sysand.namespace.verb(required, { options })`?
6. Can Python expose it as `module.verb(required, *, optional=default)`?
7. Are side effects explicit?
8. If a footnote is needed to explain the projection, something is wrong.

## Consequences

- Every CLI command has a predictable library API path in all binding surfaces
- Related operations are grouped by namespace, improving discoverability
- The CLI crate becomes a thin argument parser that delegates to the library
- New commands follow the grammar mechanically — no design decisions per command
- Deeper command paths (e.g., `project info maintainer add`) mean more typing
  for CLI users, but the consistency is worth the tradeoff
