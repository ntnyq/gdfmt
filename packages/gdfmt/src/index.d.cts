export type LanguageVersion = '4.3' | '4.4' | '4.5' | '4.6' | '4.7'

export interface LanguageOptions {
  godotVersion?: LanguageVersion
}

export type CollectionLayout = 'auto' | 'multiline' | 'preserve'

export interface FormatOptions {
  /**
   * Target width, integer 40–320; long indivisible or retained text may exceed it.
   * Defaults to 100.
   */
  lineWidth?: number
  indentStyle?: 'tab' | 'space'
  /**
   * Space indent size or tab display width, integer 1–8. Defaults to 4.
   */
  indentWidth?: number
  /**
   * Structural newline policy. Defaults to lf; literal/protected bytes are retained.
   */
  lineEnding?: 'lf' | 'crlf' | 'preserve'
  /**
   * Maximum retained blank lines, integer 0–5. Defaults to 2.
   */
  maxBlankLines?: number
  finalNewline?: boolean
  arrayLayout?: CollectionLayout
  dictionaryLayout?: CollectionLayout
  enumLayout?: CollectionLayout
  dictionarySpacing?: 'compact' | 'spaced'
  trailingCommaLayout?: 'auto' | 'multiline'
  blankLinesAroundDefinitions?: 'preserve' | 0 | 1 | 2 | 3 | 4 | 5
  lambdaBodyStyle?: 'auto' | 'multiline'
  inlineCommentSpacing?: 1 | 2 | 3 | 4
}

/**
 * Pure text options. No filesystem discovery or configuration inheritance.
 * Omit unused fields; explicit undefined property values are invalid at runtime.
 */
export interface TextOptions {
  language?: LanguageOptions
  format?: FormatOptions
  /**
   * Diagnostic source identifier; an integer from 0 to 4294967295. Defaults to 0.
   */
  sourceId?: number
}

/**
 * Half-open offsets in UTF-8 bytes, including BOM and physical CRLF bytes.
 */
export interface Span {
  start: number
  end: number
}

export interface Diagnostic {
  code: string
  category: 'lexical' | 'syntax' | 'configuration' | 'io' | 'internal'
  severity: 'error' | 'warning' | 'info'
  message: string
  sourceId: number
  span: Span
}

export interface FormatResult {
  schemaVersion: 1
  spanEncoding: 'utf8Bytes'
  /**
   * Null on source errors or a failed formatter invariant; never partial output.
   */
  output: string | null
  /**
   * Byte comparison of output and input; false when output is null.
   */
  changed: boolean
  diagnostics: Diagnostic[]
}

export interface ResolvedConfig {
  schemaVersion: 1
  language: Required<LanguageOptions>
  format: Required<FormatOptions>
}

export interface ResolveConfigOptions {
  /**
   * Absolute directory, captured at call time. Defaults to process.cwd().
   */
  cwd?: string
  /**
   * Explicit local JSON/JSONC file; relative to cwd. Disables ancestor config discovery.
   */
  config?: string
}

export interface ConfigResolution {
  config: ResolvedConfig
  files: string[]
  editorconfigFiles: string[]
  sources: Record<string, string>
  ignored: boolean
}

/**
 * Formats in the calling thread. Invalid options throw; source errors are data.
 */
export declare function formatSync(
  source: string,
  options?: TextOptions,
): FormatResult
/**
 * Runs parsing, formatting and invariant checks on a libuv worker.
 * Invalid options reject; source errors resolve with output: null.
 */
export declare function format(
  source: string,
  options?: TextOptions,
): Promise<FormatResult>
/**
 * Resolves local configuration for an existing or virtual path, including ignores.
 * Does not format or write files. Host/configuration failures throw.
 */
export declare function resolveConfigSync(
  path: string,
  options?: ResolveConfigOptions,
): ConfigResolution
/**
 * Resolves configuration on a libuv worker. Host/configuration failures reject.
 */
export declare function resolveConfig(
  path: string,
  options?: ResolveConfigOptions,
): Promise<ConfigResolution>
