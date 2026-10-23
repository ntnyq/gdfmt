import assert from 'node:assert/strict'
import { execFileSync } from 'node:child_process'
import {
  mkdir,
  mkdtemp,
  readFile,
  readdir,
  rm,
  writeFile,
} from 'node:fs/promises'
import { tmpdir } from 'node:os'
import { dirname, join, resolve } from 'node:path'
import { runTests, runVSCodeCommand } from '@vscode/test-electron'
import { build } from 'esbuild'
import { preservationCases } from '../../editors/vscode/tests/preservation-cases.ts'
import { readJson, repositoryRoot } from '../release/project.ts'
import { extensionRoot } from './build.ts'
import { packageExtension } from './package.ts'

const vsix = await packageExtension()
// The repository ignores target/, so keep documents outside that tree: the LSP
// intentionally honors the same ignore rules as CLI formatting.
const workspace = await mkdtemp(join(tmpdir(), 'gdfmt-vscode-'))
const folders = [
  { path: 'first/nested', indentWidth: 6 },
  { path: 'first', indentWidth: 2 },
  { path: 'second', indentWidth: 4 },
]
for (const { path: name, indentWidth } of folders) {
  await mkdir(resolve(workspace, name), { recursive: true })
  await writeFile(resolve(workspace, name, 'example.gd'), 'var value=1\n')
  await writeFile(
    resolve(workspace, name, '.gdfmtrc.json'),
    JSON.stringify({
      root: true,
      format: { indentStyle: 'space', indentWidth },
    }),
  )
}
for (const lineEnding of ['lf', 'crlf', 'preserve']) {
  const directory = resolve(workspace, 'first/eol', lineEnding)
  await mkdir(directory, { recursive: true })
  await writeFile(
    resolve(directory, '.gdfmtrc.json'),
    JSON.stringify({ root: true, format: { lineEnding } }),
  )
  await writeFile(resolve(directory, 'example.gd'), '')
}
for (const test of preservationCases) {
  const directory = resolve(workspace, 'first/preservation', test.name)
  await mkdir(directory, { recursive: true })
  await writeFile(
    resolve(directory, '.gdfmtrc.json'),
    JSON.stringify({
      root: true,
      format: { lineEnding: test.lineEnding, finalNewline: test.finalNewline },
    }),
  )
  const path = resolve(directory, 'example.gd')
  await writeFile(path, test.source)
  assert.equal(
    execFileSync(
      resolve(
        extensionRoot,
        'server',
        process.platform === 'win32' ? 'gdfmt.exe' : 'gdfmt',
      ),
      ['--stdin-filepath', path],
      { cwd: workspace, input: test.source, encoding: 'utf8' },
    ),
    test.formatted,
    `${test.name}: fixture must match validated core output`,
  )
}
await build({
  entryPoints: [resolve(extensionRoot, 'tests/extension.ts')],
  outfile: resolve(extensionRoot, 'dist/test.cjs'),
  bundle: true,
  platform: 'node',
  format: 'cjs',
  external: ['vscode'],
  target: 'node20',
})
const isGodotToolsEnabled = process.env.GDFMT_TEST_GODOT_TOOLS === '1'
const debugProfile =
  process.env.GDFMT_DEBUG_SAVE === '1'
    ? await mkdtemp(join(tmpdir(), 'gdfmt-save-repro-'))
    : resolve(repositoryRoot, 'target/vscode-test')
const extensionsDirectory = resolve(debugProfile, 'extensions')
if (isGodotToolsEnabled)
  await runVSCodeCommand(
    [
      '--install-extension',
      'geequlim.godot-tools@2.7.1',
      '--extensions-dir',
      extensionsDirectory,
      '--user-data-dir',
      resolve(debugProfile, 'user-data'),
      '--force',
    ],
    { version: '1.96.4', timeout: 30_000 },
  )
