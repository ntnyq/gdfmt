import assert from 'node:assert/strict'
import { createHash } from 'node:crypto'
import {
  copyFile,
  mkdir,
  readFile,
  readdir,
  rm,
  writeFile,
} from 'node:fs/promises'
import { createServer } from 'node:http'
import { createRequire } from 'node:module'
import { join, resolve } from 'node:path'
import { performance } from 'node:perf_hooks'
import { pathToFileURL } from 'node:url'
import { gzipSync } from 'node:zlib'
import { chromium } from 'playwright'
import type {
  FormatResult,
  TextOptions,
} from '../../packages/gdfmt/src/index.cjs'
import { run } from '../npm/build.ts'
import {
  gitRevision,
  isCleanReleaseSource,
  readJson,
  repositoryRoot,
} from '../release/project.ts'
import { buildWasm, wasmPackageRoot } from './build.ts'

interface Formatter {
  formatSync(source: string, options?: TextOptions): FormatResult
  parseConfig(source: string): Pick<TextOptions, 'language' | 'format'> & {
    root?: boolean
    extends?: string[]
  }
  resolveTextOptions(
    layers: Pick<TextOptions, 'language' | 'format'>[],
  ): TextOptions
}
interface WasmModule {
  createFormatter(options?: {
    wasm?: Uint8Array | WebAssembly.Module
  }): Promise<Formatter>
}

const revision = gitRevision()
const cleanSource = isCleanReleaseSource()
await buildWasm()
run('cargo', ['build', '--locked', '--release', '-p', 'gdfmt_napi'])
const nativeLibrary =
  process.platform === 'win32'
    ? 'gdfmt_napi.dll'
    : process.platform === 'darwin'
      ? 'libgdfmt_napi.dylib'
      : 'libgdfmt_napi.so'
const nativePath = resolve(repositoryRoot, 'target/wasm-native/gdfmt.node')
await mkdir(resolve(nativePath, '..'), { recursive: true })
await copyFile(
  join(repositoryRoot, 'target/release', nativeLibrary),
  nativePath,
)
const binding: { formatSync(source: string, options: string): string } =
  createRequire(import.meta.url)(nativePath)
const manifest = readJson(join(wasmPackageRoot, 'package.json'))
const destination = resolve(repositoryRoot, 'target/wasm')
await rm(destination, { recursive: true, force: true })
await mkdir(destination, { recursive: true })
const pnpm = process.env.npm_execpath
assert.ok(pnpm, 'run via pnpm test:wasm')
const pnpmRun = (args: string[], cwd: string): void => {
  if (/\.[cm]?js$/.test(pnpm)) run(process.execPath, [pnpm, ...args], { cwd })
  else run(pnpm, args, { cwd })
}
pnpmRun(['pack', '--pack-destination', destination], wasmPackageRoot)
const tarballName = `gdfmt-wasm-${manifest.version}.tgz`
const tarball = join(destination, tarballName)
const consumer = join(destination, 'consumer')
await mkdir(consumer)
await writeFile(
  join(consumer, 'package.json'),
  '{"name":"wasm-consumer","private":true,"type":"module"}\n',
)
pnpmRun(['add', '--ignore-scripts', '--offline', tarball], consumer)
const installed = join(consumer, 'node_modules/@gdfmt/wasm')
const entry = join(installed, 'dist/index.mjs')
const wasm = new Uint8Array(
  await readFile(join(installed, 'dist/binding_bg.wasm')),
)
const module: WasmModule = await import(pathToFileURL(entry).href)
await assert.rejects(() =>
  module.createFormatter({ wasm: new Uint8Array([0, 1]) }),
)
await assert.rejects(
  Reflect.apply(module.createFormatter, undefined, [{ wasm: null }]),
  /wasm must not be null/,
)
const raw: {
  createBindings(): {
    init(options: { module_or_path: Uint8Array }): Promise<WebAssembly.Exports>
    formatText(source: string, options: string): string
  }
} = await import(pathToFileURL(join(installed, 'dist/binding.js')).href)
const rawBindings = raw.createBindings()
const rawExports = await rawBindings.init({ module_or_path: wasm })
const secondExports = await raw.createBindings().init({ module_or_path: wasm })
assert.ok(rawExports.memory instanceof WebAssembly.Memory)
assert.ok(secondExports.memory instanceof WebAssembly.Memory)
assert.notEqual(rawExports.memory.buffer, secondExports.memory.buffer)
const memoryInitialBytes = rawExports.memory.buffer.byteLength
const initStarted = performance.now()
const formatter = await module.createFormatter({ wasm })
const initMs = performance.now() - initStarted
const formatSync = (source: string, options: TextOptions = {}): FormatResult =>
  formatter.formatSync(source, options)

