import assert from 'node:assert/strict'
import { createHash } from 'node:crypto'
import { mkdtemp, mkdir, writeFile, rm, readFile } from 'node:fs/promises'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import { test } from 'node:test'
import { targets, metadata } from '../../scripts/npm/build.ts'
import { prepareEditors } from '../../scripts/release/editor-artifacts.ts'
import {
  gitRevision,
  readJson,
  repositoryRoot,
} from '../../scripts/release/project.ts'

test('editor freeze binds all VSIX targets to tested bytes and the clean core revision', async () => {
  const root = await mkdtemp(join(tmpdir(), 'gdfmt-editor-artifacts-'))
  try {
    const source = join(root, 'source')
    const destination = join(root, 'frozen')
    for (const target of targets) {
      const directory = join(source, `vscode-${target.id}`)
      await mkdir(directory, { recursive: true })
      const file = `gdfmt-${metadata.version}-${target.id}.vsix`
      const bytes = `synthetic VSIX ${target.id}`
      await writeFile(join(directory, file), bytes)
      await writeFile(
        join(directory, 'acceptance.json'),
        JSON.stringify({
          schemaVersion: 1,
          coreVersion: metadata.version,
          extensionVersion: readJson(
            join(repositoryRoot, 'editors/vscode/package.json'),
          ).version,
          revision: gitRevision(),
          cleanSource: true,
          target: target.id,
          file,
          sha512: createHash('sha512').update(bytes).digest('hex'),
          godotToolsPassed: target.id === 'linux-x64-gnu',
          nativeSmokePassed: true,
          extensionHostPassed: target.id !== 'linux-x64-musl',
          published: false,
        }),
      )
    }
    const webDirectory = join(source, 'vscode-web')
    await mkdir(webDirectory, { recursive: true })
    const webFile = `gdfmt-${metadata.version}-web.vsix`
    const webBytes = 'synthetic Web VSIX'
    await writeFile(join(webDirectory, webFile), webBytes)
    const web = {
      schemaVersion: 1,
      coreVersion: metadata.version,
      extensionVersion: readJson(
        join(repositoryRoot, 'editors/vscode/package.json'),
      ).version,
      revision: gitRevision(),
      cleanSource: true,
      target: 'web',
      file: webFile,
      sha512: createHash('sha512').update(webBytes).digest('hex'),
      webExtensionHostPassed: true,
      browser: 'chromium',
      crossOriginIsolated: false,
      published: false,
    }
    const webReceiptPath = join(webDirectory, 'acceptance.json')
    await writeFile(
      webReceiptPath,
      JSON.stringify({ ...web, webExtensionHostPassed: false }),
    )
    await assert.rejects(
      () => prepareEditors(source, destination),
      /Web Extension Host/,
    )
    await writeFile(webReceiptPath, JSON.stringify(web))
    await prepareEditors(source, destination)
    const manifest: unknown = JSON.parse(
      await readFile(join(destination, 'editors.json'), 'utf8'),
    )
    assert.ok(
      manifest &&
        typeof manifest === 'object' &&
        'receipts' in manifest &&
        Array.isArray(manifest.receipts),
    )
    assert.equal(manifest.receipts.length, targets.length + 1)
    const directory = join(source, `vscode-${targets[0].id}`)
    const receiptPath = join(directory, 'acceptance.json')
    const receipt = readJson(receiptPath)
    await writeFile(
      receiptPath,
      JSON.stringify({ ...receipt, cleanSource: false }),
    )
    await assert.rejects(
      () => prepareEditors(source, destination),
      /clean source/,
    )
    await writeFile(receiptPath, JSON.stringify(receipt))
    await writeFile(
      join(directory, `gdfmt-${metadata.version}-${targets[0].id}.vsix`),
      'changed bytes',
    )
    await assert.rejects(
      () => prepareEditors(source, destination),
      /VSIX must match/,
    )
  } finally {
    await rm(root, { recursive: true, force: true })
  }
})
