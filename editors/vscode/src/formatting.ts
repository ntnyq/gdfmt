import * as vscode from 'vscode'
import { replacement } from './web/positions'

/**
 * The validated output cannot be represented by a VS Code text buffer.
 */
export class UnsupportedLineEndingsError extends Error {}

/**
 * Reproduce the validated output using VS Code's single buffer EOL. Refuse
 * output that would require rewriting protected bytes or multiline strings.
 */
export function documentEdits(
  document: vscode.TextDocument,
  source: string,
  output: string,
  lineEnding: 'lf' | 'crlf' | 'preserve' | undefined,
): vscode.TextEdit[] {
  if (source === output) return []
  // Configuration describes structural newlines; retained text may use another
  // ending. Only the actual core output determines which EOL can represent it.
  const newline = output.match(/\r\n|\r|\n/)?.[0]
  const requestedEol =
    lineEnding === 'crlf'
      ? vscode.EndOfLine.CRLF
      : lineEnding === 'lf'
        ? vscode.EndOfLine.LF
        : document.eol
  const eol =
    newline === '\r\n'
      ? vscode.EndOfLine.CRLF
      : newline === '\n'
        ? vscode.EndOfLine.LF
        : requestedEol
  const targetNewline = eol === vscode.EndOfLine.CRLF ? '\r\n' : '\n'
  if (output.replace(/\r\n|\r|\n/g, targetNewline) !== output)
    throw new UnsupportedLineEndingsError(
      'VS Code cannot preserve this formatter output with a single buffer EOL. No edits were applied, to retain protected regions and multiline strings exactly. Try lineEnding: preserve when the input uses a uniform EOL; use the CLI if the output still contains mixed line endings.',
    )
  const normalized = output.replace(
    /\r\n|\r|\n/g,
    document.eol === vscode.EndOfLine.CRLF ? '\r\n' : '\n',
  )
  const edit = replacement(source, normalized)
  const edits = edit
    ? [
        vscode.TextEdit.replace(
          new vscode.Range(
            document.positionAt(edit.start),
            document.positionAt(edit.end),
          ),
          edit.text,
        ),
      ]
    : []
  if (eol !== document.eol) edits.push(vscode.TextEdit.setEndOfLine(eol))
  return edits
}
