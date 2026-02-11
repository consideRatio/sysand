# Sysand Codebase Guide for AI Assistants

This document provides architectural insights and guidance for working with the Sysand codebase.

## Project Overview

Sysand is a package manager for SysML v2 and KerML (Systems Modeling Language), similar to pip, npm, or Maven. It manages model interchange projects and `.kpar` (KerML Project Archive) files.

**Current Version**: 0.0.9 (early preview)
**License**: MIT OR Apache-2.0 (dual-licensed)
**Maintainer**: Sensmetry
**Repository**: https://github.com/sensmetry/sysand
**Documentation**: https://docs.sysand.org

## Workspace Structure

This is a Rust workspace with 5 members:

```
sysand/
├── core/              # Core business logic library
├── sysand/           # CLI application
├── bindings/
│   ├── py/          # Python bindings (PyO3)
│   ├── js/          # JavaScript/WASM bindings (wasm-bindgen)
│   └── java/        # Java bindings (JNI)
├── docs/            # mdBook documentation
└── scripts/         # Build/release scripts
```

## Core Architecture

### Key Abstractions

1. **Environment Traits** (`core/src/env/mod.rs`)
   - `ReadEnvironment`: Read-only access to project storage
   - `ReadEnvironmentAsync`: Async version for I/O-bound operations
   - `WriteEnvironment`: Write access to project storage

   Implementations:
   - `local_directory`: Filesystem-based (default for CLI)
   - `memory`: In-memory (used in tests)
   - `reqwest_http`: HTTP-based remote environments
   - `null`: No-op implementation

2. **Project Abstractions** (`core/src/project/`)
   - `ProjectRead`: Read-only project access
   - `ProjectMut`: Mutable project access
   - Projects contain metadata, dependencies, and file contents
   - Can be serialized to `.kpar` files (ZIP archives)

3. **Workspace** (`core/src/workspace.rs`)
   - Manages multiple projects in a directory
   - Uses `.workspace.json` for configuration
   - Maps projects to IRIs (Internationalized Resource Identifiers)

4. **Lock Files** (`core/src/lock.rs`)
   - Dependency resolution and version pinning
   - Ensures reproducible builds
   - Similar to package-lock.json or Cargo.lock

5. **Configuration** (`core/src/config/`)
   - Hierarchical config system
   - Supports package indexes and sources
   - Can load from filesystem or be provided programmatically

### Command Implementation Pattern

All CLI commands follow this pattern:
1. Core logic in `core/src/commands/<command>.rs`
2. CLI wrapper in `sysand/src/commands/<command>.rs`
3. Integration tests in `sysand/tests/cli_<command>.rs`

Available commands:
- **init**: Initialize new project with metadata
- **add/remove**: Manage dependencies
- **include/exclude**: Control build inclusion
- **build**: Create `.kpar` interchange files
- **lock**: Generate/update lock files
- **sync**: Synchronize dependencies from lock file
- **env**: Manage environments (install/uninstall/list/sources)
- **info**: Display project information
- **sources**: Manage package sources
- **print-root**: Print project root path

### Feature Flags

The codebase uses Cargo features extensively:

- **std**: Standard library support (currently required)
- **filesystem**: Local filesystem operations
- **networking**: HTTP-based operations
- **python**: PyO3 Python bindings
- **js**: WebAssembly/JavaScript support

Platform-specific features:
- Windows/macOS: Uses `native-tls`
- Linux: Uses `rustls`

## Technology Stack

- **Rust Edition**: 2024
- **MSRV**: 1.85
- **Key Dependencies**:
  - `camino`: UTF-8 path handling (migrated from std::path)
  - `clap`: CLI argument parsing
  - `serde/serde_json`: Serialization
  - `tokio`: Async runtime
  - `reqwest`: HTTP client
  - `sha2`: Hashing for URIs
  - `zip`: Reading/writing .kpar files
  - `semver`: Semantic versioning
  - `url`: URL/URI parsing

## Important Files and Locations

### Core Library (`core/`)
- `core/src/lib.rs`: Main entry point
- `core/src/env/`: Environment abstraction
- `core/src/project/`: Project handling
- `core/src/commands/`: Command implementations
- `core/src/config/`: Configuration system
- `core/src/lock.rs`: Lock file handling
- `core/src/auth/`: Authentication (HTTP, bearer tokens)
- `core/src/init.rs`: Project initialization
- `core/src/stdlib.rs`: Known standard libraries

### CLI Application (`sysand/`)
- `sysand/src/lib.rs`: CLI entry point
- `sysand/src/cli.rs`: Clap argument definitions
- `sysand/src/commands/`: CLI command wrappers
- `sysand/tests/`: Integration tests

### Bindings
- `bindings/py/src/lib.rs`: Python API surface
- `bindings/js/src/lib.rs`: JavaScript/WASM API
- `bindings/java/src/lib.rs`: Java JNI interface

## Testing Strategy

