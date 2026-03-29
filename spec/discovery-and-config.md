# Discovery and Config

How projects and workspaces are found, and how configuration is loaded.
For type definitions (`ProjectContext`, `WorkspaceContext`, `ConfigMode`),
see `public-api.md`.

## Locate

The library provides `locate(path)` and
`workspace::locate(path)` — explicit operations that walk up from a
given path to find `.project.json` or `.workspace.json` respectively.

- Returns a path, not a context
- Errors if the filesystem root is reached without finding the target
- The caller constructs a `ProjectContext` or `WorkspaceContext` from
  the result

```rust
let path = locate("./deeply/nested/dir")?;
let ctx = ProjectContext::new(path);
```

Implicit locate (using CWD when `--project`/`--workspace` is omitted)
is CLI-only behavior.

## Config

Config controls two things:

- Package index URLs (with default/non-default distinction and priority)
- Project source overrides (mapping IRIs to specific sources)

### Config Loading

The facade takes a `Config` value as a parameter. It never reads
config files itself. The caller decides how to construct it.

Three loading functions are available:

| Function                   | Reads                                              | Use case                        |
| -------------------------- | -------------------------------------------------- | ------------------------------- |
| `get_config(path)`         | A single config file                               | Library — project config only   |
| `load_configs(project_path)` | User-level + project-level, append-only merge    | CLI — full config with defaults |
| Manual construction        | Nothing — caller builds `Config` directly          | Library — custom indexes/overrides |

`load_configs` reads `<user-config-dir>/sysand/sysand.toml` (if it
exists), then `<project>/sysand.toml`, and appends the entries from
both. The user-level file adds indexes and overrides to the list — it
cannot remove or shadow project-level entries.

Library callers that want CLI-like behavior can call `load_configs`.
Library callers that want isolation call `get_config` with the
project config path, or construct `Config` directly.

### What the Facade Receives

The facade receives a `Config` value and the project/workspace path.
From the project path it reads:

- `<project>/.project.json` — project info
- `<project>/.meta.json` — project metadata

Config is not read from disk by the facade — it arrives as a
parameter.

Given a workspace path:

- `<workspace>/.workspace.json` — workspace info

## Authentication

The facade takes an auth policy as a parameter alongside `Config`.
It never reads credentials from the environment itself.

### Schemes

Two authentication schemes are supported:

| Scheme | Credentials              | When used                     |
| ------ | ------------------------ | ----------------------------- |
| Basic  | username + password      | HTTP indexes, remote sources  |
| Bearer | token                    | HTTP indexes, remote sources  |

Credentials are scoped to URL glob patterns (e.g.,
`https://*.corp.com/**`). A request first tries unauthenticated
access; credentials are sent only after a 401/403 response.

### Builder API

Library callers construct an auth policy with a builder:

```rust
let auth = AuthBuilder::new()
    .add_basic("https://*.corp.com/**", user, pass)
    .add_bearer("https://github.com/**", token)
    .build()?;

facade::lock_update(&project, &config, &auth, ...)?;
```

Callers that need no authentication pass a default (unauthenticated)
policy. Callers that need custom schemes (OAuth, mTLS) can implement
the auth trait directly.

### CLI Credential Loading

The CLI reads credentials from environment variables:

```
SYSAND_CRED_<NAME>              URL glob pattern
SYSAND_CRED_<NAME>_BASIC_USER   Basic auth username
SYSAND_CRED_<NAME>_BASIC_PASS   Basic auth password
SYSAND_CRED_<NAME>_BEARER_TOKEN Bearer token
```

Each credential set needs the glob pattern plus exactly one scheme
(basic or bearer, not both). Multiple credential sets can coexist
with different names.

```sh
export SYSAND_CRED_CORP="https://*.corp.com/**"
export SYSAND_CRED_CORP_BASIC_USER="alice"
export SYSAND_CRED_CORP_BASIC_PASS="secret"

export SYSAND_CRED_GITHUB="https://github.com/**"
export SYSAND_CRED_GITHUB_BEARER_TOKEN="ghp_..."
```

