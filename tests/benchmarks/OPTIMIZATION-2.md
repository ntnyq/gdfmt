# Second profiling and optimization pass, 2026-10-26

This pass uses the [first optimized release](2026-10-26-optimized-check.json) as
its preserved baseline. It retains source/output reparsing, exact significant
spellings and tree topology, comment anchors, protected bytes, idempotence and
the existing guarded `sync_all()`/atomic replacement implementation.

## Remaining hotspots and changes

Eight-second macOS sampling and staged measurements on the same 594 inputs
identified document construction, parent-aware CST traversal and comment
comparison as the remaining substantial work. The changes are:

- Compare tree topology and token spelling directly through borrowed green
  children. This retains all node boundaries and token-kind checks without
  creating red cursors during the structural comparison.
- Gather comment-analysis tokens in one CST traversal per tree, then scan the
  collected list for metadata. Store compact source
  spans and compare borrowed comment text; release token cursors before preparing
  the other tree. Significant offsets use first-match binary lookup. Own-line
  versus suffix roles, next-token anchors, ordered dangling-suite ownership,
  EOF behavior and exact comment spelling remain checked.
- Let private document text borrow immutable source slices or static text through
  `Cow`, while generated text can remain owned. Public formatter output remains
  an owned String.
- Stream concatenation-only leaf/wrapper children instead of allocating a child
  vector for each node. Reuse existing owned document vectors and replace
  per-operand temporary vectors with arrays. Empty text/concat normalization
  continues to retain line-break, suffix, grouping and indentation controls.

The [serial profiling summary](profiles/2026-10-26-second-summary.json) and
per-file [before](profiles/2026-10-26-second-before.tsv)/
[after](profiles/2026-10-26-second-after.tsv) measurements retain the exploratory
data. These are sums of per-file medians over three measurements, not whole-batch
medians. The harness uses `FormatRequest::default()` and preserved line endings;
the CLI uses the benchmark configuration. Intermediate measurements had changing
machine load, so they do not isolate each change's contribution.

| Serial in-process stage   | Previous optimized core | This core |
| ------------------------- | ----------------------: | --------: |
| Parse input               |                74.57 ms |  78.94 ms |
| First layout              |               133.69 ms | 125.95 ms |
| Reparse output            |                72.72 ms |  76.72 ms |
| Structural comparison     |               117.24 ms |  64.75 ms |
| Second layout             |               132.76 ms | 123.96 ms |
| Complete validated facade |               539.76 ms | 479.87 ms |

The main measured core improvement is structural comparison; complete serial
facade time fell about 11%. Stage totals exclude different setup/drop work and
should not be added to reproduce the facade row.

Two ideas were not retained. Parser lookahead memoization did not improve the
interleaved experiment and was reverted. Holding token cursors longer does not
enable Rowan's mutable-tree cursor cache for these immutable trees; a small
unpaired timing/RSS trial did not justify retaining them. The raw exploratory
samples, including the modest borrowed/owned-document comparison, remain in the
summary. Neither rejected experiment is present in production code.

## Bounded native CLI batch workers

Serial core improvements alone left substantial batch latency. The native CLI
now processes eligible batches with at most four workers, including the caller.
Eligibility requires at least eight inputs, every source at most 256 KiB and
more than one available CPU. A batch containing a larger file, a smaller batch
or a single-CPU environment stays serial. This limits working-set multiplication
and avoids thread startup for single-file editor requests. No public flag or
configuration schema changed.

All configuration resolution and source reads finish before workers start.
Workers execute the same complete per-file facade and the same guarded writer;
each changed file still calls `sync_all()` before its atomic replacement.
Results are sorted back into input order with stable SourceIds and the same
maximum-error exit-code policy. Invalid sources remain untouched while other
valid files continue as before. Thread creation failure falls back to fewer
workers, with the caller draining remaining jobs. Source count is validated
before any worker can write.

The queue lock is released before parsing, validation or I/O. No red cursor or
parser state is shared between workers. The library, N-API/WASM hosts and LSP
continue to use the pure per-document facade; this scheduling change is confined
to CLI file batches. The writer still provides guarded per-file replacement,
not a multi-file transaction. Cross-file completion order may change; report
order does not.

## Paired benchmark

The original first-pass executable was copied before this pass. Its hash must
match the previous report's `gdfmt` artifact. Both modes rotate it as
`gdfmt-before` alongside the new executable, gdformat 4.5.0, GDQuest 0.27.0 and
gdstyle 0.4.0. Each workload/tool uses two warmups and ten measured invocations.
The corpus and all old/new canonical output fingerprints must remain identical.
Builds, tests and sampling do not run concurrently with these measurements.

