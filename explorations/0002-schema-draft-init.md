# 0002: Schema Draft — `init` Command

**Status: Distilled into ADRs 0001–0005**

## Context

Exploring what a meta-description schema looks like for Sysand operations.
Goal: single source of truth that maps to CLI (clap), Rust library, and bindings
(Python/Java/WASM).

## Survey of Existing Approaches

No existing IDL solves the full problem (CLI + library + multi-language bindings).

| Approach                       | Verdict                                                                                                                                                              |
| ------------------------------ | -------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Smithy** (AWS IDL)           | Best existing IDL for this shape of problem. Rich traits, extensible, multi-target codegen. But JVM toolchain, HTTP-oriented, custom codegen plugins needed for CLI. |
| **Protobuf/gRPC**              | Shape-only, RPC-oriented. No constraints, no CLI semantics. Too limited.                                                                                             |
| **Cap'n Proto**                | Similar to protobuf but smaller ecosystem. Not a fit.                                                                                                                |
| **TypeSpec** (Microsoft)       | Elegant syntax, good extensibility. But TypeScript toolchain, no Rust emitter, HTTP-oriented.                                                                        |
| **UniFFI** (Mozilla)           | Solves Rust→Python/Kotlin/Swift bindings specifically. Likely part of any solution for bindings, but doesn't help with CLI or schema.                                |
| **OpenAPI/JSON Schema**        | Universal but verbose, HTTP-centric. Custom CLI generator needed.                                                                                                    |
| **WIT** (Wasm Component Model) | Relevant for WASM target specifically. Doesn't help with CLI or native FFI.                                                                                          |
| **KDL**                        | Pleasant syntax, but just a data format — no IDL semantics. Could be used as syntax for a custom schema.                                                             |

**Conclusion:** Custom schema (approach 3), likely informed by Smithy's concepts
(operations, types, traits), using a pleasant authoring format.

## Strawman Schema: TOML-based

### Types

```toml
[types.ProjectInfo]
description = "Core project metadata (maps to .project.json)"

[types.ProjectInfo.fields]
name        = { type = "string", required = true }
publisher   = { type = "string" }
description = { type = "string" }
version     = { type = "string", required = true }
license     = { type = "string" }
maintainer  = { type = "list<string>", default = "[]" }
website     = { type = "iri" }
topic       = { type = "list<string>", default = "[]" }
usage       = { type = "list<ProjectUsage>", default = "[]" }

[types.ProjectMetadata]
description = "Project file index and checksums (maps to .meta.json)"

[types.ProjectMetadata.fields]
index            = { type = "map<string, path>", default = "{}" }
created          = { type = "datetime", required = true }
metamodel        = { type = "iri" }
includes_derived = { type = "bool" }
includes_implied = { type = "bool" }
checksum         = { type = "map<path, Checksum>" }
```

### Operations

```toml
[operations.init]
description = "Create a new SysML v2 or KerML interchange project"

[operations.init.inputs]
path      = { type = "path", description = "Project directory", default = "." }
name      = { type = "string", description = "Project name. Defaults to directory name" }
publisher = { type = "string", description = "Project publisher", default = "untitled" }
version   = { type = "string", description = "Project version", default = "0.0.1" }
no_semver = { type = "bool", description = "Skip SemVer validation", default = "false", requires = ["version"] }
license   = { type = "string", description = "SPDX license identifier" }
no_spdx   = { type = "bool", description = "Skip SPDX validation", default = "false", requires = ["license"] }

[operations.init.outputs]
info     = { type = "ProjectInfo" }
metadata = { type = "ProjectMetadata" }

[operations.init.errors]
invalid_semver = "version provided but not valid SemVer"
invalid_spdx   = "license provided but not valid SPDX"
already_exists = "project already exists at path"

[operations.init.behavior]
steps = [
  "validate version as SemVer (unless no_semver)",
  "validate license as SPDX (unless no_spdx)",
  "derive name from directory if not provided",
  "create ProjectInfo with defaults for missing fields",
  "create ProjectMetadata with empty index and current timestamp",
  "write both to storage",
]

[operations.init.cli]
positional = ["path"]
```

## Questions This Raises

1. **Format**: TOML works but gets nested fast. Would something like KDL or even
   a Rust-like DSL be more ergonomic?

2. **Type system**: How rich does it need to be? Just `string`, `bool`, `list<T>`,
   `map<K,V>`, and named types? Or do we need generics, enums, tagged unions?

3. **The `cli` section**: Is it enough to just mark which inputs are positional?
   Or do we need aliases, short flags, mutual exclusion groups, etc.?

4. **The `behavior` section**: Free-form steps feel right for human readability
   but aren't useful for codegen. Is that OK? Or should behavior be more structured
   (preconditions, postconditions, state changes)?

5. **Binding-specific sections**: Should there be `[operations.init.python]` etc.
   for binding-specific overrides, or should the mapping be fully automatic?

6. **Storage abstraction**: The current impl takes `&mut P: ProjectMut`. Should
   the schema describe _where_ things are written (filesystem, memory), or is
   that an implementation concern outside the schema?

7. **Validation as types vs. flags**: Instead of `no_semver` / `no_spdx`, should
   there be a `semver` type and an `spdx` type that carry their own validation?
   Then the schema would be `version = { type = "semver" }` and the `no_semver`
   flag would mean "treat as plain string instead."
