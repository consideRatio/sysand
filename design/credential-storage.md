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

## 2. Current state

- **Request-time application of credentials already exists** in
  `core/src/auth.rs`: a per-URL glob map of auth policies (bearer, basic),
  combinators that try unauthenticated first and send the secret only on a
  4xx, and same-host redirect handling.
- **Publish OIDC (trusted publishing)** exists in
  `core/src/commands/publish.rs` for CI.
- **Token sourcing is a stopgap:** `sysand/src/lib.rs` scans
  `SYSAND_CRED_*` environment variables on every run. There is no
  persistence: a user re-supplies env vars each invocation.

The gap this plan fills is the missing middle: persist a credential once,
retrieve it on later runs, feed it into the existing glob-based auth layer.

## 3. Model and constraints

Three surfaces, and one access rule:

| Surface    | Probe path                 | Creds required? |
| ---------- | -------------------------- | --------------- |
| Discovery  | `sysand-index-config.json` | sometimes       |
| Index root | `index.json`               | sometimes       |
| API root   | `v1/whoami`, `v1/upload`   | always          |

Locked constraints:

- **C1 - unified read access.** Discovery and index root share one
  auth status (public or private together). This removes pathological
  split permutations and collapses the space to a 2x2.
- **C2 - one credential per index.** `auth login` stores one credential
  per index, used for both the read leg and the API leg. Separate
  read/write tokens are not a login concept; `auth set` covers them.
- **P2 - API requires an advertised `api_root`.** An index has an API
  iff its discovery document advertises `api_root`. The plain-URL default
  (assume `api_root = root`) is dropped; plain and templated URLs are
  treated identically. "Has an API" becomes a pure function of the
  discovery document; no runtime probe is needed to classify.
- **P1 - public discovery: deferred.** Whether the discovery document
  must be publicly readable is left open. Under C2 it does not matter: the
  single credential reads discovery on a private index, so there is no
  bootstrap paradox. Current behavior (discovery may be private) stands.

The collapsed situation space:

| #   | Read surface | API     | Example                     | Creds used for  |
| --- | ------------ | ------- | --------------------------- | --------------- |
| S1  | public       | none    | public static index         | nothing         |
| S2  | private      | none    | private static index        | read            |
| S3  | public       | present | official sysand.com         | write (publish) |
| S4  | private      | present | fully private dynamic index | read + write    |

## 4. Command surface

All under a `sysand auth` namespace. Symmetric create/remove pairs plus a
status view.

| Command                          | Role                                                                                            |
| -------------------------------- | ----------------------------------------------------------------------------------------------- |
| `sysand auth login <index-url>`  | validated, index-keyed credential (see §5)                                                      |
| `sysand auth logout <index-url>` | remove an index login                                                                           |
| `sysand auth set <pattern>`      | raw glob credential, no validation                                                              |
| `sysand auth unset <pattern>`    | remove a raw credential                                                                         |
| `sysand auth status`             | list stored credentials (never secrets), backend in use, and any `SYSAND_CRED_*` shadowing them |

Scheme selection: no `--username` means bearer token; `--username <u>`
means basic auth (prompts for password). Secrets are accepted only via a
hidden prompt or `--token-stdin` / `--password-stdin`, never an inline
value flag (shell-history / `ps` leakage).

`login` derives the glob set from the index URL: `<index-root>/**`, plus
`<api-root>/**` when the two diverge (§8), auto-anchored before
`{path}` / `{path_raw}` for templated URLs. `--pattern` overrides the
derived glob. The index URL is normalized (trailing slash, scheme) before
use as the storage key and for glob derivation, so different spellings do
not create duplicate entries.

## 5. Validation

`auth login` takes `--validation true|false` (default `true`).

- `--validation true` (default): probe every surface the index supports and
  store unless the credential is rejected everywhere it was actually
  tested (see the refusal rule below). A static index has only the read
  surface; a dynamic index adds the API.
- `--validation false`: store without any credential probe. The index-aware
  counterpart to `auth set`: discovery is still fetched best-effort for
  glob scoping (§8) and the entry is per-index, but no probe runs. If
  discovery is unreachable, fall back to the index-root glob with a
  warning. Use it offline, or when a probe would false-refuse.

Validation is a boolean, not a set of per-surface levels. Since `v1/whoami`
checks only that a token is _accepted_ by the API (identity, not
capability, see §6), validating everything almost never wrongly refuses a
valid token, so an intermediate "read-only" level would add a choice
without real payoff. If a genuine need appears, `--validation` could later
give way to a levelled flag without disrupting this default.

