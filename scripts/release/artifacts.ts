import assert from 'node:assert/strict'
import { createHash } from 'node:crypto'
import { readFileSync } from 'node:fs'
import { copyFile, mkdir, writeFile } from 'node:fs/promises'
import { join, resolve } from 'node:path'
import { metadata, targets } from '../npm/build.ts'
import { packedMetadata, verifyArtifacts } from '../npm/verify-artifacts.ts'
import {
  checkRelease,
  gitRevision,
  readJson,
  readObject,
  releaseChannel,
} from './project.ts'

export interface ReleasePackage {
  name: string
  file: string
  integrity: string
}

export interface FrozenRelease {
  schemaVersion: number
  version: string
  revision: string
  channel: 'next' | 'latest'
  packages: ReleasePackage[]
}

export const releaseDirectory: string = resolve('target/release/npm')

export function integrity(path: string): string {
  return `sha512-${createHash('sha512').update(readFileSync(path)).digest('base64')}`
}

/**
 * Selects the Node 24 artifact for each platform after verifying all ten consumers.
 */
export async function prepareRelease(
  source: string,
  destination: string,
): Promise<void> {
  const version = checkRelease()
  const matrix = await verifyArtifacts(source)
  assert.equal(matrix.completeMatrix, true)
  await mkdir(destination, { recursive: true })
  const packages: ReleasePackage[] = []
  for (const target of targets) {
    const file = `gdfmt-${target.id}-${version}.tgz`
    await copyFile(
      join(source, `npm-${target.id}-24.0.0`, file),
      join(destination, file),
    )
    packages.push({
      name: `@gdfmt/${target.id}`,
      file,
      integrity: integrity(join(destination, file)),
    })
  }
  const file = `gdfmt-${version}.tgz`
  await copyFile(
    join(source, `npm-${targets[0].id}-24.0.0`, file),
    join(destination, file),
  )
  packages.push({
    name: 'gdfmt',
    file,
    integrity: integrity(join(destination, file)),
  })
  await writeFile(
    join(destination, 'release.json'),
    `${JSON.stringify({ schemaVersion: 1, version, revision: matrix.revision, channel: releaseChannel(version), packages, matrix }, null, 2)}\n`,
  )
  validateFrozenRelease(destination)
  console.log(
    `Frozen ${packages.length} npm tarballs in ${destination}. No publication performed.`,
  )
}

/**
 * Refuses incomplete matrices, changed tarballs and artifacts from another commit.
 */
export function validateFrozenRelease(directory: string): FrozenRelease {
  const version = checkRelease()
  const manifest = readJson(join(directory, 'release.json'))
  assert.equal(manifest.schemaVersion, 1)
  assert.equal(manifest.version, version)
  const revision = gitRevision()
  assert.equal(manifest.revision, revision)
  const channel = releaseChannel(version)
  assert.equal(manifest.channel, channel)
  const matrix = readObject(manifest.matrix)
  assert.equal(
    matrix.completeMatrix,
    true,
    'a complete platform matrix is required',
  )
  assert.equal(matrix.published, false)
  assert.equal(
    matrix.cleanSource,
    true,
    'release artifacts must be built from clean source',
  )
  assert.equal(matrix.version, version)
  assert.equal(matrix.revision, revision)
  assert.ok(Array.isArray(matrix.receipts))
  const receipts = matrix.receipts.map(readObject)
  assert.equal(receipts.length, targets.length * 2)
  const expected = targets.map(target => ({
    name: `@gdfmt/${target.id}`,
    file: `gdfmt-${target.id}-${version}.tgz`,
  }))
  expected.push({ name: 'gdfmt', file: `gdfmt-${version}.tgz` })
  assert.ok(Array.isArray(manifest.packages))
  const entries = manifest.packages.map(readObject)
  assert.deepEqual(
    entries.map(entry => ({ name: entry.name, file: entry.file })),
    expected,
    'expected exactly five platforms followed by the root package',
  )
  const packages: ReleasePackage[] = expected.map(entry => {
    const path = join(directory, entry.file)
    const actualIntegrity = integrity(path)
    const declared = entries.find(item => item.name === entry.name)
    assert.ok(declared)
    assert.equal(
      declared.integrity,
      actualIntegrity,
      `frozen tarball changed: ${entry.name}`,
    )
    const packed = packedMetadata(path)
    assert.equal(packed.name, entry.name)
    assert.equal(packed.version, version)
    if (entry.name === 'gdfmt') assert.deepEqual(packed, metadata)
    else {
      const target = targets.find(
        target => entry.name === `@gdfmt/${target.id}`,
      )
      assert.ok(target)
      assert.deepEqual(packed.os, [target.os])
      assert.deepEqual(packed.cpu, [target.cpu])
      assert.deepEqual(packed.libc, target.libc ? [target.libc] : undefined)
    }
    return { ...entry, integrity: actualIntegrity }
  })
  const rootHash = createHash('sha512')
    .update(readFileSync(join(directory, `gdfmt-${version}.tgz`)))
    .digest('hex')
  for (const target of targets) {
    for (const node of ['v22.13.0', 'v24.0.0']) {
      const matches = receipts.filter(
        receipt => receipt.target === target.id && receipt.node === node,
      )
      assert.equal(
        matches.length,
        1,
        `missing or duplicate receipt: ${target.id}/${node}`,
      )
      assert.equal(matches[0].rootSha512, rootHash)
      if (node === 'v24.0.0')
        assert.equal(
          matches[0].nativeSha512,
          createHash('sha512')
            .update(
              readFileSync(
                join(directory, `gdfmt-${target.id}-${version}.tgz`),
              ),
            )
            .digest('hex'),
        )
    }
  }
  return { schemaVersion: 1, version, revision, channel, packages }
}
