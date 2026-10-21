import assert from 'node:assert/strict'
import { execFileSync } from 'node:child_process'
import { resolve } from 'node:path'
import { setTimeout } from 'node:timers/promises'
import { fileURLToPath } from 'node:url'
import { crateChecksum, validateFrozenCrates } from './crate-artifacts.ts'
import {
  assertCleanReleaseSource,
  readObject,
  repositoryRoot,
} from './project.ts'
import { confirmRegistryWrite, registryJson } from './registry.ts'

export interface CrateOperations {
  version: (
    name: string,
    version: string,
  ) => Promise<Record<string, unknown> | undefined>
  package: (name: string, version: string) => string
  publish: (name: string) => void
  wait: (name: string, version: string, checksum: string) => Promise<void>
}

function assertPublishedCrate(
  current: Record<string, unknown>,
  name: string,
  version: string,
  expected: string,
): void {
  const published = readObject(current.version)
  assert.equal(
    published.checksum,
    expected,
    `registry checksum differs: ${name}; stop and use a new version`,
  )
  assert.equal(
    published.yanked,
    false,
    `${name}@${version} is yanked; use a new version`,
  )
}

const crateOperations: CrateOperations = {
  version: (name, version) =>
    registryJson(`https://crates.io/api/v1/crates/${name}/${version}`),
  package: (name, version) => {
    execFileSync(
      'cargo',
      ['package', '-p', name, '--registry', 'crates-io', '--locked'],
      { cwd: repositoryRoot, stdio: 'inherit' },
    )
    return crateChecksum(
      resolve(repositoryRoot, 'target/package', `${name}-${version}.crate`),
    )
  },
  publish: name => {
    execFileSync(
      'cargo',
      ['publish', '-p', name, '--registry', 'crates-io', '--locked'],
      { cwd: repositoryRoot, stdio: 'inherit' },
    )
  },
  wait: async (name, version, expected) => {
    for (let attempt = 0; attempt < 12; attempt++) {
      const current = await crateOperations.version(name, version)
      if (current) {
        assertPublishedCrate(current, name, version, expected)
        return
      }
      await setTimeout(5_000)
    }
    throw new Error(
      `${name}@${version} is not visible yet; rerun the failed job`,
    )
  },
}

/**
 * Compares all existing immutable versions before publishing missing crates.
 */
export async function publishCrates(
  directory: string,
  execute: boolean,
  operations: CrateOperations = crateOperations,
): Promise<void> {
  if (execute && operations === crateOperations) assertCleanReleaseSource()
  const { version, packages: entries } = validateFrozenCrates(directory)
  const pending = []
  for (const entry of entries) {
    const current = await operations.version(entry.name, version)
    if (current) {
      assertPublishedCrate(current, entry.name, version, entry.checksum)
    } else pending.push(entry.name)
  }
  console.log(
    `${execute ? 'Publishing' : 'Would publish'} ${pending.join(', ') || 'no missing crates'} at ${version}.`,
  )
  if (!execute) {
    console.log('Read-only plan. Pass --execute to publish.')
    return
  }
  for (const entry of entries) {
    if (!pending.includes(entry.name)) continue
    // Cargo repackages from the checked-out tag. Verify it agrees with the frozen
    // workspace archive before allowing an upload.
    assert.equal(
      operations.package(entry.name, version),
      entry.checksum,
      `repackaged bytes differ: ${entry.name}`,
    )
    await confirmRegistryWrite(
      () => operations.publish(entry.name),
      () => operations.wait(entry.name, version, entry.checksum),
    )
  }
}

if (
  process.argv[1] &&
  resolve(process.argv[1]) === fileURLToPath(import.meta.url)
) {
  const args = process.argv.slice(2)
  assert.ok(
    args.every(arg => arg === '--execute'),
    'usage: pnpm release:publish:crates [--execute]',
  )
  await publishCrates(
    resolve('target/release/crates'),
    args.includes('--execute'),
  )
}
