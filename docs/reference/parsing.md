# Script parsing

`gdfmt_parser::parse` parses complete scripts, returning `ParseResult` (sharing the `ExpressionResult` structure) with a `SourceFile` root. This is a Rust library entry point; the CLI's `lex` still performs independent lexical analysis only.

```rust
use gdfmt_parser::{ParseOptions, parse};
use gdfmt_syntax::ast::SourceFile;

let source = "var count = 0\nfunc run():\n    count += 1\n";
let result = parse(source, &ParseOptions::default());
assert!(!result.has_errors());
let tree = result.tree.unwrap();
assert_eq!(tree.text(), source);
let file = SourceFile::cast(tree.root()).unwrap();
assert_eq!(file.declarations().count(), 2);
```

## Current syntax scope

- `class`, `extends` with dotted class names or path strings (no collection type arguments), and `class_name`, optionally followed by `extends` on the same line.
- Variables, constants, explicit types, `:=` inference, initialization, and `static var`.
- Named/anonymous enums, entry values, signals, and signal parameters.
- Functions, `static func`, parameter types, `:=` default inference, defaults, final rest parameters, and return types including `void`.
- Annotation nodes and expression arguments; registered annotations' targets, arity, repeatability, script placement, and standalone-line constraints. Argument-value semantics still require an analyzer.
- Getter/setter bodies including `get()`, delegated accessors, and optional property types; duplicate accessors, mixed forms, commas, and indentation are checked.
- Single-line suites (one member for an inline class), statement-separating semicolons (repeatable after a complete statement, forbidden at block start), multiline indented suites, `pass` in empty classes, and comment strings.
- `if/elif/else`, `for` with optional iterator types, `while`, `return`, `assert`, `break/continue`, `pass`, and `breakpoint`.
- Assignment, compound assignment, and expression statements using [Pratt expressions](/reference/expressions).
- `match` branches, alternative patterns, array/dictionary/binding/wildcard/rest patterns, and `when` guards.
- `@abstract` scripts, inner classes, and bodyless methods; abstract ownership, static/constructor restrictions, and duplicate annotations are checked.
- Anonymous/named lambdas, default parameters, return types, single/multiline bodies, and lambdas embedded in calls, arrays, and dictionaries.

Declarations, statements, suites, parameters, and patterns have their own CST nodes. `SourceFile::declarations()` yields direct declarations; `Declaration::body()` gives a direct suite; `Suite::statements()` yields direct statements. Views' `syntax()` methods reference the same CST. Accessors and nested control flow remain accessible through CST traversal.

## Declaration and scope constraints

Validation checks `class_name`/`extends` placement, duplication, and inner-class restrictions, plus collisions between class members, enum entries, parameters, and local names. Anonymous enum entries enter the class-member namespace. Function scopes include parameters, existing locals, loop variables, and match bindings. Names added by a branch are removed on exit. Lambdas see outer locals already declared at their source position; their parameters may shadow these names, while declarations in the body may not. Initializers and iterable expressions are checked before introducing their new variable.

Non-abstract functions require a body. Abstract methods end at a newline, semicolon, or EOF and create no `Suite`. Checks cover `_static_init` static/no-parameter requirements, constructor return types, return values in void functions/constructors, and `self`, `super`, or node paths in static contexts. Property setters have no implicit void return restriction in these five pins. Lambdas inherit outer static restrictions; return constraints apply to the nearest function, lambda, or accessor.

Type arguments are restricted to `Array[T]` and `Dictionary[K, V]`, rejecting nested typed collections. An explicit rest parameter type must be `Array` without type arguments. Declarations and as/is share type nodes and version checks. In value expressions, `[]` after a collection name is still a subscript; generic-style constructors `Array[int]([])` and `Dictionary[String, int]({})` are rejected.

## Lambda lexical context

The parser reads tokens on demand and sets suite context before consuming the lambda-body colon. Brackets in the outer expression temporarily stop suppressing body newlines and indentation; brackets opened inside the body still provide implicit continuation. Exiting the lambda restores the outer mode and, when needed, its indentation stack. Nested lambdas use the same mechanism.

Independent `lex` and parser tokens can therefore differ. For example, mixed indentation inside a bracketed lambda may be diagnosed only in parser-driven mode. Both entry points preserve byte order, trivia, and original text; neither reorders tokens between syntax nodes.

Functions and lambdas establish their own loop context. A lambda cannot use an outer loop's `break/continue`; the outer loop remains valid after the lambda exits.

## Diagnostics and recovery

Lexer and parser share `ParseOptions.lex.max_diagnostics`, adding at most one suppression notice beyond the budget. Oversized input produces no tree; other errors retain a lossless tree. Successful recovery does not make invalid source valid.

`max_expression_depth` limits expression, type, statement, lambda, and pattern recursion. Its default and hard cap are 128. At the block-depth limit, iterative scanning preserves remaining tokens.

In addition to expression diagnostics `PAR001`–`PAR008`:

| Code     | Meaning                                                                                |
| -------- | -------------------------------------------------------------------------------------- |
| `PAR009` | Missing statement end or unexpected block token                                        |
| `PAR010` | Invalid declaration/function/loop context or assignment target                         |
| `PAR011` | Invalid parameter defaults or rest parameter position                                  |
| `PAR012` | Duplicate property accessor or invalid body form                                       |
| `PAR013` | Invalid rest pattern, conflicting alternative bindings, duplicate/conflicting bindings |
| `PAR014` | Invalid annotation name, arity, target, duplication, or placement                      |
| `PAR015` | Declaration ordering, uniqueness, or local name collision                              |
| `PAR016` | Abstract, static, constructor, or return-context restriction                           |
| `PAR017` | Feature unsupported by the selected language version                                   |

Malformed property accessors recover inside their property block, retaining a later valid getter/setter. Simple statements recover at newlines, semicolons, and dedents; lambdas also recognize outer commas and closing brackets. Lists use expression recovery. Unclosed strings can span the remaining source, and unclosed delimiters may suppress later newlines, so recovery cannot guarantee every later declaration becomes an independent node.

## Remaining work

No Godot version has full syntax conformance declared. [Version gates](/reference/language-versions) cover verified differences across 4.3–4.7 profiles; namespace/trait remain unsupported. Complete annotation-value rules, inheritance and abstract overrides, type-name resolution, captured-value/type analysis, resource paths, and runtime behavior require further implementation or pinned-engine validation.

Tests cover structure, abstract binding, declaration constraints, version boundaries, lexical context restoration, later-declaration recovery, truncation at every UTF-8 boundary, token byte offsets, random tokens, budgets, depth limits, and cross-thread reconstruction. `cargo xtask compatibility` checks 292 lexical fixtures, including 16 expression and 262 script fixtures, plus 1310 profile checks; all are original. [Pinned-engine conformance](/development/engine-conformance) covers all 262 scripts, with 4–5 semantic boundaries per version classified separately.
