# 初始架构与契约

当前已有 P1 基础、P2 词法实现与 P3 表达式、声明和语句解析。`parse` 提供脚本入口；abstract 声明、语法约束与版本 gates 已接入；P3 已[按明确排除项验收](/zh/development/syntax-audit)，[P4 脚本格式化](/zh/reference/formatting)与验证后的 CLI 已开放；P4 源码构建 alpha 已[按项目试用排除项验收](/zh/development/formatter-acceptance)，引擎语义不属于此范围。

## 已创建模块

| crate             | 职责                                                                       | 正常依赖                                      |
| ----------------- | -------------------------------------------------------------------------- | --------------------------------------------- |
| `gdfmt_text`      | SourceId、UTF-8 Span、行索引、UTF-16 位置、结构化诊断、TextEdit 校验与应用 | serde、thiserror                              |
| `gdfmt_syntax`    | SyntaxKind、Token、LanguageVersion/Feature、Rowan 无损存储                 | text、rowan、serde、schemars                  |
| `gdfmt_parser`    | lexer、固定 Unicode XID 表、Pratt 表达式、声明与语句、按需词法与语法恢复   | syntax、text、rowan                           |
| `gdfmt_formatter` | 私有 Doc 打印器、表达式/调用/容器布局；正常依赖不含 parser/文件系统        | syntax、text、thiserror                       |
| `gdfmt`           | 纯文本 facade，执行重解析、结构与幂等门禁                                  | parser、formatter、syntax、text               |
| `gdfmt_config`    | Partial/Resolved 配置、语言版本、严格反序列化、Schema                      | syntax、serde、schemars                       |
| `gdfmt_workspace` | 配置/文件发现、UTF-8 读取、格式配置映射与受保护替换                        | gdfmt、parser、config、text、ignore、tempfile |
| `gdfmt_cli`       | clap 参数、JSON/文本输出、共享 runner、二进制入口                          | workspace、config、parser、syntax、text       |
| `gdfmt_napi`      | 同步/异步 Node 文本、配置与共享 CLI 适配                                   | gdfmt、config、workspace、CLI、napi           |
| `xtask`           | Schema 生成与漂移检测、词法、表达式与脚本 fixture 验证                     | config、parser                                |

lint、语义修复和规则插件宿主不属于 formatter 范围；napi crate 与 npm 包仅承担宿主适配，见 [Node/npm 契约](/zh/reference/node)。文件扫描和 CLI 不进入 parser 的依赖树。

## 源码与诊断

- Span 是 `[start, end)` 的 UTF-8 字节范围，使用 `u32`；构造与反序列化拒绝反向范围，切片检查 UTF-8 边界。
- `LineIndex` 拥有对应源码，避免使用另一份文档查询位置；字节列和 UTF-16 列通过不同方法提供，行列均从零开始。CRLF 作为一个物理换行。
- `TextEdit` 在完整校验所有范围与冲突后才生成新字符串。同起点插入及重叠范围被拒绝，相邻替换允许。
- 诊断只包含数据，不打印或终止进程。词法诊断包含稳定 code、category、severity、SourceId 和 span。

## 无损存储决策

采用 Rowan 0.16 的 green tree；以 `LosslessTree` 封装 green 存储，按需创建 red cursor。测试实际将 green 存储传入工作线程并重建原文。

`lex` 返回 `SourceFile` 根下的平铺 token 流；`parse_expression` 返回 `ExpressionRoot` 下的分层表达式节点。Rowan checkpoint 用于包裹已解析的左操作数，所有 trivia 和零宽 token 保持原顺序。`parse` 返回 `SourceFile` 下的声明、suite、语句和表达式。`gdfmt_syntax::ast` 提供 `Expression`、`SourceFile`、`Declaration`、`Suite` 和 `Statement` 的共享 CST 视图。

物理空白、换行、注释、BOM 均保留。逻辑 Newline、Indent、Dedent 和 Eof 是零宽 token。有效及无效输入均须满足 `tree.text() == source`；输入超限时不构建树。

输入默认限制为 16 MiB、256 层缩进或括号深度、100 条详细诊断；超过诊断数后仅增加一条抑制提示，并继续扫描以保留完整文本。词法实现不递归。parser 默认将递归和表达式高度限制为 128，用户可调低，硬上限仍为 128；词法与语法诊断共享预算，超限仍保留输入。语句、lambda 和模式的递归也使用相同限制。

脚本和表达式 parser 按需向 lexer 请求 token，不预先构建平铺词法树。进入 lambda body 前，parser 设置 suite 的括号基线；body 内新开的括号仍控制隐式续行。退出 lambda 后恢复外层模式；位于外层表达式的 lambda 同时保存并恢复缩进栈。`lex` 仍为独立词法入口，不识别 suite 语法；其缩进 token 和诊断不保证与 parser 驱动结果相同。

## 宿主与 CLI

`gdfmt_cli` 同时提供 library target 与 binary target。`run` 接收 argv、cwd、输入与输出流，返回退出码；只有二进制的 `main` 调用 `process::exit`。napi CLI 复用这个 runner。

`lex` 支持单文件、目录、多路径与单独 stdin；文件排序去重，遍历不跟随符号链接。显式文件必须为 `.gd`。目录遵守项目 gitignore，默认排除 `.git`、`.godot`、`node_modules` 和 `target`；不读取用户全局 gitignore。没有匹配文件时返回错误。

所有输入配置先验证，再读取源码。无效 UTF-8 不做有损解码。格式化使用受保护的单文件替换，check/diff 保持只读；`init` 不覆盖已有文件。预览范围见[格式化](/zh/reference/formatting)。

JSON 报告有 `schemaVersion: 1`、`spanEncoding: "utf8Bytes"`、command、status、files 和 diagnostics。文件诊断的 SourceId 对应文件报告的 sourceId。`schema` 直接输出 JSON Schema，`explain-config` 始终输出含来源信息的 JSON。

## 工程决策

- Edition 2024，MSRV 1.88，开发工具链 1.98.0；CI 单独编译 MSRV，并在 Linux、macOS、Windows 运行测试。
- 根目录提交 Cargo.lock；resolver 3 按 MSRV 选择依赖。
- 本项目采用 MIT；当前语料全部原创，未复制上游测试。Unicode XID 表从固定 Godot commit 的 `char_range.cpp` 派生，许可归属见根目录 `LICENSE-THIRD-PARTY`。
- 所有 crate 暂时 `publish = false`，crates.io/npm 名称可用性与实际账号发布权限未验收。
- P7 固定 Node 22.13+/24.x 和五个拟定 native target；产物已在本地准备，完整矩阵仍是 CI 门禁，尚未发布；动态规则插件不在范围内。

## 语法约束与版本

解析完成后，`validation` 遍历同一棵 CST，执行 annotation 绑定、abstract body 规则、声明顺序、重名、函数上下文和特性版本检查，不构建第二棵 AST。局部作用域进入时记录新增名字、退出时撤销，分支之间互不污染，lambda 重新建立参数作用域；此阶段复用词法与语法诊断预算及 SourceId。

`LanguageVersion` 与 `LanguageFeature` 位于 syntax crate，parser 和 config 使用同一类型。版本枚举和 Schema 拒绝未知版本；默认 profile 固定为 4.7。完整特性表、证据与边界见 [语言版本](/zh/reference/language-versions.md)。
