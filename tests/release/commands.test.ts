import assert from 'node:assert/strict'
import { execFileSync } from 'node:child_process'
import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import { test } from 'node:test'
import { fileURLToPath } from 'node:url'
import { runReleaseCommand } from '../../scripts/release/commands.ts'

test('npm launcher canonicalizes scoped GET and PUT paths while retaining the original command and auth options', () => {
  const directory = mkdtempSync(join(tmpdir(), 'gdfmt-npm-launcher-'))
  try {
    mkdirSync(join(directory, 'bin'))
    mkdirSync(join(directory, 'lib/commands'), { recursive: true })
    writeFileSync(
      join(directory, 'lib/commands/dist-tag.js'),
      'module.exports = class { fetchTags(spec, options) { return { originalOptions: options } } }',
    )
    const cli = join(directory, 'bin/npm-cli.cjs')
    writeFileSync(
      cli,
      `
      const DistTag = require('../lib/commands/dist-tag.js')
      const spec = { name: '@scope/package', escapedName: '@scope%2fpackage' }
      const options = { auth: 'synthetic fixture' }
      const tags = new DistTag().fetchTags(spec, options)
      console.log(JSON.stringify({
        read: '/-/package/' + spec.escapedName + '/dist-tags',
        write: '/-/package/' + spec.escapedName + '/dist-tags/next',
        sameAuth: tags.originalOptions === options,
        args: process.argv.slice(2),
      }))
    `,
    )
    const args = ['dist-tag', 'add', '@scope/package@1.0.0', 'next']
    const output = execFileSync(
      process.execPath,
      [
        fileURLToPath(
          new URL('../../scripts/release/npm-cli.mjs', import.meta.url),
        ),
        cli,
        ...args,
      ],
      { encoding: 'utf8' },
    )
    assert.deepEqual(JSON.parse(output), {
      read: '/-/package/@scope%2Fpackage/dist-tags',
      write: '/-/package/@scope%2Fpackage/dist-tags/next',
      sameAuth: true,
      args,
    })
  } finally {
    rmSync(directory, { recursive: true, force: true })
  }
})

test('release commands terminate a hung child and report progress before the deadline', async t => {
  const messages: string[] = []
  t.mock.method(console, 'log', (...args: unknown[]) => {
    messages.push(args.join(' '))
  })
  await assert.rejects(
    () =>
      runReleaseCommand(
        process.execPath,
        ['-e', 'setInterval(() => {}, 1000)'],
        {
          label: 'hung registry command',
          timeoutMs: 500,
          heartbeatMs: 100,
          captureOutput: true,
        },
      ),
    /timed out after 500ms/,
  )
  assert.ok(
    messages.some(message =>
      message.includes('Starting hung registry command'),
    ),
  )
  assert.ok(
    messages.some(message =>
      message.includes('Still running hung registry command'),
    ),
  )
  assert.ok(
    messages.some(message => message.includes('Failed hung registry command')),
  )
})

test('release commands preserve captured npm errors for install retry classification', async () => {
  await assert.rejects(
    () =>
      runReleaseCommand(
        process.execPath,
        ['-e', 'console.error("npm error code ETARGET"); process.exit(1)'],
        {
          label: 'failed registry install',
          captureOutput: true,
        },
      ),
    error =>
      error instanceof Error &&
      'stderr' in error &&
      String(error.stderr).includes('npm error code ETARGET'),
  )
  assert.equal(
    await runReleaseCommand(
      process.execPath,
      ['-e', 'process.stdout.write("installed")'],
      {
        label: 'successful registry install',
        captureOutput: true,
      },
    ),
    'installed',
  )
})
