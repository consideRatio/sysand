# Review of `ap/new-env` against `origin/main`

Scope: filesystem local environment migration from `entries.txt` /
`versions.txt` to `env.toml` plus `sysand_env/lib/...`. WASM local storage
is intentionally still on the old layout.

## Findings

1. High: legacy filesystem envs are not migrated or recognized.

   Location: `core/src/env/local_directory/mod.rs:78`,
   `sysand/src/lib.rs:740`, `sysand/src/lib.rs:745`,
   `core/src/commands/env/mod.rs:47`.

   `LocalDirectoryEnvironment::try_read()` treats a missing
   `sysand_env/env.toml` as "no env". For a pre-migration filesystem env,
   `sysand_env` already exists and contains `entries.txt` plus hashed project
   directories, but no `env.toml`. Commands that need an env can then proceed
   to `get_or_create_env()`, which tries to create `sysand_env` and fails
   because the directory already exists.

   Why it matters: users upgrading with an existing local env can be unable to
   `sync`, `env install`, or use APIs that read the env. The PR says the old
   structure is removed, but it does not define an upgrade path or even a clear
   legacy-layout error.

   Suggested fix direction: add explicit legacy-layout detection. Either
   migrate old envs by reading `entries.txt` / `versions.txt`, generating
   `env.toml`, and moving/copying projects under `lib/`, or return a clear
   "legacy env requires migration" error before creation is attempted.

   Suggested verification: create an env on `origin/main`, install a project,
   switch to this branch, then run `sysand env list`, `sysand sync`, and Python
   env APIs.

2. High: corrupt `env.toml` paths can make uninstall delete outside
   `sysand_env`.

   Location: `core/src/env/local_directory/metadata.rs:237`,
   `core/src/env/local_directory/mod.rs:492`,
   `core/src/env/local_directory/utils.rs:16`.

   Non-editable `EnvProject.path` is documented as relative to the env
   directory, but deserialization does not enforce that it stays under
   `sysand_env/lib` or even under `sysand_env`. Uninstall computes
   `self.root_dir.join(project.path.as_str())` and recursively cleans that
   directory. A crafted or corrupted `env.toml` with `path = "../victim"` can
   delete sibling files outside the env.

   Why it matters: `env.toml` is persisted local state, and corruption or manual
   edits should not let an uninstall remove arbitrary neighboring directories.

   Suggested fix direction: validate metadata paths on read and before delete.
   For non-editable projects, reject absolute paths, `..`, root/prefix
   components, and paths outside the owned project directory prefix. Prefer
   canonical containment checks before recursive deletion.

   Suggested verification: add a negative test with `path = "../victim"` and
   assert uninstall errors without touching the sibling directory.

3. High: installed projects lose non-primary identifiers in `env.toml`.

   Location: `core/src/commands/sync.rs:127`, `core/src/commands/sync.rs:190`,
   `core/src/env/local_directory/mod.rs:102`, `core/src/env/local_directory/mod.rs:464`.

   `do_sync()` chooses `project.identifiers.first()` as `main_uri` and passes
   only that identifier to `try_install()` / `put_project()`. `put_project()`
   records only `vec![identifier]` for installed projects. After syncing,
   `merge_lock()` only adds editable/workspace projects, so aliases from the
   lock entry are never persisted for normal installed dependencies.

   Why it matters: a lock project with multiple identifiers can be installed
   successfully, but resolving/listing by an alias will fail because `env.toml`
   is incomplete.

   Suggested fix direction: make the write path accept all identifiers and
   usages from the lock entry, or have `merge_lock()` also merge identifiers and
   usages for installed projects already present in metadata.

   Suggested verification: create a lock/project with two identifiers, sync it,
   then assert `env.toml` contains both and `env.get_project(alias, version)`
   resolves.

4. Medium: uninstall can make metadata lie after deletion failures.

   Location: `core/src/env/local_directory/utils.rs:14`,
   `core/src/env/local_directory/mod.rs:490`, `core/src/env/local_directory/mod.rs:497`,
   `core/src/env/local_directory/mod.rs:512`, `core/src/env/local_directory/mod.rs:523`.

   `clean_dir()` logs and ignores every filesystem error. `del_project_version()`
   and `del_uri()` then remove metadata entries and write `env.toml` anyway.

   Why it matters: a failed delete can leave project files on disk that the env
   no longer knows about. Later installs can collide with or overwrite these
   orphaned directories.

   Suggested fix direction: use fallible deletion, ideally `remove_dir_all()` for
   env-owned project dirs after validating containment. Only remove metadata
   after deletion succeeds, or explicitly preserve/mark entries when cleanup is
   incomplete.

   Suggested verification: add a test that forces deletion failure and assert
   metadata remains consistent.

5. Medium: persistent-state conflicts are handled with panics.

   Location: `core/src/env/local_directory/mod.rs:121`,
   `core/src/env/local_directory/mod.rs:425`, `core/src/env/local_directory/mod.rs:488`,
   `core/src/env/local_directory/metadata.rs:224`.

   The new `env.toml` stores editable/workspace state, but write paths use
   `assert!`, `assert_eq!`, and `unwrap()` for conditions that can arise from
   persisted metadata or changed lock resolution. Examples include overwriting
   an existing editable entry, uninstalling a workspace entry, or a malformed
   path with no filename.

   Why it matters: user-visible commands can abort the process instead of
   returning an actionable env error.

   Suggested fix direction: replace these assertions with `LocalWriteError` or
   metadata validation errors. Define behavior for editable/workspace uninstall
   and editable-to-installed conflicts.

   Suggested verification: seed `env.toml` with editable/workspace entries and
   malformed paths, then exercise sync and uninstall paths and assert errors, not
   panics.