const fixtures = await Promise.all(
  (await readdir(resolve(repositoryRoot, 'tests/compatibility/fixtures')))
    .filter(name => name.endsWith('.gd'))
    .map(async name => ({
      name,
      source: await readFile(
        resolve(repositoryRoot, 'tests/compatibility/fixtures', name),
        'utf8',
      ),
    })),
)
const cases: {
  source: string
  options: TextOptions
  expected: FormatResult
}[] = []
for (const fixture of fixtures) {
  for (const godotVersion of ['4.3', '4.4', '4.5', '4.6', '4.7'] as const) {
    const options: TextOptions = { language: { godotVersion }, sourceId: 7 }
    const expected: FormatResult = JSON.parse(
      binding.formatSync(fixture.source, JSON.stringify(options)),
    )
    assert.deepEqual(
      formatSync(fixture.source, options),
      expected,
      `${fixture.name} / ${godotVersion}`,
    )
    cases.push({ source: fixture.source, options, expected })
  }
}
for (const item of cases)
  rawBindings.formatText(item.source, JSON.stringify(item.options))
const memoryAfterFixturesBytes = rawExports.memory.buffer.byteLength
for (const options of [
  null,
  { format: { lineWidth: 1 } },
  { cwd: '/tmp' },
  { format: { lineWidth: Number.NaN } },
  { format: undefined },
  { format: new Date() },
  Object.defineProperty({}, 'format', { value: { lineWidth: 40 } }),
  { format: Object.defineProperty({}, 'lineWidth', { value: undefined }) },
  {
    toJSON() {
      return {}
    },
  },
])
  assert.throws(() =>
    Reflect.apply(formatter.formatSync, formatter, ['', options]),
  )
assert.throws(() => formatSync('\ud800'))
assert.equal(formatSync('var =\n').output, null)
assert.deepEqual(
  formatter.parseConfig('{// JSONC\n"root":true,"extends":["base.json"],}'),
  { root: true, extends: ['base.json'] },
)
const resolved = formatter.resolveTextOptions([
  { format: { indentWidth: 2 } },
  { format: { finalNewline: false } },
])
assert.equal(resolved.format?.indentWidth, 2)
assert.equal(formatSync('var x=1\n', resolved).output, 'var x = 1')
const base = formatter.parseConfig('{/* shared */ "format":{"indentWidth":2}}')
const exampleOptions = formatter.resolveTextOptions([
  { language: base.language ?? {}, format: base.format ?? {} },
  { format: { indentStyle: 'space', finalNewline: false } },
])
assert.equal(formatSync('var x=1\n', exampleOptions).output, 'var x = 1')
assert.throws(() => formatter.parseConfig('{"overrides":[]}'))
assert.throws(() => formatter.parseConfig('{"ignore":[]}'))
assert.deepEqual(
  formatter.parseConfig('{"$schema":"schema.json","schemaVersion":1}'),
  { $schema: 'schema.json', schemaVersion: 1 },
)
const hugeSource = '#'.repeat(16 * 1024 * 1024 + 1)
assert.deepEqual(
  formatSync(hugeSource),
  JSON.parse(binding.formatSync(hugeSource, '{}')),
)
assert.equal(
  (
    await module.createFormatter({ wasm: await WebAssembly.compile(wasm) })
  ).formatSync('var x=1\n').output,
  'var x = 1\n',
)

// The packed declaration/export map must resolve without the native package.
await copyFile(
  resolve(repositoryRoot, 'tests/wasm/consumer.mts'),
  join(consumer, 'consumer.mts'),
)
await writeFile(
  join(consumer, 'tsconfig.json'),
  JSON.stringify({
    compilerOptions: {
      strict: true,
      noEmit: true,
      target: 'ES2023',
      module: 'NodeNext',
      types: [],
    },
    include: ['consumer.mts'],
  }),
)
run(process.execPath, [
  resolve(repositoryRoot, 'node_modules/typescript/bin/tsc'),
  '-p',
  join(consumer, 'tsconfig.json'),
])

