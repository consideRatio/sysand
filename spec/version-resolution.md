# Version Resolution

Sources: ADR-0007, ADR-0008

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
