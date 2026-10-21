# P7 installed artifact acceptance

Accepted on **2026-10-21** for package **0.1.0-alpha.0**, tested revision
`5abcb012ebaba76481cea4f3328e624ca0007568`. Nothing was published.

The [npm artifact workflow](https://github.com/ntnyq/gdfmt/actions/runs/39592751263)
passed all ten installed consumers and the aggregate `parity` job. The
[Rust workflow](https://github.com/ntnyq/gdfmt/actions/runs/39592751260) passed all
ten jobs: quality, Rust 1.88 MSRV, both feature configurations on Linux/macOS/Windows,
and all five pinned engine comparisons.

| Native target           | Node 22.13.0                  | Node 24.0.0                   |
| ----------------------- | ----------------------------- | ----------------------------- |
| macOS arm64             | 8 tests + declarations passed | 8 tests + declarations passed |
| macOS x64               | 8 tests + declarations passed | 8 tests + declarations passed |
| Windows x64 MSVC        | 8 tests + declarations passed | 8 tests + declarations passed |
| Linux x64 glibc         | 8 tests + declarations passed | 8 tests + declarations passed |
| Linux x64 musl (Alpine) | 8 tests + declarations passed | 8 tests + declarations passed |

These **80 passing Vitest tests** include **13,100 fixture/profile comparisons**
(262 scripts × five language profiles × ten consumers), final standalone Rust
output equality and idempotence. Every consumer installs the actual packed root
through an isolated registry, resolves its optional native package with an empty
cache, and runs with a PATH containing only Node. ESM/CJS exports and installed
declarations, sync/async formatting and configuration, UTF-8 diagnostics,
protected regions, CLI stdout/stderr and exit codes, guarded writes, worker
scheduling and loader failures all passed.

The aggregate gate verified each tested tarball's SHA-512, exact package versions
and target metadata. All ten root tarballs have the same SHA-512:

```text
047bf4ba6b2b248de8a6f53008ac8dd4218fd34f82d981f75b29e70915de386ac9e75bf401ab87c9d224cfd4800287109e313474c2143fdc787a8b39626bb3ec
```

[acceptance.json](acceptance.json) retains the tested revision, workflow links,
all ten runtime/target pairs and each root/native tarball hash. The CI aggregate
reported `completeMatrix: true` and `published: false`; native hashes identify each
job's tested build, while byte identity across jobs is required for the root.
Downloaded tarballs, ten actual Vitest reports and individual receipts are
retained locally under ignored `target/p7-ci/accepted-matrix/`. Independent local
verification of that download passed:

```sh
pnpm verify:npm target/p7-ci/accepted-matrix
```

## Windows failure and regression

The [initial artifact run](https://github.com/ntnyq/gdfmt/actions/runs/39584231649)
passed eight combinations but failed both Windows consumers. Its JSON-only
reporter hid the failing assertion from the console, and failure skipped artifact
upload. The [diagnostic run](https://github.com/ntnyq/gdfmt/actions/runs/39591746034)
retained reports and showed seven passing tests with one CLI parity failure:
standalone `--help` rendered `Usage: gdfmt.exe`, while the npm shim rendered
`Usage: gdfmt`.

The shared Clap command now explicitly fixes its binary name to `gdfmt`.
`help_and_argument_errors_ignore_launcher_filename` reproduces the old failure
on any host using `.exe`, renamed and path-qualified launchers. It checks help,
version, subcommand help and argument errors. The regression failed before the
fix and passed afterward; all 20 local CLI tests passed, followed by both Windows
installed consumers in the complete matrix. The console reporter and failure
uploads remain enabled.

## Action dependencies and scope

Official sources were checked on 2026-10-21: checkout 7.0.1, setup-node 7.1.0,
upload-artifact 7.0.2, download-artifact 8.0.2, pnpm/action-setup 6.1.0, and
rust-toolchain master commit `683b588c487a69d01f6c6ddf40f036ff682cc2c8`. The upgrade
is folded into the initialization commit. Rust 1.98.0, Rust 1.88 MSRV and the two
Node test versions remain fixed. Local workflow static validation, npm consumer
tests, typechecking and bilingual documentation builds passed.

Acceptance covers the tested hosted runners, architectures and Node versions.
Linux glibc artifacts target Ubuntu 24.04; older glibc, other OS versions and
unlisted architectures have not been validated. Name ownership, publishing
permissions, registry publication and recovery remain untested. The workflow has
no publication job.
