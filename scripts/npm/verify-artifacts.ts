import assert from 'node:assert/strict'
import { spawnSync } from 'node:child_process'
import { createHash } from 'node:crypto'
import { readFile, writeFile } from 'node:fs/promises'
import { join, resolve } from 'node:path'
import { metadata, targets } from './build.ts'

function readObject(text: string): Record<string, unknown> {
  const value: unknown = JSON.parse(text)
  assert.ok(value && typeof value === 'object' && !Array.isArray(value))
  return Object.fromEntries(Object.entries(value))
}

function packedMetadata(path: string): Record<string, unknown> {
  const result = spawnSync('tar', ['-xOzf', path, 'package/package.json'], {
    encoding: 'utf8',
  })
  if (result.error) throw result.error
  assert.equal(result.status, 0, result.stderr)
  return readObject(result.stdout)
}

const local = process.argv.includes('--local')
const runtimeHeader = readObject(
  JSON.stringify(process.report.getReport()),
).header
const libc =
  runtimeHeader &&
  typeof runtimeHeader === 'object' &&
  'glibcVersionRuntime' in runtimeHeader
    ? 'glibc'
    : 'musl'
const directory = resolve(
  process.argv.find(
    value =>
      !value.startsWith('--') &&
      value !== process.argv[0] &&
      value !== process.argv[1],
  ) || (local ? 'target/npm/artifacts' : 'target/npm-ci'),
)
const entries = local
  ? targets
      .filter(
        target =>
          target.os === process.platform &&
          target.cpu === process.arch &&
          (target.os !== 'linux' || target.libc === libc),
      )
      .map(target => ({
        target,
        directory: join(directory, target.id),
        node: process.version,
      }))
  : targets.flatMap(target =>
      ['22.13.0', '24.0.0'].map(node => ({
        target,
        directory: join(directory, `npm-${target.id}-${node}`),
        node: `v${node}`,
      })),
    )
assert.ok(entries.length > 0, 'no matching artifacts')
let rootHash: string | undefined
const receipts = []
for (const entry of entries) {
  const receipt = readObject(
    await readFile(join(entry.directory, 'acceptance.json'), 'utf8'),
  )
  assert.equal(receipt.passed, true)
  assert.equal(receipt.published, false)
  assert.equal(receipt.target, entry.target.id)
  assert.equal(receipt.triple, entry.target.triple)
  assert.equal(receipt.version, metadata.version)
  assert.equal(receipt.node, entry.node)
  const rootTarball = join(entry.directory, `gdfmt-${metadata.version}.tgz`)
  const nativeTarball = join(
    entry.directory,
    `gdfmt-${entry.target.id}-${metadata.version}.tgz`,
  )
  assert.deepEqual(packedMetadata(rootTarball), metadata)
  const native = packedMetadata(nativeTarball)
  assert.equal(native.name, `@gdfmt/${entry.target.id}`)
  assert.equal(native.version, metadata.version)
  assert.deepEqual(native.os, [entry.target.os])
  assert.deepEqual(native.cpu, [entry.target.cpu])
  assert.deepEqual(
    native.libc,
    entry.target.libc ? [entry.target.libc] : undefined,
  )
  const hash = createHash('sha512')
    .update(await readFile(rootTarball))
    .digest('hex')
  rootHash ??= hash
  assert.equal(
    hash,
    rootHash,
    'every platform must test the identical root tarball',
  )
  receipts.push({
    target: entry.target.id,
    node: entry.node,
    rootSha512: hash,
    nativeSha512: createHash('sha512')
      .update(await readFile(nativeTarball))
      .digest('hex'),
  })
}
await writeFile(
  join(directory, 'verified.json'),
  `${JSON.stringify({ schemaVersion: 1, version: metadata.version, completeMatrix: !local, published: false, receipts }, null, 2)}\n`,
)
console.log(
  `Verified ${entries.length} packaged artifact receipt(s), exact versions and root parity. Complete matrix: ${!local}. No publication performed.`,
)
