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

`auth login` takes `--validate none|index|api` (default `index`). The
values name the surface probed, not a capability: `index` = the index read
surface (`index_root`), `api` = the API (`api_root`). This is deliberate:
`v1/whoami` only checks that the token is _accepted_ by the API, not that
it can write, so a capability name like `write` would overclaim and would
break if a read-only-api token type is ever introduced. Any future
capability granularity is a separate dimension (for example a hypothetical
`--require publish`), not a rename of these values.

- `--validate index` (default): probe the read surface. On a private index,
  an unauth-4xx followed by an authed-2xx verifies; an authed-4xx refuses.
  On a public read surface there is nothing to check: "stored, not
  verified."
- `--validate api`: probe `api_root/v1/whoami`. A `200` verifies (the token
  is accepted by the API); refuse if it is rejected. Implies index-read
  under C2.
- `--validate none`: store without a credential probe. This is the
  index-aware counterpart to `auth set`: discovery is still fetched
  best-effort for glob scoping (§8) and the entry is per-index, but no
  probe runs. If discovery is unreachable, fall back to the index-root glob
  with a warning. Use it offline, or when a probe would false-refuse.

Because `api_root` is known only after reading discovery, validation is
discovery-first: fetch discovery (with the credential, which also verifies
the read leg on a private index), then, if applicable, probe `v1/whoami`.

**Refusal rule:** refuse to store only on a _proven auth-gated reject_,
either an unauth-4xx-then-authed-4xx escalation, or a rejection from
`v1/whoami` (an endpoint we know is always authenticated). A bare
third-party 403 without that proof downgrades to "stored, not verified".
Never print a bare "verified"; always scope the claim to what the probe
covered.

## 6. The `v1/whoami` endpoint

New endpoint on the index API (server side), under `api_root`. Its purpose
is credential validation and identity for `auth status`.

- `GET api_root/v1/whoami`, bearer credential.
- `200` on a valid, unexpired token; `401` otherwise. The `200` is the
  `--validate api` pass signal.
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
- A keyring entry is **per index**, recording `{token, scheme, [globs]}`,
  so `logout` removes the whole login and `status` shows one login covering
  N patterns. Globs are cached at login time; re-login refreshes them if
  the index later relocates `api_root`.

## 9. Storage, precedence, security

- **Backends:** OS keyring by default (macOS Keychain, Windows Credential
  Manager, Linux Secret Service via the `keyring` crate), with environment
  variables as the automatic fallback where no keyring exists. **No
  plaintext credentials file, ever.**
- **Keyring error taxonomy:** _absent_ backend (no Secret Service on a
  headless Linux box) falls back to env; _present-but-locked/denied_
  surfaces the error and suggests unlocking, rather than silently
  degrading.
- **No-keyring host:** `auth login` / `auth set` refuse to persist and
  print the exact `SYSAND_CRED_*` lines to set instead.
- **Precedence** per URL: `SYSAND_CRED_*` > keyring > unauthenticated
  (so CI can override an interactive login), applied at map-build time.
- **Expiry UX:** on a 401 against a stored credential, suggest re-running
  `sysand auth login`. `v1/whoami`'s `expires_at` also enables a proactive
  "expires in N days" note.
- `auth status` never prints secrets.

## 10. Out of scope

Acquisition beyond store-what-you-paste (OAuth apps, device flows,
refresh-token lifecycle); a self-written encrypted vault or plaintext
credentials file; a third auth scheme; a user-facing credential "label"
concept; multi-account-per-host switching; git credentials (git keeps its
own credential system).

## 11. Build phases

Each phase is independently shippable.

1. **Credential store.** Keyring + env fallback, keyring error taxonomy,
   index-URL normalization, most-specific-wins lookup. Wire it into the
   startup auth builder, replacing the `SYSAND_CRED_*`-only scan in
   `sysand/src/lib.rs`. Delivers persistence on its own.
2. **`auth set` / `auth unset` / `auth status`.** Thin CLI over the store,
   no validation.
3. **`v1/whoami`** (index server side): identity + token metadata,
   acceptance via HTTP status.
4. **`auth login` / `auth logout`.** Discovery fetch, glob derivation
   including divergent-`api_root` scoping, `--validate none|index|api`,
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
