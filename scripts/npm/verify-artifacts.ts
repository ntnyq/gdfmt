import assert from 'node:assert/strict'
import { spawnSync } from 'node:child_process'
import { createHash } from 'node:crypto'
import { readFile, writeFile } from 'node:fs/promises'
import { join, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'
import { gitRevision, readObject } from '../release/project.ts'
import { metadata, targets } from './build.ts'

export interface VerifiedReceipt {
  target: string
  node: string
  rootSha512: string
  nativeSha512: string
}

export interface VerifiedMatrix {
  schemaVersion: number
  version: string
  revision: string
  completeMatrix: boolean
  cleanSource: boolean
  published: boolean
  receipts: VerifiedReceipt[]
}

export function packedMetadata(path: string): Record<string, unknown> {
  const result = spawnSync('tar', ['-xOzf', path, 'package/package.json'], {
    encoding: 'utf8',
  })
  if (result.error) throw result.error
  assert.equal(result.status, 0, result.stderr)
  return readObject(JSON.parse(result.stdout))
}

/**
 * Recomputes every receipt from the tested tarballs and the current commit.
 */
export async function verifyArtifacts(
  directory: string,
  local: boolean = false,
): Promise<VerifiedMatrix> {
  const runtimeHeader = readObject(
    JSON.parse(JSON.stringify(process.report.getReport())),
  ).header
  const libc =
    runtimeHeader &&
    typeof runtimeHeader === 'object' &&
    'glibcVersionRuntime' in runtimeHeader
      ? 'glibc'
      : 'musl'
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
  const receipts: VerifiedReceipt[] = []
  const revision = gitRevision()
  let cleanSource = true
  for (const entry of entries) {
    const receipt = readObject(
      JSON.parse(
        await readFile(join(entry.directory, 'acceptance.json'), 'utf8'),
      ),
    )
    assert.equal(receipt.passed, true)
    assert.equal(receipt.published, false)
    cleanSource &&= receipt.cleanSource === true
    if (!local)
      assert.equal(
        receipt.cleanSource,
        true,
        'release artifacts must be built from clean source',
      )
    assert.equal(
      receipt.revision,
      revision,
      'receipt must belong to the release commit',
    )
    assert.equal(receipt.target, entry.target.id)
    assert.equal(receipt.triple, entry.target.triple)
    assert.equal(receipt.version, metadata.version)
    assert.equal(receipt.node, entry.node)
    assert.equal(receipt.tests, 8)
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
      receipt.rootSha512,
      hash,
      'root tarball must match the tested bytes',
    )
    const nativeHash = createHash('sha512')
      .update(await readFile(nativeTarball))
      .digest('hex')
    assert.equal(
      receipt.nativeSha512,
      nativeHash,
      'native tarball must match the tested bytes',
    )
    assert.equal(
      hash,
      rootHash,
      'every platform must test the identical root tarball',
    )
    receipts.push({
      target: entry.target.id,
      node: entry.node,
      rootSha512: hash,
      nativeSha512: nativeHash,
    })
  }
  const matrix: VerifiedMatrix = {
    schemaVersion: 1,
    version: metadata.version,
    revision,
    completeMatrix: !local,
    cleanSource,
    published: false,
    receipts,
  }
  await writeFile(
    join(directory, 'verified.json'),
    `${JSON.stringify(matrix, null, 2)}\n`,
  )
  console.log(
    `Verified ${entries.length} packaged artifact receipt(s), exact versions and root parity. Complete matrix: ${!local}. No publication performed.`,
  )
  return matrix
}

if (
  process.argv[1] &&
  resolve(process.argv[1]) === fileURLToPath(import.meta.url)
) {
  const args = process.argv.slice(2)
  assert.ok(
    args.filter(arg => arg !== '--local').length <= 1 &&
      args.every(arg => !arg.startsWith('--') || arg === '--local'),
    'usage: pnpm verify:npm [DIRECTORY] [--local]',
  )
  const local = args.includes('--local')
  await verifyArtifacts(
    resolve(
      args.find(arg => arg !== '--local') ||
        (local ? 'target/npm/artifacts' : 'target/npm-ci'),
    ),
    local,
  )
}
