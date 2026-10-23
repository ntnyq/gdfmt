import assert from 'node:assert/strict'
import { createHash } from 'node:crypto'
import { copyFile, mkdir, readFile, writeFile } from 'node:fs/promises'
import { join } from 'node:path'
import { targets } from '../npm/build.ts'
import {
  checkRelease,
  gitRevision,
  readJson,
  repositoryRoot,
} from './project.ts'

/**
 * Freezes the tested VSIX bytes, requiring Extension Host evidence on desktop
 * runners, a native CLI smoke check for Alpine, and a browser Extension Host
 * check for the independent Web package.
 */
export async function prepareEditors(
  source: string,
  destination: string,
): Promise<void> {
  const version = checkRelease()
  const revision = gitRevision()
  const extensionVersion = readJson(
    join(repositoryRoot, 'editors/vscode/package.json'),
  ).version
  const receipts: Record<string, unknown>[] = []
  await mkdir(destination, { recursive: true })
  for (const target of targets) {
    const directory = join(source, `vscode-${target.id}`)
    const receipt = readJson(join(directory, 'acceptance.json'))
    assert.equal(receipt.schemaVersion, 1)
    assert.equal(receipt.coreVersion, version)
    assert.equal(receipt.extensionVersion, extensionVersion)
    assert.equal(receipt.revision, revision)
    assert.equal(
      receipt.cleanSource,
      true,
      'VSIX artifacts require clean source',
    )
    assert.equal(receipt.target, target.id)
    assert.equal(receipt.published, false)
    assert.equal(receipt.nativeSmokePassed, true)
    if (target.id !== 'linux-x64-musl') {
      assert.equal(receipt.extensionHostPassed, true)
      assert.equal(
        receipt.installedVsixPassed,
        true,
        'installed VSIX acceptance is required',
      )
    }
    if (target.id === 'linux-x64-gnu')
      assert.equal(receipt.godotToolsPassed, true)
    const file = `gdfmt-${version}-${target.id}.vsix`
    assert.equal(receipt.file, file)
    const bytes = await readFile(join(directory, file))
    assert.equal(
      receipt.sha512,
      createHash('sha512').update(bytes).digest('hex'),
      'VSIX must match tested bytes',
    )
    await copyFile(join(directory, file), join(destination, file))
    receipts.push(receipt)
  }
  const webDirectory = join(source, 'vscode-web')
  const web = readJson(join(webDirectory, 'acceptance.json'))
  assert.equal(web.schemaVersion, 1)
  assert.equal(web.coreVersion, version)
  assert.equal(web.extensionVersion, extensionVersion)
  assert.equal(web.revision, revision)
  assert.equal(web.cleanSource, true, 'VSIX artifacts require clean source')
  assert.equal(web.target, 'web')
  assert.equal(web.published, false)
  assert.equal(
    web.webExtensionHostPassed,
    true,
    'Web Extension Host acceptance is required',
  )
  assert.equal(web.browser, 'chromium')
  assert.equal(web.crossOriginIsolated, false)
  const webFile = `gdfmt-${version}-web.vsix`
  assert.equal(web.file, webFile)
  const webBytes = await readFile(join(webDirectory, webFile))
  assert.equal(
    web.sha512,
    createHash('sha512').update(webBytes).digest('hex'),
    'VSIX must match tested bytes',
  )
  await copyFile(join(webDirectory, webFile), join(destination, webFile))
  receipts.push(web)
  await writeFile(
    join(destination, 'editors.json'),
    `${JSON.stringify({ schemaVersion: 1, version, revision, receipts }, null, 2)}\n`,
  )
}
