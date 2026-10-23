import type { FormatResult, TextOptions } from '@gdfmt/wasm'

export type PlaygroundErrorCode =
  | 'formatTimeout'
  | 'loadTimeout'
  | 'workerStopped'
  | 'unreadableResponse'
  | 'loadFailed'
  | 'formatFailed'

export interface PlaygroundError {
  code: PlaygroundErrorCode
  detail?: string
}

export interface FormatRequest {
  id: number
  source: string
  options: TextOptions
}

export type WorkerResponse =
  | { type: 'ready' }
  | { type: 'result'; id: number; result: FormatResult; duration: number }
  | { type: 'error'; id: number | null; message: string }

/**
 * Converts a UTF-8 diagnostic byte offset to a JavaScript string position.
 */
export function byteOffsetToPosition(
  source: string,
  offset: number,
): { lineNumber: number; column: number } {
  let bytes = 0
  let lineNumber = 1
  let column = 1
  const encoder = new TextEncoder()
  for (const character of source) {
    if (bytes >= offset) break
    bytes += encoder.encode(character).length
    if (character === '\n') {
      lineNumber++
      column = 1
    } else {
      column += character.length
    }
  }
  return { lineNumber, column }
}
