import assert from 'node:assert/strict'
import { spawnSync } from 'node:child_process'
import { cp, mkdtemp, readFile, rm, writeFile, mkdir } from 'node:fs/promises'
import { createRequire } from 'node:module'
import { tmpdir } from 'node:os'
import { dirname, join, resolve } from 'node:path'
import { fileURLToPath, pathToFileURL } from 'node:url'
import { test } from 'vitest'
import packageManifest from '../../packages/gdfmt/package.json' with { type: 'json' }
import type * as Gdfmt from '../../packages/gdfmt/src/index.cjs'
import targets from '../../packages/gdfmt/src/targets.json' with { type: 'json' }
import manifest from '../compatibility/manifest.json' with { type: 'json' }

function requiredEnv(name: string): string {
  const value = process.env[name]
  assert.ok(
    value,
    `${name} is required; run pnpm test:npm to test installed tarballs`,
  )
  return value
}

function parseLanguageVersion(value: string): Gdfmt.LanguageVersion {
  switch (value) {
    case '4.3':
    case '4.4':
    case '4.5':
    case '4.6':
    case '4.7':
      return value
    default:
      throw new Error(`unsupported manifest language: ${value}`)
  }
}

const root = resolve(dirname(fileURLToPath(import.meta.url)), '../..')
const installed = requiredEnv('GDFMT_PACKAGE_ROOT')
const consumer = requiredEnv('GDFMT_CONSUMER_ROOT')
const packageName = packageManifest.name
const consumerRequire = createRequire(join(installed, 'package.json'))
const cjs: typeof Gdfmt = consumerRequire(packageName)
const esm: typeof Gdfmt = await import(
  pathToFileURL(consumerRequire.resolve(packageName)).href.replace(
    /index\.cjs$/,
    'index.mjs',
  )
)
const cliPath = join(installed, 'dist/cli.cjs')
const standalone = requiredEnv('GDFMT_STANDALONE')
const packageMetadata: { version: string } = consumerRequire(
  `${packageName}/package.json`,
)

function execute(
  isNode: boolean,
  args: string[],
  cwd: string,
  input: string | Uint8Array = '',
) {
  const result = spawnSync(
    isNode ? process.execPath : standalone,
    isNode ? [cliPath, ...args] : args,
    {
      cwd,
      input,
      encoding: 'utf8',
      maxBuffer: 32 * 1024 * 1024,
    },
  )
  if (result.error) throw result.error
  assert.equal(result.signal, null)
  return { status: result.status, stdout: result.stdout, stderr: result.stderr }
}

async function temporary(
  run: (directory: string) => Promise<void>,
): Promise<void> {
  const directory = await mkdtemp(join(tmpdir(), 'gdfmt-npm-'))
  try {
    await run(directory)
  } finally {
    await rm(directory, { recursive: true, force: true })
  }
}

test('installed ESM/CJS export branches share the same public functions and schema', async () => {
  assert.deepEqual(Object.keys(cjs).sort(), [
    'format',
    'formatSync',
    'resolveConfig',
    'resolveConfigSync',
  ])
  assert.deepEqual(Object.keys(esm).sort(), Object.keys(cjs).sort())
  for (const key of [
    'format',
    'formatSync',
    'resolveConfig',
    'resolveConfigSync',
  ] as const)
    assert.equal(cjs[key], esm[key])
  // This test file lives outside the consumer; the child resolves its public map.
  const child = spawnSync(
    process.execPath,
    [
      '--input-type=module',
      '-e',
      `import {formatSync} from ${JSON.stringify(packageName)}; import {createRequire} from 'node:module'; const r=createRequire(import.meta.url); if(formatSync!==r(${JSON.stringify(packageName)}).formatSync) process.exit(1)`,
    ],
    { cwd: consumer, encoding: 'utf8' },
  )
  assert.equal(child.status, 0, child.stderr)
  assert.deepEqual(
    consumerRequire(`${packageName}/schema.json`),
    JSON.parse(
      await readFile(join(root, 'schemas/gdfmtrc.schema.json'), 'utf8'),
    ),
  )
  assert.equal(
    execute(true, ['--version'], root).stdout.trim(),
    `gdfmt ${packageMetadata.version}`,
  )
})

