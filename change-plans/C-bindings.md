# C. Binding Overhaul

Restructure all three binding surfaces (Java, Python, JS/WASM) to
match the spec's namespace structure and use the new facade + types.

Depends on: A (error model), B (facade restructure).

## Current State

### Java

**API shape:** Static methods on `Sysand` class.

```java
Sysand.init(name, publisher, version, license, path)
Sysand.env(path)
Sysand.infoPath(path)
Sysand.info(uri, relativeFileRoot, indexUrl)
Sysand.buildProject(outputPath, projectPath, compression)
Sysand.buildWorkspace(outputPath, workspacePath, compression)
Sysand.workspaceProjectPaths(workspacePath)
Sysand.setProjectIndex(projectPath, index)
Sysand.defaultEnvName()
```

**Rust side:** 9 JNI functions with flat params, per-function error
mapping to 10 custom exception classes.

**Error mapping:** ~100 lines of match arms per function mapping
internal errors to `ExceptionKind` variants.

### Python

**API shape:** Flat functions with `do_` prefix.

```python
_sysand_core.do_init_py_local_file(name, publisher, version, path, license)
_sysand_core.do_env_py_local_dir(path)
_sysand_core.do_info_py_path(path)
_sysand_core.do_info_py(uri, relative_file_root, index_urls)
_sysand_core.do_build_py(output_path, project_path, compression)
_sysand_core.do_sources_env_py(env_path, iri, version, include_deps, include_std)
_sysand_core.do_sources_project_py(path, include_deps, env_path, include_std)
_sysand_core.do_add_py(path, iri, version)
_sysand_core.do_remove_py(path, iri)
_sysand_core.do_include_py(path, src_path, compute_checksum, index_symbols, force_format)
_sysand_core.do_exclude_py(path, src_path)
_sysand_core.do_env_install_path_py(env_path, iri, location)
```

Plus `_run_cli()` for CLI access.

**Error mapping:** Per-function match mapping to stdlib exceptions
(`PyValueError`, `PyFileExistsError`, `PyIOError`, `PyRuntimeError`).

### JS/WASM

**API shape:** Flat functions with storage-specific names.

```javascript
initLogger();
ensureDebugHook();
clearLocalStorage(prefix);
doInitJsLocalStorage(name, publisher, version, prefix, rootPath, license);
doEnvJsLocalStorage(prefix, rootPath);
```

**Error mapping:** All errors → `JsValue::from_str(&e.to_string())`.

Only 2 commands are bound (init, env create). No build, lock, sync,
usage, source commands.

## Target State

### Java

```java
// Namespaced via accessor chain
SysandClient client = new SysandClient();

client.init(path, new InitOptions.Builder()
    .name("sensors")
    .version("1.0.0")
    .build());

client.build(ctx, new BuildOptions.Builder()
    .compression(Compression.DEFLATED)
    .build());

client.source().add(ctx, paths, new SourceAddOptions.Builder()
    .checksum(ChecksumMode.SHA256)
    .build());

client.usage().add(ctx, iri, new UsageAddOptions.Builder()
    .update(UpdateMode.SYNC)
    .build());

client.locate(path);

client.lock().update(ctx, opts);
client.env().sync(ctx, opts);
client.env().install(ctx, iri, opts);
client.env().uninstall(ctx, iri, opts);
client.env().list(ctx, opts);

client.workspace().build(ctx, opts);
client.workspace().locate(path);
```

Single exception: `SysandException` with `ErrorCode getCode()`.

### Python

```python
import sysand

sysand.init(path, name="sensors", version="1.0.0")
sysand.build(ctx, compression="deflated")
sysand.locate(path)
sysand.clone(locator, target="/out")

sysand.source.add(ctx, paths, checksum="sha256")
sysand.source.remove(ctx, paths)

sysand.usage.add(ctx, iri, version_req="^1.0", update=UpdateMode.SYNC)
sysand.usage.remove(ctx, iri)

sysand.lock.update(ctx, include_std=True)
sysand.env.sync(ctx)
sysand.env.install(ctx, iri)
sysand.env.uninstall(ctx, iri)
sysand.env.list(ctx)

sysand.workspace.build(ctx)
sysand.workspace.locate(path)
```

Single exception: `SysandError` with `code: ErrorCode` enum.

### JS/WASM

