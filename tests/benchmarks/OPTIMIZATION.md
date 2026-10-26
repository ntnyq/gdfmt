# Core profiling and optimization, 2026-10-26

The performance changes preserve the validated facade and guarded writer.
Output reparsing, exact significant-token spelling and tree topology, comment
attachments, protected-region bytes and second-pass idempotence remain checked.
The file replacement path still performs permission/identity/content checks,
`sync_all()` and atomic replacement. No fast mode, parallelism or validation
policy change was introduced.

## Findings and implementation

A release build with line information was sampled on the 594-script common
corpus using macOS `sample` at a 1 ms interval for five seconds. The process
repeatedly called the full validated facade with sources already in memory.
An independent staged harness timed parsing, layout, output reparsing, structural
comparison, second layout and the full facade. The staged report sums the median
of three measurements **per file**; it is not a median of whole-corpus runs.
Sources match the initial benchmark's real workload. The in-process harness uses
`FormatRequest::default()` (including preserved source line endings); the paired
CLI benchmark uses the explicit settings described in the benchmark README. All stages
check successful parsing, structural equivalence, repeatable formatting and
agreement with the validated facade.

The [summary](profiles/2026-10-26-summary.json) and sanitized per-file
[before](profiles/2026-10-26-before.tsv)/[after](profiles/2026-10-26-after.tsv)
timings retain the exploratory measurements. The original symbolized sample
reports and throwaway harness remain in ignored `target/benchmarks/` locally.
The large sampling logs are not checked in.

Before optimization, `LosslessTree::text` appeared in 1,010 of 3,361 active-root
samples (about 30%). `formatter_regions` appeared in 973 (about 29%). These are
inclusive, overlapping call-stack counts and must not be added. The expected
comment-owner search was a minor contributor. Much of the work was repeated
whole-tree traversal, red-cursor allocation and temporary document/shape storage.

Four changes remove redundant work while retaining the checks:

1. `LosslessTree::text` traverses borrowed green children with an explicit stack
   and a correctly sized output string. Reconstructing source bytes no longer
   creates a parent-aware red cursor for each syntax element. Traversal remains
   iterative and retains every token, including recovery/trivia text.
2. Directive scanning reuses one reconstructed source. If the exact required
   `gdfmt:` prefix is absent, no comment can be a directive. If it is present,
   existing token, standalone-line, pairing and span checks still run; text in
   string literals or suffix comments is still ignored.
3. Structural validation compares corresponding nodes and borrowed token text
   directly, retaining all node boundaries and significant spellings. It avoids
   allocating two complete shape vectors and a String per significant token.
   The independent comment-association comparison is unchanged.
4. Document concatenation removes empty text/concat documents and returns single
   children directly. This removes heap containers around ordinary tokens.
   Empty-width line breaks, suffixes, indentation and grouping remain meaningful
   controls and are retained.

| Staged in-process total   |    Before |     After |
| ------------------------- | --------: | --------: |
| Parse input               |  79.53 ms |  78.88 ms |
| First layout              | 239.84 ms | 139.98 ms |
| Reparse output            |  76.58 ms |  76.17 ms |
| Structural comparison     | 185.72 ms | 122.32 ms |
| Second layout             | 237.74 ms | 140.13 ms |
| Complete validated facade | 980.10 ms | 570.45 ms |

These exploratory totals suggest about 42% less full-facade time; the stages
exclude different setup/drop work and are not additive to the facade row.
Individual intermediate runs have environmental variation and are not a causal
allocation of savings across changes. In a follow-up sample, source reconstruction
appeared in 151/3,055 active-root samples (about 5%), and directive-region scanning
in 85 (about 3%). This supports removal of the initial hotspot.

## Paired CLI benchmark

The original optimized release executable was preserved before changing Rust
sources. Its SHA-256 matches the initial benchmark artifact. The modified binary
was built with the same pinned Rust toolchain and workspace release profile.
Old and new binaries are interleaved with the three fixed comparators on identical
fresh inputs. Each pair uses two warmups and ten measured invocations; all output
fingerprints must match between old and new gdfmt. Check and write modes are
collected separately, without compiling or running tests concurrently.

[Check-mode samples](2026-10-26-optimized-check.json) and
[write-mode samples](2026-10-26-optimized-write.json) retain all 300 measured
invocations, executable/input hashes, exclusions and output fingerprints. The
common corpus remains exactly 594 scripts and 1,544,095 original bytes. The
preserved baseline executable SHA-256 matches the gdfmt executable in the
[initial report](2026-10-26-macos-arm64.json).

Median milliseconds from these paired runs:

| Workload              | Mode  |   Before |    After | Less time |
| --------------------- | ----- | -------: | -------: | --------: |
| 56-line small file    | check |     5.33 |     4.48 |     15.9% |
| 5,501-line large file | check |   153.92 |    86.85 |     43.6% |
| 594 real scripts      | check | 1,121.75 |   738.20 |     34.2% |
| 56-line small file    | write |    11.60 |    11.28 |      2.8% |
| 5,501-line large file | write |   162.55 |    96.34 |     40.7% |
| 594 real scripts      | write | 3,881.09 | 3,490.19 |     10.1% |

The small-file write difference is minor and should not be treated as a robust
speedup. The unchanged synchronous writer is still a substantial cost for many
small files. Check mode includes CLI/configuration/discovery work and the complete
validated facade, so it is closer to the core path than write mode but is not a
pure library benchmark. These results compare the preserved old and new binaries
within each run; they should not be calculated by subtracting medians from the
earlier diagnostic session under different machine load.

GDQuest and gdstyle remain faster on this combined batch under their own policies:

| Tool                     |  Check mode |  Write mode |
| ------------------------ | ----------: | ----------: |
| gdfmt after optimization |   738.20 ms | 3,490.19 ms |
| gdformat 4.5.0           | 3,850.68 ms | 4,008.84 ms |
| GDQuest 0.27.0           |    99.72 ms |   129.29 ms |
| gdstyle 0.4.0            |   201.37 ms |   211.21 ms |

This is a comparison of each tool's default functionality and validation, not
matched amounts of work. The change improves gdfmt's implementation while keeping
its existing validation and write guarantees.

The exact collection commands were:

```sh
pnpm bench --mode check --baseline target/benchmarks/tools/gdfmt-before-optimization --gdquest target/benchmarks/tools/gdscript-formatter-0.27.0-macos-aarch64 --gdstyle target/benchmarks/tools/gdstyle --output target/benchmarks/2026-10-26-optimized-check.json
pnpm bench --mode write --baseline target/benchmarks/tools/gdfmt-before-optimization --gdquest target/benchmarks/tools/gdscript-formatter-0.27.0-macos-aarch64 --gdstyle target/benchmarks/tools/gdstyle --output target/benchmarks/2026-10-26-optimized-write.json
```

## Validation

Eight focused regression tests cover byte-exact green text reconstruction,
empty/recovery nodes, directive recognition and pairing, structural topology,
literal/comment changes, and empty document wrappers versus real line breaks.
The document-wrapper test failed against the original concatenation behavior and
passed after normalization; no timing threshold is placed in ordinary CI tests.

The full pinned project trial passed: 606 source scripts, 605 valid scripts,
2,420 invariant layouts, one expected invalid source, nine known gdtoolkit
rejections, zero unexpected failures and zero unexplained output differences.
All 606 existing reviewed fingerprints remained unchanged; no review records or
expected formatter output were updated.

Workspace tests with all features and without default features, workspace Clippy
with warnings denied, Rust formatting, compatibility fixtures, syntax coverage,
TypeScript type checking and the benchmark harness tests passed.
