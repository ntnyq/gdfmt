# 兼容性基线与缺口

核验日期：2026-10-21。当前已有词法实现与表达式、声明和语句 parser，尚未声明任何 Godot 版本的完整语法支持。

## 固定参考

| 参考                                                                                                                                              | 固定版本或 commit                        | 用途                         |
| ------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------- | ---------------------------- |
| [Godot 发布归档](https://godotengine.org/download/archive/)                                                                                       | 4.7.2-stable；归档还列出 4.8-beta1       | 稳定与预览基线区分           |
| [Godot tokenizer](https://github.com/godotengine/godot/blob/ed1daf0bf001b61586d9930840f2f1394092c079/modules/gdscript/gdscript_tokenizer.cpp)     | ed1daf0bf001b61586d9930840f2f1394092c079 | 缩进、token 和字符串行为参考 |
| [gdtoolkit grammar](https://github.com/Scony/godot-gdscript-toolkit/blob/f61a1f3a9f2f757813bd5176a28903502bd44959/gdtoolkit/parser/gdscript.lark) | f61a1f3a9f2f757813bd5176a28903502bd44959 | 语法范围与用例分类参考       |
| [Oxc workspace](https://github.com/oxc-project/oxc/blob/e97a8b121811c6fdb915ece70c2de3890bc7897f/Cargo.toml)                                      | e97a8b121811c6fdb915ece70c2de3890bc7897f | crate 职责与依赖方向参考     |

稳定 Godot tag 已通过 git remote 解析为上表 commit。预览 4.8-beta1 的 tag 本次未解析到 commit，在 manifest 中保留 null，保持不支持状态。

`tests/compatibility/manifest.json` 记录固定参考及 292 个原创 fixture 的特性、阶段和预期诊断。其中 16 个单表达式 fixture、262 个脚本 fixture 同时检查词法与语法诊断；其余为词法用例。全部用例为原创；262 个完整脚本已通过固定引擎对照，各版本有 4–5 个单独登记为 analyzer 边界，详见[引擎一致性测试](/zh/development/engine-conformance.md)。Unicode XID 数据派生自固定 Godot commit 的 `core/string/char_range.cpp`，已保留 MIT 归属。

## 特性矩阵

以下证据区分词法、单表达式和脚本阶段。完整语法一致性和 formatter 验收仍待完成。

| 特性组                                | 当前证据                                                                    | 后续工作                                        |
| ------------------------------------- | --------------------------------------------------------------------------- | ----------------------------------------------- |
| BOM、CRLF、空文件、注释、EOF          | 无损回归用例                                                                | parser 与 formatter 关联校验                    |
| tab/space 缩进、注释行、dedent        | 正反例、逻辑与物理换行分离                                                  | 与固定引擎做差异测试                            |
| 括号续行、反斜线续行                  | 按需词法、lambda suite 的括号基线与外层缩进恢复                             | 引擎差异验证                                    |
| Unicode 标识符                        | 固定 Godot Unicode 17 XID 表（含下划线扩展）、中文与组合字符用例            | 引擎差异验证与安全警告                          |
| 数值、运算符、字符串前缀              | 带符号 token、最长匹配、数值边界、错误恢复、特殊诊断 token                  | 数值解码与溢出行为验证                          |
| 多行、raw、StringName、NodePath       | 单引号跨行、UTF-16 对、六位 escape、scalar 范围与方向控制符诊断             | 解码值校验、格式化不变量                        |
| 变量、常量、class、继承、enum、signal | 声明 CST、顺序、唯一性、typed facade、正反例与恢复                          | 类型与继承语义、引擎验证                        |
| 函数、static、类型、参数、lambda      | 默认参数、rest 参数、返回类型、函数上下文、suite 恢复与版本 gates           | 完整函数语义与引擎验证                          |
| 属性 getter/setter                    | 类型可选、get()、body/委托互斥、分隔符、重复 accessor 反例                  | 类型/委托函数语义与引擎验证                     |
| await、super、调用、节点路径          | 分层表达式 CST、正反例与无损恢复                                            | 语句上下文与 Godot 验证                         |
| if/elif/else、循环、return、assert    | 语句 CST、单行与多行 suite、函数与循环上下文                                | 更多错误恢复和引擎验证                          |
| match、模式、guard                    | 数组/字典/绑定/通配/rest 模式、guard 与分支 CST                             | 绑定冲突检查；guard 语义与引擎验证              |
| 类型化数组、字典                      | 声明及 as/is 类型引用、非法泛型式构造器回归；参数数量与嵌套反例、版本 gates | 类型名解析及引擎验证                            |
| annotations、abstract                 | annotation 目标、参数数量、位置；无 body abstract 类/方法、版本 gates       | 完整参数值规则、继承与 abstract 覆盖            |
| namespace、trait、可变参数等新结构    | rest 参数可解析；namespace、trait 仅保留词形                                | P0 对 4.6/4.7/4.8 的引入版本及 grammar 逐项审计 |
| 语言结构约束                          | P3 结构约束已按明确排除项验收                                               | 项目语义排除；不设计 lint/fix 工作流            |
| 格式化、注释附着、结构等价            | Doc 打印器、注释及 lambda/声明/suite 布局，重解析、结构/附着等价与幂等门禁  | P4 alpha 已验收；P5 保护区域已实现              |

parser 不再依赖随版本推进的 `unicode-ident` 表，而使用固定 Godot commit 的 17.0 XID 数据（宏依赖仍可间接包含 `unicode-ident`）。token 识别不代表目标版本接受相应语法；`LanguageVersion` 和 feature gates 已接入 parser 与配置，范围见[语言版本](/zh/reference/language-versions.md)。表达式范围及优先级见 [表达式解析](/zh/reference/expressions.md)。

## 当前验证入口

```sh
cargo test --workspace --all-features
cargo xtask compatibility
cargo xtask conformance --godot /path/to/pinned/godot
```

测试包括无损 token 分区、随机 Unicode 输入与表达式的前进性、全部二元运算符优先级配对、大小与深度限制、诊断抑制、列表恢复、跨线程 green tree 重建。`xtask compatibility` 分别输出词法、单表达式与脚本 fixture 数量；脚本阶段使用 parser 驱动的词法上下文，`parserLexicalErrors` 可覆盖独立词法期望。manifest 另有完整 profile 矩阵，共执行 1310 次版本检查。独立的 `conformance` 检查 262 个脚本与引擎的登记预期，输出 JSON 报告；各版本的 4–5 个语义边界单独分类。

[语法审计](/zh/development/syntax-audit)已登记固定 parser/tokenizer 入口和 675 个官方脚本，记录 190 个直接 parser 探测、五引擎结果及明确后续项。仍待推进类型/继承语义、项目语料、数值/字符串解码、Unicode 近形关键字策略、长期 fuzz、名称与发布权限。[P4 布局记录](/zh/development/formatter-acceptance)说明注释附着、整脚本格式化与五引擎验证。
