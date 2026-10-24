# 0.1.0 发布检查清单

[English](RELEASE_CHECKLIST.md)

本清单记录首个正式版本的剩余工作。当前功能基线包括 CLI、Rust 与 Node API、
WASM、stdio LSP、VS Code 桌面与 Web 扩展，以及双语文档 Playground。

审查对应证据后再勾选条目。在发布 PR 或发布说明中记录候选提交、工作流/报告链接
和产物 hash。历史验收记录不能证明新提交已通过验收。本清单不改变 CI；实际发布
与失败恢复遵循[发布指南](RELEASING.zh-CN.md)。

## 范围与公开契约

[风格/Schema 审查](docs/zh/research/0.1.0-style-schema-review.md)及两项 P2 后续工作
已完成，已审查的默认值与 Schema 契约冻结；下面的候选版本验收仍需最终发布提交的证据。

[CLI/API/版本审查](docs/zh/research/0.1.0-api-contract-review.md)也已完成，参数路由、
JavaScript 严格校验、WASM 元数据和版本同步均已补充回归。

[桌面/Web 编辑器审查](docs/zh/research/0.1.0-editor-contract-review.md)已完成。
已补充配置限制、混合换行拒绝和不支持范围格式化的回归，并对齐信任及换行说明。

[完整真实项目试用审查](docs/zh/research/0.1.0-project-trials-review.md)已在当前
候选前 alpha.6 工作区通过：606 个脚本、2,420 种有效布局、零未解释差异，606 个已审查
指纹全部未变。已修复比较器冒号分类过宽的缺口并补两个回归。这是非干净工作区
证据，下面的候选验收已记录干净 rc.1 提交的完整复跑。

- [x] 审查并冻结默认格式化风格与配置 Schema；接受输出变化前，检查注释、lambda、
      换行、字面量词形和保护区域的代表性差异。
- [x] 审查 0.1.0 的 CLI 参数/退出码、Rust 与 JavaScript API、诊断及版本同步契约，
      明确 Godot 4.3–4.7 语法配置与已知兼容性边界。
- [x] 确认桌面与 Web 编辑器行为并记录差异，包括 Web 配置限制、混合换行和不支持
      范围格式化。项目分析与语义修复仍在范围之外。
- [x] 对齐整数值小数/指数配置输入与 Schema 的接受范围，覆盖 overrides、浏览器
      配置辅助 API，并扩展回归矩阵（审查 P2 问题）。
- [x] 补全中英文行宽限制及生成 Schema 描述，覆盖表达式/调用链和保留内容
      （审查 P2 问题）。
- [x] 对当前基线运行完整固定项目试用，复查全部类别组合，修复内部类冒号分类
      过宽的问题，并在不更新指纹的情况下复跑；归档源码/报告 hash，保持现有
      默认格式化风格。

## 候选版本验收

