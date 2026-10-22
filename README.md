# gdfmt

<p align="center">
  <img src="docs/public/logo.svg" width="160" height="160" alt="gdfmt logo" />
</p>

<p align="center">A GDScript formatter, built in Rust.</p>
<p align="center"><a href="docs/guide/index.md">Get started</a> · <a href="docs/guide/configuration.md">Configuration</a> · <a href="CONTRIBUTING.md">Contributing</a></p>

English | [简体中文](README.zh-CN.md)

gdfmt formats `.gd` files with consistent spacing, indentation and line wrapping. Use it from the command line or through its Node.js API.

- **Validated output.** Formatting reparses the result, checks syntax and comment preservation, and verifies that a second pass produces the same output.
- **Project configuration.** Configure line width, indentation and newlines with `.gdfmtrc.json` or `.gdfmtrc.jsonc`, EditorConfig and per-path overrides.
- **Fits your workflow.** Format a project, check it in CI, preview a diff or pipe a buffer through stdin. Git ignore rules and `.gdfmtignore` control file selection.
- **Preserve intentional layout.** Keep selected regions unchanged with paired `formatter-off` / `formatter-on` comments.

## Get started

Install from source with Rust 1.88 or newer. The repository pins the development toolchain.

```sh
git clone https://github.com/ntnyq/gdfmt.git
cd gdfmt
cargo install --path crates/gdfmt_cli --locked
```

Run these commands in your Godot project:

```sh
gdfmt .                 # Format GDScript files in place
gdfmt --check .         # Check formatting without writing
gdfmt --diff src        # Preview formatting changes
gdfmt init              # Create .gdfmtrc.json
```

The default command formats the current directory. A check exits with `1` for syntax errors or formatting differences, and `2` for configuration, argument or I/O failures.

Install the npm prerelease with `pnpm add @gdfmt/gdfmt@next`. See the [Node.js API](docs/reference/node.md) for usage.

## Configuration

gdfmt works without a configuration file. To share settings, create `.gdfmtrc.json` or `.gdfmtrc.jsonc` (both accept comments and trailing commas):

```json
{
  "root": true,
  "language": { "godotVersion": "4.7" },
  "format": {
    "lineWidth": 100,
    "indentStyle": "tab"
  }
}
```

See [configuration](docs/guide/configuration.md) for all options, inheritance, overrides and EditorConfig support.

## Documentation

- [Quickstart](docs/guide/index.md)
- [CLI reference](docs/reference/cli.md)
- [CI and editor workflows](docs/guide/integrations.md)
- [Formatting and protected regions](docs/reference/formatting.md)
- [Language support and limitations](docs/reference/compatibility.md)
- [Node.js API](docs/reference/node.md)
- [Contributing](CONTRIBUTING.md)
- [Publishing and CI releases](RELEASING.md)

Grammar profiles cover Godot 4.3–4.7. Full Godot compatibility is not declared; project analysis and runtime behavior remain the engine's responsibility. gdfmt uses its own formatting style and does not promise identical output to gdtoolkit's `gdformat`.

## WASM and VS Code

The browser package exposes `createFormatter()` from `@gdfmt/wasm`; see the
[WASM API](docs/reference/wasm.md). Build and test from source with
`pnpm build:wasm` and `pnpm test:wasm` until its first release.

`gdfmt --lsp` provides formatting over stdio. Build a native desktop extension
with `pnpm package:vscode`, or a browser-only VSIX with `pnpm package:vscode:web`.
Install the matching VSIX and set `editor.defaultFormatter` to
`ntnyq.gdfmt` for GDScript. See [editor integration](docs/guide/integrations.md).

## License

[MIT](LICENSE). Third-party attribution is listed in [LICENSE-THIRD-PARTY](LICENSE-THIRD-PARTY).

The logo incorporates the Godot Engine icon by Andrea Calabró under [CC BY 4.0](docs/public/logo.LICENSE.txt). gdfmt is an independent project and is not endorsed by the Godot Engine project.
