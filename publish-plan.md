# Implementation Plan: `sysand publish` Command

## Context

This plan adds a new `sysand publish` command to publish `.kpar` files (built with `sysand build`) to the sysand package index at beta.sysand.org or other compatible registries.

**Why this change is needed:**
- Users need a way to share their SysML v2/KerML packages with others
- The sysand-index registry exists but requires manual upload via web UI
- Other package managers (npm, cargo, pip) all have a `publish` command
- Enables CI/CD automation for package publishing

**Scope:**
- ✅ Individual project publishing only
- ✅ Integration tests and documentation following existing patterns
- ❌ Workspace publishing (out of scope)
- ❌ Language bindings (Python/Java/JS out of scope)
- 🎯 **Minimalistic implementation for easy review**

## Architecture

Following the established two-layer pattern:
1. **Core logic** (`core/src/commands/publish.rs`) - business logic
2. **CLI wrapper** (`sysand/src/commands/publish.rs`) - CLI integration
3. **Tests** (`sysand/tests/cli_publish.rs`) - integration tests
4. **Docs** (`docs/src/commands/publish.md`) - user documentation

## Target API: sysand-index

The sysand-index at `/home/erik/dev/sensmetry/sysand-index` provides:
- **Endpoint:** `POST /api/v1/upload`
- **Content-Type:** `multipart/form-data`
- **Required fields:**
  - `purl`: Package URL (e.g., `pkg:sysand/myproject@1.0.0`)
  - `file`: The `.kpar` file
- **Authentication:** Bearer token via `Authorization: Bearer <token>` header
- **Responses:**
  - `201 Created`: New project
  - `200 OK`: New release of existing project
  - `400/401/403/404/409`: Various errors

## Implementation Details

### 1. Core Error Types

**File:** `core/src/commands/publish.rs` (new file)

```rust
#[derive(Error, Debug)]
pub enum PublishError {
    #[error("failed to read kpar file at `{0}`: {1}")]
    KparRead(Box<str>, std::io::Error),

    #[error("failed to open kpar project at `{0}`: {1}")]
    KparOpen(Box<str>, String),

    #[error("missing project info in kpar")]
    MissingInfo,

    #[error("missing project metadata in kpar")]
    MissingMeta,

    #[error("failed to generate PURL: {0}")]
    PurlGeneration(String),

    #[error("HTTP request failed: {0}")]
    Http(#[from] reqwest_middleware::Error),

    #[error("server error ({0}): {1}")]
    ServerError(u16, String),

    #[error("authentication failed: {0}")]
    AuthError(String),

    #[error("conflict: package version already exists: {0}")]
    Conflict(String),

    #[error("bad request: {0}")]
    BadRequest(String),
}
```

### 2. Core Logic Function

**File:** `core/src/commands/publish.rs`

**Signature:**
```rust
#[cfg(feature = "filesystem")]
pub fn do_publish_kpar<P: AsRef<Utf8Path>, Policy: HTTPAuthentication>(
    kpar_path: P,
    index_url: &str,
    auth_policy: Arc<Policy>,
    client: reqwest_middleware::ClientWithMiddleware,
    runtime: Arc<tokio::runtime::Runtime>,
) -> Result<PublishResponse, PublishError>
```

**Response Type:**
```rust
#[derive(Debug)]
pub struct PublishResponse {
    pub status: u16,
    pub message: String,
    pub is_new_project: bool,
}
```

**Implementation steps:**
1. Open and validate kpar using `LocalKParProject::new_guess_root(kpar_path)`
2. Extract project info and metadata via `get_project()`
3. Generate PURL: `pkg:sysand/{name}@{version}` (no org support initially)
4. Create multipart form with `reqwest::multipart::Form`
5. Make authenticated POST request to `{index_url}/api/v1/upload`
6. Handle response codes: 200/201 success, 400/401/403/404/409 errors
7. Return `PublishResponse` with status and message

**Pattern:** Follows `core/src/commands/build.rs` structure exactly

### 3. CLI Wrapper

**File:** `sysand/src/commands/publish.rs` (new file)

**Signature:**
```rust
pub fn command_publish<P: AsRef<Utf8Path>, Policy: HTTPAuthentication>(
    kpar_path: P,
    index_url: &str,
    auth_policy: Arc<Policy>,
    client: reqwest_middleware::ClientWithMiddleware,
    runtime: Arc<tokio::runtime::Runtime>,
) -> Result<()>
```

**Implementation:**
- Calls `do_publish_kpar()` from core
- Logs results using `log::info!()` with styled output
- Pattern: Follows `sysand/src/commands/build.rs` exactly (lines 12-19)

### 4. CLI Arguments

**File:** `sysand/src/cli.rs` (add after Build variant, around line 161)

