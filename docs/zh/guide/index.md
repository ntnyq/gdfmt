# 快速开始

gdfmt 通过命令行格式化 GDScript 文件。可以直接使用默认设置，需要共享项目风格时再提交配置文件。

当前正式版本为 **0.1.0**，可从 crates.io 和 npm `latest` 安装，详见
[发布说明](../releases/0.1.0.md)及[registry 验收](../research/0.1.0-publication.md)。
冻结 VSIX 使用扩展版本 **0.2.1**，同一批产物已正式上线
[Marketplace](../research/0.1.0-marketplace.md)。
源码构建使用 checkout 的版本。

## 从源码安装

使用 Rust 1.88 或更高版本。仓库中的 `rust-toolchain.toml` 固定了开发工具链。

```sh
git clone https://github.com/ntnyq/gdfmt.git
cd gdfmt
cargo install --path crates/gdfmt_cli --locked
gdfmt --version
```

确保 Cargo 的可执行文件目录已加入 `PATH`。也可以在仓库内通过 `cargo run -p gdfmt_cli -- <参数>` 直接运行 CLI。

## 安装正式版本

固定发布版本，Cargo 包名与可执行命令不同：

```sh
cargo install gdfmt_cli --version '=0.1.0' --locked
gdfmt --version
```

npm 方式需要 Node 22.x 的 22.13+ 或 Node 24.x，并保留 optional dependencies：

```sh
pnpm add -D @gdfmt/gdfmt@latest
pnpm exec gdfmt --version
```

单次调用可运行 `pnpm dlx @gdfmt/gdfmt@latest --version`。支持的原生平台与 API 用法见
[Node 参考](/zh/reference/node)。`latest` 是会移动的正式通道；CI 中应固定精确版本，
或提交项目 lockfile。上方源码安装使用检出的 revision。

## 格式化项目

在你的 Godot 项目目录中运行：

```sh
gdfmt .
```

默认命令将格式化结果写回原文件，也可以指定单个文件、目录或多个路径：

```sh
gdfmt src/player.gd
gdfmt src addons
```

只选择 `.gd` 文件，遵循项目忽略文件，并跳过 `.git`、`.godot`、`node_modules` 和 `target`。详见[文件选择](/zh/guide/configuration#ignore-rules-and-input-selection)。

例如：

```gdscript
func run( value : int=1 ): return [value,2] # result
```

格式化后：

```gdscript
func run(value: int = 1):
	return [value, 2] # result
```

## 检查或预览差异

```sh
gdfmt --check .
gdfmt --diff src/player.gd
```

两种命令都不修改文件。格式一致时退出码为 `0`，发现源码错误或格式差异时为 `1`，配置、参数或 I/O 错误为 `2`。

只检查语法可运行 `gdfmt check . --syntax-only`。它使用 gdfmt 的解析器，不能替代 Godot 的项目分析。

## 共享配置

```sh
gdfmt init
```

这会在当前目录创建 `.gdfmtrc.json`，已有文件时拒绝覆盖。默认使用 100 列行宽、Tab 缩进、LF 换行和文件末尾换行。

选项、EditorConfig、继承与按路径覆盖见[配置说明](/zh/guide/configuration)。运行 `gdfmt explain-config src/player.gd` 可查看某个文件的实际配置。

## 接下来

- [接入 CI 或编辑器命令](/zh/guide/integrations)。
- [保持指定区域不变](/zh/reference/formatting#protected-regions)。
- [查阅 CLI 命令](/zh/reference/cli)或 [Node.js API](/zh/reference/node)。
- [了解语言支持与限制](/zh/reference/compatibility)。
