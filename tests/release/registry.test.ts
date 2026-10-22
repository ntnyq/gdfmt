import assert from 'node:assert/strict'
import { test } from 'node:test'
import {
  confirmRegistryWrite,
  registryJson,
  waitForNpmInstall,
} from '../../scripts/release/registry.ts'

test('registry reads back off on transient failures, then validate the successful response', async () => {
  const delays: number[] = []
  let attempts = 0
  const fetcher: typeof fetch = async () => {
    attempts++
    if (attempts === 1) throw new TypeError('network interruption')
    if (attempts === 2)
      return new Response('{}', {
        status: 429,
        headers: { 'retry-after': '2' },
      })
    if (attempts === 3) return new Response('{}', { status: 503 })
    return new Response('{"name":"fixture"}')
  }
  assert.deepEqual(
    await registryJson('https://example.invalid', fetcher, async delay => {
      delays.push(delay)
    }),
    { name: 'fixture' },
  )
  assert.equal(attempts, 4)
  assert.deepEqual(delays, [1_000, 2_000, 4_000])
  for (const status of [401, 403, 404, 500]) {
    let reads = 0
    const failed: typeof fetch = async () => {
      reads++
      return new Response('{}', { status })
    }
    if (status === 404)
      assert.equal(
        await registryJson('https://example.invalid', failed, async () => {}),
        undefined,
      )
    else
      await assert.rejects(
        () => registryJson('https://example.invalid', failed, async () => {}),
        /registry request failed/,
      )
    assert.equal(reads, status === 500 ? 4 : 1)
  }
})

test('ambiguous writes are reconciled without ever repeating the write', async () => {
  let writes = 0
  await confirmRegistryWrite(
    () => {
      writes++
      throw new Error('connection lost after commit')
    },
    async () => {},
  )
  assert.equal(writes, 1)
  await assert.rejects(
    () =>
      confirmRegistryWrite(
        () => {
          writes++
          throw new Error('upload rejected')
        },
        async () => {
          throw new Error('bytes not confirmed')
        },
      ),
    AggregateError,
  )
  assert.equal(writes, 2)
  await assert.rejects(
    () =>
      confirmRegistryWrite(
        () => {
          writes++
        },
        async () => {
          throw new Error('integrity conflict')
        },
      ),
    /integrity conflict/,
  )
  assert.equal(writes, 3)
})

test('asynchronous write failures are awaited and reconciled exactly once', async () => {
  const events: string[] = []
  await confirmRegistryWrite(
    async () => {
      await Promise.resolve()
      events.push('write')
      throw new Error('request timed out after submission')
    },
    async () => {
      assert.deepEqual(events, ['write'])
      events.push('confirm')
    },
  )
  assert.deepEqual(events, ['write', 'confirm'])
})

test('an overall registry deadline aborts a pending response body without retrying', async () => {
  const controller = new AbortController()
  const timer = setTimeout(
    () => controller.abort(new DOMException('deadline', 'TimeoutError')),
    30,
  )
  let attempts = 0
  try {
    const fetcher: typeof fetch = async (_input, options) => {
      attempts++
      return new Response(
        new ReadableStream({
          start(stream) {
            options?.signal?.addEventListener(
              'abort',
              () => stream.error(options.signal?.reason),
              { once: true },
            )
          },
        }),
      )
    }
    await assert.rejects(
      () =>
        registryJson(
          'https://example.invalid/body',
          fetcher,
          async () => assert.fail('deadline must not retry'),
          controller.signal,
        ),
      /Registry deadline exceeded/,
    )
    assert.equal(attempts, 1)
  } finally {
    clearTimeout(timer)
  }
})

test('npm install tolerates delayed metadata but fails immediately on authentication and other errors', async () => {
  let attempts = 0
  const delays: number[] = []
  await waitForNpmInstall(
    () => {
      attempts++
      if (attempts < 3)
        throw Object.assign(new Error('install index has not caught up'), {
          stderr: Buffer.from(
            `npm error code ${attempts === 1 ? 'ETARGET' : 'E404'}\n`,
          ),
        })
    },
    async delay => {
      delays.push(delay)
    },
  )
  assert.equal(attempts, 3)
  assert.deepEqual(delays, [5_000, 5_000])
  for (const code of ['E401', 'E403', 'EINTEGRITY', 'ECONNRESET']) {
    let reads = 0
    const original = Object.assign(new Error(code), {
      stderr: `npm error code ${code}\n`,
    })
    await assert.rejects(
      () =>
        waitForNpmInstall(
          () => {
            reads++
            throw original
          },
          async () => {
            assert.fail('unexpected retry')
          },
        ),
      error => error === original,
    )
    assert.equal(reads, 1)
  }
  let exhausted = 0
  const original = Object.assign(new Error('still missing'), {
    stderr: 'npm error code ETARGET\n',
  })
  await assert.rejects(
    () =>
      waitForNpmInstall(
        () => {
          exhausted++
          throw original
        },
        async () => {},
      ),
    error => error === original,
  )
  assert.equal(exhausted, 120)
})
