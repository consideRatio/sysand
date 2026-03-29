// SPDX-FileCopyrightText: © 2026 Sysand contributors <opensource@sensmetry.com>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Public vocabulary types for the sysand facade API.
//!
//! These types are used in facade function signatures — context objects,
//! options structs, return types, and enums. They form the public API
//! contract alongside the facade functions themselves.

pub mod context;
pub mod enums;
pub mod network;
pub mod options;
pub mod output;
