# CI and editor workflows

Use the same gdfmt version and committed configuration locally and in CI. The commands below assume `gdfmt` is on `PATH`; see the [quickstart](/guide/) for source installation.

## Continuous integration

Run a read-only formatting check after checkout and installation:

```sh
gdfmt --check .
```

A nonzero exit fails the step. To inspect the changes locally, run `gdfmt --diff .`; to apply them, run `gdfmt .`. See [exit codes](/reference/cli#exit-codes) to distinguish formatting differences from configuration or I/O failures.

For a source-based CI installation, pin a reviewed Git revision. For example,
in a CI shell step with Rust installed:

```sh
# Set GDFMT_REVISION to a full commit SHA you have reviewed.
cargo install --git https://github.com/ntnyq/gdfmt.git --rev "$GDFMT_REVISION" --locked gdfmt_cli
gdfmt --check .
```

Pin the revision in your CI configuration and update it deliberately when adopting formatter changes.

For npm-based CI, commit the dependency and lockfile, run
`pnpm install --frozen-lockfile`, then `pnpm exec gdfmt --check .`. Avoid resolving
the moving `next` tag on every CI run.

## Editor commands

For an editor that supports external formatters over stdin, use:

```sh
gdfmt --stdin-filepath path/to/current-file.gd
```

Pass the buffer to stdin, use stdout as the replacement text only on exit `0`, and show diagnostics on failure. The virtual path selects the file's configuration, EditorConfig, overrides and ignore rules without writing the file. Run from the project directory, or pass an absolute virtual path.

## VS Code

Install a matching native VSIX from the project's [GitHub Releases](https://github.com/ntnyq/gdfmt/releases),
or build one with `pnpm package:vscode`. Use **Extensions: Install from VSIX**.
The extension bundles its native formatter and needs no separate Node installation.
For SSH, WSL and containers, connect first and install in that remote window for
the remote extension host's platform.

Source builds require the pinned Rust toolchain, Node 22.13+ on 22.x or Node 24.x,
and the pnpm version in `package.json`. Run `pnpm install --frozen-lockfile` first.
See [build setup](https://github.com/ntnyq/gdfmt/blob/main/CONTRIBUTING.md#wasm-and-editors).
Choose the native VSIX for the extension host's OS/CPU/libc; a WSL, SSH or container
host needs its own matching target. Outputs are under `target/editors/TARGET/`.

The 0.1.0 recommendation is to deliver frozen VSIX assets with the core release
and treat Marketplace stable publication as a separate follow-up. As checked on
2026-10-23, [Marketplace](https://marketplace.visualstudio.com/items?itemName=ntnyq.gdfmt)
already has extension `0.1.0` as a prerelease with core `0.1.0-alpha.6`. Select
**Install Pre-Release Version** for that preview; it is not the final core 0.1.0.
The [rc.1 GitHub Release](https://github.com/ntnyq/gdfmt/releases/tag/v0.1.0-rc.1)
provides six frozen VSIX targets, with numeric extension version `0.2.0` and core
`0.1.0-rc.1`, marked as prerelease. This release did not upload to Marketplace.
Recheck Gallery availability before a later stable extension upload; see the
[distribution review](../research/0.1.0-editor-distribution-review.md).

VSIX filenames use the core version, while the Extensions view shows the numeric
extension version. Replace `CORE_VERSION` and select the correct suffix:

| Host             | VSIX suffix           | Gallery target |
| ---------------- | --------------------- | -------------- |
| macOS ARM64      | `darwin-arm64.vsix`   | `darwin-arm64` |
| macOS x64        | `darwin-x64.vsix`     | `darwin-x64`   |
| Windows x64 MSVC | `win32-x64-msvc.vsix` | `win32-x64`    |
| Linux x64 glibc  | `linux-x64-gnu.vsix`  | `linux-x64`    |
| Linux x64 musl   | `linux-x64-musl.vsix` | `alpine-x64`   |
| Browser          | `web.vsix`            | `web`          |

For a downloaded native asset, desktop CLI installation is also available:

```sh
code --install-extension ./gdfmt-CORE_VERSION-darwin-arm64.vsix
```

Use the filename matching your host. VSIX installations default to manual updates;
see [VS Code's installation guide](https://code.visualstudio.com/docs/configure/extensions/extension-marketplace#install-from-a-vsix).
Download the newer asset to update, or deliberately reinstall an older asset to
roll back. If intentionally replacing the same numeric version via CLI, add
`--force`. Use **gdfmt: Show Version** to confirm the core after installation.
Native and Web packages share `ntnyq.gdfmt`; select the appropriate variant for
each extension host.

```json
{
  "[gdscript]": {
    "editor.defaultFormatter": "ntnyq.gdfmt",
    "editor.formatOnSave": true
  }
}
```

The formatter uses unsaved buffer text. Invalid source produces diagnostics and no edits. Godot Tools can remain enabled for its language and engine features; select gdfmt as the default formatter explicitly.

VS Code buffers use one line ending throughout. If the core output requires mixed
line endings to retain formatter-off regions or multiline strings, desktop and Web
formatting report an error and leave the document unchanged. Setting
`lineEnding: "preserve"` can avoid this when the input has uniform line endings;
it does not guarantee uniform output for already mixed input. Use the CLI when
exact mixed output must be retained. VS Code may normalize mixed endings when
loading a file, before the formatter receives the buffer. When retained text determines a
uniform ending, that output takes precedence over the configured structural EOL.

Settings: `gdfmt.enable`, `gdfmt.serverPath` and `gdfmt.configPath`. Relative paths resolve from each workspace folder. Program selection is an explicit path, then a trusted workspace's `@gdfmt/gdfmt`, then the bundled executable. A selected program must match the extension's core version exactly. Workspace scripts use the extension host's Node runtime and must satisfy the npm package's Node requirement. Untrusted workspaces use only the bundled program and normal configuration discovery.

Commands: **gdfmt: Restart Server**, **gdfmt: Show Output**, **gdfmt: Show Version**. Multi-root folders have separate clients. Untitled documents use formatter defaults rather than project configuration. Their desktop session honors a trusted global `gdfmt.serverPath` when set, otherwise using the bundled program; save them into a project to apply project configuration. Use the separate Web VSIX below for browser and virtual workspaces.

### VS Code Web

Web source builds additionally require wasm-pack and the `wasm32-unknown-unknown`
Rust target. Run `pnpm package:vscode:web` to build
`target/editors/web/gdfmt-CORE_VERSION-web.vsix`. `CORE_VERSION` is the exact
`gdfmtVersion`, including any prerelease suffix; it is independent of the numeric
Marketplace extension version. Building a VSIX does not publish it to Marketplace.
The tested browser-only VS Code 1.96.4 does not expose the local VSIX installation
action without a local/remote extension-management server. Use the existing Gallery
prerelease, the staged development host via `pnpm test:vscode:web`, or
[official developer sideloading](https://code.visualstudio.com/api/extension-guides/web-extensions#test-your-web-extension-in-vscode-dev).
A Web host that supports VSIX installation can use the Web asset; do not infer
support merely from a successful development-host test. Use the same default
formatter/save settings above. Web bundles `@gdfmt/wasm` in a dedicated Worker;
no native executable, Node runtime or cross-origin isolation headers are required.
`gdfmt.serverPath` applies only to desktop. **Restart Server** recreates the Web Worker.

Web supports virtual workspace URIs through `vscode.workspace.fs`, unsaved buffers,
save formatting, undo, multiple workspace folders and UTF-16 diagnostics. Each
request uses the owning folder's configuration. Untitled documents use defaults.
Documents outside a workspace folder must be opened in a folder first.

Configuration discovery walks from the document directory to its owning workspace
folder, preferring `.gdfmtrc.json` over `.gdfmtrc.jsonc` at each level. Ancestor layers
are merged before child layers; `root: true` stops discovery. JSONC comments and
relative `extends` are supported using the shared Rust parser and option validation.
`gdfmt.configPath` is relative to the owning workspace folder and replaces discovery.
All paths, including extended configurations, must stay inside that folder and use
`/` separators. Nested workspace folders form their own configuration boundary.

Web currently rejects any configuration containing `overrides` or `ignore`, even
empty arrays, and rejects discovered `.editorconfig` files. It reports an error and
returns no edits. Use supported JSON/JSONC options or the desktop extension for these
features. Web formats an explicitly opened document: `.gitignore` and `.gdfmtignore`
do not select or exclude it. These Web configuration restrictions apply in trusted
and untrusted workspaces. The packaged Web extension allows workspace-relative
`gdfmt.configPath` in both; it remains bounded by the owning workspace. Desktop
restricts workspace-defined `configPath` and custom executables until trusted.

Configuration is bounded to 1 MiB per file, 16 MiB total, 32 inheritance levels and
128 merged layers; discovery is limited to 128 directories. Source is limited to
16 MiB. Cancelled or outdated requests cannot apply edits. A failed or timed-out
Worker is discarded and the next request starts a new one.

## Language server

```sh
gdfmt --lsp
```

The formatting-only server communicates over stdio using LSP 3.17. It advertises full document synchronization, UTF-16 positions and document formatting. It does not advertise range formatting. Range-formatting requests return JSON-RPC `-32601` (method not supported); neither VS Code host registers a range provider, even for a selection covering the whole document. Use Format Document instead. Editor tab size and insert-spaces preferences do not override gdfmt configuration. It reads current project configuration for each request, including inherited JSON/JSONC, overrides, EditorConfig and ignores. Source errors return no edits and publish diagnostics; configuration failures return an LSP error. The server never writes documents.

The VS Code client uses the `gdfmt/formatting` extension request with the same
`textDocument` identifier. Its result contains `output` (formatted text, or `null`
for invalid or ignored source) and the resolved `lineEnding` (`lf`, `crlf` or
`preserve`). This lets the client apply text and buffer EOL changes together.
The capability is advertised as `experimental.gdfmtTextFormattingProvider`;
cancellation, invalidation and diagnostics follow the standard formatting request.

Pass an optional `configPath` in `initializationOptions`, or use `gdfmt --lsp --config path/to/config.json`. Send `workspace/didChangeConfiguration` with `{ "settings": { "gdfmt": { "configPath": "..." } } }` to replace the explicit path. An empty path restores discovery. Cancellation responds immediately; document or configuration updates invalidate pending results. A running formatter pass finishes in the background, and cancelled queued work is skipped.

The [Node.js API](/reference/node) remains available for custom host integrations, and the [WASM API](/reference/wasm) supports browsers and Workers.

## Ignore generated and third-party code

Create `.gdfmtignore` at the project root using gitignore-style patterns:

```text
generated/
addons/vendor/
```

Project `.gitignore` rules also apply. Built-in exclusions include Godot's `.godot` cache. See [file selection](/guide/configuration#ignore-rules-and-input-selection) for precedence and explicit-file behavior.

To preserve only part of a script, use [protected regions](/reference/formatting#protected-regions):

```gdscript
# gdfmt: formatter-off
var table=[ 1,    2,3 ]
# gdfmt: formatter-on
```

## Git hooks

A simple pre-commit hook can check selected project directories:

```sh
gdfmt --check src addons
```

Use directories that exist in your project. The command checks those directories in the working tree; staged-file integrations should pass explicit `.gd` paths and handle an empty selection before invoking gdfmt, because no matching inputs is an error.
