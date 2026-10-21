# Repository Guidelines

## Project Structure & Module Organization

gdfmt is a Rust-based GDScript formatter. The Cargo workspace contains nine crates under `crates/`: `gdfmt_text` (spans and diagnostics), `gdfmt_syntax` (lossless CST and language versions), `gdfmt_parser` (lexing and parsing), `gdfmt_formatter` (Doc printer, comment attachment and script layout), `gdfmt` (validated text facade), `gdfmt_config` (configuration and schema), `gdfmt_workspace` (file/configuration discovery and guarded writes), `gdfmt_cli` (command runner and binary), and `gdfmt_napi` (Node-API host adapter). Keep filesystem traversal and CLI concerns outside the parser and formatter.

`packages/gdfmt/` contains the npm facade and declarations. TypeScript scripts under `scripts/npm/` run through `tsx`; installed-tarball integration tests under `tests/npm/` use Vitest and TypeScript.

Integration tests live in each crate’s `tests/` directory. Shared GDScript fixtures and expected results live in `tests/compatibility/`. `xtask/` implements schema and compatibility tooling; `schemas/` contains the generated configuration schema; `docs/` contains the bilingual VitePress user documentation; `CONTRIBUTING.md` covers workspace structure and contributor workflows.

## Build, Test, and Development Commands

Use the development toolchain pinned in `rust-toolchain.toml`; preserve the Rust 1.88 minimum supported version and edition 2024.

- `pnpm typecheck`, `pnpm test:npm`, and `pnpm verify:npm --local`: validate Node scripts, installed tarballs, and current-platform receipts; CI requires the complete `pnpm verify:npm` matrix. Nothing is published by these commands.
- `cargo build --workspace`: build all workspace members.
- `cargo run -p gdfmt_cli -- lex tests/compatibility/fixtures/containers.gd`: inspect tokens and lexical diagnostics.
- `cargo test --workspace --all-features --locked`: run the primary test suite.
- `cargo test --workspace --no-default-features --locked`: verify the alternate feature configuration.
- `cargo fmt --all -- --check`: check Rust formatting.
- `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`: run lint checks.
- `cargo xtask schema --check` and `cargo xtask compatibility`: detect schema drift and validate fixture expectations.
- `cargo xtask syntax-coverage`: validate the pinned syntax inventory and coverage evidence.

For engine comparisons, run `cargo xtask conformance --godot /path/to/pinned/godot` and `cargo xtask formatter-conformance --godot /path/to/pinned/godot`; see `CONTRIBUTING.md#engine-comparisons` for the required binary. Ordinary Rust tests require no Godot installation.

## Coding Style & Naming Conventions

Use rustfmt’s default four-space indentation. Follow existing Rust conventions: `snake_case` modules/functions, `PascalCase` types, and `SCREAMING_SNAKE_CASE` constants. Inherit workspace dependencies and lints in crate manifests. Handwritten unsafe code is forbidden. The napi host crate uses `deny(unsafe_code)` to permit napi-generated FFI constructors; core crates retain workspace `forbid(unsafe_code)`. Keep diagnostics structured; reserve process termination for the binary entry point.

## Testing Guidelines

Use Rust’s `#[test]` framework with descriptive `snake_case` test names. Add focused regressions in the affected crate. Parser tests should verify diagnostics, structure, and exact source preservation for valid and invalid input. Name fixtures descriptively with hyphens, register expectations in `tests/compatibility/manifest.json`, and classify script fixtures in `engine.json`. No numeric coverage threshold is configured.

## Commit & Pull Request Guidelines

Use Conventional Commit titles, such as `fix(parser): preserve lambda indentation`. Keep changes focused. PRs should explain the behavior change, link relevant issues, and list validation commands/results. Update schema and compatibility documentation when contracts change. Formatting covers expressions, comments, lambdas, declarations and suites; known compatibility boundaries are documented in `docs/reference/compatibility.md`; lint rules and semantic auto-fixes are outside scope. Preserve formatter-off/on regions exactly while continuing to reject invalid syntax. Avoid claiming full Godot compatibility.
