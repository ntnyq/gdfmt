# P4 layout acceptance record

Verified on 2026-10-21, macOS. P4's formatter alpha acceptance is complete within the scope and exclusions below: script layout, invariant checks, five pinned engines, licensed project trials and reviewed gdtoolkit differences. This records source-build acceptance; no package has been published.

## Scope and evidence

| Area              | Layout contract                                                                                                                                     | Evidence                                                                                              |
| ----------------- | --------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------- |
| Comments          | Leading and inline token anchors, delimiter dangling comments, suite/property/match dangling ownership; text, order and own-line role retained      | `formatter-comments.gd`, token-boundary insertion tests and negative attachment/ownership comparisons |
| Lambdas           | One simple comment-free body inline; compound/multiple/commented bodies indented; named, nested, typed/default parameters, lists/dictionaries/calls | `formatter-lambdas.gd`, existing lambda/default/loop/return fixtures                                  |
| Declarations      | Headers, inheritance, typed/inferred variables/constants, enums, signals, functions, parameters, annotations, nested classes and properties         | `formatter-declarations.gd`, existing 4.3–4.7 feature gates                                           |
| Suites/statements | Class/function/accessor suites, if/elif/else, for/while, match branches/patterns/guards, assignment/return/assert/control statements                | `formatter-suites.gd`, existing parser-accepted corpus                                                |
| Literal safety    | Exact numeric/string/raw/StringName/NodePath spelling, multiline literal bytes and BOM; no parenthesis/comma/semicolon rewriting                    | `formatter-multiline-literals.gd`, standalone expressions, BOM/CRLF and unary-sign regressions        |
| Whole files       | Uniform configured indentation and layout newlines, bounded retained blank lines, removed leading/trailing blank lines, configured terminal newline | Formatter option matrix and CLI stdin/file-write regression                                           |

The five original layout scripts and two project-trial regressions are registered in both compatibility manifests. Ten reviewed snapshots live in `crates/gdfmt_formatter/tests/snapshots/` (widths 40 and 100). All 544 parser-accepted script/profile pairs run under four layouts: tab/40/preserve, tab/100/preserve, two spaces/40/CRLF/no blanks/no final newline, and four spaces/100/LF/one blank/final newline. This gives **2176 reparse/structure/comment/idempotence combinations**. The 22 parser-accepted analyzer boundaries are included here and excluded from engine acceptance. Invalid input still yields no writable output.

Comment comparison ignores accidental Rowan trivia placement. It compares significant node topology/token spellings, then each comment's spelling, adjacent significant token position, own-line/suffix role and dangling block owner. Negative tests reject moving a comment to another element or outside its body. Doc suffixes force a line boundary before subsequent code, and fitting accounts for comments after closing delimiters.

## Five fixed engines

The exact tags, commits and binary checks remain in [engine conformance](/development/engine-conformance). Every original script is checked against its classified expectation; every formatted accepted script is separately checked at width 40.

| Profile | Original script comparisons | Formatted accepted outputs | Failures |
| ------- | --------------------------: | -------------------------: | -------: |
| 4.3     |                         262 |                         90 |        0 |
| 4.4     |                         262 |                         98 |        0 |
| 4.5     |                         262 |                        110 |        0 |
| 4.6     |                         262 |                        112 |        0 |
| 4.7     |                         262 |                        112 |        0 |
| Total   |                    **1310** |                    **522** |    **0** |

Reports remain reproducible at `target/conformance/report-<profile>.json` and `target/conformance/formatter-<profile>.json`; `p4-summary.json` aggregates the local runs. They record source/output and engine diagnostics. These are check-only comparisons, not runtime execution or proof of semantic equivalence.

The first engine run exposed a layout restriction that Rust reparsing did not: Godot rejects a newline immediately after a collection type's `[`. Type references now remain indivisible, with a focused regression and a successful full engine rerun. A block lambda also cannot dedent to an arbitrary container continuation column; its layout uses the enclosing statement's indentation stack instead.

## Pinned, licensed project trials

`tests/project-trials/manifest.json` records the complete `.gd` inventories at two immutable commits, with SHA-256 digests for every script and the applicable license notices. Third-party sources remain in ignored `target/project-trials/sources/`, with notices intact. Scenes, assets, project imports and runtime execution are outside this source-only trial.

