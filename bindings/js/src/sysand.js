// Public API wrapper for sysand JS/WASM binding.
// Re-exports flat wasm-bindgen functions into the namespaced API
// defined in spec/public-api.md.

// Import all flat exports from wasm-pack generated module.
// The actual import path is resolved by webpack to browser_pkg/.
export { init_logger, ensure_debug_hook, clear_local_storage } from "sysand-wasm";

import { init as _init, env_create as _env_create } from "sysand-wasm";

// Root commands
export const init = _init;

// env namespace
export const env = {
  create: _env_create,
};
