# Expression parsing

`gdfmt_parser::parse_expression` parses one standalone expression and returns a hierarchical Rowan lossless CST with structured diagnostics. Lambdas and their suites are supported. For declarations outside expressions, assignment statements, and complete `.gd` files, use the [script entry point `parse`](/reference/parsing). The [CLI](/reference/cli) formats and checks complete scripts.

```rust
use gdfmt_parser::{ParseOptions, parse_expression};
use gdfmt_syntax::ast::Expression;

let result = parse_expression("a + b * c", &ParseOptions::default());
assert!(!result.has_errors());
let tree = result.tree.unwrap();
assert_eq!(tree.text(), "a + b * c");
let expression = Expression::cast(tree.root().first_child().unwrap()).unwrap();
assert_eq!(expression.operands().count(), 2);
```

`ExpressionResult::tree` is `None` only when source exceeds the lexical byte limit. Other invalid inputs retain a tree, complete text, and error nodes. The root is `ExpressionRoot`; logical Newline, Indent, Dedent, and Eof are zero-width tokens. Comments, whitespace, BOM, physical newlines, and continuations retain their order and byte ranges.

`Expression` is a typed view over the same CST. `syntax()` exposes the underlying node and `operands()` yields direct expression children. Call arguments and dictionary entries have `ArgumentList` and `DictionaryEntry` wrappers; they are accessible through CST traversal rather than direct operands.

## Syntax scope

- Numbers, strings, StringName, NodePath, booleans, null, names, self, and built-in constants.
- Groups, arrays, both dictionary styles, and trailing commas.
- Unary and binary operations, `not in`, `is not`, `as`, `await`, and conditional expressions.
- Calls, members, subscripts, `super(...)`, `super.method(...)`, and `preload(...)`.
- `$Root/Child`, `%Unique`, `$Root/%Unique`, quoted paths, and keyword node names.
- Anonymous/named lambdas, typed parameters, defaults, return types, and single/multiline suites, including nesting inside brackets.
- Qualified type names and collection type arguments in `as`/`is`. Only `Array[T]` and `Dictionary[K, V]` are allowed. Argument counts are checked and nested typed collections rejected; type-name resolution is deferred.

Assignment belongs to statements. This entry point reports extra tokens after expressions such as `a = b` and `a += b`. Missing expressions, delimiters, and type names produce syntax diagnostics. Array, call, and dictionary recovery uses commas and closing delimiters to preserve later elements.

Scripts check `super` function context and static instance-access restrictions; standalone expressions assume no host function. Type references in as/is and lambda rest parameters apply [version gates](/reference/language-versions). Preload path types/existence and type-name resolution are not implemented. The parser drives lexer context for lambda-body indentation and newlines, restoring outer brackets and indentation on exit.

## Precedence

The following table is ordered low to high, checked against the pinned [Godot parser](https://github.com/godotengine/godot/blob/ed1daf0bf001b61586d9930840f2f1394092c079/modules/gdscript/gdscript_parser.cpp) and [Precedence enum](https://github.com/godotengine/godot/blob/ed1daf0bf001b61586d9930840f2f1394092c079/modules/gdscript/gdscript_parser.h). Binary operators associate left; ternary conditionals associate right.

| Level | Operation                                                |
| ----- | -------------------------------------------------------- |
| 1     | `as`                                                     |
| 2     | `a if condition else b`                                  |
| 3–5   | `or` / `\|\|`, `and` / `&&`, prefix `not` / `!`          |
| 6–7   | `in` / `not in`, all comparisons and equality operations |
| 8–11  | `\|`, `^`, `&`, `<<` / `>>`                              |
| 12–13 | `+` / `-`, `*` / `/` / `%`                               |
| 14–16 | Prefix signs, `~`, `**`                                  |
| 17–18 | `is` / `is not`, `await`                                 |
| 19–21 | Calls, members, subscripts                               |

In particular, `a ** b ** c` means `(a ** b) ** c`; `a ** b is int` means `a ** (b is int)`; the cast in `a if b else c as Node` wraps the entire conditional.

The pinned tokenizer includes a sign in a numeric token when the preceding token cannot end an operand and the sign touches the number. Thus `-2 ** 2` has literal `-2` as its left operand, while `- 2 ** 2` negates the exponentiation. Formatting preserves the spacing required to retain this tokenization and keeps comments attached to the same tokens.

## Limits and verification

`ParseOptions.lex` supplies the lexical SourceId, size limit, and diagnostic budget. `max_expression_depth` defaults to 128 and can be lowered; the implementation clamps it to 1–128 to protect the Rust stack. Recursion, long binary chains, postfix chains, and type nesting are bounded. Lexer and parser share the diagnostic budget, with at most one extra suppression notice.

Syntax diagnostics use category `syntax`: `PAR001` missing expression, `PAR002` suppression notice, `PAR003` extra token, `PAR004` missing structure, `PAR005` depth limit, `PAR006` invalid direct call, `PAR007` dictionary style, and `PAR008` nested typed collection. See [script parsing](/reference/parsing) for suite diagnostics.

```sh
cargo test -p gdfmt_parser
cargo xtask compatibility
```

Tests assert tree structure, every binary precedence pair, original token order and byte offsets, recovery, extreme depth, diagnostic budgets, and random Unicode input. Expression fixtures verify lexical and syntax diagnostics. The engine harness uses complete scripts; differences such as typed constructors have equivalent script regressions, and standalone expressions are never executed directly by the engine.

In value expressions, `[]` always means subscript. `Array[int]([])` and `Dictionary[String, int]({})` produce syntax errors, verified by [pinned-engine regressions](/reference/compatibility).
