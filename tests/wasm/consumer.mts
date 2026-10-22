import { createFormatter } from '@gdfmt/wasm'
import type { FormatResult, TextOptions } from '@gdfmt/wasm'

const options: TextOptions = {
  language: { godotVersion: '4.7' },
  format: { lineWidth: 100 },
}
const formatter = await createFormatter({ wasm: new Uint8Array() })
const result: FormatResult = formatter.formatSync('var x=1\n', options)
result.output?.toUpperCase()
// @ts-expect-error filesystem options belong to the native host
formatter.formatSync('', { cwd: '/tmp' })

formatter.parseConfig('{\"root\":true}')
formatter.resolveTextOptions([{ format: { finalNewline: false } }])
