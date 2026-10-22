import { copyFile, cp, mkdir, rm, writeFile } from 'node:fs/promises'
import { resolve } from 'node:path'
import { readJson } from '../release/project.ts'
import { extensionRoot } from './build.ts'

/**
 * Stage separate platform packages so a Web VSIX has no executable or Node
 * entry, and native VSIX builds do not require the WASM toolchain.
 */
export async function stageExtension(
  destination: string,
  kind: 'web' | 'native',
): Promise<void> {
  await rm(destination, { recursive: true, force: true })
  await mkdir(resolve(destination, 'dist'), { recursive: true })
  for (const name of ['LICENSE', 'LICENSE-THIRD-PARTY', 'README.md'])
    await copyFile(resolve(extensionRoot, name), resolve(destination, name))
  await cp(resolve(extensionRoot, 'schemas'), resolve(destination, 'schemas'), {
    recursive: true,
  })
  const manifest = readJson(resolve(extensionRoot, 'package.json'))
  // Runtime dependencies are bundled in each entry. A VSIX must not contain
  // workspace protocol dependencies or rely on installation scripts.
  delete manifest.dependencies
  manifest.files = [
    'dist/**',
    'schemas/**',
    ...(kind === 'native' ? ['server/**'] : []),
    'LICENSE*',
    'README.md',
  ]
  if (kind === 'web') {
    delete manifest.main
    manifest.capabilities = {
      untrustedWorkspaces: { supported: true },
      virtualWorkspaces: { supported: true },
    }
    await mkdir(resolve(destination, 'dist/web'), { recursive: true })
    for (const name of [
      'extension.js',
      'formatter-worker.js',
      'formatter.wasm',
    ])
      await copyFile(
        resolve(extensionRoot, 'dist/web', name),
        resolve(destination, 'dist/web', name),
      )
  } else {
    delete manifest.browser
    const capabilities = manifest.capabilities
    if (capabilities && typeof capabilities === 'object')
      manifest.capabilities = {
        ...capabilities,
        virtualWorkspaces: { supported: false },
      }
    await copyFile(
      resolve(extensionRoot, 'dist/extension.cjs'),
      resolve(destination, 'dist/extension.cjs'),
    )
    await cp(resolve(extensionRoot, 'server'), resolve(destination, 'server'), {
      recursive: true,
    })
  }
  await writeFile(
    resolve(destination, 'package.json'),
    `${JSON.stringify(manifest, null, 2)}\n`,
  )
}
