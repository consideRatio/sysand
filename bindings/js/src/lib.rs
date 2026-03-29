// SPDX-FileCopyrightText: © 2025 Sysand contributors <opensource@sensmetry.com>
// SPDX-License-Identifier: MIT OR Apache-2.0

use wasm_bindgen::prelude::*;

pub mod env;
pub mod io;

#[cfg(feature = "browser")]
mod local_storage_utils;

// ---------------------------------------------------------------------------
// Unified error conversion
// ---------------------------------------------------------------------------

fn sysand_err(err: sysand_core::SysandError) -> JsValue {
    // TODO: return structured { code, message, context } object
    JsValue::from_str(&format!("[{}] {}", err.code, err.message))
}

/// Convert the JS/WASM storage error to SysandError so the facade's
/// `where P::Error: Into<SysandError>` bound is satisfied.
#[cfg(feature = "browser")]
impl From<io::local_storage::Error> for sysand_core::SysandError {
    fn from(err: io::local_storage::Error) -> Self {
        use io::local_storage::Error;
        match err {
            Error::AlreadyExists(path) => sysand_core::SysandError::with_context(
                sysand_core::ErrorCode::EnvConflict,
                "already exists",
                path.to_string(),
            ),
            Error::Io(e) => sysand_core::SysandError::from(e),
            Error::KeyNotFound(key) => sysand_core::SysandError::with_context(
                sysand_core::ErrorCode::PathNotFound,
                "key not found in localStorage",
                key,
            ),
            _ => sysand_core::SysandError::new(
                sysand_core::ErrorCode::Internal,
                err.to_string(),
            ),
        }
    }
}

// ---------------------------------------------------------------------------
// Utilities
// ---------------------------------------------------------------------------

#[wasm_bindgen(js_name = init_logger)]
pub fn init_logger() {
    let _ = console_log::init_with_level(log::Level::Debug);
}

#[wasm_bindgen(js_name = ensure_debug_hook)]
pub fn ensure_debug_hook() {
    console_error_panic_hook::set_once();
}

#[cfg(feature = "browser")]
#[wasm_bindgen(js_name = clear_local_storage)]
pub fn clear_local_storage(prefix: &str) -> Result<(), JsValue> {
    let local_storage = local_storage_utils::get_local_browser_storage(prefix)
        .map_err(|e| JsValue::from_str(&e.to_string()))?;
    local_storage.local_storage.clear()
}

// ---------------------------------------------------------------------------
// Facade commands
// ---------------------------------------------------------------------------

#[cfg(feature = "browser")]
#[wasm_bindgen(js_name = init)]
pub fn init(
    name: String,
    publisher: Option<String>,
    version: String,
    prefix: &str,
    root_path: &str,
    license: Option<String>,
) -> Result<(), JsValue> {
    use typed_path::Utf8UnixPath;
    use sysand_core::types::options::InitOptions;

    let mut project = io::local_storage::ProjectLocalBrowserStorage {
        vfs: local_storage_utils::get_local_browser_storage(prefix)
            .map_err(|e| JsValue::from_str(&e.to_string()))?,
        root_path: Utf8UnixPath::new(root_path).to_path_buf(),
    };

    sysand_core::facade::init::init(
        &mut project,
        InitOptions {
            name: Some(name),
            publisher,
            version: Some(version),
            license,
            allow_non_spdx: false,
        },
    )
    .map_err(sysand_err)
}

#[cfg(feature = "browser")]
#[wasm_bindgen(js_name = env_create)]
pub fn env_create(prefix: &str, root_path: &str) -> Result<(), JsValue> {
    use typed_path::Utf8UnixPath;
    use crate::env::local_storage::{DEFAULT_ENV_NAME, empty_environment_local_storage};

    empty_environment_local_storage(prefix, Utf8UnixPath::new(root_path).join(DEFAULT_ENV_NAME))
        .map_err(|e| JsValue::from_str(&e.to_string()))?;

    Ok(())
}
