import * as vscode from 'vscode'
import { documentEdits } from '../formatting'
import { resolveWebOptions } from './configuration'
import { utf16Offsets } from './positions'
import { Cancelled, WorkerClient } from './worker-client'

let stop: (() => void) | undefined

export async function activate(
  context: vscode.ExtensionContext,
): Promise<void> {
  const output = vscode.window.createOutputChannel('gdfmt')
  const diagnostics = vscode.languages.createDiagnosticCollection('gdfmt')
  context.subscriptions.push(output, diagnostics)
  let client: Promise<WorkerClient> | undefined
  let current: WorkerClient | undefined
  let epoch = 0
  const configurationFiles = new Set<string>()
  let disposed = false

  function restart(): void {
    epoch++
    const running = current
    current = undefined
    client = undefined
    running?.dispose()
    diagnostics.clear()
  }

  function getClient(): Promise<WorkerClient> {
    if (disposed) return Promise.reject(new Cancelled())
    if (client) return client
    const generation = epoch
    const promise = (async () => {
      const [script, wasm] = await Promise.all(
        ['formatter-worker.js', 'formatter.wasm'].map(name =>
          vscode.workspace.fs.readFile(
            vscode.Uri.joinPath(context.extensionUri, 'dist', 'web', name),
          ),
        ),
      )
      if (disposed || generation !== epoch) throw new Cancelled()
      const url = URL.createObjectURL(
        new Blob([Uint8Array.from(script)], { type: 'text/javascript' }),
      )
      let instance: WorkerClient | undefined
      try {
        const worker = new Worker(url, { name: 'gdfmt formatter' })
        const running = new WorkerClient(worker, () => {
          if (current === running) {
            current = undefined
            client = undefined
          }
        })
        instance = running
        current = running
        const response = await instance.request({ kind: 'initialize', wasm })
        if (response.kind !== 'ready')
          throw new Error('formatter worker did not initialize')
        if (disposed || generation !== epoch) throw new Cancelled()
        return instance
      } catch (error) {
        instance?.dispose()
        throw error
      } finally {
        URL.revokeObjectURL(url)
      }
    })()
    client = promise
    void promise.catch(() => {
      if (client === promise) client = undefined
    })
    return promise
  }

  const provider = vscode.languages.registerDocumentFormattingEditProvider(
    { language: 'gdscript' },
    {
      async provideDocumentFormattingEdits(document, _options, token) {
        if (
          !vscode.workspace
            .getConfiguration('gdfmt', document)
            .get<boolean>('enable', true)
        )
          return []
        const source = document.getText()
        const version = document.version
        const generation = epoch
        const isCurrent = (): boolean =>
          !disposed &&
          !token.isCancellationRequested &&
          !document.isClosed &&
          document.version === version &&
          epoch === generation
        try {
          const worker = await getClient()
          const options = await resolveWebOptions(
            document.uri,
            vscode.workspace
              .getConfiguration('gdfmt', document)
              .get<string>('configPath', ''),
            {
              async parse(source) {
                if (!isCurrent()) throw new Cancelled()
                const response = await worker.request(
                  { kind: 'parseConfig', source },
                  token,
                )
                if (response.kind !== 'config')
                  throw new Error('unexpected configuration response')
                return response.config
              },
              async resolve(layers) {
                if (!isCurrent()) throw new Cancelled()
                const response = await worker.request(
                  { kind: 'resolveOptions', layers },
                  token,
                )
                if (response.kind !== 'options')
                  throw new Error('unexpected options response')
                return response.options
              },
            },
            uri => configurationFiles.add(uri.toString()),
          )
          if (!isCurrent()) return []
          const response = await worker.request(
            { kind: 'format', source, options },
            token,
          )
          if (!isCurrent()) return []
          if (response.kind !== 'format')
            throw new Error('unexpected formatting response')
          const result = response.result
          const positions = utf16Offsets(
            source,
            result.diagnostics.flatMap(item => [
              item.span.start,
              item.span.end,
            ]),
          )
          diagnostics.set(
            document.uri,
            result.diagnostics.map(item => {
              const range = new vscode.Range(
                document.positionAt(positions.get(item.span.start) ?? 0),
                document.positionAt(positions.get(item.span.end) ?? 0),
              )
              const diagnostic = new vscode.Diagnostic(
                range,
                item.message,
                item.severity === 'error'
                  ? vscode.DiagnosticSeverity.Error
                  : item.severity === 'warning'
                    ? vscode.DiagnosticSeverity.Warning
                    : vscode.DiagnosticSeverity.Information,
              )
              diagnostic.code = item.code
              diagnostic.source = 'gdfmt'
              return diagnostic
            }),
          )
          if (result.output === null) return []
          return documentEdits(
            document,
            source,
            result.output,
            options.format?.lineEnding,
          )
        } catch (error) {
          if (error instanceof Cancelled || !isCurrent()) return []
          output.appendLine(String(error))
          void vscode.window.showErrorMessage(`gdfmt Web: ${String(error)}`)
          return []
        }
      },
    },
  )

  const watch = (folder: vscode.WorkspaceFolder): vscode.FileSystemWatcher => {
    const watcher = vscode.workspace.createFileSystemWatcher(
      new vscode.RelativePattern(folder, '**/*'),
    )
    const invalidate = (uri: vscode.Uri): void => {
      if (configurationFiles.has(uri.toString())) restart()
    }
    context.subscriptions.push(
      watcher.onDidCreate(invalidate),
      watcher.onDidChange(invalidate),
      watcher.onDidDelete(invalidate),
    )
    return watcher
  }
  let watchers = (vscode.workspace.workspaceFolders ?? []).map(watch)
  const version: unknown = context.extension.packageJSON.gdfmtVersion
  context.subscriptions.push(
    provider,
    vscode.commands.registerCommand('gdfmt.restartServer', restart),
    vscode.commands.registerCommand('gdfmt.showOutput', () => output.show()),
    vscode.commands.registerCommand('gdfmt.showVersion', () =>
      vscode.window.showInformationMessage(
        `gdfmt ${String(version)} (WebAssembly)`,
      ),
    ),
    vscode.workspace.onDidChangeConfiguration(event => {
      if (event.affectsConfiguration('gdfmt')) restart()
    }),
    vscode.workspace.onDidChangeWorkspaceFolders(() => {
      restart()
      watchers.forEach(watcher => watcher.dispose())
      watchers = (vscode.workspace.workspaceFolders ?? []).map(watch)
    }),
    vscode.workspace.onDidChangeTextDocument(event =>
      diagnostics.delete(event.document.uri),
    ),
    vscode.workspace.onDidCloseTextDocument(document =>
      diagnostics.delete(document.uri),
    ),
  )
  stop = () => {
    disposed = true
    restart()
    watchers.forEach(watcher => watcher.dispose())
  }
  output.appendLine(
    'Web mode uses bundled WASM. Configuration is bounded by the workspace; overrides, ignore selection and EditorConfig are unsupported. serverPath applies to desktop only.',
  )
}

export function deactivate(): void {
  stop?.()
  stop = undefined
}
