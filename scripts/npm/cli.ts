import { execFileSync } from 'node:child_process'
import type { ExecFileSyncOptions } from 'node:child_process'
import { dirname, join, resolve } from 'node:path'

/**
 * Invoke npm's JavaScript CLI with the active Node runtime, including on Windows.
 */
export function resolveNpmCli(
  node: string = process.execPath,
  platform: NodeJS.Platform = process.platform,
  override: string | undefined = process.env.GDFMT_NPM_CLI,
): string {
  if (override) return resolve(override)
  return platform === 'win32'
    ? join(dirname(node), 'node_modules/npm/bin/npm-cli.js')
    : join(resolve(node, '../..'), 'lib/node_modules/npm/bin/npm-cli.js')
}

export function runNpm(
  args: string[],
  options: ExecFileSyncOptions = {},
): string {
  return (
    execFileSync(process.execPath, [resolveNpmCli(), ...args], {
      ...options,
      encoding: 'utf8',
    }) || ''
  )
}
