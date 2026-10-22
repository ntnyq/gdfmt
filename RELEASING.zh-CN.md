# 发布 gdfmt

[English](RELEASING.md)

Rust workspace 与公开 npm 包使用统一版本。推送 `v版本号` tag 后，`release.yml` 等待完整 Rust CI 与 npm 平台矩阵通过，冻结产物，依次发布 crates.io、npm，最后创建 GitHub Release。普通分支推送和 PR 只运行检查。

仓库变量 **`RELEASE_ENABLED` 必须等于 `true`** 才允许实际发布。没有配置时，tag 只触发验证和产物准备。手动运行工作流的 `publish` 默认关闭。构建、测试和冻结产物不需要 registry token。

## 发布范围与顺序

crates.io 按依赖顺序发布：

```text
gdfmt_text → gdfmt_syntax → gdfmt_parser → gdfmt_formatter
→ gdfmt → gdfmt_config → gdfmt_workspace → gdfmt_cli
```

`gdfmt_napi`、`gdfmt_wasm` 和 `xtask` 保持不可发布。安装 Rust CLI 时包名是 `gdfmt_cli`，命令名是 `gdfmt`：

```sh
cargo install gdfmt_cli --version '=VERSION' --locked
```

npm 需要发布 7 个包：

```text
@gdfmt/darwin-arm64
@gdfmt/darwin-x64
@gdfmt/linux-x64-gnu
@gdfmt/linux-x64-musl
@gdfmt/win32-x64-msvc
@gdfmt/wasm
@gdfmt/gdfmt
```

平台包先发布，主包最后发布，版本必须完全一致。预发布版本进入 `next`，正式版本进入 `latest`。所有包先以 `candidate` tag 上传；确认整套版本可查询、完整性匹配，并从 registry 安装的消费者通过 API 和 CLI 检查后，才更新发布渠道。先更新平台包渠道，再更新主包渠道。`candidate` 保留用于检查。

## 需要手动完成的平台配置

### GitHub

在 `ntnyq/gdfmt` 中：

1. 启用 GitHub Actions，允许工作流引用的 actions。平台矩阵使用 GitHub 托管 runner，包括 macOS Intel 和 arm64。
2. 创建名为 **`release`** 的 Environment，允许 `v*` tag 部署。需要全自动发布时不要设置 required reviewers；设置后会在发布前等待人工审批。
3. 保护默认分支，为 `v*` 设置 tag ruleset，限制创建、修改和删除权限。工作流还会检查 tag 指向的提交已包含在默认分支中。
4. 首次发布和 Trusted Publisher 配置完成前，不要开启发布开关。完成后，在 **Settings → Secrets and variables → Actions → Variables** 添加 **`RELEASE_ENABLED=true`**。

后续 OIDC 发布无需保存 `NPM_TOKEN`、`CARGO_REGISTRY_TOKEN` 或个人 GitHub token secret。crates.io job 自动交换短期 token，npm 直接使用 OIDC；只有创建 GitHub Release 的 job 获得 `contents: write`。

### crates.io

1. 登录并验证邮箱，确认 8 个 crate 名称可用，或已经拥有发布权限。
2. 首次发布需要 API token，通过 `cargo login` 在本机认证。创建新 crate 时，token 需要相应的新包发布权限。
3. 按下一节完成首次正式发布。
4. 在 **8 个 crate 各自的设置**中添加 GitHub Actions Trusted Publisher：

| 配置项            | 值            |
| ----------------- | ------------- |
| Repository owner  | `ntnyq`       |
| Repository        | `gdfmt`       |
| Workflow filename | `release.yml` |
| Environment       | `release`     |

确认 OIDC 成功发布新版本后，撤销首次发布 token，也可以启用 Trusted Publishing Only。发布 job 直接位于 `release.yml`，不要填写 `ci.yml` 或 `npm.yml`。

### npm

