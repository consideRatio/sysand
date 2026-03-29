# Binding Architecture

How the Rust core exposes functionality to Java, JS/WASM, and Python.
Covers the facade pattern, storage backends, and per-surface binding
tools.

## Facade

The core library exports facade functions that accept trait objects
(`impl ProjectMut` / `impl ProjectRead`) and handle error mapping to
`SysandError`. Each facade function wraps one internal generic function.
See `crate-structure.md` for the module layout and migration mapping.

```rust
pub fn init(
    project: &mut impl ProjectMut,
    opts: InitOptions,
) -> Result<(), SysandError> {
    internal::commands::init::do_init(project, opts)
        .map_err(SysandError::from)
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
sysand_core::init(&mut project, opts)?;

// Browser (JS/WASM)
let mut project = ProjectLocalBrowserStorage::open(prefix, root_path)?;
sysand_core::init(&mut project, opts)?;
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

| Layer           | Mechanical? | Surfaces              | Maintained by      |
| --------------- | ----------- | --------------------- | ------------------ |
| Command wrapper | Yes         | Java, Python, JS/WASM | AI-generated       |
| Storage backend | No          | JS/WASM only          | Manual engineering |

## Maintenance

All binding command wrappers across Java, Python, and JS/WASM are
AI-generated from the facade by applying the projection rules. When a
facade function is added or changed, the bindings across all three
surfaces are updated in the same change.

The binding generation rules live in `CLAUDE.md`.

## Rationale

**Why UniFFI was rejected.** UniFFI auto-generates bindings from a
single definition, eliminating JNI boilerplate and giving Kotlin/Swift
for free. However, the facade pattern already reduces per-command
binding code to ~5-10 lines — the main pain point UniFFI solves
(verbose JNI) is already addressed. UniFFI also lacks generics across
FFI, has limited async support, requires a build-time code generation
step, and gives less control over language-specific types and
namespace mapping.

**Why facade + thin wrappers.** The facade centralizes command logic
and error mapping in one place. Each binding surface only needs to:
construct storage, call the facade function, convert the error. This
pattern makes each command wrapper mechanical and uniform — the same
~5-10 line structure everywhere.

**Why AI-generated bindings over build-time codegen.** With the facade
and projection rules, binding wrappers are fully mechanical — there
is no design judgment involved. Rather than building and maintaining
codegen tooling, the projection rules in `CLAUDE.md` are sufficient
for Claude to generate correct wrappers. This avoids a build
dependency, keeps the binding code readable and editable, and makes
it trivial to update all surfaces when a facade function changes.

**Why storage backends are manual.** The JS/WASM storage backend
(~390 lines) implements core traits (`ProjectRead`, `ProjectMut`,
`ReadEnvironment`, `WriteEnvironment`) on browser `localStorage`.
This is irreducible platform-specific code — it bridges two different
storage models. Changes to core traits may require corresponding
manual changes here. This is fundamentally different from command
wrappers, which are mechanical.
