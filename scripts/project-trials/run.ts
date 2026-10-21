import assert from 'node:assert/strict'
import { spawnSync } from 'node:child_process'
import { createHash } from 'node:crypto'
import { existsSync } from 'node:fs'
import {
  mkdir,
  mkdtemp,
  readFile,
  readdir,
  rm,
  writeFile,
} from 'node:fs/promises'
import { tmpdir } from 'node:os'
import { dirname, isAbsolute, resolve, sep, win32 } from 'node:path'
import { fileURLToPath, pathToFileURL } from 'node:url'
import { isDeepStrictEqual, parseArgs } from 'node:util'

interface SourceFile {
  path: string
  sha256: string
}

interface SourceRejection {
  diagnostics: string[]
  engineError: string
}

interface GdtoolkitRejection {
  exception: string
  message: string
  line: number
  column: number
  context: string
  expected: string[]
}

export interface Project {
  id: string
  repository: string
  commit: string
  files: SourceFile[]
  sourceRejections: Record<string, SourceRejection>
  gdtoolkitRejections: Record<string, GdtoolkitRejection>
}

interface TrialInput {
  project: string
  path: string
  source: string
  expectedDiagnostics: string[]
}

interface LayoutCase {
  input: TrialInput
  sourceRejected: boolean
  layouts: { passed: boolean; output: string | null }[]
}

export interface TokenFacts {
  tokens: string[]
  strings: string[]
  comments: string[]
}

interface Comparison {
  output: string | null
  rejection: GdtoolkitRejection | null
  gdfmt: TokenFacts | null
  gdtoolkit: TokenFacts | null
}

const repositoryRoot = fileURLToPath(new URL('../../', import.meta.url))
const trialDirectory = resolve(repositoryRoot, 'tests/project-trials')

// gdtoolkit is a Python library. Keep this bridge limited to its parser and
// formatter APIs; orchestration, inventory, classification and reports use TS.
const GDTOOLKIT_BRIDGE = String.raw`
import ast
import importlib.metadata
import json
from pathlib import Path
import re
import sys
from gdtoolkit.formatter import format_code
from gdtoolkit.parser import parser
from lark.exceptions import UnexpectedInput

request = json.load(sys.stdin)
for package, version in re.findall(r"^([\w-]+)==([^\s\\]+)", Path(request["requirements"]).read_text(), re.M):
    installed = importlib.metadata.version(package)
    if installed != version:
        raise RuntimeError(f"{package}: expected {version}, installed {installed}")

def facts(source):
    tokens, strings = [], []
    for token in parser._parser.lex(source):
        if token.type in {"_NL", "_INDENT", "_DEDENT"}:
            continue
        if "STRING" in token.type:
            strings.append(str(token))
            try:
                tokens.append(repr(ast.literal_eval(str(token))))
            except (ValueError, SyntaxError):
                tokens.append(str(token))
        else:
            tokens.extend(re.findall(r"\w+|[^\w\s]", str(token)))
    return {"tokens": tokens, "strings": strings,
            "comments": [str(token) for token in parser.parse_comments(source).children]}

comparisons = []
for case in request["cases"]:
    source = case["source"]
    try:
        output = format_code(source, 100)
    except UnexpectedInput as error:
        rejection = {"exception": type(error).__name__, "message": str(error).splitlines()[0],
                     "line": error.line, "column": error.column, "context": error.get_context(source),
                     "expected": sorted(getattr(error, "expected", None) or getattr(error, "allowed", []))}
        comparisons.append({"output": None, "rejection": rejection, "gdfmt": None, "gdtoolkit": None})
        continue
    comparisons.append({"output": output, "rejection": None,
                        "gdfmt": facts(case["output"]) if case["output"] is not None else None,
                        "gdtoolkit": facts(output)})
json.dump(comparisons, sys.stdout, ensure_ascii=False)
`

function readObject(value: unknown): Record<string, unknown> {
  assert.ok(
    value && typeof value === 'object' && !Array.isArray(value),
    'expected an object',
  )
  return Object.fromEntries(Object.entries(value))
}

