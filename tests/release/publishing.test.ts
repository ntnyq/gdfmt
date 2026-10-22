import assert from 'node:assert/strict'
import { execFileSync } from 'node:child_process'
import { createHash } from 'node:crypto'
import {
  copyFileSync,
  mkdirSync,
  mkdtempSync,
  readFileSync,
  rmSync,
  writeFileSync,
} from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import { test } from 'node:test'
import { metadata, targets } from '../../scripts/npm/build.ts'
import { verifyArtifacts } from '../../scripts/npm/verify-artifacts.ts'
import {
  prepareRelease,
  validateFrozenRelease,
} from '../../scripts/release/artifacts.ts'
import {
  gitRevision,
  readObject,
  repositoryRoot,
} from '../../scripts/release/project.ts'
import { publishNpm } from '../../scripts/release/publish-npm.ts'
import type { NpmOperations } from '../../scripts/release/publish-npm.ts'
import { registryJson } from '../../scripts/release/registry.ts'

function writeJson(path: string, value: unknown): void {
  writeFileSync(path, `${JSON.stringify(value, null, 2)}\n`)
}

/**
 * Synthetic archives exercise release guards, never native code or a real registry.
 */
function fixture(): { directory: string; matrix: string; frozen: string } {
  const directory = mkdtempSync(join(tmpdir(), 'gdfmt-artifacts-'))
  const matrix = join(directory, 'matrix')
  const frozen = join(directory, 'frozen')
  const staging = join(directory, 'staging')
  mkdirSync(join(staging, 'package'), { recursive: true })
  const root = join(directory, `gdfmt-${metadata.version}.tgz`)
  writeJson(join(staging, 'package/package.json'), metadata)
  execFileSync('tar', ['-czf', root, '-C', staging, 'package'])
  for (const target of targets) {
    for (const node of ['22.13.0', '24.0.0']) {
      const destination = join(matrix, `npm-${target.id}-${node}`)
      mkdirSync(destination, { recursive: true })
      copyFileSync(root, join(destination, `gdfmt-${metadata.version}.tgz`))
      const native = join(
        destination,
        `gdfmt-${target.id}-${metadata.version}.tgz`,
      )
      writeJson(join(staging, 'package/package.json'), {
        name: `@gdfmt/${target.id}`,
        version: metadata.version,
        os: [target.os],
        cpu: [target.cpu],
        ...(target.libc ? { libc: [target.libc] } : {}),
      })
      writeFileSync(
        join(staging, 'package/gdfmt.node'),
        `synthetic fixture for ${target.id}/${node}`,
      )
      execFileSync('tar', ['-czf', native, '-C', staging, 'package'])
      writeJson(join(destination, 'acceptance.json'), {
        revision: gitRevision(),
        cleanSource: true,
        passed: true,
        published: false,
        version: metadata.version,
        target: target.id,
        triple: target.triple,
        node: `v${node}`,
        tests: 8,
        rootSha512: createHash('sha512')
          .update(readFileSync(root))
          .digest('hex'),
        nativeSha512: createHash('sha512')
          .update(readFileSync(native))
          .digest('hex'),
      })
    }
  }
  return { directory, matrix, frozen }
}

function fakePublisher(frozen: string): {
  operations: NpmOperations
  writes: string[]
  versions: Map<string, Record<string, unknown>>
  channels: Map<string, string>
} {
  const release = validateFrozenRelease(frozen)
  const writes: string[] = []
  const versions = new Map<string, Record<string, unknown>>()
  const channels = new Map<string, string>()
  const operations: NpmOperations = {
    version: async name => versions.get(name),
    package: async name => ({
      'dist-tags': { [release.channel]: channels.get(name) },
    }),
    publish: entry => {
      writes.push(`publish:${entry.name}`)
      versions.set(entry.name, { dist: { integrity: entry.integrity } })
    },
    wait: async (name, _version, integrity) => {
      assert.equal(readObject(versions.get(name)?.dist).integrity, integrity)
      writes.push(`verified:${name}`)
    },
    promote: (name, version) => {
      writes.push(`promote:${name}`)
      channels.set(name, version)
    },
    waitChannel: async (name, _channel, version) => {
      assert.equal(channels.get(name), version)
    },
    verifyConsumer: () => {
      writes.push('consumer')
    },
  }
  return { operations, writes, versions, channels }
}

