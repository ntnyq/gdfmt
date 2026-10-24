# 发布 gdfmt

[English](RELEASING.md)

Rust workspace 与公开 npm 包使用统一版本。推送 `v版本号` tag 后，`release.yml` 等待完整 Rust CI 与 npm 平台矩阵通过，冻结产物，依次发布 crates.io、npm，最后创建 GitHub Release。普通分支推送和 PR 只运行检查。

仓库变量 **`RELEASE_ENABLED` 必须等于 `true`** 才允许实际发布。没有配置时，tag 只触发验证和产物准备。手动运行工作流的 `publish` 默认关闭。构建、测试和冻结产物不需要 registry token。

首个正式版本的剩余验收工作见 [0.1.0 发布检查清单](RELEASE_CHECKLIST.zh-CN.md)。

## 当前 0.1.0 准备状态

**0.1.0-rc.1 已发布**到 crates.io 和 npm `next`/`candidate`，独立扩展数字版本
**0.2.0**，`gdfmtVersion: 0.1.0-rc.1`。全部 npm `latest` 保持原值，本次不发布
正式版 0.1.0，也不上传 Marketplace。

[PR #1](https://github.com/ntnyq/gdfmt/pull/1) 已 squash 合并为
`1329b72dc26179921739e4c2342ffa0bf7b7a314`，`v0.1.0-rc.1` 绑定该提交。
[Release 运行 40051909889](https://github.com/ntnyq/gdfmt/actions/runs/40051909889)
通过全部 tag 检查、冻结及两个 registry 发布。[发布证据](docs/zh/research/0.1.0-rc.1-publication.md)
记录原 artifact ID/hash、十五个 registry 包、真实 Rust/npm 消费者及 GitHub 预发布的
全部 24 个附件。[候选 CI 报告](docs/zh/research/0.1.0-rc.1-ci.md)和
[最初准备](docs/zh/research/0.1.0-rc.1-preparation.md)保留各自历史源码身份，不能
将旧回执重新标记为发布提交。

[registry 与环境审查](docs/zh/research/0.1.0-registry-readiness-review.md)记录了
2026-10-23 的初次只读快照及后续分支/tag 保护落实情况。八个 crate 和七个 npm 包均已存在，公开 owner/maintainer
均为 `ntnyq`，alpha.6 已在
[发布运行 39749770445 的第 2 次尝试](https://github.com/ntnyq/gdfmt/actions/runs/39749770445/attempts/2)
通过 Trusted Publishing 发布。下文首次发布步骤供初始配置使用，下一版本无需重做。

`release` environment 已存在并允许 `v*` tag，仓库变量 `RELEASE_ENABLED` 已为
`true`。`main` 分支保护与已启用的
[Version tags ruleset](https://github.com/ntnyq/gdfmt/rules/25720774) 已配置并验证，
预期设置记录在 [repository-protection.json](.github/repository-protection.json)，
维护者例外见下文。Environment 部署准入与 tag 创建、修改及删除保护仍是不同的控制。

[最初 Trusted Publisher/账号审查](docs/zh/research/0.1.0-trusted-publisher-review.md)
记录历史私有配置读取限制。rc.1 已实际证明十五个 Trusted Publisher，以及 npm
直接发布/dist-tag 权限可用：全部 crate 的公开发布身份匹配本次运行/提交，全部
npm 包记录 GitHub Trusted Publishing 和 provenance。保留 owner 配置完成确认；
邮箱/2FA 和首次发布 token 撤销尚未独立回读。将剩余账号证据与已完成的候选发布/
消费者门槛分开记录，可选 Trusted Publishing Only 加固不新增发布门槛。

本地只读审查可运行 `pnpm release:check` 和 `pnpm test:release`。GitHub 环境、
变量、分支、ruleset API 读取和 `npm trust list` 也为只读操作；registry 管理设置
需要合适的认证。开关已启用时，不要把新 `v*` tag 推送当作只验证；手动工作流保持
`publish: false` 时仍只验证。历史保护落实仅修改分支/tag 设置；后来的 rc.1 tag
与实际发布单独记录在发布报告中。

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

#### 已落实的分支与 tag 策略

已审查的 REST 请求内容保存在
[`.github/repository-protection.json`](.github/repository-protection.json)，用于记录
实际配置；工作流不会自动应用此文件。

- **`main`：**要求 PR、分支保持最新、讨论已解决，以及现有 29 项 CI/npm/宿主
  检查通过，并绑定 GitHub Actions app（ID `15368`）。要求审批人数为 **0**，
  不要求 code-owner 或最后一次推送审批，单维护者可以合并检查通过的 PR。禁用
  force push 和删除。`enforce_admins: false` 保留管理员应急绕过，常规修改仍应
  通过检查成功的 PR 合并。
- **`refs/tags/v*`：**已启用的 ruleset **25720774** 限制创建、更新和删除。
  只有仓库 **Admin（角色 ID 5）**和 **Maintain（角色 ID 2）**具有 `always`
  绕过权限，单有 Write 权限不足以操作。规则覆盖 `v-recovery-*`，没有排除恢复
  前缀，也没有向 Actions/app 开放绕过。

维护者可以在不关闭保护的情况下创建发布和恢复 tag。这些例外不表示可以通过
重指向已发布版本来恢复：保持已发布 tag 不变，保留原冻结 bundle，文档流程需要时
使用新的恢复 ref。发布 job 用 `--verify-tag` 操作已有 tag，`GITHUB_TOKEN` 无需
创建 tag 的绕过权限。`release` 环境准入、OIDC 权限和 `RELEASE_ENABLED` 均未改动。

配置前已核对 `main` 提交 `6c11445a329b688f27fd6693a3c6314ffeedce64` 上全部
29 个检查名称及成功状态。重命名 job 或调整矩阵时，同步修改 JSON 和远端 required
checks，避免 PR 等待已不存在的名称。通过分支保护/ruleset API 回读及 GraphQL
角色名称回读验证了配置，没有使用测试 tag、force push、工作流触发或实际发布验证。

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
pnpm release:version 0.1.0-rc.2 --dry-run
pnpm release:version 0.1.0-rc.2
pnpm release:check
pnpm typecheck
pnpm test:release
```

版本脚本同步修改：

- `Cargo.toml` 中的 workspace 版本；
- 7 个内部 workspace 依赖的精确版本；
- `Cargo.lock` 中全部 11 个本地 workspace 包，包括私有适配层和工具；
- npm 主包、WASM 包版本、5 个 optionalDependencies 版本及 VS Code 的 `gdfmtVersion`。

脚本在修改前后执行一致性检查，后置检查失败时回滚版本文件。它拒绝降级、非法版本和 build metadata，不创建提交或 tag，也不发布。私有根 npm 工具包和文档包保留独立版本。当前 pnpm lockfile 不保存发布包版本，因此不需要修改；如果以后出现对应的锁定版本，脚本会停止，要求先更新同步策略。不要单独使用 `bumpp` 完成跨生态版本更新。

版本替换定位实际的 `[workspace.package]` 字段，不改动注释中的版本示例。
VS Code Marketplace 版本是独立的数字 SemVer，不允许前导零或预发布后缀；脚本只更新
`gdfmtVersion`，需要时应单独递增扩展数字版本。配置与 JSON 结果的
`schemaVersion: 1` 是格式契约，与 Cargo/npm 发布版本独立。

检查版本差异、按需更新发布说明，并合并发布提交。

已准备 0.1.0 的[英文说明](docs/releases/0.1.0.md)与
[简体中文说明](docs/zh/releases/0.1.0.md)。请对照最终候选版本复核，发布完成后才
移除草稿状态。当前工作流使用 GitHub 自动生成的说明；发布完成检查时，将这些
双语内容整合到最终 GitHub Release 正文。仓库中的说明文件不会自动作为发布正文。

然后创建并推送发布 tag：

```sh
git tag -a v0.1.0-rc.2 -m 'Release 0.1.0-rc.2'
git push origin v0.1.0-rc.2
```

正式版本可使用 `0.1.0`，自动进入 `latest`；预发布（包括 rc.1）不会移动 `latest`。

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

核心 0.1.0 建议以 GitHub Release 的六份冻结 VSIX 为交付基线，Marketplace
正式发布独立跟进，不阻塞核心发布。[分发审查](docs/zh/research/0.1.0-editor-distribution-review.md)
记录两条路线和剩余工作。截至 2026-10-23，公开 Gallery 已列出数字扩展 `0.1.0`
的六种目标预发布；已检查的 macOS/Web 清单包含核心 `0.1.0-alpha.6`，不是核心正式版。

rc.1 GitHub Release 已提供六份冻结 VSIX，扩展数字版本 `0.2.0`、内置核心
`0.1.0-rc.1`，带预发布标记。本次未上传 Gallery 或修改 Marketplace 条目。后续上传前
重新核对可用性，统一重建、测试和冻结六种变体。如果先将 `0.2.0` 上传为 Gallery
预发布，正式上传需另选未使用的数字版本。
不能复用现有 Gallery 预发布版本，也不能直接从源码目录发布。先确认 publisher
权限/认证，再通过单独授权的流程上传冻结 VSIX，记录逐目标结果和正式安装消费者，
之后才宣布 Gallery 正式版可用。本轮准备及现有工作流均不执行 Marketplace 上传。

桌面 `pnpm test:vscode` 还会将 VSIX 安装到隔离临时配置，检查安装后扩展/核心版本、
内置 server 字节并使用该 server 格式化 stdin。桌面冻结同时要求
`installedVsixPassed: true` 和 Extension Host 验收；Alpine 保留原生 smoke 检查。

`hosts.yml` 安装真实 WASM tarball，在 Node 和 Chromium 中比较五种语言版本的共享 fixtures，并验证无需隔离响应头的 module Worker。冻结要求 `npm-wasm` 回执的提交、干净源码状态和 SHA-512 匹配。`@gdfmt/wasm` 与平台候选包一起先上传，主包最后上传，使用相同的 `next`/`latest` 渠道；需要单独配置其 npm Trusted Publisher。

同一工作流构建五个平台的 VSIX；冻结要求每个平台原生程序的版本检查，以及四个桌面平台的 Extension Host 验收。Alpine 仅进行原生程序 smoke check，因为 Electron 不支持 musl。Linux 桌面额外启用 Godot Tools 验证共存。`editors.json` 记录核心版本、数字扩展版本、提交和 SHA-512；GitHub Release 附加冻结后的 VSIX。流程不会发布到 VS Code Marketplace。

`hosts.yml` 还使用同一 WASM 包构建独立的纯浏览器 Web VSIX。冻结要求 Chromium Web Extension Host 回执包含 `webExtensionHostPassed: true` 和 `crossOriginIsolated: false`，并匹配干净的 revision 与精确 SHA-512。六个 VSIX 都作为 GitHub Release 附件，不会自动发布到 Marketplace。本地使用 `pnpm test:vscode:web` 验收；单独执行 `pnpm package:vscode:web` 不会记录验收通过。

`release:version` 同步 WASM npm 版本和插件 `gdfmtVersion`。Marketplace 的数字版本独立维护，发布插件更新前需递增，不能使用 Cargo 的预发布后缀。`gdfmt_wasm` 不参与 crates.io 打包。恢复旧版六包冻结产物时，仍以原始源码 checkout 的发布契约校验，不强制补建 WASM。

原生安装应选择匹配本地/远程扩展宿主的附件，通过 **Extensions: Install from VSIX**
安装；手动更新或回滚后运行 **gdfmt: Show Version**。纯浏览器 VS Code 1.96.4
没有本地/远程扩展管理 server 时，不提供该本地 VSIX 入口。现有 Gallery 预发布与
开发侧载是独立路线；staging Web 宿主验收不能证明最终用户的 Gallery/VSIX 安装，
详见[安装说明](docs/zh/guide/integrations.md#vs-code)。
