# Credential storage and authentication commands (plan)

> **Status: plan / draft.** Not yet a stable contract. This document
> captures the agreed design for persisting index credentials and the
> `sysand auth` command family, so implementation can proceed in phases.

## 1. Goal

Let users store an index credential once and have `sysand` reuse it across
runs, on Windows, macOS, and Linux, without `sysand` owning any secret
storage or cryptography. Acquisition of the token (GitLab/GitHub PAT,
Sysand Index API token) is out of scope: `sysand` stores what the user
provides.

v1 is deliberately narrow: bearer tokens, `login` / `logout` / `status`.
Basic auth, raw-pattern commands, and the P2 protocol change are deferred
(§10).

## 2. Current state

- **Request-time application of credentials already exists** in
  `core/src/auth.rs`: a per-URL glob map of auth policies (bearer, basic),
  combinators that try unauthenticated first and send the secret only on a
  4xx, and same-host redirect handling. The policy is built **eagerly and
  immutably** at startup in `sysand/src/lib.rs` from `SYSAND_CRED_*`.
- **Publish OIDC (trusted publishing)** exists in
  `core/src/commands/publish.rs` for CI.
- **Token sourcing is a stopgap:** `sysand/src/lib.rs` scans
  `SYSAND_CRED_*` environment variables on every run. There is no
  persistence: a user re-supplies env vars each invocation.

The gap this plan fills is the missing middle: persist a credential once,
retrieve it on later runs, feed it into the glob-based auth layer.

## 3. Model and constraints

Three surfaces, and one access rule:

| Surface    | Probe path                 | Creds required? |
| ---------- | -------------------------- | --------------- |
| Discovery  | `sysand-index-config.json` | sometimes       |
| Index root | `index.json`               | sometimes       |
| API root   | `v1/whoami`, `v1/upload`   | always          |

Constraints:

- **C1 - unified read access.** Discovery and index root share one auth
  status (public or private together). This removes pathological split
  permutations and collapses the space to a 2x2.
- **C2 - one credential per index.** `auth login` stores one credential per
  index, used for both the read leg and the API leg. Separate read/write
  tokens are not a v1 concept (a later `auth set` could cover them, §10).
- **P2 - API presence is read from discovery (consumed, not enforced
  here).** This plan treats an index as having an API iff its discovery
  document advertises `api_root`, and derives globs accordingly. It does
  **not** change the runtime default (today a plain-URL index still defaults
  `api_root` to its root). Enforcing "`api_root` required" at the protocol
  level, and dropping that default, is a **separate, decoupled change**
  (§12), out of this plan's phases.
- **P1 - public discovery: not required.** Under C2 the single credential
  reads discovery on a private index, so there is no bootstrap paradox.
  Current behavior (discovery may be private) stands.

The collapsed situation space:

| #   | Read surface | API     | Example                     | Creds used for  |
| --- | ------------ | ------- | --------------------------- | --------------- |
| S1  | public       | none    | public static index         | nothing         |
| S2  | private      | none    | private static index        | read            |
| S3  | public       | present | official sysand.com         | write (publish) |
| S4  | private      | present | fully private dynamic index | read + write    |

## 4. Command surface (v1)

Under a `sysand auth` namespace:

| Command                          | Role                                                                            |
| -------------------------------- | ------------------------------------------------------------------------------- |
| `sysand auth login [index-url]`  | validated, index-keyed bearer credential (see §5); no URL = the default index   |
| `sysand auth logout [index-url]` | remove an index login; no URL = the default index (symmetric with `login`)      |
| `sysand auth status`             | list stored credentials (never secrets), backend, and `SYSAND_CRED_*` shadowing |

- **Bearer only in v1.** The token is entered via a hidden prompt
  ("Enter API token for `<index>`:") or `--token-stdin`, never an inline
  value flag (shell-history / `ps` leakage). Basic auth (`--username`) and
  raw-pattern `auth set` / `unset` are deferred (§10); request-time basic
  auth via `SYSAND_CRED_*` still works.