function readString(value: unknown): string {
  assert.equal(typeof value, 'string', 'expected a string')
  assert.ok(typeof value === 'string')
  return value
}

function readStrings(value: unknown): string[] {
  assert.ok(Array.isArray(value), 'expected a string array')
  return value.map(readString)
}

function parseRejection(value: unknown): GdtoolkitRejection {
  const diagnostic = readObject(value)
  assert.ok(
    Number.isInteger(diagnostic.line) && typeof diagnostic.line === 'number',
  )
  assert.ok(
    Number.isInteger(diagnostic.column) &&
      typeof diagnostic.column === 'number',
  )
  return {
    exception: readString(diagnostic.exception),
    message: readString(diagnostic.message),
    line: diagnostic.line,
    column: diagnostic.column,
    context: readString(diagnostic.context),
    expected: readStrings(diagnostic.expected),
  }
}

export function parseProject(value: unknown): Project {
  const project = readObject(value)
  const id = readString(project.id)
  assert.match(id, /^[a-z0-9-]+$/, 'invalid project id')
  const repository = readString(project.repository)
  assert.match(
    repository,
    /^https:\/\/github\.com\/[\w.-]+\/[\w.-]+$/,
    'expected a GitHub repository',
  )
  const commit = readString(project.commit)
  assert.match(commit, /^[a-f0-9]{40}$/, 'expected a pinned commit')
  assert.ok(
    Array.isArray(project.files) && project.files.length > 0,
    'empty source inventory',
  )
  const files = project.files.map(entry => {
    const file = readObject(entry)
    const path = readString(file.path)
    assert.ok(
      !isAbsolute(path) &&
        !win32.isAbsolute(path) &&
        !path.split(/[\\/]/).includes('..'),
      `invalid source path: ${path}`,
    )
    const sha256 = readString(file.sha256)
    assert.match(sha256, /^[a-f0-9]{64}$/, 'expected a source SHA-256')
    return { path, sha256 }
  })
  assert.equal(
    new Set(files.map(file => file.path)).size,
    files.length,
    'duplicate source path',
  )
  const sourceRejections = Object.fromEntries(
    Object.entries(readObject(project.sourceRejections ?? {})).map(
      ([path, entry]) => {
        const rejection = readObject(entry)
        return [
          path,
          {
            diagnostics: readStrings(rejection.diagnostics),
            engineError: readString(rejection.engineError),
          },
        ]
      },
    ),
  )
  const gdtoolkitRejections = Object.fromEntries(
    Object.entries(readObject(project.gdtoolkitRejections ?? {})).map(
      ([path, entry]) => [path, parseRejection(entry)],
    ),
  )
  const scripts = new Set(
    files.filter(file => file.path.endsWith('.gd')).map(file => file.path),
  )
  for (const [field, rejections] of Object.entries({
    sourceRejections,
    gdtoolkitRejections,
  })) {
    for (const path of Object.keys(rejections))
      assert.ok(scripts.has(path), `unknown ${field} script: ${id}/${path}`)
  }
  return {
    id,
    repository,
    commit,
    files,
    sourceRejections,
    gdtoolkitRejections,
  }
}

export function digest(source: string | Uint8Array): string {
  return createHash('sha256').update(source).digest('hex')
}

async function writeJson(path: string, value: unknown): Promise<void> {
  await mkdir(dirname(path), { recursive: true })
  await writeFile(path, `${JSON.stringify(value, null, 2)}\n`)
}

