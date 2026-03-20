# Exploration 0009: Return Types and Error Model

## Starting Point

ADR-0005 establishes four result wrappers and an error type. This
exploration fills in the details: what fields, what error codes, and
whether the wrappers hold up under real usage patterns.

## Current Commitments (from ADRs)

- Every operation returns a typed result object — no unwrapping
- Four wrapper kinds: `ScalarFieldResult<T>`, `ListFieldResult<T>`,
  `MutationResult`, `ResolveFieldResult<T>`
- Errors use a shared `ErrorCode` enum across all surfaces
- Adding fields to wrappers is backwards-compatible

## Result Types — Proposed Fields

### ScalarFieldResult\<T\>

For single-value queries: `project info name get`, `project info version get`,
`project locate`, etc.

```rust
pub struct ScalarFieldResult<T> {
    pub value: T,
}
```

Minimal by design. The wrapper exists so we can add fields later (e.g.,
`source_file`, `deprecated`) without breaking changes.

**Open question:** Is just `value` enough, or should we include the field
path (e.g., `"info.name"`) for programmatic consumers?

### ListFieldResult\<T\>

For list queries: `usage list`, `project info maintainer list`,
`project source list`, `env list`, etc.

```rust
pub struct ListFieldResult<T> {
    pub values: Vec<T>,
}
```

**Open question:** Should this include a count? Probably not — `values.len()`
is trivial. But what about cases where the list is truncated? We don't
currently have any truncated lists, so leave this for later.

### MutationResult

For operations that change state: `project info name set`, `usage add`,
`env sync`, `project build`, `workspace build`, etc.

```rust
pub struct MutationResult {
    pub changed: bool,
    pub warnings: Vec<Warning>,
}

pub struct Warning {
    pub code: WarningCode,
    pub message: String,
}
```

`changed` answers "did anything actually change?" — useful for idempotent
operations. If `usage add` is called with an already-present usage, `changed`
is `false`.

Warnings are non-fatal issues: deprecated version, non-semver version used
with `--allow-non-semver`, etc.

**Open question:** Should `build` return a `MutationResult` or something
richer? Build produces an artifact (a KPAR file). The caller probably wants
to know the output path. Options:

1. `MutationResult` with the path conveyed via a separate mechanism
2. A `BuildResult` that extends or replaces `MutationResult`
3. `ScalarFieldResult<BuildOutput>` where `BuildOutput` has path, size, etc.

Option 3 feels wrong — build is a mutation, not a query. Option 2 breaks
the "four wrapper kinds" rule. Option 1 could work if `MutationResult`
gains an optional `output` field... but that's straining the abstraction.

**Tentative direction:** Make `MutationResult` generic enough to carry
operation-specific output:

```rust
pub struct MutationResult<T = ()> {
    pub changed: bool,
    pub warnings: Vec<Warning>,
    pub output: T,
}
```

Default `T = ()` means most mutations don't carry output. Build uses
`MutationResult<BuildOutput>`. This preserves the four-kind taxonomy
while allowing richer results.

### ResolveFieldResult\<T\>

For remote queries: `resolve show`, `resolve info name get`, etc.

```rust
pub struct ResolveFieldResult<T> {
    pub matches: Vec<ResolveMatch<T>>,
}

pub struct ResolveMatch<T> {
    pub iri: String,
    pub version: String,
    pub value: T,
}
```

A resolve query can match multiple versions of a package. Each match
carries the IRI, the version, and the requested field value.

For `resolve show`, `T` would be a full `PackageSnapshot`. For
`resolve info name get`, `T` is `String`.

## Error Model

### SysandError

```rust
pub struct SysandError {
    pub code: ErrorCode,
    pub message: String,
    pub context: Option<String>,
}
```

`context` provides location info — which file, which field, which IRI
caused the problem. Optional because some errors are context-free.

### ErrorCode — Draft Enum

