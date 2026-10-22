interface PreservationCase {
  name: string
  source: string
  formatted: string
  lineEnding: 'lf' | 'crlf'
  finalNewline: boolean
  rejects: boolean
}

/**
 * Core output retains protected bytes even when structural newlines change.
 * Both editor hosts must either reproduce that output exactly or leave input intact.
 */
export const preservationCases: readonly PreservationCase[] = (
  ['lf', 'crlf'] as const
).flatMap(sourceEnding => {
  const newline = sourceEnding === 'crlf' ? '\r\n' : '\n'
  const targetEnding = sourceEnding === 'lf' ? 'crlf' : 'lf'
  const targetNewline = targetEnding === 'crlf' ? '\r\n' : '\n'
  const protectedText = [
    '# gdfmt: formatter-off',
    'var kept=[ 1,2 ]',
    '# gdfmt: formatter-on',
    '',
  ].join(newline)
  const cases: PreservationCase[] = [
    {
      name: `protected-only-${sourceEnding}`,
      source: protectedText,
      formatted: protectedText,
      lineEnding: targetEnding,
      finalNewline: true,
      rejects: false,
    },
    {
      name: `protected-mixed-${sourceEnding}`,
      source: `${protectedText}var changed=1${newline}`,
      formatted: `${protectedText}var changed = 1${targetNewline}`,
      lineEnding: targetEnding,
      finalNewline: true,
      rejects: true,
    },
    {
      name: `protected-matching-${sourceEnding}`,
      source: `${protectedText}var changed=1${newline}`,
      formatted: `${protectedText}var changed = 1${newline}`,
      lineEnding: sourceEnding,
      finalNewline: true,
      rejects: false,
    },
  ]
  for (const quote of ['"""', "'''"]) {
    const name = quote === '"""' ? 'double' : 'single'
    const literal = `${quote}first${newline}second${quote}`
    cases.push(
      {
        name: `multiline-${name}-mixed-${sourceEnding}`,
        source: `var text=${literal}${newline}var changed=1${newline}`,
        formatted: `var text = ${literal}${targetNewline}var changed = 1${targetNewline}`,
        lineEnding: targetEnding,
        finalNewline: true,
        rejects: true,
      },
      {
        name: `multiline-${name}-matching-${sourceEnding}`,
        source: `var text=${literal}${newline}var changed=1${newline}`,
        formatted: `var text = ${literal}${newline}var changed = 1${newline}`,
        lineEnding: sourceEnding,
        finalNewline: true,
        rejects: false,
      },
      {
        name: `multiline-${name}-only-${sourceEnding}`,
        source: `var text=${literal}`,
        formatted: `var text = ${literal}`,
        lineEnding: targetEnding,
        finalNewline: false,
        rejects: false,
      },
    )
  }
  return cases
})
