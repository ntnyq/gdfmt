# gdfmt for VS Code

Validated GDScript formatting with a bundled Rust language server. No separate
Node installation is required. Supports desktop VS Code, SSH, WSL and containers
on the published native targets. A separate Web VSIX uses the same validated
Rust core through the existing WASM package in a Worker.

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
EditorConfig and ignore behavior match the CLI. Untitled documents use the
bundled server's defaults; save to a project to select its configuration.

VS Code buffers use one EOL throughout. When the core retains another ending in
protected regions or multiline strings, mixed output cannot be applied losslessly:
the extension reports an error and leaves the document unchanged. Use
`lineEnding: "preserve"` or the CLI. Uniform core output determines the actual EOL,
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

Build with `pnpm package:vscode:web` and install the Web VSIX from
`target/editors/web/`. No native executable or Node runtime is included. Virtual
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
Limits: 16 MiB source, 1 MiB per configuration, 16 MiB total configuration,
32 inheritance levels, 128 merged layers and 128 discovery directories.

The extension contributes only formatting and configuration schema support; it
can coexist with Godot Tools for engine integration, navigation and debugging.
It never writes files itself and does not provide range formatting or lint rules.

The Marketplace-compatible extension version is numeric. `gdfmtVersion` and the
VSIX receipt record the exact core prerelease. VSIX files follow the core prerelease status;
Marketplace publication is a separate maintainer action.
