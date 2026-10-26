import assert from 'node:assert/strict'
import { spawnSync } from 'node:child_process'
import { mkdir, mkdtemp, readFile, rm, writeFile } from 'node:fs/promises'
import { cpus, platform, release, tmpdir, totalmem } from 'node:os'
import { basename, dirname, isAbsolute, relative, resolve } from 'node:path'
import { performance } from 'node:perf_hooks'
import { fileURLToPath, pathToFileURL } from 'node:url'
import { parseArgs } from 'node:util'
import { digest, parseProject, prepare } from '../project-trials/run.ts'

interface InputFile {
  path: string
  source: string
}

interface Tool {
  id: string
  binary: string
  args: string[]
}

interface Workload {
  id: string
  files: InputFile[]
}

const repositoryRoot = fileURLToPath(new URL('../../', import.meta.url))

/**
 * Summarizes independent CLI invocations; throughput uses original input bytes.
 */
export function summarize(samplesMs: number[], bytes: number) {
  assert.ok(
    samplesMs.length > 0 &&
      samplesMs.every(ms => Number.isFinite(ms) && ms > 0),
  )
  assert.ok(Number.isFinite(bytes) && bytes >= 0)
  const sorted = [...samplesMs].sort((left, right) => left - right)
  const meanMs = samplesMs.reduce((sum, ms) => sum + ms, 0) / samplesMs.length
  const middle = Math.floor(sorted.length / 2)
  const medianMs =
    sorted.length % 2
      ? sorted[middle]
      : (sorted[middle - 1] + sorted[middle]) / 2
  return {
    samplesMs,
    medianMs,
    meanMs,
    minMs: sorted[0],
    maxMs: sorted.at(-1),
    p95Ms: sorted[Math.ceil(sorted.length * 0.95) - 1],
    standardDeviationMs: Math.sqrt(
      samplesMs.reduce((sum, ms) => sum + (ms - meanMs) ** 2, 0) /
        samplesMs.length,
    ),
    mibPerSecond: bytes / 1024 ** 2 / (medianMs / 1000),
  }
}

/**
 * Generates valid, deliberately unformatted Godot 4 code without external assets.
 */
export function createSyntheticSource(functions: number): string {
  assert.ok(Number.isSafeInteger(functions) && functions > 0)
  return (
    'extends Node\n\n' +
    Array.from({ length: functions }, (_, index) =>
      [
        `# Update actor ${index}.`,
        `func update_actor_${index}(delta:float, enabled:bool=true)->Dictionary:`,
        '\tvar values=[1,2,3,4,5]',
        '\tvar state={"speed":delta*2.0,"active":enabled,"label":"actor"}',
        '\tfor value in values:',
        '\t\tif enabled and value>2:',
        '\t\t\tstate["speed"]+=value',
        '\t\telse:',
        '\t\t\tstate["speed"]-=delta',
        '\treturn state',
        '',
      ].join('\n'),
    ).join('\n')
  )
}

function invoke(tool: Tool, cwd: string, timeout: number, args = ['.']) {
  const start = performance.now()
  const execution = spawnSync(tool.binary, [...tool.args, ...args], {
    cwd,
    // Drain output identically for every tool. Never invoke through a shell.
    stdio: ['ignore', 'pipe', 'pipe'],
    encoding: 'utf8',
    env: { ...process.env, NO_COLOR: '1', PYTHONHASHSEED: '0' },
    timeout,
    maxBuffer: 32 * 1024 * 1024,
  })
  const elapsedMs = performance.now() - start
  assert.ifError(execution.error)
  assert.equal(execution.signal, null, `${tool.id} terminated by signal`)
  return { ...execution, elapsedMs }
}

async function restore(directory: string, files: InputFile[]): Promise<void> {
  await rm(directory, { recursive: true, force: true })
  await mkdir(directory)
  // Flat filenames avoid tool-specific addon/hidden-directory exclusions.
  for (const [index, file] of files.entries())
    await writeFile(resolve(directory, `case_${index}.gd`), file.source)
}

async function fingerprint(directory: string, files: InputFile[]) {
  const outputs = await Promise.all(
    files.map((_, index) => readFile(resolve(directory, `case_${index}.gd`))),
  )
  assert.ok(
    outputs.every((output, index) => output.length > 0 || !files[index].source),
  )
  return {
    sha256: digest(outputs.map(output => digest(output)).join('\n')),
    changedFiles: outputs.filter(
      (output, index) => !output.equals(Buffer.from(files[index].source)),
    ).length,
  }
}

