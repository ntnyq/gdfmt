# Quickstart

gdfmt formats GDScript files from the command line. Start with the defaults, then commit a configuration file when your project needs shared settings.

## Install from source

Use Rust 1.88 or newer. The repository's `rust-toolchain.toml` pins the development toolchain.

```sh
git clone https://github.com/ntnyq/gdfmt.git
cd gdfmt
cargo install --path crates/gdfmt_cli --locked
gdfmt --version
```

Make sure Cargo's binary directory is on your `PATH`. You can also run the CLI directly from the checkout with `cargo run -p gdfmt_cli -- <arguments>`.

The npm package is not yet published. To build and use local npm artifacts, see [contributing](https://github.com/ntnyq/gdfmt/blob/main/CONTRIBUTING.md#nodejs-packages).

## Format a project

Run in your Godot project directory:

```sh
gdfmt .
```

The default command writes formatting changes in place. You can pass individual files, directories or multiple paths:

```sh
gdfmt src/player.gd
gdfmt src addons
```

Only `.gd` files are selected. gdfmt respects project ignore files and skips `.git`, `.godot`, `node_modules` and `target`. See [file selection](/guide/configuration#ignore-rules-and-input-selection).

For example:

```gdscript
func run( value : int=1 ): return [value,2] # result
```

becomes:

```gdscript
func run(value: int = 1):
	return [value, 2] # result
```

## Check or preview changes

```sh
gdfmt --check .
gdfmt --diff src/player.gd
```

Both commands leave files unchanged. A check returns `0` when files are formatted, `1` for source errors or formatting differences, and `2` for configuration, argument or I/O failures.

To check syntax alone, run `gdfmt check . --syntax-only`. This uses gdfmt's parser; it does not replace Godot's project analysis.

## Share configuration

```sh
gdfmt init
```

This creates `.gdfmtrc.json` in the current directory and refuses to overwrite an existing file. The defaults use a line width of 100, tabs, LF line endings and a final newline.

See [configuration](/guide/configuration) for options, EditorConfig, inheritance and per-path overrides. Use `gdfmt explain-config src/player.gd` to inspect the settings applied to a file.

## Next steps

- [Set up CI or an editor command](/guide/integrations).
- [Keep selected regions unchanged](/reference/formatting#protected-regions).
- [Explore CLI commands](/reference/cli) or the [Node.js API](/reference/node).
- [Check language support and limitations](/reference/compatibility).
