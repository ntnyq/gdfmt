# Formatter implementation plan

gdfmt is a Rust-based GDScript formatter. Its parser, diagnostics, configuration and file handling support safe, repeatable formatting through a Rust library, CLI and locally prepared npm artifacts.

The product scope was narrowed on 2026-10-21. The earlier toolchain plan is superseded: built-in lint rules, rule registries, lint suppression directives, unused-variable analysis, naming/function-size checks, semantic auto-fixes and dynamic rule plugins are outside scope. They are not release requirements. Existing parser diagnostics remain mandatory because formatting requires a valid syntax tree.

## Product contract

- `gdfmt [paths...]` and `gdfmt format [paths...]` format valid GDScript, defaulting to the current directory.
- `gdfmt check [paths...]` checks syntax and formatting without writing. It does not run lint rules.
- `check --syntax-only` exposes the parser diagnostics without formatting.
- `--diff` shows formatting differences without writing; stdin and JSON share the same core.
- Paired `# gdfmt: formatter-off` / `# gdfmt: formatter-on` comments preserve exact source bytes, while the entire file must still parse.
- `.gdfmtrc.json`, generated JSON Schema and explainable resolved configuration control layout.
- The Rust library, standalone CLI and Node package must produce identical formatting.

Formatting preserves significant tokens, literal spellings and comment associations. Renaming, declaration sorting, type insertion, removing statements and other semantic rewrites are excluded. Formatting a file is already the layout transformation; a separate lint `--fix` workflow is unnecessary.

Passing `check` does not establish Godot project compilation or runtime correctness. Cross-file inheritance, resources, scenes, autoloads and engine overload resolution are outside scope. Godot 3, a full type checker, LSP and code generation are also excluded.

## Architecture

| Module            | Responsibility                                                               |
| ----------------- | ---------------------------------------------------------------------------- |
| `gdfmt_text`      | Source identity, byte spans, positions, diagnostics and validated text edits |
| `gdfmt_syntax`    | Lossless CST, version profiles, typed views and formatter comment regions    |
| `gdfmt_parser`    | Lexing, parsing, recovery and mandatory structural constraints               |
| `gdfmt_formatter` | Comment attachment, Doc layout, printing and protected-region restoration    |
| `gdfmt`           | Pure text facade with reparse, structural equivalence and idempotence gates  |
| `gdfmt_config`    | Strict partial/resolved layout configuration and generated schema            |
| `gdfmt_workspace` | File/config discovery, option mapping and guarded writes                     |
| `gdfmt_cli`       | Arguments, reporters, shared runner and executable                           |
| `gdfmt_napi`      | Node-API text/configuration workers and the shared CLI adapter               |
| `xtask`           | Schema, syntax, compatibility, engine and project-trial verification         |

Keep Node, filesystem traversal and terminal output outside parser/formatter dependencies. The napi crate and npm package are host adapters; no linter or rule-plugin host is planned.

The public formatter entry point remains `gdfmt::format(source, &FormatRequest) -> FormatResult`. `output: None` means the host must not write. Spans use half-open UTF-8 byte offsets; future JS bindings must explicitly convert byte positions to UTF-16 when needed.

## Correctness and file behavior

1. Lossless parsing reconstructs original valid and invalid input exactly.
2. Invalid/unsupported syntax and recovery nodes prevent writable output.
3. Output reparses using the same language profile, limits and source identity.
4. Normalized CST topology, significant token spellings and comment associations agree.
5. Formatting the result again produces exactly the same bytes.
6. Protected regions retain their original bytes, including marker lines and line endings.