**`0.1.0-rc.1` 已发布**到 crates.io 和 npm `next`/`candidate`，独立扩展数字版本
**`0.2.0`**，默认风格不变。[PR #1](https://github.com/ntnyq/gdfmt/pull/1) 已 squash
合并为 `1329b72dc26179921739e4c2342ffa0bf7b7a314`，tag 为 `v0.1.0-rc.1`。
[发布报告](docs/zh/research/0.1.0-rc.1-publication.md)记录精确 tag 的 CI、artifact
ID/hash、十五个 registry 包和真实 Rust/npm 安装消费者。全部 npm `latest` 保持原值。

[候选 CI 报告](docs/zh/research/0.1.0-rc.1-ci.md)与
[最初准备](docs/zh/research/0.1.0-rc.1-preparation.md)保留各自源码的历史快照。
候选发布不能替代未来正式版 0.1.0 产物验收，本次没有发布正式版或上传 Marketplace。

- [x] 准备 rc.1 源码版本及双语候选说明，审查当前改动，修复依赖版本的旧版恢复
      测试夹具，运行本机发布检查；与干净提交/完整矩阵验收分开记录。

- [x] 完整通过 [Rust CI](.github/workflows/ci.yml)：rustfmt、Clippy、Schema、
      兼容性与语法覆盖，Linux/macOS/Windows 的默认与无默认特性测试，以及
      Rust 1.88 workspace/WASM 检查。
- [x] 五种固定 Godot profile 的 parser 与 formatter conformance 全部通过。
      单独审查归类的 analyzer 边界，详见[引擎对比](CONTRIBUTING.md#engine-comparisons)。
- [x] 通过 `pnpm typecheck`、`pnpm release:check`、`pnpm test:release` 和完整
      [npm 矩阵](.github/workflows/npm.yml)。要求全部十份消费者回执与
      `pnpm verify:npm` 通过；仅 `--local` 通过不足以验收。
- [x] 通过 [WASM 与编辑器验收](.github/workflows/hosts.yml)：安装后的 WASM tarball
      在 Node/Chromium/module Worker 中运行，桌面 Extension Host、Godot Tools 共存、
      Alpine smoke check，以及无跨源隔离的 Web 生命周期与真实 Web Extension Host 测试。
      桌面 runner 补充隔离的 VSIX 安装 smoke check；staging Web 宿主测试不代表
      最终用户的浏览器安装路线已验证。
- [x] 通过 `pnpm typecheck:docs` 和 `pnpm test:docs`，包括两种语言的 Playground；
      对照选定候选提交及对应版本产物，重查安装/示例/API/编辑器文档。
- [x] 在候选提交上重跑[固定真实项目试用](tests/project-trials/README.md)的完整流程，
      不能仅运行普通的 `pnpm test:trials` 守卫测试。审查变化的差异与指纹，解决未解释
      的变化，再不带 `--record-review` 重跑，附上摘要与测试提交。这是 0.1.0 的手动
      验收项；普通 CI 中的完整试用仍为可选。

精确发布 tag 通过完整 CI、八个 crate 打包编译、六个 VSIX 目标和完整项目试用，
606 个审查条目不变、零未解释差异。原冻结 artifact `12544141250` 包含七份 npm
包、八份 crate 和六份 VSIX；十五个 registry 下载及全部 24 个 GitHub 附件均匹配
其 hash。Linux CI 和独立 macOS ARM64 真实安装消费者通过，原生/WASM 对比覆盖
1,465 个 case。最终正式 tag 的冻结及发布门槛独立保留。

## 文档与发布说明

[双语文档审查](docs/zh/research/0.1.0-documentation-review.md)已针对之前的
`0.1.0-alpha.6` 基线完成。rc.1 准备更新源码候选标识，复核示例、声明和编辑器
版本；rc.1 发布后，安装命令已改用真实发布的 rc.1 并关联 registry 验收，正式版
0.1.0 仍未发布。

已准备 0.1.0 的[英文发布说明](docs/releases/0.1.0.md)与
[简体中文发布说明](docs/zh/releases/0.1.0.md)，覆盖宿主/profile、默认风格、
`gdformat` 迁移步骤和已知限制。最终验收及发布前仍保留草稿状态，发布完成检查时
将内容整合到 GitHub Release 正文。

[Marketplace/VSIX 审查](docs/zh/research/0.1.0-editor-distribution-review.md)建议核心
0.1.0 以 GitHub 冻结 VSIX 为交付基线，Marketplace 正式发布独立跟踪。Gallery
已有数字扩展 0.1.0 的六目标预发布，已检查的 macOS/Web 清单内置 alpha.6。
rc.1 选择数字版本 0.2.0，六份冻结预发布 VSIX 已在 GitHub Release 提供，仍未上传
Gallery。上传前重新核对；如果先将 0.2.0 上传为 Gallery 预发布，正式上传需另选
数字版本。文档已明确 Web 安装及打包后的信任差异。

- [x] 修正中英文文档中“没有扩展/LSP”的描述，移除 WASM 源码构建说明的首次发布
      限定，并澄清 parser/引擎职责与表达式空白保留要求。
- [x] 对照 0.1.0 契约基线审查双语安装说明、配置示例、API 声明、编辑器设置与
      Web 限制，并记录已审查版本和验证证据。
- [x] 准备面向用户的 0.1.0 发布说明，覆盖支持的宿主/profile、格式化风格、
      从 `gdformat` 迁移的方法和已知限制。不声明完整 Godot 兼容性或运行时等价。
- [x] 记录 0.1.0 分发建议：冻结 VSIX 为交付基线，Marketplace 正式发布使用新数字
      扩展版本并独立规划，明确原生/Web 安装路线；现有工作流附加 VSIX，
      Marketplace 发布不作为核心发布门槛。

## 正式版本与发布准备

**源码准备版本为核心 0.1.0、扩展 0.2.1，尚未发布。** 本地验证与剩余关卡见
[正式版准备报告](docs/zh/research/0.1.0-preparation.md)。当前工作区尚不是冻结发布
修订，下面 rc.1 的发布回执保留各自源码身份。默认风格与 API/Schema 契约未改，
registry 安装仍使用 rc.1；本次未创建正式 tag 或上传 Marketplace。

[registry/环境审查](docs/zh/research/0.1.0-registry-readiness-review.md)确认十五个
包的现有归属及 alpha.6 历史 Trusted Publishing。2026-10-23 读取到
`RELEASE_ENABLED=true`，`release` 环境允许 `v*` tag。后续已配置 `main` 保护及
启用的 tag ruleset [25720774](https://github.com/ntnyq/gdfmt/rules/25720774)，保留
管理员/维护者例外，并通过 API 回读验证。策略记录在
[.github/repository-protection.json](.github/repository-protection.json)。
owner 随后确认配置完成，rc.1 现已通过新发布实际验证十五个 Trusted Publisher
及 npm 直接发布/dist-tag 权限，公开 registry 身份、hash 与真实消费者见
[发布报告](docs/zh/research/0.1.0-rc.1-publication.md)。[最初逐包审查](docs/zh/research/0.1.0-trusted-publisher-review.md)
保留历史私有读取/会话限制。邮箱/2FA/写权限与首次发布 token 撤销尚未独立回读，
因此保留剩余 owner 账号证据项；可选 Trusted Publishing Only 加固不新增门槛。

- [x] 完成八个 crate、七个 npm 包（含 `@gdfmt/wasm`）的只读归属、历史
      Trusted Publishing 和 `release` 环境审查，记录证据、逐项当前配置查询结果、
      账号会话限制及剩余 owner 操作。
- [x] 落实并验证 `main` 保护与 `v*` tag ruleset：PR、29 项必需检查、管理员应急
      绕过及 Admin/Maintain tag 例外，覆盖恢复 tag；记录策略及 API/角色回读证据，
      不发布或创建 ref。
- [ ] 归档剩余已认证 owner 账号证据：邮箱验证、2FA/写权限与首次发布 token 撤销。
      rc.1 已证明十五个 Trusted Publisher 及 npm 直接发布/dist-tag 权限可用，
      设置未变时无需重复配置。
- [x] 通过选定候选的新发布及真实 registry 安装验证发布配置。rc.1 运行
      `40051909889` 和发布报告记录十五个包、精确安装消费者、`next` 推进及
      全部 `latest` 保持原值。
- [x] 审查并保留固定 npm 11.21.0 运行时与 scoped dist-tag 补丁；本轮只读审查
      未满足发布指南中的移除条件。
- [x] 运行 `pnpm release:version 0.1.0 --dry-run` 并执行更新，审查 Cargo/npm
      版本、lockfile、五项原生 optional 依赖和扩展核心版本；`pnpm release:check`、
      `pnpm typecheck` 与全部 32 项发布回归通过。独立数字扩展选定 **0.2.1**，
      高于 rc.1 的 0.2.0，2026-10-24 Gallery 快照尚未占用。
      [正式版准备报告](docs/zh/research/0.1.0-preparation.md)区分本地未提交验收与
      剩余的干净源码/完整 CI 关卡。
- [ ] 提交/审查/合并正式版改动，在精确干净修订通过完整 Rust/npm/宿主 CI 矩阵
      与固定项目试用，确认提交已在默认分支历史中；按发布指南推送 `v0.1.0` 前，
      确认预期的 release 环境与 `RELEASE_ENABLED` 设置。

## 发布完成检查

以下剩余条目针对最终正式版 0.1.0。rc.1 的 tag 冻结、registry 消费者及 GitHub
预发布已在上方报告完成。部分发布失败时，保留对应版本的原冻结 bundle 用于恢复。

- [ ] 确认最终 tag 的完整 CI 矩阵与冻结 job 通过。记录成功运行/artifact ID，
      核对八个 crate 归档、七个 npm tarball、六个 VSIX 的提交、回执与 hash 一致。
- [ ] 验证 crates.io 精确版本安装、npm 原生/WASM API 与 CLI 消费者、registry
      integrity，以及全部七个 npm 包的 `latest` 渠道更新后，再将发布标记为完成。
- [ ] 确认 GitHub Release 包含发布说明与冻结产物；将双语安装说明更新为正式渠道，
      并确认文档/Playground 部署使用预期版本。

## 不阻塞 0.1.0 的后续候选工作

- 可复现的性能/内存基准与预算、持续 fuzz，以及更广的真实项目覆盖。
- 更多原生平台，或 Bun、Deno 与通用 WASI 宿主。
- 范围格式化等编辑器能力，或更完整的 Web 配置支持。
- Marketplace 正式上线：完成 publisher/认证检查、新数字版本、六目标发布/恢复及
  Gallery 安装验收；现有预发布条目与最终核心版本保持区分。

以上是后续候选方向，不是 0.1.0 的交付承诺。lint 规则、语义自动修复、项目分析和
运行时验证仍不属于 formatter 当前范围，详见[兼容性说明](docs/zh/reference/compatibility.md)。
