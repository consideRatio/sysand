# Binding Architecture

How the Rust core exposes functionality to Java, JS/WASM, and Python.
Covers the facade pattern, storage backends, and per-surface binding
tools.

Sources: ADR-0010, ADR-0011

## Facade

The core library exports facade functions that accept trait objects
(`impl ProjectMut` / `impl ProjectRead`) and handle error mapping to
`SysandError`. Each facade function wraps one internal generic function:

```rust
pub fn project_init(
    project: &mut impl ProjectMut,
    opts: InitOptions,
) -> Result<(), SysandError> {
    do_init(project, opts).map_err(SysandError::from)
}
```

The facade is the single entry point for all binding surfaces. Command
logic and error mapping happen once here.

## Storage Backends

Each binding surface provides only project construction — how to obtain
a `ProjectMut` / `ProjectRead` instance. The facade does the rest.

| Surface             | Storage Type                 | Description            |
| ------------------- | ---------------------------- | ---------------------- |
| CLI / Java / Python | `LocalSrcProject`            | Filesystem-backed      |
| JS/WASM             | `ProjectLocalBrowserStorage` | Browser `localStorage` |

```rust
// Filesystem (CLI, Java, Python)
let mut project = LocalSrcProject::open(path)?;
facade::project_init(&mut project, opts)?;

// Browser (JS/WASM)
let mut project = ProjectLocalBrowserStorage::open(prefix, root_path)?;
facade::project_init(&mut project, opts)?;
```

## Binding Tools

Each non-Rust surface uses an existing Rust FFI crate. No new binding
tools are introduced.

| Surface | Tool         | Pattern                                     |
| ------- | ------------ | ------------------------------------------- |
| Java    | `jni` crate  | JNI with shared helpers for arg extraction  |
| Python  | PyO3         | Native submodules, kwargs, dunder methods   |
| JS/WASM | wasm-bindgen | Same thin-wrapper pattern as other surfaces |

Each binding is ~5-10 lines per command: extract arguments, construct
storage backend, call facade, convert error.

## Command Wrappers vs Storage Backends

Command wrappers are mechanical across all surfaces — they follow the
projection rules (`projection-rules.md`) and the binding generation
rules in `CLAUDE.md`. They are AI-generated from the facade.

The JS/WASM storage backend (`ProjectLocalBrowserStorage`,
`LocalBrowserStorageEnvironment`, `LocalStorageVFS` — ~390 lines) is
irreducible platform-specific code. It implements core traits
(`ProjectRead`, `ProjectMut`, `ReadEnvironment`, `WriteEnvironment`) on
browser `localStorage`. Changes to these traits may require
corresponding manual changes in the storage backend.

| Layer           | Mechanical? | Surfaces              | Maintained by           |
| --------------- | ----------- | --------------------- | ----------------------- |
| Command wrapper | Yes         | Java, Python, JS/WASM | AI-generated (ADR-0011) |
| Storage backend | No          | JS/WASM only          | Manual engineering      |

## Maintenance

All binding command wrappers across Java, Python, and JS/WASM are
AI-generated from the facade by applying the projection rules. When a
facade function is added or changed, the bindings across all three
surfaces are updated in the same change.

The binding generation rules live in `CLAUDE.md` — see ADR-0011 for
rationale.
