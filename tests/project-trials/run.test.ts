import assert from 'node:assert/strict'
import { spawnSync } from 'node:child_process'
import { existsSync } from 'node:fs'
import { mkdir, mkdtemp, readFile, rm, writeFile } from 'node:fs/promises'
import { tmpdir } from 'node:os'
import { resolve } from 'node:path'
import { test } from 'node:test'
import { fileURLToPath } from 'node:url'
import {
  checkGdtoolkitRejection,
  classify,
  compareWithGdtoolkit,
  digest,
  parseProject,
  prepare,
  type Project,
  type TokenFacts,
} from '../../scripts/project-trials/run.ts'

function project(overrides: Record<string, unknown> = {}): Project {
  return parseProject({
    id: 'example',
    repository: 'https://github.com/example/project',
    commit: 'a'.repeat(40),
    files: [{ path: 'main.gd', sha256: digest('var value = 1\n') }],
    sourceRejections: {},
    gdtoolkitRejections: {},
    ...overrides,
  })
}

async function withCache(
  run: (cache: string, source: string) => Promise<void>,
): Promise<void> {
  const cache = await mkdtemp(resolve(tmpdir(), 'gdfmt-trial-test-'))
  const source = resolve(cache, 'sources/example/main.gd')
  try {
    await mkdir(resolve(cache, 'sources/example'), { recursive: true })
    await writeFile(source, 'var value = 1\n')
    await run(cache, source)
  } finally {
    await rm(cache, { recursive: true, force: true })
  }
}

test('modified cached source fails before any formatter runs', async () => {
  await withCache(async (cache, source) => {
    await writeFile(source, 'var value = 2\n')
    await assert.rejects(
      prepare(project(), cache, false),
      /cached source hash mismatch/,
    )
  })
})

test('cache preserves BOM and CRLF bytes and rejects extra scripts', async () => {
  await withCache(async (cache, source) => {
    const text = '\ufeffvar value = 1\r\n'
    await writeFile(source, text)
    const pinned = project({
      files: [{ path: 'main.gd', sha256: digest(text) }],
    })
    assert.equal((await prepare(pinned, cache, false))['main.gd'], text)
    await writeFile(resolve(cache, 'sources/example/extra.gd'), 'pass\n')
    await assert.rejects(
      prepare(pinned, cache, false),
      /script inventory mismatch/,
    )
  })
})

test('offline cache misses never download implicitly', async () => {
  await withCache(async (cache, source) => {
    await rm(source)
    await assert.rejects(prepare(project(), cache, false), /use --fetch/)
  })
})

test('downloads are hash checked before being cached', async context => {
  await withCache(async (cache, source) => {
    await rm(source)
    context.mock.method(
      globalThis,
      'fetch',
      async () => new Response('var value = 2\n'),
    )
    await assert.rejects(
      prepare(project(), cache, true),
      /download hash mismatch/,
    )
    assert.equal(existsSync(source), false)
    context.mock.restoreAll()
    context.mock.method(
      globalThis,
      'fetch',
      async () => new Response('var value = 1\n'),
    )
    assert.equal(
      (await prepare(project(), cache, true))['main.gd'],
      'var value = 1\n',
    )
  })
})

test('CLI help works without a Python environment or Godot binary', () => {
  const runner = fileURLToPath(
    new URL('../../scripts/project-trials/run.ts', import.meta.url),
  )
  const help = spawnSync(
    process.execPath,
    ['--import', 'tsx', runner, '--help'],
    { encoding: 'utf8' },
  )
  assert.ifError(help.error)
  assert.equal(help.status, 0, help.stderr)
  assert.match(help.stdout, /--godot/)
  assert.match(help.stdout, /--python/)
  assert.match(help.stdout, /--record-review/)
})

test('manifest rejects traversal, duplicate paths and unpinned inputs', () => {
  for (const path of [
    '/main.gd',
    '../main.gd',
    'a/../main.gd',
    'C:\\main.gd',
    '..\\main.gd',
  ]) {
    assert.throws(
      () => project({ files: [{ path, sha256: digest('') }] }),
      /invalid source path/,
    )
  }
  const file = { path: 'main.gd', sha256: digest('') }
  assert.throws(() => project({ files: [file, file] }), /duplicate source path/)
  assert.throws(() => project({ commit: 'main' }), /pinned commit/)
  assert.throws(
    () => project({ files: [{ ...file, sha256: 'unchecked' }] }),
    /SHA-256/,
  )
})

test('stale source and comparator rejection paths fail inventory checks', () => {
  assert.throws(
    () =>
      project({
        sourceRejections: {
          'missing.gd': { diagnostics: ['LEX005'], engineError: 'expected' },
        },
      }),
    /unknown sourceRejections script/,
  )
  assert.throws(
    () =>
      project({
        gdtoolkitRejections: {
          'missing.gd': {
            exception: 'UnexpectedToken',
            message: 'expected',
            line: 1,
            column: 1,
            context: '^',
            expected: [],
          },
        },
      }),
    /unknown gdtoolkitRejections script/,
  )
})

const facts: TokenFacts = {
  tokens: ['var', 'value', '=', '1'],
  strings: [],
  comments: ['# first'],
}

