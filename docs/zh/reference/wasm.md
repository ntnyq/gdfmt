# WebAssembly API

`@gdfmt/wasm` 是显式导入的浏览器 ESM 包，接受与 [Node API](/zh/reference/node) 相同的纯文本 `TextOptions`，返回相同的 `FormatResult`。初始化异步，格式化同步执行。

```js
import { createFormatter } from '@gdfmt/wasm'

const formatter = await createFormatter()
const result = formatter.formatSync('var value=1\n', {
  language: { godotVersion: '4.7' },
  format: { lineWidth: 100 },
})
if (result.output !== null) console.log(result.output)
```

运行 `pnpm add @gdfmt/wasm@next` 安装浏览器包；消费该包无需原生平台 addon 或
Rust。`next` 通道会移动，需要可复现构建时固定精确版本或提交 lockfile。

从仓库源码构建时，先运行 `pnpm install --frozen-lockfile`，再运行
`pnpm build:wasm`。构建需要 Node 22.x 的 22.13+ 或 Node 24.x、仓库声明的 pnpm
版本、`wasm-pack` 和 `wasm32-unknown-unknown` Rust 目标，详见
[构建准备](https://github.com/ntnyq/gdfmt/blob/main/CONTRIBUTING.md#wasm-and-editors)。

## 加载与打包器

默认加载器获取 ESM 入口旁的 `binding_bg.wasm`。部署完整 `dist`，为 WASM 设置 `application/wasm` MIME 类型；无需线程、共享内存或 COOP/COEP。跨源资源需要允许 CORS。

省略 `wasm` 或传入 `undefined` 时使用默认加载器。显式 `null` 是非法初始化参数，
会 reject，不会回退到默认加载。

`createFormatter({ wasm })` 接受字符串或 URL、`Response`、`ArrayBuffer`、`Uint8Array` 或预编译的 `WebAssembly.Module`。打包器移动资源时，应复制导出的 `@gdfmt/wasm/gdfmt.wasm`，使用打包器的资源 URL 语法显式传入地址。

Node 也可显式加载同一模块：

```js
import { readFile } from 'node:fs/promises'
import { createFormatter } from '@gdfmt/wasm'

const wasm = await readFile(
  new URL(import.meta.resolve('@gdfmt/wasm/gdfmt.wasm')),
)
const formatter = await createFormatter({ wasm })
console.log(formatter.formatSync('var value=1\n'))
```

## Web Worker

在 module Worker 内初始化格式化器，通过 `postMessage` 传递文本、选项与结果。宿主应携带请求 ID 并丢弃过期结果。

```js
// formatter-worker.js
import { createFormatter } from '@gdfmt/wasm'
const formatter = await createFormatter()
self.onmessage = ({ data: { id, source, options } }) => {
  try {
    self.postMessage({ id, result: formatter.formatSync(source, options) })
  } catch (error) {
    self.postMessage({ id, error: String(error) })
  }
}
self.postMessage({ ready: true })
```

收到 ready 消息后再提交任务。同步格式化会阻塞调用线程，较大输入建议交给 Worker。

## 配置辅助 API

具有文件系统适配器的宿主可以通过共享 Rust 实现解析 JSON/JSONC、合并已加载的配置层，与原生宿主保持相同的选项校验：

```js
const base = formatter.parseConfig('{/* shared */ "format":{"indentWidth":2}}')
const options = formatter.resolveTextOptions([
  { language: base.language ?? {}, format: base.format ?? {} },
  { format: { indentStyle: 'space', finalNewline: false } },
])
const result = formatter.formatSync('var value=1\n', options)
```

`parseConfig(source)` 校验不超过 1 MiB 的配置，返回可选的 `$schema`、`schemaVersion`、`root`、`extends`、`language` 和 `format` 字段，`schemaVersion` 存在时值为 1；拒绝 `overrides` 和 `ignore`（包括空数组）。`resolveTextOptions(layers)` 最多接受 128 层，逐层校验并按数组顺序合并，补齐默认值；它拒绝未展开的 `extends`，宿主需要先加载文件并移除发现元数据。显式的 `false` 和 `0` 会被保留并校验，不会当作缺失值。这两个方法均不读取文件或发现项目。

[VS Code Web 适配器](/zh/guide/integrations#vs-code-web) 通过 Worker 调用这些方法，并用 `vscode.workspace.fs` 加载配置。工作区和配置限制由该适配器负责，不属于纯文本 API。

## 契约与边界

非法参数抛错；源码错误返回 `output: null`、`changed: false` 和结构化 diagnostics。输出为 null 时不能产生编辑。成功输出仍需通过共享 Rust 核心的重解析、结构等价和幂等检查。诊断区间以 UTF-8 字节计数，不能直接当作 JavaScript UTF-16 位置。源码限制为 16 MiB。

实例拥有独立 WASM 内存，可以被垃圾回收。发生 trap 后应丢弃实例并重新调用 `createFormatter()`；初始化失败不影响后续新实例。

WASM 不提供文件系统、项目配置发现、文件加载、忽略文件选择或 EditorConfig API。浏览器、module Worker 和 VS Code Web Extension Host 在 Chromium 中验收，也测试 Node 的显式 WASM 调用。暂不承诺 Bun、Deno 或非 JS WASI 宿主支持。
