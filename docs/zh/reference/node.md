# Node.js API

`gdfmt` 包提供经过验证的文本格式化与显式项目配置解析，支持 ESM、CommonJS 和 TypeScript 声明。

npm 包尚未发布。本地产物准备见[贡献指南](https://github.com/ntnyq/gdfmt/blob/main/CONTRIBUTING.md#nodejs-packages)。以下示例假设已安装主包与对应的原生平台包。

## 格式化文本

```js
import { format, formatSync } from 'gdfmt'

const source = 'var count=1\n'
const result = await format(source, {
  language: { godotVersion: '4.5' },
  format: { lineWidth: 100, indentStyle: 'tab' },
})

if (result.output === null) {
  console.error(result.diagnostics)
} else {
  console.log(result.output)
}

const syncResult = formatSync(source)
```

CommonJS 通过 `require('gdfmt')` 提供相同的具名函数。

| API                                 | 行为                                            |
| ----------------------------------- | ----------------------------------------------- |
| `formatSync(source, options?)`      | 在调用线程验证并格式化文本                      |
| `format(source, options?)`          | 返回相同结果的 Promise，核心工作在 libuv 上运行 |
| `resolveConfigSync(path, options?)` | 同步解析文件系统配置                            |
| `resolveConfig(path, options?)`     | 通过 libuv worker 解析配置                      |

文本选项接受 `language`、`format` 与 `sourceId`，布局默认值与范围和[配置](/zh/guide/configuration)一致。文本调用不发现项目配置，也不写文件。

无效选项会抛出异常或 reject，包括未知字段、`null`、非 JSON 值和未配对的 Unicode surrogate。源码错误或内部验证失败返回 `output: null`、`changed: false` 和结构化诊断；成功时包含经过验证的 `output` 与 `changed`。结果使用 `schemaVersion: 1` 和 `spanEncoding: 'utf8Bytes'`。

## 解析项目配置

```js
import { format, resolveConfig } from 'gdfmt'

const resolution = await resolveConfig('src/player.gd')
if (!resolution.ignored) {
  const result = await format(source, {
    language: resolution.config.language,
    format: resolution.config.format,
  })
  // 仅当 result.output 非 null 时使用它。
}
```

这里的 `source` 是已读取的文件或编辑器缓冲区文本。解析选项为 `{ cwd, config }`：`cwd` 必须是绝对路径，默认取调用时的工作目录；`config` 可指定本地 JSON 配置。结果包含 `config`、已应用的 `files`、`editorconfigFiles`、字段有效来源 `sources` 和 `ignored`。

调用方应自行检查 `ignored`，只将 `config.language` 和 `config.format` 传给文本 API。文件选择与写入由调用方负责。

## 诊断偏移

span 为半开 UTF-8 字节偏移，包括 BOM 与实际 CRLF 字节，不是 JavaScript 字符串索引：

```js
const bytes = Buffer.from(source)
const selected = bytes.subarray(span.start, span.end)
```

这里的 `span` 是诊断的源码范围。编辑器集成需要显式转换为 UTF-16 偏移。

## 运行时与原生包

包声明的运行时为 **Node 22.x 的 22.13+** 或 **24.x**，使用 Node-API 4。

| 平台包                  | Rust target                 |
| ----------------------- | --------------------------- |
| `@gdfmt/darwin-arm64`   | `aarch64-apple-darwin`      |
| `@gdfmt/darwin-x64`     | `x86_64-apple-darwin`       |
| `@gdfmt/win32-x64-msvc` | `x86_64-pc-windows-msvc`    |
| `@gdfmt/linux-x64-gnu`  | `x86_64-unknown-linux-gnu`  |
| `@gdfmt/linux-x64-musl` | `x86_64-unknown-linux-musl` |

[已记录的产物矩阵](https://github.com/ntnyq/gdfmt/blob/main/tests/npm/acceptance.json)覆盖以上五个目标与 Node 22.13.0、24.0.0。每次发布仍须验证当次产物的完整矩阵。请保留 optional dependencies：加载器检查 OS、CPU、libc、包版本与编译 target，缺失、错配、损坏或不支持的原生产物会报错。安装时不编译，也不额外下载二进制。Linux glibc 产物基于 Ubuntu 24.04，未验证更旧 glibc。

包中的 `gdfmt` 可执行入口与独立 [CLI](/zh/reference/cli) 使用同一个 Rust runner。`gdfmt/schema.json` 导出配置 JSON Schema。