test('sync/async results match Rust on every script fixture and all five language profiles', async () => {
  const fixtures = manifest.fixtures.filter(item => item.stage === 'script')
  assert.ok(
    fixtures.length > 0,
    'compatibility manifest must contain script fixtures',
  )
  await temporary(async directory => {
    await writeFile(join(directory, '.gdfmtrc.json'), '{"root":true}\n')
    const paths: string[] = []
    const sources = new Map<string, string>()
    for (const fixture of fixtures) {
      const name = fixture.file.replace('fixtures/', '')
      const source = await readFile(
        join(root, 'tests/compatibility', fixture.file),
        'utf8',
      )
      const path = join(directory, name)
      paths.push(path)
      sources.set(path, source)
    }
    for (const godotVersion of manifest.languageProfiles) {
      // The runtime manifest is checked by Rust; this boundary narrows its profile.
      const language = parseLanguageVersion(godotVersion)
      const options = { language: { godotVersion: language } }
      await writeFile(
        join(directory, '.gdfmtrc.json'),
        JSON.stringify({ root: true, ...options }),
      )
      for (const [path, source] of sources) await writeFile(path, source)
      const args = ['--reporter=json', '--check', ...paths]
      const rust = execute(false, args, directory)
      assert.deepEqual(execute(true, args, directory), rust)
      const report: {
        files: {
          path: string
          sourceId: number
          changed: boolean
          diagnostics: Gdfmt.Diagnostic[]
        }[]
      } = JSON.parse(rust.stdout)
      assert.equal(report.files.length, fixtures.length)
      const results = new Map<
        string,
        { result: Gdfmt.FormatResult; source: string }
      >()
      for (const file of report.files) {
        const request = { ...options, sourceId: file.sourceId }
        const source = await readFile(file.path, 'utf8')
        const sync = cjs.formatSync(source, request)
        assert.deepEqual(
          await esm.format(source, request),
          sync,
          `${godotVersion} ${file.path}`,
        )
        assert.equal(sync.spanEncoding, 'utf8Bytes')
        assert.equal(sync.changed, file.changed)
        assert.deepEqual(sync.diagnostics, file.diagnostics)
        results.set(file.path, { result: sync, source })
      }
      // The standalone Rust facade supplies independent final output evidence.
      const written = execute(false, ['--reporter=json', ...paths], directory)
      assert.ok(written.status === 0 || written.status === 1, written.stdout)
      for (const [path, { result, source }] of results) {
        assert.equal(
          await readFile(path, 'utf8'),
          result.output ?? source,
          path,
        )
        if (result.output !== null)
          assert.equal(cjs.formatSync(result.output, options).changed, false)
      }
    }
  })
})

test('Unicode/BOM/CRLF, protected regions and layout options agree with stdin CLI', async () => {
  const sources = [
    '\uFEFF# 中文 😀\r\nvar 标识="😀"\r\n',
    '# gdfmt: formatter-off\r\nvar x=  [1,2]\r\n# gdfmt: formatter-on\r\nvar y=3\r\n',
    'func f():\n  var é="😀";var x=1\n',
    'var items=[1,2,] # values\nvar data={name=1}\nenum E {A,B}\nfunc f(): return func(x): return x+1\n',
    'var items=[\n  1,2\n]\n## docs\n@warning_ignore("unused_parameter")\nfunc f(value): pass\nclass Inner:\n  func g(): pass\n',
    '# 😀 中文\nvar x = )\n',
    'var x = "😀"; var y = )\r\n',
    'var data=[{key=func(): pass}] # tail\n',
    'var callbacks=[func(): pass\n# before comma\n,]\n',
    '# gdfmt: formatter-off\nvar x = )\n# gdfmt: formatter-on\n',
  ]
  const layouts: Gdfmt.FormatOptions[] = [
    {},
    {
      arrayLayout: 'multiline',
      dictionaryLayout: 'multiline',
      enumLayout: 'multiline',
      dictionarySpacing: 'spaced',
      trailingCommaLayout: 'multiline',
      blankLinesAroundDefinitions: 2,
      lambdaBodyStyle: 'multiline',
      inlineCommentSpacing: 4,
    },
    {
      arrayLayout: 'preserve',
      dictionaryLayout: 'preserve',
      enumLayout: 'preserve',
      dictionarySpacing: 'spaced',
      trailingCommaLayout: 'auto',
      blankLinesAroundDefinitions: 0,
      maxBlankLines: 5,
      inlineCommentSpacing: 2,
    },
    {
      indentStyle: 'space',
      indentWidth: 2,
      lineWidth: 40,
      lineEnding: 'crlf',
      maxBlankLines: 0,
      finalNewline: false,
    },
    {
      lineEnding: 'preserve',
      lineWidth: 320,
      indentWidth: 8,
      maxBlankLines: 5,
    },
  ]
  await temporary(async directory => {
    for (const format of layouts) {
      await writeFile(
        join(directory, '.gdfmtrc.json'),
        JSON.stringify({ root: true, format }),
      )
      for (const source of sources) {
        const sync = cjs.formatSync(source, { format })
        assert.deepEqual(await esm.format(source, { format }), sync)
        if (!source.includes('= )')) {
          assert.notEqual(sync.output, null, JSON.stringify(sync.diagnostics))
          if (sync.output !== null)
            assert.equal(cjs.formatSync(sync.output, { format }).changed, false)
        }
        const args = ['--reporter=json', '--stdin-filepath', 'virtual.gd']
        const rust = execute(false, args, directory, source)
        assert.deepEqual(execute(true, args, directory, source), rust)
        const file = JSON.parse(rust.stdout).files[0]
        assert.deepEqual(file.diagnostics, sync.diagnostics)
        assert.equal(file.output ?? null, sync.output)
        if (source.includes('var y = )')) {
          const offset = Buffer.from(source).indexOf(Buffer.from(')'))
          assert.ok(sync.diagnostics.some(item => item.span.start === offset))
          assert.notEqual(offset, source.indexOf(')'))
        }
      }
    }
  })
})

