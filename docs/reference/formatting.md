# Script formatting

gdfmt formats complete scripts, including expressions, comments, lambdas, declarations and suites. Use the defaults or adjust [project configuration](/guide/configuration). See [language support](/reference/compatibility) for known boundaries.

## Current behavior

The formatter normalizes operators, calls, arrays, dictionaries, groups, annotations, signatures, declarations, accessors, statements and match patterns. Named functions, classes, branches, loops and accessor bodies use indented multiline suites. Semicolon-separated statements become separate lines while retaining the original semicolon tokens. A lambda with one simple, comment-free statement uses an inline body; other lambdas use a block. Block lambdas inside containers keep their suite indentation relative to the enclosing statement, because continuation columns are not lexer indentation-stack entries.

Argument, parameter and element lists wrap to `format.lineWidth`. `indentStyle` and `indentWidth` apply to suites and continuation lines. Columns count Unicode scalar values and tab stops. Unbreakable tokens and type references may exceed the width: Godot rejects a newline immediately after a collection type's opening bracket. Parentheses, commas, semicolons, dictionary separator styles, literal spellings and BOMs remain intact. Multiline literal contents retain their exact bytes, including their original line endings and indentation.

Own-line comments stay anchored before the next token, inline comments stay after the preceding token, and dangling comments stay inside closing delimiters or at the end of their suite, property or match block. Documentation, region and warning comments retain spelling and order. Annotation order and targets are preserved. Comments force line boundaries without swallowing following code.

`lineEnding` selects LF, CRLF or the first source newline (`preserve`, falling back to LF). It affects layout, not literal contents. `maxBlankLines` caps existing blank lines; leading and trailing empty lines are removed. `finalNewline` controls one terminal newline on nonempty files. Host configuration defaults to LF and a final newline; pure Rust `FormatOptions` defaults to preserving the source newline convention. Standalone expression trees retain their expression entry point rather than applying file-final-newline policy.

```gdscript
func run( value : int=1 ): return [value,2] # result
```

becomes:

```gdscript
func run(value: int = 1):
	return [value, 2] # result
```

## Protected regions

Use paired standalone comments:

```gdscript
# gdfmt: formatter-off
var table=[ 1,    2,3 ]
# gdfmt: formatter-on
var other=4
```

The region, including both marker lines, retains its exact original bytes; the final declaration becomes `var other = 4`. Region contents take precedence over indentation, blank-line, line-ending and final-newline settings, so mixed line endings can remain intentionally. A BOM remains at the file start. Multiple disjoint regions are supported.

Only real standalone comment tokens are directives. Marker text inside strings or suffix comments has no effect. Markers must pair in order and cannot nest; unmatched or nested markers produce `FMT002`, no output and CLI exit 1. Unknown `gdfmt:` comments are ordinary comments. `check --syntax-only` does not interpret formatter directives.

The whole file, including protected contents, must parse. Regions do not disable syntax diagnostics. The facade verifies protected bytes as well as reparsing, syntax/comment equivalence and idempotence. If restoring a region would conflict with surrounding indentation or an expression boundary, it conservatively retains the complete top-level declaration/statement touched by the region. This may leave additional surrounding lines unchanged; unrelated top-level units still format. If validation still fails, no output is returned.

Lint suppression directives, lint rules and semantic auto-fixes are outside the formatter scope. `check` remains a syntax/formatting check; use `format` to apply layout changes.

## Command line

Use `gdfmt .` to write changes, `gdfmt --check .` to check without writing, or `gdfmt --diff .` to preview changes. See the [CLI reference](/reference/cli) for stdin, reporters and exit codes.

## Rust and correctness gates

`gdfmt_formatter` accepts a lossless tree, its parser diagnostics and layout options. Its normal dependencies contain no parser or filesystem code. `gdfmt::format(source, &FormatRequest)` combines parsing and formatting and returns `FormatResult { output, changed, diagnostics }`.

The facade returns writable output only if the input has no parser errors, the output reparses with the same profile/limits, normalized node topology and significant token spellings agree, comment text/adjacent tokens/own-line roles/dangling block ownership agree, and a second formatting pass is identical. Failure returns no output and an internal diagnostic. Recovery nodes and error diagnostics are refused independently by the formatter.

Reproduction commands, fixture organization and engine comparisons are documented in [contributing](https://github.com/ntnyq/gdfmt/blob/main/CONTRIBUTING.md).