- **Non-interactive safety.** If stdin is not a TTY and `--token-stdin` was
  not given, `login` fails fast ("no terminal for prompt; pass the token
  with `--token-stdin`") instead of hanging or reading a pipe as a secret.
- **Default index.** `sysand auth login` with no URL targets the configured
  default index (the same resolution `publish` uses), so onboarding to
  sysand.com is a bare command.
- **Glob derivation** (§8): automatic from the URL; no manual `--pattern` in
  v1. If derivation is ever wrong for an unusual layout, the `SYSAND_CRED_*`
  env var is the escape hatch until `--pattern` / `auth set` land (§10).

The index URL is normalized (trailing slash, scheme) before use as the
storage key and for glob derivation, so different spellings do not create
duplicate entries.

## 5. Validation

`auth login` takes `--validation true|false` (default `true`). It maps to
an `Option<bool>` argument (absent = `None` = the default), so the language
bindings expose a clean optional keyword: `validation: Optional[bool] =
None`. This intentionally diverges from the repo's `--no-<flag>` boolean
convention (for example `--no-lock`), which binds as a required,
negative-sense `no_lock: bool`; a positive `Option<bool>` reads better as an
optional keyword across the py/js/java bindings.

- `--validation true` (default): probe every surface the index supports and
  store unless the credential is rejected everywhere it was actually tested
  (see the refusal rule below). A static index has only the read surface; a
  dynamic index adds the API.
- `--validation false`: store without any credential probe. Discovery is
  still fetched best-effort for glob scoping (§8); if unreachable, fall back
  to the URL-derived glob with a warning. Use it offline, or when a probe
  would false-refuse.

Validation is a boolean, not per-surface levels: since `v1/whoami` checks
only that a token is _accepted_ by the API (identity, not capability, §6),
validating everything almost never wrongly refuses a valid token, so a
"read-only" level would add a choice without payoff. `--validation` could
later give way to a levelled flag without disrupting this default.

**Probe mechanism.** Validation cannot reuse the runtime unauth-first
policy, which returns only the final response and cannot report whether a
surface actually _accepted_ the credential. Each surface is probed as an
**unauth baseline then a forced-auth retry**: a surface counts as
accepted/tested only when the unauth baseline was a 4xx and the forced retry
then succeeded, so a public surface (200 unauth, credential never sent) is
correctly "not tested", not "accepted". The API surface (`v1/whoami`) is
always authenticated, so its baseline is a known 401 and only the forced
request is needed. Validation is discovery-first (`api_root` is known only
after reading discovery): fetch discovery, resolve `index_root` and
`api_root`, probe `index_root/index.json`, and, **only if discovery
advertised an `api_root`** (not the runtime plain-URL default, §3), probe
`api_root/v1/whoami`, so a static plain-URL index is never phantom-probed
for an API it does not have.

**Refusal rule.** Store if the credential is _accepted by any surface it
actually tested_, warning about any surface that rejected or was
unreachable. Refuse only when at least one exercised surface rejected the
credential and none accepted it. A surface counts as "tested" only if the
credential was exercised: a _public_ read surface returns 200 without
sending the credential, so it proves nothing. If nothing exercised the
credential (fully public read with no API, or every probe unreachable),
store as "stored, not verified".

This self-adjusts across the situation space:

- Private index, read works, API rejects: store with an "API access failed"
  warning (the token is still useful for reading).
- Public-read index (for example sysand.com): the read probe never tests
  the token, so `v1/whoami` is the only real test, and a rejected token is
  refused, keeping the publish flow protected.
- Every exercised surface rejects: refuse.

Never print a bare "verified"; always scope the claim to the surfaces that
actually accepted the credential.

## 6. The `v1/whoami` endpoint

New endpoint on the index API (server side, the `sysand-index` Django app),
under `api_root`. Its purpose is credential validation and identity for
`auth status`.

- `GET api_root/v1/whoami`, bearer credential. The server routes it under
  `api/` (`api/v1/whoami`); `api_root` carries the `/api/` segment, so the
  client's `api_root/v1/whoami` join is consistent with `v1/upload`.
- `200` on a valid, unexpired token; `401` otherwise. Under
  `--validation true` a `200` passes the API leg (§5). The `401` body is
  unspecified (the client only reads the status).
- Body on `200`:

```json
{
  "subject": { "type": "user", "name": "alice" },
  "token": {
    "name": "laptop",
    "prefix": "siu_1a2b3c4d",
    "expires_at": "2026-09-01T00:00:00Z"
  }
}
```

- `subject.type` is `user`, `project`, or `oidc`, so the endpoint is
  principal-agnostic (hence `whoami`, not `user`). `subject.name` is the
  **username** for a user token, the **project id** (`publisher/name`) for a
  project token, and the **publisher identity** for an OIDC token; it is
  distinct from `token.name` (the user-given token label).
- `token.expires_at` is always **returned** by whoami (the model's
  `expires_at` is non-nullable); the stored record persists it only when a
  validating login actually ran, hence "expires_at-if-known" in §8/§9.
  `token.prefix` is the non-secret display prefix (type prefix + first 8
  hex).
- No `can_publish` flag and no scope list: at login there is no target
  project and every valid token can publish somewhere, so a capability
  boolean is vacuous; per-project authorization stays enforced at the upload
  (its existing `403`). An optional `?project=<id>` pre-flight may be added
  later.

## 7. Publish interaction

Publish is two legs with **different** credential handling:

- **Leg 1, discovery read** (`sysand-index-config.json`, to resolve
  `api_root`): uses the general read auth policy, unauth-first, authenticated
  only if the index is private. `login` scopes the credential to cover the
  discovery/index root (§8), so a private index's discovery fetch gets it.
- **Leg 2, upload** (`POST api_root/v1/upload`): **bearer only**, sent
  proactively (an upload cannot be tried unauthenticated then retried), so
  publish reads the keyring up front here (one keychain access) and selects
  the bearer whose glob matches the upload URL.

The one change to publish's bearer selection is **source precedence**: try
env bearer matches first (single match within env), then keyring (single
match within keyring), instead of one flat "exactly one match or error" over
the merged set. Within a source the existing exactly-one rule stands (its
`AmbiguousPublishBearer` error becomes per-source). Concretely this changes
`try_into_publish_bearer_auth_map` / `resolve_publish_bearer_from_config` in
`core/src/commands/publish.rs` to keep env and keyring as **two maps** (or
source-tagged) with a two-stage lookup, rather than collapsing to one flat
`GlobMap`. This makes the stated precedence real, a CI `SYSAND_CRED_*`
overrides an interactive login. The two-leg flow and trusted publishing are
otherwise unchanged.

- **Trusted-publishing precedence:** in `auto` mode publish uses OIDC
  trusted publishing when a supported CI environment is detected, and
  otherwise falls back to the bearer map (env > keyring). CI has no keyring,
  so the two rarely coexist.
- **Basic auth cannot publish** (leg 2 is bearer-only); a basic
  `SYSAND_CRED_*` entry is ignored for the upload.
- **No matching bearer** fails up front (before the upload) with a hint to
  run `sysand auth login <index>` to store a publish token.
- **Fail fast on expiry (advisory):** if the selected bearer carries a known
  `expires_at` (§9) already past, publish warns and stops before uploading
  the archive, pointing at `sysand auth login`. Because a fast client clock
  could false-trip this, it is advisory (the server's `401` remains the
  authority); allow a small skew margin.

## 8. Glob scoping and conflict resolution

- **Source precedence, single match within a source.** For a given URL, all
  `SYSAND_CRED_*` (env) matches take precedence over all keyring matches
  (so CI can override an interactive login). Within one source, the existing
  single-match rule applies (publish errors on a within-source ambiguity;
  reads try-all). v1 deliberately does **not** add longest-prefix
  tie-breaking, that is only needed once raw-pattern `auth set` or
  same-host nested logins create within-source overlaps (§10).
- **Glob coverage.** `login` anchors the primary glob on the **discovery URL
  the user supplied** (so the discovery fetch itself is authenticated), and
  additionally covers the resolved `index_root` and `api_root` when they
  diverge from it, minimal and non-overlapping. Templated URLs are anchored
  before `{path}` / `{path_raw}`.
- **Divergent `api_root` (Case B).** If `api_root` nests under the derived
  root (Case A), one glob suffices. If it is a disjoint host/path, store the
  same credential under both globs (minimal, non-overlapping), so the upload
  URL matches exactly the api glob. Templated indexes are inherently Case B
  (their `api_root` is a disjoint plain URL).
- Each login is one record (`{key, globs, scheme, secret, expires_at-if-known}`)
  inside the single keyring blob (§9), so `logout` removes it and `status`
  shows one login covering N patterns.
- **Discovery changes over time (globs are a login-time boundary).** Reads
  and publish re-fetch discovery live each run, but the stored globs are the
  login-time snapshot and are **not** auto-updated from discovery. This is
  deliberate: auto-following a changed `api_root`/`index_root` would let a
  changed (see the trust model below) discovery silently redirect the stored
  token to a new host. So when a discovery change moves a root **outside**
  the login's globs, the credential stops matching and the request fails
  cleanly rather than following, either way safe. **Best-effort diagnostic
  (may land later):** where sysand can correlate the failing request with a
  login whose snapshot globs no longer cover the resolved root, it prints
  "the index configuration has changed since you logged in; re-run
  `sysand auth login <index>` to update". This correlation is non-trivial on
  the read path (the auth layer sees per-request URLs, not the resolved
  index identity), so if it does not ship in the first cut the generic "no
  bearer / re-run login" hint applies. Re-login re-derives the globs and
  re-validates.
  **Caveat:** this boundary covers the login's own globs only. A broad
  `SYSAND_CRED_*` env pattern that also matches the moved root can still
  shadow it (env is user-controlled and takes precedence), so the guarantee
  is "sysand does not itself auto-follow discovery", not "no configured glob
  can ever match the new root".

**Trust model.** The discovery document at the URL you supply is the trust
anchor: `sysand` sends the credential to the `index_root`/`api_root` it
advertises (including a different host) and to `v1/whoami`, with no
same-origin or HTTPS restriction. Trusting the discovery URL means trusting
what it points at. Note the amplification honestly: over plain `http`, a
_one-time_ MITM at login can rewrite discovery to a hostile `api_root`,
which both leaks the freshly entered token and gets **persisted** as a glob,
so it keeps being sent there until re-login, not merely a single
eavesdropped request. `http` (localhost or a trusted LAN) is still
supported; the full transport-security guidance lives in the docs (§13).

## 9. Storage, consumption, precedence

- **Backends:** OS keyring by default (macOS Keychain, Windows Credential
  Manager, Linux Secret Service via the `keyring` crate), with environment
  variables as the automatic fallback where no keyring exists. **No
  plaintext credentials file, ever.**
- **Single keyring entry.** All persisted credentials live in **one**
  keyring entry (for example `service = "sysand"`, `account =
"credentials"`) holding a JSON blob: a list of records `{key, globs,
scheme, secret, expires_at-if-known}`. Deliberate over a manifest file:
  the `keyring` crate cannot portably enumerate entries, and one blob is
  **atomic** (no metadata/secret drift), needs **no file**, and prompts the
  keychain at most once. `login` / `logout` read-modify-write the blob;
  `status` reads it.
  - **Windows size limit.** Windows caps a blob at ~2.5 KB
    (`CRED_MAX_CREDENTIAL_BLOB_SIZE` = 2560). With small tokens that is
    roughly ten entries; large JWTs, fewer, and a single token can exceed it
    on the first login. One message covers both: "credential store full on
    this platform (Windows ~2.5 KB limit); remove an unused login or use a
    smaller token", and `status`/the error flag stale or expired entries so
    the user knows what to drop.
- **Concurrency.** Read-modify-write is guarded by a **cross-process file
  lock** at a fixed path (a lock file is not a credentials file, so it is
  permitted, pick a path that works even when only a keyring, and no writable
  config dir, exists), because parallel `sysand` invocations are real; an
  in-process mutex alone would lose one writer's record.
- **Consumption and keyring access.** The blob is read only when a
  credential might actually be needed, to avoid unnecessary keychain prompts.
  This requires a credential source the auth policy consults on demand. The
  natural shape (implementable from the existing combinators) is
  `SequenceAuthentication<EnvLayer, LazyKeyringLayer>`: the env layer is the
  existing eager `RestrictAuthentication` from `SYSAND_CRED_*` (no keychain),
  and the lazy keyring layer is consulted only in `SequenceAuthentication`'s
  4xx-escalation branch, so the blob read happens exactly when needed and
  env-before-keyring falls out for free. Note it can **not** be a
  `RestrictAuthentication` with a lazy inner map (that classifies the URL up
  front and would force the read). This replaces the eager immutable policy
  built in `sysand/src/lib.rs`, and ripples into the concrete
  `StandardHTTPAuthentication` alias used by `command_publish` and
  `try_into_publish_bearer_auth_map`, which must accept the new type. Because
  `LazyKeyringLayer` holds a cache (`OnceCell`/`Mutex`) it is not `Clone`, so
  publish's `Arc::unwrap_or_clone(...).try_into_publish_bearer_auth_map()`
  (publish.rs) must become a **by-ref** extraction (it already clones secrets
  into the new map), or the cache must be `Arc`-wrapped. It defers the blob
  read to the first auth-relevant 4xx (or publish / `auth` command), reads
  the whole blob once, and caches it for the process. Escalation semantics to
  pin at implementation: a **failed** env credential (env 4xx) escalates into
  the keyring layer, and when the keyring layer has no matching record it
  re-issues the unauthenticated request to produce the final response.
  - **Never read** for local/offline commands, for reads that succeed
    unauthenticated (public indexes return 200 and never touch the keyring),
    or for users who never ran `auth login` (no entry: a cheap "not found",
    no unlock).
  - **Read once, then cache** on the first auth-relevant 4xx, on publish's
    upload leg, and on the `auth` commands. At most one keychain touch per
    command.
  - Reads escalate on **any** 4xx (not just 401/403), because some hosts
    (GitLab) answer `404` on missing/under-scoped auth. Cost: a logged-in
    user on a _locked_ Linux keyring may see one unlock prompt on a non-auth
    404, rare, once per session, preferred over breaking the zero-config
    GitLab flow. (Future refinement, gated on `keyring` support for
    non-forcing reads: force-unlock only on `401`/`403`.)
  - In steady state keychain reads are silent (Windows no prompt, macOS
    one-time "always allow", unlocked Linux no re-prompt).
- **Keyring error taxonomy:** _absent_ backend falls back to env;
  _present-but-locked/denied_ surfaces the error and suggests unlocking,
  never silent degrade.
- **No-keyring host:** `auth login` refuses to persist and prints the exact
  `SYSAND_CRED_*` lines to set instead.
- **Precedence:** `SYSAND_CRED_*` > keyring > unauthenticated (source
  precedence, §8), so CI can override an interactive login. **Env-shadow
  warning (opportunistic):** if the keyring blob is already loaded this run
  (some request needed it) and an env var overrode a keyring entry that would
  have matched, warn so a stale env var does not silently authenticate with
  an old token. It is opportunistic on purpose, forcing a keyring read on the
  env-win path just to check for a shadow would defeat "never read the
  keyring when env already works".
- **Expiry:** reactive first, on a 401 against a stored credential, print
  "credential for `<index>` may be expired or revoked; re-run
  `sysand auth login <index>`". Proactive when known, `expires_at` (stored
  from `v1/whoami` at login, absent for static/read-only or unvalidated
  logins) lets `auth status` show "expires in N days / expired".
