// SPDX-FileCopyrightText: © 2026 Sysand contributors <opensource@sensmetry.com>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Lock facade functions.
//!
//! `lock::update` requires a resolver chain, HTTP client, and tokio
//! runtime. The facade for this command is deferred until the
//! infrastructure abstraction is designed. CLI and bindings continue
//! calling `commands::lock::do_lock_*` directly.

// TODO: pub fn update(ctx, opts) -> Result<(), SysandError>
// Needs: resolver chain assembly, auth, HTTP client, tokio runtime.
// Open question: should facade construct these internally from
// ProjectContext + config, or should caller provide them?
