# 配置

使用 `.gdfmtrc.json` 或 `.gdfmtrc.jsonc` 共享格式选项并选择语言版本。全部列出的配置用于[脚本格式化](/zh/reference/formatting)，包含 suite/续行缩进、布局换行、保留空行和最终换行；字面量内容和 formatter-off/on 保护区域仍逐字节保留。

```json
{
  "$schema": "./gdfmtrc.schema.json",
  "schemaVersion": 1,
  "root": true,
  "language": { "godotVersion": "4.7" },
  "format": {
    "lineWidth": 100,
    "indentStyle": "tab",
    "indentWidth": 4,
    "lineEnding": "lf",
    "maxBlankLines": 2,
    "finalNewline": true
  }
}
```

运行 `gdfmt schema > gdfmtrc.schema.json`，将 Schema 保存在配置旁，即可使用上面的 `$schema` 路径。`$schema` 仅是编辑器元信息，运行时不会联网加载。

两种文件名都支持 JSONC：`//` 行注释、`/* ... */` 块注释，以及对象和数组中的尾随逗号。显式 `--config` 文件和 `extends` 加载的文件也使用相同解析方式，不受后缀限制。字符串中的 URL、glob 等类似注释的内容保持原样。不支持单引号字符串、无引号键名等 JSON5 语法。

例如，将以下内容保存为 `.gdfmtrc.jsonc`：

```jsonc
{
  // 项目共享设置
  "root": true,
  "extends": ["./base.jsonc"],
  "format": {
    /* 使用更宽的布局 */
    "lineWidth": 120,
  },
}
```

`gdfmt init` 创建 `.gdfmtrc.json`；两种配置文件名任一已存在时，都会拒绝创建。

| 字段                  | 默认值 | 范围                        |
| --------------------- | ------ | --------------------------- |
| schemaVersion         | 1      | 仅 1                        |
| root                  | false  | 布尔值，终止祖先配置查找    |
| language.godotVersion | 4.7    | 4.3 / 4.4 / 4.5 / 4.6 / 4.7 |
| format.lineWidth      | 100    | 40–320                      |
| format.indentStyle    | tab    | tab / space                 |
| format.indentWidth    | 4      | 1–8                         |
| format.lineEnding     | lf     | lf / crlf / preserve        |
| format.maxBlankLines  | 2      | 0–5                         |
| format.finalNewline   | true   | 布尔值                      |

## 发现、继承与 overrides {#discovery-inheritance-and-overrides}

逐个输入从所在目录向上收集 `.gdfmtrc.json` 或 `.gdfmtrc.jsonc`，遇到 `root: true` 或文件系统根停止。每个目录中 `.gdfmtrc.json` 优先；两者同时存在时仅读取 `.gdfmtrc.json`，该文件出错时不会回退到 `.gdfmtrc.jsonc`。stdin 虚拟路径可尚不存在；按现有祖先查找，仍保留完整文件名用于匹配。显式 `--config` 相对工作目录解析，替代目录 JSON/JSONC 发现，但仍展开其 `extends`，并保留 EditorConfig 和本地忽略规则。

```json
{
  "root": true,
  "extends": ["./base.json", "./team.json"],
  "ignore": ["generated/", "vendor/"],
  "overrides": [
    {
      "files": ["src/**/*.gd"],
      "exclude": ["src/generated/**"],
      "format": { "indentStyle": "space", "indentWidth": 2 }
    }
  ]
}
```

优先级从低到高：默认值 → EditorConfig → 全部 JSON/JSONC 基础字段 → 全部匹配的 overrides。JSON/JSONC 按祖先到子目录应用；每层先按 `extends` 数组从左到右深度优先展开，再应用自身。共享父配置按顺序重复应用；使用规范化真实路径检测循环。`root` 仅阻止祖先发现，不阻止显式继承边。限制为 32 个活跃继承文件、128 层应用、16 MiB 配置总文本；每份配置/ignore 文件上限 1 MiB。仅加载本地文件，不联网、不解析包名。

对象逐字段合并，缺失字段及空对象继续继承，显式 `false` 和 `0` 保留。普通数组（`extends`、`ignore`）整体替换，包括 `[]`。`overrides` 是追加例外：`[]` 不新增条目，也不清空继承条目。每份基础配置及 override 都验证，即使后续设置将覆盖它。

继承路径、override glob 和 JSON/JSONC ignore 均以声明文件所在目录为基准。Override 匹配完整相对路径，区分大小写，各平台统一使用 `/`：`*.gd` 只匹配直接子文件，`**/*.gd` 同时匹配后代。支持 `*`、`?`、字符类和花括号备选；绝对路径、`..` 段、反斜线、空 glob 和错误 glob 均拒绝。每项必须有 `files`，`files: []` 不匹配任何文件；任一 `exclude` 匹配则跳过该项。按顺序应用，后项逐字段覆盖。Override 只允许 `files`、`exclude`、`language`、`format`。