test('review rejects changed code, removed comments and reordered comments', () => {
  assert.throws(
    () =>
      classify('left', 'right', facts, {
        ...facts,
        tokens: ['var', 'value', '=', '2'],
      }),
    /unexplained token/,
  )
  assert.throws(
    () =>
      classify('left', 'right', facts, { ...facts, comments: ['# second'] }),
    /unexplained comment/,
  )
  assert.throws(
    () => classify('left', 'right', facts, { ...facts, comments: [] }),
    /unexplained comment/,
  )
  assert.throws(
    () =>
      classify(
        'left',
        'right',
        { ...facts, comments: ['# first', '# second'] },
        { ...facts, comments: ['# second', '# first'] },
      ),
    /unexplained comment/,
  )
})

test('review explains only trailing comment whitespace', () => {
  assert.deepEqual(
    classify(
      'left',
      'right',
      { ...facts, comments: ['# first \t', '# last  '] },
      { ...facts, comments: ['# first', '# last'] },
    ),
    ['comment-trailing-whitespace', 'layout'],
  )
  assert.throws(
    () =>
      classify(
        'left',
        'right',
        { ...facts, comments: ['# first  word'] },
        { ...facts, comments: ['# first word'] },
      ),
    /unexplained comment/,
  )
})

test('quote, punctuation and layout policies remain classified', () => {
  const before = {
    tokens: ['[', "'hello'", ',', ']'],
    strings: ["'hello'"],
    comments: [],
  }
  const after = {
    tokens: ['[', "'hello'", ']'],
    strings: ['"hello"'],
    comments: [],
  }
  assert.deepEqual(classify('left', 'right', before, after), [
    'layout',
    'parentheses-commas-semicolons',
    'quotes',
  ])
  assert.deepEqual(classify('same', 'same', before, before), ['identical'])
  assert.deepEqual(
    classify(
      'left',
      'right',
      { ...facts, tokens: ['class', 'Inner', ':', 'extends', 'Node'] },
      { ...facts, tokens: ['class', 'Inner', 'extends', 'Node', ':'] },
    ),
    ['inner-class-extends', 'layout', 'parentheses-commas-semicolons'],
  )
})

test('colon changes outside inner-class extends headers remain unexplained', () => {
  const before: TokenFacts = {
    tokens: 'var value : int = 1'.split(' '),
    strings: [],
    comments: [],
  }
  assert.throws(
    () =>
      classify('left', 'right', before, {
        ...before,
        tokens: 'var value int = 1'.split(' '),
      }),
    /unexplained token/,
  )
  assert.throws(
    () =>
      classify(
        'left',
        'right',
        {
          ...before,
          tokens: 'class Inner : extends Node var value : int = 1'.split(' '),
        },
        {
          ...before,
          tokens: 'class Inner extends Node : var value int = 1'.split(' '),
        },
      ),
    /unexplained token/,
  )
})

test('inner-class extends review retains body colons and handles qualified bases', () => {
  const tokens =
    'class First : extends Outer . Base var value : int = 1 class Second : extends Node func run ( ) : pass'.split(
      ' ',
    )
  const other =
    'class First extends Outer . Base : var value : int = 1 class Second extends Node : func run ( ) : pass'.split(
      ' ',
    )
  for (const [before, after] of [
    [tokens, other],
    [other, tokens],
  ])
    assert.deepEqual(
      classify(
        'left',
        'right',
        { tokens: before, strings: [], comments: [] },
        { tokens: after, strings: [], comments: [] },
      ),
      ['inner-class-extends', 'layout', 'parentheses-commas-semicolons'],
    )
})

test('gdtoolkit rejections must reproduce every pinned diagnostic field', () => {
  const expected = {
    exception: 'UnexpectedCharacters',
    message: 'BOM',
    line: 1,
    column: 1,
    context: '^',
    expected: ['VAR'],
  }
  checkGdtoolkitRejection(expected, expected)
  checkGdtoolkitRejection(null)
  assert.throws(
    () => checkGdtoolkitRejection(expected),
    /unexpected gdtoolkit rejection/,
  )
  assert.throws(
    () => checkGdtoolkitRejection({ ...expected, column: 2 }, expected),
    /unexpected gdtoolkit rejection/,
  )
  assert.throws(
    () => checkGdtoolkitRejection(null, expected),
    /no longer reproduced/,
  )
})

const python = process.env.GDFMT_TRIAL_PYTHON
test(
  'locked gdtoolkit bridge reproduces BOM diagnostics and extracts comments',
  { skip: !python },
  async () => {
    assert.ok(python)
    const manifest: unknown = JSON.parse(
      await readFile(new URL('./manifest.json', import.meta.url), 'utf8'),
    )
    assert.ok(
      manifest &&
        typeof manifest === 'object' &&
        'projects' in manifest &&
        Array.isArray(manifest.projects),
    )
    const pinned = manifest.projects
      .map(parseProject)
      .find(entry => entry.id === 'godot-statecharts')
    assert.ok(pinned)
    const source = "var value = ['hello',]\n# first  \n"
    const comparisons = compareWithGdtoolkit(
      python,
      fileURLToPath(new URL('./requirements.txt', import.meta.url)),
      [
        {
          source: '\ufeffextends StateChartTestBase\n',
          output: '\ufeffextends StateChartTestBase\n',
        },
        { source, output: source },
      ],
    )
    checkGdtoolkitRejection(
      comparisons[0].rejection,
      pinned.gdtoolkitRejections['tests/test_not_guard.gd'],
    )
    const comparison = comparisons[1]
    assert.ok(comparison.output && comparison.gdfmt && comparison.gdtoolkit)
    assert.deepEqual(
      classify(
        source,
        comparison.output,
        comparison.gdfmt,
        comparison.gdtoolkit,
      ),
      ['comment-trailing-whitespace', 'layout', 'quotes'],
    )
  },
)