test('complete matrices freeze exactly six packages and reject missing, changed and foreign receipts', async () => {
  const files = fixture()
  try {
    await prepareRelease(files.matrix, files.frozen)
    const release = validateFrozenRelease(files.frozen)
    assert.equal(release.packages.length, 6)
    const receiptPath = join(
      files.matrix,
      `npm-${targets[0].id}-22.13.0/acceptance.json`,
    )
    const original = readFileSync(receiptPath, 'utf8')
    const receipt = readObject(JSON.parse(original))
    writeJson(receiptPath, { ...receipt, revision: '0'.repeat(40) })
    await assert.rejects(() => verifyArtifacts(files.matrix), /release commit/)
    writeFileSync(receiptPath, original)
    writeJson(receiptPath, { ...receipt, cleanSource: false })
    await assert.rejects(() => verifyArtifacts(files.matrix), /clean source/)
    writeFileSync(receiptPath, original)
    rmSync(receiptPath)
    await assert.rejects(
      () => prepareRelease(files.matrix, files.frozen),
      /ENOENT/,
    )
    writeFileSync(join(files.frozen, release.packages[0].file), 'changed bytes')
    assert.throws(
      () => validateFrozenRelease(files.frozen),
      /frozen tarball changed/,
    )
  } finally {
    rmSync(files.directory, { recursive: true, force: true })
  }
})

test('recovery validates artifacts against the explicit clean source commit rather than the tooling commit', async () => {
  const files = fixture()
  const source = join(files.directory, 'source')
  try {
    await prepareRelease(files.matrix, files.frozen)
    execFileSync('git', [
      'clone',
      '--shared',
      '--quiet',
      repositoryRoot,
      source,
    ])
    execFileSync(
      'git',
      [
        '-c',
        'user.name=Release test',
        '-c',
        'user.email=test@example.invalid',
        'commit',
        '--allow-empty',
        '-qm',
        'Frozen source fixture',
      ],
      { cwd: source },
    )
    const manifestPath = join(files.frozen, 'release.json')
    const manifest = readObject(JSON.parse(readFileSync(manifestPath, 'utf8')))
    const revision = gitRevision(source)
    writeJson(manifestPath, {
      ...manifest,
      revision,
      matrix: { ...readObject(manifest.matrix), revision },
    })
    assert.equal(validateFrozenRelease(files.frozen, source).revision, revision)
    assert.throws(() => validateFrozenRelease(files.frozen))
    writeFileSync(join(source, 'untracked'), 'dirty source')
    await assert.rejects(
      () => publishNpm(files.frozen, true, undefined, source),
      /clean checkout/,
    )
  } finally {
    rmSync(files.directory, { recursive: true, force: true })
  }
})

test('npm plans never write, publishes platforms first, and only promotes a complete set', async () => {
  const files = fixture()
  try {
    await prepareRelease(files.matrix, files.frozen)
    const fake = fakePublisher(files.frozen)
    await publishNpm(files.frozen, false, fake.operations)
    assert.equal(fake.writes.length, 0)
    await publishNpm(files.frozen, true, fake.operations)
    const publishes = fake.writes.filter(value => value.startsWith('publish:'))
    assert.equal(publishes.length, 6)
    assert.equal(publishes.at(-1), `publish:${metadata.name}`)
    assert.ok(
      fake.writes.indexOf('promote:@gdfmt/darwin-arm64') >
        fake.writes.indexOf('consumer'),
    )
    assert.equal(fake.writes.at(-1), `promote:${metadata.name}`)
    fake.writes.length = 0
    await publishNpm(files.frozen, true, fake.operations)
    assert.equal(
      fake.writes.some(value => value.startsWith('publish:')),
      false,
    )
  } finally {
    rmSync(files.directory, { recursive: true, force: true })
  }
})

test('npm integrity conflicts, newer channels and platform failures prevent root publication', async () => {
  const files = fixture()
  try {
    await prepareRelease(files.matrix, files.frozen)
    const fake = fakePublisher(files.frozen)
    fake.versions.set(metadata.name, { dist: { integrity: 'sha512-conflict' } })
    await assert.rejects(
      () => publishNpm(files.frozen, true, fake.operations),
      /registry integrity differs/,
    )
    assert.equal(fake.writes.length, 0)
    fake.versions.clear()
    await assert.rejects(
      () =>
        publishNpm(files.frozen, true, {
          ...fake.operations,
          package: async () => ({
            'dist-tags': { next: '999.0.0', latest: '999.0.0' },
          }),
        }),
      /backwards/,
    )
    assert.equal(fake.writes.length, 0)
    await assert.rejects(
      () =>
        publishNpm(files.frozen, true, {
          ...fake.operations,
          wait: async () => {
            throw new Error('platform unavailable')
          },
        }),
      /platform unavailable/,
    )
    assert.equal(fake.writes.includes(`publish:${metadata.name}`), false)
    assert.equal(
      fake.writes.some(value => value.startsWith('promote:')),
      false,
    )
    fake.writes.length = 0
    fake.versions.clear()
    await assert.rejects(
      () =>
        publishNpm(files.frozen, true, {
          ...fake.operations,
          verifyConsumer: async () => {
            throw new Error('consumer failed')
          },
        }),
      /consumer failed/,
    )
    assert.equal(
      fake.writes.some(value => value.startsWith('promote:')),
      false,
    )
  } finally {
    rmSync(files.directory, { recursive: true, force: true })
  }
})

