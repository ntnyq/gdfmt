import { execFile } from 'node:child_process'
import { access } from 'node:fs/promises'
import { createRequire } from 'node:module'
import { basename, isAbsolute, join, resolve } from 'node:path'
import { promisify } from 'node:util'
import * as vscode from 'vscode'
import type { Executable } from 'vscode-languageclient/node'

const execute = promisify(execFile)

/**
 * Uses a configured executable, a trusted workspace installation, then the
 * platform binary shipped in the VSIX. No shell or system PATH lookup.
 */
export async function resolveServer(
  context: vscode.ExtensionContext,
  folder: vscode.WorkspaceFolder | undefined,
  expectedVersion: string,
): Promise<Executable> {
  const cwd = folder?.uri.fsPath ?? context.extensionPath

  const settings = vscode.workspace.getConfiguration(
    'gdfmt',
    folder?.uri ?? null,
  )
  const explicit = vscode.workspace.isTrusted
    ? settings.get<string>('serverPath', '')
    : ''
  let path: string | undefined
  if (explicit) path = isAbsolute(explicit) ? explicit : resolve(cwd, explicit)
  if (!path && folder && vscode.workspace.isTrusted) {
    const require = createRequire(join(cwd, 'package.json'))
    let manifest: string | undefined
    try {
      manifest = require.resolve('@gdfmt/gdfmt/package.json')
    } catch (error) {
      if (
        !(
          error instanceof Error &&
          'code' in error &&
          error.code === 'MODULE_NOT_FOUND'
        )
      )
        throw error
    }
    if (manifest) path = join(manifest, '..', 'dist', 'cli.cjs')
  }
  path ??= context.asAbsolutePath(
    join('server', process.platform === 'win32' ? 'gdfmt.exe' : 'gdfmt'),
  )
  await access(path)
  const isScript = /\.[cm]?js$/.test(basename(path))
  const [major, minor] = process.versions.node.split('.').map(Number)
  if (isScript && !((major === 22 && minor >= 13) || major === 24))
    throw new Error(
      `The workspace npm CLI requires Node 22.13+ in the 22.x line or Node 24.x, but this extension host uses ${process.versions.node}. Set gdfmt.serverPath to a native executable, or remove the workspace npm installation to use the bundled server.`,
    )
  const command = isScript ? process.execPath : path
  const args = isScript ? [path] : []
  const env = isScript
    ? { ...process.env, ELECTRON_RUN_AS_NODE: '1' }
    : { ...process.env }
  const { stdout } = await execute(command, [...args, '--version'], {
    cwd,
    env,
    timeout: 10_000,
    windowsHide: true,
  })
  if (stdout.trim() !== `gdfmt ${expectedVersion}`)
    throw new Error(
      `Expected gdfmt ${expectedVersion}, received ${stdout.trim()} from ${path}. Update the executable or clear gdfmt.serverPath to use the bundled version.`,
    )
  return { command, args: [...args, '--lsp'], options: { cwd, env } }
}
