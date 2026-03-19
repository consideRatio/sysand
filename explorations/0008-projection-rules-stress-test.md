# 0008: Projection Rules — Stress Test

## Purpose

Before locking projection rules into an ADR, test them against concrete
commands from ADR-0004 to find gaps, inconsistencies, or awkwardness.

## Test Cases

### 1. Simple query: `project info name get`

**CLI:**

```
sysand project info name get --project /path
```

**Rust:**

```rust
let ctx = ProjectContext::new("/path");
let result: ScalarFieldResult<String> = project::info::name::get(&ctx)?;
```

**Java:**

```java
ProjectContext ctx = new ProjectContext("/path");
ScalarFieldResult<String> result = client.project().info().name().get(ctx);
```

**JS/WASM:**

```ts
const ctx = new ProjectContext("/path");
const result: ScalarFieldResult<string> =
  await sysand.project.info.name.get(ctx);
```

**Python:**

```python
ctx = ProjectContext("/path")
result: ScalarFieldResult[str] = sysand.project.info.name.get(ctx)
```

**Observation:** Consistent across all surfaces — explicit context, typed wrapper.

---

### 2. Mutation with side effects: `usage add`

**CLI:**

```
sysand usage add urn:example 1.0.0 --project . --update sync --index https://my-registry.com
```

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
            index: vec!["https://my-registry.com".parse()?],
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
            .index(List.of("https://my-registry.com")))
);
```

**JS/WASM:**

```ts
const ctx = new ProjectContext(".");
const result = await sysand.usage.add(ctx, "urn:example", "1.0.0", {
  update: "sync",
  resolve: { index: ["https://my-registry.com"] },
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
    resolve=ResolveOptions(index=["https://my-registry.com"]),
)
```

**Observation:** Consistent — context first, then required args, then options.
All surfaces follow the same order.

---

### 3. List mutation: `project info maintainer add`

**CLI:**

```
sysand project info maintainer add "Alice" --project .
```

**Rust:**

```rust
let ctx = ProjectContext::new(".");
let result: MutationResult = project::info::maintainer::add(&ctx, "Alice")?;
```

**Java:**

```java
ProjectContext ctx = new ProjectContext(".");
MutationResult result = client.project().info().maintainer().add(ctx, "Alice");
```

**JS/WASM:**

```ts
const ctx = new ProjectContext(".");
const result = await sysand.project.info.maintainer.add(ctx, "Alice");
```

**Python:**

```python
ctx = ProjectContext(".")
result = sysand.project.info.maintainer.add(ctx, "Alice")
```

**Observation:** Clean across all surfaces. Context first, then required args.
The deep namespace chain (`project().info().maintainer().add()`) is verbose in
Java but predictable.

---

### 4. Remote query: `resolve info name get`

**CLI:**

```
sysand resolve info name get urn:example --index https://my-registry.com
```

**Rust:**

```rust
let result: ResolveFieldResult<String> = resolve::info::name::get(
    "urn:example",
    ResolveOptions {
        index: vec!["https://my-registry.com".parse()?],
        ..Default::default()
    },
).await?;
```

**Java:**

```java
ResolveFieldResult<String> result = client.resolve().info().name().get(
    "urn:example",
    new ResolveOptions().index(List.of("https://my-registry.com"))
);
```

**JS/WASM:**

```ts
const result = await sysand.resolve.info.name.get("urn:example", {
  index: ["https://my-registry.com"],
});
```

**Python:**

```python
result = sysand.resolve.info.name.get(
    "urn:example",
    resolve=ResolveOptions(index=["https://my-registry.com"]),
)
```

```

**Observation:** No `ProjectContext` here — `resolve` commands are project-less.
The first arg is always the IRI. `ResolveOptions` is the only options group.
Clean.

But: **Rust is async, Java/Python are sync.** The binding layer hides the
async runtime. This is stated in the rules but worth being explicit: bindings
always present sync APIs and manage the runtime internally.

---

### 5. Workspace operation: `build workspace`

**CLI:**
```

sysand build workspace --workspace /path --target ./output

````

**Rust:**
```rust
let ctx = WorkspaceContext::new("/path");
let result: BuildArtifactsResult = build::workspace(
    &ctx,
    BuildWorkspaceOptions {
        target: Some("./output".into()),
        compression: Compression::Deflated,  // default
    },
)?;
````

**Java:**

```java
WorkspaceContext ctx = new WorkspaceContext("/path");
BuildArtifactsResult result = client.build().workspace(
    ctx,
    new BuildWorkspaceOptions()
        .target("./output")
);
```

**JS/WASM:**

```ts
const ctx = new WorkspaceContext("/path");
const result = await sysand.build.workspace(ctx, { target: "./output" });
```

**Python:**

```python
ctx = WorkspaceContext("/path")
result = sysand.build.workspace(ctx, target="./output")
```

**Observation:** Consistent — explicit context in all surfaces, same pattern
as `ProjectContext`.

---

## Gaps Found

### Gap 1: ProjectContext in JS/WASM and Python

In Rust and Java, `ProjectContext` is an explicit type passed as first arg.
In Python and JS, it's flattened — `project=` and `config=` become keyword
args / options fields.

This is arguably fine — Python and JS are dynamically typed and options objects
are idiomatic. But it means the projection isn't purely structural for these
surfaces. The binding layer must know to extract `project` and `config` from
the options and construct a `ProjectContext` internally.

**Resolved:** Explicit `ProjectContext` (and `WorkspaceContext`) in all surfaces.
Consistent projection, reusable across multiple operations, and keeps the
binding layer mechanical. The cost (one extra line + import) is minimal.

### Gap 2: Return type unwrapping

Exploration 0004 says Python and JS may unwrap `ScalarFieldResult<T>` and
`ListFieldResult<T>` for ergonomics. This means:

- Rust: `ScalarFieldResult<String>` (has `.value` field)
- Java: `ScalarFieldResult<String>` (has `.getValue()` method)
- JS: returns `string` directly
- Python: returns `str` directly

If `ScalarFieldResult` ever gains fields, Python/JS would need breaking changes
to add them. Alternatively, we could keep wrappers everywhere for consistency.

**Resolved:** Keep wrappers everywhere. Every operation returns a typed result
object in all surfaces. Adding fields to wrappers is backwards-compatible;
changing a bare return type to a wrapper is breaking. The cost is `.value` —
trivial. Consistent rule: "every operation returns a typed result object."

### Gap 3: Async boundary

The rules say "Rust: async for network ops" and "bindings: sync by default."
But which operations are async? From the command tree:

- `project clone` — network (fetches remote project)
- `usage add` with `--update lock|sync` — network (resolves deps)
- `lock update` — network
- `env sync` — network
- `env install` — network
- All `resolve` commands — network

The binding layer must know which operations need a runtime. This isn't
visible in the CLI or the binding API — it's an implementation detail the
binding layer handles. Just needs to be documented somewhere.

**No decision needed** — this is an implementation concern, not an API one.

### Gap 4: `--source-kind` + `--source` as paired options

These two options always appear together. In the CLI they're two flags. In
bindings they should be a single `SourceSpec` type:

```rust
pub struct SourceSpec { pub kind: SourceKind, pub value: String }
```

But the CLI has them as separate flags. The binding layer needs to combine
them. The projection rule for paired options isn't explicitly stated.

**Resolved:** No separate rule needed. This is just the smallest case of the
existing shared options grouping rule. If `--index` + `--default-index` +
`--index-mode` + `--include-std` become `ResolveOptions`, then `--source-kind`

- `--source` becoming `SourceSpec` is obvious.