1. 验证账号和邮箱，启用双因素认证。
2. 创建或取得 **`@gdfmt` scope** 的发布权限，包括主包 **`@gdfmt/gdfmt`**。GitHub 仓库权限与 npm scope 权限相互独立。
3. 确保账号可以创建和更新全部 7 个公开包，首次通过交互式认证发布。
4. 在 **7 个包各自的设置**中添加 GitHub Actions Trusted Publisher：

| 配置项               | 值            |
| -------------------- | ------------- |
| Organization or user | `ntnyq`       |
| Repository           | `gdfmt`       |
| Workflow filename    | `release.yml` |
| Environment          | `release`     |
| Allow `npm publish`  | 开启          |
| Allow `npm dist-tag` | 开启          |

工作流名称只填写带扩展名的文件名。新配置可能默认只允许 staged publishing，本流程需要另外开启直接发布和 dist-tag 权限。新 Trusted Publisher 必须在 **两天内成功发布一个新版本**；重试时跳过已存在的版本不能完成这个验证，过期后需要重新创建配置。

发布 job 固定使用 Node 24.0.0 和 `scripts/release/npm-compat.json` 中记录的 npm 版本（目前为 11.21.0），支持 OIDC 发布及 dist-tag 操作。执行前检查和临时 dist-tag 补丁都会拒绝其他 npm 版本，避免未验证的运行时开始写入；消费者矩阵继续覆盖 Node 22.13.0 和 24.0.0。OIDC 验证成功后，可以在 npm 包设置中启用要求 2FA 并禁止 token 发布。

## 首次发布

两个平台都需要先有真实包，才能配置对应的 Trusted Publisher。首次发布由维护者明确执行，期间保持 `RELEASE_ENABLED` 关闭，也不要上传空的占位包。

1. 将发布配置和初始版本提交合并到默认分支，确保发布变更均已提交。准备发布时使用仓库固定的 Rust 工具链和 Node 24.0.0。
2. 创建并推送匹配的 tag。发布开关关闭时，只会运行验证并生成冻结产物，不会上传包。
3. 等待 **Release** 工作流的 `freeze` job 成功，记录 run ID 及该次成功冻结的 artifact 名称，下载单个完整 bundle：

```sh
gh run download RUN_ID --name release-artifacts-FREEZE_ATTEMPT --dir target/release
```

4. 使用对应 tag 的干净 checkout，验证版本并查看只读发布计划：

```sh
pnpm install --frozen-lockfile
pnpm release:check --tag v0.1.0-alpha.0
pnpm release:publish:crates
pnpm release:publish:npm
```

将示例 tag 替换为实际版本。`FREEZE_ATTEMPT` 填写成功冻结的运行次数，例如 `1`；后续只重试发布 job 不会改变该 artifact 名称。bundle 包含 `npm/`、`crates/` 和 `editors/`。`npm/release.json` 记录提交、源码干净状态、完整矩阵、待发布文件及 SHA-512 integrity；`crates/crates.json` 记录 8 个 Rust 归档对应的提交和 SHA-256，校验时还会核对归档内的 Cargo VCS 信息。每个平台选择 Node 24.0.0 job 的 tarball，Node 22.13.0 job 独立验证同一份主包；两个 Node job 生成的原生包不要求字节完全相同。

5. 确定开始首次实际发布时，使用固定的 npm 11.21.0，登录并显式执行。Node 24.0.0 最初自带的 npm 较旧，需要先升级：

```sh
npm install --global npm@11.21.0 --ignore-scripts
cargo login
npm login
pnpm release:publish:crates --execute
pnpm release:publish:npm --execute
```

**两个 `--execute` 命令执行真实 registry 写入**，要求 checkout 干净，包括已暂存和未跟踪文件。npm 操作通过当前 Node 执行 npm 的 JavaScript CLI，避免 Windows 上直接执行 `npm.cmd` 的问题；如果使用自定义 npm 安装前缀，通过 `GDFMT_NPM_CLI` 指定对应的 `npm/bin/npm-cli.js`。发布脚本按顺序补发缺失包，并校验已存在版本的 hash。Cargo 需要正常解析 crates.io；registry source replacement 可能干扰 workspace 临时 registry，遇到这类问题应使用配置干净的发布环境。

