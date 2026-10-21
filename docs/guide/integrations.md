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

There is no dedicated gdfmt editor extension or language server. Editors can integrate through an external command or the [Node.js API](/reference/node). API consumers must resolve project configuration explicitly.

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