export async function prepare(
  project: Project,
  cache: string,
  shouldFetch: boolean,
): Promise<Record<string, string>> {
  const directory = resolve(cache, 'sources', project.id)
  const files: Record<string, string> = {}
  // Bound network concurrency while checking every pinned license and script.
  for (let offset = 0; offset < project.files.length; offset += 8) {
    await Promise.all(
      project.files.slice(offset, offset + 8).map(async entry => {
        const path = resolve(directory, entry.path)
        if (!existsSync(path)) {
          assert.ok(
            shouldFetch,
            `missing ${path}; use --fetch to download pinned sources`,
          )
          const url = `${project.repository.replace('https://github.com/', 'https://raw.githubusercontent.com/')}/${project.commit}/${entry.path.split('/').map(encodeURIComponent).join('/')}`
          const response = await fetch(url, {
            signal: AbortSignal.timeout(30_000),
          })
          assert.ok(response.ok, `download failed (${response.status}): ${url}`)
          const downloaded = new Uint8Array(await response.arrayBuffer())
          assert.equal(
            digest(downloaded),
            entry.sha256,
            `download hash mismatch: ${project.id}/${entry.path}`,
          )
          await mkdir(dirname(path), { recursive: true })
          await writeFile(path, downloaded)
        }
        const source = await readFile(path)
        assert.equal(
          digest(source),
          entry.sha256,
          `cached source hash mismatch: ${path}`,
        )
        files[entry.path] = new TextDecoder('utf-8', {
          fatal: true,
          ignoreBOM: true,
        }).decode(source)
      }),
    )
  }
  const found = (await readdir(directory, { recursive: true }))
    .filter(path => path.endsWith('.gd'))
    .map(path => path.split(sep).join('/'))
    .sort()
  assert.deepEqual(
    found,
    project.files
      .filter(file => file.path.endsWith('.gd'))
      .map(file => file.path)
      .sort(),
    `script inventory mismatch: ${project.id}`,
  )
  return files
}

function parseFacts(value: unknown): TokenFacts {
  const facts = readObject(value)
  return {
    tokens: readStrings(facts.tokens),
    strings: readStrings(facts.strings),
    comments: readStrings(facts.comments),
  }
}

export function compareWithGdtoolkit(
  python: string,
  requirements: string,
  cases: { source: string; output: string | null }[],
): Comparison[] {
  const result = spawnSync(python, ['-c', GDTOOLKIT_BRIDGE], {
    input: JSON.stringify({ requirements, cases }),
    env: { ...process.env, PYTHONUTF8: '1' },
    encoding: 'utf8',
    maxBuffer: 64 * 1024 * 1024,
    timeout: 120_000,
  })
  assert.ifError(result.error)
  assert.equal(result.status, 0, `gdtoolkit bridge failed:\n${result.stderr}`)
  const comparisons: unknown = JSON.parse(result.stdout)
  assert.ok(
    Array.isArray(comparisons) && comparisons.length === cases.length,
    'incomplete gdtoolkit report',
  )
  return comparisons.map(entry => {
    const comparison = readObject(entry)
    return {
      output: comparison.output === null ? null : readString(comparison.output),
      rejection:
        comparison.rejection === null
          ? null
          : parseRejection(comparison.rejection),
      gdfmt: comparison.gdfmt === null ? null : parseFacts(comparison.gdfmt),
      gdtoolkit:
        comparison.gdtoolkit === null ? null : parseFacts(comparison.gdtoolkit),
    }
  })
}

export function checkGdtoolkitRejection(
  actual: GdtoolkitRejection | null,
  expected?: GdtoolkitRejection,
): void {
  if (actual !== null) {
    assert.deepEqual(actual, expected, 'unexpected gdtoolkit rejection')
    return
  }
  assert.equal(
    expected,
    undefined,
    'expected gdtoolkit rejection no longer reproduced',
  )
}

/**
 * Classifies reviewed policy differences; the Rust facade independently checks
 * the original CST, significant spelling, comment anchors and idempotence.
 */
export function classify(
  left: string,
  right: string,
  before: TokenFacts,
  after: TokenFacts,
): string[] {
  if (left === right) return ['identical']
  const categories = new Set(['layout'])
  if (!isDeepStrictEqual(before.tokens, after.tokens)) {
    const punctuation = new Set(['(', ')', ',', ';'])
    const withoutPunctuation = (tokens: string[]): string[] =>
      tokens.filter(token => !punctuation.has(token))
    if (
      !isDeepStrictEqual(
        withoutPunctuation(before.tokens),
        withoutPunctuation(after.tokens),
      )
    ) {
      punctuation.add(':')
      assert.deepEqual(
        withoutPunctuation(before.tokens),
        withoutPunctuation(after.tokens),
        'unexplained token differences',
      )
      categories.add('inner-class-extends')
    }
    categories.add('parentheses-commas-semicolons')
  }
  if (!isDeepStrictEqual(before.strings, after.strings))
    categories.add('quotes')
  if (!isDeepStrictEqual(before.comments, after.comments)) {
    const trimComments = (comments: string[]): string[] =>
      comments.map(comment => comment.replace(/[ \t]+$/, ''))
    assert.deepEqual(
      trimComments(before.comments),
      trimComments(after.comments),
      'unexplained comment text/order difference',
    )
    categories.add('comment-trailing-whitespace')
  }
  return [...categories].sort()
}

