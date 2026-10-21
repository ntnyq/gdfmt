# P3 syntax audit and acceptance record

Audit date: 2026-10-21. The acceptance decision is **accepted with exclusions**. This record defines the bounded contract being accepted; it does not claim complete Godot semantic compatibility.

The authoritative inventory is [`syntax-coverage.json`](https://github.com/ntnyq/gdfmt/blob/main/tests/compatibility/syntax-coverage.json). It contains 50 grammar rows (48 covered and two diagnostic-only exclusions), 556 per-version source-symbol dispositions, all 173 annotation registrations across five pins, and classifications for all 675 official baseline test scripts. Every covered row links upstream symbols, pinned official test blobs, original positive/negative fixtures, the oldest supported profile or feature introduction, semantic boundaries and exclusions. `4.3-or-earlier` means present by the oldest supported profile; it makes no historical introduction claim.

## Reference and coverage contract

The exact parser/tokenizer/header contents are identified by commit and SHA-256 in `sourceFiles`. The audit enumerates parser `parse*` entry points, recovery, annotation registration/validation/callbacks, tokenizer text methods and the identifier/node-name/binary-operator context predicates. Each definition is mapped to grammar rows or excluded with a reason; overloaded definitions retain their source lines.

| Profile | Pinned primary source                                                                                                                                                                                                                                                      |
| ------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 4.3     | [Parser](https://github.com/godotengine/godot/blob/77dcf97d82cbfe4e4615475fa52ca03da645dbd8/modules/gdscript/gdscript_parser.cpp), [tokenizer](https://github.com/godotengine/godot/blob/77dcf97d82cbfe4e4615475fa52ca03da645dbd8/modules/gdscript/gdscript_tokenizer.cpp) |
| 4.4     | [Parser](https://github.com/godotengine/godot/blob/4c311cbee68c0b66ff8ebb8b0defdd9979dd2a41/modules/gdscript/gdscript_parser.cpp), [tokenizer](https://github.com/godotengine/godot/blob/4c311cbee68c0b66ff8ebb8b0defdd9979dd2a41/modules/gdscript/gdscript_tokenizer.cpp) |
| 4.5     | [Parser](https://github.com/godotengine/godot/blob/876b290332ec6f2e6d173d08162a02aa7e6ca46d/modules/gdscript/gdscript_parser.cpp), [tokenizer](https://github.com/godotengine/godot/blob/876b290332ec6f2e6d173d08162a02aa7e6ca46d/modules/gdscript/gdscript_tokenizer.cpp) |
| 4.6     | [Parser](https://github.com/godotengine/godot/blob/89cea143987d564363e15d207438530651d943ac/modules/gdscript/gdscript_parser.cpp), [tokenizer](https://github.com/godotengine/godot/blob/89cea143987d564363e15d207438530651d943ac/modules/gdscript/gdscript_tokenizer.cpp) |
| 4.7     | [Parser](https://github.com/godotengine/godot/blob/ed1daf0bf001b61586d9930840f2f1394092c079/modules/gdscript/gdscript_parser.cpp), [tokenizer](https://github.com/godotengine/godot/blob/ed1daf0bf001b61586d9930840f2f1394092c079/modules/gdscript/gdscript_tokenizer.cpp) |

| Family                  | Rows | Accepted scope                                                                                                                 |
| ----------------------- | ---: | ------------------------------------------------------------------------------------------------------------------------------ |
| Lexical                 |    5 | Layout/continuation, Unicode identifiers, numeric/string spelling, punctuation and delimiter diagnostics                       |
| Declarations            |    8 | Script/class headers, members, variables/constants, properties, parameters/functions, signals and enums                        |
| Statements              |    9 | Suites/separators, conditionals/loops/control context, return/assert, expressions and assignments                              |
| Patterns                |    2 | Match branches/guards, bind/wildcard/rest/array/dictionary patterns and structural binding scopes                              |
| Expressions             |   15 | Pratt precedence, atoms/operators, await, containers, grouping, attributes, subscripts, calls, node paths, preload and lambdas |
| Types                   |    1 | Type spelling, qualified names, collection parameters and explicit `as`/`is` sites                                             |
| Annotation structure    |    1 | Registry names, targets, argument expression lists/arity, binding and literal-only icon/region restrictions                    |
| Version gates           |    7 | Typed dictionaries/tool buttons/warning regions (4.4), abstract/rest parameters/file paths (4.5), multiline export hints (4.6) |
| Diagnostic-only grammar |    2 | Removed `yield` and invalid `?` syntax remain rejected; there is no positive supported form                                    |

`namespace` and `trait` are reserved token spellings in the pinned tokenizer, not supported declarations. Recognizing their spelling does not introduce a language feature. Text comments remain lossless; documentation extraction/BBCode, editor cursor state and serialized bytecode parsing are excluded explicitly.

## Official-test audit and corrections

The baseline [official test tree](https://github.com/godotengine/godot/tree/ed1daf0bf001b61586d9930840f2f1394092c079/modules/gdscript/tests/scripts) contains 675 `.gd` scripts. All are inventoried without implying all are executed. The audit fetched all 190 parser scripts (87 feature, 76 error and 27 warning scripts), verified each content against its Git blob SHA-1, and parsed the unchanged source. All 950 parser/profile combinations preserved exact source. Acceptance/rejection comparisons use the 4.7 profile: 186 agree with the syntax expectation, while four documented boundaries remain. Warning output and official `.out` runtime assertions were not compared. Older-profile source preservation does not assert that a later official test is valid in an older language.

The probes found and corrected these gaps, with 20 original `audit-syntax-*.gd` regressions registered in the five-engine corpus:

- Builtin constant spellings `PI`, `TAU`, `INF`, `NAN` are legal declaration identifiers, following [`Token::is_identifier`](https://github.com/godotengine/godot/blob/ed1daf0bf001b61586d9930840f2f1394092c079/modules/gdscript/gdscript_tokenizer.cpp#L195).
- Attribute and node-name contexts accept the reserved keyword allowlist, including `_`, following [`Token::is_node_name`](https://github.com/godotengine/godot/blob/ed1daf0bf001b61586d9930840f2f1394092c079/modules/gdscript/gdscript_tokenizer.cpp#L216). Literal tokens such as `true` remain invalid attribute names.
- A match body may contain standalone newline-terminated `pass` and `@warning_ignore` bound to a later branch; orphan/wrong-target annotations and semicolons after this special `pass` are rejected, following [`parse_match`](https://github.com/godotengine/godot/blob/ed1daf0bf001b61586d9930840f2f1394092c079/modules/gdscript/gdscript_parser.cpp#L2426).
- `@icon` requires a string literal with a nonempty decoded value, following [`validate_annotation_arguments` and `icon_annotation`](https://github.com/godotengine/godot/blob/ed1daf0bf001b61586d9930840f2f1394092c079/modules/gdscript/gdscript_parser.cpp#L4406).

The added negative scripts independently cover malformed array elements, missing member/preload/await/ternary operands, invalid node path literals, removed `yield` and invalid `?`. These replace unrelated negatives in the corresponding coverage rows.

## Explicit exclusions and follow-ups

These exclusions are part of the acceptance contract, not silently counted as passing syntax cases.

| Follow-up                | Excluded scope and evidence                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                           |
| ------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `SYN-CONFUSABLE-KEYWORD` | The official `parser/errors/identifier_similar_to_keyword.gd` is accepted by Rust. Godot uses TextServer Unicode-security checks to reject a confusable keyword in tokenizer [`potential_identifier`](https://github.com/godotengine/godot/blob/ed1daf0bf001b61586d9930840f2f1394092c079/modules/gdscript/gdscript_tokenizer.cpp#L594). This is an explicit lexical-security divergence, not an analyzer classification.                                                                                                                                                                                                              |
| `SEM-ANNOTATION-VALUES`  | Export enum property/array types and tool-button tool-mode requirements account for three official parser-folder exceptions. Folder naming does not determine phase: [`validate_annotation_arguments`](https://github.com/godotengine/godot/blob/ed1daf0bf001b61586d9930840f2f1394092c079/modules/gdscript/gdscript_parser.cpp#L4406) explicitly delegates general resolved arguments to `GDScriptAnalyzer::resolve_annotation`; callbacks consume those resolved values/types. Name, target, arity and literal-only icon/region syntax remain covered. Warning-name catalogs, RPC/export values and resource checks remain excluded. |
| `SEM-ANALYZER`           | Name/type/inheritance resolution, constant evaluation, abstract overrides, actual return/assignment types and decoded numeric/string value equivalence. The 222 analyzer scripts are inventoried without claiming analyzer conformance.                                                                                                                                                                                                                                                                                                                                                                                               |
| `SEM-RUNTIME`            | The 118 runtime scripts require a VM and execution/output comparisons. No runtime semantics are accepted here.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                        |
| `EDITOR-INTEGRATION`     | The 142 completion/LSP scripts require editor sentinels, project state and protocol behavior.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                         |
| `PROJECT-CORPUS`         | Three shared/support scripts plus multi-file/resource interactions require a project corpus; the current engine harness isolates scripts.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                             |

The 50 grammar rows are a reviewed inventory of the stated families and pinned entry points. They are not a proof that every possible input or every upstream semantic branch has been exercised. No complete-language, AST/literal-value or runtime-equivalence claim follows from the finite matrix.

## Reproducible acceptance checks

```sh
cargo xtask syntax-coverage
cargo xtask compatibility
cargo test -p gdfmt_parser --test audit --test mutations
cargo test --workspace --all-features --locked
cargo test --workspace --no-default-features --locked
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo xtask schema --check
cargo xtask conformance --language 4.3 --godot /path/to/pinned/godot-4.3
# Repeat conformance for the other four fixed binaries, using distinct report paths.
```

`syntax-coverage` checks pin parity, row evidence, fixture parse verdicts, complete source-symbol dispositions, official-test classifications and annotation registry references. It does not fetch or execute the upstream tests during ordinary CI; committed blob hashes and probe records identify the audited source snapshot. `compatibility` checks all 285 original fixtures, with 255 scripts explicitly classified in every engine profile (1275 engine comparisons).

Token mutation tests apply deletion, duplication, delimiter, keyword, newline/indent and assignment substitutions at every physical token of every fixture in all five profiles. Each profile runs in an isolated child process with a 60-second watchdog; a crash, timeout, missing tree or source mismatch fails. The acceptance run checked 45510 mutations per profile, 227550 total, without a crash, timeout or source mismatch. The finite deterministic mutation set supplements focused recovery regressions; sustained fuzzing remains future work.

The acceptance run passed all 1275 engine comparisons with zero failures, all 1285 compatibility version checks, and 227550 token mutations (45510 per profile). The source/official-test inventory checks passed. These results close the stated P3 syntax contract with its explicit exclusions; the remaining workspace checks above also guard subsequent implementation changes. P4 starts separately with Doc printing and expression/call/container formatting, verified by reparsing, structural equivalence and idempotence before CLI integration. Formatter acceptance is not implied by this parser audit.