test('strict invalid options throw synchronously and reject asynchronously without source output', async () => {
  const cases = [
    null,
    [],
    { format: null },
    { format: { lineWidth: 39 } },
    { format: { lineWidth: 40.5 } },
    { format: { indentWidth: 9 } },
    { format: { maxBlankLines: 6 } },
    { format: { finalNewline: 0 } },
    { format: { lineEnding: 'auto' } },
    { format: { arrayLayout: 'compact' } },
    { format: { dictionaryLayout: null } },
    { format: { enumLayout: true } },
    { format: { dictionarySpacing: 'auto' } },
    { format: { trailingCommaLayout: 'none' } },
    { format: { blankLinesAroundDefinitions: 6 } },
    { format: { blankLinesAroundDefinitions: -1 } },
    { format: { blankLinesAroundDefinitions: 1.5 } },
    { format: { lambdaBodyStyle: 'inline' } },
    { format: { inlineCommentSpacing: 0 } },
    { format: { inlineCommentSpacing: 5 } },
    { language: { godotVersion: '4.8' } },
    { unknown: true },
    { extends: [] },
    { root: true },
    { sourceId: -1 },
    { sourceId: 2 ** 32 },
    { sourceId: 1.5 },
    { format: { lineWidth: NaN } },
    { format: undefined },
    { format: { lineWidth: Infinity } },
    { sourceId: 1n },
    { format: new Date() },
    { [Symbol('hidden')]: true },
  ]
  for (const options of cases) {
    assert.throws(() =>
      Reflect.apply(cjs.formatSync, undefined, ['var x=1\n', options]),
    )
    await assert.rejects(
      Reflect.apply(esm.format, undefined, ['var x=1\n', options]),
    )
  }
  for (const source of [null, 42, '\ud800']) {
    assert.throws(() => Reflect.apply(cjs.formatSync, undefined, [source]))
    await assert.rejects(Reflect.apply(esm.format, undefined, [source]))
  }
  const source = '# 中文 😀\nvar x = )\n'
  const invalid = await esm.format(source, { sourceId: 4294967295 })
  assert.equal(invalid.output, null)
  assert.equal(invalid.changed, false)
  assert.ok(invalid.diagnostics.every(item => item.sourceId === 4294967295))
})

