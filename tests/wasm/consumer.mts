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
const parsed = formatter.parseConfig(
  '{"$schema":"schema.json","schemaVersion":1}',
)
parsed.$schema satisfies string | undefined
parsed.schemaVersion satisfies 1 | undefined
formatter.resolveTextOptions([{ format: { finalNewline: false } }])
// @ts-expect-error null is not a WASM source or a request for default loading
createFormatter({ wasm: null })