// Serve the installed tarball without isolation headers. Both the page and its
// module Worker import the exact packed ESM entry and fetch its WASM asset.
const server = createServer((request, response) => {
  const path = decodeURIComponent((request.url ?? '/').split('?')[0])
  if (path === '/') {
    response.setHeader('Content-Type', 'text/html')
    response.end('<!doctype html><title>gdfmt WASM acceptance</title>')
    return
  }
  if (path === '/worker.mjs') {
    response.setHeader('Content-Type', 'text/javascript')
    response.end(
      `import {createFormatter} from '/dist/index.mjs'; const formatter = await createFormatter(); onmessage = ({data}) => {try {postMessage({result: formatter.formatSync(data)})} catch (error) {postMessage({error: String(error)})}}; postMessage({ready:true});`,
    )
    return
  }
  if (!/^\/dist\/[a-z_.-]+$/.test(path)) {
    response.statusCode = 404
    response.end()
    return
  }
  response.setHeader(
    'Content-Type',
    path.endsWith('.wasm') ? 'application/wasm' : 'text/javascript',
  )
  readFile(join(installed, path)).then(
    bytes => response.end(bytes),
    () => {
      response.statusCode = 404
      response.end()
    },
  )
})
await new Promise<void>(done => server.listen(0, '127.0.0.1', done))
const address = server.address()
assert.ok(address && typeof address === 'object')
const browser = await chromium.launch()
let browserMetrics: unknown
try {
  const page = await browser.newPage()
  const requests: string[] = []
  page.on('request', request => requests.push(request.url()))
  await page.goto(`http://127.0.0.1:${address.port}`)
  browserMetrics = await page.evaluate(async cases => {
    // The dynamic import belongs to the served browser, not the Node test runner.
    const entry = '/dist/index.mjs'
    const { createFormatter } = await import(entry)
    const start = performance.now()
    const formatter = await createFormatter()
    const initMs = performance.now() - start
    const formatStart = performance.now()
    for (const item of cases) {
      if (
        JSON.stringify(formatter.formatSync(item.source, item.options)) !==
        JSON.stringify(item.expected)
      )
        throw new Error('browser/native fixture mismatch')
    }
    const formatMs = performance.now() - formatStart
    const worker = new Worker('/worker.mjs', { type: 'module' })
    const workerResult = await new Promise<unknown>((done, reject) => {
      const timeout = setTimeout(
        () => reject(new Error('Worker timed out')),
        15_000,
      )
      worker.onerror = reject
      worker.onmessage = ({ data }) => {
        if (data.ready) worker.postMessage('var 名称=\"😀\"\r\n')
        else {
          clearTimeout(timeout)
          done(data)
        }
      }
    })
    worker.terminate()
    const bytes = await (await fetch('/dist/binding_bg.wasm')).arrayBuffer()
    const compiled = await WebAssembly.compile(bytes)
    // WebAssembly imports must expose no WASI filesystem, threads or shared memory.
    const imports = WebAssembly.Module.imports(compiled)
    if (imports.some(item => /wasi|thread|memory/.test(item.module)))
      throw new Error('unexpected WASM host dependency')
    return { initMs, formatMs, crossOriginIsolated, workerResult, imports }
  }, cases)
  assert.ok(browserMetrics && typeof browserMetrics === 'object')
  const metrics = browserMetrics as {
    crossOriginIsolated: boolean
    workerResult: { result: FormatResult }
  }
  assert.equal(metrics.crossOriginIsolated, false)
  assert.equal(metrics.workerResult.result.output, 'var 名称 = "😀"\n')
  assert.ok(requests.every(url => !url.endsWith('.node')))
} finally {
  await browser.close()
  await new Promise<void>((done, reject) =>
    server.close(error => (error ? reject(error) : done())),
  )
}
const glue = await readFile(join(installed, 'dist/binding.js'))
await writeFile(
  join(destination, 'acceptance.json'),
  `${JSON.stringify(
    {
      schemaVersion: 1,
      revision,
      cleanSource: cleanSource && isCleanReleaseSource(),
      version: manifest.version,
      tarball: tarballName,
      sha512: createHash('sha512')
        .update(await readFile(tarball))
        .digest('hex'),
      wasmBytes: wasm.byteLength,
      wasmGzipBytes: gzipSync(wasm).byteLength,
      glueBytes: glue.byteLength,
      memoryInitialBytes,
      memoryAfterFixturesBytes,
      nodeInitMs: initMs,
      fixtureCases: cases.length,
      browser: browserMetrics,
      passed: true,
      published: false,
    },
    null,
    2,
  )}\n`,
)
assert.equal(gitRevision(), revision)
console.log(
  `WASM tarball: ${cases.length} fixture/profile cases passed in Node and Chromium, plus Worker and limit checks.`,
)