async function writeDiff(
  path: string,
  left: string,
  right: string,
): Promise<void> {
  await mkdir(dirname(path), { recursive: true })
  if (left === right) {
    await writeFile(path, '')
    return
  }
  const directory = await mkdtemp(resolve(tmpdir(), 'gdfmt-project-diff-'))
  try {
    const before = resolve(directory, 'gdtoolkit.gd')
    const after = resolve(directory, 'gdfmt.gd')
    await writeFile(before, left)
    await writeFile(after, right)
    const result = spawnSync(
      'git',
      [
        '-c',
        'core.autocrlf=false',
        'diff',
        '--no-index',
        '--no-ext-diff',
        '--no-color',
        '--text',
        '--',
        before,
        after,
      ],
      { encoding: 'utf8', maxBuffer: 8 * 1024 * 1024 },
    )
    assert.ifError(result.error)
    assert.equal(
      result.status,
      1,
      `could not generate comparison diff: ${result.stderr}`,
    )
    const hunk = result.stdout.indexOf('@@ ')
    assert.ok(hunk >= 0, 'missing comparison diff hunks')
    await writeFile(
      path,
      `--- gdtoolkit-4.5.0\n+++ gdfmt\n${result.stdout.slice(hunk)}`,
    )
  } finally {
    await rm(directory, { recursive: true, force: true })
  }
}

async function checkRejection(
  binary: string,
  source: string,
  expected: string,
): Promise<{ exitCode: number; stdout: string; stderr: string }> {
  const directory = await mkdtemp(resolve(tmpdir(), 'gdfmt-project-rejection-'))
  try {
    await writeFile(resolve(directory, 'project.godot'), 'config_version=5\n')
    await writeFile(resolve(directory, 'case.gd'), source)
    const result = spawnSync(
      binary,
      [
        '--headless',
        '--path',
        directory,
        '--check-only',
        '--script',
        'case.gd',
      ],
      { encoding: 'utf8', timeout: 15_000 },
    )
    assert.ifError(result.error)
    assert.ok(
      result.status === 0 || result.status === 1,
      `unexpected engine exit: ${result.status}`,
    )
    const log = result.stdout + result.stderr
    assert.ok(
      log.includes(expected) && log.includes('Failed to load script'),
      `source rejection no longer matches pinned engine: ${log}`,
    )
    return {
      exitCode: result.status,
      stdout: result.stdout,
      stderr: result.stderr,
    }
  } finally {
    await rm(directory, { recursive: true, force: true })
  }
}

function parseLayoutCases(value: unknown, inputs: TrialInput[]): LayoutCase[] {
  assert.ok(
    Array.isArray(value) && value.length === inputs.length,
    'incomplete formatter report',
  )
  return value.map((entry, index) => {
    const record = readObject(entry)
    assert.deepEqual(
      record.input,
      inputs[index],
      'formatter report input mismatch',
    )
    assert.equal(
      record.sourceRejected,
      inputs[index].expectedDiagnostics.length > 0,
    )
    assert.ok(
      Array.isArray(record.layouts) && record.layouts.length === 4,
      'incomplete layout matrix',
    )
    return {
      input: inputs[index],
      sourceRejected: inputs[index].expectedDiagnostics.length > 0,
      layouts: record.layouts.map(entry => {
        const layout = readObject(entry)
        assert.equal(layout.passed, true, 'formatter invariant gate failed')
        const output = layout.output === null ? null : readString(layout.output)
        assert.equal(
          output === null,
          inputs[index].expectedDiagnostics.length > 0,
          'unexpected source rejection',
        )
        return { passed: true, output }
      }),
    }
  })
}

