const { binding, isAbsolute } = require('./binding.cjs')

const { text, object, encodeOptions } = require('./text-options.cjs')

function resolveOptions(path, options) {
  text(path, 'path')
  object(options, 'options')
  for (const key of Reflect.ownKeys(options)) {
    if (key !== 'cwd' && key !== 'config')
      throw new TypeError(`unknown resolve option: ${String(key)}`)
  }
  const cwd = text(
    options.cwd === undefined ? process.cwd() : options.cwd,
    'cwd',
  )
  if (!isAbsolute(cwd)) throw new TypeError('cwd must be absolute')
  const config =
    options.config === undefined ? undefined : text(options.config, 'config')
  return [path, cwd, config]
}

function formatSync(source, options = {}) {
  return JSON.parse(
    binding.formatSync(text(source, 'source'), encodeOptions(options)),
  )
}

async function format(source, options = {}) {
  return JSON.parse(
    await binding.formatAsync(text(source, 'source'), encodeOptions(options)),
  )
}

function resolveConfigSync(path, options = {}) {
  return JSON.parse(binding.resolveConfigSync(...resolveOptions(path, options)))
}

async function resolveConfig(path, options = {}) {
  return JSON.parse(
    await binding.resolveConfigAsync(...resolveOptions(path, options)),
  )
}

module.exports = { formatSync, format, resolveConfigSync, resolveConfig }