test('explicit sync/async config resolution agrees with explain-config and keeps text calls pure', async () => {
  await temporary(async directory => {
    await mkdir(join(directory, 'nested'))
    await writeFile(
      join(directory, '.editorconfig'),
      'root=true\n[*]\nindent_style=space\nindent_size=2\n',
    )
    await writeFile(
      join(directory, 'base.json'),
      '{"format":{"lineWidth":80,"arrayLayout":"multiline","enumLayout":"preserve"}}',
    )
    await writeFile(
      join(directory, '.gdfmtrc.json'),
      JSON.stringify({
        root: true,
        extends: ['base.json'],
        ignore: ['ignored.gd'],
        overrides: [
          {
            files: ['nested/*.gd'],
            format: {
              lineWidth: 40,
              finalNewline: false,
              arrayLayout: 'auto',
              dictionaryLayout: 'multiline',
              dictionarySpacing: 'spaced',
              trailingCommaLayout: 'multiline',
              blankLinesAroundDefinitions: 0,
              lambdaBodyStyle: 'multiline',
              inlineCommentSpacing: 3,
            },
          },
        ],
      }),
    )
    const path = 'nested/virtual.gd'
    const sync = cjs.resolveConfigSync(path, { cwd: directory })
    assert.deepEqual(await esm.resolveConfig(path, { cwd: directory }), sync)
    const args = ['explain-config', path, '--reporter=json']
    const rust = execute(false, args, directory)
    assert.deepEqual(execute(true, args, directory), rust)
    assert.deepEqual(JSON.parse(rust.stdout).configuration, sync)
    assert.equal(sync.config.format.lineWidth, 40)
    assert.equal(sync.config.format.indentWidth, 2)
    assert.equal(sync.config.format.finalNewline, false)
    assert.equal(sync.config.format.arrayLayout, 'auto')
    assert.equal(sync.config.format.enumLayout, 'preserve')
    assert.equal(sync.config.format.dictionaryLayout, 'multiline')
    assert.equal(sync.config.format.blankLinesAroundDefinitions, 0)
    assert.ok(sync.sources['format.enumLayout']?.endsWith('base.json'))
    for (const field of [
      'arrayLayout',
      'dictionaryLayout',
      'dictionarySpacing',
      'trailingCommaLayout',
      'blankLinesAroundDefinitions',
      'lambdaBodyStyle',
      'inlineCommentSpacing',
    ])
      assert.ok(
        sync.sources[`format.${field}`]?.endsWith('.gdfmtrc.json#overrides[0]'),
      )
    assert.equal(
      cjs.resolveConfigSync('ignored.gd', { cwd: directory }).ignored,
      true,
    )
    const explicit = { cwd: directory, config: 'base.json' }
    assert.deepEqual(
      await esm.resolveConfig(path, explicit),
      cjs.resolveConfigSync(path, explicit),
    )
    assert.equal(
      cjs.resolveConfigSync(path, explicit).config.format.lineWidth,
      80,
    )
    const source = 'func f():\n    var x=1\n'
    assert.ok(cjs.formatSync(source).output?.includes('\tvar x = 1\n'))
    for (const options of [
      { cwd: 'relative' },
      { cwd: null },
      { config: null },
      { nope: true },
    ]) {
      assert.throws(() =>
        Reflect.apply(cjs.resolveConfigSync, undefined, [path, options]),
      )
      await assert.rejects(
        Reflect.apply(esm.resolveConfig, undefined, [path, options]),
      )
    }
    await writeFile(
      join(directory, 'broken.json'),
      '{"format":{"lineWidth":0}}',
    )
    assert.throws(() =>
      cjs.resolveConfigSync(path, { cwd: directory, config: 'broken.json' }),
    )
    await assert.rejects(
      esm.resolveConfig(path, { cwd: directory, config: 'broken.json' }),
    )
  })
})

test('npm CLI has byte-identical stdout/stderr, exit codes, writes and host failures', async () => {
  await temporary(async directory => {
    await writeFile(
      join(directory, '.gdfmtrc.json'),
      '{"root":true,"ignore":["ignored.gd"]}',
    )
    const source = 'var x=1\n'
    await writeFile(join(directory, 'file.gd'), source)
    await writeFile(join(directory, 'invalid.gd'), 'var x = )\n')
    await writeFile(join(directory, 'ignored.gd'), source)
    const cases = [
      [],
      ['--help'],
      ['--version'],
      ['schema'],
      ['lex', 'file.gd'],
      ['check', 'file.gd'],
      ['--check', '--reporter=json', 'file.gd'],
      ['--diff', 'file.gd'],
      ['check', '--syntax-only', 'invalid.gd'],
      ['--reporter=json', 'missing.gd'],
      ['--reporter=json', 'ignored.gd'],
      ['--reporter=json', '--check', '--diff'],
      ['--config', 'missing.json', '--reporter=json', 'file.gd'],
      ['--stdin-filepath', 'file.gd'],
      ['--stdin-filepath', 'file.gd', '--check'],
      ['--reporter=json', '--stdin-filepath', 'ignored.gd'],
      ['--reporter=json', '--stdin-filepath', 'virtual.txt'],
      ['unknown-command', '--reporter=json'],
    ]
    for (const args of cases) {
      // Default invocation writes the directory; restore all sources for each runner.
      const originals = {
        'file.gd': source,
        'invalid.gd': 'var x = )\n',
        'ignored.gd': source,
      }
      for (const [path, text] of Object.entries(originals))
        await writeFile(join(directory, path), text)
      const rust = execute(false, args, directory, source)
      const after = await readFile(join(directory, 'file.gd'), 'utf8')
      for (const [path, text] of Object.entries(originals))
        await writeFile(join(directory, path), text)
      assert.deepEqual(
        execute(true, args, directory, source),
        rust,
        args.join(' '),
      )
      assert.equal(await readFile(join(directory, 'file.gd'), 'utf8'), after)
    }
    const bytes = Buffer.from([0xff])
    assert.deepEqual(
      execute(true, ['--reporter=json', '-'], directory, bytes),
      execute(false, ['--reporter=json', '-'], directory, bytes),
    )
    // Actual installed bin mapping and executable bit/shebang are also exercised.
    const bin = join(
      consumer,
      'node_modules/.bin',
      process.platform === 'win32' ? 'gdfmt.cmd' : 'gdfmt',
    )
    const invoked = spawnSync(bin, ['--version'], {
      encoding: 'utf8',
      shell: process.platform === 'win32',
    })
    assert.equal(invoked.status, 0, invoked.stderr)
    assert.equal(
      invoked.stdout,
      execute(false, ['--version'], directory).stdout,
    )
  })
})

