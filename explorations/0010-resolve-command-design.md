# Exploration 0010: Resolve Command Design

## Context

The current resolve command tree (ADR-0004) returns multiple matching
versions per query — `Vec<ResolveMatch<T>>`. This creates complex
return types that don't project well to bindings, and the generic
`ResolveMatch<T>` wrapper feels like the kind of speculative abstraction
we're moving away from (see exploration 0009).

More fundamentally, most callers want information about one specific
package, not a list of everything that matched. The local `project info`
commands operate on one project — resolve should be symmetrical.

## Decision Direction

**Resolve always returns one package.** The IRI identifies the package,
an optional version constraint narrows to a single version. The index
resolves to the best match.

### Version Resolution Rules

- No constraint → latest stable version (highest semver, pre-releases
  excluded)
- Constraint like `^2.0` → latest stable matching the constraint
  (pre-releases excluded)
- Constraint naming a pre-release like `^2.1.0-beta.1` → latest
  matching including pre-releases (pre-releases opted into via the
  constraint itself)
- Exact version like `=2.1.0` → that specific version

This follows standard semver range semantics, consistent with Cargo
and npm.

### Pre-releases

Pre-releases are excluded from matching unless the constraint explicitly
names a pre-release. No flag like `--include-pre-releases`. The version
constraint string is the single source of truth for what versions are
eligible.

### Non-semver versions

Versions in an index that don't parse as valid semver are invisible to
resolution — skipped entirely. If the only available versions are
non-semver, the result is a `VersionNotFound` error.

### What if no version matches?

Error. `VersionNotFound` if the IRI exists but no version satisfies the
constraint. `PackageNotFound` if the IRI isn't in any index at all.

## Revised Command Tree

Current (ADR-0004):

```
resolve
  show <IRI_OR_URL>
    [--relative-root <PATH>]
    [resolve options]
  info
    name get <IRI_OR_URL> [--relative-root <PATH>] [resolve options]
    description get <IRI_OR_URL> [--relative-root <PATH>] [resolve options]
    version get <IRI_OR_URL> [--relative-root <PATH>] [resolve options]
    ...
  metadata
    created get <IRI_OR_URL> [--relative-root <PATH>] [resolve options]
    ...
```

Proposed:

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

Changes:
- `<IRI_OR_URL>` → `<IRI>` (clearer — it's a package identifier)
- `[<VERSION_CONSTRAINT>]` added as optional positional arg
- `--relative-root` dropped (was for URL-as-locator, needs separate
  consideration if still needed)

## Return Types

With single-version resolution, every resolve command returns the same
shape as its local counterpart:

| Command | Returns | Local equivalent |
| ------- | ------- | ---------------- |
| `resolve show` | `PackageSnapshot` | `project show` → `ProjectSnapshot` |
| `resolve info name get` | `String` | `project info name get` → `String` |
| `resolve info maintainer list` | `Vec<String>` | `project info maintainer list` → `Vec<String>` |
| `resolve info version get` | `String` | `project info version get` → `String` |
| `resolve metadata created get` | `Option<DateTime>` | `project metadata created get` → `Option<DateTime>` |
| `resolve metadata checksum list` | `Vec<ChecksumEntry>` | `project metadata checksum list` → `Vec<ChecksumEntry>` |

No `ResolveMatch`, no `ResolveFieldResult`, no generics. The return
type of `resolve info name get` is just `String`, same as
`project info name get`.

The only difference from local commands is the resolved version, which
the caller already knows from the constraint they passed in. If they
need the exact version that was resolved, `resolve info version get`
returns it.

## End-to-End Scenarios

### Scenario 1: Rust — inspect a remote package

```rust
let name: String = resolve::info::name::get(
    "urn:example:sensors",
    None,  // latest stable
    ResolveOptions::default(),
)?;
println!("Package name: {name}");

let version: String = resolve::info::version::get(
    "urn:example:sensors",
    Some("^2.0"),
    ResolveOptions::default(),
)?;
println!("Resolved version: {version}");
```

No wrappers, no unwrapping. Just the value.

### Scenario 2: Java — check maintainers of a specific version

```java
List<String> maintainers = client.resolve().info().maintainer().list(
    "urn:example:sensors",
    "=2.1.0",
    new ResolveOptions()
);
System.out.println("Maintainers: " + maintainers);
```

### Scenario 3: JS/WASM — show full package info

```ts
const pkg = await sysand.resolve.show("urn:example:sensors", "^2.0");
console.log(`${pkg.name} @ ${pkg.version}`);
console.log(`License: ${pkg.license}`);
```

### Scenario 4: Python — error on missing package

```python
try:
    name = sysand.resolve.info.name.get("urn:nonexistent:package")
except PackageNotFoundError as e:
    print(f"Not found: {e.context}")
```

### Scenario 5: Python — pre-release

```python
# Latest stable
version = sysand.resolve.info.version.get("urn:example:sensors")
print(version)  # "2.1.0"

# Opting into pre-releases via constraint
version = sysand.resolve.info.version.get(
    "urn:example:sensors", "^3.0.0-beta.1"
)
print(version)  # "3.0.0-beta.3"
```

## Symmetry with Local Commands

The resolve namespace mirrors the project namespace for read operations.
The calling pattern is almost identical:

**Local:**

```rust
let ctx = ProjectContext::new(path);
let name = project::info::name::get(&ctx)?;
```

**Remote:**

```rust
let name = resolve::info::name::get(iri, version_constraint, resolve_opts)?;
```

The first arg differs (context vs IRI) because the inputs are
fundamentally different — one is a local path, the other is a remote
identifier. But the return types are identical.

## What About Listing Available Versions?

With single-version resolution, there's no built-in way to ask "what
versions exist?" This could be a separate command:

```
resolve version list <IRI> [resolve options]
```

Returns `Vec<String>` — just version strings, ordered by semver.
This is a different operation from inspecting a package — it's
browsing the index. Worth adding but separable from this decision.

## Open Questions

1. Is `resolve version list` needed now or can it be deferred?
2. Should `resolve show` include the resolved version in its return
   type (`PackageSnapshot` presumably has a version field), or should
   callers use `resolve info version get` separately?
3. Does `--relative-root` still have a use case? It was for
   URL-as-locator patterns. If resolve always takes an IRI, it may
   not be needed.
