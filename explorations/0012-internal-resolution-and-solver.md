# Exploration 0012: Internal Resolution and the Solver

**Status: Open**

## Context

ADR-0008 simplifies the public `lookup` API to single-version lookups.
But dependency resolution (triggered by `lock update`, `env sync`,
`usage add --update lock|sync`) needs to explore many versions of many
packages to find a compatible set. This exploration examines how the
reference implementation does this and what it means for the reworked
architecture.

## How the Reference Implementation Works

### The call chain

1. **CLI command** (`sync`, `add` with `--no-lock=false`) calls
   `command_lock` or `command_sync`
2. **`do_lock_projects`** reads the local project's usages, then calls
   `do_lock_extend`
3. **`do_lock_extend`** calls **`solve(usages, resolver)`** — the
   PubGrub solver
4. **The solver** iterates, calling back into the resolver to:
   - Fetch all candidate versions for an IRI (`resolve_candidates`)
   - Read each candidate's usages (to discover transitive dependencies)
   - Choose the best version given constraints
5. The solver returns a `Solution`: one chosen version per IRI, with
   full project info and storage

### The ResolveRead trait

The solver is parameterized over `ResolveRead`, a trait with one key
method:

```rust
fn resolve_read(&self, uri: &Iri<String>)
    -> Result<ResolutionOutcome<Self::ResolvedStorages>, Self::Error>;
```

Given an IRI, it returns **all** available versions of that package
as an iterator of project storages. The solver iterates over these,
reads each one's info (name, version, usages), and feeds them into
PubGrub.

### What the solver needs internally

- **All versions of a package** — not just one. The solver explores
  the version space to find compatible combinations.
- **Usages of each version** — to discover transitive dependencies.
  Package A v2.0 might depend on C ^2.0, while A v1.0 depends on
  C ^1.0.
- **Version ordering** — to prefer newer versions. The solver sorts
  candidates by semver descending.
- **Caching** — `resolve_candidates` caches results per IRI to avoid
  re-fetching during backtracking.

### What the public lookup API needs

Just one version. Look up an IRI, optionally constrain the version,
return info about the best match.

## The Gap

The solver needs multi-version queries internally. The public API
exposes single-version queries. These are fundamentally different
operations:

| | Public lookup | Internal solver |
| --- | --- | --- |
| Input | IRI + optional constraint | IRI (all versions) |
| Output | One package's info | All candidates with full info |
| Purpose | User inspects a package | Solver explores version space |
| Caching | Not needed | Essential for performance |
| Transitive | No | Yes — reads usages recursively |

## How This Should Be Structured

### Option A: Private solver, public lookup is separate

The solver and its `ResolveRead` trait are private implementation
details of the library. The public `lookup` namespace is a thin
layer that uses the same index client but presents single-version
results.

```
public API:
  lookup::show(iri, constraint, opts) → PackageSnapshot
  lookup::info::name::get(iri, constraint, opts) → String
  ...

internal (pub(crate) or private):
  solve::solve(usages, resolver) → Solution
  ResolveRead trait + implementations
  resolve_candidates(resolver, iri) → Vec<(Info, Meta, Storage)>
```

The public and internal paths share index access code (HTTP client,
index protocol) but diverge in what they expose. The public API
picks the best match and returns it. The solver gets the full
candidate list.

### Option B: Two-level public API

Expose both levels publicly:

- `lookup` namespace: user-facing, single-version (ADR-0008)
- `index` namespace: lower-level, multi-version, for advanced
  consumers building custom solvers or tools

This would let IDE plugins, CI tools, or alternative solvers query
the index directly.

### Option C: Solver is public

Expose `solve()` as a public function. It takes usages and a
resolver, returns a solution. Advanced consumers bring their own
resolver implementation.

## Assessment

**Option A is the right starting point.** The solver is complex
internal machinery. Exposing it publicly means stabilizing the
`ResolveRead` trait, the `DependencyProvider` integration, the
caching strategy — all of which are implementation details that may
change.

The public `lookup` API and the solver share the same index client
code, but that's a shared dependency, not a shared API surface.

If demand emerges for lower-level access (Option B or C), it can be
added later without changing the public lookup API. The internal
structure supports this — `ResolveRead` is already a trait that
different implementations can plug into.

## What This Means for the Rework

1. **`solve` module is `pub(crate)`** — not part of the public API
2. **`ResolveRead` trait is internal** — the solver's abstraction
   over index access
3. **Public `lookup` namespace** uses the same index client but
   returns single-version results per ADR-0008
4. **Index client code** (HTTP, protocol) is shared but the
   interface boundary is at the public API level, not at the
   trait level
5. **Caching** is an internal solver concern, not exposed

## Open Questions

1. Should the solver be a separate crate within the workspace for
   code organization, even if it's not publicly exposed?
2. Does the index client need its own abstraction, separate from
   `ResolveRead`, that both the public lookup API and the solver
   can use?
3. The reference solver uses PubGrub with a custom `VersionSet`
   (`DiscreteHashSet` over indices rather than semver ranges).
   Is this the right approach for the rework, or should we use
   PubGrub with semver ranges directly?
