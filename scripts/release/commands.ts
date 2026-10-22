import assert from 'node:assert/strict'
import { spawn } from 'node:child_process'
import { resolveNpmCli } from '../npm/cli.ts'
import { logReleaseProgress } from './progress.ts'

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
  logReleaseProgress(`Starting ${label} (timeout ${timeoutMs}ms)`)
  const child = spawn(executable, args, {
    cwd,
    stdio: captureOutput ? ['ignore', 'pipe', 'pipe'] : 'inherit',
  })
  let hasTimedOut = false
  const timeout = setTimeout(() => {
    hasTimedOut = true
    child.kill('SIGKILL')
  }, timeoutMs)
  const heartbeat = setInterval(() => {
    logReleaseProgress(
      `Still running ${label} (${Math.round((Date.now() - started) / 1000)}s elapsed; limit ${timeoutMs}ms)`,
    )
  }, heartbeatMs)
  try {
    const stdout = await new Promise<string>((resolve, reject) => {
      let output = ''
      let stderr = ''
      let hasExceededOutputLimit = false
      const appendOutput = (
        stream: 'stdout' | 'stderr',
        chunk: string,
      ): void => {
        if (hasExceededOutputLimit) return
        if (stream === 'stdout') output += chunk
        else stderr += chunk
        if (
          Buffer.byteLength(output) + Buffer.byteLength(stderr) >
          16 * 1024 * 1024
        ) {
          hasExceededOutputLimit = true
          child.kill('SIGKILL')
        }
      }
      child.stdout
        ?.setEncoding('utf8')
        .on('data', (chunk: string) => appendOutput('stdout', chunk))
      child.stderr
        ?.setEncoding('utf8')
        .on('data', (chunk: string) => appendOutput('stderr', chunk))
      child.once('error', reject)
      child.once('close', (code, signal) => {
        if (hasExceededOutputLimit) {
          reject(new Error(`${label} exceeded the captured output limit`))
        } else if (code !== 0) {
          reject(
            Object.assign(
              new Error(`${label} exited with ${signal ?? `code ${code}`}`),
              { code, signal, stdout: output, stderr },
            ),
          )
        } else resolve(output)
      })
    })
    logReleaseProgress(`Completed ${label} (${Date.now() - started}ms)`)
    return stdout
  } catch (error) {
    logReleaseProgress(`Failed ${label} (${Date.now() - started}ms)`)
    if (hasTimedOut) {
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
