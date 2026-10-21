import assert from 'node:assert/strict'
import { execFileSync } from 'node:child_process'
import { mkdtempSync, rmSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'
import { runNpm } from '../npm/cli.ts'
import { releaseDirectory, validateFrozenRelease } from './artifacts.ts'
import type { ReleasePackage } from './artifacts.ts'
import { assertNpmRuntime } from './auth.ts'
import {
  assertCleanReleaseSource,
  compareVersions,
  publicNpmPackage,
  readObject,
} from './project.ts'
import {
  assertNpmIntegrity,
  confirmRegistryWrite,
  npmRegistry,
  npmVersion,
  registryJson,
  waitForNpm,
  waitForNpmChannel,
} from './registry.ts'

export interface NpmOperations {
  version: typeof npmVersion
  package: (name: string) => Promise<Record<string, unknown> | undefined>
  publish: (entry: ReleasePackage, directory: string) => void
  promote: (name: string, version: string, channel: string) => void
  wait: typeof waitForNpm
  waitChannel: typeof waitForNpmChannel
  verifyConsumer: (version: string) => void
}

const npmOperations: NpmOperations = {
  version: npmVersion,
  package: name => registryJson(`${npmRegistry}/${encodeURIComponent(name)}`),
  publish: (entry, directory) => {
    runNpm(
      [
        'publish',
        join(directory, entry.file),
        '--access',
        'public',
        '--tag',
        'candidate',
        '--ignore-scripts',
        '--registry',
        npmRegistry,
      ],
      { stdio: 'inherit' },
    )
  },
  promote: (name, version, channel) => {
    runNpm(
      [
        'dist-tag',
        'add',
        `${name}@${version}`,
        channel,
        '--registry',
        npmRegistry,
      ],
      { stdio: 'inherit' },
    )
  },
  wait: waitForNpm,
  waitChannel: waitForNpmChannel,
  verifyConsumer: version => {
    const directory = mkdtempSync(join(tmpdir(), 'gdfmt-registry-consumer-'))
    try {
      writeFileSync(
        join(directory, 'package.json'),
        '{"private":true,"type":"module"}\n',
      )
      runNpm(
        [
          'install',
          `${publicNpmPackage}@${version}`,
          '--ignore-scripts',
          '--no-audit',
          '--no-fund',
          '--cache',
          join(directory, '.cache'),
          '--registry',
          npmRegistry,
        ],
        { cwd: directory, stdio: 'inherit' },
      )
      execFileSync(
        process.execPath,
        [
          '--input-type=module',
          '-e',
          `import assert from "node:assert/strict"; import { format } from ${JSON.stringify(publicNpmPackage)}; const result = await format("var value=1\\n"); assert.equal(result.output, "var value = 1\\n"); assert.deepEqual(result.diagnostics, [])`,
        ],
        { cwd: directory, stdio: 'inherit' },
      )
      const output = execFileSync(
        process.execPath,
        [join('node_modules', publicNpmPackage, 'dist/cli.cjs'), '--version'],
        { cwd: directory, encoding: 'utf8' },
      )
      assert.equal(output.trim(), `gdfmt ${version}`)
    } finally {
      rmSync(directory, { recursive: true, force: true })
    }
  },
}

/**
 * Publishes exact tested bytes. The default is a read-only registry plan.
 */
export async function publishNpm(
  directory: string,
  execute: boolean,
  operations: NpmOperations = npmOperations,
): Promise<void> {
  if (execute && operations === npmOperations) {
    assertCleanReleaseSource()
    assertNpmRuntime()
  }
  const release = validateFrozenRelease(directory)
  const pending: string[] = []
  // Complete the existing-version checks before the first write.
  for (const entry of release.packages) {
    const current = await operations.version(entry.name, release.version)
    if (current) assertNpmIntegrity(current, entry.integrity)
    else pending.push(entry.name)
  }
  const currentChannel = async (name: string): Promise<string | undefined> => {
    const manifest = await operations.package(name)
    if (!manifest) return undefined
    const current = readObject(manifest['dist-tags'])[release.channel]
    assert.ok(
      current === undefined || typeof current === 'string',
      'invalid registry channel',
    )
    if (current !== undefined)
      assert.ok(
        compareVersions(current, release.version) <= 0,
        `refusing to move ${name} release channel backwards`,
      )
    return current
  }
  for (const entry of release.packages) await currentChannel(entry.name)
  console.log(
    `${execute ? 'Publishing' : 'Would publish'} ${pending.join(', ') || 'no missing packages'} at ${release.version}; channel ${release.channel}.`,
  )
  if (!execute) {
    console.log('Read-only plan. Pass --execute to publish.')
    return
  }
  const platforms = release.packages.slice(0, -1)
  const root = release.packages.at(-1)
  assert.ok(root && root.name === publicNpmPackage)
  const upload = async (entry: ReleasePackage): Promise<void> => {
    if (!pending.includes(entry.name)) return
    try {
      operations.publish(entry, directory)
    } catch (error) {
      // An ambiguous upload must be reconciled before submitting more writes.
      await confirmRegistryWrite(
        () => {
          throw error
        },
        () => operations.wait(entry.name, release.version, entry.integrity),
      )
    }
  }
  // npm processes accepted uploads asynchronously. Submit platform candidates
  // together, then require every platform to be visible before uploading root.
  for (const entry of platforms) await upload(entry)
  for (const entry of platforms)
    await operations.wait(entry.name, release.version, entry.integrity)
  await upload(root)
  await operations.wait(root.name, release.version, root.integrity)
  operations.verifyConsumer(release.version)
  // Recheck every platform after uploads, before the first channel write.
  for (const entry of release.packages) await currentChannel(entry.name)
  // Promotion starts only after all six immutable versions have been verified.
  for (const entry of release.packages) {
    if ((await currentChannel(entry.name)) === release.version) continue
    await confirmRegistryWrite(
      () => operations.promote(entry.name, release.version, release.channel),
      () =>
        operations.waitChannel(entry.name, release.channel, release.version),
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
    'usage: pnpm release:publish:npm [--execute]',
  )
  await publishNpm(releaseDirectory, args.includes('--execute'))
}
