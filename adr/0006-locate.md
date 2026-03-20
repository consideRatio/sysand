# ADR-0006: Locate

- **Status**: Accepted
- **Date**: 2026-03-20
- **Supersedes**: ADR-0001 §1 ("Discovery is CLI-only")
- **Updates**: ADR-0004 (command tree: `project root` → `project locate`,
  adds `workspace locate`)

## Context

ADR-0001 placed all discovery logic (walking up the directory tree to find
`.project.json` or `.workspace.json`) in the CLI only. The library required
explicit paths for all operations.

In practice, library and binding users face the same problem as CLI users:
given a path inside a project or workspace, find the root. Forcing every
consumer to reimplement upward traversal is unnecessary duplication.

At the same time, the original concern from ADR-0001 still holds: no library
operation should perform discovery implicitly. The caller should always know
when traversal happens.

## Decision

### 1. Locate is a library operation

The library provides two locate functions that walk up from a given path:

- `project::locate(path)` — walks up from `path` to find `.project.json`,
  returns the project root path
- `workspace::locate(path)` — walks up from `path` to find
  `.workspace.json`, returns the workspace root path

Both return a path, not a context. The caller constructs a `ProjectContext`
or `WorkspaceContext` from the result, choosing their own `ConfigMode` or
other settings.

```rust
let path = project::locate("./deeply/nested/dir")?;
let ctx = ProjectContext::new(path);
project::show(&ctx)?;
```

```rust
let path = workspace::locate("./some/member/project")?;
let ctx = WorkspaceContext::new(path);
workspace::build(&ctx, options)?;
```

### 2. No implicit locate

No library operation calls locate internally. Every operation still takes
an explicit context with an explicit path. Locate is opt-in — the caller
decides whether and when to search.

This preserves the predictability from ADR-0001: the library never walks
the filesystem unless the caller explicitly asks.

### 3. Locate fails if not found

If locate reaches the filesystem root without finding the target file, it
returns an error. There is no `Option`-style return — a missing project or
workspace at any ancestor is an error condition.

### 4. CLI uses locate internally

When `--project` or `--workspace` is omitted, the CLI calls the
corresponding locate function with CWD as input, then constructs a context
from the result. This replaces the CLI's own traversal implementation.

### 5. `project root` is replaced by `project locate`

The current `project root` command is renamed to `project locate`. The
operation is the same (find and print the project root path), but the name
is a proper verb and matches the library function it delegates to.

`workspace locate` is added as a new command.

## Binding Projections

**Rust:**

```rust
let path = project::locate(".")?;
let ctx = ProjectContext::new(path);
```

```rust
let path = workspace::locate(".")?;
let ctx = WorkspaceContext::new(path);
```

**Java:**

```java
String path = client.project().locate(".");
ProjectContext ctx = new ProjectContext(path);
```

```java
String path = client.workspace().locate(".");
WorkspaceContext ctx = new WorkspaceContext(path);
```

**JS/WASM:**

```ts
const path = await sysand.project.locate(".");
const ctx = new ProjectContext(path);
```

```ts
const path = await sysand.workspace.locate(".");
const ctx = new WorkspaceContext(path);
```

**Python:**

```python
path = sysand.project.locate(".")
ctx = ProjectContext(path)
```

```python
path = sysand.workspace.locate(".")
ctx = WorkspaceContext(path)
```

## CLI Commands

```
sysand project locate [--project <PATH>]
sysand workspace locate [--workspace <PATH>]
```

When `--project` or `--workspace` is provided, locate starts from that path
instead of CWD. This is useful for scripting.

## Return Type

Both locate operations return a `ScalarFieldResult<String>` containing the
absolute path to the root.

## What Changes from ADR-0001

| Before (ADR-0001)                    | After                                        |
| ------------------------------------ | -------------------------------------------- |
| Discovery is CLI-only                | Locate is a library operation, explicit       |
| CLI implements its own traversal     | CLI calls `project::locate` / `workspace::locate` |
| Library never walks the filesystem   | Library walks only when locate is called      |
| `project root` CLI command           | `project locate` CLI command                  |
| No workspace root command            | `workspace locate` CLI command                |

All other aspects of ADR-0001 remain unchanged: config is project-level
only, `ProjectContext` and `WorkspaceContext` are the shared inputs,
implicit behavior stays in the CLI.

## Consequences

- Library and binding users get first-class discovery without reimplementing
  traversal
- Locate returns a path, not a context — the caller retains full control
  over context construction
- No implicit discovery in any library operation — predictability preserved
- The CLI becomes even thinner: it delegates traversal to the library
- `locate` is a proper verb, consistent with ADR-0002's noun-verb grammar
- Both project and workspace have symmetrical locate commands