- **`auth status` output:** per entry, the index, covered globs, `subject`
  (from whoami, if a validating login ran), `expires_at` if stored, and
  whether a `SYSAND_CRED_*` var shadows it, never the secret. No `scheme`
  column in v1 (always bearer). With no keyring it lists only the active
  `SYSAND_CRED_*` vars.
- **Re-login:** `auth login` over an existing entry for the same key
  overwrites it, printing "replacing existing credential for `<index>`"
  before the write; the previous stored token is discarded locally (not
  revoked server-side).

## 10. Scope boundaries

**Deferred to later phases (intended, not v1):**

- `auth set` / `auth unset` (raw-pattern credentials) and the `--pattern`
  override on `login`.
- Basic auth via `--username` (request-time basic via `SYSAND_CRED_*` still
  works).
- Longest-prefix most-specific-glob-wins (needed only once `set` / nested
  logins create within-source overlaps).
- **P2 enforcement** (dropping the plain-URL `api_root` default), a
  separate protocol change on its own timeline (§12); this plan only
  consumes `api_root` when advertised.

**Out of scope entirely:** acquisition beyond store-what-you-paste (OAuth
apps, device flows, refresh-token lifecycle); a self-written encrypted vault
or plaintext credentials file; a user-facing credential "label" concept;
multi-account-per-host switching; git credentials (git keeps its own).

