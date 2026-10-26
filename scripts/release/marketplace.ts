import assert from 'node:assert/strict'
import { createHash } from 'node:crypto'
import { mkdir, readFile, writeFile } from 'node:fs/promises'
import { createRequire } from 'node:module'
import { dirname, join, resolve } from 'node:path'
import { setTimeout } from 'node:timers/promises'
import { fileURLToPath } from 'node:url'
import { parseArgs } from 'node:util'
import { runReleaseCommand } from './commands.ts'
import { logReleaseProgress } from './progress.ts'
import { compareVersions, readObject, validateVersion } from './project.ts'

const GALLERY_TARGETS: Readonly<Record<string, string>> = {
  'darwin-arm64': 'darwin-arm64',
  'darwin-x64': 'darwin-x64',
  'linux-x64-gnu': 'linux-x64',
  'linux-x64-musl': 'alpine-x64',
  'win32-x64-msvc': 'win32-x64',
  web: 'web',
}

export interface MarketplaceArtifact {
  target: string
  galleryTarget: string
  file: string
  sha256: string
  sha512: string
}

export interface MarketplaceBundle {
  version: string
  extensionVersion: string
  revision: string
  manifestSha256: string
  artifacts: MarketplaceArtifact[]
}

/**
 * The caller pins the original release manifest independently of the download.
 * Recovery can run from newer tooling without rebuilding the tagged source.
 */
export async function validateMarketplaceBundle(
  directory: string,
  manifestSha256: string,
  revision: string,
): Promise<MarketplaceBundle> {
  assert.match(manifestSha256, /^[a-f0-9]{64}$/)
  assert.match(revision, /^[a-f0-9]{40}$/)
  const bytes = await readFile(join(directory, 'editors.json'))
  assert.equal(
    createHash('sha256').update(bytes).digest('hex'),
    manifestSha256,
    'frozen editor manifest differs from the independently pinned digest',
  )
  const manifest = readObject(JSON.parse(bytes.toString('utf8')))
  assert.equal(manifest.schemaVersion, 1)
  assert.equal(manifest.revision, revision)
  assert.ok(typeof manifest.version === 'string')
  validateVersion(manifest.version)
  assert.ok(!manifest.version.includes('-'), 'stable core version required')
  assert.ok(Array.isArray(manifest.receipts))
  assert.equal(manifest.receipts.length, Object.keys(GALLERY_TARGETS).length)
  const artifacts: MarketplaceArtifact[] = []
  let extensionVersion: string | undefined
  for (const input of manifest.receipts) {
    const receipt = readObject(input)
    assert.ok(typeof receipt.target === 'string')
    const galleryTarget = Object.hasOwn(GALLERY_TARGETS, receipt.target)
      ? GALLERY_TARGETS[receipt.target]
      : undefined
    assert.ok(galleryTarget, 'unknown editor target')
    assert.ok(!artifacts.some(artifact => artifact.target === receipt.target))
    assert.equal(receipt.schemaVersion, 1)
    assert.equal(receipt.revision, revision)
    assert.equal(receipt.coreVersion, manifest.version)
    assert.equal(receipt.cleanSource, true)
    assert.equal(receipt.published, false)
    assert.ok(typeof receipt.extensionVersion === 'string')
    validateVersion(receipt.extensionVersion)
    assert.ok(!receipt.extensionVersion.includes('-'))
    extensionVersion ??= receipt.extensionVersion
    assert.equal(receipt.extensionVersion, extensionVersion)
    if (receipt.target === 'web') {
      assert.equal(receipt.webExtensionHostPassed, true)
      assert.equal(receipt.browser, 'chromium')
      assert.equal(receipt.crossOriginIsolated, false)
    } else {
      assert.equal(receipt.nativeSmokePassed, true)
      if (receipt.target !== 'linux-x64-musl') {
        assert.equal(receipt.extensionHostPassed, true)
        assert.equal(receipt.installedVsixPassed, true)
      }
      if (receipt.target === 'linux-x64-gnu')
        assert.equal(receipt.godotToolsPassed, true)
    }
    const file: string = `gdfmt-${manifest.version}-${receipt.target}.vsix`
    assert.equal(receipt.file, file)
    const archive = await readFile(join(directory, file))
    const sha512 = createHash('sha512').update(archive).digest('hex')
    assert.equal(receipt.sha512, sha512, 'VSIX differs from tested bytes')
    artifacts.push({
      target: receipt.target,
      galleryTarget,
      file,
      sha256: createHash('sha256').update(archive).digest('hex'),
      sha512,
    })
  }
  assert.ok(extensionVersion)
  return {
    version: manifest.version,
    extensionVersion,
    revision,
    manifestSha256,
    artifacts,
  }
}

