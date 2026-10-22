import { createBindings } from './binding.js'
import { text, object, encodeOptions } from './text-options.mjs'

/**
 * Creates an independent single-threaded formatter. Pass bytes or a compiled
 * module in hosts without URL fetching. A trap requires a new formatter.
 */
export async function createFormatter(options = {}) {
  object(options, 'initialization options')
  if (Reflect.ownKeys(options).some(key => key !== 'wasm'))
    throw new TypeError('unknown initialization option')
  const bindings = createBindings()
  await bindings.init({
    module_or_path:
      options.wasm ?? new URL('./binding_bg.wasm', import.meta.url),
  })
  let failed = false
  function invoke(operation) {
    if (failed)
      throw new Error('formatter trapped; call createFormatter() to recover')
    try {
      return JSON.parse(operation())
    } catch (error) {
      if (error instanceof WebAssembly.RuntimeError) failed = true
      throw error
    }
  }
  return {
    parseConfig(source) {
      return invoke(() => bindings.parseConfig(text(source, 'configuration')))
    },
    resolveTextOptions(layers) {
      if (!Array.isArray(layers)) throw new TypeError('layers must be an array')
      if (layers.length > 128)
        throw new RangeError('configuration layer limit exceeded')
      return invoke(() =>
        bindings.resolveTextOptions(
          `[${layers.map(layer => encodeOptions(layer)).join(',')}]`,
        ),
      )
    },
    formatSync(source, options = {}) {
      return invoke(() =>
        bindings.formatText(text(source, 'source'), encodeOptions(options)),
      )
    },
  }
}
