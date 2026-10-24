# @gdfmt/wasm

Single-threaded GDScript formatting for browsers and Web Workers. Uses the same
Rust configuration validation, reparsing, structure and idempotence checks as
`@gdfmt/gdfmt`, without native addons, filesystem APIs or shared memory.

Install with `pnpm add @gdfmt/wasm@latest`; Rust is only needed to build from source.
Pin an exact version or commit the lockfile when reproducibility matters.

```js
import { createFormatter } from '@gdfmt/wasm'

const formatter = await createFormatter()
const result = formatter.formatSync('var value=1\n', {
  language: { godotVersion: '4.7' },
  format: { lineWidth: 100 },
})
if (result.output !== null) console.log(result.output)
```

Initialization fetches the WASM file beside the ESM module. Serve `.wasm` as
`application/wasm`, or pass `{ wasm: url }`, bytes or a compiled module. No
COOP/COEP headers are needed. Deploy the entire `dist` folder. A bundler must copy
the exported `@gdfmt/wasm/gdfmt.wasm` asset and pass its URL if it relocates it.

Node uses explicit bytes:

```js
import { readFile } from 'node:fs/promises'
import { createFormatter } from '@gdfmt/wasm'

const wasm = await readFile(
  new URL(import.meta.resolve('@gdfmt/wasm/gdfmt.wasm')),
)
const formatter = await createFormatter({ wasm })
console.log(formatter.formatSync('var value=1\n'))
```

`formatSync` blocks its calling thread. For larger scripts, create a formatter
inside a module Worker; see `examples/`. Invalid arguments throw. Invalid source
returns diagnostics and `output: null`; never apply an edit in that case. Spans
are half-open UTF-8 byte offsets, including BOM and CRLF bytes. The input limit is
16 MiB. Each formatter owns its WASM instance and memory. After a WASM trap,
discard it and call `createFormatter()` again; initialization is also retryable.

`parseConfig(source)` parses and validates JSON/JSONC up to 1 MiB, returning optional
`$schema`, `schemaVersion: 1`, `root`, `extends`, `language` and `format` fields. It rejects `overrides` and
`ignore`, including empty arrays. `resolveTextOptions(layers)` merges at most 128
already loaded language/format layers in order and supplies Rust defaults. The
host must load relative `extends` itself and strip discovery metadata before
resolving; unresolved inheritance is an error. Explicit `false` and `0` survive
merging and receive normal validation.

This package does not discover projects, load files, select ignored inputs or
read EditorConfig. VS Code Web reuses it through a Worker with a workspace-bounded
virtual filesystem adapter; desktop uses the native package or CLI for full
filesystem semantics. Browser, module Worker and Web Extension Host acceptance
runs in Chromium; Node is also tested. Bun, Deno and non-JS WASI hosts are outside
the support promise.

From the repository: install `wasm-pack` and the `wasm32-unknown-unknown` target,
then run `pnpm build:wasm`. `pnpm test:wasm` packs and installs the real tarball,
compares shared fixtures against native output, and tests Chromium and a Worker.
Nothing is published by these commands.
