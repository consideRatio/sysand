// SPDX-FileCopyrightText: © 2026 Sysand contributors <opensource@sensmetry.com>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Facade functions — the public API entry points for sysand.
//!
//! Each function takes public types (options structs, context objects)
//! and returns `Result<T, SysandError>`. Internally they call the
//! command implementations and convert errors at the boundary.

pub mod init;
pub mod source;
pub mod usage;

#[cfg(feature = "filesystem")]
pub mod build;
#[cfg(feature = "filesystem")]
pub mod locate;