## 11. Build phases

Each phase is independently shippable.

1. **Credential store.** Single-keyring-blob store with cross-process file
   locking + env fallback, keyring error taxonomy, index-URL normalization,
   source-precedence lookup, Windows size-limit handling. Introduce the
   **deferred/cached auth policy** that reads the blob on demand and caches
   it, replacing the eager `SYSAND_CRED_*`-only build in
   `sysand/src/lib.rs`. Delivers persistence on its own.
2. **`v1/whoami`** (server side, `sysand-index` repo): identity + token
   metadata, acceptance via HTTP status, routed at `api/v1/whoami`.
3. **`auth login` / `logout` / `status`.** Bearer-only; default-index
   resolution; non-interactive fail-fast; discovery fetch; glob derivation
   (discovery/index root + divergent `api_root`); `--validation true|false`
   with forced-auth probes and the refusal rule; `expires_at` persistence;
   env-shadow warning. The tailored discovery-drift message (§8) is
   **best-effort** here, not required for the phase to ship.
4. **Docs and specs** (§12, §13). Protocol specs in this repo; user docs in
   the `sysand-index` repo (docs.sysand.com).

Separate, not part of these phases: **P2 enforcement** (§10, §12) and the
deferred `auth set` / basic-auth work.

## 12. Protocol/spec changes (this repo, `design/`)

