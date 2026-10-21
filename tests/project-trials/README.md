# Formatter project trials

This optional gate formats pinned third-party GDScript sources in memory. Ordinary
Rust tests need no network, Python or Godot installation. No project code is run.

`manifest.json` enumerates every `.gd` file at the recorded commits, including one
known invalid upstream file. The corpus includes Godot demos, Kenney's 3D
platformer and Godot State Charts (including its bundled GUT addon, examples and
tests). It also pins the applicable license notices: Godot, KidsCanCode, Kenney,
State Charts, GUT and the cooldown demo's asset attribution. Only scripts
and these notices are downloaded, into ignored `target/`; scenes, assets and
project imports are outside this trial. Inline copyright/SPDX notices are retained.

```sh
pnpm install
python3.12 -m venv target/project-trials/venv
target/project-trials/venv/bin/python -m pip install --require-hashes -r tests/project-trials/requirements.txt
pnpm test:trials
GDFMT_TRIAL_PYTHON=target/project-trials/venv/bin/python pnpm test:trials
pnpm trials --fetch --godot /path/to/pinned/godot-4.7
```

Omit `--fetch` for subsequent offline runs. Downloads and cached inputs must match
their SHA-256 digests. Extra `.gd` files fail the inventory check. The runner checks
all ten Python dependency versions against the hash-locked requirements, and the
Godot binary against the existing exact 4.7 engine pin. The tested Python version
is 3.12.14. The runner resolves `venv/Scripts/python.exe` on Windows; `--python`
overrides the interpreter path on any platform. Set `GDFMT_TRIAL_PYTHON` using your
shell's environment-variable syntax when running the optional bridge test.

The TypeScript runner prepares inputs, calls `cargo run --locked -p xtask --
project-trials --input <JSON> --report <JSON>`, compares gdtoolkit 4.5.0 at width 100,
and checks `reviewed.json`. Rust uses the validated facade for four layouts under
the explicitly fixed 4.7 language profile: tab/100/preserve, tab/40/preserve,
two spaces/40/CRLF/no blank lines/no final newline, and four spaces/100/LF/one
blank line/final newline. This exercises reparsing, exact significant CST/literal
spelling, comment attachment and idempotence. Any unexpected rejection or failed
invariant fails the gate. The known source rejection must return no writable
output and its exact diagnostic set; an isolated check-only run of the original
source must also reproduce its recorded engine diagnostic.

TypeScript owns downloads, hashing, inventory validation, difference categories,
Git unified diffs, reports and review fingerprints. A small inline Python adapter
is necessary for gdtoolkit's Python formatter/parser APIs and checks the locked
dependency versions. There are no repository `.py` scripts. `pnpm test:trials`
runs the native guards without Python in the existing npm native CI matrix;
`GDFMT_TRIAL_PYTHON` additionally enables
the real gdtoolkit adapter test. `pnpm typecheck` includes both runner and tests.

Reports are written to `target/project-trials/`: `layouts.json` records sources,
all outputs and diagnostics; `comparisons.json` records gdtoolkit output;
`diffs/<project>/<path>.diff` contains every width-100 comparison; `summary.json`
includes the engine rejection log. `review-candidate.json` can be compared with
the checked-in review. Every output pair has a SHA-256 fingerprint and reviewed
categories, so a layout change also requires renewed review.

`gdtoolkitRejections` in the manifest records exact comparator parse diagnostics.
Eight State Charts scripts start with a UTF-8 BOM, and one has own-line comments
after explicit continuations; gdtoolkit 4.5.0 rejects these sources. These cases
still pass every gdfmt invariant layout and retain their gdfmt output fingerprints.
The comparison output/hash is null and the category is
`gdtoolkit-rejected-gdfmt-accepted`. Unexpected failures or a pinned rejection
that no longer reproduces fail the gate. Expected-token sets are sorted before
comparison because Lark's diagnostic rendering depends on Python's hash seed.

The category classifier is a review aid, not a semantic-equivalence checker. It
explains whitespace/layout, quotes, parentheses/commas/semicolons and the two
reviewed inner-class `extends` placements. `comment-trailing-whitespace` identifies
gdtoolkit removing spaces/tabs at the end of comments, which gdfmt preserves.
It rejects other token differences and changed comment content/order. The
independent Rust gate compares each gdfmt output to its original CST. This does
not prove that gdtoolkit's rewriting is
equivalent, that projects import successfully, or that scripts behave identically.

After inspecting changed diffs and investigating every unexplained change, use
`--record-review` to replace the reviewed fingerprints, then rerun without that
option. Include the resulting validation summary in your pull request.

The original, reduced `formatter-continuation-comments.gd` fixture reproduces a
State Charts failure where dropping a backslash before an own-line comment
made formatted code invalid. It participates in the existing compatibility,
validated-facade and Godot 4.3–4.7 gates. Formatter tests also vary width,
indentation, LF/CRLF and final-newline policy and inject continuation comments at
token boundaries. These regressions run in ordinary CI without fetching projects
or installing Python dependencies; the complete project trial remains optional.

Reviewed source-only run on 2026-10-21 (the full commit and file hashes are in the
manifest):

| Project                                   | Commit prefix  | Scripts | Accepted by gdfmt |
| ----------------------------------------- | -------------- | ------: | ----------------: |
| Godot demos                               | `6ad6167e0577` |     456 |               455 |
| Kenney 3D platformer                      | `3fa8a04b1c01` |       9 |                 9 |
| Godot State Charts, bundled GUT and tests | `76d226a3efac` |     141 |               141 |
| Total                                     |                |     606 |               605 |

All 2,420 accepted-script layouts passed; the existing invalid lambda dedent was
rejected in all four layouts. At width 100, 67 outputs matched gdtoolkit, 529 had
reviewed differences, and nine reproduced the pinned gdtoolkit rejections. No
differences remained unexplained. All 465 pre-existing case fingerprints were
unchanged. The run without `--record-review` also passed under a different
`PYTHONHASHSEED`.
