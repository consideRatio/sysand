# Architecture

Internal architecture of sysand. For the public API, see `spec/public-api.md`.
This document describes the internals — crate boundaries, traits, the solver
pipeline, and how data flows through the system.

Based on the reference implementation in `reference/`.

## Crate Structure

```
sysand (binary)         Thin CLI adapter (clap → core library calls)
  └─> sysand-core       All logic: resolution, solving, project I/O, config
       └─> sysand-macros   Derive macros for ProjectRead/ProjectMut traits
```

The CLI crate owns the tokio runtime, auth policy, and terminal concerns.
Everything else lives in `sysand-core`.

## Core Module Layout

```
core/src/
├── resolve/       IRI → project resolution (trait + implementations)
├── solve/         Dependency solving (PubGrub integration)
├── project/       Project storage (trait + implementations)
├── env/           Environment storage (trait + implementations)
├── commands/      High-level operations (lock, sync, install, build, include)
├── config/        sysand.toml parsing and loading
├── symbols/       KerML/SysML symbol extraction (lexer + parser)
├── model.rs       Data types: project info, metadata, usages, checksums
├── lock.rs        Lockfile structure, parsing, Source enum
├── context.rs     ProjectContext for filesystem discovery
├── discover.rs    Workspace/.project.json/.meta.json discovery
├── auth.rs        HTTP authentication trait
└── stdlib.rs      Built-in standard library definitions
```

## Core Traits

Three trait families define the internal abstraction boundaries. Each has
sync and async variants bridged by adapter wrappers.

### ResolveRead — IRI to project resolution

```rust
trait ResolveRead {
    type Error;
    type ProjectStorage: ProjectRead;
    type ResolvedStorages: IntoIterator<Item = Self::ProjectStorage>;

    fn resolve_read(&self, uri: &Iri) -> Result<ResolutionOutcome<Self::ResolvedStorages>, Self::Error>;
}
```

Given an IRI, returns zero or more project storages that match. The solver
calls this repeatedly to discover candidates.

**Implementations:**

| Resolver | Purpose |
| --- | --- |
| `FileResolver` | Local filesystem (`file://` URLs) |
| `EnvResolver<Env>` | Wraps a `ReadEnvironment` (local cache or HTTP index) |
| `GitResolver` | Git repository cloning |
| `HTTPResolverAsync` | Direct HTTP(S) fetch |
| `StandardResolver` | Production composition: file → local env → remote → index |
| `PriorityResolver` | Config overrides: tries high-priority resolver first |
| `SequentialResolver` | Tries multiple resolvers in order |
| `CombinedResolver` | Orchestrates file → remote → local → index |
| `MemoryResolver` | In-memory (tests) |

**StandardResolver composition:**

```
1. FileResolver          (file:// URLs, relative paths)
2. LocalEnvResolver      (sysand_env/ cache)
3. RemoteResolver        (HTTP + Git for direct URLs)
4. RemoteIndexResolver   (HTTP indexes, tried sequentially)
```

Wrapped in `PriorityResolver` when config source overrides are present.

### ProjectRead — reading project data

```rust
trait ProjectRead {
    type Error;
    fn get_project(&self) -> Result<(Option<InfoRaw>, Option<MetadataRaw>), Self::Error>;
    fn read_source(&self, path: &Utf8UnixPath) -> Self::SourceReader<'_>;
    fn sources(&self, ctx: &ProjectContext) -> Vec<Source>;
    fn checksum_canonical_hex(&self) -> Result<String, Self::Error>;
}

trait ProjectMut: ProjectRead {
    fn put_info(&mut self, info: &InfoRaw) -> Result<(), Self::Error>;
    fn put_meta(&mut self, meta: &MetadataRaw) -> Result<(), Self::Error>;
    fn put_source(&mut self, path: &Utf8UnixPath, reader: impl Read) -> Result<(), Self::Error>;
    fn include_symbols(&mut self, path: &Utf8UnixPath, symbols: HashMap<String, ...>);
    // ...
}
```

**Implementations:**

