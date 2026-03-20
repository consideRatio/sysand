# ADR-0001: Discovery and Config Boundaries

- **Status**: Accepted
- **Date**: 2026-03-19
- **Scope**: Project operations only. Workspace and resolve operations are out
  of scope for this ADR.

## Context

Sysand has a CLI, a Rust library, and bindings for Python, Java, and JS/WASM.
The CLI runs in a terminal with a current working directory. The library and
bindings do not.

The CLI currently uses CWD to discover the project root (by walking up to find
`.project.json`), the workspace root (`.workspace.json`), and config files
(`sysand.toml`). This discovery logic is convenient for CLI users but creates
implicit behavior that is hard to reproduce in library and binding contexts.

We need to decide where discovery and config loading responsibilities live.

## Decision

### 1. Discovery is CLI-only

Path traversal (walking up the directory tree to find `.project.json` or
`.workspace.json`) is a CLI convenience. The library does not perform discovery.

- **CLI**: discovers project/workspace from CWD, then passes concrete paths
  to the library
- **Library**: takes an explicit project path or workspace path, never
  traverses upward
- **Bindings**: callers provide paths explicitly, same as the library

### 2. Config is project-level only

There is one config file: `<project>/sysand.toml`. No user-level config, no
workspace-level config, no config merging. The library reads one file from one
known location.

Config currently controls:

- Package index URLs (with default/non-default distinction and priority)
- Project source overrides (mapping IRIs to specific sources)

### 3. Config loading is library-side, automatically, but overridable

When the library receives a project path, it automatically loads
`<project>/sysand.toml` if present. Every project operation allows overriding
this behavior:

```rust
pub enum ConfigMode {
    /// Load sysand.toml from the project root (default)
    Auto,
    /// Use this specific config file
    File(Utf8PathBuf),
    /// Don't load any config
    None,
}
```

### 4. ProjectContext is the shared input for all project operations

Rather than repeating project path and config mode in every operation's input
struct, a shared `ProjectContext` type carries both:

```rust
pub struct ProjectContext {
    pub path: Utf8PathBuf,
    pub config: ConfigMode,  // defaults to Auto
}
```

Every project operation takes `&ProjectContext` as its first argument. This
avoids a proliferation of per-operation input types for the common fields.

Operations that need additional inputs take them as separate parameters:

```rust
project::show(project: &ProjectContext)?;
project::info::name::set(project: &ProjectContext, name: &str)?;
usage::add(project: &ProjectContext, iri: &str, options: UsageAddOptions)?;
```

### 5. WorkspaceContext is the shared input for workspace operations

```rust
pub struct WorkspaceContext {
    pub path: Utf8PathBuf,
}
```

Just a path. No config — workspace operations don't consider config at the
moment. If config is needed later, adding the field with a default is
non-breaking.

Workspace operations take `&WorkspaceContext` as their first argument:

```rust
workspace::build(workspace: &WorkspaceContext, options: WorkspaceBuildOptions)?;
```

## CLI Mapping

A single `--config` flag covers all modes:

| CLI               | ConfigMode   |
| ----------------- | ------------ |
| (no flag)         | `Auto`       |
| `--config auto`   | `Auto`       |
| `--config none`   | `None`       |
| `--config <PATH>` | `File(path)` |

No `--no-config` negation flag. The keywords `auto` and `none` are not valid
file paths in practice, so disambiguation is unambiguous.

## Binding Projections

**Rust:**

```rust
let ctx = ProjectContext::new(".");
project::show(&ctx)?;
project::info::name::set(&ctx, "my-project")?;

// With explicit config:
let ctx = ProjectContext::new(".").config(ConfigMode::File("sysand.toml".into()));
project::show(&ctx)?;
```

**Java:**

```java
ProjectContext ctx = new ProjectContext(".");
client.project().show(ctx);
client.project().info().name().set(ctx, "my-project");

// With explicit config:
ProjectContext ctx = new ProjectContext(".")
    .config(ConfigMode.file("/path/to/sysand.toml"));
client.project().show(ctx);
```

**JS/WASM:**

```ts
await sysand.project.show({ project: "." }); // auto
await sysand.project.show({ project: ".", config: "/path/to/sysand.toml" }); // explicit
await sysand.project.show({ project: ".", config: null }); // none
```

**Python:**

```python
sysand.project.show(project=".")                                 # auto
sysand.project.show(project=".", config="/path/to/sysand.toml")  # explicit
sysand.project.show(project=".", config=False)                   # none
```

## What the Library Reads at a Project Path

Given a project path, the library accesses these files directly (no traversal):

- `<project>/.project.json` — project info
- `<project>/.meta.json` — project metadata
- `<project>/sysand.toml` — project config (unless `ConfigMode::None`)

Given a workspace path:

- `<workspace>/.workspace.json` — workspace info (list of projects with paths and IRIs)

## What Stays CLI-Only

- Walking up from CWD to find `.project.json`
- Walking up to find `.workspace.json`
- Applying `--project` / `--workspace` defaults from CWD
- Terminal concerns: color, prompting, log level, output format

## Consequences

- The library API is predictable: explicit paths in, no hidden state
- `ProjectContext` is reusable across operations — construct once, use many times
- All binding surfaces get config override for free via their idiomatic patterns
- No config merging logic needed — one file, one location
- Discovery logic only needs to be maintained in the CLI crate
- User-level config (auth, personal indexes) can be added later as a
  separate layer without changing this decision
