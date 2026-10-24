# gdfmt

Rust GDScript formatter with validated output, Node.js APIs and a CLI.
Install the prerelease with `pnpm add @gdfmt/gdfmt@next`, or run the CLI with `pnpm dlx @gdfmt/gdfmt@next --check .`. See the [language support and limitations](https://github.com/ntnyq/gdfmt/blob/main/docs/reference/compatibility.md).

```js
import { format, formatSync, resolveConfig } from '@gdfmt/gdfmt'

const result = await format('var score=1\n', {
  language: { godotVersion: '4.7' },
  format: { lineWidth: 100, indentStyle: 'space', indentWidth: 2 },
})
if (result.output !== null) console.log(result.output)

// CommonJS exposes the same named functions via require('@gdfmt/gdfmt').
const immediate = formatSync('var score=1\n')
const resolution = await resolveConfig('src/player.gd')
if (!resolution.ignored) {
  const configured = await format('var score=1\n', {
    language: resolution.config.language,
    format: resolution.config.format,
  })
  if (configured.output !== null) console.log(configured.output)
}
```

`formatSync` runs in the caller; `format` runs parsing, formatting and validation
on a libuv worker. Both return `{ schemaVersion: 1, spanEncoding: 'utf8Bytes',
output, changed, diagnostics }`. Source errors and failed output invariants return
`output: null`; invalid options throw/reject. Spans are half-open **UTF-8 byte**
ranges, not JavaScript UTF-16 indices. Use `Buffer.from(source).subarray(start, end)`
to select the original bytes. Strings with unpaired surrogates are rejected before
conversion to Rust UTF-8. Omitted options use the same Rust defaults as the CLI;
unknown fields, explicit null, invalid ranges and non-JSON values are rejected.

The nested `options.format` object also accepts `arrayLayout`, `dictionaryLayout` and `enumLayout`
(`auto` / `multiline` / `preserve`), `dictionarySpacing` (`compact` / `spaced`),
`trailingCommaLayout` (`auto` / `multiline`), `blankLinesAroundDefinitions`
(`preserve` or 0–5), `lambdaBodyStyle` (`auto` / `multiline`) and
`inlineCommentSpacing` (1–4). Defaults preserve existing output. These options
change layout only; existing comma tokens, literal spellings and protected bytes
remain intact. See the [configuration guide](https://github.com/ntnyq/gdfmt/blob/main/docs/guide/configuration.md#layout-policies)
for scopes and precedence.

Text calls never discover configuration or write files. `resolveConfig` and
`resolveConfigSync` explicitly discover JSON/EditorConfig/ignore rules for an
existing or virtual path. They accept `{ cwd, config }`; `cwd` must be absolute
and defaults to the directory captured at call time. Inspect `ignored` before
formatting a resolved path. The CLI applies discovery, ignores and guarded writes
using the existing Rust runner, with identical flags, reporters and exit codes:

```sh
gdfmt --check src
gdfmt --diff src
gdfmt --stdin-filepath src/player.gd
gdfmt --reporter=json check src
```

Exit codes: 0 success, 1 source/format findings, 2 host/internal failures. The
export `@gdfmt/gdfmt/schema.json` is the configuration schema generated from Rust.

The native packages target Node **22.13+ on 22.x and 24.x** with Node-API 4:
macOS arm64/x64, Windows x64 MSVC, Linux x64 glibc/musl. Each root/platform package
has the same exact version. Install with optional dependencies enabled. Loading
fails clearly for missing, mismatched, corrupt or unsupported binaries; there is
no install script, compiler fallback, binary download or WASM fallback. The complete target/runtime matrix remains a release gate; see
[contributing](https://github.com/ntnyq/gdfmt/blob/main/CONTRIBUTING.md#nodejs-packages).
