import assert from 'node:assert/strict'
import { runNpm } from '../npm/cli.ts'
import compatibility from './npm-compat.json' with { type: 'json' }
import { compareVersions } from './project.ts'

/**
 * Rejects unverified npm versions before any write while the internal patch exists.
 */
export function validateNpmRuntime(
  node: string,
  npm: string,
  environment: NodeJS.ProcessEnv,
): void {
  assert.ok(
    compareVersions(node, '22.14.0') >= 0,
    'npm Trusted Publishing requires Node >=22.14.0',
  )
  assert.equal(
    npm,
    compatibility.version,
    `release publication requires pinned npm ${compatibility.version}; review the temporary dist-tag patch before upgrading (RELEASING.md)`,
  )
  if (environment.GITHUB_ACTIONS === 'true') {
    assert.ok(
      environment.ACTIONS_ID_TOKEN_REQUEST_URL &&
        environment.ACTIONS_ID_TOKEN_REQUEST_TOKEN,
      'GitHub OIDC is unavailable: the publishing job needs id-token: write',
    )
  }
}

export function assertNpmRuntime(): void {
  validateNpmRuntime(
    process.versions.node,
    runNpm(['--version'], { timeout: 10_000, killSignal: 'SIGKILL' }).trim(),
    process.env,
  )
}