test('only registry 404 responses are treated as a missing version', async () => {
  const response =
    (status: number): typeof fetch =>
    async () =>
      new Response('{}', { status })
  assert.equal(
    await registryJson('https://example.invalid/package', response(404)),
    undefined,
  )
  for (const status of [401, 403, 429, 500])
    await assert.rejects(
      () =>
        registryJson(
          'https://example.invalid/package',
          response(status),
          async () => {},
        ),
      /registry request failed/,
    )
})

test('a newer platform channel or channel changes during uploads cannot be rolled back', async () => {
  const files = fixture()
  try {
    await prepareRelease(files.matrix, files.frozen)
    const fake = fakePublisher(files.frozen)
    fake.channels.set('@gdfmt/darwin-arm64', '999.0.0')
    await assert.rejects(
      () => publishNpm(files.frozen, true, fake.operations),
      /backwards/,
    )
    assert.equal(fake.writes.length, 0)
    fake.channels.clear()
    await assert.rejects(
      () =>
        publishNpm(files.frozen, true, {
          ...fake.operations,
          verifyConsumer: () => {
            fake.channels.set('@gdfmt/darwin-arm64', '999.0.0')
          },
        }),
      /backwards/,
    )
    assert.equal(
      fake.writes.some(value => value.startsWith('promote:')),
      false,
    )
  } finally {
    rmSync(files.directory, { recursive: true, force: true })
  }
})

test('npm resumes a partially promoted set without uploading or rewriting completed channels', async () => {
  const files = fixture()
  try {
    await prepareRelease(files.matrix, files.frozen)
    const fake = fakePublisher(files.frozen)
    const failed = '@gdfmt/win32-x64-msvc'
    await assert.rejects(
      () =>
        publishNpm(files.frozen, true, {
          ...fake.operations,
          promote: (name, version, channel) => {
            if (name === failed) throw new Error('temporary promotion failure')
            fake.operations.promote(name, version, channel)
          },
        }),
      AggregateError,
    )
    assert.equal(fake.channels.has(metadata.name), false)
    fake.writes.length = 0
    await publishNpm(files.frozen, true, fake.operations)
    assert.deepEqual(
      fake.writes.filter(value => value.startsWith('promote:')),
      [`promote:${failed}`, `promote:${metadata.name}`],
    )
    assert.equal(
      fake.writes.some(value => value.startsWith('publish:')),
      false,
    )
  } finally {
    rmSync(files.directory, { recursive: true, force: true })
  }
})

test('npm confirms a committed upload even when the publishing CLI reports an error', async () => {
  const files = fixture()
  try {
    await prepareRelease(files.matrix, files.frozen)
    const fake = fakePublisher(files.frozen)
    await publishNpm(files.frozen, true, {
      ...fake.operations,
      publish: (entry, directory) => {
        fake.operations.publish(entry, directory)
        throw new Error('connection lost after upload')
      },
    })
    assert.equal(
      fake.writes.filter(value => value.startsWith('publish:')).length,
      6,
    )
    assert.equal(fake.writes.at(-1), `promote:${metadata.name}`)
  } finally {
    rmSync(files.directory, { recursive: true, force: true })
  }
})

test('asynchronous registry processing overlaps platforms and gates root on visible platforms', async () => {
  const files = fixture()
  try {
    await prepareRelease(files.matrix, files.frozen)
    const fake = fakePublisher(files.frozen)
    const availableAt = new Map<string, number>()
    const verified = new Set<string>()
    let time = 0
    const latency = 180_000
    await publishNpm(files.frozen, true, {
      ...fake.operations,
      publish: entry => {
        if (entry.name === metadata.name) assert.equal(verified.size, 5)
        availableAt.set(entry.name, time + latency)
        fake.operations.publish(entry, files.frozen)
      },
      wait: async (name, version, integrity) => {
        const visible = availableAt.get(name)
        assert.ok(visible !== undefined)
        time = Math.max(time, visible)
        await fake.operations.wait(name, version, integrity)
        verified.add(name)
      },
    })
    assert.equal(time, 2 * latency)
    assert.equal(verified.size, 6)
  } finally {
    rmSync(files.directory, { recursive: true, force: true })
  }
})
