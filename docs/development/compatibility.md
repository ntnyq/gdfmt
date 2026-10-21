# Compatibility baseline and gaps

Baseline verification date: 2026-10-21. Lexing and expression, declaration, and statement parsing are implemented. Full syntax support is not declared for any Godot version.

## Pinned references

| Reference                                                                                                                                         | Pinned version or commit                                 | Purpose                                         |
| ------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------- | ----------------------------------------------- |
| [Godot release archive](https://godotengine.org/download/archive/)                                                                                | 4.7.2-stable; the recorded archive also listed 4.8-beta1 | Separate stable and preview baselines           |
| [Godot tokenizer](https://github.com/godotengine/godot/blob/ed1daf0bf001b61586d9930840f2f1394092c079/modules/gdscript/gdscript_tokenizer.cpp)     | ed1daf0bf001b61586d9930840f2f1394092c079                 | Indentation, tokens, and strings                |
| [gdtoolkit grammar](https://github.com/Scony/godot-gdscript-toolkit/blob/f61a1f3a9f2f757813bd5176a28903502bd44959/gdtoolkit/parser/gdscript.lark) | f61a1f3a9f2f757813bd5176a28903502bd44959                 | Syntax scope and fixture classification         |
| [Oxc workspace](https://github.com/oxc-project/oxc/blob/e97a8b121811c6fdb915ece70c2de3890bc7897f/Cargo.toml)                                      | e97a8b121811c6fdb915ece70c2de3890bc7897f                 | Crate responsibilities and dependency direction |

The stable Godot tag was resolved through the Git remote to the commit above. The preview 4.8-beta1 tag was not resolved during that verification; the manifest retains a null commit and treats it as unsupported.

`tests/compatibility/manifest.json` records pinned references and the features, stages, and expected diagnostics of 292 original fixtures. Sixteen expression and 262 script fixtures check both lexical and syntax diagnostics; the rest are lexical fixtures. All 262 scripts match pinned-engine expectations, including 4–5 separately classified analyzer boundaries per version; see [engine conformance](/development/engine-conformance). Unicode XID data derives from the pinned Godot `core/string/char_range.cpp`, retaining MIT attribution.

## Feature matrix

Evidence below distinguishes lexical, standalone-expression, and script stages. [P3 syntax is accepted with exclusions](/development/syntax-audit); full language conformance and formatter alpha acceptance remain pending.

| Feature group                                              | Current evidence                                                                                                      | Remaining work                                               |
| ---------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------ |
| BOM, CRLF, empty files, comments, EOF                      | Lossless regressions                                                                                                  | Parser/formatter integration checks                          |
| Tab/space indentation, comment lines, dedent               | Positive/negative cases; separate logical and physical newlines                                                       | Pinned-engine differential tests                             |
| Bracket and backslash continuation                         | On-demand lexing, lambda suite bracket baselines, outer indentation restoration                                       | Engine differential validation                               |
| Unicode identifiers                                        | Pinned Godot Unicode 17 XID tables, underscore extension, Chinese and combining characters                            | Engine comparison and security warnings                      |
| Numbers, operators, string prefixes                        | Signed tokens, longest match, numeric boundaries, recovery, special diagnostic tokens                                 | Decoded values and overflow behavior                         |
| Multiline/raw/StringName/NodePath                          | Single-quote multiline strings, UTF-16 pairs, six-digit escapes, scalar-range and bidi diagnostics                    | Decoded-value checks and formatter invariants                |
| Variables, constants, classes, inheritance, enums, signals | Declaration CST, ordering, uniqueness, typed facade, positive/negative cases, recovery                                | Type/inheritance semantics and engine coverage               |
| Functions, static, types, parameters, lambdas              | Defaults, rest parameters, return types, function context, suite recovery, version gates                              | Complete function semantics and engine coverage              |
| Property getters/setters                                   | Optional types, get(), exclusive body/delegation forms, delimiters, duplicate-accessor negatives                      | Type/delegation semantics and engine coverage                |
| await, super, calls, node paths                            | Hierarchical expression CST, positive/negative cases, lossless recovery                                               | Statement context and engine coverage                        |
| if/elif/else, loops, return, assert                        | Statement CST, single/multiline suites, function/loop context                                                         | Broader recovery and engine coverage                         |
| match, patterns, guards                                    | Array/dictionary/binding/wildcard/rest patterns, guard and branch CST                                                 | Binding-conflict checks; guard semantics and engine coverage |
| Typed arrays/dictionaries                                  | Declaration and as/is types; invalid generic-style constructor regressions; arity/nesting negatives and version gates | Type-name resolution and engine coverage                     |
| Annotations, abstract                                      | Annotation targets, arity, placement; bodyless abstract classes/methods; version gates                                | Full argument-value rules, inheritance, abstract overrides   |
| New constructs such as namespace, trait, variadics         | Rest parameters parse; namespace/trait retain only token spelling                                                     | P0 audit of introductions and grammar in 4.6/4.7/4.8         |
| Language structural constraints                            | P3 structural constraints accepted with documented exclusions                                                         | Project semantics excluded; no lint/fix workflow             |
| Formatting, comment attachment, structural equivalence     | Doc printer, comments, lambda/declaration/suite layout; reparse, structure/attachment and idempotence gates           | P4 alpha accepted; protected regions implemented (P5)        |

The parser uses pinned Godot 17.0 XID data rather than a moving `unicode-ident` table; macro dependencies may still include `unicode-ident` indirectly. Token recognition does not establish acceptance by a target version. `LanguageVersion` and feature gates are integrated into parser/config; see [language versions](/reference/language-versions). See [expressions](/reference/expressions) for precedence and syntax scope.

## Verification

```sh
cargo test --workspace --all-features
cargo xtask compatibility
cargo xtask conformance --godot /path/to/pinned/godot
```

Tests cover lossless token partitioning, progress on random Unicode/expressions, all binary precedence pairs, size/depth limits, suppression, list recovery, and cross-thread green-tree reconstruction. `xtask compatibility` reports lexical, expression, and script fixture counts separately. Scripts use parser-driven lexical context; `parserLexicalErrors` may override independent lexer expectations. The complete profile matrix performs 1310 version checks. Separate `conformance` compares 262 scripts with five pinned engines (1310 comparisons), emits JSON reports, and classifies 4–5 semantic boundaries per version separately.

The [syntax audit](/development/syntax-audit) inventories all pinned parser/tokenizer entry points and 675 official scripts, recording 190 direct parser probes, five-engine results and explicit follow-ups. Remaining work includes type/inheritance semantics, broader project corpora, numeric/string decoding, Unicode confusable-keyword policy, sustained fuzzing, and name/publishing permissions. The [P4 layout record](/development/formatter-acceptance) documents comment attachment and whole-script formatting with five-engine validation.
