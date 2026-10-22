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

## VS Code

在仓库运行 `pnpm package:vscode` 生成当前平台的 VSIX，再通过 **Extensions: Install from VSIX** 安装。插件内置匹配版本的原生程序，无需额外安装 Node。SSH、WSL 和容器中，服务在工作区扩展宿主运行。

```json
{
  "[gdscript]": {
    "editor.defaultFormatter": "ntnyq.gdfmt",
    "editor.formatOnSave": true
  }
}
```

格式化使用未保存的缓冲区；源码无效时展示诊断，不返回文本编辑。可以保留 Godot Tools 的语言与引擎功能，并显式将 gdfmt 选为默认格式化器。

VS Code 缓冲区只能使用统一换行符。若核心输出需要混合换行以保留 formatter-off 区域或多行字符串，桌面端与 Web 端会报告错误并保持文档不变。可设置 `lineEnding: "preserve"`，或使用 CLI 保留精确输出。若保留文本决定了统一换行符，则以核心实际输出为准，而不是覆盖为配置中的结构换行符。

支持 `gdfmt.enable`、`gdfmt.serverPath`、`gdfmt.configPath`。相对路径以所属工作区目录为基准。程序选择顺序为显式路径、受信任工作区安装的 `@gdfmt/gdfmt`、插件内置程序；所选程序必须与插件记录的核心版本完全一致。工作区 npm 脚本使用扩展宿主的 Node，并需满足 npm 包的 Node 版本要求。未信任的工作区只运行内置程序，使用常规配置发现。

提供 **gdfmt: Restart Server**、**gdfmt: Show Output**、**gdfmt: Show Version** 命令。多根目录使用独立客户端。Untitled 文档使用内置程序，直接使用格式化器默认配置；保存到项目后才使用项目配置。浏览器与虚拟工作区使用下方独立的 Web VSIX。

### VS Code Web

运行 `pnpm package:vscode:web` 生成 `target/editors/web/gdfmt-VERSION-web.vsix`，在 VS Code Web 中安装，使用上方相同的默认格式化器和保存设置。Web 版本在独立 Worker 中复用现有 `@gdfmt/wasm`，无需原生程序、Node 运行时或跨源隔离响应头。`gdfmt.serverPath` 仅用于桌面端；**Restart Server** 在 Web 中重建 Worker。

Web 通过 `vscode.workspace.fs` 支持虚拟工作区 URI、未保存缓冲区、保存格式化、撤销、多根工作区与 UTF-16 诊断。每次请求使用文档所属工作区的配置；Untitled 文档使用默认值。工作区外的文档需要先放入工作区目录。

配置发现从文档目录向上搜索到所属工作区根目录，每层优先使用 `.gdfmtrc.json`，其次是 `.gdfmtrc.jsonc`。先合并祖先层，再合并子层；`root: true` 停止向上发现。JSONC 注释和相对 `extends` 使用共享 Rust 解析器与选项校验。`gdfmt.configPath` 相对于所属工作区目录，设置后替代自动发现。所有路径及继承配置必须留在该目录内，使用 `/` 分隔符；嵌套工作区目录有独立的配置边界。

Web 暂时拒绝包含 `overrides` 或 `ignore` 的配置（包括空数组），也拒绝发现到的 `.editorconfig`，展示错误并保留原文。需要这些功能时使用桌面扩展，或改用受支持的 JSON/JSONC 选项。Web 格式化显式打开的文档，`.gitignore` 和 `.gdfmtignore` 不参与选择或排除。受信任和未信任工作区使用相同的 Web 配置规则。

每个配置文件限制为 1 MiB，累计 16 MiB，最多 32 层继承、128 个合并层，配置发现最多经过 128 个目录；源码限制为 16 MiB。取消或过期的请求不会应用编辑。Worker 异常或超时后会被丢弃，下次请求重新创建。

## 语言服务器

```sh
gdfmt --lsp
```

服务通过 stdio 使用 LSP 3.17，仅声明全文同步、UTF-16 位置和全文格式化能力，不声明范围格式化。每次请求都重新读取项目配置，包括 JSON/JSONC 继承、override、EditorConfig 和忽略规则。源码错误发布诊断并返回空编辑，配置失败返回 LSP 错误。服务不会写入文档。

VS Code 客户端使用 `gdfmt/formatting` 扩展请求，传入相同的 `textDocument` 标识。结果包含 `output`（格式化文本，源码无效或被忽略时为 `null`）和解析后的 `lineEnding`（`lf`、`crlf` 或 `preserve`），以便同时应用文本编辑与缓冲区 EOL 变更。服务通过 `experimental.gdfmtTextFormattingProvider` 声明该能力；取消、失效检查和诊断行为与标准格式化请求一致。

可通过 `initializationOptions.configPath` 或 `gdfmt --lsp --config path/to/config.json` 指定配置。发送 `workspace/didChangeConfiguration`，参数为 `{ "settings": { "gdfmt": { "configPath": "..." } } }` 可替换指定路径，空路径恢复发现。取消请求会立即响应；文档或配置更新使待处理结果失效。正在运行的格式化检查在后台完成，已取消的排队任务直接跳过。

自定义宿主仍可使用 [Node.js API](/zh/reference/node)，浏览器和 Worker 可使用 [WASM API](/zh/reference/wasm)。

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
