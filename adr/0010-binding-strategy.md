# ADR-0010: Binding Strategy

- **Status**: Accepted
- **Date**: 2026-03-28
- **Applies**: ADR-0005 (projection rules)
- **Informed by**: Exploration 0014 (binding strategy)

## Context

The project targets five binding surfaces: Rust CLI, Rust library, Java,
JS/WASM, and Python. Each non-Rust surface needs foreign-function
bindings to call the core Rust library.

The core library uses generics internally (e.g., `do_init<P:
ProjectMut>`, `do_add<P: ProjectMut>`). These generics are well-
motivated — there are 5 `ProjectRead` implementations, composable
resolvers, etc.

However, at the binding boundary, CLI, Java, and Python all
monomorphize every generic function with the **same concrete types**
(primarily `LocalSrcProject` for mutable operations,
`StandardResolver` for reads). Each binding independently writes
`do_init::<LocalSrcProject>(...)`. JS/WASM uses
`ProjectLocalBrowserStorage` instead, but calls the same generic
`do_init` with its own type.

The existing JS/WASM binding (~70 lines of command wrappers) confirms
this: the only per-surface difference is **project construction** —
how you obtain a `ProjectMut` instance. The command logic
(`do_init`, `do_add`, etc.) is identical across all surfaces.

This redundant monomorphization means each binding independently picks
concrete types, extracts arguments, and maps errors. The existing JNI
bindings are ~515 lines of boilerplate for a handful of commands — but
most of that pain comes from the missing shared facade, not from JNI
itself.

### UniFFI considered and rejected

Exploration 0014 evaluated replacing JNI with UniFFI (Mozilla's
multi-language FFI generator). UniFFI would eliminate JNI boilerplate
and generate Java bindings automatically.

However, with a facade in the core (decision 1 below), the per-command
JNI wrapper shrinks to ~5–10 lines: extract arguments via a shared
helper, call the facade function, let a shared `run` helper map
`SysandError → SysandException`. At that size, the remaining
boilerplate does not justify a new build dependency, code generation
step, and reduced control over the Java API shape.

UniFFI also does not support JS/WASM, so it could never unify all
surfaces. Keeping JNI + PyO3 + wasm-bindgen means one mental model
(hand-written thin wrappers over a shared facade) rather than two
(generated bindings for Java, hand-written for everything else).

## Decision

### 1. Facade in the Rust core, generic over storage

The core library exports facade functions that accept `impl ProjectMut`
/ `impl ProjectRead` and handle error mapping to `SysandError`:

```rust
// Facade — generic over storage, handles error mapping
pub fn project_init(
    project: &mut impl ProjectMut,
    opts: InitOptions,
) -> Result<(), SysandError> {
    do_init(project, opts).map_err(SysandError::from)
}
```

Each binding surface provides only project construction:

```rust
// CLI / Java / Python — filesystem
let mut project = LocalSrcProject::open(path)?;
facade::project_init(&mut project, opts)?;

// JS/WASM — browser localStorage
let mut project = ProjectLocalBrowserStorage::open(prefix, root_path)?;
facade::project_init(&mut project, opts)?;
```

The command logic and error mapping happen once in the facade. The
bindings are thin wrappers that construct the appropriate storage
backend and call through.

### 2. Keep existing binding tools

- **JNI** (jni crate) for Java — with shared helpers for argument
  extraction and `SysandError → SysandException` mapping
- **PyO3** for Python — native submodules, kwargs, dunder methods
- **wasm-bindgen** for JS/WASM — same thin-wrapper pattern as the
  other surfaces

No new binding tools introduced.

### End-to-end example: `project init`

**Core internals** (generic, not exposed across FFI):

```rust
fn do_init<P: ProjectMut>(
    project: &mut P, opts: InitOptions,
) -> Result<(), InitError<P::Error>> { /* ... */ }
```

**Facade** (in core lib, the shared entry point):

```rust
pub fn project_init(
    project: &mut impl ProjectMut, opts: InitOptions,
) -> Result<(), SysandError> {
    do_init(project, opts).map_err(SysandError::from)
}
```

**CLI** (Rust):

```rust
let mut project = LocalSrcProject::open(&path)?;
facade::project_init(&mut project, opts)?;
```

**Python binding** (PyO3):

```rust
#[pyfunction]
#[pyo3(signature = (path, /, *, name, version, publisher=None, license=None))]
fn init(
    path: String, name: String, version: String,
    publisher: Option<String>, license: Option<String>,
) -> PyResult<()> {
    let mut project = LocalSrcProject::open(&path).map_err(into_py_err)?;
    let opts = InitOptions { name, version, publisher, license };
    facade::project_init(&mut project, opts).map_err(into_py_err)
}
```

End user calls: `sysand.project.init("/tmp/proj", name="foo", version="1.0.0")`

**Java binding** (JNI):

```rust
#[unsafe(no_mangle)]
pub extern "system" fn Java_com_sensmetry_sysand_Project_init<'local>(
    mut env: JNIEnv<'local>, _class: JClass<'local>,
    path: JString<'local>, options: JObject<'local>,
) {
    sysand_jni::run(&mut env, |env| {
        let path = env.get_string_required(&path)?;
        let opts = InitOptions::from_jobject(env, &options)?;
        let mut project = LocalSrcProject::open(&path)?;
        facade::project_init(&mut project, opts)
    });
}
```

End user calls: `client.project().init("/tmp/proj", options)`

**JS/WASM binding** (wasm-bindgen):

```rust
#[wasm_bindgen(js_name = projectInit)]
pub fn project_init_js(
    prefix: &str, root_path: &str,
    name: String, version: String,
    publisher: Option<String>, license: Option<String>,
) -> Result<(), JsValue> {
    let mut project = ProjectLocalBrowserStorage::open(prefix, root_path)
        .map_err(|e| JsValue::from_str(&e.to_string()))?;
    let opts = InitOptions { name, version, publisher, license };
    facade::project_init(&mut project, opts)
        .map_err(|e| JsValue::from_str(&e.to_string()))
}
```

End user calls: `sysand.project.init(prefix, rootPath, "foo", "1.0.0")`

Each binding is ~5–10 lines. The only variation is project construction
(filesystem vs browser storage) and error conversion (PyResult vs
SysandException vs JsValue).

## Consequences

1. **One facade, all surfaces.** Adding a command means writing one
   facade function. CLI, Java, Python, and JS/WASM all call it — they
   differ only in how they construct the storage backend.

2. **JS/WASM is no longer a special case for commands.** The browser
   storage backend (~390 lines) is irreducible platform-specific code,
   but command wrappers follow the same mechanical pattern as Java and
   Python: construct project, call facade, map error to `JsValue`.

3. **Bindings become truly thin.** Each binding is ~5–10 lines per
   command: argument conversion, project construction, facade call.

4. **JNI becomes manageable.** The existing ~515 lines of JNI
   boilerplate was largely due to each function independently wiring
   generics and mapping errors. With the facade and shared helpers,
   the per-command cost drops to the same order as PyO3.

5. **No new dependencies.** PyO3, jni, and wasm-bindgen are already
   in the dependency tree. No build-time code generation added.

6. **Uniform approach.** All binding surfaces follow the same pattern:
   construct storage, call facade, convert errors. One mental model.

7. **Generic internals unchanged.** The core library's trait-based
   architecture stays intact for internal composition, testing with
   `InMemoryProject`, and future storage backends.
