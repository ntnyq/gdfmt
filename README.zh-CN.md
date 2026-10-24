# gdfmt

<p align="center">
  <img src="docs/public/logo.svg" width="160" height="160" alt="gdfmt 标志" />
</p>

<p align="center">以 Rust 构建的 GDScript 格式化工具。</p>
<p align="center"><a href="docs/zh/guide/index.md">快速开始</a> · <a href="docs/zh/guide/configuration.md">配置</a> · <a href="CONTRIBUTING.md">参与贡献</a></p>

[English](README.md) | 简体中文

gdfmt 为 `.gd` 文件提供统一的空格、缩进与换行布局，可通过命令行或 Node.js API 使用。

- **验证格式化结果。** 重新解析输出，检查语法结构与注释保留情况，并验证再次格式化的结果完全一致。
- **项目级配置。** 通过 `.gdfmtrc.json` 或 `.gdfmtrc.jsonc`、EditorConfig 和按路径覆盖配置行宽、缩进与换行。
- **融入日常工作流。** 格式化项目、在 CI 中检查、预览差异，或通过 stdin 处理编辑器缓冲区。支持 Git 忽略规则与 `.gdfmtignore`。
- **保留手动排版。** 使用成对的 `formatter-off` / `formatter-on` 注释，保持指定区域原样。

**0.1.0 已正式发布**到 crates.io 和 npm `latest`，六份冻结 VSIX 使用扩展版本
**0.2.1**。详见[发布说明](docs/zh/releases/0.1.0.md)和
[发布及 registry 验收](docs/zh/research/0.1.0-publication.md)。
默认格式化风格及公开 API/Schema 契约与 rc.1 保持一致，Marketplace 正式发布独立跟进。

## 快速开始

使用 Rust 1.88 或更高版本从源码安装，仓库已固定开发工具链。

```sh
git clone https://github.com/ntnyq/gdfmt.git
cd gdfmt
cargo install --path crates/gdfmt_cli --locked
```

在你的 Godot 项目中运行：

```sh
gdfmt .                 # 原地格式化 GDScript 文件
gdfmt --check .         # 只检查，不写入文件
gdfmt --diff src        # 预览格式化差异
gdfmt init              # 创建 .gdfmtrc.json
```

不传参数时默认格式化当前目录。检查发现语法错误或格式差异时退出码为 `1`，配置、参数或 I/O 错误为 `2`。

安装正式版本可运行 `cargo install gdfmt_cli --version '=0.1.0' --locked`，
或在 Node 22.x 的 22.13+、Node 24.x 下运行 `pnpm add @gdfmt/gdfmt@latest`。
Cargo 包名为 `gdfmt_cli`，两种方式提供的命令均为 `gdfmt`。
库用法与支持的原生平台见 [Node.js API](docs/zh/reference/node.md)。

## 配置

gdfmt 无需配置即可使用。需要共享项目设置时，创建 `.gdfmtrc.json` 或 `.gdfmtrc.jsonc`（均支持注释和尾随逗号）：

```json
{
  "root": true,
  "language": { "godotVersion": "4.7" },
  "format": {
    "lineWidth": 100,
    "indentStyle": "tab"
  }
}
```

全部选项、继承、路径覆盖与 EditorConfig 支持见[配置说明](docs/zh/guide/configuration.md)。

## 文档

- [快速开始](docs/zh/guide/index.md)
- [CLI 参考](docs/zh/reference/cli.md)
- [CI 与编辑器工作流](docs/zh/guide/integrations.md)
- [格式化与保护区域](docs/zh/reference/formatting.md)
- [语言支持与限制](docs/zh/reference/compatibility.md)
- [Node.js API](docs/zh/reference/node.md)
- [参与贡献](CONTRIBUTING.md)
- [发布与 CI 自动发布](RELEASING.zh-CN.md)
- [0.1.0 发布检查清单](RELEASE_CHECKLIST.zh-CN.md)
- [0.1.0 发布说明](docs/zh/releases/0.1.0.md)

提供 Godot 4.3–4.7 语法配置，尚未声明完整 Godot 兼容性；项目分析与运行时行为仍由引擎负责。gdfmt 使用自己的格式化风格，不保证与 gdtoolkit 的 `gdformat` 输出完全一致。

## WASM 与 VS Code

浏览器包 `@gdfmt/wasm` 提供 `createFormatter()`，用法见 [WASM API](docs/zh/reference/wasm.md)。可运行 `pnpm add @gdfmt/wasm@latest` 安装，或运行 `pnpm build:wasm`、`pnpm test:wasm` 从源码构建与验收。

`gdfmt --lsp` 通过 stdio 提供格式化服务。运行 `pnpm package:vscode` 构建桌面 VSIX，或运行 `pnpm package:vscode:web` 构建复用 WASM 的 Web VSIX。桌面安装匹配的原生 VSIX 后，将 GDScript 的 `editor.defaultFormatter` 设为 `ntnyq.gdfmt`。纯浏览器使用现有 Gallery 预发布或受支持的开发路线，并非所有宿主都能安装本地 Web VSIX。详见[编辑器集成](docs/zh/guide/integrations.md)。

源码包及 VSIX 构建需要 Node、仓库声明的 pnpm 版本，并先运行
`pnpm install --frozen-lockfile`；WASM/Web 构建还需要 wasm-pack 和 WASM Rust
目标，详见[构建前置条件](CONTRIBUTING.md#wasm-and-editors)。

## 许可

[MIT](LICENSE)。第三方归属见 [LICENSE-THIRD-PARTY](LICENSE-THIRD-PARTY)。

Logo 使用了 Andrea Calabró 创作的 Godot Engine 图标，遵循 [CC BY 4.0](docs/public/logo.LICENSE.txt)。gdfmt 是独立项目，未获 Godot Engine 项目背书。