async function main(): Promise<void> {
  const { values } = parseArgs({
    options: {
      fetch: { type: 'boolean', default: false },
      help: { type: 'boolean', short: 'h', default: false },
      cache: { type: 'string' },
      godot: { type: 'string' },
      python: { type: 'string' },
      'record-review': { type: 'boolean', default: false },
    },
  })
  if (values.help) {
    console.log(
      'Usage: pnpm trials --godot <pinned binary> [--fetch] [--cache <directory>] [--python <interpreter>] [--record-review]',
    )
    return
  }
  assert.ok(values.godot, 'project trials require --godot <pinned binary>')
  const cache = resolve(
    values.cache ?? resolve(repositoryRoot, 'target/project-trials'),
  )
  const binary = resolve(values.godot)
  const python = resolve(
    values.python ??
      resolve(
        repositoryRoot,
        'target/project-trials/venv',
        process.platform === 'win32' ? 'Scripts/python.exe' : 'bin/python',
      ),
  )
  const requirements = resolve(trialDirectory, 'requirements.txt')
  compareWithGdtoolkit(python, requirements, [])
  const manifestPath = resolve(trialDirectory, 'manifest.json')
  const manifestBytes = await readFile(manifestPath)
  const manifest = readObject(JSON.parse(manifestBytes.toString('utf8')))
  assert.equal(manifest.schemaVersion, 1)
  assert.equal(manifest.gdtoolkit, '4.5.0')
  assert.ok(Array.isArray(manifest.projects) && manifest.projects.length > 0)
  const projects = manifest.projects.map(parseProject)
  assert.equal(
    new Set(projects.map(project => project.id)).size,
    projects.length,
    'duplicate project id',
  )
  const engines = readObject(
    JSON.parse(
      await readFile(
        resolve(repositoryRoot, 'tests/compatibility/engine.json'),
        'utf8',
      ),
    ),
  ).engines
  assert.ok(Array.isArray(engines))
  const pin = engines
    .map(readObject)
    .find(engine => engine.languageVersion === '4.7')
  assert.ok(pin, 'missing Godot 4.7 engine pin')
  const engine = spawnSync(binary, ['--version'], {
    encoding: 'utf8',
    timeout: 15_000,
  })
  assert.ifError(engine.error)
  assert.equal(engine.status, 0, engine.stderr)
  const version = engine.stdout.trim()
  assert.equal(version, pin.engineVersion, 'Godot version mismatch')
  const inputs: TrialInput[] = []
  const sourceRejections: Record<
    string,
    Awaited<ReturnType<typeof checkRejection>>
  > = {}
  const gdtoolkitRejections: Record<string, GdtoolkitRejection> = {}
  for (const project of projects) {
    const files = await prepare(project, cache, values.fetch)
    for (const [name, diagnostic] of Object.entries(
      project.gdtoolkitRejections,
    ))
      gdtoolkitRejections[`${project.id}/${name}`] = diagnostic
    for (const name of Object.keys(files).sort()) {
      if (!name.endsWith('.gd')) continue
      const source = files[name]
      const rejection = project.sourceRejections[name]
      inputs.push({
        project: project.id,
        path: name,
        source,
        expectedDiagnostics: rejection?.diagnostics ?? [],
      })
      if (rejection)
        sourceRejections[`${project.id}/${name}`] = await checkRejection(
          binary,
          source,
          rejection.engineError,
        )
    }
  }
  await writeJson(resolve(cache, 'inputs.json'), inputs)
  const gate = spawnSync(
    'cargo',
    [
      'run',
      '--locked',
      '-p',
      'xtask',
      '--',
      'project-trials',
      '--input',
      resolve(cache, 'inputs.json'),
      '--report',
      resolve(cache, 'layouts.json'),
    ],
    { cwd: repositoryRoot, encoding: 'utf8', maxBuffer: 8 * 1024 * 1024 },
  )
  assert.ifError(gate.error)
  assert.equal(
    gate.status,
    0,
    `project trial invariant gate failed:\n${gate.stdout}\n${gate.stderr}`,
  )
  process.stdout.write(gate.stdout)
  const cases = parseLayoutCases(
    JSON.parse(await readFile(resolve(cache, 'layouts.json'), 'utf8')),
    inputs,
  )
  const comparisons = compareWithGdtoolkit(
    python,
    requirements,
    cases.map(entry => ({
      source: entry.input.source,
      output: entry.layouts[0].output,
    })),
  )
  const reviewed: Record<
    string,
    {
      gdfmtSha256: string | null
      gdtoolkitSha256: string | null
      categories: string[]
    }
  > = {}
  const comparisonReport = []
  for (const [index, entry] of cases.entries()) {
    const original = entry.input
    const key = `${original.project}/${original.path}`
    const comparison = comparisons[index]
    checkGdtoolkitRejection(comparison.rejection, gdtoolkitRejections[key])
    const output = entry.layouts[0].output
    const other = comparison.output
    let categories: string[]
    if (comparison.rejection !== null) {
      assert.notEqual(output, null, `both formatters rejected ${key}`)
      categories = ['gdtoolkit-rejected-gdfmt-accepted']
    } else if (entry.sourceRejected) {
      categories = ['source-rejected-gdtoolkit-accepted']
    } else {
      assert.ok(
        output !== null &&
          other !== null &&
          comparison.gdfmt &&
          comparison.gdtoolkit,
      )
      categories = classify(
        output,
        other,
        comparison.gdfmt,
        comparison.gdtoolkit,
      )
      await writeDiff(
        resolve(cache, 'diffs', original.project, `${original.path}.diff`),
        other,
        output,
      )
    }
    reviewed[key] = {
      gdfmtSha256: output === null ? null : digest(output),
      gdtoolkitSha256: other === null ? null : digest(other),
      categories,
    }
    comparisonReport.push({
      project: original.project,
      path: original.path,
      gdtoolkitOutput: other,
      gdtoolkitRejection: comparison.rejection,
      categories,
    })
  }
  const review = {
    schemaVersion: 1,
    manifestSha256: digest(manifestBytes),
    cases: reviewed,
  }
  await writeJson(resolve(cache, 'review-candidate.json'), review)
  await writeJson(resolve(cache, 'comparisons.json'), comparisonReport)
  const accepted = cases.filter(entry => !entry.sourceRejected)
  const summary = {
    schemaVersion: 1,
    languageVersion: '4.7',
    gdtoolkitVersion: manifest.gdtoolkit,
    engineVersion: version,
    manifestSha256: review.manifestSha256,
    scripts: cases.length,
    acceptedScripts: accepted.length,
    invariantLayouts: accepted.reduce(
      (count, entry) => count + entry.layouts.length,
      0,
    ),
    sourceRejections,
    gdtoolkitRejections,
    identicalToGdtoolkit: Object.values(reviewed).filter(entry =>
      isDeepStrictEqual(entry.categories, ['identical']),
    ).length,
    differentFromGdtoolkit: Object.entries(reviewed).filter(
      ([key, entry]) =>
        !isDeepStrictEqual(entry.categories, ['identical']) &&
        !(key in sourceRejections) &&
        !(key in gdtoolkitRejections),
    ).length,
    unexplainedDifferences: 0,
  }
  await writeJson(resolve(cache, 'summary.json'), summary)
  const reviewPath = resolve(trialDirectory, 'reviewed.json')
  if (values['record-review']) await writeJson(reviewPath, review)
  else
    assert.deepEqual(
      review,
      JSON.parse(await readFile(reviewPath, 'utf8')),
      `reviewed fingerprints changed; inspect ${resolve(cache, 'review-candidate.json')} and diffs before --record-review`,
    )
  console.log(
    JSON.stringify(
      {
        ...summary,
        sourceRejections: Object.keys(sourceRejections).length,
        gdtoolkitRejections: Object.keys(gdtoolkitRejections).length,
      },
      null,
      2,
    ),
  )
}

if (
  process.argv[1] &&
  import.meta.url === pathToFileURL(resolve(process.argv[1])).href
) {
  await main()
}
