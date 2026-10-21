# CI 与编辑器工作流

本地与 CI 应使用相同的 gdfmt 版本和已提交的配置。以下命令假设 `gdfmt` 已在 `PATH` 中，源码安装步骤见[快速开始](/zh/guide/)。

## 持续集成

检出代码并安装工具后运行只读检查：

```sh
gdfmt --check .
```

非零退出码使步骤失败。本地可运行 `gdfmt --diff .` 查看差异，运行 `gdfmt .` 应用格式化。通过[退出码](/zh/reference/cli#exit-codes)可区分格式差异与配置、I/O 错误。

发布包可用之前，请从经过审查的固定 Git revision 安装。例如，在已安装 Rust 的 CI shell 步骤中执行：

```sh
# 将 GDFMT_REVISION 设置为你已审查的完整 commit SHA。
cargo install --git https://github.com/ntnyq/gdfmt.git --rev "$GDFMT_REVISION" --locked gdfmt_cli
gdfmt --check .
```

在 CI 配置中固定 revision，需要采用格式化变化时再显式更新。

## 编辑器命令

对于支持通过 stdin 调用外部格式化器的编辑器，使用：

```sh
gdfmt --stdin-filepath path/to/current-file.gd
```

将缓冲区内容传入 stdin，仅在退出码为 `0` 时用 stdout 替换文本，失败时展示诊断。虚拟路径用于选择文件的配置、EditorConfig、覆盖和忽略规则，不会写入文件。请从项目目录运行，或传入绝对虚拟路径。

目前没有专用的 gdfmt 编辑器扩展或语言服务器。编辑器可通过外部命令或 [Node.js API](/zh/reference/node) 集成；API 调用方需要显式解析项目配置。

## 忽略生成代码与第三方代码

在项目根目录创建 `.gdfmtignore`，使用 gitignore 风格的规则：

```text
generated/
addons/vendor/
```

项目的 `.gitignore` 同样有效，内置排除包含 Godot 的 `.godot` 缓存目录。优先级与显式文件行为见[文件选择](/zh/guide/configuration#ignore-rules-and-input-selection)。

只保留脚本的一部分排版时，使用[保护区域](/zh/reference/formatting#protected-regions)：

```gdscript
# gdfmt: formatter-off
var table=[ 1,    2,3 ]
# gdfmt: formatter-on
```

## Git hooks

简单的 pre-commit hook 可以检查指定项目目录：

```sh
gdfmt --check src addons
```

请选择项目中实际存在的目录。该命令检查工作区中的这些目录；按暂存文件集成时，应传入明确的 `.gd` 路径，并在调用 gdfmt 前处理空选择，因为没有匹配输入会报错。
