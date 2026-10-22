import assert from 'node:assert/strict'
import { readFile, writeFile } from 'node:fs/promises'
import { dirname, resolve } from 'node:path'
import { runTests } from '@vscode/test-web'
import { build } from 'esbuild'
import { repositoryRoot } from '../release/project.ts'
import { extensionRoot } from './build.ts'
import { packageWebExtension, webStagingRoot } from './package-web.ts'

const vsix = await packageWebExtension()
const testPath = resolve(webStagingRoot, 'dist/web/test.js')
await build({
  entryPoints: [resolve(extensionRoot, 'tests/web/extension.ts')],
  outfile: testPath,
  bundle: true,
  platform: 'browser',
  format: 'cjs',
  target: 'es2022',
  external: ['vscode'],
})
await runTests({
  browserType: 'chromium',
  headless: true,
  coi: false,
  quality: 'stable',
  commit: 'cd4ee3b1c348a13bafd8f9ad8060705f6d4b9cba',
  extensionDevelopmentPath: webStagingRoot,
  extensionTestsPath: testPath,
  port: 3000,
  testRunnerDataDir: resolve(repositoryRoot, 'target/vscode-web-test/runtime'),
})
const receiptPath = resolve(dirname(vsix), 'acceptance.json')
const receipt: unknown = JSON.parse(await readFile(receiptPath, 'utf8'))
assert.ok(receipt && typeof receipt === 'object')
await writeFile(
  receiptPath,
  `${JSON.stringify({ ...receipt, webExtensionHostPassed: true, browser: 'chromium', crossOriginIsolated: false }, null, 2)}\n`,
)
