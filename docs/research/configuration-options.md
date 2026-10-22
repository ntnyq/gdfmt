# 配置扩展评估

调研日期：2026-10-22。

本文记录分阶段设计。第一批 8 个纯布局选项已实现，取值与默认值见[配置指南](../guide/configuration.md#layout-policies)，默认输出保持不变。后续批次仍为候选，运行时会拒绝尚未实现的字段。引入新默认风格应单独评估。

## 外部参考

以下上游源码指调研时的开发分支，不代表已经进入某个发行版。建议来自官方文档与实现；后文的候选 API 是结合 gdfmt 架构作出的设计推论。

| 来源                         | 已核实的能力                                                                                             | 对 gdfmt 的启发                                                                    |
| ---------------------------- | -------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------- |
| gdtoolkit / gdformat         | 当前源码提供 `line_length` 和 `use_spaces`；`excluded_directories`、`safety_checks` 属于发现和执行策略。 | 配置数量少是工具选择，不是 GDScript 的语法上限；不要把 gdlint 规则当成格式化选项。 |
| GDQuest / GDScript-formatter | CLI 提供声明间空行、续行缩进和 `preserve` / `single` / `double` 引号偏好；未暴露尾逗号配置。             | 空行和续行是已有同类工具验证过的配置维度；引号属于另一类改写能力。                 |
| Prettier / Biome             | Prettier 提供引号偏好、尾逗号和对象换行策略；Biome 提供集合展开与括号内空格策略。                        | 可以借鉴配置维度和作用范围，但须按 GDScript 语法重新定义。                         |

来源：[gdformat 配置源码](https://github.com/Scony/godot-gdscript-toolkit/blob/master/gdtoolkit/formatter/__init__.py)、[gdformat CLI](https://github.com/Scony/godot-gdscript-toolkit/blob/master/gdtoolkit/formatter/__main__.py)、[gdlint 配置](https://github.com/Scony/godot-gdscript-toolkit/blob/master/gdtoolkit/linter/__init__.py)、[GDQuest CLI](https://github.com/GDQuest/GDScript-formatter/blob/main/src/cli.rs)、[GDQuest 配置类型](https://github.com/GDQuest/GDScript-formatter/blob/main/src/lib.rs)、[Prettier 选项](https://prettier.io/docs/options)、[Biome 配置](https://biomejs.dev/reference/configuration/)。

gdformat 官方 Wiki 仍写着只有行宽可配置，与当前源码不一致；本评估以实现为准。GDQuest 的引号策略也不是强制重写所有字面量：内容已有目标分隔符时会保留原文，且不重写 escapes。它自动处理尾逗号的行为不等于提供用户配置。

来源：[gdformat Wiki](https://github.com/Scony/godot-gdscript-toolkit/wiki/4.-Formatter)、[GDQuest 字符串与容器实现](https://github.com/GDQuest/GDScript-formatter/blob/main/src/formatter.rs)。

Godot 官方风格指南建议函数和类之间两空行、一般续行增加两层缩进，而数组、字典、枚举增加一层；多行集合使用尾逗号，单行字典花括号内留空格。这些是风格建议，不是全部都属于语法要求。因此不应把一种统一的 continuation 策略应用于所有节点，也不能把格式化默认值与完整 Godot 兼容性划等号。

来源：[Godot GDScript 风格指南](https://docs.godotengine.org/en/stable/tutorials/scripting/gdscript/gdscript_styleguide.html#formatting)、[GDScript 语言参考](https://docs.godotengine.org/en/stable/tutorials/scripting/gdscript/gdscript_basics.html)。

## 现状与扩展基础

评估前已有 6 个排版选项：`lineWidth`、`indentStyle`、`indentWidth`、`lineEnding`、`maxBlankLines`、`finalNewline`。`language.godotVersion` 是语法配置。配置继承、按文件覆盖、忽略规则、EditorConfig 和字段来源解释也已实现，无须重新建设配置框架。

来源：[配置定义](https://github.com/ntnyq/gdfmt/blob/main/crates/gdfmt_config/src/lib.rs)、[配置指南](../guide/configuration.md)、[配置解析与来源](https://github.com/ntnyq/gdfmt/blob/main/crates/gdfmt_workspace/src/configuration.rs)。

列表布局已统一通过 `Layout::delimited` 和 Doc 的分组、换行、缩进实现。数组、字典、调用参数、函数形参等共享基础设施；可以按语法节点选择策略。当前 `Layout` 只保存部分配置，新增布局策略需要显式传入。

来源：[布局实现](https://github.com/ntnyq/gdfmt/blob/main/crates/gdfmt_formatter/src/layout.rs)、[Doc 打印器](https://github.com/ntnyq/gdfmt/blob/main/crates/gdfmt_formatter/src/doc.rs)。

## 第一批：保留有效 token 的布局选项

以下 8 项已按评估方案实现，不是其他工具配置的直接复制。字段放在 `format` 内，沿用现有 camelCase。

| 候选字段                      | 候选值与默认值                                 | 作用与限制                                                                                                         |
| ----------------------------- | ---------------------------------------------- | ------------------------------------------------------------------------------------------------------------------ |
| `arrayLayout`                 | `auto` / `multiline` / `preserve`；默认 `auto` | 数组按宽度展开、强制展开或保留已有多行意图。空数组保持紧凑；`preserve` 不保留任意空白，也不强制超宽内容留在单行。  |
| `dictionaryLayout`            | 同上                                           | 字典可独立选择布局；保留原有 `:` / `=` 分隔形式。                                                                  |
| `enumLayout`                  | 同上                                           | 适合一个成员一行的枚举；复用列表基础设施，但不改变成员顺序、名称或值。                                             |
| `dictionarySpacing`           | `compact` / `spaced`；默认 `compact`           | 控制单行非空字典花括号内的空格，支持官方指南的 `{ "key": 1 }` 风格；空字典仍为 `{}`，不影响数组与调用。            |
| `trailingCommaLayout`         | `auto` / `multiline`；默认 `auto`              | `multiline` 让已有尾逗号的非空列表强制展开；不增删逗号。与任一要求展开的布局策略组合时，展开优先。                 |
| `blankLinesAroundDefinitions` | `preserve` 或整数 0–5；默认 `preserve`         | 对脚本或类作用域内函数、内部类与相邻声明之间的空行作规范化；不在作用域首尾补空行，不拆开注解、文档注释和目标声明。 |
| `lambdaBodyStyle`             | `auto` / `multiline`；默认 `auto`              | `auto` 保持现有单条简单语句可内联的行为；`multiline` 强制块体。复杂或含注释的 lambda 始终服从语法和注释约束。      |
| `inlineCommentSpacing`        | 整数 1–4；默认 1                               | 调整代码与行尾 `#` 之间的空格；不改写注释 token 内的文本。                                                         |

`maxBlankLines` 当前只限制保留的已有空行，不会自动补齐声明间隔。建议 `blankLinesAroundDefinitions` 显式设置数字时优先决定目标间隔；`preserve` 时仍由 `maxBlankLines` 限制。两者不能在每次格式化时交替覆盖，否则可能破坏幂等性。

来源：[空行、行尾注释、列表与 lambda 实现](https://github.com/ntnyq/gdfmt/blob/main/crates/gdfmt_formatter/src/layout.rs)、[当前行为与不变量](../reference/formatting.md)。

示例：下面的变化只需要布局能力，不需要插入尾逗号。

```gdscript
# 输入
var points = [Vector2.ZERO, Vector2.ONE,]

# trailingCommaLayout = "multiline"
var points = [
	Vector2.ZERO,
	Vector2.ONE,
]
```

## 第二批：先扩展换行能力，再决定配置粒度

- 调用参数和函数形参布局：可以沿用 `auto` / `multiline` / `preserve`，但应先用真实项目验证，是否值得分别公开字段，避免为每种语法机械增加开关。
- 续行缩进：当前续行和 suite 共用缩进单位。若引入独立策略，应明确它表示缩进层数还是显示列数，以及 tab 下的行为。含块 lambda 的列表不能直接套用普通续行缩进。
- 二元表达式、条件表达式和调用链换行：当前没有完整的可断行布局；应先实现合法断点。不能仅增加 `operatorPosition` 等配置字段就宣称已支持自动换行。
- 注解布局：只改变合法注解与声明之间的布局，必须保留顺序、目标绑定及注释关联；不与注解排序混合。

来源：[表达式与缩进分支](https://github.com/ntnyq/gdfmt/blob/main/crates/gdfmt_formatter/src/layout.rs)、[已知宽度限制](../reference/formatting.md)。

## 第三批：需要限定改写规则的选项

| 候选字段         | 候选值                            | 实现前提                                                                                                                                                                            |
| ---------------- | --------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `quoteStyle`     | `preserve` / `single` / `double`  | 先实现相同字符串值的校验与重编码；逐类处理普通字符串、转义、raw、三引号、StringName 和 NodePath。第一版可只覆盖普通单行字符串，其余保持原样并明确记录范围。                         |
| `trailingComma`  | `preserve` / `multiline` / `none` | 仅在已验证允许的位置插入或删除尾逗号；分别覆盖数组、字典、枚举、调用、形参和模式，不能将字面量与类型参数一概处理。`none` 不能删除为 lambda 消歧所需的逗号，应保留或拒绝不安全改写。 |
| `semicolonStyle` | `preserve` / `remove`             | 在语句已拆行后，只移除已确认等价的语句分隔符；保留注释、suite 归属和控制流。                                                                                                        |

默认均应为 `preserve`。这些选项可以属于格式化，但当前实现会将相应输出判为不等价，不能只增加配置枚举。

目前有三处需要共同考虑：

1. `structurally_equivalent` 比较 CST 拓扑与有效 token 的种类、文本；括号、逗号、分号及字面量拼写都参与比较。
2. 注释关联使用相邻有效 token 的序号，增删标点会改变序号，需要与改写规则一致的关联映射。
3. `preserve_regions` 按原始 token 文本在输出中顺序查找；即使字符串位于保护区外，引号改写也可能使映射失败。

应为每类允许的改写建立窄范围的等价规则和映射；不能通过忽略全部标点或所有字符串文本绕过保护。输出仍须重解析、保护区域逐字节保留、注释关联一致并通过第二遍幂等校验。

来源：[结构比较与保护区域映射](https://github.com/ntnyq/gdfmt/blob/main/crates/gdfmt_formatter/src/lib.rs)、[可写输出校验](https://github.com/ntnyq/gdfmt/blob/main/crates/gdfmt/src/lib.rs)、[回归测试](https://github.com/ntnyq/gdfmt/blob/main/crates/gdfmt_formatter/tests/format.rs)。

## 实施边界与验证

建议先交付第一批布局选项，再单独推进字符串和标点改写。格式化选项不应混入类型推断、声明重排、成员排序、命名检查或 lint 规则；这些超出当前项目契约。

新增选项应同步更新：

- `gdfmt_config`：局部配置、解析后配置、默认值、逐字段合并、输入验证和生成 schema。
- `gdfmt_formatter` 与 workspace 适配：实际布局、语法上下文限制、配置到格式化请求的映射。
- 配置来源：默认来源和显式字段来源，使 `explain-config` 及 overrides 对新选项保持一致。
- npm：公开类型声明、同步与异步 API、安装包测试。N-API 复用 Rust 配置解析和 workspace 映射，不应另建一份默认值逻辑。
- 文档与样例：中英文配置及行为说明；更新受影响的快照和兼容性预期。

来源：[配置结构](https://github.com/ntnyq/gdfmt/blob/main/crates/gdfmt_config/src/lib.rs)、[workspace 适配](https://github.com/ntnyq/gdfmt/blob/main/crates/gdfmt_workspace/src/lib.rs)、[来源跟踪](https://github.com/ntnyq/gdfmt/blob/main/crates/gdfmt_workspace/src/configuration.rs)、[N-API 适配](https://github.com/ntnyq/gdfmt/blob/main/crates/gdfmt_napi/src/lib.rs)、[npm 类型](https://github.com/ntnyq/gdfmt/blob/main/packages/gdfmt/src/index.d.cts)。

测试重点是选项真的改变目标行为，同时保留默认输出。覆盖嵌套容器、尾部及悬空注释、文档注释和注解、块 lambda、typed collections、格式化保护区、不同语言 profile，以及选项组合的幂等性。自动改写字符串和标点还需要无效改写被拒绝的反例，及仓库规定的固定 Godot 引擎比较。

当前 typed collection 的类型引用保持不可拆分；即使设置强制多行，也不能把字面量的换行策略套到类型的 `[` 后。未先验证语法的情况下，不能靠自动增加括号或反斜杠来强行满足宽度。

来源：[typed collection 回归](https://github.com/ntnyq/gdfmt/blob/main/crates/gdfmt_formatter/tests/format.rs)、[贡献者验证流程](https://github.com/ntnyq/gdfmt/blob/main/CONTRIBUTING.md)、[格式化契约](../reference/formatting.md)。
