// Public API wrapper for sysand JS/WASM binding.
// Re-exports flat wasm-bindgen functions into the namespaced API
// defined in spec/public-api.md.

export { init_logger, ensure_debug_hook, clear_local_storage } from "sysand-wasm";

import {
  init as _init,
  source_add as _source_add,
  source_remove as _source_remove,
  usage_add as _usage_add,
  usage_remove as _usage_remove,
  env_create as _env_create,
  env_list as _env_list,
  env_uninstall as _env_uninstall,
} from "sysand-wasm";

// Root commands
export const init = _init;

// source namespace
export const source = {
  add: _source_add,
  remove: _source_remove,
};

// usage namespace
export const usage = {
  add: _usage_add,
  remove: _usage_remove,
};

// env namespace
export const env = {
  create: _env_create,
  list: _env_list,
  uninstall: _env_uninstall,
};
