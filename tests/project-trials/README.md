# P4 project trials

This optional gate formats pinned third-party GDScript sources in memory. Ordinary
Rust tests need no network, Python or Godot installation. No project code is run.

`manifest.json` enumerates every `.gd` file at the recorded commits, including one
known invalid upstream file. It also pins the MIT license notices (the Godot root
notice, KidsCanCode's Dodge the Creeps notice and Kenney's notice). Only scripts
and these notices are downloaded, into ignored `target/`; scenes, assets and
project imports are outside this trial. Inline copyright/SPDX notices are retained.

```sh
python3.12 -m venv target/project-trials/venv
target/project-trials/venv/bin/python -m pip install --require-hashes -r tests/project-trials/requirements.txt
target/project-trials/venv/bin/python tests/project-trials/test_run.py
target/project-trials/venv/bin/python tests/project-trials/run.py --fetch --godot /path/to/pinned/godot-4.7
```

Omit `--fetch` for subsequent offline runs. Downloads and cached inputs must match
their SHA-256 digests. Extra `.gd` files fail the inventory check. The runner checks
all ten Python dependency versions against the hash-locked requirements, and the
Godot binary against the existing exact 4.7 engine pin. The tested Python version
is 3.12.14. Use an equivalent virtual-environment path on Windows.

The Python runner prepares inputs, calls `cargo run --locked -p xtask --
project-trials --input <JSON> --report <JSON>`, compares gdtoolkit 4.5.0 at width 100,
and checks `reviewed.json`. Rust uses the validated facade for four layouts under
the explicitly fixed 4.7 language profile: tab/100/preserve, tab/40/preserve,
two spaces/40/CRLF/no blank lines/no final newline, and four spaces/100/LF/one
blank line/final newline. This exercises reparsing, exact significant CST/literal
spelling, comment attachment and idempotence. Any unexpected rejection or failed
invariant fails the gate. The known source rejection must return no writable
output and its exact diagnostic set; an isolated check-only run of the original
source must also reproduce its recorded engine diagnostic.

Reports are written to `target/project-trials/`: `layouts.json` records sources,
all outputs and diagnostics; `comparisons.json` records gdtoolkit output;
`diffs/<project>/<path>.diff` contains every width-100 comparison; `summary.json`
includes the engine rejection log. `review-candidate.json` can be compared with
the checked-in review. Every output pair has a SHA-256 fingerprint and reviewed
categories, so a layout change also requires renewed review.

The category classifier is a review aid, not a semantic-equivalence checker. It
explains whitespace/layout, quotes, parentheses/commas/semicolons and the two
reviewed inner-class `extends` placements. It rejects other token differences
and changed comment text/order. The independent Rust gate compares each gdfmt
output to its original CST. This does not prove that gdtoolkit's rewriting is
equivalent, that projects import successfully, or that scripts behave identically.

After inspecting changed diffs and investigating every unexplained change, use
`--record-review` to replace the reviewed fingerprints, then rerun without that
option. Update the bilingual formatter acceptance record with the results.