```typescript
import * as sysand from "sysand";

await sysand.init(path, { name: "sensors", version: "1.0.0" });
await sysand.build(ctx, { compression: "deflated" });
await sysand.locate(path);

await sysand.source.add(ctx, paths, { checksum: "sha256" });
await sysand.usage.add(ctx, iri, { update: "sync" });
await sysand.lock.update(ctx);
await sysand.env.sync(ctx);
await sysand.env.install(ctx, iri);
await sysand.env.list(ctx);

await sysand.workspace.build(ctx);
```

All functions return `Promise`. Plain objects everywhere (no classes
to `.free()`). TypeScript `.d.ts` for type safety. Thrown errors have
`{ code: string, message: string, context?: string }`.

## Steps

### Step 1: Java binding

**Rust side (`bindings/java/src/lib.rs`):**

Replace 9 JNI functions with new ones matching the namespace structure.
Each function:

1. Extract args from JNI (using existing helpers)
2. Build options struct from JObject fields
3. Call facade function (e.g., `sysand_core::init(...)`)
4. Convert `SysandError` → `SysandException` via single helper

```rust
fn throw_sysand_error(env: &mut JNIEnv, err: SysandError) {
    let code_str = err.code.as_str(); // e.g., "version-invalid"
    // Construct SysandException with code, message, context
    env.throw_new("com/sensmetry/sysand/exceptions/SysandException", &format!("{}: {}", code_str, err.message));
}

// Shared runner that catches SysandError
fn run<F, R>(env: &mut JNIEnv, f: F) -> R
where F: FnOnce() -> Result<R, SysandError> {
    match f() {
        Ok(v) => v,
        Err(e) => { throw_sysand_error(env, e); R::default() }
    }
}
```

Each command wrapper becomes ~5-10 lines:

```rust
#[no_mangle]
pub extern "system" fn Java_com_sensmetry_sysand_SysandClient_init(
    mut env: JNIEnv, _class: JClass, path: JString, opts: JObject,
) {
    run(&mut env, || {
        let path = env.get_string_required(&path)?;
        let opts = InitOptions::from_jobject(&mut env, &opts)?;
        let mut project = LocalSrcProject::open(&path)?;
        sysand_core::init(&mut project, opts)
    });
}
```

**Java side:**

Replace `Sysand.java` (static methods) with:

- `SysandClient.java` — root methods: `init()`, `locate()`, `clone()`, `build()`
- `SysandClient.Source` — inner class returned by `source()`: `add()`, `remove()`
- `SysandClient.Usage` — inner class returned by `usage()`: `add()`, `remove()`
- `SysandClient.Lock` — inner class returned by `lock()`: `update()`
- `SysandClient.Env` — inner class returned by `env()`: `create()`, `sync()`, `install()`, `uninstall()`, `list()`
- `SysandClient.Workspace` — inner class returned by `workspace()`: `locate()`, `build()`

Options classes with builder pattern:

- `InitOptions`, `BuildOptions`, `UsageAddOptions`, etc.

Delete 9 custom exception classes. Keep single `SysandException` with
`ErrorCode` enum.

**Tests:** Update `BasicTest.java` to use new API shape.

### Step 2: Python binding

**Rust side (`bindings/py/src/lib.rs`):**

Replace flat functions with PyO3 submodules:

```rust
#[pymodule]
fn sysand(m: &Bound<'_, PyModule>) -> PyResult<()> {
    // Root functions
    m.add_function(wrap_pyfunction!(init, m)?)?;
    m.add_function(wrap_pyfunction!(locate, m)?)?;
    m.add_function(wrap_pyfunction!(clone_project, m)?)?;
    m.add_function(wrap_pyfunction!(build, m)?)?;

    // Submodules
    let source = PyModule::new(m.py(), "source")?;
    source.add_function(wrap_pyfunction!(source_add, &source)?)?;
    source.add_function(wrap_pyfunction!(source_remove, &source)?)?;
    m.add_submodule(&source)?;

    let usage = PyModule::new(m.py(), "usage")?;
    usage.add_function(wrap_pyfunction!(usage_add, &usage)?)?;
    usage.add_function(wrap_pyfunction!(usage_remove, &usage)?)?;
    m.add_submodule(&usage)?;

    // ... lock, env, workspace submodules
    Ok(())
}
```

Each function: unpack kwargs → build options struct → call facade →
convert error:

```rust
#[pyfunction]
#[pyo3(signature = (path, *, name=None, publisher=None, version=None, license=None, allow_non_spdx=false))]
fn init(path: String, name: Option<String>, publisher: Option<String>,
        version: Option<String>, license: Option<String>,
        allow_non_spdx: bool) -> PyResult<()> {
    let mut project = LocalSrcProject::open(&path).map_err(into_py_err)?;
    sysand_core::init(&mut project, InitOptions {
        name, publisher, version, license, allow_non_spdx,
    }).map_err(into_py_err)
}
```

