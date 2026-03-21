# Dependency Resolution

How usages are resolved into a lockfile and synced to the environment.

## Three-Stage Pipeline

```
.project.json (usages)
        │
        ▼
   lock update         ← solver (PubGrub), reads indexes + config overrides
        │
        ▼
sysand-lock.toml       ← exact versions, sources, exports, checksums
        │
        ▼
    env sync            ← fetches and installs into sysand_env/
        │
        ▼
   sysand_env/          ← installed project files, ready for use
```

### Stage 1: Lock Update

Reads the root project's `.project.json` usages and resolves them to
exact versions. Produces `sysand-lock.toml`.

**Inputs:**

- Root project's `.project.json` — the usages (IRI + version constraint)
- Root project's `sysand.toml` — source overrides, index URLs
- Index servers — queried for available versions

**Process:**

1. Collect all direct usages from `.project.json`
2. Build a resolver from `sysand.toml` (config overrides > stdlib > standard resolver)
3. Run the PubGrub solver, which iteratively:
   - Picks a package to resolve (fewest candidates first)
   - Queries the resolver for all available versions of that package
   - Reads `.project.json` and `.meta.json` from each candidate
   - Filters versions by constraint
   - Picks the best match (highest semver)
   - Reads the chosen version's usages to discover transitive dependencies
   - Repeats until all dependencies are resolved or a conflict is found
4. Write the solution to `sysand-lock.toml`

**What the solver reads from each candidate:**

- `.project.json` — version (for constraint matching) and usages
  (for transitive dependency discovery). Essential for resolution.
- `.meta.json` — symbol exports, checksums, metamodel. Needed for the
  lockfile output, carried alongside `.project.json`.

**Only the root project's config applies.** Dependencies' own
`sysand.toml` files are not read during resolution. The root project
owner controls where everything comes from.

### Stage 2: Lockfile

`sysand-lock.toml` records the complete resolved dependency graph.

Each entry contains:

- `name` — project name
- `publisher` — optional
- `version` — exact resolved version
- `identifiers` — IRIs for this project
- `exports` — symbol names exported (from `.meta.json` index)
- `usages` — what this project depends on (for integrity)
- `sources` — how to fetch this project (from config or resolver)
- `checksum` — canonical content hash

The lockfile is deterministic: the same inputs produce the same
lockfile. It should be committed to version control.

### Stage 3: Env Sync

Reads `sysand-lock.toml` and ensures `sysand_env/` matches it.

For each project in the lockfile:

- If already installed with the correct checksum → skip
- If missing → fetch from the recorded source and install
- If the installed version differs → replace

Sync does not re-resolve. It trusts the lockfile completely. The
lockfile records which source to use for each project, so sync
doesn't need config or index access for already-locked dependencies.

## When Resolution Runs

| Command                               | Locks | Syncs                         |
| ------------------------------------- | ----- | ----------------------------- |
| `lock update`                         | Yes   | No                            |
| `env sync`                            | No    | Yes                           |
| `project usage add --update manifest` | No    | No                            |
| `project usage add --update lock`     | Yes   | No                            |
| `project usage add --update sync`     | Yes   | Yes                           |
| `env install`                         | No    | Installs one package directly |

`project usage add` with `--update manifest` only edits `.project.json`.
With `--update lock`, it also re-runs the solver. With `--update sync`
(the default), it re-runs the solver and syncs the environment.

`env install` bypasses the lockfile entirely — it installs a specific
package directly into the environment.

## Conflict Resolution

The PubGrub algorithm handles version conflicts through backtracking
and conflict-driven learning. If package A requires C ^2.0 and
package B requires C <2.0, PubGrub will try alternative versions of
A or B, or report an unsatisfiable dependency tree with a clear
explanation of which constraints conflict.

Additionally, the solver detects symbol export collisions — if two
resolved projects export the same symbol name, the lock fails with
a `NameCollision` error.

## The Solver is Internal

The PubGrub solver and the multi-version index queries it uses are
private library internals (`pub(crate)`). The public API provides:

- `lock update` — runs the solver, produces a lockfile
- `env sync` — installs from the lockfile
  The solver's `ResolveRead` trait and `DependencyProvider` implementation
  are not part of the public API.
