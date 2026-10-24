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

Source preparation is now `0.1.0-rc.1`; it has not been tagged or published. See the
[candidate notes](docs/releases/0.1.0-rc.1.md) and
[committed-source CI evidence](docs/research/0.1.0-rc.1-ci.md). Published installation
commands below continue to reference the existing registry release.

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

For a published prerelease, use `cargo install gdfmt_cli --version 0.1.0-alpha.6 --locked`,
or `pnpm add @gdfmt/gdfmt@next` with Node 22.13+ on 22.x or Node 24.x.
The Cargo package is `gdfmt_cli`; both routes install the `gdfmt` command.
See the [Node.js API](docs/reference/node.md) for library usage and supported native targets.

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
- [0.1.0 release checklist](RELEASE_CHECKLIST.md)
- [0.1.0 release notes (draft)](docs/releases/0.1.0.md)

Grammar profiles cover Godot 4.3–4.7. Full Godot compatibility is not declared; project analysis and runtime behavior remain the engine's responsibility. gdfmt uses its own formatting style and does not promise identical output to gdtoolkit's `gdformat`.

## WASM and VS Code

The browser package exposes `createFormatter()` from `@gdfmt/wasm`; see the
[WASM API](docs/reference/wasm.md). Install with `pnpm add @gdfmt/wasm@next`, or build and test from source with
`pnpm build:wasm` and `pnpm test:wasm`.

`gdfmt --lsp` provides formatting over stdio. Build a native desktop extension
with `pnpm package:vscode`, or a browser-only VSIX with `pnpm package:vscode:web`.
Install the matching native VSIX on desktop and set `editor.defaultFormatter` to
`ntnyq.gdfmt` for GDScript. Browser-only installation uses the existing Gallery
prerelease or a supported developer route; local Web VSIX installation is not
available in every host. See [editor integration](docs/guide/integrations.md).
Source package/VSIX builds require Node, the repository's pnpm version and
`pnpm install --frozen-lockfile`; WASM/Web builds also require wasm-pack and the
WASM Rust target. See [build prerequisites](CONTRIBUTING.md#wasm-and-editors).

## License

[MIT](LICENSE). Third-party attribution is listed in [LICENSE-THIRD-PARTY](LICENSE-THIRD-PARTY).

The logo incorporates the Godot Engine icon by Andrea Calabró under [CC BY 4.0](docs/public/logo.LICENSE.txt). gdfmt is an independent project and is not endorsed by the Godot Engine project.
