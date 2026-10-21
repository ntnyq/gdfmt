# Formatter 实施计划

gdfmt 是以 Rust 为核心的 GDScript formatter。解析器、诊断、配置和文件处理服务于安全、可重复的格式化，并通过 Rust library、CLI 和本地准备的 npm 产物提供同一能力。

2026-10-21 已收窄产品范围，本文替代此前的工具链计划：内置 lint、规则注册、lint 禁用指令、unused 分析、命名/函数长度检查、语义自动修复和动态规则插件均不在范围内，也不是发布门槛。已有 parser 诊断继续保留，因为格式化必须建立在有效语法树上。

## 产品契约

- `gdfmt [paths...]` 和 `gdfmt format [paths...]` 格式化有效 GDScript，默认处理当前目录。
- `gdfmt check [paths...]` 只读检查语法和格式差异，不运行 lint。
- `check --syntax-only` 只输出 parser 诊断，不格式化。
- `--diff` 只读显示格式差异；stdin 和 JSON 使用同一核心。
- 成对的 `# gdfmt: formatter-off` / `# gdfmt: formatter-on` 注释逐字节保留原文，但整个文件仍须语法有效。
- `.gdfmtrc.json`、生成的 JSON Schema 和可解释的配置来源控制布局。
- Rust library、独立 CLI 与 Node 包必须给出相同格式化结果。

格式化保留有效 token、字面量词形和注释关联。不做重命名、声明排序、类型插入、语句删除等语义改写。格式化本身就是布局转换，不再设计单独的 lint `--fix` 工作流。

通过 `check` 不代表 Godot 项目编译或运行正确。跨文件继承、资源、场景、autoload 和引擎重载解析不属于范围；Godot 3、完整类型检查器、LSP 和代码生成也不在范围内。

## 架构

| 模块              | 职责                                                      |
| ----------------- | --------------------------------------------------------- |
| `gdfmt_text`      | 源码标识、字节 Span、位置、诊断和经验证的文本编辑         |
| `gdfmt_syntax`    | 无损 CST、版本 profile、typed views 和 formatter 注释区域 |
| `gdfmt_parser`    | 词法、解析、恢复和必要的结构约束                          |
| `gdfmt_formatter` | 注释附着、Doc 布局、打印和保护区域恢复                    |
| `gdfmt`           | 纯文本 facade，执行重解析、结构等价和幂等门禁             |
| `gdfmt_config`    | 严格的 Partial/Resolved 布局配置与生成 Schema             |
| `gdfmt_workspace` | 文件/配置发现、配置映射与受保护写入                       |
| `gdfmt_cli`       | 参数、reporter、共享 runner 和可执行入口                  |
| `gdfmt_napi`      | Node-API 文本/配置 worker 与共享 CLI 适配                 |
| `xtask`           | Schema、语法、兼容性、引擎和项目试用验证                  |

Node、文件扫描和终端输出不得进入 parser/formatter 依赖。napi crate 和 npm 包仅承担宿主适配；不再规划 linter 或规则插件宿主。

公共格式化入口仍为 `gdfmt::format(source, &FormatRequest) -> FormatResult`。`output: None` 表示宿主不得写入。Span 使用半开区间 UTF-8 字节偏移；未来 JS 绑定需要 UTF-16 位置时必须显式转换。

## 正确性与文件行为

1. 无损解析精确重建有效和无效输入。
2. 无效/不支持的语法及恢复节点不得产生可写输出。
3. 输出使用相同语言版本、限制和源码标识重解析。
4. 规范化 CST 结构、有效 token 原文和注释关联一致。
5. 再次格式化得到完全相同的字节。
6. 保护区域保留原始字节，包括标记行和换行形式。

