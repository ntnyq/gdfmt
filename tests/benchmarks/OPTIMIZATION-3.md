# Third profiling and optimization pass, 2026-10-27

This pass preserves the [second optimized release](2026-10-26-optimized-2-check.json)
as its executable baseline. It focuses on layout construction and syntax-tree
navigation. Parser behavior, validation requirements, CLI worker policy and
filesystem replacement are unchanged. No check or formatter option was disabled.

## Evidence and implementation

An eight-second macOS sampling profile of repeated serial validated formatting
identified document construction and parent-aware Rowan cursor allocation/freeing
as remaining substantial work. Width probing was a smaller contributor. The
baseline profile classified about 19.6% of sampled stacks under document building,
10.7% under comment-layout preparation and 7.7% under document printing. These
categories include children and count recursive frames only once per stack.

The implementation changes only private formatter internals:

- Construct layout documents through borrowed green-node/token views with byte
  offsets. These views retain all trivia widths and zero-width token positions
  without allocating a red cursor for each child. Parent-aware comment attachment
  and declaration spacing keep their existing Rowan paths.
- Inspect the final significant-child path when testing whether an element ends
  in an inline lambda. Other sibling subtrees cannot end at that final token in
  parsed sources. Inline-lambda classification and block-lambda detection retain
  their previous rules, including nested lambdas and comments.
- Use borrowed green children to check every node and token for error kinds.
  Error diagnostics still reject formatting before any writable output is returned.
- Reserve a bounded iterator's remaining capacity when constructing multi-part
  documents. Retain the existing empty/single-document normalization and all line,
  grouping, suffix and indentation controls.
- Reuse one scratch stack per print operation for width probes, clearing it even
  after early exits. Reuse the current indentation string and its measured column
  instead of allocating indentation strings at every line break and emission.

The new traversal tests compare kinds, token ranges, child order in both directions,
immediate parents and token lookup against Rowan across compatibility fixtures,
including recovery trees, BOM, CRLF, repeated shared token text and virtual tokens.
They caught the empty-first-child `first_token()` boundary during implementation;
the borrowed view now retains Rowan's exact lookup behavior. Lambda-tail results
are compared against the previous full-subtree algorithm across valid fixtures
under both automatic and forced-multiline lambda policies. Focused regressions
also cover caller-built error nodes/tokens without diagnostics, reused fit stacks
after early exits, and indentation dedents with mixed tabs and base columns.

## Serial profiling

[Profiling summary](profiles/2026-10-27-third-summary.json) retains raw serial A/B
batch samples, stage totals, sampled-stack category counts, hashes and RSS probes.
The per-file [before](profiles/2026-10-27-third-before.tsv) and
[after](profiles/2026-10-27-third-after.tsv) tables use the same 594 hash-checked
sources as the CLI benchmark.

The dedicated serial A/B alternated previous/new processes for six rounds. Each
process formats the full corpus five times with `FormatRequest::default()`,
discards its first batch, validates that every result contains output and retains
four batches. The medians of the 24 retained batches per executable were
**484.36 ms before and 392.26 ms after**, about **19.0% less time**. No builds,
tests, sampling or other benchmark processes ran concurrently with this collection.
This isolates the pure facade from the unchanged CLI worker policy and writes.

Staged measurements below are sums of per-file medians over three measurements,
not whole-batch medians. The profiling harness preserves line endings, whereas the
CLI benchmark uses its own explicit configuration. Stage boundaries exclude
different setup/drop work, so adding stages does not reproduce the facade row.
Separate profiling collections also see changing host load; the unchanged parser's
small shifts are environmental variation, not a parser optimization.

| Serial stage              | Previous core | This core |
| ------------------------- | ------------: | --------: |
| Parse input               |      81.03 ms |  74.68 ms |
| First layout              |     130.67 ms |  77.64 ms |
| Reparse output            |      78.89 ms |  73.01 ms |
| Structural comparison     |      65.97 ms |  62.68 ms |
| Second layout             |     125.73 ms |  75.36 ms |
| Complete validated facade |     501.16 ms | 381.52 ms |

First and second layout stage totals fell roughly 40% in this exploratory probe.
After optimization, document construction accounts for about 13.0% of sampled
stacks, while unchanged parent-aware comment preparation and structural comparison
make up larger shares of the faster pipeline. Stack proportions are not absolute
latencies and do not establish per-change causal speedups. The dedicated serial
A/B above and the paired CLI results below are the performance evidence.

## Paired CLI benchmark

Both modes interleave the preserved executable as `gdfmt-before`, the new gdfmt,
gdformat 4.5.0, GDQuest 0.27.0 and gdstyle 0.4.0. Tool order rotates. Each workload
uses two warmups and ten measured invocations per tool, with exact original input
bytes restored outside the timer. The old/new canonical output fingerprints must
match. The runner and all safety policies are unchanged from the previous round.
The preserved binary hash matches the previous report's optimized executable.

The machine remains an Apple M1 Pro with 10 logical CPUs and 16 GiB memory; this
run uses Darwin 27.0.1. Historical reports used Darwin 27.0.0, so improvements are
computed from this run's interleaved baseline, not historical wall times. Both old
and new CLI releases use the same maximum of four eligible batch workers. Single
files remain serial. No builds, tests or profiling overlap timed CLI runs.