crates.io 首次创建新包允许连续发布 5 个，之后每 10 分钟恢复 1 个名额。因此这 8 个 crate 的首次发布可能需要约 30 分钟等待配额恢复。遇到 429 时按服务器给出的时间等待，再使用原冻结产物重新执行发布命令；已发布且 hash 相同的 crate 会跳过。后续更新已有 crate 使用独立的版本发布限额，见 [crates.io 发布限流](https://crates.io/docs/rate-limits)。

本机 npm 首次发布和 Trusted Publisher 设置可能分别要求浏览器二次认证；`npm login` 成功不代表后续写入无需验证。手动发布请使用交互式终端，并按 npm 提示完成认证。

6. 完成 Trusted Publisher 配置后开启 `RELEASE_ENABLED=true`，并在 npm 的两天验证窗口内通过 CI 发布下一个新版本。两个注册表的安装检查通过后，使用原冻结产物完成首次 GitHub Release。上传后不要完整重跑首次发布工作流：重建原生包可能改变字节内容。跳过已发布版本也不能验证尚未使用过的 npm Trusted Publisher。

## 后续版本更新

在包含预期修改的工作区中运行：

```sh
pnpm release:version 0.1.0-alpha.1 --dry-run
pnpm release:version 0.1.0-alpha.1
pnpm release:check
pnpm typecheck
pnpm test:release
```

版本脚本同步修改：

- `Cargo.toml` 中的 workspace 版本；
- 7 个内部 workspace 依赖的精确版本；
- `Cargo.lock` 中全部 11 个本地 workspace 包，包括私有适配层和工具；
- npm 主包版本及 5 个 optionalDependencies 版本。

脚本在修改前后执行一致性检查，后置检查失败时回滚版本文件。它拒绝降级、非法版本和 build metadata，不创建提交或 tag，也不发布。私有根 npm 工具包和文档包保留独立版本。当前 pnpm lockfile 不保存发布包版本，因此不需要修改；如果以后出现对应的锁定版本，脚本会停止，要求先更新同步策略。不要单独使用 `bumpp` 完成跨生态版本更新。

检查版本差异、按需更新发布说明，合并发布提交后推送 tag：

```sh
git tag -a v0.1.0-alpha.1 -m 'Release 0.1.0-alpha.1'
git push origin v0.1.0-alpha.1
```

正式版本可使用 `0.1.0`，自动进入 `latest`；alpha 发布不会移动 `latest`。

## CI 执行内容

`release.yml` 通过 `workflow_call` 复用 `ci.yml`、`npm.yml` 和 `hosts.yml`，明确等待它们通过，包括 Rust 测试、Clippy、rustfmt、schema、兼容性与语法覆盖检查、Rust 1.88 检查、引擎对比及 10 份 npm 消费者测试。普通 npm CI 也运行发布脚本测试。

冻结 job 重新检查 tarball hash、源码干净状态、提交来源和完整矩阵，复制 7 个 npm tarball，并通过 Cargo 临时 registry 打包、编译 8 个 Rust crate。整套产物作为一个不可变的 `release-artifacts-ATTEMPT` bundle 上传，保留 30 天；冻结 job 重试使用新的 attempt 名称，避免上传已成功而后置清理失败时出现名称冲突。发布 job 使用**成功冻结 job 返回的 artifact ID** 下载，避免按名称搜索或误选最新 attempt 的产物，不重新构建 npm 包。

Rust 发布前核对源码清单、归档内的 VCS 信息及已存在版本的 SHA-256，Cargo 重新打包的字节也必须与冻结产物一致。npm 发布前比较 SHA-512 并检查冻结的完整矩阵。网络中断、429 和 5xx 使用有次数上限的退避重试；401/403 立即失败，只有 404 才表示不存在。已存在版本字节不同则需要排查并使用新版本。上传命令报错时，会先查询 registry 是否已提交预期字节；不会自动重复写入。

Rust CLI 发布后会以精确的 `=VERSION` 要求从 crates.io 重新安装并验证版本。npm 在更新发布渠道前，通过空缓存安装精确版本，并优先获取新元数据。安装索引可能落后于不可变版本查询，因此 `ETARGET`/404 最多重试十分钟；认证及其他安装错误立即停止。安装后检查 API 输出和 CLI 版本。执行前会核对 Node/npm 版本；在 GitHub Actions 中还检查 `id-token: write` 提供的 OIDC 请求环境。不使用 `npm whoami` 验证 OIDC 权限，平台信任是否正确仍以成功上传新版本为准。两侧发布成功后才创建 GitHub Release 并附加产物。发布不会由 `workflow_run` 或 `pull_request_target` 触发。

## 演练与失败恢复

发布命令会输出开始、结束及每 15 秒一次的运行状态。npm 命令最多运行三分钟，单个 HTTP 请求超时为 30 秒，并禁用 npm 自带的 HTTP 重试，避免静默重复 registry 写入。Cargo 打包和发布命令最多运行十分钟。registry 读取会记录每次重试；npm 版本可见性和安装索引等待最多十分钟，渠道确认最多一分钟。写入超时后，仍会先核对 registry 结果，再决定停止或继续。

临时 npm 启动器在 dist-tag 读写时，将 scoped 包名中的斜杠编码规范为 `%2F`，继续使用 npm 原有的 OIDC/OTP 认证逻辑。alpha.5 恢复时实测小写 `%2f` 请求停滞、等价的大写编码请求成功，但未确定故障位于网络或 registry 的哪一层。补丁依赖 npm 私有的 `DistTag.fetchTags` 方法，因此只允许 `scripts/release/npm-compat.json` 中的精确版本；其他命令不加载该私有模块。升级时不要直接放宽版本限制。

移除条件：候选 npm 运行时在不加载此补丁的情况下，通过 scoped dist-tag 读取、现有 `release` environment 中使用 GitHub OIDC 的受控标签写入，以及全新安装后的 API/CLI 检查。写入使用可丢弃的测试渠道，并核对 registry 中的实际结果。验证通过后，在同一次修改中固定新的 `npm-compat.json` 版本、删除私有方法覆盖、更新启动器测试并记录验证证据；此前继续保留当前固定版本和补丁。

npm 版本检查优先读取安装时使用的包元数据并核对其中的 integrity。如果索引尚未列出目标版本，再检查不可变版本接口，避免将索引延迟误判为版本不存在。预检查已确认的不可变版本不会再次轮询；新上传的版本仍须等待 integrity 匹配后才能继续。

在 **Actions → Release → Run workflow** 中保持 `publish` 不勾选，可验证选定 ref 并冻结产物。尚未上传任何包时，可以手动选择版本 tag、开启 `publish` 并确保仓库开关已开启来开始发布；分支 ref 不能发布。已有上传后应恢复原运行，不要通过 dispatch 重建产物。

发生部分发布失败时，优先在原运行中选择 **Re-run failed jobs**，即使运行次数增加，也继续使用成功冻结 job 返回的 artifact ID。已存在且 hash 相同的版本会跳过，只补发缺失版本；已完成的渠道更新也会跳过，只在整套版本和消费者测试通过后补做剩余更新。上传前和更新渠道前都会检查全部 7 个渠道，禁止回退较新的平台包或主包渠道。各平台渠道确认更新成功后，才更新主包渠道。

如果恢复需要新版发布脚本，可从已提交且干净的工具 checkout 执行，并另外准备原版本 tag 的干净 checkout。下载原成功冻结的 bundle，不重建产物，先查看计划再执行：

```sh
pnpm release:publish:npm --source ../gdfmt-release-source --artifacts target/release/npm
pnpm release:publish:npm --source ../gdfmt-release-source --artifacts target/release/npm --execute
```

`--source` 只选择校验源码的位置，不会替换产物中的提交。源码的精确 Git 提交、版本、包元数据、tarball hash 和原完整矩阵仍须全部匹配。执行时两个 checkout 都必须干净，并满足前文的 npm 运行时及交互式认证要求。只有两个 registry 和安装检查均已成功后，才使用原 bundle 创建 GitHub Release。

恢复需要新版工具且要保留 GitHub Trusted Publishing 时，为已提交的工具版本创建唯一的 `v-recovery-*` tag，再在该 ref 上手动运行 `release.yml`：只设置 `publish=true` 和原冻结产物 `recovery_artifact` ID。原运行、版本 tag 和提交均从该 artifact 推导，避免手动填写不一致。恢复 tag 不触发常规发布。恢复 job 要求两个提交均位于默认分支历史中，核对指定 artifact 确实来自原版本的 Release 运行，且原冻结和 Rust 发布 job 已成功，然后按精确 ID 下载产物。新版 npm 发布脚本对照独立的原源码 checkout 校验，继续使用已有 `release` environment 和 OIDC 信任，不重建产物，也不创建 GitHub Release；两侧 registry 确认完成后，再用原 bundle 创建 Release。不要移动原版本 tag，也不要放宽 environment 策略。

一次只推送一个发布 tag。GitHub 默认并发策略会保留正在运行的发布和一个等待中的发布，额外的等待运行可能替换前一个等待运行。如果某个 tag 的等待运行被取消，在当前发布完成后显式选择该 tag 手动运行即可。

任何包上传后，都不要重建或覆盖冻结产物。重新运行全部 job 可能生成不同的原生字节，完整性冲突会阻止发布。产物过期或字节无法匹配时，准备新版本。不要强制移动已使用的版本 tag。

两个 registry 和多个 npm tag 之间没有事务。外部故障可能导致 crates.io 已完成而 npm 未完成，或平台渠道已更新而主包渠道仍是旧版本。保留原运行及产物，修复外部问题后重试，不要手动将不完整版本提升为主包发布渠道。

## 官方参考

- [Cargo 发布指南](https://doc.rust-lang.org/cargo/reference/publishing.html)
- [Cargo workspace 打包](https://doc.rust-lang.org/cargo/commands/cargo-package.html)
- [crates.io Trusted Publishing](https://crates.io/docs/trusted-publishing)
- [官方 crates.io 认证 action](https://github.com/rust-lang/crates-io-auth-action)
- [npm Trusted Publishing](https://docs.npmjs.com/trusted-publishers/)

## WASM 与 VS Code 产物

`hosts.yml` 安装真实 WASM tarball，在 Node 和 Chromium 中比较五种语言版本的共享 fixtures，并验证无需隔离响应头的 module Worker。冻结要求 `npm-wasm` 回执的提交、干净源码状态和 SHA-512 匹配。`@gdfmt/wasm` 与平台候选包一起先上传，主包最后上传，使用相同的 `next`/`latest` 渠道；需要单独配置其 npm Trusted Publisher。

同一工作流构建五个平台的 VSIX；冻结要求每个平台原生程序的版本检查，以及四个桌面平台的 Extension Host 验收。Alpine 仅进行原生程序 smoke check，因为 Electron 不支持 musl。Linux 桌面额外启用 Godot Tools 验证共存。`editors.json` 记录核心版本、数字扩展版本、提交和 SHA-512；GitHub Release 附加冻结后的 VSIX。流程不会发布到 VS Code Marketplace。

`hosts.yml` 还使用同一 WASM 包构建独立的纯浏览器 Web VSIX。冻结要求 Chromium Web Extension Host 回执包含 `webExtensionHostPassed: true` 和 `crossOriginIsolated: false`，并匹配干净的 revision 与精确 SHA-512。六个 VSIX 都作为 GitHub Release 附件，不会自动发布到 Marketplace。本地使用 `pnpm test:vscode:web` 验收；单独执行 `pnpm package:vscode:web` 不会记录验收通过。

`release:version` 同步 WASM npm 版本和插件 `gdfmtVersion`。Marketplace 的数字版本独立维护，发布插件更新前需递增，不能使用 Cargo 的预发布后缀。`gdfmt_wasm` 不参与 crates.io 打包。恢复旧版六包冻结产物时，仍以原始源码 checkout 的发布契约校验，不强制补建 WASM。
