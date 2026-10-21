import { spawnSync } from 'node:child_process'
import type { SpawnSyncOptions } from 'node:child_process'
import {
  chmod,
  copyFile,
  cp,
  mkdir,
  readFile,
  rm,
  writeFile,
} from 'node:fs/promises'
import { dirname, join, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'
import packageManifest from '../../packages/gdfmt/package.json' with { type: 'json' }
import targetManifest from '../../packages/gdfmt/src/targets.json' with { type: 'json' }

export interface NativeTarget {
  id: string
  triple: string
  os: string
  cpu: string
  libc?: string
}

export interface BuildResult {
  target: NativeTarget
  nativeRoot: string
  packageRoot: string
  standalone: string
}

const root = resolve(dirname(fileURLToPath(import.meta.url)), '../..')
const packageRoot = join(root, 'packages/gdfmt')
export const targets: NativeTarget[] = targetManifest
export const metadata: typeof packageManifest = packageManifest
export const outputRoot: string = join(root, 'target/npm')

export function run(
  command: string,
  args: string[],
  options: SpawnSyncOptions = {},
): void {
  const result = spawnSync(command, args, {
    cwd: root,
    stdio: 'inherit',
    ...options,
  })
  if (result.error) throw result.error
  if (result.status !== 0)
    throw new Error(`${command} exited with ${result.status}`)
}

export async function build(): Promise<BuildResult> {
  const triple =
    process.argv[2] ||
    spawnSync('rustc', ['-vV'], { encoding: 'utf8' }).stdout.match(
      /^host: (.+)$/m,
    )?.[1]
  const target = targets.find(item => item.triple === triple)
  if (!target) throw new Error(`unsupported build target: ${triple}`)
  const versions = Object.values(metadata.optionalDependencies)
  if (
    versions.length !== targets.length ||
    versions.some(version => version !== metadata.version) ||
    targets.some(
      item =>
        !Object.entries(metadata.optionalDependencies).some(
          ([name, version]) =>
            name === `@gdfmt/${item.id}` && version === metadata.version,
        ),
    )
  ) {
    throw new Error(
      'root and platform package versions/targets must match exactly',
    )
  }
  const cargo = await readFile(join(root, 'Cargo.toml'), 'utf8')
  if (!cargo.includes(`version = "${metadata.version}"`))
    throw new Error('Cargo/npm version mismatch')

  // Keep the standard Cargo build, with no generated loader or build-time npm dependency.
  run('cargo', [
    'build',
    '--locked',
    '--release',
    '--target',
    target.triple,
    '-p',
    'gdfmt_napi',
    '-p',
    'gdfmt_cli',
  ])
  const cargoOutput = join(root, 'target', target.triple, 'release')
  const nativeName =
    target.os === 'win32'
      ? 'gdfmt_napi.dll'
      : target.os === 'darwin'
        ? 'libgdfmt_napi.dylib'
        : 'libgdfmt_napi.so'
  const nativeRoot = join(outputRoot, target.id)
  await rm(nativeRoot, { recursive: true, force: true })
  await mkdir(nativeRoot, { recursive: true })
  await copyFile(join(cargoOutput, nativeName), join(nativeRoot, 'gdfmt.node'))
  await writeFile(
    join(nativeRoot, 'package.json'),
    `${JSON.stringify(
      {
        name: `@gdfmt/${target.id}`,
        version: metadata.version,
        description: `Native ${target.id} binding for gdfmt`,
        license: metadata.license,
        repository: metadata.repository,
        engines: metadata.engines,
        main: './gdfmt.node',
        exports: { '.': './gdfmt.node', './package.json': './package.json' },
        files: ['gdfmt.node', 'LICENSE', 'LICENSE-THIRD-PARTY', 'README.md'],
        os: [target.os],
        cpu: [target.cpu],
        ...(target.libc ? { libc: [target.libc] } : {}),
        publishConfig: { access: 'public' },
      },
      null,
      2,
    )}\n`,
  )
  for (const directory of [packageRoot, nativeRoot]) {
    await copyFile(join(root, 'LICENSE'), join(directory, 'LICENSE'))
    await copyFile(
      join(root, 'LICENSE-THIRD-PARTY'),
      join(directory, 'LICENSE-THIRD-PARTY'),
    )
  }
  await writeFile(
    join(nativeRoot, 'README.md'),
    `# @gdfmt/${target.id}\n\nNative binary for gdfmt ${metadata.version}. Install the gdfmt root package.\n`,
  )
  const dist = join(packageRoot, 'dist')
  await rm(dist, { recursive: true, force: true })
  await cp(join(packageRoot, 'src'), dist, { recursive: true })
  await copyFile(join(dist, 'index.d.cts'), join(dist, 'index.d.mts'))
  await copyFile(
    join(root, 'schemas/gdfmtrc.schema.json'),
    join(dist, 'schema.json'),
  )
  await chmod(join(dist, 'cli.cjs'), 0o755)
  console.log(
    `Prepared ${metadata.name}@${metadata.version} and @gdfmt/${target.id}; no publication performed.`,
  )
  return {
    target,
    nativeRoot,
    packageRoot,
    standalone: join(
      cargoOutput,
      target.os === 'win32' ? 'gdfmt.exe' : 'gdfmt',
    ),
  }
}

if (
  process.argv[1] &&
  resolve(process.argv[1]) === fileURLToPath(import.meta.url)
)
  await build()
