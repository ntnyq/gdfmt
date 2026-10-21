import assert from 'node:assert/strict'
import { execFileSync } from 'node:child_process'
import {
  mkdirSync,
  mkdtempSync,
  readFileSync,
  rmSync,
  writeFileSync,
} from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import { test } from 'node:test'
import {
  crateChecksum,
  prepareCrates,
  validateFrozenCrates,
} from '../../scripts/release/crate-artifacts.ts'
import {
  checkRelease,
  gitRevision,
  publicCrates,
} from '../../scripts/release/project.ts'
import { publishCrates } from '../../scripts/release/publish-crates.ts'
import type { CrateOperations } from '../../scripts/release/publish-crates.ts'

test('crates publishing checks all immutable versions, resumes in dependency order and defaults to a plan', async () => {
  const directory = mkdtempSync(join(tmpdir(), 'gdfmt-crates-'))
  try {
    const version = checkRelease()
    const writes: string[] = []
    const existing = new Map<string, Record<string, unknown>>()
    const source = join(directory, 'source')
    const staging = join(directory, 'staging')
    mkdirSync(source)
    for (const name of publicCrates) {
      const archiveRoot = `${name}-${version}`
      mkdirSync(join(staging, archiveRoot), { recursive: true })
      writeFileSync(
        join(staging, archiveRoot, '.cargo_vcs_info.json'),
        JSON.stringify({
          git: { sha1: gitRevision(), dirty: false },
          path_in_vcs: `crates/${name}`,
        }),
      )
      execFileSync('tar', [
        '-czf',
        join(source, `${archiveRoot}.crate`),
        '-C',
        staging,
        archiveRoot,
      ])
    }
    await prepareCrates(source, directory)
    const hashes = new Map(
      publicCrates.map(name => [
        name,
        crateChecksum(join(directory, `${name}-${version}.crate`)),
      ]),
    )
    const operations: CrateOperations = {
      version: async name => existing.get(name),
      package: name => {
        const hash = hashes.get(name)
        assert.ok(hash)
        return hash
      },
      publish: name => {
        writes.push(name)
        existing.set(name, {
          version: { checksum: hashes.get(name), yanked: false },
        })
      },
      wait: async () => {},
    }
    await publishCrates(directory, false, operations)
    assert.equal(writes.length, 0)
    existing.set('gdfmt_cli', {
      version: { checksum: 'conflict', yanked: false },
    })
    await assert.rejects(
      () => publishCrates(directory, true, operations),
      /registry checksum differs/,
    )
    assert.equal(writes.length, 0)
    existing.clear()
    existing.set(publicCrates[0], {
      version: { checksum: hashes.get(publicCrates[0]), yanked: true },
    })
    await assert.rejects(
      () => publishCrates(directory, true, operations),
      /is yanked/,
    )
    assert.equal(writes.length, 0)
    existing.clear()
    existing.set(publicCrates[0], {
      version: { checksum: hashes.get(publicCrates[0]), yanked: false },
    })
    await publishCrates(directory, true, operations)
    assert.deepEqual(writes, publicCrates.slice(1))
    writes.length = 0
    await publishCrates(directory, true, operations)
    assert.equal(writes.length, 0)
    existing.clear()
    await assert.rejects(
      () =>
        publishCrates(directory, true, {
          ...operations,
          package: () => 'changed bytes',
        }),
      /repackaged bytes differ/,
    )
    assert.equal(writes.length, 0)
    const manifestPath = join(directory, 'crates.json')
    const manifest = readFileSync(manifestPath, 'utf8')
    writeFileSync(manifestPath, manifest.replace(gitRevision(), '0'.repeat(40)))
    assert.throws(() => validateFrozenCrates(directory), /another commit/)
    writeFileSync(manifestPath, manifest)
    writeFileSync(
      join(directory, `${publicCrates[0]}-${version}.crate`),
      'changed archive',
    )
    assert.throws(() => validateFrozenCrates(directory), /frozen crate changed/)
  } finally {
    rmSync(directory, { recursive: true, force: true })
  }
})
