# Version Resolution

Sources: ADR-0007, ADR-0008

## Semver Required

All project versions must be valid semver (Semantic Versioning 2.0.0).
Non-semver version strings are rejected everywhere: `project init`,
`project info version set`, reading `.project.json`, and index queries.

## Lookup: Single-Version Result

Every `lookup` command identifies a single package version and returns
its data. The IRI identifies the package, an optional version constraint
selects the version.

```
sysand lookup show <IRI> [<VERSION_CONSTRAINT>] [lookup options]
```

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

- `PackageNotFound` — IRI not found in any index
- `VersionNotFound` — IRI exists but no version satisfies the
  constraint

## Return Types

Lookup return types mirror local commands:

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

## Internal Resolution

See `dependency-resolution.md` for the full dependency resolution
pipeline: solver, lockfile, and environment sync.
