# Discovery and Config

Sources: ADR-0001, ADR-0006

## Locate

The library provides `project::locate(path)` and
`workspace::locate(path)` — explicit operations that walk up from a
given path to find `.project.json` or `.workspace.json` respectively.

- Returns a path, not a context
- Errors if the filesystem root is reached without finding the target
- The caller constructs a `ProjectContext` or `WorkspaceContext` from
  the result

```rust
let path = project::locate("./deeply/nested/dir")?;
let ctx = ProjectContext::new(path);
```

Implicit locate (using CWD when `--project`/`--workspace` is omitted)
is CLI-only behavior.

## ProjectContext

Shared input for all project operations. Carries a path and config mode.

```rust
pub struct ProjectContext {
    pub path: Utf8PathBuf,
    pub config: ConfigMode,  // defaults to Auto
}
```

## WorkspaceContext

Shared input for workspace operations. Just a path.

```rust
pub struct WorkspaceContext {
    pub path: Utf8PathBuf,
}
```

## Config

One config file: `<project>/sysand.toml`. No user-level config, no
workspace-level config, no config merging.

Config controls:
- Package index URLs (with default/non-default distinction and priority)
- Project source overrides (mapping IRIs to specific sources)

Loading is automatic but overridable:

```rust
pub enum ConfigMode {
    Auto,              // Load sysand.toml from project root (default)
    File(Utf8PathBuf), // Use this specific config file
    None,              // Don't load any config
}
```

CLI mapping:

| CLI | ConfigMode |
| --- | --- |
| (no flag) | `Auto` |
| `--config auto` | `Auto` |
| `--config none` | `None` |
| `--config <PATH>` | `File(path)` |

## What the Library Reads

Given a project path:
- `<project>/.project.json` — project info
- `<project>/.meta.json` — project metadata
- `<project>/sysand.toml` — project config (unless `ConfigMode::None`)

Given a workspace path:
- `<workspace>/.workspace.json` — workspace info

## What Stays CLI-Only

- Implicit locate from CWD when `--project`/`--workspace` is omitted
- Terminal concerns: color, prompting, log level, output format
