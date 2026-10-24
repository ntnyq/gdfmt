# 脚本解析

`gdfmt_parser::parse` 解析完整脚本，返回 `ParseResult`（与 `ExpressionResult` 共享结果结构）和 `SourceFile` 根节点。此入口属于 Rust library；CLI 的 `lex` 仍只执行独立词法分析。

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

## 当前语法范围

- `class`、`extends`（点分隔类名或路径字符串，不接受集合类型参数）、`class_name`（可同一行接 `extends`）。
- 变量、常量、显式类型、`:=` 推断、初始化、`static var`。
- 命名或匿名 enum、条目值、signal 及其参数。
- 函数、`static func`、参数类型、`:=` 默认值推断、默认值、末尾 rest 参数、返回类型（含 `void`）。
- annotation 节点及表达式参数；已登记 annotation 的目标、参数数量、重复使用、script 位置和独立行约束。参数值语义仍需 analyzer。
- getter/setter body（含 `get()`）与委托函数形式、可选属性类型；检查重复 accessor、两种形式混用、逗号与缩进。
- 单行 suite（单行内部类仅有一个成员）、分号分隔语句（完整语句后可连续出现，不能位于块首）、多行缩进 suite、空类的 `pass`、注释字符串。
- `if/elif/else`、带可选迭代变量类型的 `for`、`while`、`return`、`assert`、`break/continue`、`pass`、`breakpoint`。
- 赋值与复合赋值、表达式语句，复用 [Pratt 表达式](/zh/reference/expressions.md)。
- `match` 分支、备选模式、数组/字典/绑定/通配/rest 模式和 `when` guard。
- `@abstract` 脚本、内部类和无 body 方法，检查所属 abstract 类、static/构造器限制与重复 annotation。
- 匿名或命名 lambda、默认参数、返回类型、单行与多行 body，以及嵌入调用、数组、字典的 lambda。

声明、语句、suite、参数和模式各自构建 CST 节点。`SourceFile::declarations()` 返回直接声明；`Declaration::body()` 返回直接 suite；`Suite::statements()` 返回直接语句。视图的 `syntax()` 指向同一棵 CST，属性访问器和嵌套控制流可通过 CST 遍历。

## 声明与作用域约束

检查 `class_name`/`extends` 的位置、重复和嵌套类限制，类成员、enum 条目、参数及局部名字冲突；匿名 enum 的条目进入类成员空间。函数参数、已有局部、循环变量、match 绑定参与函数内的作用域检查。分支退出后撤销新增名字。lambda 能看到源码位置之前已声明的外层局部名字；参数可以遮蔽这些名字，body 中的声明不能重复使用。初始化表达式和 iterable 在引入新变量前检查。

非 abstract 函数必须有 body；abstract 方法以换行、分号或 EOF 结束，不生成 `Suite`。检查 `_static_init` 的 static/无参数限制、构造器返回类型、`void`/构造器返回值，以及 static 上下文中的 `self`、`super`、节点路径。这五个固定版本不会给 setter 隐式施加 void 返回限制。lambda 保留其外层 static 限制，返回约束按最近的函数/lambda/accessor 判断。

类型参数仅允许 `Array[T]` 和 `Dictionary[K, V]`，仍拒绝嵌套类型化集合；rest 参数的显式类型只能是无类型参数的 `Array`。声明和 as/is 使用相同的类型节点与版本检查。值表达式中的集合名后接 `[]` 仍是下标，不接受 `Array[int]([])` / `Dictionary[String, int]({})` 形式的泛型式构造器。

## lambda 的词法上下文

parser 按需读取 token，在消费 lambda body 的冒号前设置 suite 上下文。外层表达式的括号暂时不再抑制 body 的换行和缩进；body 自己新开的括号仍保留隐式续行。退出 lambda 后恢复外层模式，并在需要时恢复外层缩进栈。嵌套 lambda 使用相同机制。

因此，独立 `lex` 的虚拟 token 与 parser 的 token 不保证相同。例如括号内 lambda 的混合缩进可能只在 parser 驱动时报告。两种入口都保持字节顺序、trivia 与原始文本，不在语法节点间重排 token。

函数和 lambda 建立自己的循环上下文。lambda 中不能使用外层循环的 `break/continue`；退出 lambda 后外层循环仍然有效。

## 诊断与恢复

词法和语法共享 `ParseOptions.lex.max_diagnostics`，超预算最多增加一条抑制提示。源码超过大小限制时不构树；其他错误保留无损树。语法错误不能因为恢复成功而视为有效源码。

`max_expression_depth` 同时限制表达式、类型、语句、lambda 和模式递归；默认及硬上限为 128。到达 block 深度限制时，使用迭代扫描保留剩余 token。

沿用表达式的 `PAR001`–`PAR008`；新增诊断如下：

| code     | 含义                                              |
| -------- | ------------------------------------------------- |
| `PAR009` | 缺少语句结束或意外 block token                    |
| `PAR010` | 声明/函数/循环上下文或赋值目标错误                |
| `PAR011` | 参数默认值或 rest 参数位置错误                    |
| `PAR012` | 重复属性访问器或 body 形式错误                    |
| `PAR013` | rest 模式位置、绑定与备选模式冲突、重复或冲突绑定 |
| `PAR014` | annotation 名称、参数数量、目标、重复或位置错误   |
| `PAR015` | 声明顺序、唯一性或局部名字冲突                    |
| `PAR016` | abstract、static、构造器和返回上下文限制          |
| `PAR017` | 当前目标版本不支持该特性                          |

属性 accessor 出错后在属性块内恢复，保留后续合法 getter/setter。简单语句错误以换行、分号、dedent 为恢复边界；lambda 还识别外层逗号和闭括号。列表沿用表达式恢复。未闭合字符串可能覆盖剩余源码，未闭合的分隔符可能抑制后续换行，恢复不保证每个后续声明都能成为独立节点。

## 范围与限制

目前不声明任何 Godot 版本的完整语法一致性。[版本 gates](/zh/reference/language-versions.md)覆盖 4.3–4.7 profiles 的已核验差异；namespace/trait 仍不支持。annotation 参数值仅检查部分字面量限制，不完整求值。继承与 abstract 覆盖关系、类型名解析、捕获值和类型分析、资源路径检查与运行时行为仍由 Godot 负责，不是待实现的 formatter 功能。公开范围见[兼容性说明](/zh/reference/compatibility)。

测试覆盖结构、abstract 绑定、声明约束、版本边界、词法上下文恢复、后续声明恢复、全部 UTF-8 字符边界截断、token 字节偏移、随机 token 输入、诊断预算、深度限制和跨线程无损重建。`cargo xtask compatibility` 检查 292 个词法 fixture，其中 16 个也检查单表达式、262 个也检查脚本，并执行 1310 次 profile 检查；全部为原创用例。[固定引擎 conformance](/zh/reference/compatibility) 已覆盖全部 262 个脚本，各版本的 4–5 个语义边界单独分类。
