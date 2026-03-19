# 0004: CLI Design Rules and Projection to Library & Bindings

## Purpose

Rules for designing CLI commands such that a Rust library API, Python bindings,
Java bindings, and JS/WASM bindings can all be derived by following mechanical
projection rules. No machine-readable schema — just discipline.

The contract: **if you understand the CLI, you can predict every other surface.**

---

## Part 1: CLI Design Rules

These rules constrain how CLI commands are shaped so that projections work.

### Grammar

```
sysand [GLOBAL_OPTIONS] <namespace> [<resource>...] <verb> [OPERANDS...] [OPTIONS...]
```

- Nouns for namespaces and resources (`project`, `info`, `source`, `env`)
- The final token is always the verb (`get`, `set`, `add`, `remove`, `list`, `show`, `clear`, `update`, `sync`, `create`)
- Required data is positional operands
- Optional behavior is named options

### Rule 1: Command path encodes the operation

The action lives in the command path, not in flags.

```
Good:  sysand env install <IRI>
Bad:   sysand env <IRI> --install
```

This matters because every command path segment becomes a namespace level in
bindings. Flags-as-verbs break that mapping.

### Rule 2: One command = one return shape

Never overload a command so that different flags produce different return types.
If the CLI can operate on different resource kinds, split it into separate commands.

```
Good:  sysand project show      →  ProjectSnapshot
       sysand resolve show      →  ResolveMatchesResult
Bad:   sysand info --path .     →  ProjectSnapshot
       sysand info --iri x      →  Vec<ResolveMatch>
```

### Rule 3: Stable option names across all commands

The same option name means the same thing everywhere:

- `--project <PATH>` — always the project root
- `--env <PATH>` — always the environment directory
- `--workspace <PATH>` — always the workspace root
- `--target <PATH>` — always the output target
- `--index <URL>` — always an additional package index (repeatable)

Never repurpose `--path` to mean different things in different commands.

### Rule 4: Verbs as subcommands, not mode flags

When two modes do substantially different work, use two commands.

```
Good:  sysand build project
       sysand build workspace
Bad:   sysand build --workspace
```

### Rule 5: Positive enums over negative booleans

Replace `--no-lock`, `--no-sync` with an explicit mode enum.

```
Good:  sysand usage add <IRI> --update sync
       sysand usage add <IRI> --update manifest
Bad:   sysand usage add <IRI> --no-lock --no-sync
```

Similarly: `--deps all|none` instead of `--no-deps`.

### Rule 6: Required data as positional args, modifiers as options

```
sysand usage add <IRI> [<VERSION_REQ>] --update sync --project .
                 ^^^^^  ^^^^^^^^^^^^^  ^^^^^^^^^^^^^^^^^^^^^^^^
                 required  optional     modifiers (named options)
```

Keep positional args to 1-2 at most. If it needs more, the design is probably wrong.

### Rule 7: Explicit side effects

If a command mutates one thing and triggers other changes, make that explicit.

```
Good:  sysand usage add <IRI> --update sync   (explicitly: update manifest + lock + env)
       sysand usage add <IRI> --update lock    (explicitly: update manifest + lock only)
Bad:   sysand add <IRI>                        (implicitly: does it lock? sync? who knows?)
```

### Rule 8: Selectors over ambient discovery

Commands should accept explicit paths for everything. Discovery (cwd, auto-config)
is a CLI convenience layer on top, not the primary behavior.

```
Good:  sysand project show --project /path/to/project
Also OK: sysand project show   (defaults to ".", but explicit form exists)
Bad:   only works from the right directory with no override
```

### Rule 9: Structured output first

Design the operation around its structured result. The CLI decides how to _render_
that result (text, JSON), but the result contract is stable and typed.

### Rule 10: Design checklist

Before adding a CLI command, verify:

1. What Rust function does this map to?
2. Does it have exactly one return shape?
3. Can the CLI impl just parse args and call the Rust function?
4. Can Python expose it as `module.verb(required, *, optional=default)`?
5. Can Java expose it as `client.namespace().verb(required, options)`?
6. Can JS expose it as `await sysand.namespace.verb(required, { options })`?
7. Are side effects explicit?
8. If a footnote is needed to explain the projection, something is wrong.

---

## Part 2: Projection Rules

Given a well-designed CLI command, here's how each surface is derived.

### Command path → namespaces

| CLI                            | Rust                         | Python                           | Java                                   | JS/WASM                          |
| ------------------------------ | ---------------------------- | -------------------------------- | -------------------------------------- | -------------------------------- |
| `sysand project init`          | `project::init()`            | `sysand.project.init()`          | `client.project().init()`              | `sysand.project.init()`          |
| `sysand project info name set` | `project::info::name::set()` | `sysand.project.info.name.set()` | `client.project().info().name().set()` | `sysand.project.info.name.set()` |
| `sysand env install`           | `env::install()`             | `sysand.env.install()`           | `client.env().install()`               | `sysand.env.install()`           |

