import assert from 'node:assert/strict'
import { execFileSync } from 'node:child_process'
import { createHash } from 'node:crypto'
import {
  mkdir,
  mkdtemp,
  readFile,
  readdir,
  rm,
  writeFile,
} from 'node:fs/promises'
import { tmpdir } from 'node:os'
import { join, resolve } from 'node:path'
import { parseArgs } from 'node:util'
import {
  runTests as runDesktopTests,
  runVSCodeCommand,
} from '@vscode/test-electron'
import { runTests as runWebTests } from '@vscode/test-web'
import { build } from 'esbuild'
import { readJson, repositoryRoot } from '../release/project.ts'

const { values } = parseArgs({
  options: {
    web: { type: 'boolean', default: false },
    'upgrade-from': { type: 'string' },
  },
})
assert.ok(
  !values.web || !values['upgrade-from'],
  'upgrade acceptance requires a desktop host',
)
if (values['upgrade-from'])
  assert.match(values['upgrade-from'], /^\d+\.\d+\.\d+$/)
const manifest = readJson(
  resolve(repositoryRoot, 'editors/vscode/package.json'),
)
const version = '1.96.4'
const profile = await mkdtemp(join(tmpdir(), 'gdfmt-gallery-'))
const driver = join(profile, 'test-extension')
const testPath = join(driver, values.web ? 'test.js' : 'test.cjs')
await mkdir(driver)
await writeFile(
  join(driver, 'package.json'),
  JSON.stringify({
    name: 'gdfmt-marketplace-tests',
    publisher: 'gdfmt-tests',
    version: '0.0.0',
    engines: { vscode: '^1.96.0' },
    main: './empty.cjs',
    browser: './empty.cjs',
  }),
)
await writeFile(join(driver, 'empty.cjs'), 'exports.activate = () => {}\n')
await build({
  entryPoints: [resolve(repositoryRoot, 'editors/vscode/tests/marketplace.ts')],
  outfile: testPath,
  bundle: true,
  platform: values.web ? 'browser' : 'node',
  format: 'cjs',
  target: 'es2022',
  external: ['vscode'],
  define: {
    EXPECTED_EXTENSION_VERSION: JSON.stringify(manifest.version),
    EXPECTED_CORE_VERSION: JSON.stringify(manifest.gdfmtVersion),
    EXPECTED_UI_KIND: values.web ? '2' : '1',
  },
})
const report: Record<string, unknown> = {
  schemaVersion: 1,
  checkedAt: new Date().toISOString(),
  extensionVersion: manifest.version,
  coreVersion: manifest.gdfmtVersion,
  vscodeVersion: version,
  source: 'gallery',
  web: values.web,
  upgradeFrom: values['upgrade-from'],
}
try {
  if (values.web) {
    await runWebTests({
      browserType: 'chromium',
      // test-web documents this flag for external resources from its local host.
      // It applies only to this disposable browser, not the normal Web CI suite.
      browserOptions: ['--disable-web-security'],
      headless: true,
      coi: false,
      quality: 'stable',
      commit: 'cd4ee3b1c348a13bafd8f9ad8060705f6d4b9cba',
      extensionDevelopmentPath: driver,
      extensionTestsPath: testPath,
      extensionIds: [{ id: 'ntnyq.gdfmt', preRelease: false }],
      port: 3001,
      testRunnerDataDir: resolve(
        repositoryRoot,
        'target/vscode-web-test/runtime',
      ),
    })
    report.crossOriginIsolated = false
    report.browser = 'chromium'
    report.webSecurityDisabled = true
  } else {
    const extensions = join(profile, 'extensions')
    const profileArgs = [
      '--extensions-dir',
      extensions,
      '--user-data-dir',
      join(profile, 'user-data'),
    ]
    const command = (args: string[]) =>
      runVSCodeCommand([...args, ...profileArgs], { version, timeout: 60_000 })
    if (values['upgrade-from']) {
      await command([
        '--install-extension',
        `ntnyq.gdfmt@${values['upgrade-from']}`,
        '--pre-release',
        '--force',
      ])
      const before = await command(['--list-extensions', '--show-versions'])
      assert.ok(
        before.stdout
          .split(/\r?\n/)
          .includes(`ntnyq.gdfmt@${values['upgrade-from']}`),
      )
    }
    // The unversioned Gallery request exercises the default stable install/update route.
    await command(['--install-extension', 'ntnyq.gdfmt', '--force'])
    const listed = await command(['--list-extensions', '--show-versions'])
    assert.ok(
      listed.stdout.split(/\r?\n/).includes(`ntnyq.gdfmt@${manifest.version}`),
    )
    const candidates = (await readdir(extensions)).filter(name =>
      name.startsWith(`ntnyq.gdfmt-${manifest.version}`),
    )
    assert.equal(candidates.length, 1)
    const installed = join(extensions, candidates[0])
    const installedManifest = readJson(join(installed, 'package.json'))
    assert.equal(installedManifest.gdfmtVersion, manifest.gdfmtVersion)
    const metadata = installedManifest.__metadata
    assert.ok(metadata && typeof metadata === 'object' && 'source' in metadata)
    assert.equal(metadata.source, 'gallery')
    assert.ok('isPreReleaseVersion' in metadata)
    assert.equal(metadata.isPreReleaseVersion, false)
    const server = join(
      installed,
      'server',
      process.platform === 'win32' ? 'gdfmt.exe' : 'gdfmt',
    )
    assert.equal(
      execFileSync(server, ['--version'], { encoding: 'utf8' }).trim(),
      `gdfmt ${manifest.gdfmtVersion}`,
    )
    report.serverSha256 = createHash('sha256')
      .update(await readFile(server))
      .digest('hex')
    await runDesktopTests({
      version,
      timeout: 90_000,
      extensionDevelopmentPath: driver,
      extensionTestsPath: testPath,
      launchArgs: [
        ...profileArgs,
        '--skip-welcome',
        '--skip-release-notes',
        '--disable-workspace-trust',
      ],
    })
    report.target = `${process.platform}-${process.arch}`
  }
  report.passed = true
  const reportDirectory = resolve(repositoryRoot, 'target/marketplace')
  await mkdir(reportDirectory, { recursive: true })
  const name = values.web
    ? 'web'
    : values['upgrade-from']
      ? 'upgrade'
      : 'desktop'
  await writeFile(
    join(reportDirectory, `${name}-acceptance.json`),
    `${JSON.stringify(report, null, 2)}\n`,
  )
} finally {
  await rm(profile, { recursive: true, force: true })
}
