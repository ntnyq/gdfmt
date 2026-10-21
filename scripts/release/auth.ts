import assert from 'node:assert/strict'
import { runNpm } from '../npm/cli.ts'
import { compareVersions } from './project.ts'

/**
 * OIDC dist-tags need npm 11.21+ or 12.2+, independently of publish permission.
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
  assert.ok(
    npm.startsWith('11.')
      ? compareVersions(npm, '11.21.0') >= 0
      : compareVersions(npm, '12.2.0') >= 0,
    'release publication requires npm 11.21+ or 12.2+ for OIDC dist-tags',
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
    runNpm(['--version']).trim(),
    process.env,
  )
}
