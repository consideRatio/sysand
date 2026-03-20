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
