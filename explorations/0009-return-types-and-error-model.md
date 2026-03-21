# Exploration 0009: Return Types and Error Model

**Status: Distilled into ADR-0005 amendment** — direction established (natural return types, drop
wrappers), ADR-0005 updated.

## Starting Point

ADR-0005 establishes four result wrappers (`ScalarFieldResult<T>`,
`ListFieldResult<T>`, `MutationResult`, `LookupFieldResult<T>`) and an
error type. This exploration questions whether those wrappers earn their
keep, and proposes a simpler alternative.

## Why the Wrappers Were Introduced

The stated rationale: forward compatibility. If `project info name get`
returns a bare `String` and we later want to also return where the value
came from, that's a breaking change. A wrapper lets us add fields without
breaking callers.

## Why the Wrappers Don't Earn Their Keep

**Speculative abstractions.** The wrappers exist to protect against a
future need that may never arrive. Every caller pays the ceremony cost
now.

**`MutationResult` fields don't hold up under scrutiny:**

- `changed: bool` — When would a caller branch on this? Most callers
  just want success or failure. If `usage add` with an existing usage is
  a no-op, that's fine — the caller doesn't need to know. CI scripts
  that care about whether state changed can check other ways (diffing
  files, exit codes).

- `warnings: Vec<Warning>` — What goes here concretely? Non-semver
  version with `--allow-non-semver`? The caller explicitly asked for
  that. Deprecated package? That's a diagnostic concern — the CLI can
  log it, but library callers won't programmatically act on it.
  Warnings are better handled via a diagnostic/logging channel, not a
  return value.

**Query wrappers are pure overhead.** `ScalarFieldResult<String>` adds
`.value` indirection to what could just be a `String`. `ListFieldResult`
wraps a `Vec` with `.values`. These wrappers exist only to reserve space
for future fields — a tax on every caller, paid against a hypothetical.

## Proposed Direction: Natural Return Types

Drop the four-wrapper taxonomy. Each operation returns `Result<T, SysandError>`
where `T` is whatever makes sense for that operation.

### The Rule

Every operation returns `Result<T, SysandError>`. `T` is the natural
shape — a primitive, a domain struct, a `Vec`, or `()`.

### What T Looks Like Across Operations

**Returns `()`** — mutations with no meaningful output:

```
project init                          → ()
project info name set                 → ()
project info maintainer add           → ()
project source add                    → ()
usage add                             → ()
usage remove                          → ()
lock update                           → ()
env create                            → ()
env sync                              → ()
env install                           → ()
env uninstall                         → ()
```

**Returns a primitive or simple type:**

```
project info name get                 → String
project info version get              → String
project info license get              → Option<String>
project info website get              → Option<String>
project metadata includes-derived get → Option<bool>
project locate                        → Utf8PathBuf
workspace locate                      → Utf8PathBuf
```

**Returns a list:**

```
project info maintainer list          → Vec<String>
project info topic list               → Vec<String>
usage list                            → Vec<UsageEntry>
project source list                   → Vec<SourceEntry>
env list                              → Vec<EnvEntry>
env source list                       → Vec<SourceEntry>
project metadata index list           → Vec<IndexEntry>
project metadata checksum list        → Vec<ChecksumEntry>
```

**Returns a domain struct:**

```
project show                          → ProjectSnapshot
project build                         → BuildOutput { path, size_bytes, checksum }
workspace build                       → Vec<BuildOutput>
project clone                         → CloneOutput { path }
project metadata metamodel get        → Option<Metamodel>
project metadata created get          → Option<DateTime>
```

**Returns lookup-specific structures:**

```
lookup show                          → Vec<LookupMatch>
lookup info name get                 → Vec<LookupMatch<String>>
lookup info maintainer list          → Vec<LookupMatch<Vec<String>>>
```

Lookup is the one area where a wrapper-like structure is inherent to the
domain — a query can match multiple versions. But `Vec<LookupMatch<T>>`
is just the natural shape of the data, not a speculative wrapper.

### If a Return Type Needs to Grow

If `project info name get` later needs to carry source file info, it
becomes:

```rust
pub struct NameInfo {
    pub name: String,
    pub source_file: Utf8PathBuf,
}
```

This is a breaking change. But:

1. It only happens when there's a real need, not speculatively.
2. It's a targeted break affecting one operation, not a system-wide
   wrapper change.
3. Pre-1.0 software breaks APIs — that's expected. Post-1.0, we can
   evaluate whether any field genuinely needs enrichment before
   committing to a stable API.

The alternative (wrappers everywhere) pays the cost on every operation
up front to avoid a break that may never happen on most operations.

## End-to-End Scenarios

### Scenario 1: Rust library — add a usage and inspect

```rust
let path = project::locate(".")?;
let ctx = ProjectContext::new(path);

usage::add(
    &ctx,
    "urn:example:sensors",
    Some("^2.0"),
    UsageAddOptions {
        update: UpdateMode::Sync,
        ..Default::default()
    },
)?;
// Success means it worked. No result to inspect.

let usages: Vec<UsageEntry> = usage::list(&ctx)?;
let name: String = project::info::name::get(&ctx)?;
println!("Project {} has {} usages", name, usages.len());
```

No `.value`, no `.values`, no `.changed`. Just the data.

### Scenario 2: Rust library — build a KPAR

