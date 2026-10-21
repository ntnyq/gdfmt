# Architecture and contracts

P1 foundations, P2 lexing, and P3 expression, declaration, and statement parsing are in place. `parse` is the script entry point; abstract declarations, structural constraints, and version gates are integrated. P3 is [accepted with exclusions](/development/syntax-audit); the [P4 script formatter](/reference/formatting) and verified CLI are available. P4 source-build alpha is [accepted with documented project-trial exclusions](/development/formatter-acceptance); engine semantics remain outside this scope.

## Existing modules

| Crate             | Responsibility                                                                                                   | Normal dependencies                           |
| ----------------- | ---------------------------------------------------------------------------------------------------------------- | --------------------------------------------- |
| `gdfmt_text`      | SourceId, UTF-8 spans, line index, UTF-16 positions, structured diagnostics, TextEdit validation and application | serde, thiserror                              |
| `gdfmt_syntax`    | SyntaxKind, Token, LanguageVersion/Feature, Rowan lossless storage                                               | text, rowan, serde, schemars                  |
| `gdfmt_parser`    | Lexer, pinned Unicode XID tables, Pratt expressions, declarations and statements, on-demand lexing and recovery  | syntax, text, rowan                           |
| `gdfmt_formatter` | Private Doc printer and expression/call/container layout; no parser or filesystem normal dependency              | syntax, text, thiserror                       |
| `gdfmt`           | Pure facade with reparse, structure and idempotence gates                                                        | parser, formatter, syntax, text               |
| `gdfmt_config`    | Partial/resolved configuration, language versions, strict deserialization, schema                                | syntax, serde, schemars                       |
| `gdfmt_workspace` | Configuration/file discovery, UTF-8 reading, format-option mapping and guarded replacement                       | gdfmt, parser, config, text, ignore, tempfile |
| `gdfmt_cli`       | clap arguments, JSON/text reporting, shared runner, binary entry point                                           | workspace, config, parser, syntax, text       |
| `gdfmt_napi`      | Sync/async Node text/configuration boundary and shared CLI adapter                                               | gdfmt, config, workspace, CLI, napi           |
| `xtask`           | Schema generation and drift checks; lexical, expression, and script fixture verification                         | config, parser                                |

Lint rules, semantic fixes and rule-plugin hosts are outside the formatter scope. The napi crate and npm package provide host adapters; see [Node/npm contracts](/reference/node). File scanning and the CLI stay outside the parser's dependency tree.

## Source and diagnostics

- Spans are half-open UTF-8 byte ranges `[start, end)` using `u32`. Construction and deserialization reject reversed ranges; slicing checks UTF-8 boundaries.
- `LineIndex` owns its source so positions cannot accidentally be queried against another document. Separate methods expose byte and UTF-16 columns. Lines and columns are zero-based; CRLF is one physical newline.
- `TextEdit` validates all ranges and conflicts before producing a new string. Inserts at the same offset and overlapping ranges are rejected; adjacent replacements are allowed.
- Diagnostics contain data only. They never print or terminate the process. Lexical diagnostics include a stable code, category, severity, SourceId, and span.

## Lossless storage

Rowan 0.16 green trees are wrapped by `LosslessTree`, with red cursors created on demand. Tests transfer green storage to a worker thread and reconstruct the original source.

`lex` returns a flat token stream under `SourceFile`; `parse_expression` returns hierarchical expressions under `ExpressionRoot`. Rowan checkpoints wrap already-parsed left operands, preserving trivia and zero-width tokens in order. `parse` returns declarations, suites, statements, and expressions under `SourceFile`. `gdfmt_syntax::ast` provides shared CST views: `Expression`, `SourceFile`, `Declaration`, `Suite`, and `Statement`.

Physical whitespace, newlines, comments, and BOM are preserved. Logical Newline, Indent, Dedent, and Eof tokens have zero width. Valid and invalid input must satisfy `tree.text() == source`; oversized input produces no tree.

Default input limits are 16 MiB, 256 indentation or bracket levels, and 100 detailed diagnostics. Once the diagnostic limit is exceeded, one suppression notice is added and scanning continues to retain all text. Lexing is non-recursive. The parser defaults to 128 recursion/expression-height levels; callers can lower this, with a hard cap of 128. Lexer and parser share a diagnostic budget and preserve input when it is exhausted. Statements, lambdas, and patterns share the depth limit.

Script and expression parsers request tokens on demand rather than building a flat lexical tree first. Before a lambda body, the parser sets the suite's bracket baseline; new brackets inside the body still control implicit continuation. Exiting the lambda restores the outer mode and, for lambdas in expressions, the indentation stack. Independent `lex` does not recognize suite syntax, so its indentation tokens and diagnostics may differ from parser-driven results.

## Host and CLI

`gdfmt_cli` has library and binary targets. `run` accepts argv, cwd, and input/output streams, returning an exit code. Only binary `main` calls `process::exit`. The napi CLI reuses this runner.

`lex` accepts a file, directory, multiple paths, or stdin alone. Files are sorted and deduplicated; traversal does not follow symlinks. Explicit files must have a `.gd` extension. Directories respect project gitignore rules and exclude `.git`, `.godot`, `node_modules`, and `target`; user-global gitignore files are not read. No matching files is an error.

All input configurations are validated before source reading. Invalid UTF-8 is rejected without lossy decoding. Formatting uses guarded per-file replacement; checks/diffs remain read-only. `init` refuses to overwrite an existing file. See [formatting](/reference/formatting) for the preview scope.

JSON reports contain `schemaVersion: 1`, `spanEncoding: "utf8Bytes"`, command, status, files, and diagnostics. Diagnostic SourceIds match each file report's sourceId. `schema` emits JSON Schema directly; `explain-config` always emits JSON with provenance.

## Engineering decisions

- Edition 2024, MSRV 1.88, development toolchain 1.98.0. CI separately builds with MSRV and tests on Linux, macOS, and Windows.
- Commit the root Cargo.lock; resolver 3 selects dependencies according to MSRV.
- Project code is MIT licensed. Current fixtures are original. Unicode XID tables derive from a pinned Godot `char_range.cpp` commit, with attribution in the root `LICENSE-THIRD-PARTY`.
- All crates currently use `publish = false`. crates.io/npm name availability and account publishing permissions remain unverified.
- P7 fixes Node 22.13+/24.x and five proposed native targets; artifacts are prepared locally and full matrix execution remains a CI gate. Nothing is published; dynamic rule plugins are outside scope.

## Structural constraints and versions

After parsing, `validation` traverses the same CST for annotation binding, abstract-body rules, declaration ordering, duplicate names, function context, and feature versions. It does not build a second AST. Local scopes track added names and remove them on exit; branches do not contaminate each other, and lambdas see prior outer locals while allowing parameter shadowing. Validation shares the lexical/syntax diagnostic budget and SourceId.

`LanguageVersion` and `LanguageFeature` live in the syntax crate and are shared by parser and config. The enum and schema reject unknown versions; the default profile is fixed at 4.7. See [language versions](/reference/language-versions) for features, evidence, and boundaries.