The mapping is purely structural. No renaming, no special cases.

### Arguments → parameters

| CLI element                       | Rust                      | Python                                       | Java                                      | JS/WASM                          |
| --------------------------------- | ------------------------- | -------------------------------------------- | ----------------------------------------- | -------------------------------- |
| Required positional `<IRI>`       | regular param `iri: &str` | positional param `iri: str`                  | regular param `String iri`                | regular param `iri: string`      |
| Optional positional `[<VERSION>]` | `Option<&str>`            | positional `version: str \| None = None`     | nullable param `@Nullable String version` | `version?: string`               |
| Named option `--project <PATH>`   | field in input struct     | keyword-only `project: Path = "."`           | field in options builder                  | field in options object          |
| Repeated option `--index <URL>`   | `Vec<Url>`                | `Sequence[str]`                              | `List<String>`                            | `string[]`                       |
| Enum option `--update <MODE>`     | `UpdateMode` enum         | `UpdateMode` enum                            | `UpdateMode` enum                         | `"manifest" \| "lock" \| "sync"` |
| Boolean flag `--allow-overwrite`  | `bool` field              | keyword-only `allow_overwrite: bool = False` | builder `.setAllowOverwrite(true)`        | `allowOverwrite?: boolean`       |

### Options grouping

When multiple commands share the same set of options (e.g., resolve options:
`--index`, `--default-index`, `--index-mode`, `--include-std`), they become a
shared type in every surface:

| Surface | Shape                                                       |
| ------- | ----------------------------------------------------------- |
| Rust    | `ResolveOptions` struct                                     |
| Python  | `ResolveOptions` dataclass, passed as keyword arg           |
| Java    | `ResolveOptions` builder, composed into per-command options |
| JS/WASM | `ResolveOptions` object literal or interface                |

### Return types

| CLI result kind | Rust                    | Python                  | Java                    | JS/WASM                 |
| --------------- | ----------------------- | ----------------------- | ----------------------- | ----------------------- |
| Mutation        | `MutationResult`        | `MutationResult`        | `MutationResult`        | `MutationResult`        |
| Scalar query    | `ScalarFieldResult<T>`  | return `T` directly     | `ScalarFieldResult<T>`  | `T` directly            |
| List query      | `ListFieldResult<T>`    | return `list[T]`        | `ListFieldResult<T>`    | `T[]`                   |
| Resolve query   | `ResolveFieldResult<T>` | `ResolveFieldResult[T]` | `ResolveFieldResult<T>` | `ResolveFieldResult<T>` |

Python and JS may unwrap thin wrappers for ergonomics. Rust and Java keep them
for type clarity. This is the one place where surfaces may differ slightly —
but the _operation_ is the same.

### Errors

| Surface | Shape                                                                             |
| ------- | --------------------------------------------------------------------------------- |
| Rust    | `Result<T, SysandError>` with `ErrorCode` enum                                    |
| Python  | raise `SysandError` subclass per code (`NotFoundError`, `InvalidInputError`, ...) |
| Java    | throw `SysandException` with `ErrorCode` field                                    |
| JS/WASM | throw `SysandError` with `code` property                                          |

Error codes are the same enum everywhere. No per-command exception classes.

### Naming conventions

| Concept            | Rust                      | Python                                | Java                           | JS/WASM                     |
| ------------------ | ------------------------- | ------------------------------------- | ------------------------------ | --------------------------- |
| Modules/namespaces | `snake_case`              | `snake_case`                          | `camelCase()` accessor methods | `camelCase`                 |
| Functions/methods  | `snake_case`              | `snake_case`                          | `camelCase`                    | `camelCase`                 |
| Types              | `PascalCase`              | `PascalCase`                          | `PascalCase`                   | `PascalCase`                |
| Enum variants      | `PascalCase`              | `UPPER_SNAKE`                         | `UPPER_SNAKE`                  | `"kebab-case"` string union |
| Options struct     | `XxxInput` / `XxxOptions` | `XxxOptions` dataclass / keyword args | `XxxOptions` builder           | `XxxOptions` interface      |

### Async

| Surface | Rule                                                                                  |
| ------- | ------------------------------------------------------------------------------------- |
| Rust    | async for network ops, sync for local. `blocking` module wraps async for sync callers |
| Python  | sync by default (binding layer handles runtime). Async variants optional              |
| Java    | sync by default (binding layer handles runtime)                                       |
| JS/WASM | everything returns `Promise`                                                          |

---

## The Test

The projection rules are correct if this holds:

> Given any CLI command, a developer can write the Rust function signature,
> the Python call, the Java call, and the JS call **without looking anything up**
> — just by applying the rules above.

If a command needs a special-case mapping note, either the CLI design or the
projection rules need to change.
