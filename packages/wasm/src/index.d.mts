import type {
  TextOptions,
  FormatResult,
  LanguageOptions,
  FormatOptions,
} from '../../gdfmt/src/index.d.cts'
export type {
  LanguageVersion,
  LanguageOptions,
  CollectionLayout,
  FormatOptions,
  TextOptions,
  Span,
  Diagnostic,
  FormatResult,
} from '../../gdfmt/src/index.d.cts'

export interface InitOptions {
  /**
   * Defaults to the WASM file beside the ESM entry. For Node, pass bytes or a
   * precompiled module. URL responses must allow CORS when hosted separately.
   */
  wasm?: string | URL | Response | ArrayBuffer | Uint8Array | WebAssembly.Module
}

export interface Formatter {
  /**
   * Parses one JSON/JSONC configuration layer. Path-based overrides and ignore
   * rules throw, including empty arrays. Input is limited to 1 MiB of UTF-8.
   * The host must load extends and decide its discovery boundary.
   */
  parseConfig(source: string): TextConfig
  /**
   * Merges at most 128 flattened language/format layers and applies shared Rust
   * defaults. Omit absent fields; explicit undefined property values are invalid.
   */
  resolveTextOptions(
    layers: readonly Pick<TextConfig, 'language' | 'format'>[],
  ): TextOptions
  /**
   * Invalid options throw; invalid GDScript returns output: null and diagnostics.
   * Executes synchronously; move the formatter into a Worker for large inputs.
   */
  formatSync(source: string, options?: TextOptions): FormatResult
}

export interface TextConfig {
  /**
   * Editor metadata only; the formatter does not fetch this address.
   */
  $schema?: string
  /**
   * Configuration format version, independent of the package release version.
   */
  schemaVersion?: 1
  root?: boolean
  extends?: string[]
  language?: LanguageOptions
  format?: FormatOptions
}

/**
 * Initializes an independent formatter without threads or cross-origin isolation.
 */
export declare function createFormatter(
  options?: InitOptions,
): Promise<Formatter>
