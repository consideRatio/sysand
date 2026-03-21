# Exploration 0011: IRI Variants and Source Types

## Context

Several commands accept what ADR-0004 calls `<IRI>` or `<IRI_OR_URL>`,
but this hides real complexity. The reference implementation reveals 8
distinct source types, and the CLI has multiple flags for specifying them
(`--from-path`, `--from-url`, `--as-editable`, `--as-local-src`,
`--as-local-kpar`, `--as-remote-src`, `--as-remote-kpar`,
`--as-remote-git`). This exploration inventories the source types and
proposes how they should surface in the reworked design.

## Source Types in the Reference Implementation

From the `Source` enum in `core/src/lock.rs`:

| Variant | Resolves from | Data |
| ------- | ------------- | ---- |
| `Editable` | Local path (relative to workspace) | Unix path |
| `LocalSrc` | Local path to source directory | Unix path |
| `LocalKpar` | Local path to .kpar archive | Unix path |
| `Registry` | Package index lookup by IRI | Registry URL |
| `RemoteKpar` | URL to .kpar archive | URL + optional size |
| `RemoteSrc` | URL to remote source | URL |
| `RemoteGit` | URL to git repo | URL |
| `RemoteApi` | API endpoint | URL |

These fall into three categories by how you locate the package:

### 1. Index lookup (Registry)

You have an IRI like `urn:example:sensors`. You query a package index.
The index returns metadata and a download location. This is the standard
package manager flow.

### 2. Direct reference (URL-based)

You have a URL pointing directly at the package:
- `https://example.com/sensors-2.0.kpar` → remote KPAR
- `https://example.com/sensors/` → remote source
- `https://github.com/example/sensors.git` → git repo
- `https://api.example.com/v1/packages/sensors` → API endpoint

No index involved. You know where the package is.

### 3. Local reference (path-based)

You have a path on disk:
- `./libs/sensors/` → local source directory
- `./archives/sensors-2.0.kpar` → local KPAR archive
- `./libs/sensors/` (editable) → like local source, but changes are
  picked up live without reinstalling

## Which Commands Need Which Source Types?

| Command | Index | Direct URL | Local path |
| ------- | ----- | ---------- | ---------- |
| `resolve show` | Yes | ? | No |
| `resolve info *` | Yes | ? | No |
| `usage add` | Yes | Yes | Yes |
| `project clone` | Yes | Yes | Yes |
| `env install` | Yes | Yes | Yes |
| `env sync` | Yes | Yes | Yes (from lock) |

`resolve` is the interesting case. Its purpose is querying package
metadata — "tell me about this package." For index lookups, that's
clear: query the index, get metadata. For a direct URL or path, it
would mean: fetch/read the package, extract its metadata. That's
technically possible but conflates "query an index" with "inspect an
artifact."

## The Overloaded IRI Problem

The current design uses `<IRI_OR_URL>` as a single positional argument
and relies on parsing to determine the source type. The reference CLI
adds `--from-path`, `--from-url`, `--as-*` flags to disambiguate. This
is messy because:

1. Auto-detection is fragile — is `sensors` a path or a short IRI?
2. The flag explosion (`--as-local-src`, `--as-local-kpar`,
   `--as-remote-src`, `--as-remote-kpar`, `--as-remote-git`) is hard
   to learn
3. The same flag means different things depending on the command

ADR-0004 simplified this to `--source-kind <KIND> --source <VALUE>` as
a paired flag, which is cleaner. But we haven't defined what `<KIND>`
values exist.

## Proposed Source Kind Taxonomy

Flatten the 8 reference variants into a smaller set of source kinds
that users actually think about:

| Kind | What it means | Example |
| ---- | ------------- | ------- |
| `index` | Look up in a package index (default) | `urn:example:sensors` |
| `path` | Local directory containing a project | `./libs/sensors` |
| `kpar` | Local or remote KPAR archive | `./sensors.kpar` or `https://example.com/sensors.kpar` |
| `git` | Git repository | `https://github.com/example/sensors.git` |