- `design/index-api-protocol.md`: specify `v1/whoami` (§6), routed under
  `api/`.
- `design/index-protocol.md` (**decoupled change**, not this plan's
  phases): enforce "an index has an API iff discovery advertises `api_root`"
  and drop the plain-URL default. This is a **breaking change**, a
  third-party plain-URL _dynamic_ index that relies on the defaulted
  `api_root` (no discovery document, or one without the field) becomes
  read-only and must serve `sysand-index-config.json` with an explicit
  `api_root` (the official index already does). Both the field-absent branch
  and the 404 `flat()` path in `core/src/env/discovery.rs` must change.

## 13. Documentation (docs.sysand.com, in the `sysand-index` repo)

Cross-repo: the published docs live under `docs/source/` in `sysand-index`,
not here. Follow that repo's `docs/README.md` (sentence case, no em-dash,
trailing-slash links). Pages to touch:

- **Reference, rewrite** `docs/source/client/reference/authentication.md`:
  the `sysand auth` model, single-keyring storage, `--validation`,
  precedence (`SYSAND_CRED_*` > keyring), read/API surfaces, the trust
  model. Keep the `SYSAND_CRED_*` reference, it remains the CI / no-keyring
  path (and the only basic-auth path in v1).
- **Reference, new** `docs/source/client/reference/commands/auth/`
  subdirectory (mirroring `commands/index/` and `commands/env/`): an
  `auth-command.md` parent plus `login.md`, `logout.md`, `status.md`; add
  them to the command toctree.
- **How-to, rewrite** `docs/source/client/how-to/authenticate-to-an-index.md`:
  lead with `sysand auth login`; demote the env-var steps to a CI / fallback
  section.
- **Explanation, update**
  `docs/source/client/explanation/authentication.md`: keyring persistence,
  read vs API surfaces, validation, the publish two-leg flow, the
  discovery-drift boundary and trust model (§8).
- **Index side, light**: cross-link `v1/whoami` from the index API
  reference if user-facing; the token pages
  (`docs/source/index/reference/api-tokens.md`,
  `how-to/create-an-api-token.md`) may gain a "use with `sysand auth login`"
  pointer.
- **CLI help**: `about`/`long_about` text for the `sysand auth` command and
  subcommands (in this repo), which the reference pages mirror.
