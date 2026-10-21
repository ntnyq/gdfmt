# Contributing to gdfmt

Bug reports, focused fixes and documentation improvements are welcome. For formatter bugs, include a minimal `.gd` input, the expected output, your configuration, language profile and `gdfmt --version`.

## Development setup

Use the Rust toolchain pinned in `rust-toolchain.toml`. The workspace uses edition 2024 and supports Rust 1.88 or newer.

```sh
cargo build --workspace --locked
cargo run -p gdfmt_cli -- --help
```

## Workspace layout

| Location                 | Responsibility                                                    |
| ------------------------ | ----------------------------------------------------------------- |
| `crates/gdfmt_text`      | Source spans, positions and structured diagnostics                |
| `crates/gdfmt_syntax`    | Lossless CST, typed views and language profiles                   |
| `crates/gdfmt_parser`    | Lexing, parsing, structural constraints and recovery              |
| `crates/gdfmt_formatter` | Layout and comment attachment; no parser or filesystem dependency |
| `crates/gdfmt`           | Validated text formatting facade                                  |
| `crates/gdfmt_config`    | Configuration model and JSON Schema                               |
| `crates/gdfmt_workspace` | Configuration/file discovery and guarded writes                   |
| `crates/gdfmt_cli`       | Shared command runner and binary                                  |
| `crates/gdfmt_napi`      | Node-API adapter                                                  |
| `packages/gdfmt`         | npm facade and TypeScript declarations                            |
| `xtask`                  | Schema, compatibility and project-trial tooling                   |
| `tests/compatibility`    | Original fixtures, expected diagnostics and engine pins           |
| `docs`                   | English and Chinese VitePress documentation                       |

Keep filesystem and CLI concerns outside the parser and formatter. Diagnostics are structured data; only the binary entry point terminates the process. Core crates forbid unsafe code; the napi crate denies handwritten unsafe code while allowing generated FFI constructors.

The parser preserves exact input text in the CST for valid and invalid input within its size limit. Formatting returns writable output only after reparsing, syntax/comment preservation and idempotence checks. Protected regions must retain exact bytes. See the [formatting reference](docs/reference/formatting.md) and [language support](docs/reference/compatibility.md) for the public contract.

## Validate a change

Run focused tests while working, then the relevant workspace checks before opening a PR:

```sh
cargo test --workspace --all-features --locked
cargo test --workspace --no-default-features --locked
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo xtask schema --check
cargo xtask compatibility
cargo xtask syntax-coverage
```

CI also checks Rust 1.88 and runs tests on Linux, macOS and Windows. Ordinary Rust tests require no Godot installation.

Add focused regressions to the affected crate. Parser tests should check diagnostics, tree structure and exact source preservation. Shared fixtures use descriptive hyphenated names and entries in `tests/compatibility/manifest.json`; classify script fixtures in `engine.json`. Formatting changes should check comments, literal spelling, protected bytes and idempotence. Regenerate the schema with `cargo xtask schema` when configuration changes.

## Engine comparisons

`tests/compatibility/engine.json` is the source of truth for exact engine commits, version strings and Linux archive digests. Use the editor binary matching the selected pin:

| Grammar profile | Engine tag     | Exact `--version`                 |
| --------------- | -------------- | --------------------------------- |
| 4.3             | `4.3-stable`   | `4.3.stable.official.77dcf97d8`   |
| 4.4             | `4.4-stable`   | `4.4.stable.official.4c311cbee`   |
| 4.5             | `4.5-stable`   | `4.5.stable.official.876b29033`   |
| 4.6             | `4.6-stable`   | `4.6.stable.official.89cea1439`   |
| 4.7             | `4.7.2-stable` | `4.7.2.stable.official.ed1daf0bf` |

```sh
cargo xtask conformance --language 4.7 --godot /path/to/pinned/godot --report target/conformance/source-4.7.json
cargo xtask formatter-conformance --language 4.7 --godot /path/to/pinned/godot --report target/conformance/formatted-4.7.json
```

Each invocation checks one exact engine; repeat for all five profiles when changing grammar or formatting. Reports include source, diagnostics, engine logs and verdicts. The harness uses fresh isolated projects and `--check-only`; it does not run scripts. Analyzer boundaries are classified separately from syntax expectations. This does not establish project import success, complete language conformance or runtime equivalence.

The syntax inventory and explicit upstream-test exclusions live in `tests/compatibility/syntax-coverage.json`. Pinned third-party project trials and reviewed gdtoolkit differences have their own [reproduction instructions](tests/project-trials/README.md). Keep those records current when their inputs or output change.

## Node.js packages

Use Node 22.13+ on 22.x or Node 24.x, with pnpm:

```sh
pnpm install --frozen-lockfile
pnpm typecheck
pnpm test:npm
pnpm verify:npm --local
```

`test:npm` builds release bindings and the standalone CLI, packs the root and current platform packages, installs the actual tarballs through an isolated local registry, and runs Vitest consumer tests plus ESM/CJS declaration checks. Packages and receipts are written under `target/npm/artifacts/`. To try them in another project, install both the root tarball and its matching platform tarball from that directory. Native builds require Rust; consuming the packages does not.

The packaging matrix is macOS arm64/x64, Windows x64 MSVC and Linux x64 glibc/musl, each tested on Node 22.13.0 and 24.0.0. `.github/workflows/npm.yml` collects the matrix. `pnpm verify:npm` requires all ten receipts and identical root tarballs; `--local` deliberately records an incomplete matrix. A local pass does not establish cross-platform readiness. The [recorded complete-matrix result](tests/npm/acceptance.json) identifies its tested revision and artifact hashes; rerun the matrix for new release artifacts.

`.github/workflows/release.yml` publishes version tags after the complete Rust CI and npm matrix pass. Actual publication is disabled until platform bootstrap is complete and the repository variable `RELEASE_ENABLED` is `true`. It freezes tested artifacts, publishes the platform packages before the root, verifies registry consumers, and only then advances the npm release channel. Repeated runs skip matching existing versions; conflicting bytes require a new version. See [publishing](RELEASING.md) ([简体中文](RELEASING.zh-CN.md)) for first-release authentication, OIDC setup, coordinated version updates and recovery.

```sh
pnpm release:version 0.1.0-alpha.1 --dry-run
pnpm release:check
pnpm test:release
```

## Documentation

```sh
cd docs
pnpm install --frozen-lockfile
pnpm dev
pnpm build
pnpm preview
```

English pages live under `docs/`; Chinese pages mirror their relative paths under `docs/zh/`. Keep navigation, examples and public behavior synchronized. Shared assets live in `docs/public/`; configuration and theme live in `docs/.vitepress/`.

For subpath hosting, use the same base for build and preview, such as `pnpm build --base /gdfmt/` and `pnpm preview --base /gdfmt/`. VitePress checks internal page links during builds.

For Vercel, set the project's Root Directory to `docs` and use Node.js 22 or newer. `docs/vercel.json` configures dependency installation, the VitePress build, `.vitepress/dist` output and extensionless URLs.

## Pull requests

Use Conventional Commit titles such as `fix(parser): preserve lambda indentation`. Explain the user-visible change, link related issues and report validation results. Keep PRs focused and update relevant documentation when a public contract changes. Avoid claims of full Godot compatibility or runtime equivalence.
