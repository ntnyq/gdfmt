import { format, formatSync, resolveConfigSync } from 'gdfmt'
import type { ConfigResolution, FormatResult } from 'gdfmt'

const result: FormatResult = formatSync('var x=1\n')
const pending: Promise<FormatResult> = format('var x=1\n')
const config: ConfigResolution = resolveConfigSync('virtual.gd')
void result
void pending
void config
// @ts-expect-error Async results require awaiting.
const invalid: FormatResult = format('')
void invalid
