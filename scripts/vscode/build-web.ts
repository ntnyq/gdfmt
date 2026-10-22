import assert from 'node:assert/strict'
import { copyFile, mkdir, writeFile } from 'node:fs/promises'
import { resolve } from 'node:path'
import { fileURLToPath } from 'node:url'
import { build } from 'esbuild'
import { buildWasm, wasmPackageRoot } from '../wasm/build.ts'
import { extensionRoot, prepareExtensionAssets } from './build.ts'

export async function buildWebExtension(): Promise<void> {
  await buildWasm()
  await prepareExtensionAssets()
  const directory = resolve(extensionRoot, 'dist/web')
  await mkdir(directory, { recursive: true })
  for (const [entry, file, format] of [
    ['extension', 'extension.js', 'cjs'],
    ['formatter-worker', 'formatter-worker.js', 'iife'],
  ] as const) {
    const result = await build({
      entryPoints: [resolve(extensionRoot, 'src/web', `${entry}.ts`)],
      outfile: resolve(directory, file),
      bundle: true,
      platform: 'browser',
      target: 'es2022',
      format,
      external: ['vscode'],
      metafile: true,
      // The worker always initializes the existing WASM API with explicit bytes.
      define: { 'import.meta.url': '""' },
    })
    for (const output of Object.values(result.metafile.outputs))
      assert.ok(
        output.imports.every(item => item.path === 'vscode'),
        'Web bundles must not load Node or extra JS modules',
      )
    await writeFile(
      resolve(directory, `${file}.meta.json`),
      JSON.stringify(result.metafile),
    )
  }
  await copyFile(
    resolve(wasmPackageRoot, 'dist/binding_bg.wasm'),
    resolve(directory, 'formatter.wasm'),
  )
}

if (
  process.argv[1] &&
  resolve(process.argv[1]) === fileURLToPath(import.meta.url)
)
  await buildWebExtension()
