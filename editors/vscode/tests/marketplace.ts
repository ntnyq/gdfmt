import * as vscode from 'vscode'

declare const EXPECTED_EXTENSION_VERSION: string
declare const EXPECTED_CORE_VERSION: string
declare const EXPECTED_UI_KIND: number

/**
 * Runs with a separate test driver; gdfmt itself must come from the Gallery.
 */
async function runSuite(): Promise<void> {
  const extension = vscode.extensions.getExtension('ntnyq.gdfmt')
  if (!extension) throw new Error('Gallery extension is missing')
  if (extension.extensionUri.path.startsWith('/static/devextensions/'))
    throw new Error('gdfmt must not be loaded as a development extension')
  if (
    extension.packageJSON.version !== EXPECTED_EXTENSION_VERSION ||
    extension.packageJSON.gdfmtVersion !== EXPECTED_CORE_VERSION
  )
    throw new Error('Gallery extension/core versions differ from the release')
  if (vscode.env.uiKind !== EXPECTED_UI_KIND)
    throw new Error('Unexpected extension host')
  if (EXPECTED_UI_KIND === vscode.UIKind.Web && globalThis.crossOriginIsolated)
    throw new Error('Web acceptance must run without cross-origin isolation')
  await extension.activate()
  const document = await vscode.workspace.openTextDocument({
    language: 'gdscript',
    content: 'var value=1\n',
  })
  const editor = await vscode.window.showTextDocument(document)
  const edits = await vscode.commands.executeCommand<vscode.TextEdit[]>(
    'vscode.executeFormatDocumentProvider',
    document.uri,
    { tabSize: 4, insertSpaces: false },
  )
  if (!edits?.length) throw new Error('Gallery formatter returned no edits')
  await editor.edit(builder => {
    for (const edit of edits) builder.replace(edit.range, edit.newText)
  })
  if (document.getText() !== 'var value = 1\n')
    throw new Error(
      `Unexpected Gallery formatting: ${JSON.stringify(document.getText())}`,
    )
  await vscode.commands.executeCommand('gdfmt.restartServer')
  await editor.edit(builder =>
    builder.replace(
      new vscode.Range(0, 0, document.lineCount, 0),
      'var restarted=2\n',
    ),
  )
  const restartedEdits = await vscode.commands.executeCommand<
    vscode.TextEdit[]
  >('vscode.executeFormatDocumentProvider', document.uri, {
    tabSize: 4,
    insertSpaces: false,
  })
  if (!restartedEdits?.length)
    throw new Error('Restarted Gallery formatter returned no edits')
  await editor.edit(builder => {
    for (const edit of restartedEdits) builder.replace(edit.range, edit.newText)
  })
  if (document.getText() !== 'var restarted = 2\n')
    throw new Error('Gallery formatting failed after restart')
  console.log(
    `Gallery ${EXPECTED_EXTENSION_VERSION} / core ${EXPECTED_CORE_VERSION}: activation, formatting and restart passed`,
  )
}

export async function run(): Promise<void> {
  let timeout: ReturnType<typeof setTimeout> | undefined
  try {
    await Promise.race([
      runSuite(),
      new Promise<never>((_resolve, reject) => {
        timeout = setTimeout(
          () => reject(new Error('Gallery consumer timed out')),
          60_000,
        )
      }),
    ])
  } finally {
    clearTimeout(timeout)
  }
}
