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
    let obj = js_sys::Object::new();
    let _ = js_sys::Reflect::set(&obj, &"code".into(), &err.code.as_kebab_str().into());
    let _ = js_sys::Reflect::set(&obj, &"message".into(), &err.message.into());
    if let Some(ctx) = err.context {
        let _ = js_sys::Reflect::set(&obj, &"context".into(), &ctx.into());
    }
    obj.into()
}

/// Convert project storage errors to SysandError for facade bounds.
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

/// Convert environment storage errors to SysandError for facade bounds.
#[cfg(feature = "browser")]
impl From<crate::env::local_storage::Error> for sysand_core::SysandError {
    fn from(err: crate::env::local_storage::Error) -> Self {
        sysand_core::SysandError::new(sysand_core::ErrorCode::IoError, err.to_string())
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
// Helper: construct browser project storage
// ---------------------------------------------------------------------------

#[cfg(feature = "browser")]
fn open_browser_project(
    prefix: &str,
    root_path: &str,
) -> Result<io::local_storage::ProjectLocalBrowserStorage, JsValue> {
    use typed_path::Utf8UnixPath;
    Ok(io::local_storage::ProjectLocalBrowserStorage {
        vfs: local_storage_utils::get_local_browser_storage(prefix)
            .map_err(|e| JsValue::from_str(&e.to_string()))?,
        root_path: Utf8UnixPath::new(root_path).to_path_buf(),
    })
}

#[cfg(feature = "browser")]
fn open_browser_env(
    prefix: &str,
    root_path: &str,
) -> Result<env::local_storage::LocalBrowserStorageEnvironment, JsValue> {
    use typed_path::Utf8UnixPath;
    use crate::env::local_storage::DEFAULT_ENV_NAME;
    Ok(env::local_storage::LocalBrowserStorageEnvironment {
        root_path: Utf8UnixPath::new(root_path).join(DEFAULT_ENV_NAME),
        vfs: local_storage_utils::get_local_browser_storage(prefix)
            .map_err(|e| JsValue::from_str(&e.to_string()))?,
    })
}

// ---------------------------------------------------------------------------
// Root commands
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
    use sysand_core::types::options::InitOptions;

    let mut project = open_browser_project(prefix, root_path)?;
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

// ---------------------------------------------------------------------------
// source namespace
// ---------------------------------------------------------------------------

#[cfg(feature = "browser")]
#[wasm_bindgen(js_name = source_add)]
pub fn source_add(
    prefix: &str,
    root_path: &str,
    src_path: &str,
    checksum: Option<bool>,
    index_symbols: Option<bool>,
) -> Result<(), JsValue> {
    use sysand_core::types::{enums::{ChecksumMode, IndexSymbols}, options::SourceAddOptions};

    let mut project = open_browser_project(prefix, root_path)?;
    sysand_core::facade::source::add(
        &mut project,
        typed_path::Utf8UnixPath::new(src_path),
        SourceAddOptions {
            checksum: if checksum.unwrap_or(false) { ChecksumMode::Sha256 } else { ChecksumMode::None },
            index_symbols: if index_symbols.unwrap_or(true) { IndexSymbols::On } else { IndexSymbols::Off },
            ..Default::default()
        },
    )
    .map_err(sysand_err)
}

#[cfg(feature = "browser")]
#[wasm_bindgen(js_name = source_remove)]
pub fn source_remove(
    prefix: &str,
    root_path: &str,
    src_path: &str,
) -> Result<(), JsValue> {
    let mut project = open_browser_project(prefix, root_path)?;
    sysand_core::facade::source::remove(
        &mut project,
        typed_path::Utf8UnixPath::new(src_path),
    )
    .map_err(sysand_err)
}

// ---------------------------------------------------------------------------
// usage namespace
// ---------------------------------------------------------------------------

#[cfg(feature = "browser")]
#[wasm_bindgen(js_name = usage_add)]
pub fn usage_add(
    prefix: &str,
    root_path: &str,
    iri: &str,
    version_req: Option<String>,
) -> Result<(), JsValue> {
    let mut project = open_browser_project(prefix, root_path)?;
    sysand_core::facade::usage::add(
        &mut project,
        iri,
        version_req.as_deref(),
    )
    .map(|_| ())
    .map_err(sysand_err)
}

#[cfg(feature = "browser")]
#[wasm_bindgen(js_name = usage_remove)]
pub fn usage_remove(
    prefix: &str,
    root_path: &str,
    iri: &str,
) -> Result<(), JsValue> {
    let mut project = open_browser_project(prefix, root_path)?;
    sysand_core::facade::usage::remove(&mut project, iri)
        .map(|_| ())
        .map_err(sysand_err)
}

// ---------------------------------------------------------------------------
// env namespace
// ---------------------------------------------------------------------------

#[cfg(feature = "browser")]
#[wasm_bindgen(js_name = env_create)]
pub fn env_create(prefix: &str, root_path: &str) -> Result<(), JsValue> {
    use typed_path::Utf8UnixPath;
    use crate::env::local_storage::{DEFAULT_ENV_NAME, empty_environment_local_storage};

    empty_environment_local_storage(prefix, Utf8UnixPath::new(root_path).join(DEFAULT_ENV_NAME))
        .map_err(|e| JsValue::from_str(&e.to_string()))?;

    Ok(())
}

#[cfg(feature = "browser")]
#[wasm_bindgen(js_name = env_list)]
pub fn env_list(prefix: &str, root_path: &str) -> Result<JsValue, JsValue> {
    let env = open_browser_env(prefix, root_path)?;
    let entries = sysand_core::facade::env::list(env).map_err(sysand_err)?;
    let result: Vec<JsValue> = entries
        .into_iter()
        .map(|e| {
            let obj = js_sys::Object::new();
            let _ = js_sys::Reflect::set(&obj, &"iri".into(), &e.iri.into());
            let _ = js_sys::Reflect::set(
                &obj,
                &"version".into(),
                &e.version.map(|v| JsValue::from_str(&v)).unwrap_or(JsValue::UNDEFINED),
            );
            obj.into()
        })
        .collect();
    let arr = js_sys::Array::new();
    for v in result {
        arr.push(&v);
    }
    Ok(arr.into())
}

#[cfg(feature = "browser")]
#[wasm_bindgen(js_name = env_uninstall)]
pub fn env_uninstall(
    prefix: &str,
    root_path: &str,
    iri: &str,
    version: Option<String>,
) -> Result<(), JsValue> {
    let env = open_browser_env(prefix, root_path)?;
    sysand_core::facade::env::uninstall(env, iri, version.as_deref()).map_err(sysand_err)
}
