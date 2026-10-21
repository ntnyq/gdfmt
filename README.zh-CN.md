# gdfmt

<p align="center">
  <img src="docs/public/logo.svg" width="160" height="160" alt="gdfmt 标志" />
</p>

<p align="center">以 Rust 构建的 GDScript 格式化工具。</p>
<p align="center"><a href="docs/zh/guide/index.md">快速开始</a> · <a href="docs/zh/guide/configuration.md">配置</a> · <a href="CONTRIBUTING.md">参与贡献</a></p>

[English](README.md) | 简体中文

gdfmt 为 `.gd` 文件提供统一的空格、缩进与换行布局，可通过命令行或 Node.js API 使用。

- **验证格式化结果。** 重新解析输出，检查语法结构与注释保留情况，并验证再次格式化的结果完全一致。
- **项目级配置。** 通过 `.gdfmtrc.json`、EditorConfig 和按路径覆盖配置行宽、缩进与换行。
- **融入日常工作流。** 格式化项目、在 CI 中检查、预览差异，或通过 stdin 处理编辑器缓冲区。支持 Git 忽略规则与 `.gdfmtignore`。
- **保留手动排版。** 使用成对的 `formatter-off` / `formatter-on` 注释，保持指定区域原样。

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

使用 `pnpm add @gdfmt/gdfmt@next` 安装 npm 预发布版本。使用方式见 [Node.js API](docs/zh/reference/node.md)。

## 配置

gdfmt 无需配置即可使用。需要共享项目设置时，创建 `.gdfmtrc.json`：

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

提供 Godot 4.3–4.7 语法配置，尚未声明完整 Godot 兼容性；项目分析与运行时行为仍由引擎负责。gdfmt 使用自己的格式化风格，不保证与 gdtoolkit 的 `gdformat` 输出完全一致。

## 许可

[MIT](LICENSE)。第三方归属见 [LICENSE-THIRD-PARTY](LICENSE-THIRD-PARTY)。

Logo 使用了 Andrea Calabró 创作的 Godot Engine 图标，遵循 [CC BY 4.0](docs/public/logo.LICENSE.txt)。gdfmt 是独立项目，未获 Godot Engine 项目背书。
