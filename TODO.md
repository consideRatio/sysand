# TODO

Open work items only. See `CHANGELOG.md` for completed decisions.

- Update ADR-0005 to drop wrapper taxonomy — replace with natural return types per exploration 0009
- Consider internal resolution needs: dependency resolution (env sync, env install, lock update) needs multi-version index queries and transitive usage traversal internally. How does the public resolve API relate to the internal solver? Private functions, or lower-level public API for advanced consumers?
- Explore whether --source-kind/--source belong on commands at all, or if indexes + config + workspaces are sufficient (parked in exploration 0011)
- Subfolder specification: when a source contains multiple projects, how to specify which one? Open design space includes: encoding in the IRI, --source-path flag, auto-discovery by matching IRI to .project.json identifiers. Relevant for git monorepos and multi-project directories.
- Scaffold Rust workspace: crate structure, module layout mirroring command tree
- Define JS/WASM projection rules in detail (async/Promise semantics, wasm-bindgen constraints)
