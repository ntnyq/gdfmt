import * as vscode from 'vscode'

/** In-memory URI provider for the actual browser Extension Host tests. */
export class TestFileSystem implements vscode.FileSystemProvider {
  private readonly entries = new Map<string, Uint8Array | null>([['/', null]])
  private readonly changes = new vscode.EventEmitter<vscode.FileChangeEvent[]>()
  private held?: { path: string; entered: () => void; wait: Promise<void> }
  holdNextRead(path: string): { entered: Promise<void>; release(): void } {
    let release = (): void => {}
    const wait = new Promise<void>(done => {
      release = done
    })
    const entered = new Promise<void>(done => {
      this.held = { path, entered: done, wait }
    })
    return { entered, release }
  }
  readonly onDidChangeFile = this.changes.event
  watch(): vscode.Disposable {
    return new vscode.Disposable(() => {})
  }
  stat(uri: vscode.Uri): vscode.FileStat {
    const data = this.entries.get(uri.path)
    if (data === undefined) throw vscode.FileSystemError.FileNotFound(uri)
    return {
      type: data === null ? vscode.FileType.Directory : vscode.FileType.File,
      ctime: 0,
      mtime: 0,
      size: data?.byteLength ?? 0,
    }
  }
  async readFile(uri: vscode.Uri): Promise<Uint8Array> {
    const data = this.entries.get(uri.path)
    if (data === undefined) throw vscode.FileSystemError.FileNotFound(uri)
    if (data === null) throw vscode.FileSystemError.FileIsADirectory(uri)
    const held = this.held
    if (held?.path === uri.path) {
      this.held = undefined
      held.entered()
      await held.wait
    }
    return data.slice()
  }
  readDirectory(uri: vscode.Uri): [string, vscode.FileType][] {
    this.stat(uri)
    const prefix = uri.path.replace(/\/$/, '') + '/'
    return [...this.entries]
      .filter(
        ([path]) =>
          path.startsWith(prefix) &&
          path !== uri.path &&
          !path.slice(prefix.length).includes('/'),
      )
      .map(([path, data]) => [
        path.slice(prefix.length),
        data === null ? vscode.FileType.Directory : vscode.FileType.File,
      ])
  }
  createDirectory(uri: vscode.Uri): void {
    if (this.entries.has(uri.path)) return
    this.createDirectory(vscode.Uri.joinPath(uri, '..'))
    this.entries.set(uri.path, null)
    this.changes.fire([{ type: vscode.FileChangeType.Created, uri }])
  }
  writeFile(
    uri: vscode.Uri,
    content: Uint8Array,
    options: { create: boolean; overwrite: boolean },
  ): void {
    const exists = this.entries.has(uri.path)
    if (exists && !options.overwrite)
      throw vscode.FileSystemError.FileExists(uri)
    if (!exists && !options.create)
      throw vscode.FileSystemError.FileNotFound(uri)
    this.stat(vscode.Uri.joinPath(uri, '..'))
    this.entries.set(uri.path, content.slice())
    this.changes.fire([
      {
        type: exists
          ? vscode.FileChangeType.Changed
          : vscode.FileChangeType.Created,
        uri,
      },
    ])
  }
  delete(uri: vscode.Uri): void {
    this.stat(uri)
    for (const path of this.entries.keys())
      if (path === uri.path || path.startsWith(uri.path + '/'))
        this.entries.delete(path)
    this.changes.fire([{ type: vscode.FileChangeType.Deleted, uri }])
  }
  rename(
    oldUri: vscode.Uri,
    newUri: vscode.Uri,
    options: { overwrite: boolean },
  ): void {
    this.stat(oldUri)
    if (this.entries.has(newUri.path) && !options.overwrite)
      throw vscode.FileSystemError.FileExists(newUri)
    for (const [path, data] of [...this.entries]) {
      if (path === oldUri.path || path.startsWith(oldUri.path + '/')) {
        this.entries.set(newUri.path + path.slice(oldUri.path.length), data)
        this.entries.delete(path)
      }
    }
    this.changes.fire([
      { type: vscode.FileChangeType.Deleted, uri: oldUri },
      { type: vscode.FileChangeType.Created, uri: newUri },
    ])
  }
}
