# Projection Rules

Mechanical rules for projecting CLI commands to each binding surface.
The test: given any CLI command, a developer can write the Rust
function, Java call, JS call, and Python call without looking anything
up — just by applying these rules.

## Command Path → Namespaces

Every segment in the CLI command path becomes a namespace level.
No renaming, no special cases.

| CLI                         | Rust                     | Java                              | JS/WASM                       | Python                        |
| --------------------------- | ------------------------ | --------------------------------- | ----------------------------- | ----------------------------- |
| `sysand project init`       | `project::init()`        | `client.project().init()`         | `sysand.project.init()`       | `sysand.project.init()`       |
| `sysand project source add` | `project::source::add()` | `client.project().source().add()` | `sysand.project.source.add()` | `sysand.project.source.add()` |
| `sysand env install`        | `env::install()`         | `client.env().install()`          | `sysand.env.install()`        | `sysand.env.install()`        |
| `sysand project usage add`  | `project::usage::add()`  | `client.project().usage().add()`  | `sysand.project.usage.add()`  | `sysand.project.usage.add()`  |

If this structural mapping feels awkward for a command, the command is
designed wrong.

## Context Objects

`ProjectContext` and `WorkspaceContext` are explicit types in every
surface, passed as the first argument.

| Surface | Project               | Workspace               |
| ------- | --------------------- | ----------------------- |
| Rust    | `&ProjectContext`     | `&WorkspaceContext`     |
| Java    | `ProjectContext ctx`  | `WorkspaceContext ctx`  |
| JS/WASM | `ctx: ProjectContext` | `ctx: WorkspaceContext` |
| Python  | `ctx: ProjectContext` | `ctx: WorkspaceContext` |

## Arguments → Parameters

| CLI element                       | Rust                    | Java                     | JS/WASM                          | Python                          |
| --------------------------------- | ----------------------- | ------------------------ | -------------------------------- | ------------------------------- |
| Required positional `<IRI>`       | `iri: &str`             | `String iri`             | `iri: string`                    | `iri: str`                      |
| Optional positional `[<VERSION>]` | `Option<&str>`          | `@Nullable String`       | `version?: string`               | `version: str \| None = None`   |
| Named option `--project <PATH>`   | field in options struct | field in options builder | field in options object          | keyword-only arg                |
| Repeated option `--index <URL>`   | `Vec<Url>`              | `List<String>`           | `string[]`                       | `Sequence[str]`                 |
| Enum option `--update <MODE>`     | `UpdateMode` enum       | `UpdateMode` enum        | `"manifest" \| "lock" \| "sync"` | `UpdateMode` enum               |
| Boolean flag `--allow-overwrite`  | `bool` field            | `.allowOverwrite(true)`  | `allowOverwrite?: boolean`       | `allow_overwrite: bool = False` |

## Options Grouping

Related CLI options that appear together across multiple commands become
a shared type:

| Surface | Shape                           |
| ------- | ------------------------------- |
| Rust    | `IndexOptions` struct           |
| Java    | `IndexOptions` builder          |
| JS/WASM | `IndexOptions` object/interface |
| Python  | `IndexOptions` dataclass        |

Paired flags (`--source-kind` + `--source`) become a single `SourceSpec`
type.

## Natural Return Types

Every operation returns `Result<T, SysandError>` where `T` is the
natural type for that operation:

- `()` for mutations with no meaningful output
- A primitive for simple queries (`String`, `bool`)
- A `Vec` for list queries
- A domain struct for structured output (`BuildOutput`)

No universal wrapper types.

## Errors

| Surface | Shape                                           |
| ------- | ----------------------------------------------- |
| Rust    | `Result<T, SysandError>` with `ErrorCode` enum  |
| Java    | throws `SysandException` with `ErrorCode` field |
| JS/WASM | throws `SysandError` with `code` property       |
| Python  | raises `SysandError` subclass per code          |

Error codes are the same enum everywhere. No per-command exception
classes.

## Naming Conventions

| Concept            | Rust         | Java                    | JS/WASM                     | Python                 |
| ------------------ | ------------ | ----------------------- | --------------------------- | ---------------------- |
| Modules/namespaces | `snake_case` | `camelCase()` accessors | `camelCase`                 | `snake_case`           |
| Functions/methods  | `snake_case` | `camelCase`             | `camelCase`                 | `snake_case`           |
| Types              | `PascalCase` | `PascalCase`            | `PascalCase`                | `PascalCase`           |
| Enum variants      | `PascalCase` | `UPPER_SNAKE`           | `"kebab-case"` string union | `UPPER_SNAKE`          |
| Options struct     | `XxxOptions` | `XxxOptions` builder    | `XxxOptions` interface      | `XxxOptions` dataclass |

## Async

| Surface | Rule                                                                                  |
| ------- | ------------------------------------------------------------------------------------- |
| Rust    | async for network ops, sync for local. `blocking` module wraps async for sync callers |
| Java    | sync by default; binding layer manages async runtime internally                       |
| JS/WASM | everything returns `Promise`                                                          |
| Python  | sync by default; binding layer manages async runtime internally                       |

## JS/WASM Details

### Plain objects everywhere

All types crossing the WASM boundary are plain JavaScript objects, not
classes. This includes context objects, options structs, and return
types. Conversion uses `serde-wasm-bindgen`: inputs are deserialized
from `JsValue`, outputs are serialized to `JsValue`.

No wasm-bindgen `#[wasm_bindgen]` classes are exposed to users. This
avoids the memory management burden — wasm-bindgen classes hold Rust
memory and must be `.free()`d manually, which is error-prone for
short-lived values like return types and options.

TypeScript `.d.ts` definitions provide compile-time type safety.

### Type mapping

| Rust type        | JS/WASM type                    |
| ---------------- | ------------------------------- |
| `String`         | `string`                        |
| `Option<String>` | `string \| undefined`           |
| `Vec<String>`    | `string[]`                      |
| `bool`           | `boolean`                       |
| `()`             | `void` (Promise resolves)       |
| Options struct   | plain object (`XxxOptions`)     |
| Domain struct    | plain object                    |
| Enum             | `"kebab-case"` string union     |
| `SysandError`    | thrown object with `code`, `message`, `context?` |

### Constraints

- **No overloading.** wasm-bindgen cannot export two functions with
  the same JS name. Each facade function maps to one export.
- **No generics across FFI.** Already handled by the facade — the
  binding calls a concrete facade function, not a generic one.
- **All functions return `Promise`.** The binding layer runs the Rust
  async runtime internally. From the JS caller's perspective, every
  call is `await sysand.project.build(ctx, opts)`.

## Rationale

**Why mechanical projection.** If projecting a CLI command to binding
surfaces requires design judgment, the command is designed wrong. The
mechanical test — "can a developer write the Rust, Java, JS, and Python
call without looking anything up?" — ensures the API is consistent and
that bindings can be generated rather than handcrafted per surface.

**Why natural return types instead of wrappers.** An earlier design
used a four-wrapper taxonomy (`ScalarFieldResult`, `ListFieldResult`,
`MutationResult`, `LookupFieldResult`) for forward compatibility.
These wrappers added ceremony without earning their keep: every call
site had to unwrap, generics complicated binding projections (especially
JNI), and the "future extensibility" benefit was speculative.
Replacing wrappers with natural types (`String`, `Vec<T>`, `()`,
domain structs) makes return types project trivially to all surfaces
and eliminates unnecessary unwrapping. If a return type needs to grow
pre-1.0, a targeted breaking change is cheaper than permanent wrapper
overhead.
