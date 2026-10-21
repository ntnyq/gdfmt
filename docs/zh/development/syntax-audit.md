# P3 语法审计与验收记录

审计日期：2026-10-21。验收结论为**带明确排除项验收通过**。本记录定义准备验收的有限语法契约，不声明完整的 Godot 语义兼容性。

权威清单为 [`syntax-coverage.json`](https://github.com/ntnyq/gdfmt/blob/main/tests/compatibility/syntax-coverage.json)：50 个语法条目（48 个覆盖、两个仅产生诊断的排除项）、556 个逐版本源码符号处置记录、五个固定版本共 173 项注解注册，以及基线全部 675 个官方测试脚本的分类。每个覆盖条目都关联上游符号、固定提交中的官方测试 blob、本地原创正反例、最早支持版本或引入版本、语义边界和排除原因。`4.3-or-earlier` 表示在最早支持的版本中已经存在，不推断历史引入时间。

## 固定来源与覆盖契约

`sourceFiles` 用提交和 SHA-256 标识 parser、tokenizer 及头文件的准确内容。清单枚举 parser 的 `parse*` 入口、恢复、注解注册/验证/回调、文本 tokenizer 方法，以及标识符、节点名称、二元运算前置 token 的判定方法。每个定义都关联语法条目或给出排除理由；重载定义保留各自源码行号。

| 配置 | 固定一手来源                                                                                                                                                                                                                                                               |
| ---- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 4.3  | [Parser](https://github.com/godotengine/godot/blob/77dcf97d82cbfe4e4615475fa52ca03da645dbd8/modules/gdscript/gdscript_parser.cpp)、[tokenizer](https://github.com/godotengine/godot/blob/77dcf97d82cbfe4e4615475fa52ca03da645dbd8/modules/gdscript/gdscript_tokenizer.cpp) |
| 4.4  | [Parser](https://github.com/godotengine/godot/blob/4c311cbee68c0b66ff8ebb8b0defdd9979dd2a41/modules/gdscript/gdscript_parser.cpp)、[tokenizer](https://github.com/godotengine/godot/blob/4c311cbee68c0b66ff8ebb8b0defdd9979dd2a41/modules/gdscript/gdscript_tokenizer.cpp) |
| 4.5  | [Parser](https://github.com/godotengine/godot/blob/876b290332ec6f2e6d173d08162a02aa7e6ca46d/modules/gdscript/gdscript_parser.cpp)、[tokenizer](https://github.com/godotengine/godot/blob/876b290332ec6f2e6d173d08162a02aa7e6ca46d/modules/gdscript/gdscript_tokenizer.cpp) |
| 4.6  | [Parser](https://github.com/godotengine/godot/blob/89cea143987d564363e15d207438530651d943ac/modules/gdscript/gdscript_parser.cpp)、[tokenizer](https://github.com/godotengine/godot/blob/89cea143987d564363e15d207438530651d943ac/modules/gdscript/gdscript_tokenizer.cpp) |
| 4.7  | [Parser](https://github.com/godotengine/godot/blob/ed1daf0bf001b61586d9930840f2f1394092c079/modules/gdscript/gdscript_parser.cpp)、[tokenizer](https://github.com/godotengine/godot/blob/ed1daf0bf001b61586d9930840f2f1394092c079/modules/gdscript/gdscript_tokenizer.cpp) |

| 分类       | 条目数 | 验收范围                                                                                    |
| ---------- | -----: | ------------------------------------------------------------------------------------------- |
| 词法       |      5 | 布局/续行、Unicode 标识符、数字/字符串拼写、标点和定界符诊断                                |
| 声明       |      8 | 脚本/类头、成员、变量/常量、属性、参数/函数、信号和枚举                                     |
| 语句       |      9 | 语句块/分隔符、条件/循环/控制上下文、return/assert、表达式和赋值                            |
| 模式       |      2 | match 分支/守卫、绑定/通配/rest/数组/字典模式及结构性绑定作用域                             |
| 表达式     |     15 | Pratt 优先级、原子/运算符、await、容器、分组、属性、下标、调用、节点路径、preload 和 lambda |
| 类型       |      1 | 类型拼写、限定名称、容器参数及显式 `as`/`is` 位置                                           |
| 注解结构   |      1 | 注册名称、目标、参数表达式/数量、绑定及 icon/region 的字符串字面量限制                      |
| 版本门控   |      7 | 4.4 的类型化字典/工具按钮/警告区域；4.5 的抽象声明/rest 参数/文件路径；4.6 的多行导出提示   |
| 仅诊断语法 |      2 | 已移除的 `yield` 及非法 `?` 始终拒绝，不存在支持的正例形式                                  |

`namespace` 和 `trait` 在固定 tokenizer 中是保留 token 拼写，不是支持的声明。识别拼写不代表引入语言特性。文本注释无损保留；文档语义提取/BBCode、编辑器光标状态和序列化字节码解析均明确排除。

## 官方测试审计与修正

基线[官方测试目录](https://github.com/godotengine/godot/tree/ed1daf0bf001b61586d9930840f2f1394092c079/modules/gdscript/tests/scripts)包含 675 个 `.gd` 脚本，全部登记分类，不据此声称全部执行。此次获取全部 190 个 parser 脚本（87 个特性、76 个错误、27 个警告），逐个校验内容与 Git blob SHA-1 一致，再解析未改动的源码。950 次脚本/配置解析均保持源码无损。接受/拒绝判定对比使用 4.7 配置：186 项符合语法预期，另有四项明确边界。未比较警告输出或官方 `.out` 运行结果。旧配置下无损解析不表示新版本测试在旧版本中有效。

审计发现并修复以下缺口，新增 20 个原创 `audit-syntax-*.gd` 回归脚本并登记五引擎矩阵：

- `PI`、`TAU`、`INF`、`NAN` 可作声明标识符，依据 [`Token::is_identifier`](https://github.com/godotengine/godot/blob/ed1daf0bf001b61586d9930840f2f1394092c079/modules/gdscript/gdscript_tokenizer.cpp#L195)。
- 属性及节点名称上下文允许保留关键字白名单，包括 `_`，依据 [`Token::is_node_name`](https://github.com/godotengine/godot/blob/ed1daf0bf001b61586d9930840f2f1394092c079/modules/gdscript/gdscript_tokenizer.cpp#L216)；`true` 等字面量仍不可作属性名称。
- match 主体允许独立且以换行结束的 `pass`，以及绑定到后续分支的 `@warning_ignore`；拒绝无目标/错误目标注解及该特殊 `pass` 后的分号，依据 [`parse_match`](https://github.com/godotengine/godot/blob/ed1daf0bf001b61586d9930840f2f1394092c079/modules/gdscript/gdscript_parser.cpp#L2426)。
- `@icon` 必须接收字符串字面量，解码后不得为空，依据 [`validate_annotation_arguments` 和 `icon_annotation`](https://github.com/godotengine/godot/blob/ed1daf0bf001b61586d9930840f2f1394092c079/modules/gdscript/gdscript_parser.cpp#L4406)。

新增反例分别覆盖非法数组元素、缺失属性/preload/await/三元表达式操作数、非法节点路径字面量、已移除的 `yield` 和非法 `?`。相应覆盖表条目使用这些直接相关的反例。

## 明确排除与后续事项

以下排除构成验收契约，不隐含计为语法通过。

| 跟踪 ID                  | 排除范围与依据                                                                                                                                                                                                                                                                                                                                                                                                                                                    |
| ------------------------ | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `SYN-CONFUSABLE-KEYWORD` | Rust 接受官方 `parser/errors/identifier_similar_to_keyword.gd`。Godot 在 tokenizer 的 [`potential_identifier`](https://github.com/godotengine/godot/blob/ed1daf0bf001b61586d9930840f2f1394092c079/modules/gdscript/gdscript_tokenizer.cpp#L594) 中调用 TextServer Unicode 安全检查拒绝与关键字混淆的拼写。这是明确的词法安全差异，不归类为 analyzer。                                                                                                             |
| `SEM-ANNOTATION-VALUES`  | 导出枚举的属性/数组类型及工具按钮的工具模式要求构成另外三个官方 parser 目录例外。目录名称不决定阶段：[`validate_annotation_arguments`](https://github.com/godotengine/godot/blob/ed1daf0bf001b61586d9930840f2f1394092c079/modules/gdscript/gdscript_parser.cpp#L4406) 明确将一般已解析参数交给 `GDScriptAnalyzer::resolve_annotation`，回调读取解析后的值/类型。名称、目标、数量以及 icon/region 的字面量语法仍纳入覆盖；警告名称目录、RPC/导出值和资源检查排除。 |
| `SEM-ANALYZER`           | 名称/类型/继承解析、常量求值、抽象覆写、实际返回/赋值类型和数字/字符串解码值等价。222 个 analyzer 脚本登记分类，不声明 analyzer 兼容。                                                                                                                                                                                                                                                                                                                            |
| `SEM-RUNTIME`            | 118 个 runtime 脚本需要 VM 及执行/输出对比，本验收不包含运行时语义。                                                                                                                                                                                                                                                                                                                                                                                              |
| `EDITOR-INTEGRATION`     | 142 个 completion/LSP 脚本需要编辑器哨兵、项目状态和协议行为。                                                                                                                                                                                                                                                                                                                                                                                                    |
| `PROJECT-CORPUS`         | 三个共享/辅助脚本以及多文件/资源交互需要项目语料；当前引擎工具逐脚本隔离。                                                                                                                                                                                                                                                                                                                                                                                        |

50 个语法条目是对限定语法族及固定入口的已审阅清单，不证明所有可能输入或所有上游语义分支已经执行。有限矩阵不构成完整语言、AST/字面量值或运行时等价声明。

## 可复现验收检查

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
# 对其余四个固定引擎分别运行 conformance，并使用不同的报告路径。
```

`syntax-coverage` 检查固定版本一致性、条目证据、fixture 解析结果、完整源码符号处置、官方测试分类和注解注册引用。普通 CI 不获取或执行上游测试；已提交的 blob 哈希和探测结果标识审计源码快照。`compatibility` 检查全部 285 个原创 fixture，其中 255 个脚本在每个引擎配置中都有明确分类，共 1275 次引擎对比。

Token 变异测试对每个 fixture 的所有物理 token 在五种配置中应用删除、重复、定界符、关键字、换行/缩进及赋值替换。每种配置在独立子进程中运行并设 60 秒 watchdog；崩溃、超时、缺少语法树或源码不一致均失败。验收中每配置检查 45510 次变异，总计 227550 次，未发生崩溃、超时或源码不一致。该有限确定性变异集补充定点恢复回归，持续 fuzzing 仍属后续工作。

验收运行中，1275 次引擎对比零失败，1285 次兼容性版本检查及 227550 次 token 变异（每配置 45510 次）全部通过，源码/官方测试清单检查通过。上述结果完成了限定范围内、保留明确排除项的 P3 语法验收；上列其余工作区检查也用于保护后续实现变更。随后 P4 单独从 Doc 打印器及表达式/调用/容器格式化开始，使用重解析、结构等价和幂等性验证后再接入 CLI；此次 parser 审计不包含格式化器验收。
