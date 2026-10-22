import assert from 'node:assert/strict'
import { test } from 'node:test'
import type { CancellationToken } from 'vscode'
import {
  replacement,
  utf16Offsets,
} from '../../editors/vscode/src/web/positions.ts'
import {
  Cancelled,
  WorkerClient,
} from '../../editors/vscode/src/web/worker-client.ts'
import type { WorkerPort } from '../../editors/vscode/src/web/worker-client.ts'

class Port implements WorkerPort {
  onmessage: WorkerPort['onmessage'] = null
  onerror: WorkerPort['onerror'] = null
  onmessageerror: WorkerPort['onmessageerror'] = null
  messages: unknown[] = []
  stopped = false
  postMessage(message: unknown): void {
    this.messages.push(message)
  }
  terminate(): void {
    this.stopped = true
  }
  reply(data: unknown): void {
    this.onmessage?.(new MessageEvent('message', { data }))
  }
}

function cancellation(): { token: CancellationToken; cancel(): void } {
  let cancelled = false
  const listeners = new Set<() => void>()
  return {
    token: {
      get isCancellationRequested() {
        return cancelled
      },
      onCancellationRequested(listener) {
        const callback = () => listener(undefined)
        listeners.add(callback)
        return {
          dispose() {
            listeners.delete(callback)
          },
        }
      },
    },
    cancel() {
      cancelled = true
      for (const listener of listeners) listener()
    },
  }
}

test('cancelled queued work is skipped; running work retains its slot and cannot resolve stale results', async () => {
  const port = new Port()
  const client = new WorkerClient(port, () => {})
  const active = cancellation()
  const first = client.request(
    { kind: 'parseConfig', source: '{}' },
    active.token,
  )
  const queued = cancellation()
  const second = client.request(
    { kind: 'parseConfig', source: '{}' },
    queued.token,
  )
  const firstCancelled = assert.rejects(first, Cancelled)
  const secondCancelled = assert.rejects(second, Cancelled)
  active.cancel()
  queued.cancel()
  await Promise.all([firstCancelled, secondCancelled])
  const third = client.request({ kind: 'parseConfig', source: '{}' })
  assert.equal(port.messages.length, 1)
  port.reply({ id: 1, kind: 'config', config: {} })
  assert.equal(port.messages.length, 2)
  port.reply({ id: 3, kind: 'config', config: {} })
  assert.equal((await third).kind, 'config')
  client.dispose()
})

test('worker traps, malformed replies, timeouts and restart reject all pending work', async () => {
  for (const failure of ['trap', 'invalid', 'timeout', 'restart']) {
    const port = new Port()
    let failures = 0
    const client = new WorkerClient(
      port,
      () => {
        failures++
      },
      20,
    )
    const first = client.request({ kind: 'parseConfig', source: '{}' })
    const second = client.request({ kind: 'parseConfig', source: '{}' })
    const rejected = Promise.all([
      assert.rejects(first),
      assert.rejects(second),
    ])
    if (failure === 'trap')
      port.reply({ id: 1, kind: 'error', fatal: true, error: 'trap' })
    if (failure === 'invalid') port.reply({ id: 999, kind: 'ready' })
    if (failure === 'restart') client.dispose()
    await rejected
    assert.equal(port.stopped, true)
    assert.equal(failures, 1)
  }
})

test('ordinary option errors leave the worker available for later requests', async () => {
  const port = new Port()
  const client = new WorkerClient(port, () => {})
  const first = client.request({ kind: 'parseConfig', source: 'invalid' })
  const rejected = assert.rejects(first, /invalid config/)
  port.reply({ id: 1, kind: 'error', error: 'invalid config', fatal: false })
  await rejected
  const next = client.request({ kind: 'parseConfig', source: '{}' })
  port.reply({ id: 2, kind: 'config', config: {} })
  assert.equal((await next).kind, 'config')
  client.dispose()
})

test('UTF-8 diagnostics and minimal edits preserve Chinese, emoji, BOM and CRLF boundaries', () => {
  const source = '\uFEFF# 名称😀\r\n'
  const bytes = new TextEncoder().encode(source)
  const offsets = utf16Offsets(source, [0, 3, 5, 8, 11, 15, bytes.length])
  assert.deepEqual([...offsets.values()], [0, 1, 3, 4, 5, 7, 9])
  assert.throws(() => utf16Offsets(source, [6]), /boundary/)
  for (const [before, after] of [
    ['var 名称="😀"\r\n', 'var 名称 = "😀"\n'],
    ['# 😀\r\n', '# 😀\n'],
    ['😀', '😁'],
    ['', 'var x = 1\n'],
    ['var x = 1\n', ''],
  ]) {
    const edit = replacement(before, after)
    assert.ok(edit)
    assert.equal(
      before.slice(0, edit.start) + edit.text + before.slice(edit.end),
      after,
    )
    assert.equal(
      new TextDecoder().decode(new TextEncoder().encode(edit.text)),
      edit.text,
    )
  }
  assert.equal(replacement('same', 'same'), undefined)
})
