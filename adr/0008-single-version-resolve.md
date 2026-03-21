# ADR-0008: Single-Version Resolve

- **Status**: Accepted
- **Date**: 2026-03-21
- **Updates**: ADR-0004 (command tree: resolve section rewritten)
- **Applies**: ADR-0007 (semver required)

## Context

The resolve command tree in ADR-0004 accepts `<IRI_OR_URL>` and can
return information about multiple matching versions. This leads to
complex return types (`Vec<ResolveMatch<T>>`) that don't project
well to bindings and don't match what callers typically need.

The local `project info` commands operate on one project and return
simple types. Resolve should be symmetrical.

## Decision

### 1. Resolve always returns one package

Every resolve command identifies a single package version and returns
its data. The IRI identifies the package, an optional version
constraint selects the version.

```
sysand resolve show <IRI> [<VERSION_CONSTRAINT>] [resolve options]
sysand resolve info name get <IRI> [<VERSION_CONSTRAINT>] [resolve options]
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

### 3. Resolve takes an IRI, not an IRI-or-URL

The first argument is `<IRI>` — a package identifier for index
lookup. `--relative-root` is removed.

### 4. Return types mirror local commands

| Command | Returns |
| ------- | ------- |
| `resolve show` | `PackageSnapshot` |
| `resolve info name get` | `String` |
| `resolve info version get` | `String` |
| `resolve info description get` | `Option<String>` |
| `resolve info license get` | `Option<String>` |
| `resolve info website get` | `Option<String>` |
| `resolve info maintainer list` | `Vec<String>` |
| `resolve info topic list` | `Vec<String>` |
| `resolve info usage list` | `Vec<UsageEntry>` |
| `resolve metadata created get` | `Option<DateTime>` |
| `resolve metadata index list` | `Vec<IndexEntry>` |
| `resolve metadata checksum list` | `Vec<ChecksumEntry>` |
| `resolve metadata metamodel get` | `Option<Metamodel>` |
| `resolve metadata includes-derived get` | `Option<bool>` |
| `resolve metadata includes-implied get` | `Option<bool>` |

These are the same types as the corresponding `project info` and
`project metadata` commands.

### 5. Errors

- `PackageNotFound` — IRI not found in any index
- `VersionNotFound` — IRI exists but no version satisfies the
  constraint

## Revised Command Tree (resolve section)

```
  resolve
    show <IRI> [<VERSION_CONSTRAINT>]
      [resolve options]
    info
      name get <IRI> [<VERSION_CONSTRAINT>] [resolve options]
      description get <IRI> [<VERSION_CONSTRAINT>] [resolve options]
      version get <IRI> [<VERSION_CONSTRAINT>] [resolve options]
      license get <IRI> [<VERSION_CONSTRAINT>] [resolve options]
      website get <IRI> [<VERSION_CONSTRAINT>] [resolve options]
      maintainer list <IRI> [<VERSION_CONSTRAINT>] [resolve options]
      topic list <IRI> [<VERSION_CONSTRAINT>] [resolve options]
      usage list <IRI> [<VERSION_CONSTRAINT>] [resolve options]
    metadata
      created get <IRI> [<VERSION_CONSTRAINT>] [resolve options]
      index list <IRI> [<VERSION_CONSTRAINT>] [resolve options]
      checksum list <IRI> [<VERSION_CONSTRAINT>] [resolve options]
      metamodel get <IRI> [<VERSION_CONSTRAINT>] [resolve options]
      includes-derived get <IRI> [<VERSION_CONSTRAINT>] [resolve options]
      includes-implied get <IRI> [<VERSION_CONSTRAINT>] [resolve options]
```

## Binding Projections

**Rust:**

```rust
let name: String = resolve::info::name::get(
    "urn:example:sensors",
    None,  // latest stable
    ResolveOptions::default(),
)?;
```

**Java:**

```java
String name = client.resolve().info().name().get(
    "urn:example:sensors", null, new ResolveOptions());
```

**JS/WASM:**

```ts
const name = await sysand.resolve.info.name.get(
    "urn:example:sensors", undefined, {});
```

**Python:**

```python
name = sysand.resolve.info.name.get("urn:example:sensors")
```

## What Changed

| Before | After |
| ------ | ----- |
| `<IRI_OR_URL>` | `<IRI>` |
| No version constraint arg | Optional `[<VERSION_CONSTRAINT>]` positional |
| Returns multiple matches | Returns one result |
| `ResolveMatch<T>` / `ResolveFieldResult<T>` | Same types as local commands |
| `--relative-root <PATH>` on every command | Removed |

## Consequences

- Resolve return types are simple and symmetrical with local commands
- No generic wrapper types needed for resolve
- Binding projection is trivial — same types as project info/metadata
- Version listing (`resolve version list <IRI>`) can be added later
  as a separate command if needed
