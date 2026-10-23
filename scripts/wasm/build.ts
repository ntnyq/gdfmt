import assert from 'node:assert/strict'
import { execFileSync } from 'node:child_process'
import { copyFile, mkdir, readFile, rm, writeFile } from 'node:fs/promises'
import { delimiter, dirname, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'
import { run } from '../npm/build.ts'
import {
  repositoryRoot,
  workspaceVersion,
  readJson,
} from '../release/project.ts'

export const wasmPackageRoot: string = resolve(repositoryRoot, 'packages/wasm')

export async function buildWasm(): Promise<void> {
  const manifest = readJson(resolve(wasmPackageRoot, 'package.json'))
  assert.equal(
    manifest.version,
    workspaceVersion(
      await readFile(resolve(repositoryRoot, 'Cargo.toml'), 'utf8'),
    ),
  )
  const dist = resolve(wasmPackageRoot, 'dist')
  const toolchain = await readFile(
    resolve(repositoryRoot, 'rust-toolchain.toml'),
    'utf8',
  )
  const channel = /^channel\s*=\s*"([^"]+)"/m.exec(toolchain)?.[1]
  assert.ok(channel, 'rust-toolchain.toml must pin a toolchain channel')
  const rustc = execFileSync(
    'rustup',
    ['which', '--toolchain', channel, 'rustc'],
    {
      cwd: repositoryRoot,
      encoding: 'utf8',
    },
  ).trim()
  await rm(dist, { recursive: true, force: true })
  await mkdir(dist, { recursive: true })
  // Honor the workspace pin even when another Rust installation precedes rustup.
  run(
    'wasm-pack',
    [
      'build',
      'crates/gdfmt_wasm',
      '--target',
      'web',
      '--release',
      '--out-dir',
      '../../packages/wasm/dist',
      '--out-name',
      'binding',
      '--no-pack',
      '--',
      '--locked',
    ],
    {
      env: {
        ...process.env,
        PATH: `${dirname(rustc)}${delimiter}${process.env.PATH ?? ''}`,
        RUSTC: rustc,
      },
    },
  )
  // wasm-bindgen's web output caches a singleton. Enclose its glue in a factory
  // so formatter instances own their memories and can be replaced after a trap.
  // These assertions intentionally fail on incompatible generator upgrades.
  const gluePath = resolve(dist, 'binding.js')
  let glue = await readFile(gluePath, 'utf8')
  assert.ok(!/^import /m.test(glue), 'unexpected imports in wasm-bindgen glue')
  const exports = ['format_text', 'parse_config', 'resolve_text_options']
  for (const name of exports)
    assert.equal(
      (glue.match(new RegExp(`export function ${name}\\(`, 'g')) ?? []).length,
      1,
    )
  assert.ok(glue.includes('export { initSync, __wbg_init as default };'))
  glue = glue
    .replace(/export function /g, 'function ')
    .replace('export { initSync, __wbg_init as default };', '')
  assert.ok(!/^export /m.test(glue), 'unhandled export in wasm-bindgen glue')
  await writeFile(
    gluePath,
    `export function createBindings() {\n${glue}\nreturn {init: __wbg_init, formatText: format_text, parseConfig: parse_config, resolveTextOptions: resolve_text_options};\n}\n`,
  )
  await rm(resolve(dist, 'binding.d.ts'), { force: true })
  await rm(resolve(dist, 'binding_bg.wasm.d.ts'), { force: true })
  await rm(resolve(dist, '.gitignore'), { force: true })
  await copyFile(
    resolve(wasmPackageRoot, 'src/index.mjs'),
    resolve(dist, 'index.mjs'),
  )
  // Source declarations reuse native text types before either package is built.
  // Installed consumers use the self-contained copy emitted below.
  const apiTypes = await readFile(
    resolve(wasmPackageRoot, 'src/index.d.mts'),
    'utf8',
  )
  assert.ok(apiTypes.includes('../../gdfmt/src/index.d.cts'))
  await writeFile(
    resolve(dist, 'index.d.mts'),
    apiTypes.replaceAll('../../gdfmt/src/index.d.cts', './types.js'),
  )
  const validation = await readFile(
    resolve(repositoryRoot, 'packages/gdfmt/src/text-options.cjs'),
    'utf8',
  )
  assert.ok(
    validation.includes('module.exports = { text, object, encodeOptions }'),
  )
  await writeFile(
    resolve(dist, 'text-options.mjs'),
    validation.replace(
      'module.exports = { text, object, encodeOptions }',
      'export { text, object, encodeOptions }',
    ),
  )
  const declarations = await readFile(
    resolve(repositoryRoot, 'packages/gdfmt/src/index.d.cts'),
    'utf8',
  )
  assert.ok(declarations.includes('export interface ResolvedConfig'))
  await writeFile(
    resolve(dist, 'types.d.ts'),
    declarations.slice(
      0,
      declarations.indexOf('export interface ResolvedConfig'),
    ),
  )
  for (const name of ['LICENSE', 'LICENSE-THIRD-PARTY'])
    await copyFile(
      resolve(repositoryRoot, name),
      resolve(wasmPackageRoot, name),
    )
}

if (
  process.argv[1] &&
  resolve(process.argv[1]) === fileURLToPath(import.meta.url)
)
  await buildWasm()
