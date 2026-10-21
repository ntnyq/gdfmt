import assert from 'node:assert/strict'
import { execFileSync } from 'node:child_process'
import { createHash } from 'node:crypto'
import { readFileSync } from 'node:fs'
import { copyFile, mkdir, writeFile } from 'node:fs/promises'
import { join } from 'node:path'
import {
  checkRelease,
  gitRevision,
  publicCrates,
  readJson,
  readObject,
  repositoryRoot,
} from './project.ts'

export interface FrozenCrate {
  name: string
  file: string
  checksum: string
}

export interface FrozenCrates {
  version: string
  revision: string
  packages: FrozenCrate[]
}

export function crateChecksum(path: string): string {
  return createHash('sha256').update(readFileSync(path)).digest('hex')
}

function validateCrateSource(
  path: string,
  name: string,
  version: string,
  revision: string,
): void {
  const source = readObject(
    JSON.parse(
      execFileSync(
        'tar',
        ['-xOzf', path, `${name}-${version}/.cargo_vcs_info.json`],
        { encoding: 'utf8' },
      ),
    ),
  )
  const git = readObject(source.git)
  assert.equal(git.sha1, revision, `${name} archive belongs to another commit`)
  assert.equal(
    git.dirty ?? false,
    false,
    `${name} archive was built from dirty source`,
  )
  assert.equal(
    source.path_in_vcs,
    `crates/${name}`,
    `${name} archive has an unexpected source path`,
  )
}

/**
 * Bind every frozen Rust archive to its clean source commit, name and checksum.
 */
export async function prepareCrates(
  source: string,
  destination: string,
  root: string = repositoryRoot,
): Promise<void> {
  const version = checkRelease(root)
  const revision = gitRevision(root)
  await mkdir(destination, { recursive: true })
  const packages: FrozenCrate[] = []
  for (const name of publicCrates) {
    const file = `${name}-${version}.crate`
    validateCrateSource(join(source, file), name, version, revision)
    await copyFile(join(source, file), join(destination, file))
    packages.push({
      name,
      file,
      checksum: crateChecksum(join(destination, file)),
    })
  }
  await writeFile(
    join(destination, 'crates.json'),
    `${JSON.stringify({ schemaVersion: 1, version, revision, packages }, null, 2)}\n`,
  )
  validateFrozenCrates(destination, root)
}

export function validateFrozenCrates(
  directory: string,
  root: string = repositoryRoot,
): FrozenCrates {
  const version = checkRelease(root)
  const revision = gitRevision(root)
  const manifest = readJson(join(directory, 'crates.json'))
  assert.equal(manifest.schemaVersion, 1)
  assert.equal(manifest.version, version)
  assert.equal(
    manifest.revision,
    revision,
    'frozen crates belong to another commit',
  )
  assert.ok(Array.isArray(manifest.packages))
  const entries = manifest.packages.map(readObject)
  const expected = publicCrates.map(name => ({
    name,
    file: `${name}-${version}.crate`,
  }))
  assert.deepEqual(
    entries.map(entry => ({ name: entry.name, file: entry.file })),
    expected,
    'frozen crate allowlist differs',
  )
  const packages = expected.map((entry, index) => {
    const path = join(directory, entry.file)
    const checksum = crateChecksum(path)
    assert.equal(
      entries[index].checksum,
      checksum,
      `frozen crate changed: ${entry.name}`,
    )
    validateCrateSource(path, entry.name, version, revision)
    return { ...entry, checksum }
  })
  return { version, revision, packages }
}
