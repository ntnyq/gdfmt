import { createFormatter } from '@gdfmt/wasm'
import wasmUrl from '@gdfmt/wasm/gdfmt.wasm?url'
import type { FormatRequest, WorkerResponse } from './protocol'

function reply(message: WorkerResponse): void {
  self.postMessage(message)
}

try {
  const formatter = await createFormatter({ wasm: wasmUrl })
  self.onmessage = ({ data }: MessageEvent<FormatRequest>) => {
    try {
      const start = performance.now()
      const result = formatter.formatSync(data.source, data.options)
      reply({
        type: 'result',
        id: data.id,
        result,
        duration: performance.now() - start,
      })
    } catch (error) {
      reply({
        type: 'error',
        id: data.id,
        message: error instanceof Error ? error.message : String(error),
      })
    }
  }
  reply({ type: 'ready' })
} catch (error) {
  reply({
    type: 'error',
    id: null,
    message: error instanceof Error ? error.message : String(error),
  })
}
