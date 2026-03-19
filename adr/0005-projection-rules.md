# ADR-0005: Projection Rules

- **Status**: Accepted
- **Date**: 2026-03-19
- **Applies**: ADR-0002 (noun-verb grammar), ADR-0004 (command tree)

## Context

The CLI is designed so that knowing the CLI implies knowing the library API
in every binding surface (Rust, Java, JS/WASM, Python). This ADR defines the
mechanical rules for projecting CLI commands to each surface.

The test: given any CLI command, a developer can write the Rust function
signature, the Java call, the JS call, and the Python call without looking
anything up — just by applying these rules.

If a command needs a special-case mapping note, either the CLI design or
these rules need to change.

## Decision

### 1. Command path → namespaces

Every segment in the CLI command path becomes a namespace level. No renaming.

| CLI                            | Rust                         | Java                                   | JS/WASM                          | Python                           |
| ------------------------------ | ---------------------------- | -------------------------------------- | -------------------------------- | -------------------------------- |
| `sysand project init`          | `project::init()`            | `client.project().init()`              | `sysand.project.init()`          | `sysand.project.init()`          |
| `sysand project info name set` | `project::info::name::set()` | `client.project().info().name().set()` | `sysand.project.info.name.set()` | `sysand.project.info.name.set()` |
| `sysand usage add`             | `usage::add()`               | `client.usage().add()`                 | `sysand.usage.add()`             | `sysand.usage.add()`             |

### 2. Context objects are explicit in all surfaces

`ProjectContext` and `WorkspaceContext` (ADR-0001) are explicit types in every
surface, passed as the first argument. They are not flattened into keyword args
or options objects.

**Rust:**

```rust
let ctx = ProjectContext::new(".");
project::info::name::get(&ctx)?;
```

**Java:**

```java
ProjectContext ctx = new ProjectContext(".");
client.project().info().name().get(ctx);
```

**JS/WASM:**

```ts
const ctx = new ProjectContext(".");
await sysand.project.info.name.get(ctx);
```

**Python:**

```python
ctx = ProjectContext(".")
sysand.project.info.name.get(ctx)
```

This keeps projection mechanical and makes context reusable across multiple
operations.

### 3. Arguments → parameters

| CLI element                       | Rust                      | Java                            | JS/WASM                          | Python                                       |
| --------------------------------- | ------------------------- | ------------------------------- | -------------------------------- | -------------------------------------------- |
| Required positional `<IRI>`       | regular param `iri: &str` | regular param `String iri`      | regular param `iri: string`      | positional param `iri: str`                  |
| Optional positional `[<VERSION>]` | `Option<&str>`            | `@Nullable String version`      | `version?: string`               | positional `version: str \| None = None`     |
| Named option `--project <PATH>`   | field in options struct   | field in options builder        | field in options object          | keyword-only arg                             |
| Repeated option `--index <URL>`   | `Vec<Url>`                | `List<String>`                  | `string[]`                       | `Sequence[str]`                              |
| Enum option `--update <MODE>`     | `UpdateMode` enum         | `UpdateMode` enum               | `"manifest" \| "lock" \| "sync"` | `UpdateMode` enum                            |
| Boolean flag `--allow-overwrite`  | `bool` field              | builder `.allowOverwrite(true)` | `allowOverwrite?: boolean`       | keyword-only `allow_overwrite: bool = False` |

### 4. Options grouping

Related CLI options that appear together across multiple commands become a
shared type in every surface:

| Surface | Shape                             |
| ------- | --------------------------------- |
| Rust    | `ResolveOptions` struct           |
| Java    | `ResolveOptions` builder          |
| JS/WASM | `ResolveOptions` object/interface |
| Python  | `ResolveOptions` dataclass        |

Paired flags (`--source-kind` + `--source`) follow the same rule — they
become a single `SourceSpec` type.

### 5. Every operation returns a typed result object

All surfaces use the same wrapper types. No unwrapping in any surface.

