# Quickstart

gdfmt formats GDScript files from the command line. Start with the defaults, then commit a configuration file when your project needs shared settings.

The current published prerelease is **0.1.0-rc.1**, available on crates.io and npm
`next`; see the [candidate notes](../releases/0.1.0-rc.1.md) and
[registry acceptance](../research/0.1.0-rc.1-publication.md). Stable 0.1.0 remains
unreleased. The working tree prepares core **0.1.0** with extension **0.2.1**;
see the [preparation report](../research/0.1.0-preparation.md). Source builds use
the checked-out version; the registry commands below still install rc.1.

## Install from source

Use Rust 1.88 or newer. The repository's `rust-toolchain.toml` pins the development toolchain.

```sh
git clone https://github.com/ntnyq/gdfmt.git
cd gdfmt
cargo install --path crates/gdfmt_cli --locked
gdfmt --version
```

Make sure Cargo's binary directory is on your `PATH`. You can also run the CLI directly from the checkout with `cargo run -p gdfmt_cli -- <arguments>`.

## Install a published prerelease

Cargo prereleases need an explicit version; the package name differs from the executable:

```sh
cargo install gdfmt_cli --version '=0.1.0-rc.1' --locked
gdfmt --version
```

For npm, use Node 22.13+ on 22.x or Node 24.x and keep optional dependencies enabled:

```sh
pnpm add -D @gdfmt/gdfmt@next
pnpm exec gdfmt --version
```

For a one-off invocation use `pnpm dlx @gdfmt/gdfmt@next --version`. Supported
native targets and API usage are listed in the [Node reference](/reference/node).
`next` is a moving prerelease channel; pin an exact version in CI or commit the
project's lockfile. Source installation above follows the checked-out revision.

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