/**
 * Bisects failed batches to record individual rejections without timing failures.
 */
export async function findRejections(
  tool: Tool,
  directory: string,
  files: InputFile[],
  timeout: number,
): Promise<{ path: string; tool: string; diagnostic: string }[]> {
  if (files.length === 0) return []
  await restore(directory, files)
  const execution = invoke(tool, directory, timeout)
  if (execution.status === 0) return []
  if (files.length === 1)
    return [
      {
        path: files[0].path,
        tool: tool.id,
        diagnostic: (execution.stderr + execution.stdout).slice(0, 4000),
      },
    ]
  const middle = Math.floor(files.length / 2)
  return [
    ...(await findRejections(tool, directory, files.slice(0, middle), timeout)),
    ...(await findRejections(tool, directory, files.slice(middle), timeout)),
  ]
}

async function main(): Promise<void> {
  const { values } = parseArgs({
    options: {
      help: { type: 'boolean', short: 'h' },
      fetch: { type: 'boolean', default: false },
      mode: { type: 'string', default: 'write' },
      baseline: { type: 'string' },
      runs: { type: 'string', default: '10' },
      warmup: { type: 'string', default: '2' },
      timeout: { type: 'string', default: '120000' },
      cache: { type: 'string', default: 'target/project-trials' },
      output: { type: 'string', default: 'target/benchmarks/results.json' },
      gdfmt: { type: 'string', default: 'target/release/gdfmt' },
      gdformat: {
        type: 'string',
        default: 'target/project-trials/venv/bin/gdformat',
      },
      gdquest: { type: 'string', default: 'gdscript-formatter' },
      gdstyle: { type: 'string', default: 'gdstyle' },
    },
  })
  if (values.help) {
    console.log(
      'Usage: pnpm bench [--fetch] [--mode write|check] [--baseline <old gdfmt binary>] [--runs 10] [--warmup 2] [--cache <trial cache>] [--output <JSON>] [--gdfmt <binary>] [--gdformat <binary>] [--gdquest <binary>] [--gdstyle <binary>] [--timeout <ms>]',
    )
    return
  }
  const runs = Number(values.runs)
  const warmup = Number(values.warmup)
  const timeout = Number(values.timeout)
  assert.ok(
    values.mode === 'write' || values.mode === 'check',
    'mode must be write or check',
  )
  const mode = values.mode
  assert.ok(Number.isSafeInteger(runs) && runs >= 1, 'runs must be positive')
  assert.ok(
    Number.isSafeInteger(warmup) && warmup >= 0,
    'warmup must be nonnegative',
  )
  assert.ok(
    Number.isSafeInteger(timeout) && timeout > 0,
    'timeout must be positive',
  )
  const binary = (path: string): string =>
    path.includes('/') || path.includes('\\') ? resolve(path) : path
  const tools: Tool[] = [
    { id: 'gdfmt', binary: binary(values.gdfmt), args: [] },
    ...(values.baseline
      ? [{ id: 'gdfmt-before', binary: binary(values.baseline), args: [] }]
      : []),
    {
      id: 'gdformat',
      binary: binary(values.gdformat),
      args: ['--line-length=100'],
    },
    {
      id: 'gdquest',
      binary: binary(values.gdquest),
      args: ['--max-line-length', '100'],
    },
    {
      id: 'gdstyle',
      binary: binary(values.gdstyle),
      args: ['fmt', '--config', '../gdstyle.toml'],
    },
  ]
  const sandbox = await mkdtemp(resolve(tmpdir(), 'gdfmt-benchmark-'))
  const directory = resolve(sandbox, 'work')
  const output = resolve(values.output)
  await mkdir(dirname(output), { recursive: true })
  try {
    await writeFile(
      resolve(sandbox, '.gdfmtrc.json'),
      JSON.stringify({
        language: { godotVersion: '4.7' },
        format: { lineWidth: 100, indentStyle: 'tab' },
      }),
    )
    await writeFile(
      resolve(sandbox, 'gdstyle.toml'),
      'max_line_length = 100\nuse_tabs = true\nexclude = []\n',
    )
    // Stop config discovery at the sandbox boundary for EditorConfig-aware tools.
    await writeFile(resolve(sandbox, '.editorconfig'), 'root = true\n')
    const versions = tools.map(tool => {
      const version = invoke({ ...tool, args: [] }, sandbox, timeout, [
        '--version',
      ])
      assert.equal(version.status, 0, `${tool.id}: ${version.stderr}`)
      return {
        id: tool.id,
        binary: tool.binary,
        args: tool.args,
        version: version.stdout.trim() || version.stderr.trim(),
      }
    })
    const manifestBytes = await readFile(
      resolve(repositoryRoot, 'tests/project-trials/manifest.json'),
    )
    const manifest: unknown = JSON.parse(manifestBytes.toString('utf8'))
    assert.ok(
      manifest &&
        typeof manifest === 'object' &&
        'projects' in manifest &&
        Array.isArray(manifest.projects),
    )
    const projects = manifest.projects.map(parseProject)
    const candidates: InputFile[] = []
    const exclusions: { path: string; tool: string; diagnostic: string }[] = []
    for (const project of projects) {
      const sources = await prepare(
        project,
        resolve(values.cache),
        values.fetch,
      )
      for (const entry of project.files.filter(file =>
        file.path.endsWith('.gd'),
      )) {
        const path = `${project.id}/${entry.path}`
        if (project.sourceRejections[entry.path]) {
          exclusions.push({
            path,
            tool: 'source',
            diagnostic: 'Known invalid upstream source (trial manifest)',
          })
          continue
        }
        candidates.push({ path, source: sources[entry.path] })
      }
    }
    console.log(
      `Preflight: ${candidates.length} valid-source candidates, ${tools.length} tools`,
    )
    for (const tool of tools) {
      console.log(`Checking ${tool.id} acceptance...`)
      exclusions.push(
        ...(await findRejections(tool, directory, candidates, timeout)),
      )
    }
    const excludedPaths = new Set(exclusions.map(entry => entry.path))
    const common = candidates.filter(file => !excludedPaths.has(file.path))
    assert.ok(common.length > 0, 'empty common corpus')
    const workloads: Workload[] = [
      {
        id: 'small-file',
        files: [
          { path: 'synthetic-small.gd', source: createSyntheticSource(5) },
        ],
      },
      {
        id: 'large-file',
        files: [
          { path: 'synthetic-large.gd', source: createSyntheticSource(500) },
        ],
      },
      { id: 'real-projects', files: common },
    ]
    // Prove every adapter actually formats code before accepting any timing.
    for (const tool of tools) {
      await restore(directory, workloads[0].files)
      const execution = invoke(tool, directory, timeout)
      assert.equal(execution.status, 0, `${tool.id}: ${execution.stderr}`)
      assert.ok(
        (await fingerprint(directory, workloads[0].files)).changedFiles > 0,
        `${tool.id} did no work`,
      )
    }
    const git = (args: string[]): string => {
      const execution = spawnSync('git', args, {
        cwd: repositoryRoot,
        encoding: 'utf8',
      })
      assert.equal(execution.status, 0, execution.stderr)
      return execution.stdout.trim()
    }
    const report = {
      schemaVersion: 1,
      timestamp: new Date().toISOString(),
      revision: git(['rev-parse', 'HEAD']),
      dirty: git(['status', '--porcelain']).length > 0,
      trackedDiffSha256: digest(git(['diff', 'HEAD', '--', '.'])),
      harnessSha256: digest(await readFile(fileURLToPath(import.meta.url))),
      trialRunnerSha256: digest(
        await readFile(
          resolve(repositoryRoot, 'scripts/project-trials/run.ts'),
        ),
      ),
      environment: {
        platform: platform(),
        release: release(),
        arch: process.arch,
        cpu: cpus()[0]?.model,
        logicalCpus: cpus().length,
        memoryBytes: totalmem(),
        node: process.version,
      },
      runs,
      warmup,
      timeout,
      mode,
      tools: await Promise.all(
        versions.map(async tool => {
          const path =
            tool.binary.includes('/') || tool.binary.includes('\\')
              ? tool.binary
              : (() => {
                  const found = spawnSync(
                    process.platform === 'win32' ? 'where.exe' : 'which',
                    [tool.binary],
                    { encoding: 'utf8' },
                  )
                  assert.equal(found.status, 0, `cannot resolve ${tool.binary}`)
                  return found.stdout.trim().split(/\r?\n/)[0]
                })()
          const localPath = relative(repositoryRoot, path)
          return {
            ...tool,
            binary:
              localPath.startsWith('..') || isAbsolute(localPath)
                ? basename(path)
                : localPath,
            binarySha256: digest(await readFile(path)),
          }
        }),
      ),
      manifestSha256: digest(manifestBytes),
      projects: projects.map(project => ({
        id: project.id,
        repository: project.repository,
        commit: project.commit,
      })),
      exclusions: exclusions.map(entry => ({
        ...entry,
        diagnostic: entry.diagnostic.split(sandbox).join('<sandbox>'),
      })),
      results: [] as {
        workload: string
        files: number
        bytes: number
        lines: number
        inputs: { path: string; sha256: string }[]
        timings: {
          tool: string
          output: Awaited<ReturnType<typeof fingerprint>>
          statistics: ReturnType<typeof summarize>
        }[]
      }[],
    }
    const save = async (): Promise<void> => {
      await writeFile(output, `${JSON.stringify(report, null, 2)}\n`)
    }
    await save()
    for (const workload of workloads) {
      console.log(
        `${workload.id}: ${workload.files.length} files; ${warmup} warmups + ${runs} measured rounds`,
      )
      const bytes = workload.files.reduce(
        (sum, file) => sum + Buffer.byteLength(file.source),
        0,
      )
      const samples = new Map(tools.map(tool => [tool.id, [] as number[]]))
      const outputs = new Map<string, Awaited<ReturnType<typeof fingerprint>>>()
      // One untimed reference pass also verifies every workload/tool pair.
      for (const tool of tools) {
        await restore(directory, workload.files)
        const execution = invoke(tool, directory, timeout)
        assert.equal(
          execution.status,
          0,
          `${tool.id}/${workload.id}: ${execution.stderr}`,
        )
        outputs.set(tool.id, await fingerprint(directory, workload.files))
      }
      if (values.baseline)
        assert.deepEqual(
          outputs.get('gdfmt'),
          outputs.get('gdfmt-before'),
          'gdfmt output differs from the baseline',
        )
      const original = {
        sha256: digest(
          workload.files.map(file => digest(file.source)).join('\n'),
        ),
        changedFiles: 0,
      }
      for (let round = 0; round < warmup + runs; round++) {
        for (let offset = 0; offset < tools.length; offset++) {
          const tool = tools[(round + offset) % tools.length]
          await restore(directory, workload.files)
          const reference = outputs.get(tool.id)
          assert.ok(reference)
          const execution = invoke(
            tool,
            directory,
            timeout,
            mode === 'check' ? ['--check', '.'] : ['.'],
          )
          assert.equal(
            execution.status,
            mode === 'check' && reference.changedFiles > 0 ? 1 : 0,
            `${tool.id}/${workload.id}: ${execution.stderr}`,
          )
          assert.deepEqual(
            await fingerprint(directory, workload.files),
            mode === 'check' ? original : reference,
            `${tool.id}: output changed across runs`,
          )
          if (round >= warmup) samples.get(tool.id)?.push(execution.elapsedMs)
        }
      }
      report.results.push({
        workload: workload.id,
        files: workload.files.length,
        bytes,
        lines: workload.files.reduce(
          (sum, file) =>
            sum +
            file.source.split('\n').length -
            Number(file.source.endsWith('\n')),
          0,
        ),
        inputs: workload.files.map(file => ({
          path: file.path,
          sha256: digest(file.source),
        })),
        timings: tools.map(tool => {
          const timings = samples.get(tool.id)
          const output = outputs.get(tool.id)
          assert.ok(timings && output)
          return {
            tool: tool.id,
            output,
            statistics: summarize(timings, bytes),
          }
        }),
      })
      await save()
      for (const timing of report.results.at(-1)?.timings ?? [])
        console.log(
          `  ${timing.tool}: ${timing.statistics.medianMs.toFixed(2)} ms median`,
        )
    }
    console.log(`Saved ${output}; ${exclusions.length} rejection records`)
  } finally {
    await rm(sandbox, { recursive: true, force: true })
  }
}

if (
  process.argv[1] &&
  import.meta.url === pathToFileURL(resolve(process.argv[1])).href
)
  await main()
