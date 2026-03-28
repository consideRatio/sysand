# Data Model

The files sysand reads and writes, their schemas, and how they relate.

## Overview

| File               | Location       | Format | Purpose                               | Mutable by                                      |
| ------------------ | -------------- | ------ | ------------------------------------- | ----------------------------------------------- |
| `.project.json`    | Project root   | JSON   | Project identity and usages           | `project info/metadata`, `usage add/remove`     |
| `.meta.json`       | Project root   | JSON   | Project metadata (symbols, checksums) | `project source add/remove`, `project metadata` |
| `sysand.toml`      | Project root   | TOML   | Config: indexes, source overrides     | `usage add --source-kind`, manual editing       |
| `sysand-lock.toml` | Project root   | TOML   | Resolved dependency graph             | `lock update`                                   |
| `.workspace.json`  | Workspace root | JSON   | Lists member projects                 | Manual editing                                  |

## `.project.json`

The project's identity, dependencies, and descriptive information.
Defined by KerML clause 10.3.

```json
{
  "name": "my-sensors",
  "publisher": "Example Corp",
  "description": "Sensor models for spacecraft systems",
  "version": "2.1.0",
  "license": "MIT",
  "maintainer": ["alice@example.com", "bob@example.com"],
  "website": "https://example.com/sensors",
  "topic": ["sensors", "spacecraft"],
  "usage": [
    {
      "resource": "urn:example:comm-protocols",
      "versionConstraint": "^1.0"
    },
    {
      "resource": "urn:example:components"
    }
  ]
}
```

### Fields

| Field         | Type         | Required | Notes                            |
| ------------- | ------------ | -------- | -------------------------------- |
| `name`        | string       | Yes      | Human-readable label, not unique |
| `publisher`   | string       | No       |                                  |
| `description` | string       | No       |                                  |
| `version`     | string       | Yes      | Must be valid semver             |
| `license`     | string       | No       | SPDX expression                  |
| `maintainer`  | string[]     | No       | Defaults to `[]`                 |
| `website`     | string (IRI) | No       |                                  |
| `topic`       | string[]     | No       | Defaults to `[]`                 |
| `usage`       | UsageEntry[] | Yes      | Defaults to `[]`                 |

### UsageEntry

| Field               | Type         | Required | Notes                               |
| ------------------- | ------------ | -------- | ----------------------------------- |
| `resource`          | string (IRI) | Yes      | Package identifier                  |
| `versionConstraint` | string       | No       | Semver range. Omitted = any version |

JSON uses `camelCase` field names.

## `.meta.json`

Project metadata: symbol index, checksums, and model information.

```json
{
  "index": {
    "Sensors::TemperatureSensor": "src/sensors.sysml",
    "Sensors::PressureSensor": "src/sensors.sysml"
  },
  "created": "2025-03-15T10:30:00Z",
  "metamodel": "https://www.omg.org/spec/SysML/20250201",
  "includesDerived": false,
  "includesImplied": false,
  "checksum": {
    "src/sensors.sysml": {
      "value": "a1b2c3...",
      "algorithm": "SHA-256"
    }
  }
}
```

### Fields

| Field             | Type                | Required | Notes                          |
| ----------------- | ------------------- | -------- | ------------------------------ |
| `index`           | map<string, path>   | Yes      | Symbol name → source file path |
| `created`         | string              | Yes      | Timestamp                      |
| `metamodel`       | string (IRI)        | No       | KerML or SysML spec IRI        |
| `includesDerived` | bool                | No       |                                |
| `includesImplied` | bool                | No       |                                |
| `checksum`        | map<path, Checksum> | No       | Per-file checksums             |

### Checksum

| Field       | Type   | Required |
| ----------- | ------ | -------- |
| `value`     | string | Yes      |
| `algorithm` | string | Yes      |

## `sysand.toml`

Project-level configuration. Controls index URLs and source overrides.
Only the root project's config is consulted during dependency
resolution (see `discovery-and-config.md`).

```toml
[[index]]
name = "company-internal"
url = "https://index.example.com"

[[index]]
name = "public"
url = "https://beta.sysand.org"
default = true

[[project]]
identifiers = ["urn:example:sensors"]
sources = [{ src_path = "../libs/sensors" }]

[[project]]
identifiers = ["urn:example:protocols"]
sources = [{ remote_git = "https://github.com/example/protocols.git" }]
```

### Index entry

