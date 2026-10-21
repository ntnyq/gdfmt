# 脚本格式化（P4–P5）

P3 已[按明确排除项验收](/zh/development/syntax-audit)。格式化器现已覆盖注释、lambda、声明与 suite，保留既有的表达式布局和 CLI。源码构建 alpha 验收证据、固定项目试用与排除项见 [P4 验收记录](/zh/development/formatter-acceptance)。

## 当前行为

格式化器规范运算符、调用、数组、字典、分组、注解、签名、声明、属性访问器、语句和 match 模式的布局。具名函数、类、分支、循环和访问器使用缩进后的多行 suite。分号分隔的语句展开为独立行，同时保留原分号 token。仅有一条简单语句且无注释的 lambda 使用行内函数体，其余使用块布局。容器内块 lambda 的 suite 缩进以外层语句为基准，因为续行列不是 lexer 缩进栈中的层级。

参数和元素列表按 `format.lineWidth` 换行，suite 和续行统一使用 `indentStyle`/`indentWidth`。列宽按 Unicode scalar 和 tab stop 计算。不可拆分的 token 与类型引用允许超宽：Godot 会拒绝在集合类型左方括号后立即换行。括号、逗号、分号、字典分隔符风格、字面量词形和 BOM 均保留。多行字面量内容逐字节保留，包括其中的原始换行和缩进。

独立行注释锚定后一个 token，行尾注释锚定前一个 token；悬空注释保留在闭合分隔符内或所属 suite、属性块、match 块末尾。文档、region、warning 注释保留原文和顺序，注解顺序与目标不变。打印器为注释建立换行边界，避免吞掉后续代码。

`lineEnding` 选择 LF、CRLF 或源码首个换行（`preserve`，无换行时回退到 LF），只影响布局，不改字面量。`maxBlankLines` 限制已有空行数量，去除文件首尾空行；`finalNewline` 控制非空文件末尾的一次换行。配置默认 LF 且保留最终换行，纯 Rust `FormatOptions` 默认沿用源码换行。独立表达式树保留表达式入口行为，不应用文件末尾换行策略。

```gdscript
func run( value : int=1 ): return [value,2] # result
```

变为：

```gdscript
func run(value: int = 1):
	return [value, 2] # result
```

## 保护区域

使用成对的独立行注释：

```gdscript
# gdfmt: formatter-off
var table=[ 1,    2,3 ]
# gdfmt: formatter-on
var other=4
```

区域内容及两个标记行逐字节保留，最后一条声明变为 `var other = 4`。保护区域优先于缩进、空行、换行和最终换行选项，因此可有意保留混合换行。BOM 仍位于文件开头，支持多段分离区域。

只有真实的独立行注释 token 才生效；字符串和行尾注释内的标记文字不生效。标记必须依次成对，不能嵌套；未配对或嵌套产生 `FMT002`，无输出，CLI 返回 1。未知的 `gdfmt:` 注释按普通注释处理。`check --syntax-only` 不解释 formatter 指令。

整个文件（包括保护内容）仍须语法有效，区域不能屏蔽语法诊断。facade 除重解析、结构/注释等价与幂等外，还比较保护区域的原始字节。如果恢复区域会与周围缩进或表达式边界冲突，保守保留区域涉及的完整顶层声明/语句；这可能额外保留附近若干行，其他顶层单元仍正常格式化。再次验证仍失败时，不提供输出。

lint 禁用指令、lint 规则和语义自动修复不属于 formatter 范围。`check` 继续检查语法/格式，使用 `format` 应用布局变化。

## CLI

```sh
cargo run -p gdfmt_cli -- format src addons
cargo run -p gdfmt_cli -- src/player.gd
cargo run -p gdfmt_cli -- check .
cargo run -p gdfmt_cli -- --check .
cargo run -p gdfmt_cli -- check . --syntax-only
cargo run -p gdfmt_cli -- --diff src/player.gd
cargo run -p gdfmt_cli -- --stdin-filepath src/player.gd < input.gd
```

`format` 与默认命令写入有效且发生变化的文件；`check` 检查语法及脚本格式，不执行 lint。`--syntax-only` 跳过格式化；`--diff` 输出整文件替换的 unified diff，不写入。`format -` 读取 stdin；`--stdin-filepath` 还会按虚拟路径最近的已存在父目录解析配置。stdin 格式化成功时 stdout 仅输出源码，JSON reporter 则使用 `output` 字段；无效输入不输出格式化源码。

退出码：0 为成功，1 为源码错误或只读模式发现格式差异，2 为参数/配置/I/O/内部失败。JSON 文件报告包含 `changed` 与结构化诊断；syntax-only 省略 `changed`。文件发现继续沿用排序、去重和忽略规则。

所有配置与输入读取在写入前预检。每个文件通过同目录临时文件替换，保留权限，并在替换前比较当前内容与读取的源码。无效文件保持不变，同批其他有效文件可以格式化；原子性以单文件为单位。

## Rust API 与正确性门禁

`gdfmt_formatter` 接收无损树、同次解析诊断及布局选项，正常依赖不包含 parser 或文件系统。`gdfmt::format(source, &FormatRequest)` 组合解析和格式化，返回 `FormatResult { output, changed, diagnostics }`。

facade 仅在以下条件全部成立时返回可写输出：输入无 parser 错误；输出在相同版本和限制下重解析成功；规范化节点结构及有效 token 原文一致；注释原文、相邻 token、独立行角色及悬空块归属一致；第二次格式化输出完全相同。失败时返回内部诊断且不提供输出。formatter 自身也独立拒绝恢复节点和错误诊断。

Doc 打印器支持 text、concat、group、indent、软/硬/条件换行和行尾后缀。测试覆盖宽度边界、后续文本、嵌套分组、后缀、CRLF、长 token、表达式 token 边界插入注释，以及 40/100 宽度下人工检查过的整脚本快照。每个 parser 可接受脚本/版本使用四组布局配置检查，包含 tab/空格缩进、LF/CRLF/preserve、空行上限和最终换行。无效语料不得产生可写输出。

```sh
cargo test -p gdfmt_formatter -p gdfmt -p gdfmt_cli -p gdfmt_workspace
cargo xtask formatter-conformance --language 4.7 --godot /path/to/pinned/godot --report target/conformance/formatter-4.7.json
```

引擎命令沿用五个精确版本和宽度 40，仅运行 check-only，并记录格式化源码和日志，不执行脚本、不证明运行时等价。2026-10-21 核验：522 个有效脚本/版本输出全部通过五个固定 macOS 引擎（4.3–4.7 分别 90/98/110/112/112），零失败。Rust 语料检查共 2176 个有效脚本/版本/布局组合，包含 22 个 parser 接受的 analyzer 边界在四组布局下的检查；这些边界不计入引擎接受数。固定项目试用中 464 个脚本通过 1856 组布局，另有一个上游无效源码明确拒绝；401 份 gdtoolkit 风格差异已审查并记录指纹。P4 源码构建 alpha 已按排除项验收；长期 formatter fuzz 为后续工作，生产 CLI 扩展属于 P6。
