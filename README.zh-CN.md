# gdfmt

<p align="center">
  <img src="docs/public/logo.svg" width="160" height="160" alt="gdfmt logo：Godot 图标与缩进格式化符号" />
</p>

[English](README.md) | 简体中文

以 Rust 为核心的 GDScript 格式化器，使用无损解析、语法诊断和布局配置保障格式化，并提供本地准备的 npm 产物。

当前已实现 Cargo workspace、源码位置与诊断、配置与 JSON Schema、无损词法分析、Pratt 表达式、声明与语句解析、abstract 与版本 gates、固定引擎一致性 harness 和 CLI 基础。P3 语法已按明确排除项验收，P4 完整脚本格式化和验证后的 CLI 已可用，源码构建 alpha 已按固定项目试用排除项验收；P5 formatter-off/on 保护区域已实现。Node/npm API 与本地打包已实现，跨平台产物验收仍由 CI 门禁验证；lint 和语义修复不在范围内，crate 暂不发布。

## 开发

Rust edition 2024，MSRV 为 Rust 1.88；开发工具链固定在 `rust-toolchain.toml`。

```sh
cargo test --workspace --all-features --locked
cargo test --workspace --no-default-features --locked
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo xtask schema --check
cargo xtask compatibility
cargo xtask syntax-coverage
```

## 本地 npm 产物

使用 Node 22.x 的 22.13+ 或 Node 24.x：

```sh
pnpm install --frozen-lockfile
pnpm typecheck
pnpm test:npm
pnpm verify:npm --local
```

脚本使用 TypeScript/tsx；Vitest TypeScript 测试安装并运行实际 tarball，检查 ESM/CJS 声明和 CLI 一致性。[Node/npm 契约](docs/zh/reference/node.md)记录 API、拟定平台和发布恢复流程。尚未发布，完整跨平台验收待 CI。

## 当前可运行命令

```sh
cargo run -p gdfmt_cli -- --help
cargo run -p gdfmt_cli -- check . --syntax-only
cargo run -p gdfmt_cli -- format src
cargo run -p gdfmt_cli -- lex tests/compatibility/fixtures/containers.gd
cargo run -p gdfmt_cli -- lex tests/compatibility/fixtures --reporter json
cargo run -p gdfmt_cli -- lex - --reporter json < script.gd
cargo run -p gdfmt_cli -- schema
cargo run -p gdfmt_cli -- init
cargo run -p gdfmt_cli -- explain-config .
```

`lex` 只读检查词法并输出 token；退出码为 `0`（未发现词法错误）、`1`（发现词法错误）、`2`（参数、配置或 I/O 错误）。JSON 中的 span 使用 UTF-8 字节范围 `[start, end)`，虚拟 token 不占字节。

**词法通过不代表 GDScript 语法正确。** `check --syntax-only` 运行 parser 检查；`format` 与默认命令格式化脚本，包含注释、lambda、声明和 suite，`check` 与 `--diff` 只读。使用前请阅读 [P4 预览范围](docs/zh/reference/formatting.md)。

配置文件为 `.gdfmtrc.json`，编辑器 Schema 位于 [schemas/gdfmtrc.schema.json](schemas/gdfmtrc.schema.json)。当前配置范围与合并规则见 [配置说明](docs/zh/guide/configuration.md)。

## 文档站点

[英文文档](docs/index.md)与[中文文档](docs/zh/index.md)使用 VitePress 构建，默认英文，可通过语言菜单切换到同一页面的中文版本。

使用 Node.js 22 或更高版本与 pnpm：

```sh
cd docs
pnpm install --frozen-lockfile
pnpm dev
pnpm build
pnpm preview
```

英文页面位于 `docs/`，中文页面位于 `docs/zh/`，按 `guide/`、`reference/`、`development/` 分类，并保持相同相对路径。首页使用 `index.md`，配置和主题位于 `.vitepress/`，共享资源位于 `public/`。子路径部署可使用 `pnpm build --base /gdfmt/` 与 `pnpm preview --base /gdfmt/`。

## 实现进度

| 阶段  | 当前状态                                                                                                                                                                                                            |
| ----- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| P0    | 已固定稳定参考 commit，建立原始词法用例与 Rowan 原型；语法覆盖表已验收；名称及发布权限核验待完成                                                                                                                    |
| P1    | workspace、基础契约、配置模型、CLI 外壳和 CI 已实现                                                                                                                                                                 |
| P2    | 已对齐固定 Unicode XID 表、字符串与转义、带符号数值、特殊 token 和跨空行续行、parser 驱动的 lambda suite 上下文；引擎差异验证与长期 fuzz 待完成                                                                     |
| P3    | 表达式、声明、语句、lambda、属性、match、分层无损 CST、abstract、声明约束、版本 gates 与初步 typed facade 已实现；262 个脚本已通过五个固定引擎对照；[语法审计](docs/zh/development/syntax-audit.md)按明确排除项验收 |
| P4    | Doc 打印器、注释及 lambda/声明/suite 布局、验证 facade 与最小 CLI 已实现；alpha 已按固定项目试用排除项验收                                                                                                          |
| P5    | formatter-off/on 保护区域、精确字节保留和受验证的缩进回退已实现                                                                                                                                                     |
| P6    | 已实现本地继承、有序 overrides、EditorConfig、忽略规则和受保护 CLI 写入；保留跨平台 CI 门禁                                                                                                                         |
| P7    | 同步/异步 Node API、ESM/CJS 与 CLI 打包已实现，完整产物矩阵待 CI                                                                                                                                                    |
| P8–P9 | formatter 集成与稳定性待推进                                                                                                                                                                                        |

Rust library 的单表达式入口是 `gdfmt_parser::parse_expression(source, &ParseOptions::default())`，用法、优先级和支持边界见 [表达式解析](docs/zh/reference/expressions.md)。该入口不接受完整 `.gd` 文件。

完整脚本入口为 `gdfmt_parser::parse(source, &ParseOptions::default())`，返回 `SourceFile` 分层 CST。语法范围、上下文与恢复边界见 [脚本解析](docs/zh/reference/parsing.md)。

版本配置与特性边界见 [语言版本](docs/zh/reference/language-versions.md)。固定 Godot 4.3–4.7 矩阵的对照命令与验证范围见 [引擎一致性测试](docs/zh/development/engine-conformance.md)。

P6 配置与文件/CLI 行为已实现，见[配置说明](docs/zh/guide/configuration.md)和[验收记录](docs/zh/development/formatter-acceptance.md)。Node/npm 产物已实现但未发布，见 [Node 契约](docs/zh/reference/node.md)；继续扩充项目试用和 formatter fuzz。[P3 验收记录](docs/zh/development/syntax-audit.md)列出支持范围和排除项。

- [详细实施计划](docs/zh/development/implementation-plan.md)
- [架构与基础契约](docs/zh/development/index.md)
- [兼容性与待覆盖范围](docs/zh/development/compatibility.md)

MIT License。当前 fixtures 为本项目原创；Unicode XID 表派生自固定 Godot commit，归属与许可见 [LICENSE-THIRD-PARTY](LICENSE-THIRD-PARTY)。

Logo 将 Andrea Calabró 创作的 Godot Engine 图标与代码括号、缩进线结合，遵循 [CC BY 4.0](docs/public/logo.LICENSE.txt)。gdfmt 为独立项目，未获 Godot Engine 项目背书。
