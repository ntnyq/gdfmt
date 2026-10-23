# Language support and limitations

gdfmt formats UTF-8 GDScript `.gd` files. It provides grammar profiles for Godot **4.3, 4.4, 4.5, 4.6 and 4.7**, with **4.7** as the default. Select the profile matching your project:

```json
{
  "language": { "godotVersion": "4.5" }
}
```

Profiles check verified syntax differences. They do not declare complete compatibility with a Godot release. Patch-version strings, `latest`, Godot 3.x and unknown versions are rejected. See [language versions](/reference/language-versions) for feature introduction boundaries.

## Supported formatting

Formatting covers expressions, calls, arrays and dictionaries, annotations, declarations, functions and lambdas, properties, control-flow suites and match patterns. Comments retain their text, order and attachment. Multiline literal contents and [protected regions](/reference/formatting#protected-regions) retain their original bytes.

The formatter checks the output by reparsing, comparing syntax and comments, and verifying idempotence. It refuses writable output when input syntax or output validation fails. These checks protect the formatter's contract; they do not prove runtime equivalence.

## Known boundaries

| Area                | Boundary                                                                                                                                                            |
| ------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Project semantics   | Type/name resolution, inheritance, abstract overrides, resource existence and runtime behavior remain Godot's responsibility                                        |
| Annotation values   | Names, targets, arity and selected literal restrictions are checked; general resolved arguments, export/RPC values and warning-name catalogs are not fully analyzed |
| Unicode identifiers | Pinned Godot Unicode XID tables are used, but the engine's TextServer confusable-keyword security checks are not reproduced                                         |
| Reserved constructs | `namespace` and `trait` are not supported declarations; token recognition does not add support                                                                      |
| Formatting options  | Quote conversion and trailing-comma insertion/removal options are unavailable; literal spelling and existing punctuation are preserved                              |
| Line width          | Target at supported break points; long expressions/chains, indivisible tokens/types and retained comments/literals/protected regions may exceed it                  |
| File types          | Godot scene/resource files, shaders and embedded GDScript are not formatted                                                                                         |
| Editor tooling      | Formatting-only stdio LSP and desktop/Web VS Code extensions are available; range formatting and project analysis are outside the current editor scope              |

Lint rules and semantic auto-fixes are outside the formatter's scope. A successful `gdfmt check` is a syntax/formatting result; continue using Godot to validate the project.

See [editor integration](/guide/integrations#vs-code) for desktop setup and the
Web adapter's configuration restrictions.

## Moving from gdformat

gdfmt uses its own layout rules and is not a drop-in style replacement for gdtoolkit's `gdformat`. Intentional differences include preserving literal spellings, punctuation and comment attachment. Configuration files are not interchangeable.

When adopting gdfmt, start with `gdfmt --diff .`, review a dedicated formatting change, commit `.gdfmtrc.json`, and use the same formatter revision locally and in CI. Avoid alternating formatters over the same files when their output differs.

## Compatibility tests

The repository keeps original fixtures, per-version engine expectations, a syntax inventory and pinned project trials. The engine harness checks original and formatted scripts against exact Godot binaries using `--check-only`; it does not execute scripts. Analyzer failures are classified separately from syntax failures.

Reproduction commands and engine pins are in [contributing](https://github.com/ntnyq/gdfmt/blob/main/CONTRIBUTING.md#engine-comparisons). The machine-readable records remain under [`tests/compatibility`](https://github.com/ntnyq/gdfmt/tree/main/tests/compatibility) and [`tests/project-trials`](https://github.com/ntnyq/gdfmt/tree/main/tests/project-trials).

The [pre-0.1.0 full project trial](../research/0.1.0-project-trials-review.md)
rechecked 606 pinned scripts under the 4.7 profile. All 2,420 accepted layouts
passed, all 606 review fingerprints were unchanged and no unexplained differences
remained. This working-tree evidence does not certify project imports/runtime
behavior or replace final-candidate acceptance.
