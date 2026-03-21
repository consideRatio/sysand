# 0006: Discovery, Config, and Library Boundaries

**Status: Distilled into ADRs 0001–0005**

## Decision Context

The CLI knows the current working directory. The library doesn't. We need to
decide where discovery and config loading live, and how they project across
binding surfaces.

This exploration is intended to become an ADR once finalized.

## Decision

### Discovery is CLI-only

Path traversal ("walk up to find `.project.json` or `.workspace.json`") is a
CLI convenience. The library takes explicit paths.

- **CLI**: discovers project/workspace from CWD, then passes concrete paths
- **Library**: takes a project path or workspace path directly, never traverses upward
- **Bindings**: callers provide paths explicitly, same as the library

This keeps the library predictable: "give me a path, I'll operate on what's there."

### Config loading is library-side, but overridable

The library automatically loads `sysand.toml` from the project root when given
a project path. But every operation allows overriding this:

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

The CLI maps its flags directly:

- (no flags) → `ConfigMode::Auto`
- `--config <PATH>` → `ConfigMode::File(path)`
- `--no-config` → `ConfigMode::None`

### Project actions carry config; workspace actions don't (yet)

- Every **project** operation's input includes `config: ConfigMode` (defaults to `Auto`)
- **Workspace** operations have no config field for now
- If workspace needs config later, adding the field with a default is non-breaking

Even if a project action doesn't currently use config, it accepts the field —
future config additions shouldn't require API changes.

## Projection Across Languages

### Rust

```rust
pub struct ProjectShowInput {
    pub project: Utf8PathBuf,
    pub config: ConfigMode,  // defaults to Auto via Default impl
}

// Caller:
project::show(ProjectShowInput {
    project: ".".into(),
    ..Default::default()
})?;

// With explicit config:
project::show(ProjectShowInput {
    project: ".".into(),
    config: ConfigMode::File("/path/to/sysand.toml".into()),
})?;
```

### Python

```python
# Auto (default) — keyword-only, not even visible in simple calls
sysand.project.show(project=".")

# Explicit config file
sysand.project.show(project=".", config="/path/to/sysand.toml")

# No config
sysand.project.show(project=".", config=False)
# or: config=ConfigMode.NONE — TBD which is more Pythonic
```

Python's keyword-only parameters make this zero-ceremony for the common case.

### Java

Java has no named parameters, so options objects carry config:

```java
// Minimal — auto config is the default
client.project().show(
    new ProjectShowOptions()
        .setProject(".")
);

// Explicit config
client.project().show(
    new ProjectShowOptions()
        .setProject(".")
        .setConfig(ConfigMode.file("/path/to/sysand.toml"))
);

// No config
client.project().show(
    new ProjectShowOptions()
        .setProject(".")
        .setConfig(ConfigMode.none())
);
```

The builder pattern absorbs config naturally. When doing many operations on the
same project, callers can construct a shared config and compose it:

```java
ConfigMode config = ConfigMode.file("/shared/sysand.toml");

client.project().info().name().get(
    new ProjectFieldGetOptions().setProject(".").setConfig(config)
);
client.project().info().version().get(
    new ProjectFieldGetOptions().setProject(".").setConfig(config)
);
```

### JS/WASM

```ts
// Auto (default) — config omitted
await sysand.project.show({ project: "." });

// Explicit config
await sysand.project.show({ project: ".", config: "/path/to/sysand.toml" });

// No config
await sysand.project.show({ project: ".", config: null });
```

Options objects are the natural JS pattern; config fits without friction.

## What the Library Loads at a Project Path

Given a project path, the library can find these without traversal:

- `<project>/.project.json` — project info
- `<project>/.meta.json` — project metadata
- `<project>/sysand.toml` — project config (unless `ConfigMode::None`)

All at the same level. No upward search needed.

## What Stays CLI-Only

- Walking up from CWD to find the nearest `.project.json`
- Walking up to find `.workspace.json`
- Resolving which `sysand.toml` to use when no explicit path is given
- Applying `--project` / `--workspace` defaults from CWD
- Terminal concerns: color, prompting, log level, output format

## Config Scope

### What config controls

Currently `sysand.toml` controls:

1. **Indexes** — additional package index URLs (with default/non-default
   distinction and priority ordering)
2. **Project source overrides** — mapping project identifiers (IRIs) to
   specific sources (local paths, remote URLs, etc.)

Authentication is a future concern, not yet implemented.

### Project-level config only

For the rework, there is only project-level config: `<project>/sysand.toml`.
No user-level config (`~/.config/sysand/...`), no merging.

The library reads one file from one known location. Simple.

User-level config (for auth credentials, personal default indexes, corporate
registries) can be added later as a separate layer. When that happens, merging
rules would be: user config provides defaults, project config overrides.
But that's a future concern — don't design for it now.

### No workspace-level config

Workspace actions don't consider config at the moment. If that changes in the
future, it's an additive decision.

## Open Questions (for ADR finalization)

1. **Config file name and location**: Is `<project>/sysand.toml` the right
   name and location? Or should it be something like `<project>/.sysand.toml`
   (hidden) or `<project>/sysand/config.toml` (nested)?

2. ~~**Config and resolve operations**~~ **Resolved**: `lookup` commands don't
   accept `--config`. They use explicit `--index`/`--default-index` flags only.
   No config fields exist that aren't already covered by lookup option flags.