Because `api_root` is known only after reading discovery, validation is
discovery-first: fetch discovery (this also exercises the credential
against the discovery root on a private index), resolve `index_root` and
`api_root`, then probe `index_root/index.json` and, if the index has an
API, `api_root/v1/whoami`.

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

New endpoint on the index API (server side), under `api_root`. Its purpose
is credential validation and identity for `auth status`.

- `GET api_root/v1/whoami`, bearer credential.
- `200` on a valid, unexpired token; `401` otherwise. Under
  `--validation true` a `200` passes the API leg (see §5).
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

`subject.type` is `user` or `project` (or `oidc`), so the endpoint is
principal-agnostic (hence `whoami`, not `user`). The body carries identity
and token metadata for `auth status` and the expiry warning; it does not
carry a `can_publish` flag or a publish-scope list. Rationale: at login
there is no target project, and every valid token can publish somewhere, so
a capability boolean is vacuous; per-project authorization stays enforced
at the upload (its existing `403`). An optional per-project pre-flight
(`?project=<id>`) may be added later but is not part of this contract.

## 7. Publish interaction

Publish is two legs: read discovery to resolve `api_root` (authenticated
only if the index is private), then bearer POST to `api_root/v1/upload`.
Its logic is unchanged. Because `login` scopes the one credential to cover
both roots (§8), publish's existing "match a bearer glob against the upload
URL" resolution finds the credential.

- Basic-auth logins cannot publish (upload is bearer-only). When `login`
  detects a publishable index (`api_root` advertised) and the user chose
  `--username`, it warns.
- When the bearer map is built from env + keyring together, precedence
  (§9) is applied at build time so the two sources cannot both match the
  upload URL and trip publish's match rule.

## 8. Glob scoping and conflict resolution

- **Most-specific-glob-wins for the publish leg.** When more than one
  bearer glob matches the upload URL, the longest-literal-prefix glob
  wins; env source breaks ties over keyring. This resolves env+keyring
  overlap, `auth set` overlap, and nested-index overlap without the old
  "exactly one match or error" fragility. Reads keep the existing
  try-all-matches behavior.
- **Divergent `api_root` (Case B).** `login` has already fetched discovery
  and knows both roots. If `api_root` nests under `index_root` (Case A),
  store one glob. If it is disjoint, store the same credential under both
  `index_root/**` and `api_root/**` (minimal, non-overlapping), so the
  upload URL matches exactly the api glob. Templated indexes are inherently
  Case B (their `api_root` is a disjoint plain URL).
- Each login is one record (`{key, globs, scheme, username-if-basic,
secret, expires_at-if-known}`) inside the single keyring blob (§9), so
  `logout` removes it and `status` shows one login covering N patterns.
  Globs are cached at login time; re-login refreshes them if the index
  later relocates `api_root`.

## 9. Storage, consumption, precedence, security

- **Backends:** OS keyring by default (macOS Keychain, Windows Credential
  Manager, Linux Secret Service via the `keyring` crate), with environment
  variables as the automatic fallback where no keyring exists. **No
  plaintext credentials file, ever.**
- **Single keyring entry.** All persisted credentials live in **one**
  keyring entry (for example `service = "sysand"`, `account =
"credentials"`) holding a JSON blob: a list of records
  `{key (URL or pattern), globs, scheme, username-if-basic, secret,
  expires_at-if-known}`. This
  is deliberate over a separate manifest file: the `keyring` crate cannot
  portably enumerate entries, and one blob is **atomic** (metadata and
  secret cannot drift), needs **no file**, and prompts the OS keychain at
  most once. `login` / `logout` / `set` / `unset` read-modify-write the
  blob under a process lock; `status` reads it. The only cost is scale:
  Windows Credential Manager caps a blob at ~2.5 KB
  (`CRED_MAX_CREDENTIAL_BLOB_SIZE` = 2560), roughly ten entries, so a write
  that would exceed it fails with a clear "credential store full on this
  platform, remove an unused login" error rather than silently truncating.