The CLI builds an auth policy from these variables and passes it to
the facade. This is the same pattern as config loading — the CLI
reads the environment, the library receives a value.

## What Stays CLI-Only

- Implicit locate from CWD when `--project`/`--workspace` is omitted
- User-level config loading (via `load_configs`) as the default
- Credential loading from `SYSAND_CRED_*` environment variables
- Terminal concerns: color, prompting, log level, output format

## Source Overrides and Resolution

### Manifest vs Config Separation

The manifest (`.project.json`) stores _what_ a project depends on:
IRI + optional version constraint. It never stores source information.

The config (`sysand.toml`) stores _where_ to get things: source
overrides that map IRIs to specific locations, and index URLs.

The lockfile (`sysand-lock.toml`) stores _exactly what_ was resolved:
the specific version and source that was actually used.

### How Source Overrides Work

Config can override how a specific IRI is resolved:

```toml
[[project]]
identifiers = ["urn:example:sensors"]
sources = [{ src_path = "../libs/sensors" }]
```

During resolution (lock update, env sync), the resolver checks config
overrides before querying indexes. If config maps an IRI to a local
path, git repo, or KPAR, that source is used instead of the index.

### Only the Root Project's Config Applies

The resolver is built once from the root project's `sysand.toml`.
That same resolver is used for the entire dependency tree.
Dependencies' own `sysand.toml` files are not read during resolution.

This means source overrides from the root project apply globally —
if the root config says `urn:example:sensors` lives at a local path,
that override applies even when resolving transitive usages of
`sensors` from other packages.

This is intentional: the person running `lock update` controls where
everything comes from.

### Resolver Priority

During resolution, sources are checked in this order:

1. Config overrides (from root project's `sysand.toml`)
2. Standard library IRIs (built-in)
3. Standard resolver: local environment, HTTP indexes, file, git

## Rationale

**Why locate is a library operation.** The original design placed all
discovery in the CLI. In practice, library and binding users face the
same problem — given a path inside a project, find the root. Forcing
every consumer to reimplement upward traversal is unnecessary
duplication. Locate was promoted to the library, but no library
operation calls it implicitly — the caller decides when traversal
happens. This preserves the predictability guarantee.

**Why the facade takes config as a parameter.** Other package managers
(Cargo, pip, Poetry) load user-level config implicitly — but they are
CLI tools with no supported library API. Sysand has a library API
across four surfaces. Implicit config loading in the library raises
the question "whose `~/.config` do you read?" — the developer's, the
CI runner's, the build server's. The answer is: the caller decides.
The facade receives a `Config` value; it never touches the filesystem
for config. `load_configs` (user + project, append-only merge) is
available for callers that want CLI-like behavior, but it's opt-in.

**Why auth is a facade parameter.** Same reasoning as config — the
library can't know whose environment variables or keychain to read.
The CLI reads `SYSAND_CRED_*` env vars and builds a policy; library
callers construct their own. The builder API covers the common cases
(basic, bearer, URL-scoped). The trait is open for custom schemes.

**Why unauthenticated-first.** Trying without credentials before
sending them avoids leaking tokens to servers that don't need them.
Credentials are only sent to URLs matching the glob pattern, and only
after the server responds with 401/403.

**Why env vars, not config files for credentials.** Credentials in
config files get accidentally committed to version control. Env vars
are the standard mechanism for CI and deployment systems. The
`SYSAND_CRED_*` convention groups related variables by name and is
easy to set in CI secrets, `.envrc` files, or shell profiles.

**Why discovery from CWD is CLI-only.** The library API is predictable:
explicit paths in, no hidden state. Implicit CWD discovery is a
terminal convenience that doesn't translate to library and binding
contexts where the caller knows their project path.
