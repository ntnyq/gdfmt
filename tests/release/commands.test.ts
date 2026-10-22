import assert from 'node:assert/strict'
import { test } from 'node:test'
import { runReleaseCommand } from '../../scripts/release/commands.ts'

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