Single error converter:

```rust
fn into_py_err(err: SysandError) -> PyErr {
    SysandErrorType::new_err((err.code.as_str().to_string(), err.message, err.context))
}
```

Register `SysandError` as a Python exception class with `code` attribute
as `ErrorCode` enum.

**Remove:** `do_info_py`, `do_info_py_path`, `do_sources_env_py`,
`do_sources_project_py` (removed commands). `_run_cli()` stays as
escape hatch.

**Tests:** Update `test_basic.py` to use new API shape.

### Step 3: JS/WASM binding

**Rust side (`bindings/js/src/lib.rs`):**

Major expansion — currently only 2 commands bound, need all of them.

Use `serde-wasm-bindgen` for plain object conversion:

```rust
#[wasm_bindgen]
pub async fn init(path: JsValue, opts: JsValue) -> Result<(), JsValue> {
    let path: String = serde_wasm_bindgen::from_value(path)?;
    let opts: InitOptions = serde_wasm_bindgen::from_value(opts)?;
    let mut project = ProjectLocalBrowserStorage::open(&path)?;
    sysand_core::init(&mut project, opts)
        .map_err(|e| sysand_error_to_js(e))
}
```

Namespace via JS module objects (wasm-bindgen doesn't support nested
modules natively, so use `js_namespace` or post-build JS wrapper):

```javascript
// Generated wrapper (or hand-written thin shim)
export const source = {
  add: _source_add,
  remove: _source_remove,
};
export const usage = {
  add: _usage_add,
  remove: _usage_remove,
};
// etc.
```

Error conversion:

```rust
fn sysand_error_to_js(err: SysandError) -> JsValue {
    let obj = js_sys::Object::new();
    js_sys::Reflect::set(&obj, &"code".into(), &err.code.as_kebab_str().into());
    js_sys::Reflect::set(&obj, &"message".into(), &err.message.into());
    if let Some(ctx) = err.context {
        js_sys::Reflect::set(&obj, &"context".into(), &ctx.into());
    }
    obj.into()
}
```

**TypeScript definitions:** Generate or hand-write `.d.ts`:

```typescript
export function init(path: string, opts?: InitOptions): Promise<void>;
export function build(
  ctx: ProjectContext,
  opts?: BuildOptions,
): Promise<BuildOutput>;
export function locate(path: string): Promise<string>;

export namespace source {
  function add(
    ctx: ProjectContext,
    paths: string[],
    opts?: SourceAddOptions,
  ): Promise<void>;
  function remove(ctx: ProjectContext, paths: string[]): Promise<void>;
}
// etc.
```

**Storage backend:** `ProjectLocalBrowserStorage` and
`LocalBrowserStorageEnvironment` (~390 lines) remain unchanged — they
implement internal traits, not facade types.

**Tests:** Update `browser_basic.spec.js` to use new API shape. Add
tests for newly-bound commands.

### Step 4: Remove old code

- Java: Delete `Sysand.java`, 9 exception classes, old JNI functions
- Python: Delete `do_*` functions, old error mapping
- JS: Delete `doInitJsLocalStorage`, `doEnvJsLocalStorage`

## Per-Surface Line Counts

| Surface | Current (Rust) | Current (Host) | Target (Rust) | Target (Host)     |
| ------- | -------------- | -------------- | ------------- | ----------------- |
| Java    | ~400 lines     | ~600 lines     | ~200 lines    | ~400 lines        |
| Python  | ~600 lines     | ~50 lines      | ~300 lines    | ~50 lines         |
| JS/WASM | ~70 lines      | ~0 lines       | ~300 lines    | ~50 lines (.d.ts) |

Net: Java and Python shrink (simpler error handling, facade does the
work). JS grows (many more commands bound).

## Risks

- **Java accessor chain:** `client.source().add()` requires inner
  classes or accessor objects that hold a reference back to the native
  layer. Not complex, but more Java code than static methods.

- **JS/WASM namespace:** wasm-bindgen doesn't natively support nested
  module exports. May need a thin JS wrapper that re-exports flat
  wasm-bindgen functions into namespace objects.

- **Python submodule import:** PyO3 submodules need `sys.modules`
  registration for `from sysand.source import add` to work. Known
  pattern but requires explicit setup.
