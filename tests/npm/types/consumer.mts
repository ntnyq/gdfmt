import {
  format,
  formatSync,
  resolveConfig,
  resolveConfigSync,
} from '@gdfmt/gdfmt'
import type { Diagnostic, FormatResult, TextOptions } from '@gdfmt/gdfmt'

const options: TextOptions = {
  language: { godotVersion: '4.7' },
  format: { indentStyle: 'space', indentWidth: 2 },
}
const result: FormatResult = formatSync('var x=1\n', options)
const pending: Promise<FormatResult> = format('var x=1\n', options)
const diagnostics: Diagnostic[] = result.diagnostics
resolveConfigSync('virtual.gd').config.format.lineWidth satisfies number
resolveConfig('virtual.gd').then(value => value.ignored satisfies boolean)
void pending
void diagnostics
// @ts-expect-error Unsupported language profiles must be rejected by declarations.
formatSync('', { language: { godotVersion: '5.0' } })
// @ts-expect-error Filesystem configuration is separate from pure text options.
format('', { extends: ['./config.json'] })
