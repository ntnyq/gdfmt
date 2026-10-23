import assert from 'node:assert/strict'
import * as vscode from 'vscode'
import { documentEdits, UnsupportedLineEndingsError } from '../src/formatting'
import { verifyFormattingBoundaries } from './formatting-boundaries'
import { preservationCases } from './preservation-cases'

async function expectDocumentText(
  document: vscode.TextDocument,
  expected: string,
  action: () => Thenable<unknown>,
): Promise<void> {
  // Command/save acknowledgements can precede document-change notifications.
  // Require both the completed action and exact text, without a fixed delay.
  let timeout: ReturnType<typeof setTimeout> | undefined
  let subscription: vscode.Disposable | undefined
  try {
    await new Promise<void>((done, reject) => {
      let isActionComplete = false
      const check = (): void => {
        if (isActionComplete && document.getText() === expected) done()
      }
      subscription = vscode.workspace.onDidChangeTextDocument(event => {
        if (event.document === document) check()
      })
      timeout = setTimeout(
        () =>
          reject(
            new Error(
              `document did not reach expected text: ${JSON.stringify(document.getText())}`,
            ),
          ),
        5_000,
      )
      void Promise.resolve()
        .then(action)
        .then(() => {
          isActionComplete = true
          check()
        }, reject)
    })
    assert.equal(document.getText(), expected)
  } finally {
    clearTimeout(timeout)
    subscription?.dispose()
  }
}