```rust
pub enum ErrorCode {
    // Discovery / path
    ProjectNotFound,      // locate walked to root without finding .project.json
    WorkspaceNotFound,    // locate walked to root without finding .workspace.json
    PathNotFound,         // explicit path doesn't exist
    PathNotAProject,      // path exists but no .project.json
    PathNotAWorkspace,    // path exists but no .workspace.json

    // Config
    ConfigNotFound,       // explicit config path doesn't exist
    ConfigInvalid,        // config file parse error

    // Schema / validation
    SchemaInvalid,        // .project.json or .meta.json parse error
    FieldRequired,        // required field missing
    FieldInvalid,         // field value doesn't match constraints
    VersionInvalid,       // non-semver without --allow-non-semver
    LicenseInvalid,       // non-SPDX without --allow-non-spdx

    // Usages
    UsageNotFound,        // remove/get for a usage that doesn't exist
    UsageAlreadyExists,   // add for a usage that already exists
    UsageCycle,           // circular usage detected

    // Environment
    EnvNotFound,          // env path doesn't exist
    EnvCorrupted,         // env exists but is malformed
    EnvConflict,          // install would overwrite without --allow-overwrite

    // Resolve / network
    IndexUnreachable,     // can't reach package index
    PackageNotFound,      // IRI not found in any index
    VersionNotFound,      // IRI found but version constraint unsatisfied

    // Build
    BuildFailed,          // KPAR creation failed

    // Lock
    LockStale,            // lockfile doesn't match manifest

    // Generic
    IoError,              // filesystem error
    Internal,             // unexpected / bug
}
```

This is a flat enum — no hierarchy. Binding surfaces project it directly:
Java `ErrorCode.PROJECT_NOT_FOUND`, JS `"project-not-found"`,
Python `ProjectNotFoundError` (subclass per code).

**Open question:** Should we distinguish "not found" from "not valid"
for project paths? Currently we have `PathNotFound` (doesn't exist) vs.
`PathNotAProject` (exists but no `.project.json`). Locate would use
`ProjectNotFound` (walked to root). Are three levels too many? They
cover genuinely different user errors, so probably fine.

## End-to-End Scenarios

### Scenario 1: Library user adds a usage and syncs

```rust
let path = project::locate(".")?;
let ctx = ProjectContext::new(path);

// Add a usage
let result = usage::add(
    &ctx,
    "urn:example:sensors",
    Some("^2.0"),
    UsageAddOptions {
        update: UpdateMode::Sync,
        ..Default::default()
    },
)?;
// result: MutationResult<()>
// result.changed == true
// result.warnings == []

// Check what's installed
let usages = usage::list(&ctx)?;
// usages: ListFieldResult<UsageEntry>
// usages.values contains the new entry

// Later, check the name
let name = project::info::name::get(&ctx)?;
// name: ScalarFieldResult<String>
// name.value == "my-project"
```

This flows naturally. The caller handles errors at each `?` point and
gets typed results they can inspect.

### Scenario 2: Build a project KPAR

```rust
let path = project::locate("./my-project")?;
let ctx = ProjectContext::new(path);

let result = project::build(
    &ctx,
    ProjectBuildOptions {
        target: Some("./dist".into()),
        compression: Compression::Zstd,
        ..Default::default()
    },
)?;
// result: MutationResult<BuildOutput>
// result.changed == true
// result.output.path == "./dist/my-project-1.0.0.kpar"
// result.output.size_bytes == 4096
// result.warnings == []
```

If we keep `MutationResult<()>` for build, the caller doesn't know
where the output went without convention. The generic `T` on
`MutationResult` earns its keep here.

### Scenario 3: Python user discovers and inspects

```python
path = sysand.project.locate(".")
ctx = ProjectContext(path)

name = sysand.project.info.name.get(ctx)
print(name.value)  # "my-sensors"

maintainers = sysand.project.info.maintainer.list(ctx)
for m in maintainers.values:
    print(m)
```

The `.value` / `.values` pattern is simple enough for Python's dynamic
style.

### Scenario 4: Error handling in JS/WASM

```ts
try {
  const path = await sysand.project.locate(".");
  const ctx = new ProjectContext(path);
  await sysand.usage.add(ctx, "urn:missing:package", "^1.0");
} catch (err) {
  if (err instanceof SysandError) {
    console.log(err.code);     // "package-not-found"
    console.log(err.message);  // "Package urn:missing:package not found..."
    console.log(err.context);  // "urn:missing:package"
  }
}
```

JS gets kebab-case string codes per ADR-0005 naming conventions.

### Scenario 5: Idempotent add in Java

```java
ProjectContext ctx = new ProjectContext(".");
try {
    MutationResult<?> result = client.usage().add(
        ctx, "urn:example:sensors", "^2.0",
        new UsageAddOptions().update(UpdateMode.MANIFEST)
    );
    if (!result.changed()) {
        System.out.println("Usage already present, no change.");
    }
} catch (SysandException e) {
    if (e.code() == ErrorCode.USAGE_ALREADY_EXISTS) {
        // Depends on design: is duplicate add an error or changed=false?
    }
}
```

**Open question:** Is adding an already-existing usage an error
(`UsageAlreadyExists`) or a no-op (`changed = false`)? Idempotent
semantics suggest no-op. But what if the version constraint differs?
Then it's an update, which is a different operation... or should `add`
with a different version constraint update in place?

This needs a decision. Options:
- `add` is idempotent: same IRI + same version = no-op, same IRI +
  different version = update in place
- `add` is strict: same IRI = error, use a separate `update` command
- `add` always overwrites: same IRI = replace, `changed` reflects
  whether the entry actually differed

## Open Questions Summary

1. Should `MutationResult` be generic (`MutationResult<T = ()>`) to
   carry operation-specific output like build paths?
2. Should `ScalarFieldResult` include a field path for programmatic use?
3. Is duplicate `usage add` an error or a no-op/update?
4. Should `WarningCode` be a separate enum, or just strings?
5. Should `locate` return `ScalarFieldResult<String>` or a bare path?
   Using the wrapper is consistent but adds ceremony for a very common
   operation.

## Deep Dive: Return Type Strategies and Binding Consequences

The core tension: Rust's type system is expressive (generics, default
type parameters, zero-cost abstractions). Binding tooling (UniFFI,
wasm-bindgen, PyO3) is not — it needs concrete types at the FFI boundary.
Whatever we choose in Rust ripples into maintenance burden across four
surfaces.

