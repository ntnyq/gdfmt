# WebAssembly API

`@gdfmt/wasm` is an explicit browser ESM package. It accepts the same pure
`TextOptions` and returns the same `FormatResult` as the [Node API](/reference/node).
Initialization is asynchronous; formatting is synchronous.

```js
import { createFormatter } from '@gdfmt/wasm'

const formatter = await createFormatter()
const result = formatter.formatSync('var value=1\n', {
  language: { godotVersion: '4.7' },
  format: { lineWidth: 100 },
})
if (result.output !== null) console.log(result.output)
```

Build from source with `pnpm build:wasm` until the first WASM package is released.
Building requires `wasm-pack` and the `wasm32-unknown-unknown` Rust target.

## Loading and bundlers

The default loader fetches `binding_bg.wasm` beside the ESM entry. Deploy all of
`dist` and serve WASM with the `application/wasm` MIME type. No threads, shared
memory or COOP/COEP headers are required. A cross-origin asset must allow CORS.

`createFormatter({ wasm })` accepts a string or URL, a `Response`, an
`ArrayBuffer`, a `Uint8Array`, or a compiled `WebAssembly.Module`. Bundlers that
relocate assets should copy the exported `@gdfmt/wasm/gdfmt.wasm` file and pass its
URL explicitly, using their own asset URL syntax.

Node can load the same module explicitly:

```js
import { readFile } from 'node:fs/promises'
import { createFormatter } from '@gdfmt/wasm'

const wasm = await readFile(
  new URL(import.meta.resolve('@gdfmt/wasm/gdfmt.wasm')),
)
const formatter = await createFormatter({ wasm })
console.log(formatter.formatSync('var value=1\n'))
```

## Web Workers

Create the formatter inside a module Worker. Exchange source/options and results
using `postMessage`; keep request IDs to discard outdated results in the host.

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

Wait for the ready message before submitting work. Formatting blocks the calling
thread, so a Worker is recommended for larger inputs.

## Configuration helpers

Hosts with a filesystem adapter can parse JSON/JSONC and merge already loaded
layers with the same Rust configuration implementation used by native hosts:

```js
const base = formatter.parseConfig('{/* shared */ "format":{"indentWidth":2}}')
const options = formatter.resolveTextOptions([
  { language: base.language ?? {}, format: base.format ?? {} },
  { format: { indentStyle: 'space', finalNewline: false } },
])
const result = formatter.formatSync('var value=1\n', options)
```

`parseConfig(source)` validates a configuration up to 1 MiB and returns optional
`root`, `extends`, `language` and `format` fields. It rejects `overrides` and `ignore`,
including empty arrays. `resolveTextOptions(layers)` accepts at most 128 layers,
validates them, merges in array order and supplies defaults. It rejects unresolved
`extends`; the host must load those files and remove discovery metadata first.
Explicit `false` and `0` values are preserved and validated rather than treated as
missing. Neither method reads files or discovers projects.

The [VS Code Web adapter](/guide/integrations#vs-code-web) uses these helpers through
a Worker and loads configurations with `vscode.workspace.fs`. Its workspace and
configuration restrictions belong to the adapter, not the pure text API.

## Contract and limits

Invalid parameters throw. Source errors return `output: null`, `changed: false`
and structured diagnostics. Never edit a document when output is null. Every
successful output still passes the shared Rust validation checks. Diagnostic
spans are UTF-8 bytes, not JavaScript UTF-16 positions. The source limit is 16 MiB.

Instances own separate WASM memories and can be garbage collected. After a trap,
discard the instance and call `createFormatter()` again. A failed initialization
does not poison later instances.

WASM has no filesystem, project discovery, file loading, ignore selection or
EditorConfig API. Browser, module Worker and VS Code Web Extension Host acceptance
runs in Chromium; Node also has explicit WASM tests. Bun, Deno and non-JS WASI
hosts are outside the current support promise.
