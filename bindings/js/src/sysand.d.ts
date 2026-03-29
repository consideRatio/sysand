// Public API type definitions for sysand JS/WASM binding.
// Matches spec/public-api.md.

export interface SysandError {
  code: string;
  message: string;
  context?: string;
}

export interface EnvEntry {
  iri: string;
  version?: string;
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

// source namespace
export const source: {
  add(
    prefix: string,
    rootPath: string,
    srcPath: string,
    checksum?: boolean,
    indexSymbols?: boolean,
  ): void;
  remove(prefix: string, rootPath: string, srcPath: string): void;
};

// usage namespace
export const usage: {
  add(
    prefix: string,
    rootPath: string,
    iri: string,
    versionReq?: string,
  ): void;
  remove(prefix: string, rootPath: string, iri: string): void;
};

// env namespace
export const env: {
  create(prefix: string, rootPath: string): void;
  list(prefix: string, rootPath: string): EnvEntry[];
  uninstall(
    prefix: string,
    rootPath: string,
    iri: string,
    version?: string,
  ): void;
};
