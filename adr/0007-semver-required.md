# ADR-0007: Semver Required

- **Status**: Accepted
- **Date**: 2026-03-21
- **Supersedes**: ADR-0004 `--allow-non-semver` flag

## Context

The `version` field in `.project.json` is required. The reference
implementation parses it as `semver::Version` with a TODO about
fallback for invalid semvers. ADR-0004 included an `--allow-non-semver`
flag on `project init` and `project info version set` to permit
non-semver version strings.

Allowing non-semver creates edge cases throughout the system:
resolution ordering, pre-release filtering, version constraint
evaluation, and direct source verification all depend on semver
semantics. Supporting arbitrary version strings would require
special-casing each of these situations.

## Decision

The `version` field must be a valid semver string (per the Semantic
Versioning 2.0.0 specification). Sysand errors if:

- `project init` is given a non-semver version
- `project info version set` is given a non-semver version
- A `.project.json` is read with a non-semver version
- A package index returns a version that doesn't parse as semver

The `--allow-non-semver` flag is removed from `project init` and
`project info version set`.

## Consequences

- Version ordering, constraint matching, and pre-release filtering
  are well-defined everywhere — no special cases
- Non-semver packages in an index are treated as malformed, not
  silently skipped
- Projects with non-semver versions must update their version before
  sysand can manage them
- The `--allow-non-spdx` flag for licenses is unaffected by this
  decision — SPDX compliance remains optional
