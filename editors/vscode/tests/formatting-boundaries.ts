import * as vscode from 'vscode'
import { documentEdits, UnsupportedLineEndingsError } from '../src/formatting'

/**
 * Run against both real Extension Hosts, where VS Code owns ranges and EOL edits.
 */
export async function verifyFormattingBoundaries(
  document: vscode.TextDocument,
): Promise<void> {
  const source = document.getText()
  for (const range of [
    new vscode.Range(0, 0, 0, 0),
    new vscode.Range(0, 0, 0, 3),
    new vscode.Range(0, 0, document.lineCount, 0),
  ]) {
    const edits = await vscode.commands.executeCommand<
      vscode.TextEdit[] | undefined
    >('vscode.executeFormatRangeProvider', document.uri, range, {
      tabSize: 2,
      insertSpaces: true,
    })
    if (edits?.length)
      throw new Error('range formatting must not return whole-document edits')
  }
  for (const output of ['var x = 1\r\nvar y = 2\n', 'var x = 1\r']) {
    let rejected = false
    try {
      documentEdits(document, source, output, 'preserve')
    } catch (error) {
      if (!(error instanceof UnsupportedLineEndingsError)) throw error
      rejected = true
    }
    if (!rejected)
      throw new Error('preserve must not normalize mixed or bare-CR output')
  }
  if (documentEdits(document, source, source, 'crlf').length)
    throw new Error('unchanged core output must leave the buffer EOL alone')
  if (document.getText() !== source)
    throw new Error('boundary checks must leave the document unchanged')
}
