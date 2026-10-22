import type { CancellationToken, Disposable } from 'vscode'
import { isWorkerResponse } from './protocol'
import type { WorkerJob, WorkerResponse } from './protocol'

export interface WorkerPort {
  postMessage(message: unknown): void
  terminate(): void
  onmessage: ((event: MessageEvent<unknown>) => void) | null
  onerror: ((event: ErrorEvent) => void) | null
  onmessageerror: ((event: MessageEvent<unknown>) => void) | null
}

interface Pending {
  id: number
  job: WorkerJob
  resolve: (response: WorkerResponse) => void
  reject: (error: Error) => void
  cancellation?: Disposable
}

export class Cancelled extends Error {}

/**
 * Only one job is posted at a time. Cancelled queued work never reaches WASM;
 * cancelled running work retains its slot until completion, bounding the queue.
 */
export class WorkerClient {
  private queue: Pending[] = []
  private active?: Pending
  private timer?: ReturnType<typeof setTimeout>
  private nextId = 0
  private disposed = false

  constructor(
    private worker: WorkerPort,
    private failed: () => void,
    private timeoutMs = 30_000,
  ) {
    worker.onmessage = event => {
      const response = event.data
      const active = this.active
      if (!active || !isWorkerResponse(response) || response.id !== active.id) {
        this.dispose(new Error('invalid formatter worker response'))
        return
      }
      clearTimeout(this.timer)
      this.active = undefined
      active.cancellation?.dispose()
      if (response.kind === 'error') {
        active.reject(new Error(response.error))
        if (response.fatal) {
          this.dispose(new Error(response.error))
          return
        }
      } else active.resolve(response)
      this.dispatch()
    }
    worker.onerror = event =>
      this.dispose(new Error(`formatter worker failed: ${event.message}`))
    worker.onmessageerror = () =>
      this.dispose(new Error('formatter worker message could not be decoded'))
  }

  request(job: WorkerJob, token?: CancellationToken): Promise<WorkerResponse> {
    if (this.disposed)
      return Promise.reject(new Error('formatter worker is stopped'))
    if (token?.isCancellationRequested) return Promise.reject(new Cancelled())
    if (this.queue.length >= 32)
      return Promise.reject(new Error('formatter queue is full; retry later'))
    return new Promise((resolve, reject) => {
      const pending: Pending = { id: ++this.nextId, job, resolve, reject }
      this.queue.push(pending)
      pending.cancellation = token?.onCancellationRequested(() => {
        this.queue = this.queue.filter(item => item !== pending)
        pending.cancellation?.dispose()
        reject(new Cancelled())
      })
      this.dispatch()
    })
  }

  private dispatch(): void {
    if (this.active || this.disposed) return
    const pending = this.queue.shift()
    if (!pending) return
    this.active = pending
    this.timer = setTimeout(
      () =>
        this.dispose(
          new Error(
            'formatter worker timed out; retry to start a new instance',
          ),
        ),
      this.timeoutMs,
    )
    try {
      this.worker.postMessage({ ...pending.job, id: pending.id })
    } catch (error) {
      this.dispose(new Error('cannot send formatter request', { cause: error }))
    }
  }

  dispose(error = new Cancelled('formatter restarted')): void {
    if (this.disposed) return
    this.disposed = true
    clearTimeout(this.timer)
    this.worker.terminate()
    for (const pending of [
      ...this.queue,
      ...(this.active ? [this.active] : []),
    ]) {
      pending.cancellation?.dispose()
      pending.reject(error)
    }
    this.queue = []
    this.active = undefined
    this.failed()
  }
}
