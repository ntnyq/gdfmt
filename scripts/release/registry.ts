import assert from 'node:assert/strict'
import { setTimeout } from 'node:timers/promises'
import { logReleaseProgress } from './progress.ts'
import { readObject } from './project.ts'

export const npmRegistry: string = 'https://registry.npmjs.org'

/**
 * Only a real 404 means absent; authentication, rate limits and outages fail closed.
 */
export async function registryJson(
  url: string,
  fetcher: typeof fetch = fetch,
  pause: (milliseconds: number) => Promise<void> = setTimeout,
  signal?: AbortSignal,
): Promise<Record<string, unknown> | undefined> {
  for (let attempt = 0; attempt < 4; attempt++) {
    signal?.throwIfAborted()
    const started = Date.now()
    logReleaseProgress(
      `GET ${url} (attempt ${attempt + 1}/4; request timeout 30s)`,
    )
    try {
      const requestTimeout = AbortSignal.timeout(30_000)
      const response = await fetcher(url, {
        headers: {
          'user-agent': 'gdfmt-release (https://github.com/ntnyq/gdfmt)',
          'cache-control': 'no-cache',
        },
        signal: signal
          ? AbortSignal.any([signal, requestTimeout])
          : requestTimeout,
      })
      logReleaseProgress(
        `GET ${url}: HTTP ${response.status} (${Date.now() - started}ms)`,
      )
      if (response.status === 404) {
        await response.body?.cancel()
        return undefined
      }
      if ((response.status === 429 || response.status >= 500) && attempt < 3) {
        const retryAfter = response.headers.get('retry-after')
        const delay =
          retryAfter === null
            ? Number.NaN
            : /^\d+$/.test(retryAfter)
              ? Number(retryAfter) * 1_000
              : Date.parse(retryAfter) - Date.now()
        await response.body?.cancel()
        const retryDelay = Number.isFinite(delay)
          ? Math.max(0, Math.min(delay, 30_000))
          : 1_000 * 2 ** attempt
        logReleaseProgress(
          `Retrying GET ${url} in ${retryDelay}ms after HTTP ${response.status}`,
        )
        await (signal && pause === setTimeout
          ? setTimeout(retryDelay, undefined, { signal })
          : pause(retryDelay))
        continue
      }
      if (!response.ok) await response.body?.cancel()
      assert.ok(
        response.ok,
        `registry request failed (${response.status}): ${url}`,
      )
      return readObject(await response.json())
    } catch (error) {
      logReleaseProgress(
        `GET ${url} failed (${Date.now() - started}ms): ${error instanceof Error ? error.message : String(error)}`,
      )
      if (signal?.aborted)
        throw new Error(`Registry deadline exceeded: ${url}`, { cause: error })
      const transient =
        error instanceof TypeError ||
        (error instanceof Error && error.name === 'TimeoutError')
      if (!transient || attempt === 3) throw error
      const delay = 1_000 * 2 ** attempt
      logReleaseProgress(`Retrying GET ${url} in ${delay}ms`)
      await (signal && pause === setTimeout
        ? setTimeout(delay, undefined, { signal })
        : pause(delay))
      continue
    }
  }
  throw new Error(`registry request exhausted retries: ${url}`)
}

/**
 * A failed upload can already have committed. Confirm bytes before deciding to fail.
 * This never automatically repeats a registry write.
 */
export async function confirmRegistryWrite(
  write: () => void | Promise<void>,
  confirm: () => Promise<void>,
): Promise<void> {
  let failed = false
  let writeError: unknown
  try {
    await write()
  } catch (error) {
    failed = true
    writeError = error
    logReleaseProgress(
      'Registry write failed; confirming its outcome before any further writes',
    )
  }
  try {
    await confirm()
  } catch (error) {
    if (failed)
      throw new AggregateError(
        [writeError, error],
        'Registry write was not confirmed; preserve the frozen artifacts and retry the failed job',
      )
    throw error
  }
}

export async function npmVersion(
  name: string,
  version: string,
  signal?: AbortSignal,
): Promise<Record<string, unknown> | undefined> {
  return registryJson(
    `${npmRegistry}/${encodeURIComponent(name)}/${version}`,
    fetch,
    setTimeout,
    signal,
  )
}

export function assertNpmIntegrity(
  value: Record<string, unknown>,
  expected: string,
): void {
  assert.equal(
    readObject(value.dist).integrity,
    expected,
    'registry integrity differs from the frozen tarball; stop and use a new version',
  )
}

export async function waitForNpm(
  name: string,
  version: string,
  expected: string,
): Promise<void> {
  const deadline = AbortSignal.timeout(600_000)
  logReleaseProgress(`Waiting for ${name}@${version} integrity (limit 10m)`)
  // Accepted uploads can take several minutes to reach the public registry.
  for (let attempt = 0; attempt < 120; attempt++) {
    const value = await npmVersion(name, version, deadline)
    if (value) {
      assertNpmIntegrity(value, expected)
      logReleaseProgress(`Verified ${name}@${version} integrity`)
      return
    }
    await setTimeout(5_000, undefined, { signal: deadline })
  }
  throw new Error(`${name}@${version} is not visible yet; rerun the failed job`)
}

export async function waitForNpmChannel(
  name: string,
  channel: string,
  version: string,
): Promise<void> {
  const deadline = AbortSignal.timeout(60_000)
  logReleaseProgress(`Confirming ${name} ${channel} -> ${version} (limit 60s)`)
  for (let attempt = 0; attempt < 12; attempt++) {
    const value = await registryJson(
      `${npmRegistry}/${encodeURIComponent(name)}`,
      fetch,
      setTimeout,
      deadline,
    )
    if (value && readObject(value['dist-tags'])[channel] === version) {
      logReleaseProgress(`Confirmed ${name} ${channel} -> ${version}`)
      return
    }
    await setTimeout(5_000, undefined, { signal: deadline })
  }
  throw new Error(
    `${name} channel promotion was not confirmed; rerun the failed job`,
  )
}

/**
 * npm's install index can lag behind its immutable version endpoint.
 * Retry only missing-version reads; never retry publication or consumer assertions.
 */
export async function waitForNpmInstall(
  install: (remainingMs: number) => void | Promise<void>,
  pause: (milliseconds: number) => Promise<void> = setTimeout,
): Promise<void> {
  const deadline = Date.now() + 600_000
  for (let attempt = 0; attempt < 120; attempt++) {
    try {
      await install(Math.max(1, deadline - Date.now()))
      return
    } catch (error) {
      const stderr =
        error instanceof Error && 'stderr' in error ? String(error.stderr) : ''
      if (
        !/npm error code (?:ETARGET|E404)\b/.test(stderr) ||
        attempt === 119 ||
        Date.now() >= deadline
      )
        throw error
      logReleaseProgress(
        `Waiting for npm install metadata (attempt ${attempt + 1}/120; ${deadline - Date.now()}ms remaining)`,
      )
      await pause(5_000)
    }
  }
}
