import { copyFile, mkdir } from 'node:fs/promises'
import { resolve } from 'node:path'
import { fileURLToPath } from 'node:url'
import { build } from 'esbuild'
import { repositoryRoot } from '../release/project.ts'

export const extensionRoot: string = resolve(repositoryRoot, 'editors/vscode')

export async function prepareExtensionAssets(): Promise<void> {
  await mkdir(resolve(extensionRoot, 'schemas'), { recursive: true })
  for (const name of ['LICENSE', 'LICENSE-THIRD-PARTY'])
    await copyFile(resolve(repositoryRoot, name), resolve(extensionRoot, name))
  await copyFile(
    resolve(repositoryRoot, 'schemas/gdfmtrc.schema.json'),
    resolve(extensionRoot, 'schemas/gdfmtrc.schema.json'),
  )
}

export async function buildExtension(): Promise<void> {
  await prepareExtensionAssets()
  await build({
    entryPoints: [resolve(extensionRoot, 'src/extension.ts')],
    outfile: resolve(extensionRoot, 'dist/extension.cjs'),
    bundle: true,
    platform: 'node',
    target: 'node20',
    format: 'cjs',
    external: ['vscode'],
    sourcemap: false,
  })
}

if (
  process.argv[1] &&
  resolve(process.argv[1]) === fileURLToPath(import.meta.url)
)
  await buildExtension()
