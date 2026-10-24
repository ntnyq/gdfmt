# gdfmt for VS Code

Validated GDScript formatting with a bundled Rust language server. No separate
Node installation is required. Supports desktop VS Code, SSH, WSL and containers
on the published native targets. A separate Web VSIX uses the same validated
Rust core through the existing WASM package in a Worker.

Source builds require the pinned Rust toolchain, Node 22.13+ on 22.x or Node 24.x,
and the pnpm version in the root `package.json`. Run
`pnpm install --frozen-lockfile` before building. Web builds also need wasm-pack
and the `wasm32-unknown-unknown` target; see
[build setup](https://github.com/ntnyq/gdfmt/blob/main/CONTRIBUTING.md#wasm-and-editors).
Choose a native VSIX for the extension host's OS/CPU/libc, including remote hosts.

Build a local VSIX with `pnpm package:vscode` in the repository root, then run
**Extensions: Install from VSIX**. Select gdfmt explicitly when Godot Tools or
another formatter is installed:

```json
{
  "[gdscript]": {
    "editor.defaultFormatter": "ntnyq.gdfmt",
    "editor.formatOnSave": true
  }
}
```

The extension formats the current buffer, including unsaved changes. Source
errors produce diagnostics and no edits. Each workspace folder has a separate
desktop client and configuration. On desktop, project configuration, inheritance, overrides,
EditorConfig and ignore behavior match the CLI. Untitled documents use formatter
defaults rather than project configuration. Their desktop session uses a trusted
global `gdfmt.serverPath` when set, otherwise the bundled server; save to a project
to select its configuration.

VS Code buffers use one EOL throughout. When the core retains another ending in
protected regions or multiline strings, mixed output cannot be applied losslessly:
the extension reports an error and leaves the document unchanged. Setting
`lineEnding: "preserve"` can help for uniformly ended input; it does not guarantee
uniform output for already mixed input. Use the CLI to retain exact mixed output.
VS Code may normalize mixed endings when loading a file, before formatting.
Uniform core output determines the actual EOL,
including when retained text differs from the configured structural ending.

- `gdfmt.enable`: enable the formatter (default `true`).
- `gdfmt.serverPath`: executable or Node CLI script, relative to the workspace
  folder or absolute. Desktop only; requires workspace trust.
- `gdfmt.configPath`: explicit configuration file; empty enables discovery. In Web,
  the path must be relative to the owning workspace folder.
- **gdfmt: Restart Server**, **gdfmt: Show Output**, **gdfmt: Show Version**.

Program discovery uses the explicit setting, then a trusted workspace's
`@gdfmt/gdfmt`, then the bundled executable. The selected program must match the
core version recorded in this extension. Node scripts use VS Code's extension
host runtime and must meet the npm package's Node requirement. In untrusted
workspaces only the bundled program runs, using normal project config discovery.

## VS Code Web

Build with `pnpm package:vscode:web`; the Web artifact is under
`target/editors/web/`. Browser-only VS Code 1.96.4 does not offer local VSIX
installation without a local/remote extension-management server. Use the existing
Gallery prerelease or the [documented development/sideloading routes](https://github.com/ntnyq/gdfmt/blob/main/docs/guide/integrations.md#vs-code-web).
No native executable or Node runtime is included. Virtual
workspaces, unsaved buffers, save formatting, undo, multi-root folders, untitled
defaults and UTF-16 diagnostics are supported. A failed Worker is replaced on the
next request; **Restart Server** also recreates it. No COOP/COEP headers are needed.

Web discovers `.gdfmtrc.json` / `.gdfmtrc.jsonc` up to the owning workspace root,
merges ancestors before children and honors `root: true`. JSONC and relative
`extends` use shared Rust parsing and validation. Explicit `gdfmt.configPath`
replaces discovery. Paths use `/` and cannot escape the owning workspace folder.
Documents outside a workspace must be opened in a folder first.

Web rejects `overrides`, configuration `ignore` (including empty arrays), and
ancestor `.editorconfig` files with an error and no edits. `.gitignore` and
`.gdfmtignore` do not filter explicitly opened documents in Web. Use the desktop
extension for full CLI configuration and file selection behavior. The same Web
rules apply in trusted and untrusted workspaces; `gdfmt.serverPath` is ignored.
The packaged Web extension allows workspace-relative `gdfmt.configPath` in
untrusted workspaces too. Desktop restricts workspace config paths and custom
executables until trusted. Normal configuration discovery remains available.
Limits: 16 MiB source, 1 MiB per configuration, 16 MiB total configuration,
32 inheritance levels, 128 merged layers and 128 discovery directories.

The extension contributes only formatting and configuration schema support; it
can coexist with Godot Tools for engine integration, navigation and debugging.
It never writes files itself and does not provide range formatting or lint rules.
Empty, partial and full-document selections do not gain a range provider; use
Format Document. Editor tab size and insert-spaces preferences do not override
gdfmt configuration. LSP range-formatting requests return `-32601`.

The Marketplace-compatible extension version is numeric. `gdfmtVersion` and the
VSIX receipt record the exact core version. Filenames use that core version:
`target/editors/TARGET/gdfmt-CORE_VERSION-TARGET.vsix` or
`target/editors/web/gdfmt-CORE_VERSION-web.vsix`. VSIX files follow the core prerelease status;
Marketplace publication is a separate maintainer action.

The [rc.1 GitHub Release](https://github.com/ntnyq/gdfmt/releases/tag/v0.1.0-rc.1)
provides all six frozen VSIX targets with numeric extension version `0.2.0` and
core `0.1.0-rc.1`, marked as prerelease. This release did not upload to Marketplace;
the previously published Gallery preview is separate from these rc.1 assets.

The prepared source uses core `0.1.0` with numeric extension `0.2.1`. This is
higher than the rc.1 VSIX version `0.2.0`; the six Gallery targets still listed
only prerelease `0.1.0` on 2026-10-24. These new versions are not yet available as
release downloads or Marketplace installs. Recheck availability before uploading.

For core 0.1.0, the recommended delivery baseline is the frozen GitHub Release
VSIX set. Marketplace already has a prerelease; stable publication needs a new
numeric extension version and separate acceptance. See the
[distribution review](https://github.com/ntnyq/gdfmt/blob/main/docs/research/0.1.0-editor-distribution-review.md)
and [installation/update instructions](https://github.com/ntnyq/gdfmt/blob/main/docs/guide/integrations.md#vs-code).