| Source                                                                                                                      | Pin and license                                                                                            | Scripts | Accepted | Known source rejection | Valid layouts |
| --------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------- | ------: | -------: | ---------------------: | ------------: |
| [Godot demos](https://github.com/godotengine/godot-demo-projects/tree/6ad6167e0577fe3622c18546138f456b107ce93c)             | `4.7-6ad6167`, commit `6ad6167e0577fe3622c18546138f456b107ce93c`; root MIT and Dodge the Creeps MIT notice |     456 |      455 |                      1 |          1820 |
| [Kenney 3D Platformer](https://github.com/KenneyNL/Starter-Kit-3D-Platformer/tree/3fa8a04b1c01ab23db43123d4ce814a34c3fc7f0) | commit `3fa8a04b1c01ab23db43123d4ce814a34c3fc7f0`; MIT                                                     |       9 |        9 |                      0 |            36 |
| Total                                                                                                                       | 986362 source bytes; 131 official demo directories plus the starter kit                                    | **465** |  **464** |                  **1** |      **1856** |

All 464 accepted scripts pass the validated facade under four layouts using language profile 4.7, including CRLF, space indentation, blank-line limits and no-final-newline. No unexpected rejection or invariant failure remains. The negative source runs under all four layouts and returns no writable output; it is excluded from the valid-layout count.

The known upstream rejection is `godot-demos/2d/tween/main.gd`. Its lambda dedents to a continuation column absent from the indentation stack. The original file produces `Unindent doesn't match the previous indentation level.` at line 77 under the exact Godot 4.7.2 pin. gdfmt reports `LEX005`; gdtoolkit 4.5.0 accepts and rewrites it. This is an explicit source exclusion, not a successful format or a waived formatter failure. The runner reproduces the engine diagnostic on every trial.

Trials exposed two parser errors, now fixed: `void` functions may return a call (including a parenthesized call), while constructors still reject return values; and a block lambda's final statement can end directly at an enclosing `)` or `,` without a physical dedent. `formatter-project-regressions.gd`, `project-invalid-lambda-dedent.gd` and focused parser tests cover these cases under all five profiles. Engine checks confirm the distinction between the accepted lambda forms and the invalid dedent.

## gdtoolkit difference review

The comparison uses **gdtoolkit 4.5.0**, width 100 and tabs. All ten Python dependencies are version- and hash-locked in `tests/project-trials/requirements.txt`; the tested interpreter is Python 3.12.14. At width 100, **63 outputs are byte-identical and 401 differ** (official demos: 62 identical/393 different; Kenney: 1/8). There are **zero unexplained differences**. Every source/output pair is fingerprinted in `tests/project-trials/reviewed.json`, including the excluded source's gdtoolkit output.

| Reviewed difference                             | Decision and evidence                                                                                                                                                                                                                                                                                                                           |
| ----------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Layout: all 401 differing accepted scripts      | Retain bounded source blank lines rather than imposing gdtoolkit's function spacing; print annotations on separate lines, use one space before inline comments, expand suites and choose list/enum groups by width. Binary expressions without an existing group can exceed the width because gdfmt does not invent parentheses or backslashes. |
| Parentheses, commas and semicolons: 110 scripts | Preserve original punctuation. gdtoolkit can add/remove grouping, change trailing commas, remove semicolons and wrap call/member chains. See `networking/webrtc_signaling/server/ws_webrtc_server.gd`.                                                                                                                                          |
| Quote normalization: 3 scripts                  | Preserve original literal bytes. Reviewed escaped-quote changes in `3d/global_illumination/test.gd`, `gui/regex/regex.gd`, and `misc/os_test/os_test.gd`.                                                                                                                                                                                       |
| Inner-class inheritance: 2 scripts              | Preserve `class Name extends Base:`. gdtoolkit moves `extends` into the class suite in `misc/custom_logging/custom_logger_ui.gd` and `networking/webrtc_signaling/server/ws_webrtc_server.gd`.                                                                                                                                                  |
| Source rejection: 1 script                      | Keep the invalid tween file unchanged, with no output, despite gdtoolkit accepting it.                                                                                                                                                                                                                                                          |

Category counts overlap. The classifier rejects unexplained token differences or changed comment spelling/order; output fingerprints additionally catch layout changes. Token/quote normalization is only a review aid. The independent Rust gate compares each gdfmt output against its original CST, significant token/literal spellings and comment anchors; it does not prove gdtoolkit's transformations equivalent.

Reproduction and review-update instructions are in `tests/project-trials/README.md`. A normal rerun checks the committed review without recording new expectations. `layouts.json`, `comparisons.json`, per-file unified diffs and `summary.json` remain under `target/project-trials/`; the summary includes the original-source engine log. Downloads are opt-in and hash checked, modified cached sources and extra scripts fail, and original project files are never formatted in place.

## Validation and follow-up work

Both workspace feature configurations passed 94 tests; the one ignored test is the mutation worker launched by its parent test. Formatting, Clippy, schema drift, compatibility, syntax coverage, Rust 1.88 and the documentation build passed. Three optional Python harness regressions verify cached-source tampering, unexplained code/comment changes and expected style classification.

```sh
cargo test --workspace --all-features --locked
cargo test --workspace --no-default-features --locked
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo xtask schema --check
cargo xtask compatibility
cargo xtask syntax-coverage
cargo xtask conformance --language <profile> --godot /path/to/pinned/godot --report target/conformance/report-<profile>.json
cargo xtask formatter-conformance --language <profile> --godot /path/to/pinned/godot --report target/conformance/formatter-<profile>.json
```

Rust 1.88 all-target/all-feature checking and the bilingual documentation build are also retained. The bounded P3 token-mutation tests still run as part of workspace tests; they do not substitute for sustained formatter fuzzing.

The P4 source-build alpha acceptance gate is satisfied with the explicit exclusions above; publication is a separate release action and has not occurred. Longer formatter fuzz campaigns and broader project/operating-system trials remain follow-up work. P5 formatter-off/on regions are implemented and verified below; P6 configuration/file acceptance is recorded below. Full Godot compatibility, project import success, semantic/runtime equivalence, literal-value decoding and cross-platform engine verification are not claimed.

## P5 formatter-only acceptance (2026-10-21)

The product now focuses on formatting. P5 no longer includes lint rules, rule suppression or semantic fixes; no linter crate/configuration or fix CLI is retained. P6 remains production layout configuration and file handling. The [revised roadmap](/development/implementation-plan) defines the remaining formatter milestones.

Paired standalone `# gdfmt: formatter-off` / `# gdfmt: formatter-on` regions are implemented. The facade independently compares protected bytes before allowing output, in addition to the existing reparse, structure/comment and idempotence gates. Syntax errors anywhere still prevent output; unmatched/nested markers yield `FMT002` and CLI exit 1. If an exact region conflicts with normalized indentation or expression layout, its enclosing top-level unit is preserved conservatively and validated again.

| Evidence                                            | Coverage                                                                                                                                                                                                                                                                             |
| --------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `crates/gdfmt/tests/regions.rs` — 9 regressions     | All five profiles, LF/CRLF/preserve, exact region bytes, Unicode/BOM, EOF/final-newline policy, multiple/empty/comment-only regions, literal/suffix false markers, nested suites, lambdas, accessors, match, expression/declaration boundaries, indentation fallback and idempotence |
| `crates/gdfmt_cli/tests/cli.rs` — 3 new regressions | File/check/diff/stdin agreement; invalid markers and protected syntax errors cannot write; syntax-only ignores layout directives; no lint/fix command and no semantic statement/name rewrites                                                                                        |
| `crates/gdfmt_config/tests/config.rs`               | Runtime/schema both reject lint, rules and plugin fields                                                                                                                                                                                                                             |

```sh
cargo test -p gdfmt --test regions --locked
cargo test -p gdfmt_cli --test cli --locked
cargo test -p gdfmt_config --test config --locked
```

The P4 evidence above is retained as its acceptance snapshot. P5 verification reruns the same gates; results are recorded below.

Verification completed on 2026-10-21: **106 tests passed in each workspace feature configuration**, with the mutation worker still ignored and launched by its parent. All **1310 parser comparisons and 522 formatter outputs** passed the five pinned macOS engines; an additional **20 protected-region outputs** passed engine check-only and exact-byte checks across the five profiles. The existing **2176 corpus layouts** and **1856 pinned-project layouts** remain valid; the single known upstream-invalid source remains rejected, and all 401 gdtoolkit differences retain their reviewed fingerprints.

Rustfmt, Clippy, schema drift, compatibility, syntax coverage, Rust 1.88 all-target/all-feature checking and the bilingual documentation build passed. The generated schema is excluded from general formatting so the generator remains authoritative. Local reports are `target/conformance/p5-summary.json` and `p5-regions.json`, alongside the existing engine and project-trial reports. P5 is accepted for the revised formatter-only scope; no lint/semantic-fix completion is claimed, and the P4 exclusions remain in force.

## P6 configuration and file/CLI acceptance (2026-10-21) {#p6-configuration-acceptance}

P6 implements local JSON inheritance, declaring-directory overrides, EditorConfig and consistent input ignores while retaining the formatter-only scope. The exact precedence and selection contract is in [configuration](/guide/configuration): EditorConfig before JSON, all base fields before appended overrides, ordinary arrays replaced, and defaults filled after merging. `explain-config` includes inherited JSON order, EditorConfig files, effective field/override/property provenance and ignore status, including for virtual stdin paths.

| Evidence                                    | Coverage                                                                                                                                                                                                                                                                                                                                                   |
| ------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `crates/gdfmt_config/tests/config.rs`       | Identical runtime/schema matrix for inheritance/override/ignore shapes, null/unknown/type/range failures, false/zero and empty-array merging; lint/rules/plugins remain rejected                                                                                                                                                                           |
| `crates/gdfmt_workspace/tests/workspace.rs` | Relative multi-parent inheritance, repeated shared parents, canonical-path cycles, missing inputs, depth/layer/total-byte budgets, ordered include/exclude globs, root/explicit configuration, filename-aware virtual paths, EditorConfig sections/brace/range/unset/fallback/source mapping, ignore precedence/negation/explicit inputs and bounded reads |
| `crates/gdfmt_workspace/src/writing.rs`     | Injected failure before temporary creation, write, permission preservation, sync and replacement; external edit/delete/same-byte replacement/chmod/symlink swap; interrupted subprocess and subsequent recovery                                                                                                                                            |
| `crates/gdfmt_cli/tests/cli.rs`             | File/stdin/explain agreement, explicit and stdin ignores, empty discovery, sorted/deduplicated source IDs, JSON/human errors, exit-code precedence, preflight configuration/UTF-8/size failures, mode conflicts and stream I/O errors                                                                                                                      |

The write guard additionally refuses Unix hardlinks and rechecks metadata/file identity along with bytes. Ordinary failure paths preserve the original and clean temporary files; interrupted preparation leaves the original intact and may leave an orphan temporary file. Detection is best-effort across the last-check/rename race. Atomicity is per file, with no multi-file transaction or power-loss durability claim. Local `.gitignore`, `.ignore` and `.gdfmtignore` work with Git worktree marker files; resolving external Git common-directory excludes is not implemented.

The existing Linux/macOS/Windows workspace CI matrix remains required. Local acceptance is on macOS only; Windows/Linux execution and stronger platform-specific locking/recovery are not claimed. Parser/formatter modules gain no filesystem or EditorConfig dependency, and no lint or semantic rewrite workflow is added. The P4 source/project exclusions and P5 exact protected-byte contract remain in force.

Local Rust/host verification passed **131 tests in each workspace feature configuration**, with **2 ignored workers launched by their parent tests** (mutation and interrupted-write workers). This adds 25 passing regressions over P5. Rustfmt, Clippy, generated-schema drift, compatibility (292 lexical / 16 expression / 262 script fixtures; 1310 profile checks), syntax coverage (50 grammar rows), Rust 1.88 all-target/all-feature checking and the bilingual documentation build passed. The existing project-trial runner and its three harness regressions passed again: **1856 valid layouts**, one expected upstream source rejection and **zero unexplained differences** across the 401 reviewed gdtoolkit differences.

All five pinned macOS engines passed **1310 source comparisons and 522 formatted outputs**, with an additional **20 protected-region outputs** passing engine check-only and exact-byte retention. Local reports are `target/conformance/p6-summary.json`, `p6-report-<profile>.json`, `p6-formatter-<profile>.json` and `p6-regions.json`; project evidence remains under `target/project-trials/`. P6 is accepted for the documented local source-build scope and limitations above; cross-platform execution remains an explicit CI gate rather than a completed local claim.
