# Getting started

gdfmt is a Rust-based GDScript formatter. The lexer, lossless script and expression parsers, configuration schema, and initial CLI work today. The P4 script formatter and verified CLI are available; source-build alpha acceptance is complete with documented exclusions; protected formatter-off/on regions are available. Node/npm APIs and local packaging are implemented; complete platform acceptance remains pending CI; lint and semantic fixes are outside scope; crates are not published yet.

::: warning Development status
The default command formats scripts, including comments, lambdas, declarations and suites; `check` is read-only and `check --syntax-only` runs parser checks. Pinned/licensed project trials and reviewed style differences are recorded in [P4 acceptance](/development/formatter-acceptance). Read the [P4 formatting scope](/reference/formatting). Passing `lex` does not establish syntax validity.
:::

## Build from source

Clone the repository and use the Rust toolchain pinned in `rust-toolchain.toml`. The workspace uses edition 2024 and supports Rust 1.88 or newer.

```sh
git clone https://github.com/ntnyq/gdfmt.git
cd gdfmt
cargo build --workspace --locked
cargo run -p gdfmt_cli -- --help
```

## Inspect tokens

```sh
cargo run -p gdfmt_cli -- lex tests/compatibility/fixtures/containers.gd
cargo run -p gdfmt_cli -- lex tests/compatibility/fixtures --reporter json
cargo run -p gdfmt_cli -- lex - --reporter json < script.gd
```

`lex` reads files without modifying them. It accepts `.gd` files, directories, multiple paths, or stdin on its own. Explicit files and directory traversal respect local Git/ignore files and JSON ignores; see [input selection](/guide/configuration#ignore-rules-and-input-selection). Built-in exclusions include `.git`, `.godot`, `node_modules`, and `target` by default.

| Exit code | Meaning                               |
| --------- | ------------------------------------- |
| `0`       | No lexical errors                     |
| `1`       | Lexical errors found                  |
| `2`       | Argument, configuration, or I/O error |

The JSON reporter uses `schemaVersion: 1` and `spanEncoding: "utf8Bytes"`. Spans are half-open UTF-8 byte ranges `[start, end)`; virtual tokens occupy no bytes.

## Configure a project

```sh
cargo run -p gdfmt_cli -- init
cargo run -p gdfmt_cli -- schema
cargo run -p gdfmt_cli -- explain-config .
```

`init` creates `.gdfmtrc.json` and refuses to overwrite an existing file. [Configuration](/guide/configuration) explains discovery, defaults, strict validation, and field provenance. Width, indentation and whole-file newline/blank-line settings are applied; see [formatting](/reference/formatting).

## Use the parser in Rust

Use [`parse`](/reference/parsing) for complete scripts and [`parse_expression`](/reference/expressions) for individual expressions. Both return a lossless CST and structured diagnostics. [Language profiles](/reference/language-versions) cover verified differences across Godot 4.3–4.7; they do not declare full compatibility.

## Work on the documentation

Use Node.js 22 or newer and pnpm:

```sh
cd docs
pnpm install --frozen-lockfile
pnpm dev
pnpm build
pnpm preview
```

English pages live at the site root; Chinese pages mirror their paths under `zh/`. Use the language menu to switch the current topic. Shared assets live in `public/`, while site configuration and theme extensions live in `.vitepress/`.

To host under a subpath, pass the same base to build and preview, for example `pnpm build --base /gdfmt/` and `pnpm preview --base /gdfmt/`.

See [architecture](/development/), [compatibility](/development/compatibility), and the [implementation plan](/development/implementation-plan) for contributor context.