1. **Unit Tests**: Inline in source files (`#[cfg(test)]`)
2. **Integration Tests**: `sysand/tests/` with extensive CLI testing
3. **Test Utilities**: `core/tests/` contains helpers
   - `filesystem_env.rs`: Filesystem-based test environments
   - `memory_env.rs`: In-memory test environments
   - `memory_init.rs`: Test initialization helpers

Tests use temporary directories and mock data in `sysand/tests/data/`.

## CI/CD Pipelines

GitHub Actions workflows (`.github/workflows/`):
- **test.yml**: Core Rust tests, clippy, rustfmt
- **python.yml**: Python bindings build and PyPI publishing
- **java.yml**: Java bindings build
- **js.yml**: JavaScript/WASM bindings build
- **deploy-mdbook.yml**: Documentation deployment

## Development Patterns

### Error Handling
- Uses `thiserror` for structured errors
- Error types are specific to each module
- Errors implement `Debug`, `Display`, and `Error` traits
- Many functions return `Result<T, SpecificError>`

### Path Handling
- **Always use `camino::Utf8Path` and `Utf8PathBuf`**
- The codebase recently migrated from `std::path`
- UTF-8 paths are validated and enforced

### Async/Sync
- Core uses sync APIs with optional async wrappers
- Async primarily used for HTTP operations
- Tokio runtime used where needed

### Serialization
- Uses `serde` with `camelCase` for JSON
- Python bindings use `pyo3`'s `FromPyObject`/`IntoPyObject`
- Configuration files are JSON

### Platform Compatibility
- Supports Linux, macOS, Windows
- Platform-specific code gated with `cfg` attributes
- Different TLS implementations per platform

## Common Tasks

### Adding a New Command

1. Add command logic to `core/src/commands/<name>.rs`
2. Export in `core/src/commands/mod.rs`
3. Add CLI wrapper to `sysand/src/commands/<name>.rs`
4. Add CLI args to `sysand/src/cli.rs`
5. Wire up in `sysand/src/lib.rs` main function
6. Add integration tests to `sysand/tests/cli_<name>.rs`
7. Add documentation to `docs/src/commands/<name>.md`
8. Update `docs/src/SUMMARY.md`

### Working with Environments

```rust
use sysand_core::env::local_directory::LocalDirectoryEnvironment;
use sysand_core::env::{ReadEnvironment, WriteEnvironment};

// Read from environment
let env = LocalDirectoryEnvironment::new(path)?;
let uris = env.uris()?;
let versions = env.versions("some-uri")?;
let project = env.get_project("uri", "version")?;

// Write to environment
let mut env = LocalDirectoryEnvironment::new(path)?;
env.put_project("uri", "version", project)?;
```

### Working with Projects

```rust
use sysand_core::project::{ProjectRead, ProjectMut};

// Read project metadata
let name = project.name();
let version = project.version();
let dependencies = project.dependencies();

// Modify project
project.set_name("new-name")?;
project.add_dependency("dep-uri", "1.0.0")?;

// Build to .kpar
let kpar_bytes = project.to_kpar()?;
```

## Authentication

Recent additions (v0.0.9):
- HTTP Basic authentication (`core/src/auth/`)
- Bearer token authentication
- Configurable per index/source

## Standard Library

Sysand includes knowledge of standard libraries (`core/src/stdlib.rs`):
- KerML Standard Library
- SysML v2 Standard Library
- These are pre-registered and discoverable

## Documentation

- User docs: `docs/` (mdBook) → https://docs.sysand.org
- Developer docs: `DEVELOPMENT.md`
- Contributing: `CONTRIBUTING.md`
- This file: `CLAUDE.md`

## Code Quality Tools

- **cargo-deny**: License and security checking (`deny.toml`)
- **clippy**: Linting
- **rustfmt**: Code formatting
- **renovate**: Automated dependency updates (`renovate.json5`)

## Tips for AI Assistants

1. **Always read before modifying**: Read existing implementations before making changes
2. **Follow existing patterns**: Command structure, error handling, testing patterns are consistent
3. **UTF-8 paths**: Use `camino` types, not `std::path`
4. **Feature flags**: Be aware of conditional compilation
5. **Platform compatibility**: Consider Windows/macOS/Linux differences
6. **Test coverage**: Add integration tests for CLI changes
7. **Documentation**: Update docs/ when changing user-facing behavior
8. **Licensing**: All code is dual MIT/Apache-2.0
9. **SPDX headers**: All files should have SPDX license headers
10. **Async sparingly**: Core is sync-first with async wrappers

## Recent Changes (v0.0.9)

- Added bearer token authentication
- Added HTTP basic authentication
- Consolidated docs into mdBook
- Migrated to UTF-8 paths (camino)
- Added panic hook for bug reporting
- Python PyPI publishing
- Version and license validation warnings

## Future Considerations

This is an early preview release. Interfaces are subject to change. When making changes, consider:
- Backward compatibility with existing `.kpar` files
- API stability for bindings (Python, Java, JS)
- Performance for large projects
- User experience in CLI

## Getting Help

- Issues: https://github.com/sensmetry/sysand/issues
- Docs: https://docs.sysand.org
- Development: See DEVELOPMENT.md
