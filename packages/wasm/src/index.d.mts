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
   * rules throw; the host must load extends and decide its discovery boundary.
   */
  parseConfig(source: string): TextConfig
  /**
   * Merges flattened language/format layers and applies shared Rust defaults.
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
