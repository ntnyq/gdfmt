import assert from 'node:assert/strict'
import { execFileSync } from 'node:child_process'
import { mkdtempSync, rmSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import { test } from 'node:test'
import { resolveNpmCli } from '../../scripts/npm/cli.ts'
import { validateNpmRuntime } from '../../scripts/release/auth.ts'
import {
  assertCleanReleaseSource,
  isCleanReleaseSource,
} from '../../scripts/release/project.ts'

test('OIDC validates both CLI capabilities and the actual GitHub id-token environment', () => {
  validateNpmRuntime('24.0.0', '11.21.0', {})
  for (const version of ['10.9.0', '11.20.0', '11.21.1', '12.2.0', '13.0.0'])
    assert.throws(
      () => validateNpmRuntime('24.0.0', version, {}),
      /pinned npm 11\.21\.0/,
    )
  assert.throws(() => validateNpmRuntime('22.13.0', '11.21.0', {}), /Node/)
  assert.throws(
    () => validateNpmRuntime('24.0.0', '11.21.0', { GITHUB_ACTIONS: 'true' }),
    /id-token: write/,
  )
  validateNpmRuntime('24.0.0', '11.21.0', {
    GITHUB_ACTIONS: 'true',
    ACTIONS_ID_TOKEN_REQUEST_URL: 'https://example.invalid/oidc',
    ACTIONS_ID_TOKEN_REQUEST_TOKEN: 'synthetic fixture',
  })
  assert.ok(
    resolveNpmCli('/runtime/bin/node', 'linux', '').endsWith(
      '/runtime/lib/node_modules/npm/bin/npm-cli.js',
    ),
  )
  assert.ok(
    resolveNpmCli('/runtime/node.exe', 'win32', '').endsWith(
      '/runtime/node_modules/npm/bin/npm-cli.js',
    ),
  )
})

test('real publication rejects unstaged, staged and untracked source changes', () => {
  const root = mkdtempSync(join(tmpdir(), 'gdfmt-release-source-'))
  const git = (args: string[]): void => {
    execFileSync('git', args, { cwd: root, stdio: 'ignore' })
  }
  try {
    git(['init', '-q'])
    writeFileSync(join(root, 'source'), 'clean')
    git(['add', '.'])
    git([
      '-c',
      'user.name=Release test',
      '-c',
      'user.email=test@example.invalid',
      'commit',
      '-qm',
      'Fixture',
    ])
    assertCleanReleaseSource(root)
    writeFileSync(join(root, 'source'), 'modified')
    assert.equal(isCleanReleaseSource(root), false)
    assert.throws(() => assertCleanReleaseSource(root), /clean checkout/)
    git(['add', 'source'])
    assert.throws(() => assertCleanReleaseSource(root), /clean checkout/)
    git(['restore', '--staged', 'source'])
    git(['restore', 'source'])
    writeFileSync(join(root, 'untracked'), 'modified')
    assert.throws(() => assertCleanReleaseSource(root), /clean checkout/)
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
})
