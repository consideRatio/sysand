# Lessons Learned — Migration Execution

Key insights from executing plans A through D that should inform any
re-execution or continuation of this work.

## Critical: Facade must cover full orchestrations, not just building blocks

The biggest mistake was implementing `env::install_project` and
`clone::clone_project` as "building blocks" that only handle one step,
leaving the CLI to orchestrate resolve → lock → sync itself. This
created two problems:

1. **The CLI can't migrate away from core internals.** It still imports
   `do_lock_extend`, `do_lock_projects`, resolver types, Lock types,
   etc. directly because the facade doesn't expose the full operation.

2. **Every binding that wants "install with deps" must reimplement the
   orchestration.** The facade promise — mechanical 5-10 line binding
   wrappers — breaks for these commands.

**The fix:** The facade should expose:
- `env::install(ctx, net, iri, opts)` — full orchestration: resolve
  the IRI, lock deps, sync env. Not just "install a pre-resolved
  project."
- `clone(net, locator, opts)` — full orchestration: resolve, fetch to
  target, optionally lock + sync deps.
- `usage::add(ctx, net, iri, opts)` — with UpdateMode::Sync doing the
  full resolve + lock + sync, not just manifest edit.

These are the commands users actually run. The building blocks
(`install_project`, `clone_project`) can exist for advanced use cases
but they shouldn't be the primary facade.

**If re-executing:** Design facade functions top-down from user
commands, not bottom-up from internal function signatures. Ask "what
does `sysand env install urn:x ^1.0` do end-to-end?" and make the
facade do exactly that.

## NetworkContext must be generic over auth (RPITIT)

`HTTPAuthentication` uses `impl Future<...>` in trait methods (RPITIT),
making it not object-safe. `Arc<dyn HTTPAuthentication>` does not
compile. `NetworkContext<Policy>` must be generic over the auth type.

This was discovered during implementation and couldn't have been
predicted from the spec alone.

## From<BindingError> for SysandError needed per binding crate

Each binding crate has its own storage error type (JS/WASM has
`io::local_storage::Error` and `env::local_storage::Error`). The
facade's `where P::Error: Into<SysandError>` bound requires each
binding to implement `From<BindingError> for SysandError`.

This is not onerous (5-10 lines per error type) but must be done for
each binding and wasn't anticipated in the original plans.

## Additive approach is correct for restructuring

Adding `facade/` and `types/` alongside existing code (not moving to
`internal/`) was the right call. It kept everything compiling at every
step. The physical move to `internal/` is blocked until the facade
covers all orchestrations — trying to do it earlier would have broken
everything.

## Single error converter is the highest-ROI change

Across all three bindings, replacing per-function error matching with
one `sysand_err()` function deleted more code than any other single
change (~130 lines in Python alone). If only one thing from this plan
were executed, it should be plan A (unified error model).

## Feature gates are pervasive and easy to get wrong

Many core types are behind `#[cfg(feature = "filesystem")]` or
`#[cfg(all(feature = "filesystem", feature = "networking"))]`. Every
`From` impl and every facade function must have matching gates. Test
with both `cargo check` (default features) and
`cargo check --features filesystem,networking`.

## Resolver assembly is the key piece of shared infrastructure

The `create_resolver` + `get_overrides` logic (~80 lines) was
duplicated in the CLI's lock.rs, env.rs, add.rs, and clone.rs. Moving
it to `facade::resolver` was high-value — it's the one piece of
infrastructure that every network command needs. If re-executing, move
this first, before migrating individual commands.

## CLI command tree restructure is independent of facade migration

Renaming `add` → `usage add`, `include` → `source add`, etc. was a
clap parser change that didn't depend on the facade at all. It could
have been done first or last. The flag renames (`--no-*` → positive
enums) similarly only affect the CLI dispatch layer.

## Java accessor chain is pure Java, no Rust changes

`SysandClient` with `client.env().create()` is a Java-side wrapper
over existing JNI native methods. No Rust/JNI changes needed. Can be
done independently at any time.

## JS namespace wrapper pattern has 3 touch points per command

Every new WASM command needs changes in:
1. Rust `lib.rs` — `#[wasm_bindgen]` export
2. `src/sysand.js` — re-export into namespace
3. `src/sysand.d.ts` — TypeScript type declaration

This is mechanical but must be remembered. The `js-sys` dependency
should be conditional on the `browser` feature like `web-sys`.
