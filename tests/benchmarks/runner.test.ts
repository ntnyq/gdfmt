import assert from 'node:assert/strict'
import { mkdtemp, rm, writeFile } from 'node:fs/promises'
import { tmpdir } from 'node:os'
import { resolve } from 'node:path'
import { test } from 'node:test'
import { findRejections, summarize } from '../../scripts/benchmarks/run.ts'

test('statistics retain outliers and use input bytes and median throughput', () => {
  const samples = [40, 10, 30, 20]
  const statistics = summarize(samples, 1024 ** 2)
  assert.equal(statistics.medianMs, 25)
  assert.equal(statistics.meanMs, 25)
  assert.equal(statistics.minMs, 10)
  assert.equal(statistics.maxMs, 40)
  assert.equal(statistics.p95Ms, 40)
  assert.equal(statistics.standardDeviationMs, Math.sqrt(125))
  assert.equal(statistics.mibPerSecond, 40)
  assert.deepEqual(samples, [40, 10, 30, 20])
  assert.equal(summarize([5, 20, 10], 0).medianMs, 10)
  for (const invalid of [[], [0], [-1], [NaN], [Infinity]])
    assert.throws(() => summarize(invalid, 1))
})

test('preflight isolates every rejected file even after partial batch writes', async () => {
  const sandbox = await mkdtemp(resolve(tmpdir(), 'gdfmt-bench-test-'))
  try {
    const adapter = resolve(sandbox, 'formatter.mjs')
    await writeFile(
      adapter,
      `
      import { readdirSync, readFileSync, writeFileSync } from 'node:fs';
      let rejected = false;
      for (const file of readdirSync('.')) {
        const source = readFileSync(file, 'utf8');
        if (source.includes('BROKEN')) rejected = true;
        else writeFileSync(file, 'formatted');
      }
      if (rejected) { console.error('unsupported syntax'); process.exit(2); }
    `,
    )
    const rejections = await findRejections(
      { id: 'fake', binary: process.execPath, args: [adapter] },
      resolve(sandbox, 'work'),
      [
        { path: 'valid-a.gd', source: 'valid' },
        { path: 'invalid-a.gd', source: 'BROKEN' },
        { path: 'valid-b.gd', source: 'valid' },
        { path: 'invalid-b.gd', source: 'BROKEN' },
      ],
      10_000,
    )
    assert.deepEqual(
      rejections.map(entry => entry.path),
      ['invalid-a.gd', 'invalid-b.gd'],
    )
    assert.ok(
      rejections.every(
        entry =>
          entry.tool === 'fake' &&
          entry.diagnostic.includes('unsupported syntax'),
      ),
    )
  } finally {
    await rm(sandbox, { recursive: true, force: true })
  }
})

test('missing tools fail preflight instead of silently shrinking the corpus', async () => {
  const sandbox = await mkdtemp(resolve(tmpdir(), 'gdfmt-bench-test-'))
  try {
    await assert.rejects(
      findRejections(
        { id: 'missing', binary: resolve(sandbox, 'missing'), args: [] },
        resolve(sandbox, 'work'),
        [{ path: 'valid.gd', source: 'extends Node\n' }],
        1000,
      ),
    )
  } finally {
    await rm(sandbox, { recursive: true, force: true })
  }
})
