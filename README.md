# gdfmt

<p align="center">
  <img src="docs/public/logo.svg" width="160" height="160" alt="gdfmt logo: the Godot icon with code brackets and indented lines" />
</p>

English | [简体中文](README.zh-CN.md)

A Rust-based GDScript formatter with lossless parsing, syntax diagnostics, layout configuration, and locally prepared npm distribution.

The Cargo workspace, source locations and diagnostics, configuration and JSON Schema, lossless lexer, Pratt expressions, declarations and statements, abstract constraints, language version gates, pinned engine conformance harness, and initial CLI are implemented. P3 syntax is accepted with explicit exclusions; the P4 script formatter and verified CLI are available. P4 formatter alpha is accepted with documented exclusions and pinned project trials; protected formatter-off/on regions are available. Node/npm APIs and local packaging are implemented; cross-platform artifact acceptance remains a CI gate; lint rules and semantic auto-fixes are outside scope. Crates are not published yet.

## Development

Rust edition 2024, with a minimum supported Rust version of 1.88. The development toolchain is pinned in `rust-toolchain.toml`.

```sh
cargo test --workspace --all-features --locked
cargo test --workspace --no-default-features --locked
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo xtask schema --check
cargo xtask compatibility
cargo xtask syntax-coverage
```

## Local npm artifacts

Node 22.13+ on 22.x or Node 24.x:

```sh
pnpm install --frozen-lockfile
pnpm typecheck
pnpm test:npm
pnpm verify:npm --local
```

Scripts use TypeScript/tsx; Vitest TypeScript tests install and exercise actual tarballs, including ESM/CJS declarations and CLI parity. [Node/npm contracts](docs/reference/node.md) document APIs, proposed platforms and release recovery. Nothing is published; complete cross-platform acceptance remains pending CI.

## Available commands

```sh
cargo run -p gdfmt_cli -- --help
cargo run -p gdfmt_cli -- check . --syntax-only
cargo run -p gdfmt_cli -- format src
cargo run -p gdfmt_cli -- lex tests/compatibility/fixtures/containers.gd
cargo run -p gdfmt_cli -- lex tests/compatibility/fixtures --reporter json
cargo run -p gdfmt_cli -- lex - --reporter json < script.gd
cargo run -p gdfmt_cli -- schema
cargo run -p gdfmt_cli -- init
cargo run -p gdfmt_cli -- explain-config .
```

`lex` performs read-only lexical checks and reports tokens. Exit codes are `0` for no lexical errors, `1` for lexical errors, and `2` for argument, configuration, or I/O errors. JSON spans are UTF-8 byte ranges `[start, end)`; virtual tokens occupy no bytes.

**Passing lexical checks does not prove valid GDScript syntax.** Use `check --syntax-only` for parser checks. `format` and the default command format scripts, including comments, lambdas, declarations and suites; `check` and `--diff` are read-only. See the [P4 formatting scope](docs/reference/formatting.md) before use.

Configuration lives in `.gdfmtrc.json`. The editor schema is [schemas/gdfmtrc.schema.json](schemas/gdfmtrc.schema.json); see [configuration](docs/guide/configuration.md) for the current options and merge rules.

## Documentation

The [English documentation](docs/index.md) and [中文文档](docs/zh/index.md) form a VitePress site. English is the default; the language menu switches to the matching Chinese page.

Use Node.js 22 or newer and pnpm to develop the site:

```sh
cd docs
pnpm install --frozen-lockfile
pnpm dev
pnpm build
pnpm preview
```

The source layout follows VitePress routing:

```text
docs/
├── .vitepress/       # Site configuration and theme
├── public/           # Shared logo and attribution
├── index.md          # English home
├── guide/            # Getting started and configuration
├── reference/        # Parser APIs and language profiles
├── development/      # Architecture, compatibility, and roadmap
└── zh/               # Matching Chinese routes
```

Keep the same relative page paths in both languages so that the language switcher stays on the current topic. Documentation links and navigation use site-root paths; VitePress applies the deployment base. For subpath hosting, build with `pnpm build --base /gdfmt/` and preview with `pnpm preview --base /gdfmt/`.

## Implementation status

| Phase | Status                                                                                                                                                                                                                                                                                             |
| ----- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| P0    | Stable reference commit, original lexical fixtures, and Rowan prototype in place; syntax inventory accepted; name availability and publishing permissions pending                                                                                                                                  |
| P1    | Workspace, foundational contracts, configuration model, CLI shell, and CI implemented                                                                                                                                                                                                              |
| P2    | Pinned Unicode XID tables, strings and escapes, signed numbers, special tokens, continuation across blank lines, and parser-driven lambda suite context implemented; engine differential validation and sustained fuzzing pending                                                                  |
| P3    | Expressions, declarations, statements, lambdas, properties, match, hierarchical lossless CST, abstract constraints, version gates, and initial typed facade implemented; 262 scripts match five pinned engines; [syntax audit](docs/development/syntax-audit.md) accepted with explicit exclusions |
| P4    | Doc printer, comments, lambda/declaration/suite layout, whole-file options, verified facade and minimum CLI implemented; alpha accepted with pinned project trials and reviewed gdtoolkit differences                                                                                              |
| P5    | Protected formatter-off/on regions with exact bytes and validated indentation fallback implemented                                                                                                                                                                                                 |
| P6    | Local extends, ordered overrides, EditorConfig, ignore rules and guarded CLI writes implemented; platform CI gates retained                                                                                                                                                                        |
| P7    | Sync/async Node APIs, ESM/CJS and CLI packaging implemented; full artifact matrix pending CI                                                                                                                                                                                                       |
| P8–P9 | Formatter integration and stability planned                                                                                                                                                                                                                                                        |

The Rust single-expression entry point is `gdfmt_parser::parse_expression(source, &ParseOptions::default())`. See [expression parsing](docs/reference/expressions.md) for usage, precedence, and limits. This entry point does not accept complete `.gd` files.

The full-script entry point is `gdfmt_parser::parse(source, &ParseOptions::default())`, returning a hierarchical `SourceFile` CST. See [script parsing](docs/reference/parsing.md) for syntax, context, and recovery boundaries.

See [language versions](docs/reference/language-versions.md) for profiles and feature boundaries, and [engine conformance](docs/development/engine-conformance.md) for the pinned Godot 4.3–4.7 matrix and its scope.

P6 configuration and file/CLI behavior are implemented; see [configuration](docs/guide/configuration.md) and [acceptance](docs/development/formatter-acceptance.md). Node/npm artifacts are implemented but unpublished; see [Node contracts](docs/reference/node.md). Broader project trials and formatter fuzzing continue. [P3 acceptance](docs/development/syntax-audit.md) records supported scope and exclusions.

- [Implementation plan](docs/development/implementation-plan.md)
- [Architecture and contracts](docs/development/index.md)
- [Compatibility and remaining gaps](docs/development/compatibility.md)

## License

Code is released under the [MIT License](LICENSE). Fixtures are original to this project. Unicode XID tables derive from a pinned Godot commit; see [third-party notices](LICENSE-THIRD-PARTY).

The logo combines the Godot Engine icon by Andrea Calabró with code brackets and indentation marks, under [CC BY 4.0](docs/public/logo.LICENSE.txt). gdfmt is an independent project and is not endorsed by the Godot Engine project.
