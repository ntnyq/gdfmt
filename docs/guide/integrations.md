# CI and editor workflows

Use the same gdfmt version and committed configuration locally and in CI. The commands below assume `gdfmt` is on `PATH`; see the [quickstart](/guide/) for source installation.

## Continuous integration

Run a read-only formatting check after checkout and installation:

```sh
gdfmt --check .
```

A nonzero exit fails the step. To inspect the changes locally, run `gdfmt --diff .`; to apply them, run `gdfmt .`. See [exit codes](/reference/cli#exit-codes) to distinguish formatting differences from configuration or I/O failures.

Until published packages are available, install from a reviewed, fixed Git revision. For example, in a CI shell step with Rust installed:

```sh
# Set GDFMT_REVISION to a full commit SHA you have reviewed.
cargo install --git https://github.com/ntnyq/gdfmt.git --rev "$GDFMT_REVISION" --locked gdfmt_cli
gdfmt --check .
```

Pin the revision in your CI configuration and update it deliberately when adopting formatter changes.

## Editor commands

For an editor that supports external formatters over stdin, use:

```sh
gdfmt --stdin-filepath path/to/current-file.gd
```

Pass the buffer to stdin, use stdout as the replacement text only on exit `0`, and show diagnostics on failure. The virtual path selects the file's configuration, EditorConfig, overrides and ignore rules without writing the file. Run from the project directory, or pass an absolute virtual path.

## VS Code

Build a platform VSIX from this repository with `pnpm package:vscode`, then use **Extensions: Install from VSIX**. The extension bundles a matching native formatter; a separate Node installation is unnecessary. It runs in the workspace extension host for SSH, WSL and containers.

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
formatting report an error and leave the document unchanged. Use
`lineEnding: "preserve"` or the CLI to keep the exact output. When retained text determines a
uniform ending, that output takes precedence over the configured structural EOL.

Settings: `gdfmt.enable`, `gdfmt.serverPath` and `gdfmt.configPath`. Relative paths resolve from each workspace folder. Program selection is an explicit path, then a trusted workspace's `@gdfmt/gdfmt`, then the bundled executable. A selected program must match the extension's core version exactly. Workspace scripts use the extension host's Node runtime and must satisfy the npm package's Node requirement. Untrusted workspaces use only the bundled program and normal configuration discovery.

Commands: **gdfmt: Restart Server**, **gdfmt: Show Output**, **gdfmt: Show Version**. Multi-root folders have separate clients. Untitled documents use the bundled program and formatter defaults; save them into a project to apply project configuration. Use the separate Web VSIX below for browser and virtual workspaces.

### VS Code Web

Run `pnpm package:vscode:web` to build `target/editors/web/gdfmt-VERSION-web.vsix`.
Install that VSIX in VS Code Web and use the same default formatter and save settings
shown above. It bundles the existing `@gdfmt/wasm` formatter in a dedicated Worker;
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
do not select or exclude it. These Web rules apply in trusted and untrusted workspaces.

Configuration is bounded to 1 MiB per file, 16 MiB total, 32 inheritance levels and
128 merged layers; discovery is limited to 128 directories. Source is limited to
16 MiB. Cancelled or outdated requests cannot apply edits. A failed or timed-out
Worker is discarded and the next request starts a new one.

## Language server

```sh
gdfmt --lsp
```

The formatting-only server communicates over stdio using LSP 3.17. It advertises full document synchronization, UTF-16 positions and document formatting. It does not advertise range formatting. It reads current project configuration for each request, including inherited JSON/JSONC, overrides, EditorConfig and ignores. Source errors return no edits and publish diagnostics; configuration failures return an LSP error. The server never writes documents.

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
