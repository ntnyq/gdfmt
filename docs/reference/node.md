# Node.js API

The `gdfmt` package exposes validated text formatting and explicit project configuration resolution, with ESM, CommonJS and TypeScript declarations.

The package is not yet published. To prepare local packages, follow [contributing](https://github.com/ntnyq/gdfmt/blob/main/CONTRIBUTING.md#nodejs-packages). The examples below assume the root package and matching native package are installed.

## Format text

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

CommonJS exposes the same named functions through `require('gdfmt')`.

| API                                 | Behavior                                                             |
| ----------------------------------- | -------------------------------------------------------------------- |
| `formatSync(source, options?)`      | Validate and format text in the calling thread                       |
| `format(source, options?)`          | Return the same result through a Promise, running core work on libuv |
| `resolveConfigSync(path, options?)` | Resolve filesystem configuration synchronously                       |
| `resolveConfig(path, options?)`     | Resolve configuration through a libuv worker                         |

Text options accept `language`, `format` and `sourceId`. Layout defaults and ranges match [configuration](/guide/configuration). Text calls never discover project settings or write files.

Invalid options throw or reject, including unknown fields, `null`, non-JSON values and unpaired Unicode surrogates. Source errors and invariant failures return `output: null`, `changed: false` and structured diagnostics. Successful results contain validated `output` and `changed`. Results use `schemaVersion: 1` and `spanEncoding: 'utf8Bytes'`.

## Resolve project settings

```js
import { format, resolveConfig } from 'gdfmt'

const resolution = await resolveConfig('src/player.gd')
if (!resolution.ignored) {
  const result = await format(source, {
    language: resolution.config.language,
    format: resolution.config.format,
  })
  // Use result.output only when it is non-null.
}
```

Here `source` is the file or editor-buffer text you have already read. Resolver options are `{ cwd, config }`: `cwd` must be absolute and defaults to the working directory captured at call time; `config` is an optional explicit local JSON file. Results include `config`, applied `files`, `editorconfigFiles`, effective field `sources` and `ignored`.

Inspect `ignored` yourself and pass only `config.language` and `config.format` to a text call. Filesystem selection and writing remain the caller's responsibility.

## Diagnostic offsets

Spans are half-open UTF-8 byte offsets, including BOM and physical CRLF bytes. They are not JavaScript string indices:

```js
const bytes = Buffer.from(source)
const selected = bytes.subarray(span.start, span.end)
```

Here `span` is a diagnostic's source range. Convert offsets to UTF-16 explicitly when integrating with an editor.

## Runtime and native packages

The package declares Node **22.13+ on 22.x**, or **24.x**, using Node-API 4.

| Platform package        | Rust target                 |
| ----------------------- | --------------------------- |
| `@gdfmt/darwin-arm64`   | `aarch64-apple-darwin`      |
| `@gdfmt/darwin-x64`     | `x86_64-apple-darwin`       |
| `@gdfmt/win32-x64-msvc` | `x86_64-pc-windows-msvc`    |
| `@gdfmt/linux-x64-gnu`  | `x86_64-unknown-linux-gnu`  |
| `@gdfmt/linux-x64-musl` | `x86_64-unknown-linux-musl` |

The [recorded artifact matrix](https://github.com/ntnyq/gdfmt/blob/main/tests/npm/acceptance.json) covers these five targets on Node 22.13.0 and 24.0.0. Each release must pass the complete matrix for its own artifacts. Keep optional dependencies enabled: the loader checks OS, CPU, libc, package version and compiled target. Missing, mismatched, corrupt or unsupported native packages produce an error. There is no install-time compilation or additional binary download. Linux glibc artifacts target Ubuntu 24.04; older glibc versions are not validated.

The package's `gdfmt` executable uses the same Rust runner as the standalone [CLI](/reference/cli). `gdfmt/schema.json` exports the configuration JSON Schema.