```rust
/// Publish a KPAR to the sysand package index
Publish {
    /// Path to the KPAR file to publish. If not provided, will look
    /// for a KPAR in the output directory with the current project's
    /// name and version
    #[clap(verbatim_doc_comment)]
    path: Option<Utf8PathBuf>,

    /// URL of the package index to publish to. Defaults to the
    /// first index URL from configuration or https://beta.sysand.org
    #[arg(long, verbatim_doc_comment)]
    index: Option<String>,
},
```

### 5. CLI Integration

**File:** `sysand/src/lib.rs` (add match arm around line 600, after Build)

**Logic:**
1. Require current project (no workspace support)
2. Determine kpar path:
   - Use explicit `--path` if provided
   - Otherwise: `output/{name}-{version}.kpar` (using `default_kpar_file_name()`)
   - Error if file doesn't exist: "Run `sysand build` first"
3. Determine index URL:
   - Use `--index` if provided
   - Otherwise: first URL from config or default
4. Call `command_publish()` with auth policy

**Pattern:** Similar to Build command (lib.rs:569-601) but simpler (no workspace logic)

### 6. Module Registration

**Files to update:**
- `core/src/commands/mod.rs`: Add `#[cfg(feature = "filesystem")] pub mod publish;`
- `sysand/src/commands/mod.rs`: Add `pub mod publish;`

### 7. Integration Tests

**File:** `sysand/tests/cli_publish.rs` (new file)

**Tests to implement:**
1. `test_publish_missing_kpar()` - Error when kpar doesn't exist
2. `test_publish_requires_build()` - Must build before publishing
3. `test_publish_network_error()` - Handle connection failures

**Pattern:** Follow `sysand/tests/cli_build.rs` structure exactly
- Use `run_sysand()` and `run_sysand_in()` from `tests/common/mod.rs`
- Use `assert_cmd` predicates for output validation
- Create temp directories with test projects

**Note:** Full mock server tests deferred to future work (keep minimal for review)

### 8. Documentation

**File:** `docs/src/commands/publish.md` (new file)

**Sections:**
1. Usage syntax
2. Description (what it does, authentication required)
3. Arguments and options
4. Examples (basic, explicit path, custom index)
5. Response codes explanation
6. See also (link to build command and authentication docs)

**Update:** `docs/src/SUMMARY.md` - Add entry after `build.md`

## Authentication

**Existing infrastructure** (no changes needed):
- Environment variables: `SYSAND_CRED_INDEX_BEARER_TOKEN`
- Pattern: `SYSAND_CRED_{NAME}=pattern` and `SYSAND_CRED_{NAME}_BEARER_TOKEN=token`
- Implementation: Already in `sysand/src/lib.rs` lines 176-246
- Passed as `Arc<StandardHTTPAuthentication>` to commands

**Example setup:**
```bash
export SYSAND_CRED_INDEX="https://beta.sysand.org/*"
export SYSAND_CRED_INDEX_BEARER_TOKEN="sysand_u_xxxxxxxxxxxxx"
```

## Critical Files

### To Create:
1. `core/src/commands/publish.rs` - Core implementation (~250 lines)
2. `sysand/src/commands/publish.rs` - CLI wrapper (~25 lines)
3. `sysand/tests/cli_publish.rs` - Integration tests (~80 lines)
4. `docs/src/commands/publish.md` - User documentation

### To Modify:
1. `sysand/Cargo.toml` - Add `"multipart"` feature to reqwest (2 lines at lines 46 & 48)
2. `core/src/commands/mod.rs` - Add module declaration (1 line)
3. `sysand/src/commands/mod.rs` - Add module declaration (1 line)
4. `sysand/src/cli.rs` - Add Publish variant (~12 lines at line 161)
5. `sysand/src/lib.rs` - Add match arm handler (~30 lines at line 600)
6. `docs/src/SUMMARY.md` - Add publish command link (1 line)

## Implementation Order

1. ✅ **Add multipart feature** - Update `sysand/Cargo.toml` to add `"multipart"` to reqwest features
2. ✅ **Core error types** - Define `PublishError` enum
3. ✅ **Core logic** - Implement `do_publish_kpar()` function (platform-agnostic)
4. ✅ **Module registration (core)** - Export in `core/src/commands/mod.rs`
5. ✅ **CLI wrapper** - Create `sysand/src/commands/publish.rs`
6. ✅ **Module registration (CLI)** - Export in `sysand/src/commands/mod.rs`
7. ✅ **CLI arguments** - Add variant to `sysand/src/cli.rs`
8. ✅ **Main handler** - Add match arm in `sysand/src/lib.rs`
9. ✅ **Tests** - Create `sysand/tests/cli_publish.rs`
10. ✅ **Documentation** - Create `docs/src/commands/publish.md` and update SUMMARY

## Key Reusable Functions

