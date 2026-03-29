// Public API type definitions for sysand JS/WASM binding.
// Matches spec/public-api.md.

export interface SysandError {
  code: string;
  message: string;
  context?: string;
}

// Utilities
export function init_logger(): void;
export function ensure_debug_hook(): void;
export function clear_local_storage(prefix: string): void;

// Root commands
export function init(
  name: string,
  publisher: string | undefined,
  version: string,
  prefix: string,
  rootPath: string,
  license?: string,
): void;

// env namespace
export const env: {
  create(prefix: string, rootPath: string): void;
};