This comparison measures complete default CLI behavior. The new eligible batch
can use four CPUs, so its wall-time improvement includes parallelism and is not
an isolated algorithm speedup. The single-file workloads remain serial. Comparator
formatting policies and validation strengths still differ.

[Check-mode samples](2026-10-26-optimized-2-check.json) and
[write-mode samples](2026-10-26-optimized-2-write.json) retain all 300 measured
invocations, executable hashes, input hashes, exclusions and output fingerprints.
All 594 real inputs are unchanged. The largest source is 67,016 bytes, so this
batch qualifies for four workers on the 10-CPU test machine. The preserved
baseline hash matches the previous round's optimized executable.

Median milliseconds from this round's interleaved old/new comparison:

| Workload              | Mode  | Previous optimized release | This release | Less time |
| --------------------- | ----- | -------------------------: | -----------: | --------: |
| 56-line small file    | check |                       5.26 |         5.11 |      2.8% |
| 5,501-line large file | check |                      86.95 |        74.01 |     14.9% |
| 594 real scripts      | check |                     753.75 |       335.16 |     55.5% |
| 56-line small file    | write |                      10.97 |        10.72 |      2.2% |
| 5,501-line large file | write |                      98.12 |        86.39 |     12.0% |
| 594 real scripts      | write |                   3,842.73 |     2,440.86 |     36.5% |

The small-file changes are minor and are not evidence of a robust latency
improvement. Larger single-file gains come from the core changes; batch gains
also include bounded parallel execution. All canonical output fingerprints
match the previous optimized release in both modes.

The same-run batch comparison with the other tools is:

| Tool                              |  Check mode |  Write mode |
| --------------------------------- | ----------: | ----------: |
| gdfmt, previous optimized release |   753.75 ms | 3,842.73 ms |
| gdfmt, this release               |   335.16 ms | 2,440.86 ms |
| gdformat 4.5.0                    | 3,997.83 ms | 4,135.83 ms |
| GDQuest 0.27.0                    |   101.07 ms |   126.42 ms |
| gdstyle 0.4.0                     |   197.92 ms |   196.85 ms |

For check mode, the gdfmt/gdstyle time ratio drops from 3.81x to 1.69x and the
gdfmt/GDQuest ratio from 7.46x to 3.32x within this run. The gap remains, especially
when writing. The unchanged writer still performs per-file identity/permission/
content checks, synchronous flushing and atomic replacement. Parallelism improves
wall time but does not make those operations equivalent to the other tools'
default behavior or remove their filesystem cost.

A separate single-observation macOS `time -l` RSS probe on the same check workload
recorded approximately 11.9 MiB for the previous executable, 11.7 MiB for the new
core with the serial CLI, and 13.3 MiB for the new bounded-worker CLI. These are
peak resident-set observations, not a memory benchmark across input sizes. The
extra active workers increase resource use; large-file batches retain the serial
policy. CPU time was not measured by the benchmark.

Collection commands:

```sh
pnpm bench --mode check --baseline target/benchmarks/tools/gdfmt-before-second-optimization --gdquest target/benchmarks/tools/gdscript-formatter-0.27.0-macos-aarch64 --gdstyle target/benchmarks/tools/gdstyle --output target/benchmarks/2026-10-26-optimized-2-check.json
pnpm bench --mode write --baseline target/benchmarks/tools/gdfmt-before-second-optimization --gdquest target/benchmarks/tools/gdscript-formatter-0.27.0-macos-aarch64 --gdstyle target/benchmarks/tools/gdstyle --output target/benchmarks/2026-10-26-optimized-2-write.json
```

## Validation

The new regressions cover borrowed text storage, owned document-buffer reuse,
long UTF-8 comment runs under LF/CRLF and dedents, worker limits, forced worker
report ordering/SourceIds/read-only behavior, malformed sources and protected
regions in batches, preflight errors before any batch writes, and hard-link
refusal while other batch files continue. The buffer/storage regressions failed
against the previous implementation before their fixes; no wall-clock test
threshold was added to ordinary CI.

Full workspace tests passed with all features and without default features,
along with Clippy with warnings denied, Rust formatting, compatibility fixtures
and syntax coverage. The pinned Godot 4.7.2 project trial again passed 606 source
records, 605 valid sources and all 2,420 layouts. All existing reviewed output
fingerprints remained unchanged, with one expected invalid source, nine known
gdtoolkit rejections and no unexplained differences. No expected outputs or
review records were updated.