for (const order of [folders, [...folders].reverse()]) {
  await writeFile(
    resolve(workspace, 'test.code-workspace'),
    JSON.stringify({
      folders: order.map(({ path }) => ({ path })),
      settings: {
        'security.workspace.trust.enabled': false,
        '[gdscript]': {
          'editor.defaultFormatter': 'ntnyq.gdfmt',
          'editor.formatOnSave': true,
        },
        'files.autoSave': 'off',
      },
    }),
  )
  await runTests({
    version: '1.96.4',
    vscodeExecutablePath: process.env.GDFMT_VSCODE_EXECUTABLE,
    timeout: 30_000,
    extensionDevelopmentPath: extensionRoot,
    extensionTestsPath: resolve(extensionRoot, 'dist/test.cjs'),
    extensionTestsEnv: {
      GDFMT_TEST_GODOT_TOOLS: isGodotToolsEnabled ? '1' : '0',
    },
    launchArgs: [
      resolve(workspace, 'test.code-workspace'),
      ...(isGodotToolsEnabled
        ? ['--extensions-dir', extensionsDirectory]
        : ['--disable-extensions']),
      '--skip-welcome',
      '--skip-release-notes',
      '--disable-workspace-trust',
      '--user-data-dir',
      resolve(debugProfile, 'user-data'),
    ],
  })
}

// Install the archive in a separate profile: a passing development host alone
// cannot prove that the packaged manifest and bundled server install correctly.
const installedProfile = await mkdtemp(join(tmpdir(), 'gdfmt-vsix-'))
try {
  const extensions = resolve(installedProfile, 'extensions')
  const profileArgs = [
    '--extensions-dir',
    extensions,
    '--user-data-dir',
    resolve(installedProfile, 'profile'),
  ]
  await runVSCodeCommand(
    ['--install-extension', vsix, ...profileArgs, '--force'],
    { version: '1.96.4', timeout: 30_000 },
  )
  const listed = await runVSCodeCommand(
    ['--list-extensions', '--show-versions', ...profileArgs],
    { version: '1.96.4', timeout: 30_000 },
  )
  const manifest = readJson(resolve(extensionRoot, 'package.json'))
  assert.ok(
    listed.stdout.split(/\r?\n/).includes(`ntnyq.gdfmt@${manifest.version}`),
  )
  const candidates = (
    await readdir(extensions, { withFileTypes: true })
  ).filter(
    entry =>
      entry.isDirectory() &&
      entry.name.startsWith(`ntnyq.gdfmt-${manifest.version}`),
  )
  assert.equal(candidates.length, 1, 'expected exactly one installed candidate')
  const installed = resolve(extensions, candidates[0].name)
  const installedManifest = readJson(resolve(installed, 'package.json'))
  assert.equal(installedManifest.version, manifest.version)
  assert.equal(installedManifest.gdfmtVersion, manifest.gdfmtVersion)
  const binary = process.platform === 'win32' ? 'gdfmt.exe' : 'gdfmt'
  const server = resolve(installed, 'server', binary)
  assert.deepEqual(
    await readFile(server),
    await readFile(resolve(extensionRoot, 'server', binary)),
  )
  assert.equal(
    execFileSync(server, ['--version'], { encoding: 'utf8' }).trim(),
    `gdfmt ${manifest.gdfmtVersion}`,
  )
  assert.equal(
    execFileSync(
      server,
      ['--stdin-filepath', resolve(workspace, 'first/example.gd')],
      {
        cwd: workspace,
        input: 'var value=1\n',
        encoding: 'utf8',
      },
    ),
    'var value = 1\n',
  )
} finally {
  await rm(installedProfile, { recursive: true, force: true })
}

const receiptPath = resolve(dirname(vsix), 'acceptance.json')
const receipt: unknown = JSON.parse(await readFile(receiptPath, 'utf8'))
if (!receipt || typeof receipt !== 'object')
  throw new Error('invalid VSIX receipt')
await writeFile(
  receiptPath,
  `${JSON.stringify({ ...receipt, extensionHostPassed: true, installedVsixPassed: true, godotToolsPassed: isGodotToolsEnabled, vscodeVersion: '1.96.4' }, null, 2)}\n`,
)

await rm(workspace, { recursive: true, force: true })
