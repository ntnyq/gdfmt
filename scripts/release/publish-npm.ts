import assert from 'node:assert/strict'
import { mkdtempSync, rmSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'
import { parseArgs } from 'node:util'
import { releaseDirectory, validateFrozenRelease } from './artifacts.ts'
import type { ReleasePackage } from './artifacts.ts'
import { assertNpmRuntime } from './auth.ts'
import { runReleaseCommand, runReleaseNpm } from './commands.ts'
import { logReleaseProgress } from './progress.ts'
import {
  assertCleanReleaseSource,
  compareVersions,
  publicNpmPackage,
  readObject,
  repositoryRoot,
} from './project.ts'
import {
  assertNpmIntegrity,
  confirmRegistryWrite,
  npmRegistry,
  npmVersion,
  registryJson,
  waitForNpm,
  waitForNpmInstall,
  waitForNpmChannel,
} from './registry.ts'

export interface NpmOperations {
  version: typeof npmVersion
  package: (name: string) => Promise<Record<string, unknown> | undefined>
  publish: (entry: ReleasePackage, directory: string) => void | Promise<void>
  promote: (
    name: string,
    version: string,
    channel: string,
  ) => void | Promise<void>
  wait: typeof waitForNpm
  waitChannel: typeof waitForNpmChannel
  verifyConsumer: (version: string) => void | Promise<void>
}

const npmOperations: NpmOperations = {
  version: npmVersion,
  package: name => registryJson(`${npmRegistry}/${encodeURIComponent(name)}`),
  publish: async (entry, directory) => {
    await runReleaseNpm(
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
      { label: `npm publish ${entry.name}` },
    )
  },
  promote: async (name, version, channel) => {
    await runReleaseNpm(
      [
        'dist-tag',
        'add',
        `${name}@${version}`,
        channel,
        '--registry',
        npmRegistry,
      ],
      { label: `npm dist-tag ${name} ${channel} -> ${version}` },
    )
  },
  wait: waitForNpm,
  waitChannel: waitForNpmChannel,
  verifyConsumer: async version => {
    const directory = mkdtempSync(join(tmpdir(), 'gdfmt-registry-consumer-'))
    try {
      writeFileSync(
        join(directory, 'package.json'),
        '{"private":true,"type":"module"}\n',
      )
      await waitForNpmInstall(async remainingMs => {
        await runReleaseNpm(
          [
            'install',
            `${publicNpmPackage}@${version}`,
            '--prefer-online',
            '--ignore-scripts',
            '--no-audit',
            '--no-fund',
            '--cache',
            join(directory, '.cache'),
            '--registry',
            npmRegistry,
          ],
          {
            label: `npm install ${publicNpmPackage}@${version}`,
            cwd: directory,
            captureOutput: true,
            timeoutMs: Math.min(180_000, remainingMs),
          },
        )
      })
      await runReleaseCommand(
        process.execPath,
        [
          '--input-type=module',
          '-e',
          `import assert from "node:assert/strict"; import { format } from ${JSON.stringify(publicNpmPackage)}; const result = await format("var value=1\\n"); assert.equal(result.output, "var value = 1\\n"); assert.deepEqual(result.diagnostics, [])`,
        ],
        {
          label: 'registry-installed Node API smoke test',
          cwd: directory,
          timeoutMs: 30_000,
        },
      )
      const output = await runReleaseCommand(
        process.execPath,
        [join('node_modules', publicNpmPackage, 'dist/cli.cjs'), '--version'],
        {
          label: 'registry-installed CLI version check',
          cwd: directory,
          captureOutput: true,
          timeoutMs: 30_000,
        },
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
  sourceRoot: string = repositoryRoot,
): Promise<void> {
  if (execute && operations === npmOperations) {
    assertCleanReleaseSource(sourceRoot)
    assertCleanReleaseSource()
    assertNpmRuntime()
  }
  logReleaseProgress(
    `Validating frozen npm artifacts in ${directory} against ${sourceRoot}`,
  )
  const release = validateFrozenRelease(directory, sourceRoot)
  const pending: string[] = []
  // Complete the existing-version checks before the first write.
  for (const entry of release.packages) {
    logReleaseProgress(
      `Checking immutable version ${entry.name}@${release.version}`,
    )
    const current = await operations.version(entry.name, release.version)
    if (current) {
      assertNpmIntegrity(current, entry.integrity)
      logReleaseProgress(
        `Already published with matching integrity: ${entry.name}`,
      )
    } else pending.push(entry.name)
  }
  const currentChannel = async (name: string): Promise<string | undefined> => {
    const manifest = await operations.package(name)
    if (!manifest) return undefined
    const current = readObject(manifest['dist-tags'])[release.channel]
    logReleaseProgress(`${name} ${release.channel}: ${current ?? 'unset'}`)
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
    if (!pending.includes(entry.name)) {
      logReleaseProgress(`Skipping existing upload: ${entry.name}`)
      return
    }
    try {
      await operations.publish(entry, directory)
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
    if (pending.includes(entry.name))
      await operations.wait(entry.name, release.version, entry.integrity)
  await upload(root)
  if (pending.includes(root.name))
    await operations.wait(root.name, release.version, root.integrity)
  logReleaseProgress(
    'Verifying a fresh registry install before promoting any channels',
  )
  await operations.verifyConsumer(release.version)
  // Recheck every platform after uploads, before the first channel write.
  for (const entry of release.packages) await currentChannel(entry.name)
  // Promotion starts only after all six immutable versions have been verified.
  for (const entry of release.packages) {
    if ((await currentChannel(entry.name)) === release.version) {
      logReleaseProgress(
        `Skipping completed ${release.channel} promotion: ${entry.name}`,
      )
      continue
    }
    await confirmRegistryWrite(
      () => operations.promote(entry.name, release.version, release.channel),
      () =>
        operations.waitChannel(entry.name, release.channel, release.version),
    )
  }
  logReleaseProgress(
    `npm release complete: ${publicNpmPackage}@${release.version} (${release.channel})`,
  )
}

if (
  process.argv[1] &&
  resolve(process.argv[1]) === fileURLToPath(import.meta.url)
) {
  const { values } = parseArgs({
    options: {
      execute: { type: 'boolean', default: false },
      artifacts: { type: 'string' },
      source: { type: 'string' },
    },
  })
  await publishNpm(
    resolve(values.artifacts ?? releaseDirectory),
    values.execute,
    npmOperations,
    resolve(values.source ?? repositoryRoot),
  )
}