/**
 * Validate every existing target before any write, including partial rollouts.
 * Matching immutable bytes may be resumed; prerelease reuse and downgrades stop.
 */
export function planMarketplacePublication(
  bundle: MarketplaceBundle,
  input: unknown,
): MarketplaceArtifact[] {
  const listing = readObject(input)
  assert.equal(readObject(listing.publisher).publisherName, 'ntnyq')
  assert.equal(listing.extensionName, 'gdfmt')
  assert.equal(
    listing.versionsTruncated,
    false,
    'complete Gallery history required',
  )
  assert.ok(Array.isArray(listing.versions))
  const versions = listing.versions.map(readObject)
  for (const version of versions) {
    assert.ok(typeof version.version === 'string')
    assert.ok(
      compareVersions(version.version, bundle.extensionVersion) <= 0,
      'Gallery has a newer version; refusing a stale rollout',
    )
    if (version.version !== bundle.extensionVersion) continue
    const artifact = bundle.artifacts.find(
      artifact => artifact.galleryTarget === version.targetPlatform,
    )
    assert.ok(artifact, 'unexpected Gallery target at the selected version')
    assert.ok(Array.isArray(version.properties))
    const properties = version.properties.map(readObject)
    assert.ok(
      !properties.some(
        property =>
          property.key === 'Microsoft.VisualStudio.Code.PreRelease' &&
          property.value !== 'false',
      ),
      'cannot reuse a Gallery prerelease version for stable',
    )
    assert.equal(
      properties.find(
        property =>
          property.key === 'Microsoft.VisualStudio.Services.VsixSha256',
      )?.value,
      artifact.sha256,
      'Gallery bytes differ from frozen VSIX; use a new extension version',
    )
    assert.equal(
      versions.filter(
        candidate =>
          candidate.version === version.version &&
          candidate.targetPlatform === version.targetPlatform,
      ).length,
      1,
    )
  }
  return bundle.artifacts.filter(
    artifact =>
      !versions.some(
        version =>
          version.version === bundle.extensionVersion &&
          version.targetPlatform === artifact.galleryTarget,
      ),
  )
}

/**
 * A downloadable archive can still be awaiting Gallery validation. Only the
 * Validated version flag proves that stable clients may install this target.
 */
export function isMarketplaceTargetValidated(
  bundle: MarketplaceBundle,
  input: unknown,
  galleryTarget: string,
): boolean {
  planMarketplacePublication(bundle, input)
  const versions = readObject(input).versions
  assert.ok(Array.isArray(versions))
  const version = versions
    .map(readObject)
    .find(
      version =>
        version.version === bundle.extensionVersion &&
        version.targetPlatform === galleryTarget,
    )
  if (!version) return false
  assert.ok(
    typeof version.flags === 'number' &&
      Number.isSafeInteger(version.flags) &&
      version.flags >= 0,
  )
  return (version.flags & 1) !== 0
}