### How many operations produce output beyond changed/warnings?

Looking at the command tree:

**Mutations with no meaningful output (changed + warnings is enough):**
- `project init` — creates files, but caller already knows the path
- `project info name set`, `description set`, `version set`, etc.
- `project info maintainer add/remove/set/clear`
- `project info topic add/remove/set/clear`
- `project metadata metamodel set-standard/set-custom/clear`
- `project metadata includes-derived set/clear`
- `project metadata includes-implied set/clear`
- `project source add/remove`
- `usage add/remove`
- `lock update`
- `env create`
- `env sync`
- `env install/uninstall`

**Mutations where the caller probably wants output:**
- `project build` — output path, size, checksum of the KPAR
- `workspace build` — output path(s), sizes, checksums of multiple KPARs
- `project clone` — path where the project was cloned to

That's roughly 25 plain mutations vs. 3 with meaningful output. The
question is whether those 3 justify a generic parameter on the type that
all 25 also carry.

### Option A: Generic MutationResult\<T = ()\>

**Rust library:**

```rust
pub struct MutationResult<T = ()> {
    pub changed: bool,
    pub warnings: Vec<Warning>,
    pub output: T,
}

// Most mutations:
fn set(ctx: &ProjectContext, name: &str) -> Result<MutationResult, SysandError>;

// Build:
fn build(ctx: &ProjectContext, opts: ProjectBuildOptions)
    -> Result<MutationResult<BuildOutput>, SysandError>;
```

Clean in Rust. `MutationResult` (without parameter) defaults to `()`.
The `output` field exists but is `()` — Rust optimizes it away.

**UniFFI / binding generation:**

UniFFI does not support generic structs. It needs concrete types at the
boundary. This means we'd generate:

