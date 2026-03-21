# TODO

Open work items only. See `CHANGELOG.md` for completed decisions.

- Populate spec/ directory with living specification extracted from current ADRs. Candidate files: command-tree.md, projection-rules.md, option-rules.md, discovery-and-config.md, error-model.md, version-resolution.md. This is the new single source of truth for the current design.
- Revert ADR in-place amendments — ADRs should be immutable. The amendment logs we added should be removed, and the original text restored from git history. The current state lives in spec/ instead.
- Consider internal resolution needs: dependency resolution (env sync, env install, lock update) needs multi-version index queries and transitive usage traversal internally. How does the public lookup API relate to the internal solver? Private functions, or lower-level public API for advanced consumers?
- Explore whether --source-kind/--source belong on commands at all, or if indexes + config + workspaces are sufficient (parked in exploration 0011)
- Subfolder specification: when a source contains multiple projects, how to specify which one? Open design space includes: encoding in the IRI, --source-path flag, auto-discovery by matching IRI to .project.json identifiers. Relevant for git monorepos and multi-project directories.
- Scaffold Rust workspace: crate structure, module layout mirroring command tree
- Define JS/WASM projection rules in detail (async/Promise semantics, wasm-bindgen constraints)
- Clarify "package" vs "project" terminology: decide if "package" is acceptable in user-facing text (CLI help, error messages, docs) or if we stick with "interchange project" / "project" everywhere. Make consistent across ADRs, explorations, and code.
