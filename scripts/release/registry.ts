import assert from 'node:assert/strict'
import { setTimeout } from 'node:timers/promises'
import { readObject } from './project.ts'

export const npmRegistry: string = 'https://registry.npmjs.org'

/**
 * Only a real 404 means absent; authentication, rate limits and outages fail closed.
 */
export async function registryJson(
  url: string,
  fetcher: typeof fetch = fetch,
  pause: (milliseconds: number) => Promise<void> = setTimeout,
): Promise<Record<string, unknown> | undefined> {
  for (let attempt = 0; attempt < 4; attempt++) {
    let response: Response
    try {
      response = await fetcher(url, {
        headers: {
          'user-agent': 'gdfmt-release (https://github.com/ntnyq/gdfmt)',
          'cache-control': 'no-cache',
        },
        signal: AbortSignal.timeout(30_000),
      })
    } catch (error) {
      const transient =
        error instanceof TypeError ||
        (error instanceof Error && error.name === 'TimeoutError')
      if (!transient || attempt === 3) throw error
      await pause(1_000 * 2 ** attempt)
      continue
    }
    if (response.status === 404) return undefined
    if ((response.status === 429 || response.status >= 500) && attempt < 3) {
      const retryAfter = response.headers.get('retry-after')
      const delay =
        retryAfter === null
          ? Number.NaN
          : /^\d+$/.test(retryAfter)
            ? Number(retryAfter) * 1_000
            : Date.parse(retryAfter) - Date.now()
      await response.body?.cancel()
      await pause(
        Number.isFinite(delay)
          ? Math.max(0, Math.min(delay, 30_000))
          : 1_000 * 2 ** attempt,
      )
      continue
    }
    assert.ok(
      response.ok,
      `registry request failed (${response.status}): ${url}`,
    )
    return readObject(await response.json())
  }
  throw new Error(`registry request exhausted retries: ${url}`)
}

/**
 * A failed upload can already have committed. Confirm bytes before deciding to fail.
 * This never automatically repeats a registry write.
 */
export async function confirmRegistryWrite(
  write: () => void,
  confirm: () => Promise<void>,
): Promise<void> {
  let failed = false
  let writeError: unknown
  try {
    write()
  } catch (error) {
    failed = true
    writeError = error
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
): Promise<Record<string, unknown> | undefined> {
  return registryJson(`${npmRegistry}/${encodeURIComponent(name)}/${version}`)
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
  // Accepted uploads can take several minutes to reach the public registry.
  for (let attempt = 0; attempt < 120; attempt++) {
    const value = await npmVersion(name, version)
    if (value) {
      assertNpmIntegrity(value, expected)
      return
    }
    await setTimeout(5_000)
  }
  throw new Error(`${name}@${version} is not visible yet; rerun the failed job`)
}

export async function waitForNpmChannel(
  name: string,
  channel: string,
  version: string,
): Promise<void> {
  for (let attempt = 0; attempt < 12; attempt++) {
    const value = await registryJson(
      `${npmRegistry}/${encodeURIComponent(name)}`,
    )
    if (value && readObject(value['dist-tags'])[channel] === version) return
    await setTimeout(5_000)
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
  install: () => void,
  pause: (milliseconds: number) => Promise<void> = setTimeout,
): Promise<void> {
  for (let attempt = 0; attempt < 120; attempt++) {
    try {
      install()
      return
    } catch (error) {
      const stderr =
        error instanceof Error && 'stderr' in error ? String(error.stderr) : ''
      if (!/npm error code (?:ETARGET|E404)\b/.test(stderr) || attempt === 119)
        throw error
      if (attempt === 0)
        console.log('Waiting for npm install metadata to become visible...')
      await pause(5_000)
    }
  }
}
