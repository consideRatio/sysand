// SPDX-FileCopyrightText: © 2026 Sysand contributors <opensource@sensmetry.com>
// SPDX-License-Identifier: MIT OR Apache-2.0

#![allow(refining_impl_trait)]

pub mod commands;
pub use commands::*;

pub mod model;

#[cfg(feature = "networking")]
pub mod auth;
pub mod config;
pub mod context;
pub mod env;
pub mod lock;
pub mod project;
pub mod resolve;
pub mod solve;
pub mod stdlib;
pub mod style;
pub mod symbols;

#[cfg(feature = "filesystem")]
pub mod workspace;

#[cfg(feature = "filesystem")]
pub mod discover;

#[cfg(not(feature = "std"))]
compile_error!("`std` feature is currently required to build `sysand`");

// Private tests

#[cfg(test)]
mod tests {
    //use crate::{Message, get_message};

    #[test]
    fn placeholder_test() {
        assert_eq!(1, 1);
    }
}
