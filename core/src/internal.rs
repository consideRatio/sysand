// SPDX-FileCopyrightText: © 2026 Sysand contributors <opensource@sensmetry.com>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Internal modules — not part of the public facade API.
//!
//! These modules are re-exported at the crate root for backward
//! compatibility. New code should use `facade::*` and `types::*`
//! instead of reaching into these modules directly.
//!
//! A future step will physically move these files into `internal/`
//! and restrict visibility to `pub(crate)`.

// This module exists only to document the intended boundary.
// The actual modules remain at their current locations (crate root)
// and are still exported as `pub mod` from lib.rs.
//
// When consumers (CLI, bindings) are fully migrated to use the
// facade, these modules will be moved here physically and gated
// with pub(crate).
//
// Modules that belong in internal/:
//   commands/   — command implementations (do_init, do_add, etc.)
//   project/    — ProjectRead, ProjectMut traits + implementations
//   env/        — ReadEnvironment, WriteEnvironment + implementations
//   resolve/    — ResolveRead, resolver chain
//   solve/      — PubGrub solver
//   lock.rs     — lockfile parsing
//   config/     — config loading
//   model.rs    — raw interchange types
//   auth.rs     — HTTP authentication
//   discover.rs — filesystem walk
//   stdlib.rs   — standard library definitions
//   workspace.rs — workspace metadata
//   context.rs  — old ProjectContext (with live objects)
//   symbols/    — symbol extraction
//   style.rs    — terminal styling
