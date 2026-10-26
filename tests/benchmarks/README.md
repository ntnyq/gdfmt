# Formatter CLI benchmarks

The [second profiling and optimization report](OPTIMIZATION-2.md) records the
latest core and bounded-batch changes, safety validation and paired measurements.
The [first optimization report](OPTIMIZATION.md) remains the previous baseline.

This optional benchmark compares gdfmt with three independent GDScript formatters:

| Tool                       | Version used for the recorded run                           | Upstream                                                                                        |
| -------------------------- | ----------------------------------------------------------- | ----------------------------------------------------------------------------------------------- |
| gdfmt                      | Local release build; revision and binary hash in the result | This repository                                                                                 |
| gdformat / gdtoolkit       | 4.5.0                                                       | [Scony/godot-gdscript-toolkit](https://github.com/Scony/godot-gdscript-toolkit)                 |
| GDQuest GDScript Formatter | 0.27.0                                                      | [GDQuest/GDScript-formatter](https://github.com/GDQuest/GDScript-formatter/releases/tag/0.27.0) |
| gdstyle                    | 0.4.0                                                       | [atelico/gdstyle](https://github.com/atelico/gdstyle/releases/tag/v0.4.0)                       |

## Reproduce

Use the repository's pinned Rust toolchain and a release build, with Python 3.12
for the locked gdtoolkit environment:

```sh
pnpm install
cargo build --release --locked -p gdfmt_cli
python3.12 -m venv target/project-trials/venv
target/project-trials/venv/bin/python -m pip install --require-hashes -r tests/project-trials/requirements.txt
```

Download and extract the native GDQuest 0.27.0 and gdstyle 0.4.0 binaries from the
release pages above for your platform. Check the asset SHA-256 published by
GitHub. Keep tools under ignored `target/` or supply paths to existing installs.
The benchmark does not install tools or change your PATH.

```sh
pnpm bench --fetch --gdquest /path/to/gdscript-formatter --gdstyle /path/to/gdstyle
pnpm test:bench
pnpm typecheck
```

`--fetch` downloads the pinned trial scripts and license notices; omit it for
offline runs with an existing cache. Sources, licenses, full script inventory
and SHA-256 hashes are checked by the existing project-trial downloader.
No GDScript is executed, and no Godot installation is needed.

The default `--gdfmt` is `target/release/gdfmt`; the default `--gdformat` is
`target/project-trials/venv/bin/gdformat`. On Windows, supply the executable under
the virtual environment's `Scripts` directory. All four binaries can be
overridden. The runner reports their actual versions and executable hashes;
it does not enforce the comparator versions in the table.

Other options:

| Option       | Default                          | Purpose                                                               |
| ------------ | -------------------------------- | --------------------------------------------------------------------- |
| `--runs`     | `10`                             | Measured invocations per tool and workload                            |
| `--warmup`   | `2`                              | Untimed invocations per tool and workload                             |
| `--timeout`  | `120000`                         | Maximum milliseconds per invocation                                   |
| `--cache`    | `target/project-trials`          | Existing hash-checked trial cache                                     |
| `--output`   | `target/benchmarks/results.json` | JSON report, checkpointed after each workload                         |
| `--mode`     | `write`                          | Measure in-place formatting or non-writing `check` invocations        |
| `--baseline` | unset                            | Interleave an old gdfmt binary and require identical canonical output |

To measure an optimization, preserve the old release executable before rebuilding,
then pass it with `--baseline`. The runner times it as `gdfmt-before` alongside the
new binary and the three comparators, rotating all five tools in each round.
The old and new gdfmt output fingerprints must match for every workload.

`--mode check` still prepares and validates reference output using untimed writes
to disposable copies. Timed invocations use `--check`, expect exit 1 when reference
output differs, and assert that every input byte remains unchanged. The `output`
fingerprint in this mode describes the untimed canonical output. No safety checks
are disabled by the harness. The default write mode preserves the original
benchmark behavior.

```sh
pnpm bench --mode check --baseline /path/to/old-gdfmt --gdquest /path/to/gdscript-formatter --gdstyle /path/to/gdstyle --output target/benchmarks/check.json
pnpm bench --mode write --baseline /path/to/old-gdfmt --gdquest /path/to/gdscript-formatter --gdstyle /path/to/gdstyle --output target/benchmarks/write.json
```

## What is measured

The timer wraps one synchronous CLI process, from spawn to exit, including
startup, configuration discovery, directory discovery, parsing, formatting,
default validation, output logging and, in write mode, file replacement. stdout and stderr are drained
through pipes for every tool. Builds, downloads, preflight, input restoration,
output hashing and report writing are outside the timer. No shell or package
runner is included inside a measured invocation.

Every invocation receives fresh copies of the **same original bytes** in an
isolated temporary directory. Repeated runs do not accidentally measure already
formatted output. OS and filesystem caches are warm; these are fresh-process
latencies, not cold-disk or in-process library benchmarks. Tool order rotates
between rounds, and no formatter processes run concurrently. The first tool
rotates in a fixed order, rather than a random order. No outliers are discarded.
Internal tool threads are not restricted. The second optimized gdfmt CLI uses up
to four workers for batches of at least eight sources when every source is at
most 256 KiB; otherwise it stays serial. The recorded synthetic files are single
requests and the combined real corpus qualifies for the bounded worker path.

Width 100 and tab indentation are used. gdfmt selects the Godot 4.7 profile.
Other formatting policies and safety checks remain at each tool's defaults:
gdfmt's validated facade, gdformat's normal safety checks, GDQuest without its
optional `--verify-structure`, and gdstyle's normal formatter. No `--fast` flag
is used. The tools produce different canonical output and perform different
amounts of validation. A successful exit is not proof of semantic equivalence;
gdstyle can also successfully exit on syntactically invalid input. The benchmark
checks nonempty, repeatable output and an adapter smoke case that must change,
not correctness or equal formatting quality. Memory and CPU time are not measured.

Workloads:

- `small-file`: 5 generated functions, 56 lines, 1,473 bytes. Mainly exposes
  process startup and short editor-save latency.
- `large-file`: 500 generated functions, 5,501 lines, 147,793 bytes. Exercises
  comments, type annotations, dictionaries, arrays, expressions and nested suites.
  This is synthetic scaling evidence, not a representative large production file.
- `real-projects`: a common subset of the pinned Godot demos, Kenney platformer
  and Godot State Charts/GUT corpus from `tests/project-trials/manifest.json`.
  This is one combined batch invocation per tool, not one process per source file.

Known invalid upstream sources are excluded using the trial manifest. Every tool
then attempts every remaining script. Failed batches are bisected to identify
individual rejections, recorded with the tool, original source path and diagnostic.
Every tool is timed on the intersection with no such rejections. Missing binaries,
timeouts and process signals fail the run instead of becoming source exclusions.

Scripts are given flat temporary names to avoid differing `addons`, hidden-file
and project-directory exclusions; gdstyle's `exclude` is explicitly empty.
Original paths and input hashes remain in the report. Thus this tests formatting
the selected scripts, not each tool's real-world project ignore/configuration
behavior. Each tool/workload has an untimed reference pass; every measured output
in write mode must exactly match that tool's reference digest; check mode must
leave the original input fingerprint unchanged. Original sources and license
notices in the cache remain untouched.

The JSON includes environment details, Git revision/dirty state, tracked-diff hash,
runner hashes, executable hashes/versions, upstream project commits, input hashes,
every exclusion, per-invocation milliseconds, output fingerprints and changed-file
counts. Statistics include median, mean, min/max, nearest-rank p95, population
standard deviation and input MiB/s based on the median. The runner hashes the
supplied gdfmt executable but cannot prove it was built from the recorded source
revision; rebuild it immediately before collecting results. Compare complete
reports from the same machine and tool versions. A partial checkpoint is not a
complete benchmark report.

## Recorded run: 2026-10-26

[Full JSON results](2026-10-26-macos-arm64.json) retain all 120 measured samples,
594 real input hashes, output fingerprints and 12 rejection records. This run
used Apple M1 Pro (10 logical CPUs), 16 GiB RAM, Darwin 27.0.0 arm64, Node
24.22.1, Python 3.12.14 and Lark 1.2.2. gdfmt 0.1.0 was built with the pinned
Rust 1.98.0 toolchain using `cargo build --release --locked -p gdfmt_cli`
(the repository release profile uses thin LTO). The two other Rust tools used
upstream native release binaries.

The gdfmt source revision was `deb34424004438f3f5cd787a162cbfe4568372d0`.
`dirty: true` records the local benchmark additions; formatter Rust sources were
unchanged. GDQuest 0.27.0's macOS aarch64 ZIP had SHA-256
`e7d9fe240d5c5630b8738f6e0616a2473807037b9d10b70a11690ab1a73bc5bc`;
gdstyle 0.4.0's aarch64 Apple Darwin tarball had SHA-256
`0599c5860c99f147f91ed7233c4b5d085deb39a6e3604ea7576c0158f10ee83d`.
Both were checked against the release asset digests before extraction. The
JSON separately records the SHA-256 of each extracted executable.

Each tool/workload used two warmup invocations and ten measured invocations,
plus the untimed reference pass. Median wall time in milliseconds (lower is better):

| Workload                                           | gdfmt 0.1.0 | gdformat 4.5.0 | GDQuest 0.27.0 | gdstyle 0.4.0 |
| -------------------------------------------------- | ----------: | -------------: | -------------: | ------------: |
| Small file: 56 lines / 1.44 KiB                    |       11.78 |         182.16 |           5.03 |          5.48 |
| Large file: 5,501 lines / 144.33 KiB               |      167.35 |         926.55 |          39.83 |        164.81 |
| Real projects: 594 files / 48,327 lines / 1.47 MiB |    3,962.07 |       4,527.06 |         132.54 |        254.82 |

gdfmt used about 15.5x less time than gdformat for the small synthetic input and
5.5x less for the large input. For the combined real corpus, the median ratio
was only 1.14x, with substantial variation: gdfmt ranged from 3.83 to 7.49 seconds,
gdformat from 4.14 to 12.19 seconds, GDQuest from 122 to 200 milliseconds and
gdstyle from 214 to 667 milliseconds. All samples, including a round with broad
slowdowns, are retained. The batch result does not establish a reliable small
performance advantage over gdformat on other runs or machines. GDQuest and
gdstyle completed this batch much faster than gdfmt under their respective
default validation and formatting policies. These measurements do not identify
which internal phase accounts for the difference.

The original trial inventory has 606 scripts. One known invalid source was
excluded before tool preflight; gdformat rejected nine other scripts (eight with
a UTF-8 BOM and one with continuation comments), and GDQuest rejected two Godot
demo scripts. These are distinct paths, leaving 594 common inputs totaling
1,544,095 original bytes. gdfmt and gdstyle completed all 605 valid-source
candidates successfully. This is CLI acceptance, not independent semantic
validation of comparator output. See the JSON for exact paths and diagnostics.

The measured invocation was:

```sh
pnpm bench --gdquest target/benchmarks/tools/gdscript-formatter-0.27.0-macos-aarch64 --gdstyle target/benchmarks/tools/gdstyle --output target/benchmarks/2026-10-26-macos-arm64.json
```

The generated JSON was copied here as a historical result. New runs write to
ignored `target/` by default; retain the full report when making performance claims.

## Follow-up: startup, validation and writes

A separate diagnostic run on the same machine distinguished fresh-process cost,
formatting/validation and file replacement. These numbers are separate samples,
not a decomposition obtained by subtracting the original benchmark medians.

Thirty measured CLI invocations per tool put `--version` around 2.9–3.1 ms and
formatting an empty file around 3.2–3.3 ms for all three Rust tools. Startup alone
does not explain the small-file gap.

The [write-versus-check samples](2026-10-26-write-vs-check.json) used identical
original inputs, rotated tool/mode order and two warmups. The small case has 20
measured invocations per pair; the 594-file corpus has five. `--check` avoids file
replacement while retaining each tool's checking behavior. The corpus is the same
594-script intersection as above; no project code is executed.

| Tool    | Small: write | Small: check | 594 files: write | 594 files: check |
| ------- | -----------: | -----------: | ---------------: | ---------------: |
| gdfmt   |     11.04 ms |      6.00 ms |         4,331 ms |         2,121 ms |
| GDQuest |      4.62 ms |      4.38 ms |           165 ms |           114 ms |
| gdstyle |      5.46 ms |      5.38 ms |           324 ms |           365 ms |

gdfmt's guarded replacement creates a temporary file, preserves permissions,
calls `File::sync_all()`, rechecks the original source and atomically replaces it
([implementation](../../crates/gdfmt_workspace/src/writing.rs)). A separate Rust
1.98.0 release-mode probe timing only `File::sync_all()` on freshly written
1,473-byte temporary files measured approximately 4.98 ms median over 30 samples.
This supports synchronous file durability as a major contributor to small-file
write latency. It is filesystem/platform dependent. Small differences between
write/check medians, such as gdstyle's batch result, are within the observed
environmental variation and should not be interpreted as a causal speedup.

In-process release-mode probes on the same small input measured approximately
0.10 ms for parsing, 0.38 ms for formatting an existing tree and 0.48 ms for one
parse-plus-format pass. The full validated facade took 1.52–1.63 ms across two
probe runs. It additionally reparses output, compares tree/token/comment structure,
checks protected regions and formats again to verify idempotence
([implementation](../../crates/gdfmt/src/lib.rs)). These checks add real work;
the small-file CLI write result is not the formatter's pure computation time.

The 594-file no-write result still has a substantial gap. Synchronous replacement
does not explain all performance differences, and these probes do not isolate a
specific core algorithm as the remaining bottleneck. A sampling profile and
comparisons under matched validation policies are needed before choosing a fix.