| Field     | Type   | Required | Notes                                            |
| --------- | ------ | -------- | ------------------------------------------------ |
| `name`    | string | No       | Human-readable label                             |
| `url`     | string | Yes      | Index URL                                        |
| `default` | bool   | No       | Default indexes are tried last (lowest priority) |

### Project source override

| Field         | Type     | Required | Notes                         |
| ------------- | -------- | -------- | ----------------------------- |
| `identifiers` | string[] | Yes      | IRIs this override applies to |
| `sources`     | Source[] | Yes      | Where to fetch this project   |

### Source variants

| Variant      | Fields                                                  | Description                           |
| ------------ | ------------------------------------------------------- | ------------------------------------- |
| `Editable`   | `editable` (path)                                       | Local project, changes picked up live |
| `LocalSrc`   | `src_path` (path)                                       | Local source directory                |
| `LocalKpar`  | `kpar_path` (path)                                      | Local .kpar archive                   |
| `Registry`   | `registry` (string)                                     | Package index URL                     |
| `RemoteKpar` | `remote_kpar` (URL), `remote_kpar_size` (u64, optional) | Remote .kpar archive                  |
| `RemoteSrc`  | `remote_src` (URL)                                      | Remote source directory               |
| `RemoteGit`  | `remote_git` (URL)                                      | Git repository                        |
| `RemoteApi`  | `remote_api` (URL)                                      | API endpoint                          |

Local paths are relative to the workspace root (Unix-style paths).

## `sysand-lock.toml`

The resolved dependency graph. Produced by `lock update`. Consumed by
`env sync`. Should be committed to version control.

```toml
lock_version = "1"

[[project]]
name = "comm-protocols"
version = "1.2.0"
identifiers = ["urn:example:comm-protocols"]
exports = ["Protocols::TCP", "Protocols::UDP"]
checksum = "abc123..."
sources = [{ registry = "https://beta.sysand.org" }]

[[project]]
name = "components"
version = "3.0.1"
identifiers = ["urn:example:components"]
exports = ["Components::Antenna", "Components::Radio"]
usages = [{ resource = "urn:example:materials" }]
checksum = "def456..."
sources = [{ remote_kpar = "https://example.com/components-3.0.1.kpar" }]
```

### Lock fields

| Field          | Type      | Required | Notes                   |
| -------------- | --------- | -------- | ----------------------- |
| `lock_version` | string    | Yes      | Lockfile format version |
| `project`      | Project[] | Yes      | Resolved projects       |

### Lock project entry

| Field         | Type     | Required | Notes                                  |
| ------------- | -------- | -------- | -------------------------------------- |
| `name`        | string   | No       | From `.project.json`                   |
| `publisher`   | string   | No       | From `.project.json`                   |
| `version`     | string   | Yes      | Exact resolved version                 |
| `identifiers` | string[] | No       | IRIs for this project                  |
| `exports`     | string[] | No       | Symbol names from `.meta.json` index   |
| `usages`      | Usage[]  | No       | What this project depends on           |
| `sources`     | Source[] | No       | How to fetch (same variants as config) |
| `checksum`    | string   | Yes      | Canonical content hash                 |

### Lock usage entry

| Field      | Type         | Required |
| ---------- | ------------ | -------- |
| `resource` | string (IRI) | Yes      |

Note: lock usages only record the IRI, not the version constraint.
The constraint was used during resolution; the lock records the
outcome.

## `.workspace.json`

Lists member projects in a workspace.

```json
{
  "projects": [
    {
      "path": "core",
      "iris": ["urn:example:core"]
    },
    {
      "path": "drivers",
      "iris": ["urn:example:drivers"]
    }
  ]
}
```

### Workspace project entry

| Field  | Type     | Required | Notes                                  |
| ------ | -------- | -------- | -------------------------------------- |
| `path` | string   | Yes      | Relative path to the project directory |
| `iris` | string[] | Yes      | IRIs identifying this project          |

## File Relationships

```
.workspace.json
    lists →  project paths
                 │
                 ▼
          .project.json ─── usages ──→ IRIs (resolved via indexes or config)
          .meta.json    ─── index  ──→ symbol → file mapping
                 │
                 │  lock update (solver reads both files from candidates)
                 ▼
          sysand-lock.toml ── exact versions, sources, exports, checksums
                 │
                 │  env sync (fetches from recorded sources)
                 ▼
            sysand_env/  ── installed projects
```

`sysand.toml` influences resolution by providing source overrides and
index URLs, but is not part of the data flow — it's configuration.
