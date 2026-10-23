import * as vscode from 'vscode'
import {
  documentEdits,
  UnsupportedLineEndingsError,
} from '../../src/formatting'
import { verifyFormattingBoundaries } from '../formatting-boundaries'
import { preservationCases } from '../preservation-cases'
import { TestFileSystem } from './filesystem'

function check(condition: unknown, message: string): asserts condition {
  if (!condition) throw new Error(message)
}
function equal(actual: unknown, expected: unknown): void {
  if (actual !== expected)
    throw new Error(
      `Expected ${JSON.stringify(expected)}, received ${JSON.stringify(actual)}`,
    )
}
const encode = (text: string): Uint8Array => new TextEncoder().encode(text)
const pause = (): Promise<void> => new Promise(done => setTimeout(done, 120))

async function runSuite(): Promise<void> {
  equal(vscode.env.uiKind, vscode.UIKind.Web)
  equal(globalThis.crossOriginIsolated, false)
  const extension = vscode.extensions.getExtension('ntnyq.gdfmt')
  check(extension, 'Web extension is installed')
  await extension.activate()
  check(
    vscode.workspace.workspaceFile,
    'tests must start in an untitled multi-root workspace',
  )
  const filesystem = new TestFileSystem()
  const provider = vscode.workspace.registerFileSystemProvider(
    'gdfmt-test',
    filesystem,
    { isCaseSensitive: true },
  )
  const root = vscode.Uri.parse('gdfmt-test://workspace/')
  const initialFolders = new Promise<void>(done => {
    const subscription = vscode.workspace.onDidChangeWorkspaceFolders(() => {
      subscription.dispose()
      done()
    })
  })
  check(
    vscode.workspace.updateWorkspaceFolders(0, 0, {
      uri: root,
      name: 'gdfmt-test',
    }),
    'add virtual test root',
  )
  await initialFolders
  const configuration = vscode.workspace.getConfiguration()
  await configuration.update(
    '[gdscript]',
    { 'editor.defaultFormatter': 'ntnyq.gdfmt', 'editor.formatOnSave': true },
    vscode.ConfigurationTarget.Workspace,
  )
  await configuration.update(
    'gdfmt.serverPath',
    '/must-not-execute-in-a-browser',
    vscode.ConfigurationTarget.Workspace,
  )
  const file = (path: string): vscode.Uri => vscode.Uri.joinPath(root, path)
  async function write(path: string, text: string): Promise<void> {
    await vscode.workspace.fs.writeFile(file(path), encode(text))
    await pause()
  }
  async function format(
    path: string,
    source: string,
  ): Promise<vscode.TextDocument> {
    const uri = file(path)
    try {
      await vscode.workspace.fs.stat(uri)
    } catch {
      await vscode.workspace.fs.writeFile(uri, encode(''))
    }
    const document = await vscode.workspace.openTextDocument(uri)
    await vscode.languages.setTextDocumentLanguage(document, 'gdscript')
    const editor = await vscode.window.showTextDocument(document)
    await editor.edit(builder => {
      builder.setEndOfLine(
        source.includes('\r\n') ? vscode.EndOfLine.CRLF : vscode.EndOfLine.LF,
      )
      builder.replace(new vscode.Range(0, 0, document.lineCount, 0), source)
    })
    await vscode.commands.executeCommand('editor.action.formatDocument')
    return document
  }
  await vscode.workspace.fs.createDirectory(file('config'))
  await vscode.workspace.fs.createDirectory(file('nested'))
  await write(
    'config/base.settings',
    '{// shared base\n "format":{"indentStyle":"space","indentWidth":2,"lineEnding":"preserve"}}',
  )
  await write(
    '.gdfmtrc.jsonc',
    '{"root":true,"extends":["config/base.settings"]}',
  )
  const source = 'func example():\r\n\tvar 名称="😀"\r\n'
  const document = await format('first.gd', source)
  equal(document.getText(), 'func example():\r\n  var 名称 = "😀"\r\n')
  console.log('gdfmt Web: CRLF and Unicode formatting passed')
  await vscode.commands.executeCommand('undo')
  equal(document.getText(), source)
  await document.save()
  equal(document.getText(), 'func example():\r\n  var 名称 = "😀"\r\n')
  equal(
    new TextDecoder().decode(await vscode.workspace.fs.readFile(document.uri)),
    document.getText(),
  )

  console.log('gdfmt Web: save and undo passed')

  // Hold a configuration read to prove stale requests cannot apply edits.
  const editor = await vscode.window.showTextDocument(document)
  const pendingSource = 'func example():\n\tvar pending=1\n'
  await editor.edit(builder => {
    builder.setEndOfLine(vscode.EndOfLine.LF)
    builder.replace(
      new vscode.Range(0, 0, document.lineCount, 0),
      pendingSource,
    )
  })
  const heldConfig = filesystem.holdNextRead('/config/base.settings')
  const pendingFormat = vscode.commands.executeCommand(
    'editor.action.formatDocument',
  )
  await heldConfig.entered
  await write(
    'config/base.settings',
    '{"format":{"indentStyle":"space","indentWidth":4}}',
  )
  heldConfig.release()
  await pendingFormat
  equal(document.getText(), pendingSource)
  await vscode.commands.executeCommand('editor.action.formatDocument')
  equal(document.getText(), 'func example():\n    var pending = 1\n')
  await write(
    'config/base.settings',
    '{"format":{"indentStyle":"space","indentWidth":2}}',
  )

  const heldDocument = filesystem.holdNextRead('/config/base.settings')
  const staleFormat = vscode.commands.executeCommand(
    'editor.action.formatDocument',
  )
  await heldDocument.entered
  await editor.edit(builder =>
    builder.replace(
      new vscode.Range(0, 0, document.lineCount, 0),
      'var current=2\n',
    ),
  )
  heldDocument.release()
  await staleFormat
  equal(document.getText(), 'var current=2\n')
  console.log(
    'gdfmt Web: pending results invalidated by custom configuration and document changes',
  )

  // Discovery layering and multiple roots use URI ownership rather than paths.
  await write('nested/.gdfmtrc.json', '{"format":{"indentWidth":4}}')
  equal(
    (
      await format('nested/second.gd', 'func example():\n\tvar x=1\n')
    ).getText(),
    'func example():\n    var x = 1\n',
  )
  console.log('gdfmt Web: configuration layering passed')
  const changed = new Promise<void>(done => {
    const subscription = vscode.workspace.onDidChangeWorkspaceFolders(() => {
      subscription.dispose()
      done()
    })
  })
  check(
    vscode.workspace.updateWorkspaceFolders(
      vscode.workspace.workspaceFolders?.length ?? 1,
      0,
      { uri: file('nested'), name: 'nested' },
    ),
    'add a second workspace folder',
  )
  await Promise.race([
    changed,
    new Promise<never>((_resolve, reject) =>
      setTimeout(() => reject(new Error('workspace update timed out')), 10_000),
    ),
  ])
  console.log('gdfmt Web: workspace folder update passed')
  await write(
    'nested/.gdfmtrc.json',
    '{"root":true,"format":{"indentStyle":"space","indentWidth":4}}',
  )
  equal(
    (
      await format('nested/second.gd', 'func example():\n\tvar x=1\n')
    ).getText(),
    'func example():\n    var x = 1\n',
  )
  // A base outside the owning (nested) root must not be silently inherited.
  await write('nested/.gdfmtrc.json', '{"extends":["../config/base.settings"]}')
  equal((await format('nested/second.gd', 'var x=1\n')).getText(), 'var x=1\n')

  // Source errors have UTF-16 diagnostics and never produce edits.
  const invalid = 'var text="😀"; var =\n'
  equal((await format('first.gd', invalid)).getText(), invalid)
  const findings = vscode.languages
    .getDiagnostics(document.uri)
    .filter(item => item.source === 'gdfmt')
  check(findings.length > 0, 'source errors must produce diagnostics')
  const identifierError = findings.find(item => item.code === 'PAR004')
  check(identifierError, 'expected identifier diagnostic')
  equal(identifierError.range.start.line, 0)
  equal(identifierError.range.start.character, 19)
  equal(identifierError.range.end.character, 20)

  await write('explicit.jsonc', '{"format":{"finalNewline":false}}')
  const settings = vscode.workspace.getConfiguration('gdfmt', document.uri)
  await settings.update(
    'configPath',
    'explicit.jsonc',
    vscode.ConfigurationTarget.WorkspaceFolder,
  )
  await pause()
  equal((await format('first.gd', 'var x=1\n')).getText(), 'var x = 1')
  await write('explicit.jsonc', '{"format":{"lineEnding":"crlf"}}')
  equal((await format('first.gd', 'var x=1\n')).getText(), 'var x = 1\r\n')
  await write('explicit.jsonc', '{"format":{"lineEnding":"lf"}}')
  equal((await format('first.gd', 'var x=1\r\n')).getText(), 'var x = 1\n')
  await settings.update(
    'configPath',
    '',
    vscode.ConfigurationTarget.WorkspaceFolder,
  )
  await pause()
  await settings.update(
    'enable',
    false,
    vscode.ConfigurationTarget.WorkspaceFolder,
  )
  equal((await format('first.gd', 'var x=1\n')).getText(), 'var x=1\n')
  await settings.update(
    'enable',
    true,
    vscode.ConfigurationTarget.WorkspaceFolder,
  )
  await pause()

  await vscode.workspace.fs.createDirectory(file('preservation'))
  for (const test of preservationCases) {
    const path = `preservation/${test.name}`
    await vscode.workspace.fs.createDirectory(file(path))
    await write(
      `${path}/.gdfmtrc.json`,
      JSON.stringify({
        root: true,
        format: {
          lineEnding: test.lineEnding,
          finalNewline: test.finalNewline,
        },
      }),
    )
    const document = await format(`${path}/example.gd`, test.source)
    if (test.rejects) {
      let rejected = false
      try {
        documentEdits(document, test.source, test.formatted, test.lineEnding)
      } catch (error) {
        if (!(error instanceof UnsupportedLineEndingsError)) throw error
        rejected = true
      }
      check(rejected, `${test.name}: mixed line endings must be rejected`)
    }
    const expected = test.rejects ? test.source : test.formatted
    const eol = test.source.includes('\r\n')
      ? vscode.EndOfLine.CRLF
      : vscode.EndOfLine.LF
    equal(document.getText(), expected)
    equal(document.eol, eol)
    await vscode.commands.executeCommand('editor.action.formatDocument')
    equal(document.getText(), expected)
    await document.save()
    equal(document.getText(), expected)
    equal(
      new TextDecoder().decode(
        await vscode.workspace.fs.readFile(document.uri),
      ),
      expected,
    )
  }
  console.log(
    'gdfmt Web: protected regions and multiline strings retain exact bytes',
  )

  for (const config of [
    '{"overrides":[]}',
    '{"ignore":[]}',
    '{"extends":[".gdfmtrc.jsonc"]}',
    '{"format":{"lineWidth":1}}',
    '{"schemaVersion":2}',
    '{"extends":["missing.json"]}',
    '{"extends":["../outside.json"]}',
    '{"extends":["/absolute.json"]}',
    '{"extends":["https://example.com/config.json"]}',
    '{"extends":["config\\\\base.json"]}',
  ]) {
    await write('.gdfmtrc.jsonc', config)
    equal((await format('first.gd', 'var x=1\n')).getText(), 'var x=1\n')
  }
  await write('.gdfmtrc.jsonc', '{"root":true}')
  // Ignore files do not select which already-open Web documents can format.
  await write('.gitignore', '*.gd\n')
  await write('.gdfmtignore', '*.gd\n')
  equal((await format('first.gd', 'var x=1\n')).getText(), 'var x = 1\n')
  for (const path of [
    '../outside.json',
    '/absolute.json',
    'config\\base.json',
  ]) {
    await settings.update(
      'configPath',
      path,
      vscode.ConfigurationTarget.WorkspaceFolder,
    )
    await pause()
    equal((await format('first.gd', 'var x=1\n')).getText(), 'var x=1\n')
  }
  await settings.update(
    'configPath',
    '',
    vscode.ConfigurationTarget.WorkspaceFolder,
  )
  await pause()

  // Inclusive inheritance/layer bounds, including replayed cached parents.
  await vscode.workspace.fs.createDirectory(file('limits'))
  for (let index = 0; index < 33; index++)
    await vscode.workspace.fs.writeFile(
      file(`limits/${index}.json`),
      encode(
        JSON.stringify(index === 32 ? {} : { extends: [`${index + 1}.json`] }),
      ),
    )
  await write('.gdfmtrc.jsonc', '{"root":true,"extends":["limits/1.json"]}')
  equal((await format('first.gd', 'var x=1\n')).getText(), 'var x=1\n')
  await write('.gdfmtrc.jsonc', '{"root":true,"extends":["limits/2.json"]}')
  equal((await format('first.gd', 'var x=1\n')).getText(), 'var x = 1\n')
  for (const count of [127, 128]) {
    await write(
      '.gdfmtrc.jsonc',
      JSON.stringify({
        root: true,
        extends: Array.from({ length: count }, () => 'limits/32.json'),
      }),
    )
    equal(
      (await format('first.gd', 'var x=1\n')).getText(),
      count === 127 ? 'var x = 1\n' : 'var x=1\n',
    )
  }
  await write('.gdfmtrc.jsonc', '{}'.padEnd(1024 * 1024 + 1, ' '))
  equal((await format('first.gd', 'var x=1\n')).getText(), 'var x=1\n')
  await write('.gdfmtrc.jsonc', '{"root":true}')
  await write('.editorconfig', 'root=true\n[*.gd]\nindent_style=space')
  equal((await format('first.gd', 'var x=1\n')).getText(), 'var x=1\n')
  await vscode.workspace.fs.delete(file('.editorconfig'))
  await pause()
  const untitled = await vscode.workspace.openTextDocument({
    language: 'gdscript',
    content: 'var unsaved=1\n',
  })
  await vscode.window.showTextDocument(untitled)
  await verifyFormattingBoundaries(untitled)
  await vscode.commands.executeCommand('editor.action.formatDocument')
  equal(untitled.getText(), 'var unsaved = 1\n')
  await vscode.commands.executeCommand('gdfmt.restartServer')
  equal(
    (await format('first.gd', 'var restarted=1\n')).getText(),
    'var restarted = 1\n',
  )
  provider.dispose()
  console.log(
    'gdfmt Web: virtual filesystem, config inheritance, diagnostics, save/undo, multiple roots, restrictions and restart passed',
  )
}

export async function run(): Promise<void> {
  let timeout: ReturnType<typeof setTimeout> | undefined
  try {
    await Promise.race([
      runSuite(),
      new Promise<never>((_resolve, reject) => {
        timeout = setTimeout(
          () => reject(new Error('Web Extension Host suite timed out')),
          90_000,
        )
      }),
    ])
  } finally {
    clearTimeout(timeout)
  }
}
