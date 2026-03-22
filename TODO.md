# TODO

Open work items only. See `CHANGELOG.md` for completed decisions.

- Decide whether --source-kind/--source belong on commands at all (parked in exploration 0011). Key insight: when used on `usage add`, it's really just a convenience for updating `sysand.toml` with a source override — the end state is identical to editing config manually. The question is whether that shortcut justifies 8 source kind variants, a SourceSpec type, and flags on multiple commands, given ADR-0009 already sets the precedent of "edit the file directly" for field-level operations. If removed, the SourceKind enum and SourceSpec type in public-api.md become internal to config parsing only.
- (Deferred) Git monorepo subpath: if we ever need to point at a subdirectory within a git repo, encode it in the URL fragment like pip does (`https://github.com/org/repo.git#subdirectory=core`). Only `RemoteGit` has this ambiguity — all other directory-shaped sources (`Editable`, `LocalSrc`, `RemoteSrc`) already point at exact directories. Not needed for initial implementation.
- Scaffold Rust workspace: crate structure, module layout mirroring command tree
- Define JS/WASM projection rules in detail (async/Promise semantics, wasm-bindgen constraints)