拒绝未知字段、`null`、错误类型、越界值、未知枚举和未来 Schema 版本，错误包含文件和字段路径。

```sh
gdfmt explain-config path/to/script.gd
```

`explain-config` 支持实际及虚拟路径，输出应用顺序中的 JSON/JSONC `files`、`editorconfigFiles`、合并值、`ignored` 和字段最终 `sources`。默认来源为 `default`，override 来源为 `file#overrides[index]`，EditorConfig 为 `file:line (property)`；显式 JSON/JSONC ignore 数组会增加其声明来源 `ignore`。

## EditorConfig

独立于 JSON/JSONC `root` 发现 EditorConfig，遇到自身 `root = true` 停止。祖先文件和匹配 section 按顺序生效，支持标准 EditorConfig glob、属性名/值大小写规则和 `unset`。映射范围如下：

| EditorConfig 属性          | Formatter 字段        | 支持值                                    |
| -------------------------- | --------------------- | ----------------------------------------- |
| `indent_style`             | `format.indentStyle`  | `tab`、`space`                            |
| `indent_size`、`tab_width` | `format.indentWidth`  | 1–8；`indent_size = tab` 使用 `tab_width` |
| `end_of_line`              | `format.lineEnding`   | `lf`、`crlf`                              |
| `insert_final_newline`     | `format.finalNewline` | `true`、`false`                           |
| `max_line_length`          | `format.lineWidth`    | 40–320                                    |

Tab 优先使用 `tab_width`，空格优先使用数值 `indent_size`；首选值缺失或不支持时使用另一受支持数值作为回退。`unset` 移除继承属性。不支持的属性/值（包括 `charset`、`trim_trailing_whitespace`、`cr` 和 `max_line_length = off`）不改变 formatter，也不冒充有效来源。格式错误和读取失败会报错。gdfmt 始终要求 UTF-8，保留字面量和保护区域字节；EditorConfig 不授权语义或编码转换。

## 忽略规则与输入选择 {#ignore-rules-and-input-selection}

目录遍历与显式 `.gd` 路径均遵守 `.gitignore`、`.ignore`、`.gdfmtignore` 和仓库 `.git/info/exclude`。采用 gitignore 语法，支持 `!` 反向规则、`/` 锚定和目录后缀。同目录优先级：`.gdfmtignore` > `.ignore` > `.gitignore` > `.git/info/exclude`，子目录规则优先于祖先。非 Git 仓库同样遵守 `.gitignore`，禁用机器全局 excludes。兼容 Git worktree 标记文件，但不解析外部 Git common-directory excludes。忽略目录会被剪枝，其内部反向规则不能恢复后代。

始终排除 `.git`、`.godot`、`node_modules` 和 `target` 目录。遍历跳过符号链接，显式符号链接报错。结果稳定排序并去重。本地 ignore 过滤后，逐个候选验证 JSON/JSONC 配置，再独立应用最终 `ignore` 数组（gitignore 语法）。子目录 JSON/JSONC 的 `ignore: []` 可替换继承的配置数组，但不能取消本地 ignore 或内置排除；配置 ignore 在配置解析后过滤候选，不剪枝配置发现。

显式忽略文件会跳过；空发现（含全部忽略）退出码为 2。被忽略的 `--stdin-filepath` 同样返回 2，不输出格式化源码，也不创建虚拟文件。普通 `format -` 使用 cwd 下的虚拟 `stdin.gd` 做这些决策。`explain-config` 仍能检查忽略路径。文件/check/diff/stdin 共用最终布局与验证核心。

## 写入和当前限制 {#writes-and-current-limits}

首次替换前预检全部已选配置和源码读取。某个文件的语法/布局失败阻止该文件写入，其他有效文件仍可格式化。退出码：0 成功，1 语法/格式差异，2 参数/配置/I/O/内部失败，多文件取最大值。`check` 和 `--diff` 不写入。人类错误写 stderr；JSON 成功报告时只写 stdout，stderr 为空。

写入使用源码目录中的临时文件，保留权限，并在原子替换前复核原始文件。拒绝只读文件与符号链接，Unix 下还拒绝硬链接。替换以单文件为单位保持原子性，并发修改检测采用尽力复核，不提供多文件事务。强制终止进程可能遗留临时文件。

GDScript 引号转换和尾逗号选项尚未实现。`lint`、`rules` 和规则插件不属于 formatter 范围，仍报未知字段错误。

语言版本与 parser 的 `ParseOptions.language_version` 共用枚举。CLI 的 `lex` 只做词法检查，不运行版本 gates；`check`/formatter 使用最终语法 profile。profile 与完整引擎兼容性的区别见[语言版本](/zh/reference/language-versions)。
