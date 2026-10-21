# P4 布局验收记录

核验日期：2026-10-21，平台 macOS。P4 formatter alpha 已按下述范围与排除项完成验收，覆盖脚本布局、不变量、五个固定引擎、许可明确的项目试用与 gdtoolkit 差异审查。本记录验收源码构建，尚未发布软件包。

## 范围与证据

| 范围         | 布局契约                                                                                       | 证据                                                                   |
| ------------ | ---------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------- |
| 注释         | 前导与行尾 token 锚点、分隔符内悬空注释、suite/属性/match 末尾归属；保留原文、顺序与独立行角色 | `formatter-comments.gd`、token 边界插入测试、附着/归属变动的反例       |
| Lambda       | 单条简单且无注释的函数体行内显示，其余缩进；具名、嵌套、类型/默认参数、列表/字典/调用          | `formatter-lambdas.gd` 及现有 lambda/default/loop/return fixtures      |
| 声明         | 头部、继承、显式/推断类型变量与常量、enum、signal、函数、参数、注解、内部类与属性              | `formatter-declarations.gd` 及 4.3–4.7 特性门禁                        |
| Suite 与语句 | 类/函数/访问器、if/elif/else、for/while、match 分支/模式/guard、赋值/return/assert/控制语句    | `formatter-suites.gd` 及全部 parser 可接受语料                         |
| 字面量安全   | 数字/字符串/raw/StringName/NodePath 词形、多行内容和 BOM 原样保留，不改括号/逗号/分号          | `formatter-multiline-literals.gd`、独立表达式、BOM/CRLF 与一元符号回归 |
| 整文件       | 配置化缩进与布局换行、限制已有空行、移除首尾空行、控制最终换行                                 | formatter 配置矩阵和 CLI stdin/文件写入回归                            |

五个原创布局脚本及两个项目试用回归均登记在两个兼容性 manifest。`crates/gdfmt_formatter/tests/snapshots/` 保存 40/100 宽度下共十份已检查快照。544 个 parser 接受的脚本/版本组合分别使用四组布局：tab/40/preserve、tab/100/preserve、两空格/40/CRLF/无空行/无最终换行、四空格/100/LF/一空行/最终换行，共 **2176 组重解析、结构/注释等价与幂等性检查**。其中包含 22 个 parser 接受的 analyzer 边界，它们不计入引擎接受数。无效输入继续禁止产生可写输出。

注释比较不依赖 Rowan 偶然的 trivia 存储位置：先比较有效节点拓扑与 token 词形，再比较注释原文、相邻有效 token 位置、独立行/行尾角色以及悬空块归属。反例测试拒绝注释迁移到别的元素或函数体外。Doc 行尾后缀会在后续代码前建立换行边界，闭合分隔符后的注释也参与宽度计算。

## 五个固定引擎

精确 tag、commit 和二进制验证继续沿用[引擎一致性测试](/zh/development/engine-conformance)。原脚本按已登记分类核对引擎结果；格式化后的有效脚本独立以宽度 40 检查。

| Profile | 原脚本检查 | 格式化有效输出 |  失败 |
| ------- | ---------: | -------------: | ----: |
| 4.3     |        262 |             90 |     0 |
| 4.4     |        262 |             98 |     0 |
| 4.5     |        262 |            110 |     0 |
| 4.6     |        262 |            112 |     0 |
| 4.7     |        262 |            112 |     0 |
| 合计    |   **1310** |        **522** | **0** |

报告位于 `target/conformance/report-<profile>.json` 与 `target/conformance/formatter-<profile>.json`，本地汇总为 `p4-summary.json`，包含源码/输出和引擎诊断。所有引擎比较均为 check-only，不执行脚本，不构成语义或运行时等价证明。

首轮引擎验证发现了 Rust 重解析未发现的限制：Godot 拒绝在集合类型的 `[` 后立即换行。类型引用现保持不可拆分，并补充专门回归后完整重跑引擎。块 lambda 也不能退缩进到任意容器续行列，布局因此使用外层语句的缩进栈。

## 固定版本与明确许可的项目试用

`tests/project-trials/manifest.json` 登记两个不可变 commit 的全部 `.gd` 文件及适用许可原文，为每个文件固定 SHA-256。第三方源码只保存在忽略的 `target/project-trials/sources/`，保留许可与行内版权声明。本次仅试用源码，不下载场景和素材，不导入项目，不执行项目代码。

