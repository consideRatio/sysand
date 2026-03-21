# ADR-0008: Single-Version Lookup

- **Status**: Accepted
- **Date**: 2026-03-21
- **Updates**: ADR-0004 (command tree: lookup section rewritten)
- **Applies**: ADR-0007 (semver required)

## Context

The lookup command tree in ADR-0004 accepts `<IRI_OR_URL>` and can
return information about multiple matching versions. This leads to
complex return types (`Vec<LookupMatch<T>>`) that don't project
well to bindings and don't match what callers typically need.

The local `project info` commands operate on one project and return
simple types. Lookup should be symmetrical.

## Decision

### 1. Lookup always returns one package

Every lookup command identifies a single package version and returns
its data. The IRI identifies the package, an optional version
constraint selects the version.

```
sysand lookup show <IRI> [<VERSION_CONSTRAINT>] [lookup options]
sysand lookup info name get <IRI> [<VERSION_CONSTRAINT>] [lookup options]
```

### 2. Version resolution rules

- **No constraint** → latest stable version (highest semver,
  pre-releases excluded)
- **Range constraint** (`^2.0`, `>=1.5`) → latest stable matching
  the constraint (pre-releases excluded)
- **Constraint naming a pre-release** (`^2.1.0-beta.1`) → latest
  matching, pre-releases included
- **Exact version** (`=2.1.0`) → that specific version

Pre-release inclusion is controlled entirely by the version constraint
string. No flags.

All versions are valid semver (ADR-0007), so ordering and matching
are always well-defined.

### 3. Lookup takes an IRI, not an IRI-or-URL

The first argument is `<IRI>` — a package identifier for index
lookup. `--relative-root` is removed.

### 4. Return types mirror local commands

| Command                                | Returns              |
| -------------------------------------- | -------------------- |
| `lookup show`                          | `PackageSnapshot`    |
| `lookup info name get`                 | `String`             |
| `lookup info version get`              | `String`             |
| `lookup info description get`          | `Option<String>`     |
| `lookup info license get`              | `Option<String>`     |
| `lookup info website get`              | `Option<String>`     |
| `lookup info maintainer list`          | `Vec<String>`        |
| `lookup info topic list`               | `Vec<String>`        |
| `lookup info usage list`               | `Vec<UsageEntry>`    |
| `lookup metadata created get`          | `Option<DateTime>`   |
| `lookup metadata index list`           | `Vec<IndexEntry>`    |
| `lookup metadata checksum list`        | `Vec<ChecksumEntry>` |
| `lookup metadata metamodel get`        | `Option<Metamodel>`  |
| `lookup metadata includes-derived get` | `Option<bool>`       |
| `lookup metadata includes-implied get` | `Option<bool>`       |

These are the same types as the corresponding `project info` and
`project metadata` commands.

### 5. Errors

- `PackageNotFound` — IRI not found in any index
- `VersionNotFound` — IRI exists but no version satisfies the
  constraint

## Revised Command Tree (lookup section)

```
  lookup
    show <IRI> [<VERSION_CONSTRAINT>]
      [lookup options]
    info
      name get <IRI> [<VERSION_CONSTRAINT>] [lookup options]
      description get <IRI> [<VERSION_CONSTRAINT>] [lookup options]
      version get <IRI> [<VERSION_CONSTRAINT>] [lookup options]
      license get <IRI> [<VERSION_CONSTRAINT>] [lookup options]
      website get <IRI> [<VERSION_CONSTRAINT>] [lookup options]
      maintainer list <IRI> [<VERSION_CONSTRAINT>] [lookup options]
      topic list <IRI> [<VERSION_CONSTRAINT>] [lookup options]
      usage list <IRI> [<VERSION_CONSTRAINT>] [lookup options]
    metadata
      created get <IRI> [<VERSION_CONSTRAINT>] [lookup options]
      index list <IRI> [<VERSION_CONSTRAINT>] [lookup options]
      checksum list <IRI> [<VERSION_CONSTRAINT>] [lookup options]
      metamodel get <IRI> [<VERSION_CONSTRAINT>] [lookup options]
      includes-derived get <IRI> [<VERSION_CONSTRAINT>] [lookup options]
      includes-implied get <IRI> [<VERSION_CONSTRAINT>] [lookup options]
```

## Binding Projections

**Rust:**

```rust
let name: String = lookup::info::name::get(
    "urn:example:sensors",
    None,  // latest stable
    LookupOptions::default(),
)?;
```

**Java:**

```java
String name = client.lookup().info().name().get(
    "urn:example:sensors", null, new LookupOptions());
```

**JS/WASM:**

```ts
const name = await sysand.lookup.info.name.get(
  "urn:example:sensors",
  undefined,
  {},
);
```

**Python:**

```python
name = sysand.lookup.info.name.get("urn:example:sensors")
```

## What Changed

| Before                                    | After                                        |
| ----------------------------------------- | -------------------------------------------- |
| `<IRI_OR_URL>`                            | `<IRI>`                                      |
| No version constraint arg                 | Optional `[<VERSION_CONSTRAINT>]` positional |
| Returns multiple matches                  | Returns one result                           |
| `LookupMatch<T>` / `LookupFieldResult<T>` | Same types as local commands                 |
| `--relative-root <PATH>` on every command | Removed                                      |

## Consequences

- Lookup return types are simple and symmetrical with local commands
- No generic wrapper types needed for lookup
- Binding projection is trivial — same types as project info/metadata
- Version listing (`lookup version list <IRI>`) can be added later
  as a separate command if needed
