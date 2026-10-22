import type { FormatResult, TextConfig, TextOptions } from '@gdfmt/wasm'

export type ConfigLayer = Pick<TextConfig, 'language' | 'format'>
export type WorkerJob =
  | { kind: 'initialize'; wasm: Uint8Array }
  | { kind: 'format'; source: string; options: TextOptions }
  | { kind: 'parseConfig'; source: string }
  | { kind: 'resolveOptions'; layers: ConfigLayer[] }

export type WorkerRequest = WorkerJob & { id: number }
export type WorkerResponse = { id: number } & (
  | { kind: 'ready' }
  | { kind: 'format'; result: FormatResult }
  | { kind: 'config'; config: TextConfig }
  | { kind: 'options'; options: TextOptions }
  | { kind: 'error'; error: string; fatal: boolean }
)

/**
 * Values originate from our bundled worker and Rust's validated serializers.
 * Check the envelope before matching it to a pending request.
 */
export function isWorkerResponse(value: unknown): value is WorkerResponse {
  if (
    !value ||
    typeof value !== 'object' ||
    !('id' in value) ||
    !Number.isSafeInteger(value.id) ||
    !('kind' in value)
  )
    return false
  switch (value.kind) {
    case 'ready':
      return true
    case 'format':
      return (
        'result' in value &&
        !!value.result &&
        typeof value.result === 'object' &&
        'schemaVersion' in value.result &&
        value.result.schemaVersion === 1
      )
    case 'config':
      return (
        'config' in value && !!value.config && typeof value.config === 'object'
      )
    case 'options':
      return (
        'options' in value &&
        !!value.options &&
        typeof value.options === 'object'
      )
    case 'error':
      return (
        'error' in value &&
        typeof value.error === 'string' &&
        'fatal' in value &&
        typeof value.fatal === 'boolean'
      )
    default:
      return false
  }
}
