# 0001: ProjectRead / ProjectMut Trait System

## What It Is

`ProjectRead` and `ProjectMut` are the central abstraction for accessing interchange
projects (the unit of packaging in SysML v2 / KerML). They define a uniform interface
over multiple storage backends — local files, ZIP archives, HTTP registries, git repos,
and in-memory representations.

## Core Trait Surface

### ProjectRead (3 required methods + ~10 default helpers)

| Method              | Purpose                                                               |
| ------------------- | --------------------------------------------------------------------- |
| `get_project()`     | Returns `(Option<InfoRaw>, Option<MetaRaw>)` — the two JSON documents |
| `read_source(path)` | Returns a `Read` impl for a source file                               |
| `sources(ctx)`      | Lists all source files in the project                                 |

Default helpers derive from these: `get_info()`, `get_meta()`, `name()`, `version()`,
`usage()`, `checksum()`, `canonical_meta()`, `checksum_*_hex()`, `is_definitely_invalid()`.

Associated types: `Error: ErrorBound`, `SourceReader<'a>: Read`.

### ProjectMut (3 required + 3 default helpers, extends ProjectRead)

| Method                                  | Purpose               |
| --------------------------------------- | --------------------- |
| `put_info(info, overwrite)`             | Write `.project.json` |
| `put_meta(meta, overwrite)`             | Write `.meta.json`    |
| `write_source(path, reader, overwrite)` | Write a source file   |

Default helper `put_project()` combines `put_info` + `put_meta`. Additional helpers:
`include_source()`, `exclude_source()`, `merge_index()`.

### ProjectReadAsync (mirror of ProjectRead)

Same 3 core methods with `_async` suffix, `SourceReader: AsyncRead + Unpin`.
Used by HTTP-based backends that are inherently async.

## Implementations (11 total)

### Concrete backends

| Type                           | Read | Mut | Async | Backend                            |
| ------------------------------ | ---- | --- | ----- | ---------------------------------- |
| `InMemoryProject`              | yes  | yes | —     | HashMap in memory                  |
| `LocalSrcProject`              | yes  | yes | —     | Filesystem (extracted source form) |
| `LocalKParProject`             | yes  | —   | —     | ZIP archive (.kpar file)           |
| `GixDownloadedProject`         | yes  | —   | —     | Git repo via gix                   |
| `NullProject`                  | yes  | —   | yes   | Never-type / empty                 |
| `ReqwestSrcProjectAsync`       | —    | —   | yes   | HTTP source fetch                  |
| `ReqwestKparDownloadedProject` | —    | —   | yes   | HTTP KPAR download                 |

### Wrappers and combinators

| Type                           | Purpose                                             |
| ------------------------------ | --------------------------------------------------- |
| `EditableProject<P>`           | Adds editable source tracking to any `ProjectRead`  |
| `CachedProject<Local, Remote>` | Local cache backed by remote source                 |
| `ProjectReference<P>`          | `Arc` wrapper for cloneability                      |
| `AnyProject<Policy>`           | **Derived enum** — 6 variants covering all backends |

### Sync/async bridges

| Type                                      | Direction                                        |
| ----------------------------------------- | ------------------------------------------------ |
| `AsAsyncProject<T: ProjectRead>`          | sync → async (trivial: just returns sync result) |
| `AsSyncProjectTokio<T: ProjectReadAsync>` | async → sync (uses `runtime.block_on()`)         |

## Derive Macros

`#[derive(ProjectRead)]` and `#[derive(ProjectMut)]` work on enums where each variant
holds exactly one field implementing the trait. They generate:

- An `<Enum>Error` enum wrapping each variant's error type
- An `<Enum>SourceReader` enum wrapping each variant's reader type
- `impl Read` for the reader enum (delegates to active variant)
- Trait impl that matches on self and delegates + wraps errors

Used by: `AnyProject<Policy>` (6 variants), `CombinedProjectStorage` in resolvers
(6 variants), `FileResolverProject` (2 variants).

## Where Methods Are Actually Called

### Commands using ProjectRead

- **`lock`** — `get_info()`, `get_meta()`, `checksum_canonical_hex()`, `sources(ctx)`
- **`info`** — `get_project()` to display metadata
- **`add`** — `get_info()` to read current usages before modifying
- **`include`** — `read_source()` twice (once for checksumming, once for symbol extraction)
- **`sources`** — `sources(ctx)` to list files

### Commands using ProjectMut

- **`add`** — `put_info()` to write updated usages
- **`include`** — `merge_index()` to update symbol index in metadata
- **`init`** — `put_info()` + `put_meta()` to create initial project files

### Utilities

- **`clone_project()`** — reads everything from source (`get_project()`, `read_source()`),
  writes everything to target (`put_project()`, `write_source()`). This is the core
  "copy a project between backends" operation used by environment sync.

## Complexity Observations

1. **Sync/async duality doubles the surface area** — every trait method exists twice,
   plus bridge wrappers that use `block_on()`.

2. **The derive macros exist solely to support `AnyProject`-style enums** — static
   dispatch over heterogeneous backends. Without the enum pattern, the macros aren't needed.

3. **Most commands only use 1–3 methods** — the full trait surface is much larger than
   what any single consumer needs.

4. **`SourceReader<'a>` GAT adds complexity** — each impl has a different reader type,
   forcing the macro to generate wrapper enums.

5. **The `ErrorBound` associated type propagates everywhere** — every function that
   touches a project must be generic over the error type or use a concrete impl.

6. **Only 2 types implement ProjectMut** (InMemory and LocalSrc) — the write side
   is much simpler than the read side.
