import type { TextConfig, TextOptions } from '@gdfmt/wasm'
import * as vscode from 'vscode'
import type { ConfigLayer } from './protocol'

interface ConfigParser {
  parse(source: string): Promise<TextConfig>
  resolve(layers: ConfigLayer[]): Promise<TextOptions>
}

const decoder = new TextDecoder('utf-8', { fatal: true })

function isWithin(root: vscode.Uri, uri: vscode.Uri): boolean {
  return (
    root.scheme === uri.scheme &&
    root.authority === uri.authority &&
    (uri.path === root.path ||
      uri.path.startsWith(`${root.path.replace(/\/$/, '')}/`))
  )
}

async function read(uri: vscode.Uri): Promise<string | undefined> {
  try {
    const bytes = await vscode.workspace.fs.readFile(uri)
    if (bytes.byteLength > 1024 * 1024)
      throw new Error(`${uri.toString()}: configuration exceeds 1 MiB`)
    return decoder.decode(bytes)
  } catch (error) {
    if (
      error instanceof vscode.FileSystemError &&
      error.code === 'FileNotFound'
    )
      return undefined
    throw error
  }
}

/**
 * URI-based discovery is bounded by the owning workspace. Rust parses and merges
 * each layer; this adapter only loads files and follows their relative extends.
 */
export async function resolveWebOptions(
  uri: vscode.Uri,
  explicit: string,
  parser: ConfigParser,
  observe: (uri: vscode.Uri) => void,
): Promise<TextOptions> {
  if (uri.scheme === 'untitled') return parser.resolve([])
  const folder = vscode.workspace.getWorkspaceFolder(uri)
  if (!folder)
    throw new Error(
      'Web formatting requires a workspace folder, or an untitled document',
    )
  const root = folder.uri
  const directories: vscode.Uri[] = []
  let directory = vscode.Uri.joinPath(uri, '..')
  while (isWithin(root, directory)) {
    if (directories.length >= 128)
      throw new Error('workspace directory depth limit exceeded')
    directories.push(directory)
    if (directory.path === root.path) break
    directory = vscode.Uri.joinPath(directory, '..')
  }
  // EditorConfig has independent discovery semantics. Refuse partial handling.
  for (const directory of directories) {
    const path = vscode.Uri.joinPath(directory, '.editorconfig')
    observe(path)
    if ((await read(path)) !== undefined)
      throw new Error(
        `${path.toString()}: EditorConfig is not supported in Web mode; use a gdfmt JSON/JSONC configuration or the desktop formatter`,
      )
  }
  let bytes = 0
  const configs = new Map<string, TextConfig | undefined>()
  async function readConfig(path: vscode.Uri): Promise<TextConfig | undefined> {
    const key = path.toString()
    if (configs.has(key)) return configs.get(key)
    observe(path)
    const source = await read(path)
    if (source === undefined) {
      configs.set(key, undefined)
      return undefined
    }
    bytes += new TextEncoder().encode(source).length
    if (bytes > 16 * 1024 * 1024)
      throw new Error('total configuration byte limit exceeded')
    const config = await parser.parse(source)
    configs.set(key, config)
    return config
  }
  const layers: ConfigLayer[] = []
  async function load(path: vscode.Uri, stack: Set<string>): Promise<void> {
    if (!isWithin(root, path))
      throw new Error(
        'Web configuration paths must stay inside the owning workspace',
      )
    if (stack.size >= 32 || layers.length >= 128)
      throw new Error('configuration inheritance limit exceeded')
    const key = path.toString()
    if (stack.has(key)) throw new Error(`${key}: extends cycle detected`)
    const config = await readConfig(path)
    if (!config) throw new Error(`${key}: configuration file not found`)
    stack.add(key)
    for (const parent of config.extends ?? []) {
      if (!parent || parent.startsWith('/') || /[:\\]/.test(parent))
        throw new Error(
          `${key}: extends must use workspace-relative local paths with /`,
        )
      await load(vscode.Uri.joinPath(path, '..', parent), stack)
    }
    stack.delete(key)
    if (layers.length >= 128)
      throw new Error('configuration layer limit exceeded')
    layers.push({
      language: config.language ?? {},
      format: config.format ?? {},
    })
  }
  if (explicit) {
    if (explicit.startsWith('/') || /[:\\]/.test(explicit))
      throw new Error(
        'Web gdfmt.configPath must be relative to the workspace folder using /',
      )
    await load(vscode.Uri.joinPath(root, explicit), new Set())
  } else {
    const discovered: vscode.Uri[] = []
    for (const directory of directories) {
      let stop = false
      for (const name of ['.gdfmtrc.json', '.gdfmtrc.jsonc']) {
        const path = vscode.Uri.joinPath(directory, name)
        const config = await readConfig(path)
        if (config) {
          discovered.push(path)
          stop = config.root === true
          break
        }
      }
      if (stop) break
    }
    for (const path of discovered.reverse()) await load(path, new Set())
  }
  return parser.resolve(layers)
}