From exploration, we can reuse:
- `sysand_core::build::default_kpar_file_name()` - Generate default kpar filename
- `sysand_core::project::local_kpar::LocalKParProject::new_guess_root()` - Open kpar
- `auth_policy.with_authentication(&client, &request_builder).await` - Make authenticated request
- `log::info!()` with `get_style_config()` - Styled output
- `run_sysand()` and `run_sysand_in()` - Test utilities

## Verification Plan

### Manual Testing:
1. Build a test project: `sysand init && sysand build`
2. Set up auth: `export SYSAND_CRED_INDEX_BEARER_TOKEN="..."`
3. Publish: `sysand publish`
4. Verify in web UI at beta.sysand.org
5. Try republishing same version (should get 409 Conflict)
6. Try publishing without auth (should get 401)

### Automated Testing:
1. Run `cargo test -p sysand cli_publish` - Should pass all tests
2. Run `cargo test` - Ensure no regressions
3. Run `cargo clippy` - No warnings
4. Run `cargo fmt --check` - Formatting correct

### Documentation Testing:
1. Build docs: `cd docs && mdbook build`
2. Verify publish.md renders correctly
3. Check all links work

## Design Decisions

### 1. PURL Format
**Decision:** Simple `pkg:sysand/{name}@{version}` (no org support)
**Rationale:** Minimalistic, easy to review, org support can be added later

### 2. Require Explicit Build
**Decision:** Do NOT auto-build, require `sysand build` first
**Rationale:**
- Separation of concerns (build vs publish)
- Clear error messages
- Follows Docker pattern (build, then push)

### 3. Default KPAR Path
**Decision:** Look in `output/{name}-{version}.kpar` if not specified
**Rationale:** Matches default build output location

### 4. Workspace Support
**Decision:** Not implemented (out of scope per user request)
**Rationale:** Keep implementation minimal and reviewable

### 5. Readme Support
**Decision:** Not implemented initially
**Rationale:** sysand-index supports it but it's optional, can add later

## Cross-Platform Compatibility

**CRITICAL: This implementation must work on all supported OS and architectures.**

The codebase already supports multiple platforms with platform-specific TLS configurations:
- **Windows/macOS**: Uses `native-tls` (system native TLS)
- **Linux/other**: Uses `rustls` (pure Rust TLS)

### Required Change for Multipart Support

**File:** `sysand/Cargo.toml` (lines 44-48)

**Current:**
```toml
# Use native TLS only on Windows and Apple OSs
[target.'cfg(any(target_os = "windows", target_vendor = "apple"))'.dependencies]
reqwest = { version = "0.13.1", default-features = false, features = ["native-tls", "http2", "system-proxy", "blocking"] }
[target.'cfg(not(any(target_os = "windows", target_vendor = "apple")))'.dependencies]
reqwest = { version = "0.13.1", features = ["rustls", "blocking"] }
```

**Updated (add `"multipart"` to both):**
```toml
# Use native TLS only on Windows and Apple OSs
[target.'cfg(any(target_os = "windows", target_vendor = "apple"))'.dependencies]
reqwest = { version = "0.13.1", default-features = false, features = ["native-tls", "http2", "system-proxy", "blocking", "multipart"] }
[target.'cfg(not(any(target_os = "windows", target_vendor = "apple")))'.dependencies]
reqwest = { version = "0.13.1", features = ["rustls", "blocking", "multipart"] }
```

**Why this works cross-platform:**
- The `multipart` feature works identically with both `native-tls` and `rustls`
- No platform-specific code needed in the publish implementation
- Existing authentication infrastructure already handles platform differences
- reqwest's multipart API is platform-agnostic

### Testing Across Platforms

The implementation should be tested on:
- ✅ Linux (x86_64, aarch64)
- ✅ macOS (Intel, Apple Silicon)
- ✅ Windows (x86_64)

All existing CI/CD workflows will automatically test the new command on these platforms.

## Dependencies

**One small dependency feature addition required:**

Add `multipart` feature to `reqwest` in `sysand/Cargo.toml` (see Cross-Platform Compatibility section above).

**All other needed crates already present:**
- `reqwest` with `multipart` (for file upload)
- `reqwest_middleware` (auth integration)
- `tokio` (async runtime)
- `thiserror` (error types)
- `camino` (UTF-8 paths)
- `serde/serde_json` (JSON handling)

## Success Criteria

✅ Users can publish `.kpar` files with `sysand publish`
✅ **Works on ALL supported platforms (Linux, macOS, Windows) and architectures (x86_64, aarch64)**
✅ Authentication works via environment variables
✅ Clear error messages for common failure cases
✅ Integration tests pass on all platforms
✅ Documentation complete and accurate
✅ Code follows existing patterns exactly
✅ Implementation is minimal and easy to review
✅ No platform-specific code in publish implementation (TLS differences handled by existing reqwest config)
