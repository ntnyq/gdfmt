# Pinned engine conformance

`cargo xtask conformance` compares complete scripts with the engine and Rust grammar profile pinned in `tests/compatibility/engine.json`. The matrix keeps the 4.7.2 baseline and adds the first stable releases of 4.3–4.6 to test feature introduction boundaries.

| Profile | Pinned tag                                                                     | Exact `--version`                 |
| ------- | ------------------------------------------------------------------------------ | --------------------------------- |
| 4.3     | [4.3-stable](https://github.com/godotengine/godot/releases/tag/4.3-stable)     | `4.3.stable.official.77dcf97d8`   |
| 4.4     | [4.4-stable](https://github.com/godotengine/godot/releases/tag/4.4-stable)     | `4.4.stable.official.4c311cbee`   |
| 4.5     | [4.5-stable](https://github.com/godotengine/godot/releases/tag/4.5-stable)     | `4.5.stable.official.876b29033`   |
| 4.6     | [4.6-stable](https://github.com/godotengine/godot/releases/tag/4.6-stable)     | `4.6.stable.official.89cea1439`   |
| 4.7     | [4.7.2-stable](https://github.com/godotengine/godot/releases/tag/4.7.2-stable) | `4.7.2.stable.official.ed1daf0bf` |

Each pin records its full commit, tag, exact version string, and Linux archive digest from the official `SHA512-SUMS.txt`. The default profile also agrees with the main corpus baseline. Five independent CI jobs read the same pins, verify SHA-512, run the harness, and upload separate reports.

```sh
cargo xtask compatibility
cargo xtask conformance --godot /path/to/pinned/godot
# Exact engine version selects the profile; optionally require a specific one.
cargo xtask conformance --language 4.3 --godot /path/to/godot-4.3 --report target/conformance/report-4.3.json
GODOT_BIN=/path/to/godot-4.6 cargo xtask conformance --language 4.6
```

Each invocation checks one engine. A conflicting `--language`, unpinned patch, unknown version, or custom build fails. Supply the editor binary; the harness does not download or select latest. Ordinary Rust tests need no Godot installation.

## Execution and verdicts

Each case gets a fresh temporary project containing the original source as `case.gd`. The engine runs `--headless --path <project> --check-only --script res://case.gd`; fixture code is never executed at runtime. Cases share no class registry, imported resources, or project files. Multi-file projects and resource dependencies remain outside this corpus.

Check-only enters semantic analysis, and some versions return zero for script errors. The harness checks script errors, the target `res://case.gd`, failed-load text, healthy exit status, and each negative case's pinned diagnostic. Godot 4.3/4.4 may emit ANSI colors even with `NO_COLOR`: comparisons remove CSI sequences while reports retain raw stdout/stderr. Valid and invalid controls run first. Wrong versions, no-op binaries, crashes, and startup errors fail. Processes have a 15-second timeout and a 1 MiB stdout/stderr limit; file-backed output avoids pipe deadlocks.

The schema 2 manifest covers **262 script fixtures × 5 versions = 1310 engine comparisons**. Every case explicitly records all profiles in `expectations`, with no fallback or skip rules. Missing or duplicate classifications, missing pins/profiles, negative cases without diagnostic excerpts, and analyzer cases without scope notes fail validation. Expression and lexer fixtures remain under `compatibility`; standalone expressions are not passed directly to the engine.

| Kind       | Rust parser | Godot  | Meaning                                                                             |
| ---------- | ----------- | ------ | ----------------------------------------------------------------------------------- |
| `accepted` | Accept      | Accept | Self-contained valid script                                                         |
| `syntax`   | Reject      | Reject | Grammar, version gate, or implemented structural constraint; requires `engineError` |
| `analyzer` | Accept      | Reject | Semantic boundary outside parser scope; requires `engineError` and `note`           |

Classifications are checked against pinned parser/analyzer sources, not inferred from the engine's “Parse Error” label. Versions 4.3–4.5 each have four analyzer boundaries; 4.6/4.7 each have five. The added multiline hint type case is rejected syntactically before 4.6 because hint arguments are unavailable, then semantically because its integer argument is not a string. These boundaries are checked separately, never skipped or counted as syntax agreement.

The default report is `target/conformance/report.json`. Use distinct `--report` paths for matrix runs. Schema 2 reports include the full engine commit/tag, version, profile, classification, expected error, parser diagnostics, exit code, raw logs, and verdict. Case mismatches still write a report and fail; infrastructure failures such as version/control errors fail immediately.

## Gates and parser corrections

The `gates` table assigns a positive and negative script to every `LanguageFeature`. Validation requires positives to be rejected before introduction and accepted afterward, while negatives remain rejected in every profile. Missing feature coverage or classifications inconsistent with `supports()` fail.

| Introduced | Independent positive and negative cases                                                                                                                              |
| ---------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 4.4        | Typed Dictionary declarations/parameters/returns/as/is and wrong arity; tool buttons and missing titles; warning regions and missing arguments                       |
| 4.5        | Abstract classes/methods and forbidden bodies; ordinary/static/lambda rest parameters and typed rest/following parameters; file paths and invalid annotation targets |
| 4.6        | Multiline hints and invalid annotation targets; legacy zero-argument multiline remains valid throughout                                                              |

Additional cases cover restore arguments, duplicate starts, unmatched restores, cross-function regions, escaped strings, and optional file-path arguments. The parser previously accepted duplicate starts and restores without a start. It now tracks transitions in source order, sharing state across function/class boundaries and canonicalizing case and Unicode escapes, including raw/triple string forms. An open start may extend to EOF. The source rule is `warning_ignore_region_annotations` in the pinned [4.4 parser](https://github.com/godotengine/godot/blob/4c311cbee68c0b66ff8ebb8b0defdd9979dd2a41/modules/gdscript/gdscript_parser.cpp). Warning-name catalog validation and general annotation value evaluation remain outside this structural check.

Previous `Array[int]([])` / `Dictionary[String, int]({})` fixes also run on all five engines: value-position brackets are subscripts, which cannot be called directly. Type declarations and `as`/`is` still parse collection type arguments, following `parse_subscript` / `parse_call` in the pinned [4.7.2 parser](https://github.com/godotengine/godot/blob/ed1daf0bf001b61586d9930840f2f1394092c079/modules/gdscript/gdscript_parser.cpp).

## Declaration, statement, and recovery audit

The audit adds 156 original scripts to the existing 79, checking every new case against each fixed binary. It follows the declaration/statement entry points of the pinned [4.6 parser](https://github.com/godotengine/godot/blob/89cea143987d564363e15d207438530651d943ac/modules/gdscript/gdscript_parser.cpp), with focused Rust tests for CST structure and scope. These are bounded grammar probes, not an exhaustive proof of language conformance.

| Area                                | Positive and negative evidence (`fixtures/audit-*.gd`)                                                                     |
| ----------------------------------- | -------------------------------------------------------------------------------------------------------------------------- |
| Class headers and inheritance       | Dotted/path bases, inner headers, duplicate headers, comment/header order, forbidden collection bases                      |
| Classes and members                 | Inline class member boundary, anonymous/named enum collisions, contextual/reserved names, static members                   |
| Variables and constants             | Inferred/explicit types, initializers, class/local contexts, missing names/values, static constants                        |
| Enums and signals                   | Values, trailing commas, duplicate enum entries, typed signal parameters, forbidden defaults/return types                  |
| Functions and parameters            | Inferred defaults, duplicate parameters, bodyless declarations, constructors, static initialization                        |
| Properties                          | Body/delegated accessors, `get()`, static properties, setter returns, mixed forms, typed setter rejection                  |
| Annotations and comments            | Inline binding, targets, pending annotations across pass/comment strings, standalone lines, comment newlines               |
| Suites and conditionals             | Inline/nested compounds, semicolon termination, leading/repeated semicolons, orphan/duplicate clauses, branch scopes       |
| Loops and control statements        | Typed iterators, scope reuse/conflicts, break/continue boundaries, lambdas resetting loop context                          |
| Return/assert/expression statements | Return lambdas, assert trailing commas/arity, breakpoint, standalone expressions/lambdas                                   |
| Assignments                         | All compound operators, grouped targets, calls as targets, forbidden chained assignment                                    |
| Match                               | Nested arrays/dictionaries, key-only/rest patterns, guards, binding conflicts, alternatives, loop context                  |
| Lambda scopes                       | Prior versus later locals, initializer timing, parameter shadowing, nested capture, loop/guard/default/setter contexts     |
| Recovery                            | Missing names, operands, parameters, colon/indent, clauses, patterns, accessors, enum entries, annotations, lexical errors |

Corrections include rejecting collection types in `extends`, leading semicolons, standalone grouped lambdas, and semicolons after class comment strings; accepting assert trailing commas and setter return values; preserving repeated separators after complete statements; restoring inline class boundaries, inner header ordering, pending annotation binding, and lambda scope visibility; and keeping malformed accessors inside their property block.

The five pins also accept a key-only dictionary pattern after `..`, such as `{.., "key"}`, while rejecting a subsequent value pattern or repeated rest. This source-level edge follows `parse_match_pattern`; it is recorded explicitly rather than replacing the engine behavior with a stricter rule.

Nineteen syntax-error fixtures declare `recoveredDeclarations` in the compatibility manifest. Both commands require each name to be a direct declaration in the source-file CST for every profile. A name buried in an error, initializer, or inner class does not satisfy the assertion. Reports include expected and missing recovery names. Unterminated strings and unclosed delimiters retain lossless/rejection checks, but cannot promise later declarations: multiline strings and delimiter continuation can consume their apparent source boundaries.

All five pinned engines were exercised on macOS. Linux verification runs in the CI matrix. This does not establish complete grammar coverage, general AST/literal-value agreement, resource/inheritance analysis, runtime equivalence, or formatter conformance.

The [P3 syntax audit](/development/syntax-audit) adds 20 original regressions and accepts the 50-row syntax contract with explicit exclusions. P4 adds five layout regressions and two project-trial regressions, bringing the corpus to 262 scripts. The separate `cargo xtask formatter-conformance` command checks formatted accepted scripts at width 40 using the same exact engine pin; reports include formatted source and logs. All 522 formatted accepted outputs pass; see the [P4 acceptance record](/development/formatter-acceptance).
