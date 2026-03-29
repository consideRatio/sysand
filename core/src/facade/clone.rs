// SPDX-FileCopyrightText: © 2026 Sysand contributors <opensource@sensmetry.com>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Clone facade function.
//!
//! `clone` requires resolver chain, HTTP client, and tokio runtime
//! (same infrastructure as lock::update). Deferred until the
//! infrastructure abstraction is designed.

// TODO: pub fn clone(locator, opts) -> Result<(), SysandError>
