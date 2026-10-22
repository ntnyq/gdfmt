# WASM host decision

Decision date: 2026-10-22. Keep one shipped WASM implementation using
`wasm-bindgen = 0.2.114` and `wasm32-unknown-unknown`. Both it and NAPI call
`gdfmt_config::text`; filesystem and CLI adapters stay outside that graph.

The architecture follows Oxc's shared Rust core and thin host boundaries. Oxc's
[parser manifest](https://github.com/oxc-project/oxc/blob/main/napi/parser/package.json)
and [browser entry](https://github.com/oxc-project/oxc/blob/main/napi/parser/src-js/wasm.js)
also show a non-threaded WASI route. However, the current
[napi-rs default workflow](https://napi.rs/docs/concepts/webassembly) documents
the threaded target and additional adaptation for non-threaded targets.

A throwaway text-only NAPI probe used the repository's napi 3.14.2,
napi-derive 3.6.12 and napi-build 2.6.0, with Rust 1.98.0 and `wasm32-wasip1`.
Plain Cargo compilation reached napi-build and failed with
`EMNAPI_LINK_DIR must be set`. This establishes that the existing plain Cargo
build is insufficient; it does not establish that NAPI single-threaded WASI is
impossible. Adopting it would additionally require the NAPI CLI, emnapi linkage,
runtime and browser loader acceptance. Those components provide little benefit
for the deliberately small text/JSON boundary here.

The selected wasm-bindgen artifact runs on an ordinary static HTTP page and in a
module Worker without COOP/COEP, native addons, WASI imports or shared memory.
Builds use the repository Rust toolchain, while workspace and WASM compile checks
also preserve Rust 1.88. Keep the bindgen library pin synchronized with its CLI;
wasm-pack selects the matching CLI. The generator output is wrapped in a factory
with explicit shape assertions so each formatter gets independent memory.

Local Chromium acceptance from the installed tarball compared 1,465 fixture and
language-profile cases against native output, including exact diagnostics. It
also checked malformed options, Unicode, CRLF and a source over the 16 MiB limit.
With JSONC parsing and configuration helpers, the artifact measured 543,735 WASM
bytes (193,428 gzip), with 10,207 bytes of binding glue. One local run measured
about 0.7 ms Node initialization, 8.8 ms browser initialization, and 107 ms for
all browser cases.
These are observations, not performance budgets. `target/wasm/acceptance.json`
records the current measurement, hashes, revision and whether source was clean.

Each instance can be replaced after a trap. Initialization failures and normal
option errors must not poison later instances. Catastrophic browser out-of-memory
termination is a host failure, not a recoverable formatting result. No continuous
memory budget, Bun/Deno acceptance or general WASI ABI is claimed.

Desktop editors use the native `gdfmt --lsp` server, following the
[Oxfmt editor model](https://oxc.rs/docs/guide/usage/formatter/editors.html).
VS Code Web now reuses `@gdfmt/wasm` in a dedicated Worker. Shared Rust helpers
parse JSON/JSONC and merge explicit layers; the VS Code adapter loads virtual
files, resolves relative inheritance within each workspace boundary and rejects
unsupported overrides, ignore fields and EditorConfig. It bounds queued work,
discards stale edits and replaces failed workers. A separate browser-only VSIX
and real Web Extension Host test run without cross-origin isolation. See the
[Web configuration contract](../guide/integrations.md#vs-code-web); desktop keeps
its native LSP and full CLI discovery behavior.
