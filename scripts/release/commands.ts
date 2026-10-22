import assert from 'node:assert/strict'
import { execFile } from 'node:child_process'
import { promisify } from 'node:util'
import { resolveNpmCli } from '../npm/cli.ts'
import { logReleaseProgress } from './progress.ts'

const execFileAsync = promisify(execFile)
const NPM_COMMAND_TIMEOUT_MS = 180_000

export interface ReleaseCommandOptions {
  label: string
  cwd?: string
  timeoutMs?: number
  heartbeatMs?: number
  captureOutput?: boolean
}

/**
 * Bounds each child process, reports liveness, and preserves captured npm stderr.
 */
export async function runReleaseCommand(
  executable: string,
  args: string[],
  options: ReleaseCommandOptions,
): Promise<string> {
  const {
    label,
    cwd,
    captureOutput = false,
    timeoutMs = NPM_COMMAND_TIMEOUT_MS,
    heartbeatMs = 15_000,
  } = options
  assert.ok(
    Number.isSafeInteger(timeoutMs) && timeoutMs > 0,
    'expected a positive command timeout',
  )
  assert.ok(
    Number.isSafeInteger(heartbeatMs) && heartbeatMs > 0,
    'expected a positive heartbeat interval',
  )
  const started = Date.now()
  const controller = new AbortController()
  const timeout = setTimeout(() => controller.abort(), timeoutMs)
  const heartbeat = setInterval(() => {
    logReleaseProgress(
      `Still running ${label} (${Math.round((Date.now() - started) / 1000)}s elapsed; limit ${timeoutMs}ms)`,
    )
  }, heartbeatMs)
  logReleaseProgress(`Starting ${label} (timeout ${timeoutMs}ms)`)
  try {
    const command = execFileAsync(executable, args, {
      cwd,
      encoding: 'utf8',
      signal: controller.signal,
      killSignal: 'SIGKILL',
      maxBuffer: 16 * 1024 * 1024,
    })
    if (!captureOutput) {
      command.child.stdout?.pipe(process.stdout, { end: false })
      command.child.stderr?.pipe(process.stderr, { end: false })
    }
    const { stdout } = await command
    logReleaseProgress(`Completed ${label} (${Date.now() - started}ms)`)
    return stdout
  } catch (error) {
    logReleaseProgress(`Failed ${label} (${Date.now() - started}ms)`)
    if (controller.signal.aborted) {
      throw new Error(
        `${label} timed out after ${timeoutMs}ms; confirm registry state before retrying a write`,
        { cause: error },
      )
    }
    throw error
  } finally {
    clearTimeout(timeout)
    clearInterval(heartbeat)
  }
}

/**
 * Uses the active Node runtime and disables npm's hidden HTTP write retries.
 */
export async function runReleaseNpm(
  args: string[],
  options: ReleaseCommandOptions,
): Promise<string> {
  return runReleaseCommand(
    process.execPath,
    [resolveNpmCli(), ...args, '--fetch-retries=0', '--fetch-timeout=30000'],
    options,
  )
}
