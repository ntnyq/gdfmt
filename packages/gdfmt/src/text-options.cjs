function text(value, name) {
  if (typeof value !== 'string' || !value.isWellFormed()) {
    throw new TypeError(`${name} must be a well-formed Unicode string`)
  }
  return value
}

function object(value, name) {
  if (
    value === null ||
    typeof value !== 'object' ||
    (Object.getPrototypeOf(value) !== Object.prototype &&
      Object.getPrototypeOf(value) !== null)
  ) {
    throw new TypeError(`${name} must be a plain object`)
  }
  return value
}

function encodeOptions(options) {
  object(options, 'options')
  // Reject values JSON would silently discard/coerce (undefined, NaN, toJSON,
  // class instances), so invalid sync and async requests have the same meaning.
  const active = new Set()
  function encode(value) {
    if (typeof value === 'number' && Number.isFinite(value)) return value
    if (typeof value === 'string') return text(value, 'option')
    if (value === null || typeof value === 'boolean') return value
    if (typeof value === 'object') {
      object(value, 'option')
      if (
        active.has(value) ||
        Reflect.ownKeys(value).some(
          key =>
            typeof key === 'symbol' ||
            !Object.prototype.propertyIsEnumerable.call(value, key),
        ) ||
        'toJSON' in value
      ) {
        throw new TypeError('options must contain only JSON properties')
      }
      active.add(value)
      const copy = Object.create(null)
      for (const key of Object.keys(value)) copy[key] = encode(value[key])
      active.delete(value)
      return copy
    }
    throw new TypeError('options must contain only finite JSON values')
  }
  return JSON.stringify(encode(options))
}

module.exports = { text, object, encodeOptions }