6. Medium: sync trusts the lockfile version as an independent source of truth.

   Location: `core/src/commands/env/install.rs:110`,
   `core/src/commands/sync.rs:331`, `core/src/env/local_directory/mod.rs:449`,
   `core/src/env/local_directory/metadata.rs:199`.

   `do_env_install_project()` now accepts a separate `version` parameter, and
   sync passes `project.version` from the lock. The metadata writer later reads
   the installed project's `.project.json` and stores that version, while the
   directory name and overwrite checks used the caller-provided version. With a
   corrupt or inconsistent lock/source pair, the env can be installed under one
   versioned path while metadata records another.

   Why it matters: the new env layout relies on `env.toml` and directory paths
   staying coherent. Two version sources make that contract easier to break.

   Suggested fix direction: read the version from `ProjectRead` inside
   `do_env_install_project()` and verify any expected lock version matches it, or
   pass an explicit expected version and fail on mismatch before writing.

   Suggested verification: add a negative test where the lock version differs
   from `.project.json`; sync should fail before creating env metadata or dirs.

7. Medium: project path collision checks ignore the filesystem.

   Location: `core/src/env/local_directory/mod.rs:267`,
   `core/src/env/local_directory/mod.rs:458`,
   `core/src/env/local_directory/metadata.rs:221`.

   New project directory names are disambiguated only against paths already in
   metadata. If `sysand_env/lib/<computed-name>` exists but is absent from
   `env.toml`, install will move that target out of the way and replace it.

   Why it matters: partial migration, manual repair, interrupted writes, or
   downgrade/upgrade churn can leave untracked directories. Installing should not
   silently discard them.

   Suggested fix direction: check both metadata and filesystem existence when
   choosing a project path, or fail on untracked collisions.

   Suggested verification: create empty metadata plus a sentinel directory at
   the computed path, install that IRI/version, and assert the sentinel is not
   overwritten.

8. RESOLVED: Low: naming and comments make the new env model harder to understand.

   Location: `core/src/env/local_directory/mod.rs:45`,
   `sysand/src/lib.rs:739`, `core/src/context.rs:24`,
   `core/src/env/local_directory/metadata.rs:153`,
   `core/src/env/local_directory/metadata.rs:237`.

   `root_dir` means the `sysand_env` directory, not the project/workspace root.
   `get_env()` is documented as creating an env but only reads one.
   `ProjectContext.env` is documented only as `sysand_env`. `find_project()`
   returns all projects for an identifier, not one project. `EnvProject.path`
   says editable paths are relative to workspace root, while the implementation
   actually interprets them relative to the parent of `sysand_env`.

   Why it matters: this PR changes the persisted model, so misleading names make
   it harder to reason about the new invariants.

   Suggested fix direction: rename `root_dir` to `env_dir`; document `get_env()`
   as read-only; document `ProjectContext.env` as the discovered loaded env;
   rename `find_project()` to `projects_by_identifier()`; clarify editable path
   semantics.

   Suggested verification: documentation/code-review only.

9. RESOLVED: Low: stale old-layout references remain in docs/tests.

   Location: `docs/src/changelog.md:13`, `ARCHITECTURE.md:264`,
   `bindings/java/java-test/src/test/java/com/sensmetry/sysand/BasicTest.java:51`.

   Some docs still say filesystem envs use `entries.txt` / `versions.txt` or
   that those files will be removed in the future. A Java test message also
   refers to `env.toml` as the entries file.

   Why it matters: this reinforces the confusion that prompted the review: the
   old files are removed for filesystem envs, but still exist for WASM local
   storage.

   Suggested fix direction: document filesystem local env and WASM local-storage
   env separately. State that filesystem envs use `env.toml` and `lib/`, while
   WASM local storage still uses `entries.txt` / `versions.txt`.

   Suggested verification: docs/test text review.

10. Low: simplification opportunity in metadata lookup and directory naming.

Location: `core/src/env/local_directory/metadata.rs:100`,
`core/src/env/local_directory/metadata.rs:139`,
`core/src/env/iri_normalize.rs:124`,
`core/src/env/local_directory/mod.rs:267`.

Metadata lookup has several similar helpers with subtly different names.
Directory-name generation also carries a large Unicode normalization path,
even though `env.toml` is now the authoritative mapping from identifier to
path.

Why it matters: this is not necessarily a correctness bug, but it increases
review and maintenance cost in the new env implementation.

Suggested fix direction: centralize the identifier/version matching predicate
and expose only the operations needed by callers. Consider whether project
dirs can use a simpler deterministic slug plus hash suffix, while keeping
human readability as a non-authoritative convenience.

Suggested verification: existing filename and env tests should continue to
pass; add collision tests if changing naming.

## Verification

Subagents reported running:

- `cargo test -p sysand --test cli_env --test cli_sync`
- `cargo test -p sysand-core --features filesystem --test filesystem_env`
- `cargo test -p sysand-core --test filesystem_env --features filesystem`
- `cargo test -p sysand --test cli_env`
- Manual legacy-env reproduction, which failed with
  `refusing to overwrite .../sysand_env`.
- Manual path-traversal reproduction with `path = "../victim"`, which removed a
  sibling file.

Recommended additional checks:

- `cargo test -p sysand --test cli_lock`
- Negative tests for legacy env migration/detection, path traversal in
  `env.toml`, deletion failure consistency, alias persistence, version mismatch,
  and editable/workspace conflict handling.