Notes:
- `editable` is a mode of `path`, not a separate kind — could be a
  flag like `--editable` on `usage add`
- `RemoteSrc` and `RemoteApi` from the reference are obscure. Do we
  need them? A remote source directory is unusual. An API endpoint is
  an implementation detail of an index.
- Local vs remote is determined by whether the value is a path or URL,
  not a separate kind. `--source-kind kpar --source ./local.kpar` vs
  `--source-kind kpar --source https://example.com/remote.kpar`.

### How `--source-kind` and `--source` Work

```
# Default: index lookup
sysand usage add urn:example:sensors

# Explicit index
sysand usage add urn:example:sensors --source-kind index

# Local path
sysand usage add urn:example:sensors --source-kind path --source ./libs/sensors

# KPAR archive (local)
sysand usage add urn:example:sensors --source-kind kpar --source ./sensors.kpar

# KPAR archive (remote)
sysand usage add urn:example:sensors --source-kind kpar --source https://example.com/sensors.kpar

# Git repo
sysand usage add urn:example:sensors --source-kind git --source https://github.com/example/sensors.git
```

The IRI is always the package identifier. The source kind and source
value tell the system *how to get it*, overriding the default index
lookup.

## Implications for Resolve

If resolve only does index lookups, it's simple: `resolve` takes an
IRI and optional version constraint, queries the index, returns
metadata. No `--source-kind`, no `--relative-root`, no ambiguity.

"Tell me about a package at this URL" or "tell me about a package at
this path" would be a different operation — closer to `project show`
but for a remote/archived package. Possibly:

```
sysand project show --source-kind kpar --source ./sensors.kpar
sysand project show --source-kind git --source https://github.com/example/sensors.git
```

But this overloads `project show` (which normally operates on a
`ProjectContext`). Alternative: a separate `inspect` verb that reads
metadata from any source without requiring a local project.

This is a design question worth deferring — the common case is index
lookup, and that's what resolve should do first.

## Implications for `--relative-root`

`--relative-root` was needed because local paths in the old design
were relative, and you needed to specify relative to what. With the
proposed model:

- Index lookups: no paths involved, `--relative-root` irrelevant
- `--source-kind path`: the value is a path, resolved against CWD
  in the CLI or taken as-is in the library. No `--relative-root` needed.
- `--source-kind git`: URL, not a path. No `--relative-root` needed.
- `--source-kind kpar`: path or URL. Paths resolved against CWD.

**`--relative-root` can be dropped.** Path resolution is handled the
same way as every other path in the CLI: relative to CWD, or explicit.

## Binding Projections for Source Kind

**Rust:**

```rust
usage::add(
    &ctx,
    "urn:example:sensors",
    Some("^2.0"),
    UsageAddOptions {
        source: Some(SourceSpec {
            kind: SourceKind::Git,
            value: "https://github.com/example/sensors.git".into(),
        }),
        ..Default::default()
    },
)?;
```

**Java:**

```java
client.usage().add(ctx, "urn:example:sensors", "^2.0",
    new UsageAddOptions()
        .source(new SourceSpec(SourceKind.GIT,
            "https://github.com/example/sensors.git"))
);
```

**JS/WASM:**

```ts
await sysand.usage.add(ctx, "urn:example:sensors", "^2.0", {
  source: { kind: "git", value: "https://github.com/example/sensors.git" },
});
```

**Python:**

```python
sysand.usage.add(ctx, "urn:example:sensors", "^2.0",
    source=SourceSpec(SourceKind.GIT,
        "https://github.com/example/sensors.git"))
```

## Open Questions

1. Do we need `RemoteSrc` (remote source directory) and `RemoteApi`
   as source kinds? Or are these edge cases that can be deferred?
2. Should `editable` be a source kind or a flag on `usage add`?
3. Should there be an `inspect` command for reading metadata from
   arbitrary sources (paths, URLs, KPARs) without an index lookup?
4. Git source: do we need to support branches, tags, or subdirectories?
   If so, how? Cargo uses `--branch`, `--tag`, `--rev` flags.