The facade checks every candidate before allowing writes. If restoring a region conflicts with indentation changes, preserve the enclosing top-level unit conservatively and validate it again. See [region behavior](/reference/formatting#protected-regions) for the public contract.

Exit codes remain 0 for success, 1 for source errors or formatting differences, and 2 for argument/configuration/I/O/internal failures. Mixed files use `2 > 1 > 0`. Formatting directives cannot disable parser errors. Invalid marker pairs are source diagnostics, not successful partial formatting.

Writes use same-directory temporary files, preserve permissions and detect external changes before replacement. Atomicity is per file. Validate every configuration and input read before starting replacements. Standard output contains only requested source, diff or JSON.

## Configuration direction

Current options are documented in [configuration](/guide/configuration); the Rust serde models and generated schema are the single source of truth. Unknown fields and explicit null are rejected. Missing fields retain inherited values; false and zero remain meaningful.

P6 implements local JSON `extends`, ordered directory overrides, `.gdfmtignore`, EditorConfig and explicit provenance. Resolve paths relative to their declaring configuration; detect cycles and bound inheritance depth. Merge objects fieldwise, replace ordinary arrays, and define overrides as an explicit append exception. Defaults are filled only after merging. EditorConfig mappings must report only effective supported options.

Do not add `lint`, `rules` or rule-plugin configuration. No ordinary formatter invocation requires Godot or a network request.

## Phases and acceptance

Phase identifiers are retained so existing acceptance links and engine evidence stay meaningful; P5 and P8 now have formatter-only scope.

| Phase | Scope                                                       | Status / acceptance                                                                                                            |
| ----- | ----------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------ |
| P0    | Reference pins, source contracts and original fixtures      | Syntax inventory accepted; publishing names/permissions still unverified                                                       |
| P1    | Workspace, text spans, configuration, CLI shell and CI      | Implemented                                                                                                                    |
| P2    | Lossless lexer and bounded syntax storage                   | Implemented; sustained fuzzing continues                                                                                       |
| P3    | Script parser, structural constraints and language profiles | [Accepted with explicit exclusions](/development/syntax-audit)                                                                 |
| P4    | Faithful script formatter and verified facade/CLI           | [Source-build alpha accepted with project-trial exclusions](/development/formatter-acceptance)                                 |
| P5    | Protected formatter-off/on regions                          | Exact bytes, marker errors, syntax rejection, indentation fallback and idempotence verified; see the acceptance record         |
| P6    | Production layout configuration and file/CLI handling       | [Implemented with local acceptance and cross-platform CI gates](/development/formatter-acceptance#p6-configuration-acceptance) |
| P7    | napi and npm distribution                                   | [Implemented; local packaged acceptance, full target/runtime CI pending](/development/formatter-acceptance#p7-node-acceptance) |
| P8    | Formatter integration and performance                       | Editor/CI recipes, project corpus growth, sustained fuzzing, measured throughput/startup/memory                                |
| P9    | Stable formatter release                                    | Complete compatibility, artifact, recovery and documentation gates                                                             |

### P5: Protected regions

Recognize paired standalone comment tokens, reject nesting and unmatched markers, and ignore marker-like text in strings or suffix comments. Keep original content and marker lines byte-for-byte, even when line-ending, blank-line or final-newline settings differ. Use a conservative enclosing-unit fallback for indentation or expression-boundary conflicts. Syntax errors anywhere still prevent output.

Acceptance covers multiple regions, empty/comment-only ranges, EOF, BOM, Unicode, CRLF/mixed endings, nested suites, cross-declaration/expression boundaries, every supported language profile, read-only CLI behavior and failed writes. The earlier lint/fix deliverables are removed, rather than marked implemented.

### P6: Configuration and production CLI

Local `extends`, appended overrides, EditorConfig mappings and local/JSON ignore rules are implemented with documented precedence and declaring-directory path semantics. Runtime/schema matrices cover false/zero/empty arrays and invalid shapes. Workspace/CLI regressions cover ignored explicit inputs, empty discovery, stable IDs, virtual stdin paths, reporters, preflight failures, permissions, concurrent edits and interrupted preparation. The same workspace tests remain in the Linux/macOS/Windows CI matrix; this local macOS acceptance does not establish Windows/Linux execution. See [configuration](/guide/configuration) for best-effort race detection and forced-termination temporary-file limits.

### P7: Node and npm

Sync/async text and explicit configuration APIs are implemented through Node-API 4; CPU work uses libuv workers. ESM/CJS share one facade, diagnostics declare UTF-8 byte offsets, and the CLI shim calls the existing Rust runner. Repository scripts use TypeScript/tsx and installed-package tests use Vitest TypeScript. Exact-version optional platform packages, manifest/native target validation, schema/declaration exports, clean installation without Rust and artifact receipts are implemented. See [Node/npm contracts and recovery](/reference/node) for the five proposed native targets, two pinned Node runtimes, local commands and partial-release procedure. Local macOS acceptance does not complete the ten-job platform/runtime matrix. No package is published and publishing permissions remain unverified.

### P8–P9: Formatter stability

Expand licensed, commit-pinned real projects and review formatting differences. Retain fixed-engine syntax/formatter comparisons. Continuously fuzz parser/formatter/config inputs and minimize failures. Measure before optimizing; benchmark core processing separately from process startup. Do not promise a speedup without repeatable evidence. Freeze 1.0 layout/API contracts only after supported syntax, output invariants, file-failure recovery, platform installation and documentation match shipped artifacts.

## Verification gates

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --all-features --locked
cargo test --workspace --no-default-features --locked
cargo xtask schema --check
cargo xtask compatibility
cargo xtask syntax-coverage
cargo +1.88.0 check --workspace --all-targets --all-features --locked
```

Also run `pnpm typecheck`, `pnpm test:npm` and `pnpm verify:npm --local`; CI requires the full `pnpm verify:npm` matrix. Build the VitePress site with `pnpm build` in `docs/`. Run both engine commands for each exact pin in [engine conformance](/development/engine-conformance), and reproduce the [pinned project trials](https://github.com/ntnyq/gdfmt/tree/main/tests/project-trials). Ordinary Rust tests require no Godot installation. Do not claim other operating systems or npm artifacts were tested solely because local source builds pass.

## Release checklist

- [ ] Every declared syntax feature has positive, negative, version and formatting evidence.
- [ ] Valid corpora preserve tokens, literals, comments and protected bytes without unexplained changes.
- [ ] Invalid inputs never produce writable partial output.
- [ ] Layout configuration, schema, help and host APIs agree.
- [ ] File-failure tests prove no truncation and stable exit/JSON contracts.
- [ ] Rust, standalone and npm results agree on every declared platform.
- [ ] Final artifact installation, exact dependencies and release recovery are verified.
- [ ] Limitations, layout changes and upgrades are documented.