| Result kind   | Type                    | All surfaces            |
| ------------- | ----------------------- | ----------------------- |
| Scalar query  | `ScalarFieldResult<T>`  | `.value` to access      |
| List query    | `ListFieldResult<T>`    | `.values` to access     |
| Mutation      | `MutationResult`        | `.changed`, `.warnings` |
| Resolve query | `ResolveFieldResult<T>` | `.matches` to access    |

This is a simple, teachable rule: every operation returns a typed result
object. Adding fields to wrappers is backwards-compatible; changing a bare
return type to a wrapper is breaking.

### 6. Errors

| Surface | Shape                                                          |
| ------- | -------------------------------------------------------------- |
| Rust    | `Result<T, SysandError>` with `ErrorCode` enum                 |
| Java    | throws `SysandException` with `ErrorCode` field                |
| JS/WASM | throws `SysandError` with `code` property                      |
| Python  | raises `SysandError` subclass per code (`NotFoundError`, etc.) |

Error codes are the same enum everywhere. No per-command exception classes.

### 7. Naming conventions

| Concept            | Rust         | Java                    | JS/WASM                     | Python                 |
| ------------------ | ------------ | ----------------------- | --------------------------- | ---------------------- |
| Modules/namespaces | `snake_case` | `camelCase()` accessors | `camelCase`                 | `snake_case`           |
| Functions/methods  | `snake_case` | `camelCase`             | `camelCase`                 | `snake_case`           |
| Types              | `PascalCase` | `PascalCase`            | `PascalCase`                | `PascalCase`           |
| Enum variants      | `PascalCase` | `UPPER_SNAKE`           | `"kebab-case"` string union | `UPPER_SNAKE`          |
| Options struct     | `XxxOptions` | `XxxOptions` builder    | `XxxOptions` interface      | `XxxOptions` dataclass |

### 8. Async

| Surface | Rule                                                                                  |
| ------- | ------------------------------------------------------------------------------------- |
| Rust    | async for network ops, sync for local. `blocking` module wraps async for sync callers |
| Java    | sync by default; binding layer manages async runtime internally                       |
| JS/WASM | everything returns `Promise`                                                          |
| Python  | sync by default; binding layer manages async runtime internally                       |

Which Rust operations are async is an implementation detail. Bindings always
present sync APIs (except JS/WASM which is always `Promise`-based).

## Concrete Example

`sysand usage add urn:example 1.0.0 --project . --update sync --index https://registry.com`

**Rust:**

```rust
let ctx = ProjectContext::new(".");
let result: MutationResult = usage::add(
    &ctx,
    "urn:example",
    Some("1.0.0"),
    UsageAddOptions {
        update: UpdateMode::Sync,
        resolve: ResolveOptions {
            index: vec!["https://registry.com".parse()?],
            ..Default::default()
        },
        ..Default::default()
    },
)?;
```

**Java:**

```java
ProjectContext ctx = new ProjectContext(".");
MutationResult result = client.usage().add(
    ctx,
    "urn:example",
    "1.0.0",
    new UsageAddOptions()
        .update(UpdateMode.SYNC)
        .resolve(new ResolveOptions()
            .index(List.of("https://registry.com")))
);
```

**JS/WASM:**

```ts
const ctx = new ProjectContext(".");
const result = await sysand.usage.add(ctx, "urn:example", "1.0.0", {
  update: "sync",
  resolve: { index: ["https://registry.com"] },
});
```

**Python:**

```python
ctx = ProjectContext(".")
result = sysand.usage.add(
    ctx,
    "urn:example",
    "1.0.0",
    update=UpdateMode.SYNC,
    resolve=ResolveOptions(index=["https://registry.com"]),
)
```

## Consequences

- Projection is mechanical — no per-command design decisions needed
- All surfaces share the same result types, error codes, and options types
- Context objects are reusable across operations in all languages
- Adding fields to result wrappers is non-breaking in all surfaces
- The binding layer's only job is casing conversion and runtime management
