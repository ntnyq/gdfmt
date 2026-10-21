# npm package integration tests

These tests install the actual packed root package and its optional native dependency through an isolated local registry with an empty cache. Consumers run with only Node on `PATH`; Rust is required to build the artifacts, not to consume them.

```sh
pnpm typecheck
pnpm test:npm
pnpm verify:npm --local
```

Vitest checks ESM/CJS exports and installed declarations, sync/async formatting and configuration, UTF-8 diagnostics, protected regions, CLI parity, guarded writes, worker scheduling and loader failures. Fixtures are compared against the Rust facade for every language profile. The CLI regression also checks that help and argument errors are independent of launcher filenames such as `gdfmt.exe`.

The CI matrix covers five native targets on Node 22.13.0 and 24.0.0. `pnpm verify:npm` requires all ten receipts, matching package versions and target metadata, verified tarball hashes and byte-identical root tarballs. `--local` verifies only the current target/runtime and records an incomplete matrix.

[acceptance.json](acceptance.json) preserves the recorded complete-matrix result, tested revision, workflow links and tarball hashes. It describes those tested artifacts, not every future build. No package was published. Release requirements and reproduction commands are in [contributing](../../CONTRIBUTING.md#nodejs-packages).
