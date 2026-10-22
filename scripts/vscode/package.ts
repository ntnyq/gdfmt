import assert from 'node:assert/strict'
import { execFileSync } from 'node:child_process'
import { createHash } from 'node:crypto'
import {
  chmod,
  copyFile,
  mkdir,
  readFile,
  rm,
  writeFile,
} from 'node:fs/promises'
import { join, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'
import { createVSIX } from '@vscode/vsce'
import { run, targets } from '../npm/build.ts'
import {
  checkRelease,
  gitRevision,
  isCleanReleaseSource,
  readJson,
  repositoryRoot,
} from '../release/project.ts'
import { buildExtension, extensionRoot } from './build.ts'
import { stageExtension } from './stage.ts'

const vsixTargets: Record<string, string> = {
  'darwin-arm64': 'darwin-arm64',
  'darwin-x64': 'darwin-x64',
  'linux-x64-gnu': 'linux-x64',
  'linux-x64-musl': 'alpine-x64',
  'win32-x64-msvc': 'win32-x64',
}

export async function packageExtension(triple?: string): Promise<string> {
  const coreVersion = checkRelease()
  const host = execFileSync('rustc', ['-vV'], { encoding: 'utf8' }).match(
    /^host: (.+)$/m,
  )?.[1]
  const target = targets.find(target => target.triple === (triple ?? host))
  assert.ok(
    target,
    `unsupported extension target: ${triple ?? `${process.platform}-${process.arch}`}`,
  )
  await buildExtension()
  run('cargo', [
    'build',
    '--release',
    '--locked',
    '--target',
    target.triple,
    '-p',
    'gdfmt_cli',
  ])
  const server = join(extensionRoot, 'server')
  await rm(server, { recursive: true, force: true })
  await mkdir(server, { recursive: true })
  const binary = target.os === 'win32' ? 'gdfmt.exe' : 'gdfmt'
  await copyFile(
    resolve(repositoryRoot, 'target', target.triple, 'release', binary),
    join(server, binary),
  )
  if (target.os !== 'win32') await chmod(join(server, binary), 0o755)
  const manifest = readJson(join(extensionRoot, 'package.json'))
  const destination = resolve(repositoryRoot, 'target/editors', target.id)
  await mkdir(destination, { recursive: true })
  const vsix = join(
    destination,
    `gdfmt-${manifest.gdfmtVersion}-${target.id}.vsix`,
  )
  const staging = resolve(repositoryRoot, 'target/editors/staging', target.id)
  await stageExtension(staging, 'native')
  await createVSIX({
    cwd: staging,
    packagePath: vsix,
    target: vsixTargets[target.id],
    dependencies: false,
    preRelease: coreVersion.includes('-'),
  })
  const isNativeSmokePassed = target.triple === host
  if (isNativeSmokePassed)
    assert.equal(
      execFileSync(join(server, binary), ['--version'], {
        encoding: 'utf8',
      }).trim(),
      `gdfmt ${manifest.gdfmtVersion}`,
    )
  const bytes = await readFile(vsix)
  await writeFile(
    join(destination, 'acceptance.json'),
    `${JSON.stringify(
      {
        schemaVersion: 1,
        revision: gitRevision(),
        cleanSource: isCleanReleaseSource(),
        coreVersion: manifest.gdfmtVersion,
        extensionVersion: manifest.version,
        target: target.id,
        file: vsix.split(/[\\/]/).pop(),
        sha512: createHash('sha512').update(bytes).digest('hex'),
        nativeSmokePassed: isNativeSmokePassed,
        extensionHostPassed: false,
        published: false,
      },
      null,
      2,
    )}\n`,
  )
  return vsix
}

if (
  process.argv[1] &&
  resolve(process.argv[1]) === fileURLToPath(import.meta.url)
)
  console.log(await packageExtension(process.argv[2]))
