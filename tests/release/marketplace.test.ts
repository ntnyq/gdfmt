import assert from 'node:assert/strict'
import { createHash } from 'node:crypto'
import { mkdtemp, readFile, rm, writeFile } from 'node:fs/promises'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import { test } from 'node:test'
import {
  planMarketplacePublication,
  isMarketplaceTargetValidated,
  validateMarketplaceBundle,
  type MarketplaceBundle,
} from '../../scripts/release/marketplace.ts'
import { readObject } from '../../scripts/release/project.ts'

const revision = 'd'.repeat(40)
const targets = [
  'darwin-arm64',
  'darwin-x64',
  'linux-x64-gnu',
  'linux-x64-musl',
  'win32-x64-msvc',
  'web',
]

async function createBundle(directory: string): Promise<MarketplaceBundle> {
  const receipts = []
  for (const target of targets) {
    const bytes = Buffer.from(`tested ${target}`)
    const file = `gdfmt-0.1.0-${target}.vsix`
    await writeFile(join(directory, file), bytes)
    receipts.push({
      schemaVersion: 1,
      target,
      file,
      revision,
      coreVersion: '0.1.0',
      extensionVersion: '0.2.1',
      cleanSource: true,
      published: false,
      sha512: createHash('sha512').update(bytes).digest('hex'),
      nativeSmokePassed: true,
      extensionHostPassed: true,
      installedVsixPassed: true,
      godotToolsPassed: true,
      webExtensionHostPassed: true,
      browser: 'chromium',
      crossOriginIsolated: false,
    })
  }
  const bytes = JSON.stringify({
    schemaVersion: 1,
    version: '0.1.0',
    revision,
    receipts,
  })
  await writeFile(join(directory, 'editors.json'), bytes)
  return validateMarketplaceBundle(
    directory,
    createHash('sha256').update(bytes).digest('hex'),
    revision,
  )
}

function createListing(bundle: MarketplaceBundle) {
  return {
    publisher: { publisherName: 'ntnyq' },
    extensionName: 'gdfmt',
    versionsTruncated: false,
    versions: bundle.artifacts.map(artifact => ({
      version: bundle.extensionVersion,
      flags: 1,
      targetPlatform: artifact.galleryTarget,
      properties: [
        {
          key: 'Microsoft.VisualStudio.Services.VsixSha256',
          value: artifact.sha256,
        },
      ],
    })),
  }
}

test('Marketplace requires a complete, tested frozen set and independently pinned provenance', async () => {
  const directory = await mkdtemp(join(tmpdir(), 'gdfmt-marketplace-'))
  try {
    const bundle = await createBundle(directory)
    assert.equal(bundle.artifacts.length, 6)
    assert.equal(
      bundle.artifacts.find(artifact => artifact.target === 'linux-x64-musl')
        ?.galleryTarget,
      'alpine-x64',
    )
    await assert.rejects(
      () => validateMarketplaceBundle(directory, '0'.repeat(64), revision),
      /independently pinned/,
    )
    await assert.rejects(() =>
      validateMarketplaceBundle(
        directory,
        bundle.manifestSha256,
        'a'.repeat(40),
      ),
    )
    const original = await readFile(join(directory, 'editors.json'), 'utf8')
    const mutations = [
      (manifest: Record<string, unknown>) => {
        manifest.version = '0.1.0-rc.1'
      },
      (manifest: Record<string, unknown>) => {
        manifest.receipts = []
      },
    ]
    for (const mutate of mutations) {
      const manifest: Record<string, unknown> = JSON.parse(original)
      mutate(manifest)
      const bytes = JSON.stringify(manifest)
      await writeFile(join(directory, 'editors.json'), bytes)
      await assert.rejects(() =>
        validateMarketplaceBundle(
          directory,
          createHash('sha256').update(bytes).digest('hex'),
          revision,
        ),
      )
    }
    for (const [key, value] of [
      ['cleanSource', false],
      ['published', true],
      ['extensionVersion', '0.2.1-rc.1'],
      ['extensionVersion', '0.2.2'],
      ['installedVsixPassed', false],
      ['nativeSmokePassed', false],
      ['target', '../outside'],
      ['file', '../outside.vsix'],
    ] satisfies [string, unknown][]) {
      const manifest = readObject(JSON.parse(original))
      assert.ok(Array.isArray(manifest.receipts))
      manifest.receipts[0] = {
        ...readObject(manifest.receipts[0]),
        [key]: value,
      }
      const bytes = JSON.stringify(manifest)
      await writeFile(join(directory, 'editors.json'), bytes)
      await assert.rejects(() =>
        validateMarketplaceBundle(
          directory,
          createHash('sha256').update(bytes).digest('hex'),
          revision,
        ),
      )
    }
    await writeFile(join(directory, 'editors.json'), original)
    await writeFile(
      join(directory, bundle.artifacts[0].file),
      'unreviewed rebuild',
    )
    await assert.rejects(
      () =>
        validateMarketplaceBundle(directory, bundle.manifestSha256, revision),
      /tested bytes/,
    )
  } finally {
    await rm(directory, { recursive: true, force: true })
  }
})

test('Marketplace resumes only missing targets and rejects collisions before any upload', async () => {
  const directory = await mkdtemp(join(tmpdir(), 'gdfmt-marketplace-'))
  try {
    const bundle = await createBundle(directory)
    assert.deepEqual(
      planMarketplacePublication(bundle, createListing(bundle)),
      [],
    )
    assert.equal(
      isMarketplaceTargetValidated(bundle, createListing(bundle), 'web'),
      true,
    )
    const unvalidated = createListing(bundle)
    unvalidated.versions[5].flags = 0
    assert.equal(
      isMarketplaceTargetValidated(bundle, unvalidated, 'web'),
      false,
    )
    const partial = createListing(bundle)
    partial.versions.splice(2)
    assert.deepEqual(
      planMarketplacePublication(bundle, partial),
      bundle.artifacts.slice(2),
    )
    const legacy = createListing(bundle)
    for (const version of legacy.versions) {
      version.version = '0.1.0'
      version.properties.push({
        key: 'Microsoft.VisualStudio.Code.PreRelease',
        value: 'true',
      })
    }
    assert.deepEqual(
      planMarketplacePublication(bundle, legacy),
      bundle.artifacts,
    )
    const collision = createListing(bundle)
    collision.versions[5].properties[0].value = 'e'.repeat(64)
    assert.throws(
      () => planMarketplacePublication(bundle, collision),
      /bytes differ/,
    )
    const prerelease = createListing(bundle)
    prerelease.versions[0].properties.push({
      key: 'Microsoft.VisualStudio.Code.PreRelease',
      value: 'true',
    })
    assert.throws(
      () => planMarketplacePublication(bundle, prerelease),
      /reuse a Gallery prerelease/,
    )
    const newer = createListing(bundle)
    newer.versions[0].version = '0.2.2'
    assert.throws(
      () => planMarketplacePublication(bundle, newer),
      /newer version/,
    )
    const truncated = { ...createListing(bundle), versionsTruncated: true }
    assert.throws(
      () => planMarketplacePublication(bundle, truncated),
      /complete Gallery history/,
    )
    const wrongPublisher = {
      ...createListing(bundle),
      publisher: { publisherName: 'other' },
    }
    assert.throws(() => planMarketplacePublication(bundle, wrongPublisher))
  } finally {
    await rm(directory, { recursive: true, force: true })
  }
})
