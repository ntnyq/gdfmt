import assert from 'node:assert/strict'
import { spawn } from 'node:child_process'
import type { SpawnOptions } from 'node:child_process'
import { createHash } from 'node:crypto'
import { cp, mkdir, readFile, rm, symlink, writeFile } from 'node:fs/promises'
import { createServer } from 'node:http'
import { dirname, join, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'
import { gitRevision, isCleanReleaseSource } from '../release/project.ts'
import { build, metadata, outputRoot, run, targets } from './build.ts'

const revision = gitRevision()
const cleanSource = isCleanReleaseSource()
const { target, nativeRoot, packageRoot, standalone } = await build()
const artifacts = join(outputRoot, 'artifacts', target.id)
await rm(artifacts, { recursive: true, force: true })
await mkdir(artifacts, { recursive: true })
// Packing uses portable pnpm commands; no publish hooks or registry writes exist.
const pnpm = process.env.npm_execpath
assert.ok(pnpm, 'run via pnpm test:npm')
const pack = (directory: string): void => {
  // npm_execpath can be the pnpm native executable or a JavaScript entry point.
  if (/\.[cm]?js$/.test(pnpm))
    run(process.execPath, [pnpm, 'pack', '--pack-destination', artifacts], {
      cwd: directory,
    })
  else run(pnpm, ['pack', '--pack-destination', artifacts], { cwd: directory })
}
pack(packageRoot)
pack(nativeRoot)
const rootTarball = join(artifacts, `gdfmt-${metadata.version}.tgz`)
const nativeTarball = join(
  artifacts,
  `gdfmt-${target.id}-${metadata.version}.tgz`,
)
const nativeBytes = await readFile(nativeTarball)
const rootSha512 = createHash('sha512')
  .update(await readFile(rootTarball))
  .digest('hex')
const nativeSha512 = createHash('sha512').update(nativeBytes).digest('hex')
const nativeMetadata = JSON.parse(
  await readFile(join(nativeRoot, 'package.json'), 'utf8'),
)
assert.equal(nativeMetadata.version, metadata.version)
assert.deepEqual(
  Object.keys(metadata.optionalDependencies).sort(),
  targets.map(item => `@gdfmt/${item.id}`).sort(),
)
const consumer = join(outputRoot, 'consumers', target.id)
await rm(consumer, { recursive: true, force: true })
await mkdir(consumer, { recursive: true })
await writeFile(
  join(consumer, 'package.json'),
  '{"name":"gdfmt-consumer","private":true,"type":"module"}\n',
)
const runtime = join(consumer, 'runtime')
await mkdir(runtime)
if (process.platform === 'win32')
  await cp(process.execPath, join(runtime, 'node.exe'))
else await symlink(process.execPath, join(runtime, 'node'))
const consumerEnv = { ...process.env, PATH: runtime }

// A local read-only registry exercises the final root optional dependency, with
// an empty npm cache and no source-tree/native-path override. Other targets are
// intentionally unavailable, as in a partial release; the selected one is required.
let registryUrl = ''
const requests: string[] = []
const server = createServer((request, response) => {
  const path = decodeURIComponent(request.url || '/')
  requests.push(path)
  if (path === `/@gdfmt/${target.id}`) {
    const version = {
      ...nativeMetadata,
      dist: {
        tarball: `${registryUrl}/native.tgz`,
        integrity: `sha512-${createHash('sha512').update(nativeBytes).digest('base64')}`,
      },
    }
    response.setHeader('content-type', 'application/json')
    response.end(
      JSON.stringify({
        name: nativeMetadata.name,
        'dist-tags': { latest: metadata.version },
        versions: { [metadata.version]: version },
      }),
    )
  } else if (path === '/native.tgz') response.end(nativeBytes)
  else {
    response.statusCode = 404
    response.end('{"error":"not found"}')
  }
})
await new Promise<void>(done => server.listen(0, '127.0.0.1', done))
const address = server.address()
assert.ok(address && typeof address === 'object')
registryUrl = `http://127.0.0.1:${address.port}`

function asyncRun(
  command: string,
  args: string[],
  options: SpawnOptions,
): Promise<void> {
  return new Promise<void>((done, reject) => {
    const child = spawn(command, args, { stdio: 'inherit', ...options })
    child.once('error', reject)
    child.once('exit', code =>
      code === 0 ? done() : reject(new Error(`${command} exited with ${code}`)),
    )
  })
}
try {
  // npm uses the current Node runtime's installed CLI, not any build toolchain.
  const npmPath =
    process.env.GDFMT_NPM_CLI ||
    (process.platform === 'win32'
      ? join(dirname(process.execPath), 'node_modules/npm/bin/npm-cli.js')
      : join(
          resolve(process.execPath, '../..'),
          'lib/node_modules/npm/bin/npm-cli.js',
        ))
  const npmArgs = [
    'install',
    rootTarball,
    '--ignore-scripts',
    '--no-audit',
    '--no-fund',
    '--registry',
    registryUrl,
    '--cache',
    join(consumer, '.cache'),
  ]
  await asyncRun(process.execPath, [npmPath, ...npmArgs], {
    cwd: consumer,
    env: consumerEnv,
  })
  assert.ok(
    requests.includes('/native.tgz'),
    'native optional dependency must be installed through registry resolution',
  )
  const installed = join(consumer, 'node_modules/gdfmt')
  assert.equal(
    await readFile(join(installed, 'dist/schema.json'), 'utf8'),
    await readFile(join(packageRoot, 'dist/schema.json'), 'utf8'),
  )
  await asyncRun(
    process.execPath,
    [
      fileURLToPath(
        new URL('../../node_modules/vitest/vitest.mjs', import.meta.url),
      ),
      'run',
      '--reporter=default',
      '--reporter=json',
      '--outputFile',
      join(artifacts, 'vitest.json'),
      '--config',
      fileURLToPath(new URL('../../vitest.config.ts', import.meta.url)),
    ],
    {
      env: {
        ...consumerEnv,
        GDFMT_PACKAGE_ROOT: installed,
        GDFMT_STANDALONE: standalone,
      },
      cwd: consumer,
    },
  )
  const testReport: {
    success: boolean
    numTotalTests: number
    numPassedTests: number
    numFailedTests: number
  } = JSON.parse(await readFile(join(artifacts, 'vitest.json'), 'utf8'))
  assert.equal(testReport.success, true)
  assert.equal(testReport.numTotalTests, 8)
  assert.equal(testReport.numPassedTests, 8)
  assert.equal(testReport.numFailedTests, 0)
  // Resolve both declaration branches through the installed export map.
  await cp(
    fileURLToPath(new URL('../../tests/npm/types', import.meta.url)),
    join(consumer, 'types'),
    { recursive: true },
  )
  run(process.execPath, [
    fileURLToPath(
      new URL('../../node_modules/typescript/bin/tsc', import.meta.url),
    ),
    '--project',
    join(consumer, 'types/tsconfig.json'),
    '--typeRoots',
    fileURLToPath(new URL('../../node_modules/@types', import.meta.url)),
  ])
  const summary = {
    schemaVersion: 1,
    revision,
    cleanSource: cleanSource && isCleanReleaseSource(),
    target: target.id,
    triple: target.triple,
    node: process.version,
    version: metadata.version,
    rootTarball,
    nativeTarball,
    rootSha512,
    nativeSha512,
    tests: testReport.numPassedTests,
    registryRequests: requests,
    passed: true,
    published: false,
  }
  assert.equal(
    gitRevision(),
    revision,
    'source commit changed while building the npm artifacts',
  )
  assert.equal(
    createHash('sha512')
      .update(await readFile(rootTarball))
      .digest('hex'),
    rootSha512,
    'root artifact changed during testing',
  )
  assert.equal(
    createHash('sha512')
      .update(await readFile(nativeTarball))
      .digest('hex'),
    nativeSha512,
    'native artifact changed during testing',
  )
  await writeFile(
    join(artifacts, 'acceptance.json'),
    `${JSON.stringify(summary, null, 2)}\n`,
  )
  console.log(
    `Packaged ${target.id} consumer, ESM/CJS declarations and CLI checks passed; no publication performed.`,
  )
} finally {
  await new Promise<void>((done, reject) =>
    server.close(error => (error ? reject(error) : done())),
  )
}