| Type | Source |
| --- | --- |
| `LocalSrcProject` | Local directory with `.project.json` + `.meta.json` |
| `LocalKParProject` | Local `.kpar` ZIP archive |
| `EditableProject<P>` | Wraps another project for live editable tracking |
| `ReqwestKparDownloadedProject` | HTTP → download → unzip KPAR |
| `ReqwestSrcProjectAsync` | HTTP → fetch project/meta JSON |
| `GixDownloadedProject` | Git clone → extract project |
| `AnyProject` | Enum dispatch over all the above |

### ReadEnvironment — project storage

```rust
trait ReadEnvironment {
    fn uris(&self) -> Self::UriIter;
    fn versions(&self, uri: &Iri) -> Self::VersionIter;
    fn get_project(&self, uri: &Iri, version: &Version) -> Self::InterchangeProjectRead;
}

trait WriteEnvironment {
    fn put_project(&mut self, uri: &Iri, version: &Version, project: impl ProjectRead);
}
```

**Implementations:**

| Type | Purpose |
| --- | --- |
| `LocalDirectoryEnvironment` | `sysand_env/` on disk |
| `HTTPEnvironmentAsync` | HTTP index (e.g., beta.sysand.org) |
| `MemoryEnvironment` | In-memory (tests) |

**Local environment storage layout:**

```
sysand_env/
├── entries.txt              All IRIs, one per line
└── {sha256(iri)}/
    ├── versions.txt         Available versions
    └── {version}/
        ├── .project.json
        ├── .meta.json
        └── sources/
```

## Solver Pipeline

The solver uses PubGrub for version resolution.

```rust
struct ProjectSolver<R: ResolveRead> {
    resolver: R,
    candidates: RefCell<HashMap<Iri, Vec<ProjectIndex>>>,  // lazy cache
}

impl DependencyProvider for ProjectSolver<R> {
    fn choose_version(&self, ...) -> ...;
    fn get_dependencies(&self, ...) -> ...;
    fn prioritize(&self, ...) -> ...;
}
```

**Flow:**

```
usages from .project.json
    │
    ▼
ProjectSolver::new(resolver)
    │
    ▼
pubgrub::resolve()
    │  for each package:
    │    1. resolver.resolve_read(iri) → candidate projects
    │    2. filter to projects with valid info + meta
    │    3. cache by IRI
    │    4. choose version (highest semver matching constraint)
    │    5. read chosen version's usages → transitive deps
    │
    ▼
Resolution map: IRI → (version, project storage)
    │
    ▼
Build Lock { projects: [...] }  with sources, exports, checksums
    │
    ▼
Write sysand-lock.toml
```

## Symbol Parser

Lightweight syntactic parser for extracting top-level symbol names from
`.sysml` and `.kerml` files. Used by `project source add` to populate
the `index` field in `.meta.json`.

```
symbols/
├── lex.rs     Logos-based lexer: keywords, identifiers, strings, comments
└── mod.rs     Parser: walks token stream, extracts declarations
```

The parser recognizes KerML/SysML keywords (package, class, attribute, etc.)
and extracts symbol names including short-name aliases. It does not build an
AST or perform semantic analysis.

## Source Types

The `Source` enum represents where a project can be fetched from:

```rust
enum Source {
    Editable   { editable: Utf8UnixPathBuf },
    LocalSrc   { src_path: Utf8UnixPathBuf },
    LocalKpar  { kpar_path: Utf8UnixPathBuf },
    RemoteKpar { remote_kpar: Url, remote_kpar_size: Option<u64> },
    RemoteSrc  { remote_src: Url },
    RemoteGit  { repo: Url, subdir: Option<Utf8UnixPathBuf> },
}
```

Each variant dispatches to the corresponding `ProjectRead` implementation
via `AnyProject`.

## Config

```rust
struct Config {
    indexes: Vec<Index>,
    projects: Vec<ConfigProject>,  // source overrides
}
```

Loaded from `sysand.toml` in the project root. Only the root project's
config is consulted during resolution. Config source overrides become the
high-priority resolver in `PriorityResolver`.

## Async/Sync Bridge

Each core trait has sync and async variants with adapter wrappers:

- `AsAsyncProject<T>` — wraps sync `ProjectRead` as async (trivial)
- `AsSyncProjectTokio<T>` — wraps async as sync via `runtime.block_on()`

Same pattern for `ResolveRead`/`ResolveReadAsync` and
`ReadEnvironment`/`ReadEnvironmentAsync`. This allows mixing sync and
async components in the resolver chain.
