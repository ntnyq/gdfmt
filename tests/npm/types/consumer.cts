import { format, formatSync, resolveConfigSync } from '@gdfmt/gdfmt'
import type { ConfigResolution, FormatResult } from '@gdfmt/gdfmt'

const result: FormatResult = formatSync('var x=1\n')
const pending: Promise<FormatResult> = format('var x=1\n')
const config: ConfigResolution = resolveConfigSync('virtual.gd')
void result
void pending
void config
// @ts-expect-error Async results require awaiting.
const invalid: FormatResult = format('')
void invalid

formatSync('', {
  format: {
    arrayLayout: 'preserve',
    dictionaryLayout: 'multiline',
    enumLayout: 'auto',
    dictionarySpacing: 'compact',
    trailingCommaLayout: 'auto',
    blankLinesAroundDefinitions: 'preserve',
    lambdaBodyStyle: 'auto',
    inlineCommentSpacing: 1,
  },
})
// @ts-expect-error Spacing is a finite integer union.
format('', { format: { inlineCommentSpacing: 1.5 } })