[Check-mode samples](2026-10-27-optimized-3-check.json) and
[write-mode samples](2026-10-27-optimized-3-write.json) retain all 300 measured
invocations, artifact hashes, 594 identical real inputs, twelve unchanged excluded
paths and old/new matching canonical output fingerprints. These are medians;
negative time changes mean faster execution.

| Workload              | Mode  | Previous release | This release | Time change |
| --------------------- | ----- | ---------------: | -----------: | ----------: |
| 56-line small file    | check |          4.58 ms |      4.66 ms |       +1.8% |
| 5,501-line large file | check |         74.22 ms |     56.65 ms |      -23.7% |
| 594 real scripts      | check |        329.79 ms |    297.21 ms |       -9.9% |
| 56-line small file    | write |         10.34 ms |     10.46 ms |       +1.1% |
| 5,501-line large file | write |         82.50 ms |     68.88 ms |      -16.5% |
| 594 real scripts      | write |      2,482.18 ms |  2,546.23 ms |       +2.6% |

Small-file changes are within the observed startup/filesystem variation. The
large-file check and write improvements appeared in all ten and nine of ten
same-round pairs respectively. Batch check improved in all ten pairs, about
9.9% by the medians. It does less well than the serial-facade A/B; CLI wall time
also includes unchanged configuration/discovery, source reads, reporting and
bounded-worker scheduling, so the probes are not interchangeable decompositions.

Batch write did not show a reliable improvement: its main-run median is 2.6%
higher, yet the new executable is faster in six of ten pairs and its mean is
slightly lower. The raw ranges overlap substantially: 2,331–2,602 ms for the new
executable and 2,367–2,917 ms for the old. This run does not support a claim that
batch writes are faster. Synchronous guarded replacement remains unchanged.

A [supplemental old/new-only write probe](profiles/2026-10-27-third-write-repeat.json)
alternated ten measured pairs after two warmup pairs, restoring the same original
bytes to fresh files and verifying every canonical output outside the timer.
Its medians were **2,475.81 ms before and 2,502.99 ms after** (1.1% higher).
This follow-up also did not demonstrate a batch-write improvement. Its samples
have broad variation (old: 2,292–4,066 ms; new: 2,370–3,277 ms), so they do not
isolate a deterministic I/O regression caused by the core changes either. The
five-tool results above remain the primary benchmark; the follow-up is retained
rather than replacing the unfavorable write median.

Same-run real-corpus comparison with the other tools:

| Tool                    |  Check mode |  Write mode |
| ----------------------- | ----------: | ----------: |
| gdfmt, previous release |   329.79 ms | 2,482.18 ms |
| gdfmt, this release     |   297.21 ms | 2,546.23 ms |
| gdformat 4.5.0          | 4,158.53 ms | 4,273.42 ms |
| GDQuest 0.27.0          |    97.39 ms |   142.85 ms |
| gdstyle 0.4.0           |   193.24 ms |   221.35 ms |

For batch check, the gdfmt/gdstyle time ratio decreases from 1.71x to 1.54x and
the gdfmt/GDQuest ratio from 3.39x to 3.05x. For the single large-file check the
GDQuest ratio drops from 1.85x to 1.41x. Comparator output policies and default
validation strengths differ: GDQuest's optional structure verification is not
enabled in these runs. These are default-CLI comparisons, not matched-validation
algorithm comparisons.

A separate single-observation macOS `time -l` check-mode RSS probe recorded
13,877,248 bytes before and 13,910,016 bytes after (both about 13.3 MiB). This
is one peak-RSS observation per executable, not a memory benchmark across input
sizes; CPU time is not measured by the CLI benchmark.

Collection commands:

```sh
cargo build --release --locked -p gdfmt_cli
pnpm bench --mode check --baseline target/benchmarks/tools/gdfmt-before-third-optimization --gdquest target/benchmarks/tools/gdscript-formatter-0.27.0-macos-aarch64 --gdstyle target/benchmarks/tools/gdstyle --output target/benchmarks/2026-10-27-optimized-3-check.json
pnpm bench --mode write --baseline target/benchmarks/tools/gdfmt-before-third-optimization --gdquest target/benchmarks/tools/gdscript-formatter-0.27.0-macos-aarch64 --gdstyle target/benchmarks/tools/gdstyle --output target/benchmarks/2026-10-27-optimized-3-write.json
```

## Safety and validation

The pure facade still parses input and output, compares complete significant
structure/spellings and comment anchors, checks protected bytes and formats a
second time to verify idempotence. The guarded writer still checks identity,
permissions and source contents, calls `sync_all()` and atomically replaces each
changed file. The facade, guarded writer, CLI worker implementation, parser,
expected output snapshots and project-trial reviewed records were not modified.
The error-tree guard changes its traversal, not which node/token kinds it rejects.

Validation passed:

```sh
cargo test --workspace --all-features --locked
cargo test --workspace --no-default-features --locked
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo fmt --all -- --check
cargo xtask compatibility
cargo xtask syntax-coverage
pnpm test:bench
pnpm trials --godot /path/to/pinned/godot
```

The pinned Godot 4.7.2 project trial again covered 606 source records, 605 valid
sources and 2,420 layouts, with one expected invalid source, nine known gdtoolkit
rejections and zero unexplained output differences. No expected outputs or review
records were updated.
