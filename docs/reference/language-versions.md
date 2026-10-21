# Language versions and feature gates

`gdfmt_syntax::LanguageVersion`, also re-exported by parser/config, provides five explicit grammar profiles: 4.3, 4.4, 4.5, 4.6, and 4.7. The default is fixed at 4.7. Profiles represent verified version differences in implemented syntax, not full engine conformance. The 4.8 preview, patch-version strings, `latest`, and unknown versions are rejected without silent fallback or network lookup.

```rust
use gdfmt_parser::{LanguageVersion, ParseOptions, parse};

let options = ParseOptions {
    language_version: LanguageVersion::Godot4_3,
    ..Default::default()
};
let result = parse("var data: Dictionary[String, int] = {}\n", &options);
assert!(result.diagnostics.iter().any(|d| d.code == "PAR017"));
assert_eq!(result.tree.unwrap().text(), "var data: Dictionary[String, int] = {}\n");
```

Configuration accepts `"language": { "godotVersion": "4.5" }`. Rust hosts pass `config.language.godot_version` to `ParseOptions.language_version`. Discovery, field merging, provenance, and JSON Schema use the same enum. CLI `lex` remains an independent lexical check.

## Verified feature table

| Feature                                                           | Earliest profile | Evidence                                                                                                                                                                                                                                         |
| ----------------------------------------------------------------- | ---------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `Dictionary[K, V]` in declarations, return types, and as/is       | 4.4              | [4.4 release notes](https://godotengine.org/releases/4.4/index.html)                                                                                                                                                                             |
| `@export_tool_button`                                             | 4.4              | [4.4 beta 1 notes](https://godotengine.org/article/dev-snapshot-godot-4-4-beta-1/)                                                                                                                                                               |
| `@warning_ignore_start` / `@warning_ignore_restore`               | 4.4              | [4.4 beta 1 notes](https://godotengine.org/article/dev-snapshot-godot-4-4-beta-1/)                                                                                                                                                               |
| `@abstract` classes and methods                                   | 4.5              | [4.5 language reference](https://docs.godotengine.org/en/4.5/tutorials/scripting/gdscript/gdscript_basics.html#abstract-classes-and-methods)                                                                                                     |
| Rest parameters `...args`, including static functions and lambdas | 4.5              | [4.5 variadic functions](https://docs.godotengine.org/en/4.5/tutorials/scripting/gdscript/gdscript_basics.html#variadic-functions)                                                                                                               |
| `@export_file_path`                                               | 4.5              | Annotation registries in the [4.4 parser](https://github.com/godotengine/godot/blob/4.4-stable/modules/gdscript/gdscript_parser.cpp) and [4.5 parser](https://github.com/godotengine/godot/blob/4.5-stable/modules/gdscript/gdscript_parser.cpp) |
| Hint arguments for `@export_multiline`                            | 4.6              | Annotation registries in the [4.5 parser](https://github.com/godotengine/godot/blob/4.5-stable/modules/gdscript/gdscript_parser.cpp) and [4.6 parser](https://github.com/godotengine/godot/blob/4.6-stable/modules/gdscript/gdscript_parser.cpp) |

## Behavior and testing

`LanguageFeature::since()` defines the runtime feature table; `LanguageVersion::supports()` queries it. Earlier profiles report `PAR017` while completing the lossless CST and other constraint checks. Invalid constructs still report their own `PAR` codes. Independent `lex` accepts no version parameter; recognizing a token does not establish version support.

Tests traverse every profile, covering positive/negative cases around the introduction version, scripts/expressions/lambdas, lossless recovery in older profiles, UTF-8 spans, and the shared diagnostic budget. When a manifest fixture specifies `syntaxErrorsByVersion`, it must cover every profile. `cargo xtask compatibility` checks the default profile and the complete matrix; missing or unknown profiles fail verification.

Type names, annotation argument values, inheritance/abstract overrides, runtime behavior, and complete 4.6/4.7 grammar differences require an analyzer and broader engine coverage. Stable 4.3, 4.4, 4.5, 4.6, and 4.7.2 are integrated with the [pinned engine matrix](/reference/compatibility); all seven gates have independent positive and negative cases. Namespace/trait remain unsupported.
