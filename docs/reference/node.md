# Node.js and npm artifacts

The P7 implementation provides locally prepared npm artifacts; nothing has been published. Names and publishing permissions remain unverified. See the [acceptance record](/development/formatter-acceptance#p7-node-acceptance) for actual execution evidence.

## Text and configuration APIs

Both `import { format } from 'gdfmt'` and `const { format } = require('gdfmt')` expose the same four functions:

| API                                 | Contract                                                   |
| ----------------------------------- | ---------------------------------------------------------- |
| `formatSync(source, options?)`      | Validated formatting in the calling thread                 |
| `format(source, options?)`          | The same result through a Promise; core work runs on libuv |
| `resolveConfigSync(path, options?)` | Explicit filesystem configuration resolution               |
| `resolveConfig(path, options?)`     | Configuration resolution through a libuv worker            |

Text options are `{ language: { godotVersion }, format: { ...layoutOptions }, sourceId }`. Defaults, language versions and layout ranges match Rust and the [configuration schema](/guide/configuration). Unknown fields, explicit null, non-JSON values and unpaired Unicode surrogates are rejected. Invalid options throw/reject; source errors and invariant failures return `{ schemaVersion: 1, spanEncoding: 'utf8Bytes', output: null, changed: false, diagnostics }`. Successful results include validated `output` and `changed`.

Diagnostic spans are half-open UTF-8 byte offsets, including BOM and physical CRLF bytes. They are not JavaScript string indices. Select the original bytes with `Buffer.from(source).subarray(span.start, span.end)`. Convert to UTF-16 explicitly when integrating with an editor.

Text APIs never discover configuration or write files. Resolver options are `{ cwd, config }`: `cwd` is absolute and defaults to the directory captured at call time; `config` is an optional explicit local JSON file. Results contain `config`, applied `files`, `editorconfigFiles`, field `sources` and `ignored`. Pass only `resolution.config.language` and `resolution.config.format` into text calls and inspect `ignored` yourself.

The npm `gdfmt` executable invokes the same Rust runner as the standalone CLI, including stdout/stderr, stdin, reporters, discovery, ignore rules, guarded writes and exit-code priority. `gdfmt/schema.json` exports the generated schema.

## Proposed target matrix

Node runtime ranges are `^22.13.0 || ^24.0.0`, using Node-API 4. CI pins **22.13.0** and **24.0.0** on each target. Other Node major versions and unlisted architectures are not declared.

| Platform package        | Rust target                 |
| ----------------------- | --------------------------- |
| `@gdfmt/darwin-arm64`   | `aarch64-apple-darwin`      |
| `@gdfmt/darwin-x64`     | `x86_64-apple-darwin`       |
| `@gdfmt/win32-x64-msvc` | `x86_64-pc-windows-msvc`    |
| `@gdfmt/linux-x64-gnu`  | `x86_64-unknown-linux-gnu`  |
| `@gdfmt/linux-x64-musl` | `x86_64-unknown-linux-musl` |

The root package pins all five optional dependencies to its exact version. The loader selects OS/CPU/libc and verifies the platform manifest plus the compiled version/target. Missing, wrong-version, unsupported or corrupt native packages fail clearly. Installation has no lifecycle scripts, compiler fallback or additional binary download. Keep optional dependencies enabled. Linux glibc artifacts are built on Ubuntu 24.04; older glibc systems are not validated. macOS/Windows compatibility is limited to the CI runners until broader OS testing exists.

## Local artifact gates and recovery

Repository scripts are TypeScript executed by `tsx`:

```sh
pnpm install --frozen-lockfile
pnpm typecheck
pnpm test:npm
pnpm verify:npm --local
```

`pnpm test:npm [rust-target]` builds release bindings and the standalone CLI, packs the root and platform packages, and installs the root tarball through an isolated local registry with an empty cache. The consumer PATH contains only Node. Vitest TypeScript tests compare all 262 script fixtures across five profiles, then verify Unicode/protected bytes, options, config, CLI, worker scheduling and loader errors. ESM/CJS `.mts`/`.cts` consumers typecheck against the installed export map. Native builds require Rust; consuming tarballs does not.

`.github/workflows/npm.yml` runs the ten target/runtime combinations, including Alpine musl, and uploads tarballs and receipts. `pnpm verify:npm` requires all ten receipts and byte-identical root tarballs; it verifies exact platform versions and produces SHA-512 hashes. `--local` verifies only the current target/runtime and explicitly records an incomplete matrix. Reports and packages live under ignored `target/npm/`; CI matrix downloads live under `target/npm-ci/`.

There is deliberately no publication job. Before any future release, verify name ownership/permissions, collect the complete matrix and freeze those exact tarballs and hashes. Publish platform packages first, verify their registry versions/integrities, then publish the root package and update its tag last. If interrupted, inspect the registry and retry only missing packages with the same immutable version/bytes; do not rebuild or overwrite an existing version. If an existing integrity differs, stop that release and use a new version. A published root with a missing selected platform fails clearly; finish the missing immutable platform upload or move the root tag back to the last complete release. Do not promote a partial matrix. Actual publication and registry recovery remain untested and require a separate release task.