```
MutationResult          → { changed: bool, warnings: [...] }
MutationResultBuildOutput → { changed: bool, warnings: [...], output: BuildOutput }
```

Two types. UniFFI scaffolding or manual wrapper code needed to convert.
Every new output type means another concrete struct in the binding layer.

**Java consequence:**

```java
// Users see two types:
MutationResult result = client.usage().add(ctx, iri);
MutationResultBuildOutput result = client.project().build(ctx, opts);
result.output().path();  // only on the build variant
```

Or with an inheritance approach:
```java
// MutationResultBuildOutput extends MutationResult
MutationResult result = client.project().build(ctx, opts);
((MutationResultBuildOutput) result).output();  // casting needed
```

Neither is great. The two-type approach is honest but clutters the API.
The inheritance approach is fragile.

**JS/WASM consequence:**

wasm-bindgen also doesn't support generics. Same two concrete types.
TypeScript could define a union, but the runtime value is one or the other:

```ts
// Two types at runtime:
const result: MutationResult = await sysand.usage.add(ctx, iri);
const buildResult: BuildMutationResult = await sysand.project.build(ctx, opts);
buildResult.output.path;
```

Or a single type with an optional field:
```ts
interface MutationResult {
  changed: boolean;
  warnings: Warning[];
  output?: unknown;  // messy
}
```

**Python consequence:**

PyO3 can do generics via `Generic[T]` at the Python type level, but the
underlying Rust struct still needs to be concrete at the PyO3 boundary.
Similar to Java — two classes or one class with optional fields.

**Maintenance cost:** Each new output type (if we ever add more) requires
a new concrete binding type in every surface. For 3 operations out of 28,
this is manageable but adds friction.

### Option B: Composition — Dedicated Result Types

**Rust library:**

```rust
pub struct MutationResult {
    pub changed: bool,
    pub warnings: Vec<Warning>,
}

pub struct BuildResult {
    pub mutation: MutationResult,
    pub path: Utf8PathBuf,
    pub size_bytes: u64,
    pub checksum: String,
}

pub struct CloneResult {
    pub mutation: MutationResult,
    pub path: Utf8PathBuf,
}
```

Each operation-specific result composes `MutationResult` rather than
parameterizing it.

**Java:**

```java
MutationResult result = client.usage().add(ctx, iri);
result.changed();

BuildResult result = client.project().build(ctx, opts);
result.mutation().changed();
result.path();
result.sizeBytes();
```

Clean, no generics needed. Each type is self-describing. UniFFI generates
them directly.

**JS/WASM:**

```ts
const result: MutationResult = await sysand.usage.add(ctx, iri);
result.changed;

const buildResult: BuildResult = await sysand.project.build(ctx, opts);
buildResult.mutation.changed;
buildResult.path;
```

Natural JavaScript objects. No optional fields, no type unions.

**Python:**

```python
result = sysand.usage.add(ctx, iri)
result.changed

build_result = sysand.project.build(ctx, opts)
build_result.mutation.changed
build_result.path
```

**Maintenance cost:** New output types are just new structs that compose
`MutationResult`. No generic machinery needed. But: the `mutation` field
nesting is slightly verbose — `result.mutation.changed` vs `result.changed`.

**Variant B2 — Flatten instead of nest:**

```rust
pub struct BuildResult {
    pub changed: bool,
    pub warnings: Vec<Warning>,
    pub path: Utf8PathBuf,
    pub size_bytes: u64,
    pub checksum: String,
}
```

Duplicates the `changed`/`warnings` fields in each result type. Simpler
access (`result.changed`) but code duplication in the Rust lib. For 3
types, this is fine. For 10, it would be tedious.

### Option C: MutationResult with Optional Output Map

**Rust library:**

```rust
pub struct MutationResult {
    pub changed: bool,
    pub warnings: Vec<Warning>,
    pub output: HashMap<String, String>,
}
```

Build sets `output["path"]`, `output["size_bytes"]`, etc. One type
everywhere, infinitely extensible.