```rust
let path = project::locate("./my-project")?;
let ctx = ProjectContext::new(path);

let output: BuildOutput = project::build(
    &ctx,
    ProjectBuildOptions {
        target: Some("./dist".into()),
        compression: Compression::Zstd,
        ..Default::default()
    },
)?;
println!("Built {} ({} bytes)", output.path, output.size_bytes);
```

`BuildOutput` has named fields specific to building. No generic wrapper.

### Scenario 3: Java — discover and inspect

```java
String path = client.project().locate(".");
ProjectContext ctx = new ProjectContext(path);

String name = client.project().info().name().get(ctx);
List<String> maintainers = client.project().info().maintainer().list(ctx);
System.out.println(name + " maintained by " + maintainers);
```

Direct types. `String`, `List<String>`. No `.value()` unwrapping.

### Scenario 4: JS/WASM — build and use output

```ts
const path = await sysand.project.locate(".");
const ctx = new ProjectContext(path);

const output = await sysand.project.build(ctx, {
  target: "./dist",
  compression: "zstd",
});
console.log(`Built ${output.path} (${output.sizeBytes} bytes)`);
```

### Scenario 5: Python — error handling

```python
try:
    path = sysand.project.locate(".")
    ctx = ProjectContext(path)
    sysand.usage.add(ctx, "urn:missing:package", "^1.0")
except PackageNotFoundError as e:
    print(f"Not found: {e.message}")
    print(f"Context: {e.context}")  # "urn:missing:package"
```

### Scenario 6: Python — lookup query

```python
matches = sysand.lookup.show("urn:example:sensors")
for m in matches:
    print(f"{m.iri} @ {m.version}")
```

`matches` is just a list of `LookupMatch` objects. The list structure
is the natural shape of the data, not a wrapper.

## Error Model

The error model is independent of the return type decision.

### SysandError

```rust
pub struct SysandError {
    pub code: ErrorCode,
    pub message: String,
    pub context: Option<String>,
}
```

### ErrorCode — Draft Enum

```rust
pub enum ErrorCode {
    // Discovery / path
    ProjectNotFound,
    WorkspaceNotFound,
    PathNotFound,
    PathNotAProject,
    PathNotAWorkspace,

    // Config
    ConfigNotFound,
    ConfigInvalid,

    // Schema / validation
    SchemaInvalid,
    FieldRequired,
    FieldInvalid,
    VersionInvalid,
    LicenseInvalid,

    // Usages
    UsageNotFound,
    UsageAlreadyExists,
    UsageCycle,

    // Environment
    EnvNotFound,
    EnvCorrupted,
    EnvConflict,

    // Resolve / network
    IndexUnreachable,
    PackageNotFound,
    VersionNotFound,

    // Build
    BuildFailed,

    // Lock
    LockStale,

    // Generic
    IoError,
    Internal,
}
```

Flat enum. Projected directly to every surface: Java `ErrorCode.PROJECT_NOT_FOUND`,
JS `"project-not-found"`, Python `ProjectNotFoundError` (subclass per code).

## Binding Consequences

### Return types project trivially

| Rust return       | Java               | JS/WASM          | Python             |
| ----------------- | ------------------ | ---------------- | ------------------ |
| `String`          | `String`           | `string`         | `str`              |
| `Option<String>`  | `@Nullable String` | `string \| null` | `str \| None`      |
| `bool`            | `boolean`          | `boolean`        | `bool`             |
| `Vec<String>`     | `List<String>`     | `string[]`       | `list[str]`        |
| `Vec<UsageEntry>` | `List<UsageEntry>` | `UsageEntry[]`   | `list[UsageEntry]` |
| `BuildOutput`     | `BuildOutput`      | `BuildOutput`    | `BuildOutput`      |
| `()`              | `void`             | `Promise<void>`  | `None`             |

No generic wrappers to monomorphize. No wrapper types to maintain.
Domain structs like `BuildOutput` and `UsageEntry` are concrete types
that binding generators handle directly.

### Maintenance effort comparison

**With wrappers (original ADR-0005):**

- 4 generic wrapper types to define in Rust
- Each wrapper must be projected to 4 binding surfaces
- Binding generators must monomorphize each instantiation
- Every caller unwraps `.value` / `.values` / `.changed`
- Adding a field to a wrapper affects every operation using it

**Without wrappers (proposed):**

- Domain structs only — defined as needed
- Each struct projects once to each surface
- No monomorphization needed
- Callers use return values directly
- Changing a return type is a targeted break on one operation

## What This Means for ADR-0005

ADR-0005 section 5 ("Every operation returns a typed result object")
changes from a wrapper taxonomy to a simpler rule:

> Every operation returns `Result<T, SysandError>` where `T` is the
> natural type for that operation. No unwrapping, no universal wrappers.

The section on result wrappers (`ScalarFieldResult`, `ListFieldResult`,
`MutationResult`, `LookupFieldResult`) would be removed. The error
model section is unchanged.

## Open Questions

1. Should `LookupMatch` be considered a wrapper or a domain type?
   It's inherent to the lookup operation's semantics (multiple versions
   match), so it's a domain type — but worth noting it's the closest
   thing to a wrapper that survives.

2. How does `--format json` work without wrappers? The CLI serializes
   whatever `T` is. Primitives serialize directly. Domain structs derive
   `Serialize`. This actually becomes simpler — no wrapper envelope in
   the JSON output.

3. Is `usage add` returning `()` the right choice? If it already exists
   with the same version constraint, that's a silent no-op. If it exists
   with a different constraint... is that an error or an update? This is
   a separate design question from return types.
