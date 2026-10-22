import { createFormatter } from '@gdfmt/wasm'
import type { Formatter } from '@gdfmt/wasm'
import type { WorkerRequest, WorkerResponse } from './protocol'

let formatter: Formatter | undefined
// Serial initialization also covers messages arriving while WASM is loading.
let operation = Promise.resolve()
self.addEventListener('message', (event: MessageEvent<WorkerRequest>) => {
  operation = operation.then(async () => {
    const request = event.data
    let response: WorkerResponse
    try {
      if (request.kind === 'initialize') {
        formatter = await createFormatter({ wasm: request.wasm })
        response = { id: request.id, kind: 'ready' }
      } else {
        if (!formatter) throw new Error('formatter is not initialized')
        switch (request.kind) {
          case 'format':
            response = {
              id: request.id,
              kind: 'format',
              result: formatter.formatSync(request.source, request.options),
            }
            break
          case 'parseConfig':
            response = {
              id: request.id,
              kind: 'config',
              config: formatter.parseConfig(request.source),
            }
            break
          case 'resolveOptions':
            response = {
              id: request.id,
              kind: 'options',
              options: formatter.resolveTextOptions(request.layers),
            }
            break
        }
      }
    } catch (error) {
      response = {
        id: request.id,
        kind: 'error',
        error: String(error),
        fatal:
          request.kind === 'initialize' ||
          error instanceof WebAssembly.RuntimeError,
      }
    }
    self.postMessage(response)
  })
})
