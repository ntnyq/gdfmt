import assert from 'node:assert/strict'
import { test } from 'node:test'
import {
  confirmRegistryWrite,
  registryJson,
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
