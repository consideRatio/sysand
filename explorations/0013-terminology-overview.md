# Exploration 0013: Terminology Overview

**Status: Open**

## Purpose

Several terms have been used loosely or inconsistently across
explorations and discussions. This document pins them down.

## Package Identity

- **IRI** — Internationalized Resource Identifier. The unique identity
  of a package. Example: `urn:kpar:sensors`. An IRI says *what* a
  package is, not *where* it is. A project can have multiple IRIs
  (aliases). Defined in `.project.json` as `identifiers`.

- **Name** — Human-readable label for a package. Example: `"sensors"`.
  Not unique. Not used for resolution. Defined in `.project.json` as
  `name`.

- **Version** — Semver string. Example: `"2.1.0"`. Combined with an
  IRI, uniquely identifies a specific release. Required field in
  `.project.json` (ADR-0007).

- **Version constraint** — Semver range expression. Example: `"^2.0"`,
  `">=1.5, <3"`, `"=2.1.0"`. Used in usages and in lookup queries
  to narrow the version space.

## Project Structure

- **Interchange project** — The unit of packaging in KerML (clause
  10.3). A directory containing `.project.json`, `.meta.json`, and
  source files. This is what sysand manages.

- **`.project.json`** — Project info: name, version, usages,
  publisher, description, license, maintainers, topics, website.

- **`.meta.json`** — Project metadata: created date, symbol index,
  checksums, metamodel, includes-derived, includes-implied.

- **KPAR** — KerML Project Archive. A zip-based archive containing
  an interchange project. The distributable artifact produced by
  `project build`.

- **Workspace** — A collection of related projects. Defined by
  `.workspace.json` which lists member projects with their paths
  and IRIs.

## Dependency Model

- **Usage** — KerML/SysML term for a dependency. A project's usages
  are listed in `.project.json`. Not called "dependency" — "usage"
  is the spec term.

- **Usage entry** — An IRI + optional version constraint. "This
  project uses `urn:kpar:sensors ^2.0`."

- **Lockfile** — `sysand-lock.toml`. Records the exact resolved
  versions and sources for all transitive usages. Produced by the
  solver.

## Sources and Resolution

- **Index** — A package registry. Hosts multiple versions of multiple
  packages. Queried by IRI. The only source type that produces
  multiple versions for a given IRI.

- **Source** — Where to get the actual files for a package. Could be
  an index, a local path, a KPAR file, or a git repo. Each source
  type is inherently single-version except for indexes.

- **Resolution** — The process of taking an IRI (+ optional version
  constraint) and finding a specific version from an index. The
  public `lookup` API does this for a single package. The solver
  does this recursively for a dependency tree.

- **Solver** — Internal machinery (PubGrub) that takes a set of
  usages and resolves a compatible set of versions across the entire
  dependency graph. Not part of the public API.

- **Lookup options** — Shared option group: `--index`, 
  `--default-index`, `--index-mode`, `--include-std`. Controls which
  indexes are queried.

## Local Operations

- **Environment** — The `sysand_env/` directory where resolved usages
  are installed. Analogous to `node_modules` or Python's virtualenv.

- **Sync** — `env sync`: ensure the environment matches the lockfile.
  Installs missing packages, removes extras.

- **Locate** — `project::locate` / `workspace::locate`: walk up from
  a path to find the project or workspace root. Library operation
  (ADR-0006).

## Contexts

- **ProjectContext** — Path + ConfigMode. Passed as first argument to
  all project operations. Constructed by the caller, not discovered
  implicitly (except in the CLI).

- **WorkspaceContext** — Path only. Passed to workspace operations.

- **ConfigMode** — How to load `sysand.toml`: `Auto` (default),
  `File(path)`, `None`.

## API Surface Terms

- **Binding surface** — One of the five API presentations: Rust CLI,
  Rust library, Java, JS/WASM, Python.

- **Projection** — The mechanical mapping from CLI command path to
  library function path in each binding surface (ADR-0005).

- **Natural return type** — The value an operation naturally produces
  (`String`, `Vec<UsageEntry>`, `BuildOutput`, `()`). No wrapper
  types (ADR-0005 amendment).

## Terms We've Deliberately Avoided

- **Dependency** — Use "usage" instead (KerML term).
- **Package** — We use "interchange project" or just "project"
  for the formal unit. "Package" is acceptable in casual discussion
  but not in APIs or docs.
- **Registry** — We use "index." "Registry" appears in the reference
  Source enum but should be renamed in the rework.

## Open Terminology Questions

1. **Index vs registry** — Decided: use "index." Aligns with `--index`
   flag and all ADRs. "Registry" in the reference code should be renamed.
2. **Resolve vs lookup** — Decided: `lookup` is the public command
   namespace for querying indexes. "Resolve" is reserved for the
   internal solver concept (dependency resolution).