**Binding consequence:** Easy to generate — one type in every surface.
But: completely untyped. Callers must know the right keys, cast values.
Defeats the purpose of a typed API. **Reject.**

### Option D: One Type, Optional Typed Fields

```rust
pub struct MutationResult {
    pub changed: bool,
    pub warnings: Vec<Warning>,
    pub build_output: Option<BuildOutput>,
    pub clone_output: Option<CloneOutput>,
}
```

One type everywhere. Most fields are `None` most of the time.

**Binding consequence:** Simple to project — one struct, optional fields.
But: the type grows with every new operation that has output. After 5
operations, `MutationResult` has 5 optional output fields, 4 of which
are always `None`. Semantically messy. Callers must know which field to
check for which operation. **Reject.**

### Applying the Same Analysis to Query Results

`ScalarFieldResult<T>` and `ListFieldResult<T>` have the same generic
issue. Let's check if they actually need it.

**ScalarFieldResult\<T\>** — `T` varies: `String` (name, description),
`bool` (includes-derived), version, license, URI, etc. There are ~15
different `T` values across project info/metadata fields.

Unlike `MutationResult` where only 3 out of 28 need output, *every*
scalar query needs a different `T`. Generating 15 concrete types
(`ScalarFieldResultString`, `ScalarFieldResultBool`, ...) is unworkable.

This means the binding layer *must* handle generics for query results
somehow. Options:

1. UniFFI can handle a small set of primitive types as generic params
   (`String`, `bool`, `u64`). Most scalar queries return these.
2. Use `serde_json::Value` or equivalent at the boundary — typed in
   Rust, dynamic in bindings. Loses type safety.
3. Return bare values from queries (no wrapper) — the simplest option
   but breaks the "every operation returns a result object" rule.

UniFFI actually *does* support generic-like patterns for a small set of
types through proc-macro expansion. The practical approach: define the
Rust API with generics, and let the binding generator create concrete
types for each instantiation it encounters. This is what UniFFI does —
it monomorphizes at the boundary.

So the binding tooling will already handle `ScalarFieldResult<String>`,
`ScalarFieldResult<bool>`, etc. The question is whether we also want it
to handle `MutationResult<BuildOutput>`, `MutationResult<CloneOutput>`,
etc.

### Summary Table

| Approach | Rust ergonomics | Binding complexity | Type safety | Maintenance |
| -------- | --------------- | ------------------ | ----------- | ----------- |
| A: Generic MutationResult\<T\> | Excellent | Medium — 3 concrete types generated | Full | Low in Rust, medium in bindings |
| B: Composition | Good (nesting) | Low — each type is concrete | Full | Low everywhere |
| B2: Flattened | Good (flat) | Low — each type is concrete | Full | Medium (field duplication) |
| C: Output map | OK | Low — one type | None | Low but fragile |
| D: Optional fields | Poor | Low — one type | Partial | Grows badly |

### Tentative Assessment

Since the binding layer already handles monomorphized generics for query
results (`ScalarFieldResult<String>`, etc.), adding 2–3 more
monomorphizations for `MutationResult<BuildOutput>` is marginal extra
cost. The machinery already exists.

But composition (Option B) has a pragmatic advantage: each result type
is self-documenting and has named fields specific to the operation
(`path`, `size_bytes`) rather than a generic `output` that you have to
unwrap. In bindings, `BuildResult` with a `path` field is clearer than
`MutationResultBuildOutput` with an `output` field that has a `path`.

The nesting cost (`result.mutation.changed`) is real but small. And
Rust can provide convenience accessors:

```rust
impl BuildResult {
    pub fn changed(&self) -> bool { self.mutation.changed }
    pub fn warnings(&self) -> &[Warning] { &self.mutation.warnings }
}
```

Bindings can do the same. The nesting is an implementation detail that
surfaces can flatten via methods.

**Leaning toward Option B (composition) with convenience accessors**
because:
- Self-documenting types in every surface
- No generic machinery needed for mutations
- Binding tooling does the least work
- Named fields > generic `output`
- Convenience accessors hide the nesting