test('async formatting queues CPU work on libuv and remains independent across concurrent calls', async () => {
  const code = `
    const assert = require('node:assert/strict');
    const {pbkdf2} = require('node:crypto');
    const {format} = require(${JSON.stringify(installed)});
    let released = false;
    const blocker = new Promise((resolve, reject) => pbkdf2('x','y',1000000,32,'sha256',error => {
      if(error) reject(error); else { released=true; resolve(); }
    }));
    let settled = false;
    const pending = format('var x=1\\n').then(result => { assert.ok(released); settled=true; return result; });
    setImmediate(async () => {
      assert.equal(settled,false);
      await blocker;
      assert.equal((await pending).output, 'var x = 1\\n');
    });
  `
  const child = spawnSync(process.execPath, ['-e', code], {
    env: { ...process.env, UV_THREADPOOL_SIZE: '1' },
    encoding: 'utf8',
  })
  assert.equal(child.status, 0, child.stderr)
  const sources = Array.from({ length: 24 }, (_, index) => `var x=${index}\n`)
  assert.deepEqual(
    await Promise.all(sources.map(source => esm.format(source))),
    sources.map(source => cjs.formatSync(source)),
  )
})

test('loader rejects missing, mismatched and corrupt platform packages without falling back', async () => {
  await temporary(async directory => {
    const copy = join(directory, 'node_modules', packageName)
    await cp(installed, copy, { recursive: true })
    const header: unknown = process.report.getReport()
    const reportText = JSON.stringify(header)
    const target = targets.find(
      item =>
        item.os === process.platform &&
        item.cpu === process.arch &&
        (process.platform !== 'linux' ||
          item.libc ===
            (reportText.includes('"glibcVersionRuntime"') ? 'glibc' : 'musl')),
    )
    assert.ok(target)
    const originalNative = consumerRequire.resolve(
      `@gdfmt/${target.id}/package.json`,
    )
    const nativeMetadata = JSON.parse(await readFile(originalNative, 'utf8'))
    const nativeDirectory = join(directory, 'node_modules', nativeMetadata.name)
    const invoke = () =>
      spawnSync(process.execPath, ['-e', `require(${JSON.stringify(copy)})`], {
        encoding: 'utf8',
      })
    assert.notEqual(invoke().status, 0)
    await mkdir(nativeDirectory, { recursive: true })
    await writeFile(
      join(nativeDirectory, 'package.json'),
      JSON.stringify({ ...nativeMetadata, version: '0.0.0' }),
    )
    assert.match(invoke().stderr, /expected .*found/)
    await writeFile(
      join(nativeDirectory, 'package.json'),
      JSON.stringify(nativeMetadata),
    )
    await writeFile(
      join(nativeDirectory, 'gdfmt.node'),
      'invalid native binary',
    )
    assert.notEqual(invoke().status, 0)
    const unsupported = spawnSync(
      process.execPath,
      [
        '-e',
        `Object.defineProperty(process, 'arch', {value:'unsupported'}); require(${JSON.stringify(copy)})`,
      ],
      { encoding: 'utf8' },
    )
    assert.match(unsupported.stderr, /unsupported native target/)
  })
})