async function publishMarketplace(
  directory: string,
  manifestSha256: string,
  revision: string,
  reportPath: string,
  shouldExecute: boolean,
): Promise<void> {
  const bundle = await validateMarketplaceBundle(
    directory,
    manifestSha256,
    revision,
  )
  const vsce = createRequire(import.meta.url).resolve('@vscode/vsce/vsce')
  const command = (args: string[], timeoutMs = 180_000): Promise<string> =>
    runReleaseCommand(process.execPath, [vsce, ...args], {
      label: `vsce ${args.join(' ')}`,
      captureOutput: true,
      timeoutMs,
    })
  const readListing = async (timeoutMs?: number): Promise<unknown> =>
    JSON.parse(await command(['show', 'ntnyq.gdfmt', '--json'], timeoutMs))
  const before = await readListing()
  const pending = planMarketplacePublication(bundle, before)
  await mkdir(dirname(reportPath), { recursive: true })
  const verified: string[] = []
  const uploaded: string[] = []
  const save = async (after?: unknown): Promise<void> => {
    await writeFile(
      reportPath,
      `${JSON.stringify(
        {
          schemaVersion: 1,
          checkedAt: new Date().toISOString(),
          ...bundle,
          execute: shouldExecute,
          pending: pending.map(artifact => artifact.target),
          verified,
          uploaded,
          complete:
            after !== undefined && verified.length === bundle.artifacts.length,
          before,
          after,
        },
        null,
        2,
      )}\n`,
    )
  }
  await save()
  logReleaseProgress(`${pending.length} frozen VSIX targets need publication`)
  if (!shouldExecute) return
  if (pending.length) await command(['verify-pat', 'ntnyq'])
  const confirm = async (artifact: MarketplaceArtifact): Promise<void> => {
    const deadline = Date.now() + 600_000
    while (Date.now() < deadline) {
      const listing = await readListing(
        Math.max(1, Math.min(30_000, deadline - Date.now())),
      )
      if (isMarketplaceTargetValidated(bundle, listing, artifact.galleryTarget))
        return
      await setTimeout(Math.max(0, Math.min(5_000, deadline - Date.now())))
    }
    throw new Error(
      `Gallery has not validated ${artifact.target}; preserve artifacts and resume`,
    )
  }
  for (const artifact of pending) {
    // Recheck all target identities immediately before each independent write.
    const missing = planMarketplacePublication(bundle, await readListing())
    if (!missing.some(candidate => candidate.target === artifact.target))
      continue
    assert.equal(
      createHash('sha512')
        .update(await readFile(join(directory, artifact.file)))
        .digest('hex'),
      artifact.sha512,
    )
    try {
      await command([
        'publish',
        '--packagePath',
        join(directory, artifact.file),
      ])
    } catch (writeError) {
      // A failed upload may already have committed. Never repeat the write;
      // reconcile before uploading another target.
      try {
        await confirm(artifact)
      } catch (confirmationError) {
        throw new AggregateError(
          [writeError, confirmationError],
          'Marketplace write was not confirmed; preserve the frozen artifacts and resume',
        )
      }
    }
    uploaded.push(artifact.target)
    await save()
  }
  for (const artifact of bundle.artifacts) {
    await confirm(artifact)
    const response = await fetch(
      `https://marketplace.visualstudio.com/_apis/public/gallery/publishers/ntnyq/vsextensions/gdfmt/${bundle.extensionVersion}/vspackage?targetPlatform=${artifact.galleryTarget}`,
      { signal: AbortSignal.timeout(30_000) },
    )
    assert.ok(response.ok, `Gallery download failed: HTTP ${response.status}`)
    const bytes = Buffer.from(await response.arrayBuffer())
    assert.equal(
      createHash('sha256').update(bytes).digest('hex'),
      artifact.sha256,
    )
    verified.push(artifact.target)
    await save()
    logReleaseProgress(`Verified Gallery download: ${artifact.target}`)
  }
  const after = await readListing()
  assert.equal(planMarketplacePublication(bundle, after).length, 0)
  for (const artifact of bundle.artifacts)
    assert.ok(
      isMarketplaceTargetValidated(bundle, after, artifact.galleryTarget),
    )
  await save(after)
}

if (
  process.argv[1] &&
  resolve(process.argv[1]) === fileURLToPath(import.meta.url)
) {
  const { values } = parseArgs({
    options: {
      artifacts: { type: 'string' },
      'manifest-sha256': { type: 'string' },
      revision: { type: 'string' },
      report: {
        type: 'string',
        default: 'target/marketplace/publication.json',
      },
      execute: { type: 'boolean', default: false },
    },
  })
  assert.ok(
    values.artifacts && values['manifest-sha256'] && values.revision,
    'usage: marketplace.ts --artifacts DIR --manifest-sha256 SHA256 --revision COMMIT [--execute]',
  )
  await publishMarketplace(
    resolve(values.artifacts),
    values['manifest-sha256'],
    values.revision,
    resolve(values.report),
    values.execute,
  )
}