| 来源                                                                                                                        | 固定版本与许可                                                                                            |    脚本 |    接受 | 已知源码拒绝 | 有效布局 |
| --------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------- | ------: | ------: | -----------: | -------: |
| [Godot 官方 demos](https://github.com/godotengine/godot-demo-projects/tree/6ad6167e0577fe3622c18546138f456b107ce93c)        | `4.7-6ad6167`，commit `6ad6167e0577fe3622c18546138f456b107ce93c`；根目录 MIT 与 Dodge the Creeps MIT 声明 |     456 |     455 |            1 |     1820 |
| [Kenney 3D Platformer](https://github.com/KenneyNL/Starter-Kit-3D-Platformer/tree/3fa8a04b1c01ab23db43123d4ce814a34c3fc7f0) | commit `3fa8a04b1c01ab23db43123d4ce814a34c3fc7f0`；MIT                                                    |       9 |       9 |            0 |       36 |
| 合计                                                                                                                        | 986362 字节源码；131 个官方 demo 目录与一个 starter kit                                                   | **465** | **464** |        **1** | **1856** |

464 个可接受脚本在语言 profile 4.7 的四组布局下全部通过 facade 门禁，包含 CRLF、空格缩进、空行限制和无末尾换行。没有意外拒绝或不变量失败。已知无效源码同样运行四组配置，均无可写输出，不计入有效布局数。

唯一源码拒绝是 `godot-demos/2d/tween/main.gd`：lambda 退缩进到不存在于缩进栈的续行列。精确固定的 Godot 4.7.2 对原文件第 77 行报告 `Unindent doesn't match the previous indentation level.`；gdfmt 报告 `LEX005`，gdtoolkit 4.5.0 则接受并重排。这是明确的源码排除项，不算格式化成功，也不豁免 formatter 失败。试用工具每次都重新核对原文件的引擎诊断。

试用发现并修复两处 parser 问题：`void` 函数可返回调用表达式，包括带分组括号的调用，构造函数仍拒绝返回值；块 lambda 的末条语句可直接遇到外层 `)` 或 `,`，无需先出现物理退缩进。`formatter-project-regressions.gd`、`project-invalid-lambda-dedent.gd` 及专门 parser 测试覆盖五个 profile。引擎门禁同时核对允许的 lambda 写法和不允许的退缩进。

## gdtoolkit 格式差异审查

比较版本固定为 **gdtoolkit 4.5.0**，宽度 100、tab 缩进。`tests/project-trials/requirements.txt` 固定全部十个 Python 依赖的版本与哈希，实际解释器为 Python 3.12.14。**63 份输出逐字节一致，401 份存在差异**，其中官方 demos 为 62/393，Kenney 为 1/8。**无未解释差异**。`tests/project-trials/reviewed.json` 记录每对输出的 SHA-256 与审查分类，也包含被排除源码的 gdtoolkit 输出。

| 已审查差异                          | 决策与证据                                                                                                                                                                               |
| ----------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 布局：全部 401 个有差异的可接受脚本 | 限制并保留源码已有空行，不强制采用 gdtoolkit 的函数间距；注解独立成行，行尾注释前一个空格，展开 suite，按宽度布局列表与 enum。不新增括号或反斜杠，因此没有现成分组的二元表达式可能超宽。 |
| 括号、逗号、分号：110 个脚本        | 保留原 token；gdtoolkit 可能增删分组、修改末尾逗号、删除分号并重排调用链，示例见 `networking/webrtc_signaling/server/ws_webrtc_server.gd`。                                              |
| 引号：3 个脚本                      | 保留原字面量字节；已审查 `3d/global_illumination/test.gd`、`gui/regex/regex.gd`、`misc/os_test/os_test.gd` 的引号与转义差异。                                                            |
| 内层类继承：2 个脚本                | 保留 `class Name extends Base:`；gdtoolkit 会在 `misc/custom_logging/custom_logger_ui.gd` 和 `networking/webrtc_signaling/server/ws_webrtc_server.gd` 中将 `extends` 移入类体。          |
| 源码拒绝：1 个脚本                  | tween 原文件无效，保持拒绝与无输出，即使 gdtoolkit 接受。                                                                                                                                |

分类计数可重叠。分类工具会拒绝不能解释的 token 差异及注释原文/顺序变化；输出指纹也会阻止未经重新审查的布局变化。token/引号归一化仅辅助审查，独立的 Rust 门禁仍将每份 gdfmt 输出与原 CST、有效 token/字面量词形及注释锚点比较，不据此证明 gdtoolkit 的变换等价。

复现及更新审查记录的方法见 `tests/project-trials/README.md`。正常重跑只核对已提交指纹，不更新预期。`target/project-trials/` 保存 `layouts.json`、`comparisons.json`、逐文件 unified diff 和包含原源码引擎日志的 `summary.json`。下载需显式启用且校验哈希；缓存源码遭修改或多出脚本都会失败，不就地格式化原项目文件。

## 验证与后续工作

workspace 两种特性配置均为 94 项测试通过；唯一忽略项是由父测试启动的变异测试 worker。Rust 格式、Clippy、schema 漂移、兼容性、语法覆盖、Rust 1.88 与文档构建均通过。另外三个可选 Python 测试覆盖缓存篡改、未知代码/注释变化以及预期风格分类。

```sh
cargo test --workspace --all-features --locked
cargo test --workspace --no-default-features --locked
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo xtask schema --check
cargo xtask compatibility
cargo xtask syntax-coverage
cargo xtask conformance --language <profile> --godot /path/to/pinned/godot --report target/conformance/report-<profile>.json
cargo xtask formatter-conformance --language <profile> --godot /path/to/pinned/godot --report target/conformance/formatter-<profile>.json
```

继续保留 Rust 1.88 全 target/feature 检查和双语文档构建。workspace 测试仍运行 P3 的有界 token 变异检查，它不能替代长期 formatter fuzz。

P4 源码构建的 alpha 验收门禁已按上述排除项满足；发布是独立操作，本轮没有发布。长期 formatter fuzz、更广泛项目及操作系统试用仍为后续工作。P5 formatter-off/on 区域已实现并按下文验证；P6 配置与文件/CLI 验收记录见下文；不声明完整 Godot 兼容、项目导入成功、语义/运行时等价、字面量值解码或跨平台引擎验收。

## P5 Formatter 范围验收（2026-10-21）

产品已聚焦格式化。P5 不再包含 lint、规则禁用或语义修复，未保留 linter crate/配置及 fix CLI；P6 继续完善生产布局配置和文件处理。[修订路线图](/zh/development/implementation-plan)定义后续 formatter 里程碑。

已实现成对的独立行 `# gdfmt: formatter-off` / `# gdfmt: formatter-on` 区域。facade 除重解析、结构/注释等价和幂等门禁外，还独立比较保护字节，全部通过才允许输出。任何位置的语法错误仍阻止输出；未配对/嵌套标记产生 `FMT002`，CLI 返回 1。精确恢复与规范缩进或表达式布局冲突时，保守保留相关顶层单元并再次验证。

| 证据                                           | 覆盖范围                                                                                                                                                                           |
| ---------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `crates/gdfmt/tests/regions.rs`：9 个回归      | 五种 profile、LF/CRLF/preserve、区域精确字节、Unicode/BOM、EOF/最终换行、多段/空/纯注释区域、字符串/行尾伪标记、嵌套 suite、lambda、访问器、match、表达式/声明边界、缩进回退和幂等 |
| `crates/gdfmt_cli/tests/cli.rs`：新增 3 个回归 | 文件/check/diff/stdin 一致；错误标记与区域内语法错误不写入；syntax-only 不解释布局指令；无 lint/fix 命令，不改写语句或名字                                                         |
| `crates/gdfmt_config/tests/config.rs`          | 运行时/Schema 均拒绝 lint、rules 和 plugin 字段                                                                                                                                    |

```sh
cargo test -p gdfmt --test regions --locked
cargo test -p gdfmt_cli --test cli --locked
cargo test -p gdfmt_config --test config --locked
```

上文 P4 证据保留为对应阶段的验收快照；P5 复跑相同门禁，结果记录如下。

2026-10-21 验证完成：workspace 两种特性配置均为 **106 项测试通过**，mutation worker 仍由父测试启动并单独标记忽略。五个固定 macOS 引擎的 **1310 次 parser 比较、522 份 formatter 输出**全部通过；另有 **20 份保护区域输出**通过五种 profile 的引擎 check-only 和精确字节检查。既有 **2176 组语料布局**与 **1856 组固定项目布局**继续通过；一个已知上游无效源码仍拒绝，401 份 gdtoolkit 差异指纹仍与既有审查一致。

Rustfmt、Clippy、Schema 漂移、兼容性、语法覆盖、Rust 1.88 全 target/feature 检查和双语文档构建均通过。生成 Schema 已排除通用格式化，由生成器维护其权威格式。本地汇总位于 `target/conformance/p5-summary.json` 与 `p5-regions.json`，并保留既有引擎和项目试用报告。P5 按修订后的 formatter 范围验收通过，不宣称完成 lint/语义修复；P4 排除项继续适用。

## P6 配置与文件/CLI 验收（2026-10-21） {#p6-configuration-acceptance}

P6 已实现本地 JSON 继承、以声明目录为基准的 overrides、EditorConfig 和一致的输入忽略规则，继续保持 formatter 范围。精确优先级与选择契约见[配置说明](/zh/guide/configuration)：EditorConfig 低于 JSON，全部基础字段之后再应用追加 overrides，普通数组替换，全部合并后补默认值。`explain-config` 展示继承 JSON 顺序、EditorConfig 文件、字段/override/属性最终来源及忽略状态，也支持 stdin 虚拟路径。

| 证据                                        | 覆盖范围                                                                                                                                                                                                                                |
| ------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `crates/gdfmt_config/tests/config.rs`       | 继承/override/ignore 结构的同组运行时/Schema 矩阵，null/未知字段/类型/范围拒绝，false/zero/空数组合并；继续拒绝 lint/rules/plugins                                                                                                      |
| `crates/gdfmt_workspace/tests/workspace.rs` | 相对多父继承、共享父配置重放、真实路径循环、缺失输入、深度/层数/总字节预算、有序 include/exclude glob、root/显式配置、保留文件名的虚拟路径、EditorConfig section/花括号/区间/unset/回退/来源、ignore 优先级/反向规则/显式输入和读取限额 |
| `crates/gdfmt_workspace/src/writing.rs`     | 临时创建、写入、保留权限、同步与替换前故障注入；外部编辑/删除/同字节替换/chmod/符号链接交换；子进程中断及后续恢复                                                                                                                       |
| `crates/gdfmt_cli/tests/cli.rs`             | 文件/stdin/explain 一致、显式和 stdin 忽略、空发现、排序去重后的 source ID、JSON/人类错误、退出码优先级、配置/UTF-8/大小预检失败、模式冲突和流 I/O 错误                                                                                 |

写入保护新增 Unix 硬链接拒绝，除原始字节外也复核元数据/文件身份。普通失败保留原文并清理临时文件；准备阶段中断保留原文，可能遗留临时文件。最后检查到 rename 的竞争检测是尽力复核，原子性以单文件为单位，不承诺多文件事务或掉电持久性。Git worktree 标记文件不会破坏本地 `.gitignore`、`.ignore`、`.gdfmtignore`，但尚不解析外部 Git common-directory excludes。

保留现有 Linux/macOS/Windows workspace CI 矩阵作为门禁。本地验收仅在 macOS，不宣称已实测 Windows/Linux 或实现更强的平台锁定/恢复。Parser/formatter 不增加文件系统或 EditorConfig 依赖，也未引入 lint/语义改写流程。P4 源码/项目排除项和 P5 保护字节契约继续适用。

本地 Rust/宿主验证：workspace 两种特性配置均为 **131 项测试通过**，另有 **2 个由父测试启动的忽略 worker**（mutation 与写入中断），比 P5 增加 25 项通过回归。Rustfmt、Clippy、生成 Schema 漂移、兼容性（292 词法 / 16 表达式 / 262 脚本 fixtures，1310 次 profile 检查）、语法覆盖（50 行清单）、Rust 1.88 全 target/feature 检查和双语文档构建均通过。固定项目试用及 3 项 harness 回归再次通过：**1856 组有效布局**、一个预期上游源码拒绝，401 份已审查 gdtoolkit 差异中 **零未解释差异**。

五个固定 macOS 引擎的 **1310 次源码对照、522 份格式化输出**全部通过；另有 **20 份保护区域输出**通过引擎 check-only 与精确字节保留。本地报告位于 `target/conformance/p6-summary.json`、`p6-report-<profile>.json`、`p6-formatter-<profile>.json`、`p6-regions.json`，项目证据仍在 `target/project-trials/`。P6 按上述本地源码构建范围与明确限制验收；跨平台执行继续作为 CI 门禁，不记为本地已完成。