export async function run(): Promise<void> {
  if (process.env.GDFMT_DEBUG_SAVE === '1') {
    const folder = vscode.workspace.workspaceFolders?.[0]
    assert.ok(folder)
    const probe = await vscode.workspace.openTextDocument(
      vscode.Uri.joinPath(folder.uri, 'ready.txt'),
    )
    const editor = await vscode.window.showTextDocument(probe)
    await editor.edit(builder =>
      builder.replace(new vscode.Range(0, 0, probe.lineCount, 0), 'ready \n'),
    )
    await probe.save()
    console.log(
      '[DEBUG-rc1-save] builtin probe',
      probe.getText(),
      vscode.workspace
        .getConfiguration('files', probe)
        .get('trimTrailingWhitespace'),
    )
    assert.equal(
      probe.getText(),
      'ready\n',
      'built-in save participants must be ready before testing gdfmt',
    )
  }
  if (process.env.GDFMT_DEBUG_SAVE === '1')
    vscode.workspace.onWillSaveTextDocument(event =>
      console.log(
        '[DEBUG-rc1-save] will save',
        new Date().toISOString(),
        event.reason,
        event.document.version,
        event.document.uri.toString(),
      ),
    )
  const extension = vscode.extensions.getExtension('ntnyq.gdfmt')
  assert.ok(extension)
  await extension.activate()
  if (process.env.GDFMT_TEST_GODOT_TOOLS === '1') {
    const godotTools = vscode.extensions.getExtension('geequlim.godot-tools')
    assert.ok(
      godotTools,
      'Godot Tools must be installed in the isolated test profile',
    )
    await godotTools.activate()
    assert.equal(godotTools.isActive, true)
  }
  assert.ok(vscode.workspace.workspaceFolders?.length === 3)
  for (const folder of vscode.workspace.workspaceFolders) {
    const indentWidth =
      folder.name === 'nested' ? 6 : folder.name === 'first' ? 2 : 4
    const document = await vscode.workspace.openTextDocument(
      vscode.Uri.joinPath(folder.uri, 'example.gd'),
    )
    await vscode.window.showTextDocument(document)
    const edit = new vscode.WorkspaceEdit()
    edit.replace(
      document.uri,
      new vscode.Range(0, 0, document.lineCount, 0),
      'func example():\n\tvar value=2\n',
    )
    assert.equal(await vscode.workspace.applyEdit(edit), true)
    const unformatted = document.getText()
    // Editor actions honor the selected default when multiple providers exist.
    await vscode.commands.executeCommand('editor.action.formatDocument')
    assert.notEqual(
      document.getText(),
      unformatted,
      'provider must format unsaved text',
    )
    assert.equal(
      document.getText(),
      `func example():\n${' '.repeat(indentWidth)}var value = 2\n`,
    )
    // Saving must invoke the selected formatter and use the current editor text.
    const change = new vscode.WorkspaceEdit()
    change.replace(
      document.uri,
      new vscode.Range(1, 0, 1, document.lineAt(1).text.length),
      '\tvar value=3',
    )
    await vscode.workspace.applyEdit(change)
    if (process.env.GDFMT_DEBUG_SAVE === '1')
      console.log(
        '[DEBUG-rc1-save] before save',
        new Date().toISOString(),
        folder.name,
        document.version,
        document.isDirty,
        vscode.workspace
          .getConfiguration('editor', document)
          .get('formatOnSave'),
        vscode.workspace
          .getConfiguration('editor', document)
          .get('defaultFormatter'),
      )
    const saved = `func example():\n${' '.repeat(indentWidth)}var value = 3\n`
    await expectDocumentText(document, saved, () => document.save())
    assert.equal(
      new TextDecoder().decode(
        await vscode.workspace.fs.readFile(document.uri),
      ),
      saved,
    )
    if (process.env.GDFMT_DEBUG_SAVE === '1')
      console.log('[DEBUG-rc1-save] saved', document.getText())
    if (process.env.GDFMT_SAVE_REPRO === '1') return
    // Format Document is an editor action with a single undo step.
    await vscode.window.activeTextEditor?.edit(builder =>
      builder.replace(
        new vscode.Range(1, 0, 1, document.lineAt(1).text.length),
        '\tvar value=4',
      ),
    )
    const beforeFormatting = document.getText()
    await vscode.commands.executeCommand('editor.action.formatDocument')
    assert.ok(document.getText().includes('value = 4'))
    await expectDocumentText(document, beforeFormatting, () =>
      vscode.commands.executeCommand('undo'),
    )
  }
  const first = vscode.workspace.workspaceFolders.find(
    folder => folder.name === 'first',
  )
  assert.ok(first)
  for (const [lineEnding, before, after, expectedEol] of [
    ['crlf', 'var x = 1\n', 'var x = 1\r\n', vscode.EndOfLine.CRLF],
    ['lf', 'var x = 1\r\n', 'var x = 1\n', vscode.EndOfLine.LF],
    ['crlf', 'var 名称="😀"\n', 'var 名称 = "😀"\r\n', vscode.EndOfLine.CRLF],
    ['preserve', 'var x=1\r\n', 'var x = 1\r\n', vscode.EndOfLine.CRLF],
  ] as const) {
    const document: vscode.TextDocument =
      await vscode.workspace.openTextDocument(
        vscode.Uri.joinPath(first.uri, 'eol', lineEnding, 'example.gd'),
      )
    const editor = await vscode.window.showTextDocument(document)
    await editor.edit(builder => {
      builder.setEndOfLine(
        before.includes('\r\n') ? vscode.EndOfLine.CRLF : vscode.EndOfLine.LF,
      )
      builder.replace(new vscode.Range(0, 0, document.lineCount, 0), before)
    })
    await vscode.commands.executeCommand('editor.action.formatDocument')
    assert.equal(document.getText(), after)
    assert.equal(document.eol, expectedEol)
    await vscode.commands.executeCommand('editor.action.formatDocument')
    assert.equal(document.getText(), after, 'EOL conversion must be idempotent')
    await expectDocumentText(document, before, () =>
      vscode.commands.executeCommand('undo'),
    )
    assert.equal(
      document.getText(),
      before,
      'formatting and EOL conversion share one undo step',
    )
    await expectDocumentText(document, after, () => document.save())
  }
  for (const test of preservationCases) {
    const document: vscode.TextDocument =
      await vscode.workspace.openTextDocument(
        vscode.Uri.joinPath(first.uri, 'preservation', test.name, 'example.gd'),
      )
    const editor = await vscode.window.showTextDocument(document)
    await editor.edit(builder => {
      builder.setEndOfLine(
        test.source.includes('\r\n')
          ? vscode.EndOfLine.CRLF
          : vscode.EndOfLine.LF,
      )
      builder.replace(
        new vscode.Range(0, 0, document.lineCount, 0),
        test.source,
      )
    })
    const eol = document.eol
    if (test.rejects)
      assert.throws(
        () =>
          documentEdits(document, test.source, test.formatted, test.lineEnding),
        UnsupportedLineEndingsError,
        test.name,
      )
    const expected = test.rejects ? test.source : test.formatted
    await vscode.commands.executeCommand('editor.action.formatDocument')
    assert.equal(document.getText(), expected, test.name)
    assert.equal(
      document.eol,
      eol,
      `${test.name}: retained bytes determine EOL`,
    )
    await vscode.commands.executeCommand('editor.action.formatDocument')
    assert.equal(document.getText(), expected, `${test.name}: idempotence`)
    await expectDocumentText(document, expected, () => document.save())
    assert.equal(document.getText(), expected, `${test.name}: save formatting`)
    assert.equal(
      new TextDecoder().decode(
        await vscode.workspace.fs.readFile(document.uri),
      ),
      expected,
    )
  }
  const untitled = await vscode.workspace.openTextDocument({
    language: 'gdscript',
    content: 'var unsaved=1\n',
  })
  await vscode.window.showTextDocument(untitled)
  await verifyFormattingBoundaries(untitled)
  await vscode.commands.executeCommand('editor.action.formatDocument')
  assert.equal(untitled.getText(), 'var unsaved = 1\n')
  await vscode.commands.executeCommand('gdfmt.restartServer')
  await vscode.window.activeTextEditor?.edit(builder =>
    builder.replace(
      new vscode.Range(0, 0, untitled.lineCount, 0),
      'var after_restart=1\n',
    ),
  )
  await vscode.commands.executeCommand('editor.action.formatDocument')
  assert.equal(untitled.getText(), 'var after_restart = 1\n')
}