facade 在写入前验证每个候选输出。如果恢复保护区域与外部缩进修改冲突，保守保留相关顶层单元并再次验证。公开契约见[保护区域](/zh/reference/formatting#保护区域)。

退出码保持：0 成功，1 源码错误或格式差异，2 参数/配置/I/O/内部失败；多文件取 `2 > 1 > 0`。格式化指令不能屏蔽 parser 错误。错误标记配对产生源码诊断，不允许成功输出部分文件。

使用同目录临时文件、保留权限，并在替换前检测外部改动；原子性以单文件为单位。所有配置和输入读取在写入前预检。stdout 只包含请求的源码、diff 或 JSON。

## 配置方向

当前字段见[配置说明](/zh/guide/configuration)，Rust serde 模型和生成 Schema 是唯一事实来源。拒绝未知字段和显式 null；缺失字段沿用继承值，false 和 zero 保留原义。

P6 已实现本地 JSON `extends`、有序目录 overrides、`.gdfmtignore`、EditorConfig 和来源记录。路径以声明它的配置文件为基准，检测循环并限制继承深度；对象逐字段合并，普通数组替换，overrides 明确采用追加例外。全部合并结束后才填充默认值。EditorConfig 只报告实际支持并生效的选项。

不添加 `lint`、`rules` 或规则插件配置。普通格式化不需要 Godot 或网络。

## 阶段与验收

保留阶段编号以维持既有验收链接和引擎证据；P5、P8 调整为 formatter 范围。

| 阶段 | 范围                                      | 状态 / 验收                                                                                              |
| ---- | ----------------------------------------- | -------------------------------------------------------------------------------------------------------- |
| P0   | 参考版本、源码契约和原创 fixtures         | 语法清单已验收；发布名称与权限尚未核验                                                                   |
| P1   | workspace、文本 Span、配置、CLI 外壳和 CI | 已实现                                                                                                   |
| P2   | 无损 lexer 和有界语法存储                 | 已实现；长期 fuzz 持续推进                                                                               |
| P3   | 脚本 parser、结构约束和语言 profile       | [按明确排除项验收](/zh/development/syntax-audit)                                                         |
| P4   | 可靠脚本 formatter 和验证 facade/CLI      | [源码构建 alpha 按项目试用排除项验收](/zh/development/formatter-acceptance)                              |
| P5   | formatter-off/on 保护区域                 | 已验证精确字节、标记错误、语法拒绝、缩进回退和幂等；见验收记录                                           |
| P6   | 生产布局配置与文件/CLI 处理               | [本地实现验收与跨平台 CI 门禁](/zh/development/formatter-acceptance#p6-configuration-acceptance)         |
| P7   | napi 与 npm 分发                          | [已实现；本地打包验收，全平台/运行时 CI 待执行](/zh/development/formatter-acceptance#p7-node-acceptance) |
| P8   | Formatter 集成与性能                      | 编辑器/CI 配方、项目语料扩充、长期 fuzz、实测吞吐/启动/内存                                              |
| P9   | 稳定 formatter 发布                       | 完成兼容性、产物、恢复和文档门禁                                                                         |

### P5：保护区域

仅识别独立行的真实注释 token，要求标记成对，不允许嵌套；字符串和行尾注释内的相似文字不生效。区域内容及标记行逐字节保留，优先于换行、空行和最终换行选项。缩进或表达式边界存在冲突时，保守保留相关顶层单元。文件任何位置的语法错误仍阻止输出。

验收覆盖多区域、空/纯注释区域、EOF、BOM、Unicode、CRLF/混合换行、嵌套 suite、跨声明/表达式边界、所有语言 profile、CLI 只读行为和失败时文件不变。旧计划中的 lint/fix 交付项已移除，不记为实现完成。

### P6：配置与生产 CLI

本地 `extends`、追加 overrides、EditorConfig 映射和本地/JSON ignore 已实现，优先级与声明目录路径语义已有明确契约。运行时/Schema 矩阵覆盖 false/zero/空数组及错误结构；workspace/CLI 回归覆盖显式忽略、空发现、稳定 ID、虚拟 stdin 路径、reporter、预检失败、权限、并发编辑和准备阶段中断。同一批测试继续进入 Linux/macOS/Windows CI 矩阵；本机 macOS 验收不代表 Windows/Linux 已执行。尽力检测竞争和强制终止遗留临时文件的边界见[配置说明](/zh/guide/configuration)。

### P7：Node 与 npm

已通过 Node-API 4 实现同步/异步文本 API 和显式配置 API，CPU 计算进入 libuv worker。ESM/CJS 共享 facade，诊断明确标注 UTF-8 字节偏移，CLI shim 直接调用现有 Rust runner。仓库脚本使用 TypeScript/tsx，已安装包测试使用 Vitest TypeScript。已实现精确版本的平台 optional 包、manifest/native target 校验、Schema/声明导出、无 Rust 的干净安装与产物回执。五个平台、两个固定 Node 运行时、本地命令及部分发布恢复流程见 [Node/npm 契约](/zh/reference/node)。本地 macOS 验收不等于十组平台/运行时矩阵完成。尚未发布，发布权限也未核验。

### P8–P9：Formatter 稳定性

扩充许可明确、固定 commit 的真实项目并审查格式差异，保留固定引擎的语法/格式化对照。持续 fuzz parser/formatter/config 并最小化失败用例。先测量再优化，将核心处理与进程启动分开，不承诺未经重复验证的性能提升。语法覆盖、输出不变量、写入故障恢复、平台安装和文档与实际产物一致后，才冻结 1.0 布局/API 契约。

## 验证门禁

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --all-features --locked
cargo test --workspace --no-default-features --locked
cargo xtask schema --check
cargo xtask compatibility
cargo xtask syntax-coverage
cargo +1.88.0 check --workspace --all-targets --all-features --locked
```

另运行 `pnpm typecheck`、`pnpm test:npm`、`pnpm verify:npm --local`；CI 要求完整 `pnpm verify:npm` 矩阵。在 `docs/` 运行 `pnpm build` 构建 VitePress。对[引擎一致性说明](/zh/development/engine-conformance)中的每个精确 pin 运行两条对照命令，并复现[固定项目试用](https://github.com/ntnyq/gdfmt/tree/main/tests/project-trials)。普通 Rust 测试不需要 Godot。仅有本地源码构建通过，不能宣称其他操作系统或 npm 产物已验证。

## 发布清单

- [ ] 所有声明支持的语法均有正反例、版本和格式化证据。
- [ ] 有效语料保留 token、字面量、注释和保护区域字节，无未解释差异。
- [ ] 无效输入不产生可写的部分输出。
- [ ] 布局配置、Schema、help 和宿主 API 一致。
- [ ] 文件故障测试证明无截断，退出码与 JSON 契约稳定。
- [ ] 所有声明平台的 Rust、独立 CLI 和 npm 结果一致。
- [ ] 验证最终产物安装、精确依赖和发布恢复。
- [ ] 明确记录限制、布局变更和升级方式。
