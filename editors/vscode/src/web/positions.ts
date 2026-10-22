/**
 * Maps UTF-8 byte offsets to JavaScript UTF-16 offsets in one source pass.
 */
export function utf16Offsets(
  source: string,
  offsets: number[],
): Map<number, number> {
  const requested = new Set(offsets)
  const result = new Map<number, number>()
  let bytes = 0
  let units = 0
  for (const character of source) {
    if (requested.has(bytes)) result.set(bytes, units)
    const code = character.codePointAt(0) ?? 0
    bytes += code <= 0x7f ? 1 : code <= 0x7ff ? 2 : code <= 0xffff ? 3 : 4
    units += character.length
  }
  if (requested.has(bytes)) result.set(bytes, units)
  if (result.size !== requested.size)
    throw new Error(
      'diagnostic splits a UTF-8 boundary or lies outside the source',
    )
  return result
}

/**
 * Computes a single replacement without splitting surrogate pairs or CRLF.
 */
export function replacement(
  source: string,
  output: string,
): { start: number; end: number; text: string } | undefined {
  if (source === output) return undefined
  let start = 0
  while (
    start < source.length &&
    start < output.length &&
    source[start] === output[start]
  )
    start++
  if (
    start > 0 &&
    ((source.charCodeAt(start - 1) >= 0xd800 &&
      source.charCodeAt(start - 1) <= 0xdbff) ||
      (source[start - 1] === '\r' && source[start] === '\n'))
  )
    start--
  let end = source.length
  let outputEnd = output.length
  while (
    end > start &&
    outputEnd > start &&
    source[end - 1] === output[outputEnd - 1]
  ) {
    end--
    outputEnd--
  }
  if (
    (source.charCodeAt(end) >= 0xdc00 && source.charCodeAt(end) <= 0xdfff) ||
    (source[end - 1] === '\r' && source[end] === '\n')
  ) {
    end++
    outputEnd++
  }
  return { start, end, text: output.slice(start, outputEnd) }
}
