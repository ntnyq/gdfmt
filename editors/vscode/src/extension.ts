import { isAbsolute, resolve } from 'node:path'
import * as vscode from 'vscode'
import { LanguageClient } from 'vscode-languageclient/node'
import type { DocumentSelector } from 'vscode-languageclient/node'
import { documentEdits, UnsupportedLineEndingsError } from './formatting'
import { resolveServer } from './server'

interface ClientSession {
  client: LanguageClient
  folder?: vscode.WorkspaceFolder
  watcher?: vscode.FileSystemWatcher
}

let stop: (() => Promise<void>) | undefined

export async function activate(
  context: vscode.ExtensionContext,
): Promise<void> {
  const output = vscode.window.createOutputChannel('gdfmt')
  context.subscriptions.push(output)
  const sessions: ClientSession[] = []
  let operation = Promise.resolve()
  let disposed = false
  const coreVersion: unknown = context.extension.packageJSON.gdfmtVersion
  if (typeof coreVersion !== 'string')
    throw new Error('gdfmt version metadata is missing')

  const version = coreVersion

  async function stopClients(): Promise<void> {
    if (process.env.GDFMT_DEBUG_SAVE === '1')
      console.log('[DEBUG-rc1-save] stop clients', new Date().toISOString())
    await Promise.all(
      sessions.splice(0).map(async ({ client, watcher }) => {
        watcher?.dispose()
        await client.stop()
      }),
    )
  }

  async function startClient(folder?: vscode.WorkspaceFolder): Promise<void> {
    const settings = vscode.workspace.getConfiguration(
      'gdfmt',
      folder?.uri ?? null,
    )
    if (!settings.get<boolean>('enable', true)) return
    const selector: DocumentSelector = folder
      ? [
          {
            scheme: 'file',
            language: 'gdscript',
            pattern: `${folder.uri.fsPath.replaceAll('\\', '/').replace(/[[\]{}*?]/g, '[$&]')}/**/*`,
          },
        ]
      : [
          { scheme: 'untitled', language: 'gdscript' },
          ...(!vscode.workspace.workspaceFolders?.length
            ? [{ scheme: 'file', language: 'gdscript' }]
            : []),
        ]
    const server = await resolveServer(context, folder, version)
    const config = vscode.workspace.isTrusted
      ? settings.get<string>('configPath', '')
      : ''
    const configPath = config
      ? isAbsolute(config)
        ? config
        : resolve(folder?.uri.fsPath ?? context.extensionPath, config)
      : undefined
    const watcher = folder
      ? vscode.workspace.createFileSystemWatcher(
          new vscode.RelativePattern(
            folder,
            '**/{*.json,*.jsonc,.editorconfig,.gitignore,.gdfmtignore}',
          ),
        )
      : undefined
    const client = new LanguageClient(
      `gdfmt:${folder?.uri.toString() ?? 'untitled'}`,
      'gdfmt',
      server,
      {
        documentSelector: selector,
        workspaceFolder: folder,
        outputChannel: output,
        initializationOptions: { configPath },
        synchronize: { fileEvents: watcher },
        middleware: {
          provideDocumentFormattingEdits: async (document, _options, token) => {
            // VS Code can select a parent root's overlapping provider. Route
            // every selected provider through the document's owning session.
            const owner = vscode.workspace.getWorkspaceFolder(document.uri)
            const session = sessions.find(
              session =>
                session.folder?.uri.toString() === owner?.uri.toString(),
            )
            if (process.env.GDFMT_DEBUG_SAVE === '1')
              console.log(
                '[DEBUG-rc1-save] format entry',
                new Date().toISOString(),
                document.version,
                document.getText(),
                owner?.name,
                session?.client.isRunning(),
              )
            if (!session?.client.isRunning()) return []
            const source = document.getText()
            const version = document.version
            try {
              const result = await session.client.sendRequest<unknown>(
                'gdfmt/formatting',
                { textDocument: { uri: document.uri.toString() } },
                token,
              )
              if (process.env.GDFMT_DEBUG_SAVE === '1')
                console.log(
                  '[DEBUG-rc1-save] format result',
                  new Date().toISOString(),
                  version,
                  document.version,
                  token.isCancellationRequested,
                  sessions.includes(session),
                  result,
                )
              if (
                token.isCancellationRequested ||
                document.isClosed ||
                document.version !== version ||
                !sessions.includes(session)
              )
                return []
              if (
                !result ||
                typeof result !== 'object' ||
                !('output' in result) ||
                (result.output !== null && typeof result.output !== 'string') ||
                !('lineEnding' in result) ||
                (result.lineEnding !== 'lf' &&
                  result.lineEnding !== 'crlf' &&
                  result.lineEnding !== 'preserve')
              )
                throw new Error('invalid formatter response')
              return result.output === null
                ? []
                : documentEdits(
                    document,
                    source,
                    result.output,
                    result.lineEnding,
                  )
            } catch (error) {
              if (!token.isCancellationRequested) {
                output.appendLine(String(error))
                if (error instanceof UnsupportedLineEndingsError)
                  void vscode.window.showErrorMessage(`gdfmt: ${error.message}`)
              }
              return []
            }
          },
        },
      },
    )
    sessions.push({ client, folder, watcher })
    await client.start()
    output.appendLine(
      `Started gdfmt ${version} for ${folder?.name ?? 'untitled documents'}: ${server.command}`,
    )
  }

  function restart(): Promise<void> {
    operation = operation
      .catch(error => output.appendLine(String(error)))
      .then(async () => {
        await stopClients()
        if (disposed) return
        // The extra client owns untitled documents; each workspace has its own
        // executable/configuration and runs in the workspace extension host.
        const folders: (vscode.WorkspaceFolder | undefined)[] = [
          ...(vscode.workspace.workspaceFolders ?? []),
          undefined,
        ]
        for (const folder of folders) {
          try {
            await startClient(folder)
          } catch (error) {
            output.appendLine(String(error))
            void vscode.window.showErrorMessage(`gdfmt: ${String(error)}`)
          }
        }
      })
    return operation
  }

  context.subscriptions.push(
    vscode.commands.registerCommand('gdfmt.restartServer', restart),
    vscode.commands.registerCommand('gdfmt.showOutput', () => output.show()),
    vscode.commands.registerCommand('gdfmt.showVersion', () =>
      vscode.window.showInformationMessage(`gdfmt ${version}`),
    ),
    vscode.workspace.onDidChangeWorkspaceFolders(() => {
      void restart()
    }),
    vscode.workspace.onDidGrantWorkspaceTrust(() => {
      void restart()
    }),
    vscode.workspace.onDidChangeConfiguration(event => {
      if (event.affectsConfiguration('gdfmt')) {
        if (process.env.GDFMT_DEBUG_SAVE === '1')
          console.log(
            '[DEBUG-rc1-save] config restart',
            new Date().toISOString(),
          )
        void restart()
      }
    }),
  )
  stop = async () => {
    disposed = true
    await operation
    await stopClients()
  }
  await restart()
}

export async function deactivate(): Promise<void> {
  await stop?.()
  stop = undefined
}
