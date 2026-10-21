# 脚本格式化

gdfmt 格式化完整脚本，覆盖表达式、注释、lambda、声明与语句块。可以直接使用默认设置，也可以调整[项目配置](/zh/guide/configuration)。已知边界见[语言支持](/zh/reference/compatibility)。

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

## 保护区域 {#protected-regions}

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

## 命令行

使用 `gdfmt .` 写入格式化结果，`gdfmt --check .` 只读检查，`gdfmt --diff .` 预览差异。stdin、输出格式与退出码见 [CLI 参考](/zh/reference/cli)。

## Rust API 与正确性门禁

`gdfmt_formatter` 接收无损树、同次解析诊断及布局选项，正常依赖不包含 parser 或文件系统。`gdfmt::format(source, &FormatRequest)` 组合解析和格式化，返回 `FormatResult { output, changed, diagnostics }`。

facade 仅在以下条件全部成立时返回可写输出：输入无 parser 错误；输出在相同版本和限制下重解析成功；规范化节点结构及有效 token 原文一致；注释原文、相邻 token、独立行角色及悬空块归属一致；第二次格式化输出完全相同。失败时返回内部诊断且不提供输出。formatter 自身也独立拒绝恢复节点和错误诊断。

复现命令、用例组织与引擎比较见[贡献指南](https://github.com/ntnyq/gdfmt/blob/main/CONTRIBUTING.md)。
