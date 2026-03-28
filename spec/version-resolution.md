# Version Resolution

## Semver Required

All project versions must be valid semver (Semantic Versioning 2.0.0).
Non-semver version strings are rejected everywhere: `project init`,
reading `.project.json`, and index queries.

## Version Constraint Rules

- **No constraint** → latest stable version (highest semver,
  pre-releases excluded)
- **Range constraint** (`^2.0`, `>=1.5`) → latest stable matching
  the constraint (pre-releases excluded)
- **Constraint naming a pre-release** (`^2.1.0-beta.1`) → latest
  matching, pre-releases included
- **Exact version** (`=2.1.0`) → that specific version

Pre-release inclusion is controlled entirely by the version constraint
string. No flags.

## Error Cases

- `ProjectNotInIndex` — IRI not found in any index
- `VersionNotInIndex` — IRI exists but no version satisfies the
  constraint

## Internal Resolution

See `dependency-resolution.md` for the full dependency resolution
pipeline: solver, lockfile, and environment sync.

## Rationale

**Why single-version return.** An earlier design had lookup return
`Vec<LookupMatch<T>>` — multiple packages matching a query, each with
typed fields. This complicated every binding surface (generics over
`LookupMatch<T>` are painful in JNI) and didn't match real usage
patterns: callers almost always want one specific package. Returning
one package by IRI + optional version constraint simplifies the API
and aligns with how the solver actually queries indexes internally.

**Why pre-release is constraint-driven.** A `--include-pre-releases`
flag would need to thread through every resolution code path and
interact with constraint evaluation. Instead, the constraint string
itself signals intent: `^2.1.0-beta.1` includes pre-releases because
the constraint names one; `^2.0` excludes them. One mechanism, no
flags, no ambiguity.

**Why non-semver is rejected outright.** See also `option-rules.md`.
Non-semver strings have no defined ordering — the solver cannot compare
them, evaluate range constraints against them, or determine pre-release
status. Rejecting at every entry point (init, manifest read, index
query) keeps the resolution pipeline simple rather than propagating
"might not be orderable" through every comparison.