- **Consumption and keyring access.** The blob is read only when a
  credential might actually be needed, to avoid unnecessary OS keychain
  prompts:
  - **Never read** for local/offline commands, for reads that succeed
    unauthenticated (the existing unauth-first policy: public indexes like
    sysand.com return 200 and never touch the keyring), or for users who
    never ran `auth login` (no entry exists, so the lookup is a cheap "not
    found" with no unlock).
  - **Read once, then cache** for the process on the first auth-relevant
    4xx during an unauth-first read, on publish's authenticated leg, and on
    the explicit `auth *` commands. In-process caching means at most one
    keychain touch per command regardless of request count.
  - Reads escalate on **any** 4xx (not just 401/403), because some hosts
    (GitLab) answer `404` on missing/under-scoped auth. The cost is that a
    logged-in user on a _locked_ Linux keyring may see one unlock prompt on
    a non-auth 404; this is rare, once per session, and preferred over
    breaking the zero-config GitLab flow or reintroducing a separate glob
    file. (Possible future refinement, gated on `keyring` support for
    non-forcing reads: force an unlock only on `401`/`403`, and on a bare
    `404` use the keyring only if it is already unlocked.)
  - In steady state keychain reads are silent: Windows has no per-access
    prompt, macOS grants the signed app a one-time "always allow", and an
    unlocked Linux keyring does not re-prompt.
- **Keyring error taxonomy:** _absent_ backend (no Secret Service on a
  headless Linux box) falls back to env; _present-but-locked/denied_
  surfaces the error and suggests unlocking, rather than silently
  degrading.
- **No-keyring host:** `auth login` / `auth set` refuse to persist and
  print the exact `SYSAND_CRED_*` lines to set instead.
- **Precedence and ordering** per URL: `SYSAND_CRED_*` > keyring >
  unauthenticated (so CI can override an interactive login). For publish
  this is enforced via most-specific-glob-wins (§8). For reads (try-all),
  credentials are tried most-specific glob first, env before keyring, so a
  narrowly-scoped login is not shadowed by a broad `auth set` pattern.
- **Expiry:** reactive first, on a 401 against a stored credential, print
  "credential for `<index>` may be expired or revoked; re-run
  `sysand auth login <index>`". Proactive when known, `expires_at` (stored
  from `v1/whoami` at login, absent for static/read-only or unvalidated
  logins) lets `auth status` show "expires in N days / expired" and warn
  before use.
- **`auth status` output:** per entry, the index/pattern, covered globs,
  scheme, subject/username, `expires_at` if stored, and whether a
  `SYSAND_CRED_*` var currently shadows it, never the secret. With no
  keyring it lists only the active `SYSAND_CRED_*` vars.
- **Re-login:** `auth login` over an existing entry for the same key
  overwrites it and prints "replacing existing credential for `<index>`".

## 10. Out of scope

Acquisition beyond store-what-you-paste (OAuth apps, device flows,
refresh-token lifecycle); a self-written encrypted vault or plaintext
credentials file; a third auth scheme; a user-facing credential "label"
concept; multi-account-per-host switching; git credentials (git keeps its
own credential system).

## 11. Build phases

Each phase is independently shippable.

1. **Credential store.** Single-keyring-blob store (read-modify-write under
   a process lock) + env fallback, keyring error taxonomy, index-URL
   normalization, most-specific-wins lookup. Wire it into the startup auth
   builder, replacing the `SYSAND_CRED_*`-only scan in `sysand/src/lib.rs`.
   Delivers persistence on its own.
2. **`auth set` / `auth unset` / `auth status`.** Thin CLI over the store,
   no validation.
3. **`v1/whoami`** (index server side): identity + token metadata,
   acceptance via HTTP status.
4. **`auth login` / `auth logout`.** Discovery fetch, glob derivation
   including divergent-`api_root` scoping, `--validation true|false`,
   the refusal rule, and capability-scoped output.
5. **Enforce P2** (client): drop the plain-URL `api_root` default in
   `core/src/env/discovery.rs`; update `design/index-protocol.md`.
   **Breaking change:** a third-party plain-URL _dynamic_ index that today
   relies on the defaulted `api_root` (no discovery document, or a document
   without the field) becomes read-only and must serve
   `sysand-index-config.json` with an explicit `api_root`. The official
   index is unaffected (it already serves it).
6. **Docs.** Update the client authentication how-to and reference to cover
   `sysand auth`, the keyring/env precedence, and when to use each.

## 12. Protocol/spec changes required

- `design/index-api-protocol.md`: specify `v1/whoami` (§6).
- `design/index-protocol.md`: enforce P2 (an index has an API iff
  discovery advertises `api_root`; remove the plain-URL default).
