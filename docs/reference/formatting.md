# Script formatting (P4–P5)

P3 is [accepted with explicit exclusions](/development/syntax-audit). The formatter now lays out comments, lambdas, declarations and suites as well as expressions. See the [P4 acceptance record](/development/formatter-acceptance) for source-build alpha acceptance, pinned project trials and explicit exclusions.

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

## CLI

```sh
cargo run -p gdfmt_cli -- format src addons
cargo run -p gdfmt_cli -- src/player.gd
cargo run -p gdfmt_cli -- check .
cargo run -p gdfmt_cli -- --check .
cargo run -p gdfmt_cli -- check . --syntax-only
cargo run -p gdfmt_cli -- --diff src/player.gd
cargo run -p gdfmt_cli -- --stdin-filepath src/player.gd < input.gd
```

`format` and the default command write valid changed files. `check` compares syntax and script formatting; it does not run lint rules. `--syntax-only` skips formatting. `--diff` prints whole-file unified replacement hunks without writing. `format -` reads stdin; `--stdin-filepath` also resolves configuration by the virtual path's nearest existing parent. Successful stdin formatting emits only source to stdout, or an `output` field with the JSON reporter. Invalid stdin emits no formatted source.

Exit codes are 0 for success, 1 for source errors or read-only formatting differences, and 2 for arguments/configuration/I/O/internal failures. JSON file reports include `changed` and structured diagnostics; syntax-only reports omit `changed`. File paths are sorted and deduplicated using the existing discovery rules.

All configurations and input reads are preflighted before writes. Replacements use a same-directory temporary file, retain permissions and compare current content to the source read before replacement. Invalid files remain unchanged; other valid files in a batch may be formatted. Atomic replacement is per file.

## Rust and correctness gates

`gdfmt_formatter` accepts a lossless tree, its parser diagnostics and layout options. Its normal dependencies contain no parser or filesystem code. `gdfmt::format(source, &FormatRequest)` combines parsing and formatting and returns `FormatResult { output, changed, diagnostics }`.

The facade returns writable output only if the input has no parser errors, the output reparses with the same profile/limits, normalized node topology and significant token spellings agree, comment text/adjacent tokens/own-line roles/dangling block ownership agree, and a second formatting pass is identical. Failure returns no output and an internal diagnostic. Recovery nodes and error diagnostics are refused independently by the formatter.

The Doc printer supports text, concatenation, groups, indentation, soft/hard/conditional lines and line suffixes. Tests cover fit boundaries, following text, nested groups, suffixes, CRLF, long tokens, comment insertion at expression-token boundaries, and reviewed whole-script snapshots at widths 40/100. Every parser-accepted script/profile is checked under four layout configurations (tab and space indents, LF/CRLF/preserve, blank-line limits and final-newline choices). Invalid corpus inputs produce no writable output.

```sh
cargo test -p gdfmt_formatter -p gdfmt -p gdfmt_cli -p gdfmt_workspace
cargo xtask formatter-conformance --language 4.7 --godot /path/to/pinned/godot --report target/conformance/formatter-4.7.json
```

The smoke command uses the five exact engine pins and width 40. It runs check-only, recording formatted source and logs. It does not execute scripts or establish runtime equivalence. Verification on 2026-10-21: 522 accepted script/profile outputs passed all five pinned macOS engines (90/98/110/112/112 for 4.3–4.7), with zero failures. The Rust corpus covers 2176 valid script/profile/layout combinations, including 22 parser-accepted analyzer boundaries under all four layouts. Those boundaries are excluded from engine acceptance. Pinned project trials pass 1856 layouts for 464 scripts; one upstream-invalid source is explicitly rejected. The 401 gdtoolkit style differences are reviewed and fingerprinted. P4 source-build alpha acceptance is complete with exclusions; sustained fuzzing remains follow-up work and production CLI extensions belong to P6.
