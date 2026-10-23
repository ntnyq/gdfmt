# CLI 参考

```sh
gdfmt [选项] [路径]...
gdfmt <命令> [选项]
```

不传命令和路径时，默认格式化当前目录。路径可以是文件或目录，递归发现只选择 `.gd` 文件。CLI 自身不展开 glob 表达式。

## 命令

| 命令                                  | 行为                                                   |
| ------------------------------------- | ------------------------------------------------------ |
| `gdfmt [路径]...`                     | 原地格式化文件                                         |
| `gdfmt format [路径]...`              | 显式指定默认的格式化命令                               |
| `gdfmt check [路径]...`               | 只检查语法与格式，不写入文件                           |
| `gdfmt check [路径]... --syntax-only` | 只检查解析器诊断                                       |
| `gdfmt init`                          | 创建 `.gdfmtrc.json`，任一种配置文件名已存在时拒绝创建 |
| `gdfmt explain-config [路径]`         | 以 JSON 输出实际配置及字段来源，默认路径为 `.`         |
| `gdfmt schema`                        | 输出配置的 JSON Schema                                 |
| `gdfmt lex [路径]...`                 | 只读查看 token 和词法诊断                              |

`lex` 用于调试。词法检查通过不代表脚本语法有效；检查语法请使用 `check --syntax-only`。

## 选项

| 选项                       | 行为                                               |
| -------------------------- | -------------------------------------------------- |
| `--check`                  | 只检查语法与格式，等价于 `check`                   |
| `--diff`                   | 输出格式化差异，不写入文件                         |
| `--config <文件>`          | 使用显式的本地 JSON/JSONC 配置，替代按目录发现配置 |
| `--stdin-filepath <路径>`  | 从 stdin 读取内容，按虚拟 `.gd` 路径解析配置       |
| `--reporter <human\|json>` | 选择输出格式，默认为 `human`                       |
| `--lsp`                    | 通过 stdio 运行格式化语言服务器                    |
| `-h`、`--help`             | 显示帮助，各子命令也可使用                         |
| `-V`、`--version`          | 显示版本                                           |

`--check` 与 `--diff` 互斥。`--syntax-only` 属于 `check` 子命令，不能与 `--diff` 同用。格式化模式和 stdin 路径仅用于格式化与检查命令。

`--config` 用于格式化、检查、`lex`、`explain-config` 与 LSP，`init` 和 `schema`
会拒绝该选项。`--lsp` 不能与子命令、路径、格式化模式、stdin 路径或
`--reporter=json` 同用，其 stdout 使用带帧头的 JSON-RPC。非法选项组合在读取配置
或写入文件前失败。

布局选项在[配置文件](/zh/guide/configuration)中设置。指定 `--config` 后，该文件的 `extends`、EditorConfig 与本地忽略规则仍然有效。

## 标准输入

```sh
gdfmt format - < script.gd
gdfmt --stdin-filepath src/player.gd < script.gd
```

`format -` 使用当前工作目录下的虚拟路径 `stdin.gd`。`--stdin-filepath` 用完整虚拟文件名匹配配置覆盖与忽略规则，文件可以不存在，也不会写入该路径。成功时将格式化源码输出到 stdout；输入无效时不输出格式化源码。stdin 不能与文件路径混用。

## 输出格式

```sh
gdfmt check src --reporter json
gdfmt --diff src --reporter json
```

人类可读错误写入 stderr。JSON 报告写入 stdout，成功生成报告时 stderr 为空。报告包含 `schemaVersion: 1`、`spanEncoding: "utf8Bytes"`、命令、状态、文件和诊断。格式化文件条目包含 `changed`，仅检查语法时省略该字段；stdin 结果和差异分别出现在适用条目的 `output`、`diff` 字段中。

诊断 span 使用半开 UTF-8 字节范围 `[start, end)`，不是字符位置或 UTF-16 偏移。虚拟词法 token 的范围宽度为零。`schema` 与 `explain-config` 直接输出 JSON。

帮助和版本请求退出 0，即使指定 `--reporter=json` 仍输出纯文本。其他参数错误在
`--` 之前选择 JSON reporter 时输出 JSON；`--` 之后的 token 是字面路径。
`CLI_ARGUMENT` 表示参数解析/选项校验失败，`CONFIG_INVALID` 表示项目配置失败，
`CLI_IO` 表示路径/文件/流失败。宿主诊断含 `code`、`category`、`severity` 和
`message`，源码诊断另含 `sourceId` 与 `span`。消息文本面向人类，不应作为解析接口。
`FMT001` 表示格式化器内部失败；无效保护指令使用 `FMT002` 并退出 1，尽管其诊断
类别是 `configuration`。

## 退出码 {#exit-codes}

| 退出码 | 含义                                                 |
| ------ | ---------------------------------------------------- |
| `0`    | 成功，检查未发现错误或格式差异                       |
| `1`    | 源码错误、无效格式化指令，或只读检查发现格式差异     |
| `2`    | 参数、配置、I/O 或内部验证失败，也包括未找到输入文件 |

多文件运行取最高退出码。所有配置和输入读取会在写入前预检。源码错误会保留该文件原样，其他有效文件仍可能被格式化。替换以单文件为单位保持原子性，不是整个批次的事务。详见[写入行为](/zh/guide/configuration#writes-and-current-limits)。
