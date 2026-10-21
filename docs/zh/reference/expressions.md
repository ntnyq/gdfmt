# 单表达式解析

`gdfmt_parser::parse_expression` 解析一个独立表达式，返回分层 Rowan 无损 CST 和结构化诊断。支持 lambda 表达式及其 suite；表达式外的声明、赋值语句和完整 `.gd` 文件使用 [脚本入口 `parse`](/zh/reference/parsing.md)。[CLI](/zh/reference/cli) 用于格式化与检查完整脚本。

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

`ExpressionResult::tree` 仅在源码超过词法字节限制时为 `None`。其他无效输入仍有树，并保留完整文本和错误节点。根节点为 `ExpressionRoot`，逻辑 Newline、Indent、Dedent、Eof 仍为零宽 token。注释、空白、BOM、物理换行和续行保留原顺序与字节范围。

`Expression` 是同一棵 CST 的类型化视图；`syntax()` 提供底层节点，`operands()` 返回直接表达式子节点。调用参数和字典条目有各自的 `ArgumentList`、`DictionaryEntry` 包装，不作为直接操作数返回；它们可通过 CST 遍历。

## 语法范围

- 数值、字符串、StringName、NodePath、布尔值、null、名称、self、内置常量。
- 分组、数组、两种字典风格、尾逗号。
- unary、binary、`not in`、`is not`、`as`、`await` 和条件表达式。
- 调用、成员、下标、`super(...)`、`super.method(...)`、`preload(...)`。
- `$Root/Child`、`%Unique`、`$Root/%Unique`、引号路径与关键字节点名。
- 匿名或命名 lambda、类型化参数、默认值、返回类型、单行和多行 suite（含括号内嵌套）。
- `as`/`is` 中的限定类型名、集合类型参数。只允许 `Array[T]` 和 `Dictionary[K, V]`，检查参数个数并拒绝嵌套类型化集合；类型名解析留给后续阶段。

赋值只属于语句；此入口对 `a = b`、`a += b` 等报告表达式后的多余 token。缺失表达式、分隔符和类型名均产生语法诊断。数组、调用与字典中的错误以逗号和右分隔符为恢复边界，保留后续元素。

脚本中检查 `super` 的函数上下文和 static 实例访问限制；独立表达式入口不假定宿主函数。as/is 类型引用及 lambda rest 参数均应用[版本 gates](/zh/reference/language-versions.md)。preload 路径类型与存在性、类型名解析仍未实现。lambda body 的缩进与换行由 parser 驱动 lexer 上下文；退出 body 后恢复外层括号和缩进状态。

## 优先级

依据固定 [Godot parser](https://github.com/godotengine/godot/blob/ed1daf0bf001b61586d9930840f2f1394092c079/modules/gdscript/gdscript_parser.cpp) 和 [Precedence enum](https://github.com/godotengine/godot/blob/ed1daf0bf001b61586d9930840f2f1394092c079/modules/gdscript/gdscript_parser.h) 核对，以下从低到高排列。二元运算符均左结合，三元条件表达式右结合。

| 层级  | 运算                                          |
| ----- | --------------------------------------------- |
| 1     | `as`                                          |
| 2     | `a if condition else b`                       |
| 3–5   | `or` / `\|\|`、`and` / `&&`、前缀 `not` / `!` |
| 6–7   | `in` / `not in`、全部比较与相等运算           |
| 8–11  | `\|`、`^`、`&`、`<<` / `>>`                   |
| 12–13 | `+` / `-`、`*` / `/` / `%`                    |
| 14–16 | 前缀正负号、`~`、`**`                         |
| 17–18 | `is` / `is not`、`await`                      |
| 19–21 | 调用、成员、下标                              |

特别注意：`a ** b ** c` 是 `(a ** b) ** c`；`a ** b is int` 是 `a ** (b is int)`；`a if b else c as Node` 的转换包裹整个条件表达式。

固定 tokenizer 会在前一 token 不能结束操作数、且符号紧邻数字时，将正负号纳入数值 token。所以 `-2 ** 2` 的左操作数是字面量 `-2`，而 `- 2 ** 2` 是对幂表达式取负。空白和注释的保留对后续格式化尤其重要。

## 限制与验证

`ParseOptions.lex` 复用词法的 SourceId、大小和诊断预算。`max_expression_depth` 默认为 128，可调低；实现将其限制在 1–128，保护 Rust 调用栈。递归、长二元链、后缀链和类型嵌套都受限制。词法和语法共享诊断预算，至多多出一条抑制提示。

语法诊断使用 `syntax` category：`PAR001` 缺失表达式，`PAR002` 抑制提示，`PAR003` 多余 token，`PAR004` 结构缺失，`PAR005` 深度限制，`PAR006` 非法直接调用，`PAR007` 字典风格，`PAR008` 嵌套类型化集合。suite 中的额外诊断见 [脚本解析](/zh/reference/parsing.md)。

```sh
cargo test -p gdfmt_parser
cargo xtask compatibility
```

测试断言树结构、所有二元优先级配对、原始 token 顺序与字节偏移、错误恢复、极端深度、诊断预算和随机 Unicode 输入。兼容性 corpus 中单表达式 fixtures 同时验证词法和语法诊断。引擎 harness 使用完整脚本，类型化构造器等差异另有等价脚本回归；独立表达式不会直接交给引擎执行。

值表达式的 `[]` 始终是下标；`Array[int]([])` / `Dictionary[String, int]({})` 会报告语法错误。此行为由[固定引擎回归](/zh/reference/compatibility)验证。
