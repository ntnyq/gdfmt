# Configuration

Use `.gdfmtrc.json` or `.gdfmtrc.jsonc` to share formatting options and select a language profile. All listed options affect [script formatting](/reference/formatting), including suite/continuation indentation, layout line endings, retained blank lines and the final newline. Literal contents and protected formatter-off/on regions retain their original bytes.

```json
{
  "$schema": "./gdfmtrc.schema.json",
  "schemaVersion": 1,
  "root": true,
  "language": { "godotVersion": "4.7" },
  "format": {
    "lineWidth": 100,
    "indentStyle": "tab",
    "indentWidth": 4,
    "lineEnding": "lf",
    "maxBlankLines": 2,
    "finalNewline": true
  }
}
```

Save the schema beside your configuration with `gdfmt schema > gdfmtrc.schema.json` to use the `$schema` path above. `$schema` is editor metadata; the runtime does not fetch it over the network.

Both filenames accept JSONC: `//` line comments, `/* ... */` block comments, and trailing commas in objects and arrays. Explicit `--config` files and files loaded through `extends` use the same parser, regardless of extension. Strings retain comment-like text such as URLs and glob patterns. JSON5 syntax such as single-quoted strings and unquoted keys is not supported.

For example, save this as `.gdfmtrc.jsonc`:

```jsonc
{
  // Shared project settings
  "root": true,
  "extends": ["./base.jsonc"],
  "format": {
    /* Keep a wider layout */
    "lineWidth": 120,
  },
}
```

`gdfmt init` creates `.gdfmtrc.json` and refuses to proceed if either configuration filename already exists.

## Options

| Field                              | Default  | Accepted values                                 |
| ---------------------------------- | -------- | ----------------------------------------------- |
| schemaVersion                      | 1        | Only 1                                          |
| root                               | false    | Boolean; stops ancestor configuration discovery |
| language.godotVersion              | 4.7      | 4.3 / 4.4 / 4.5 / 4.6 / 4.7                     |
| format.lineWidth                   | 100      | 40–320                                          |
| format.indentStyle                 | tab      | tab / space                                     |
| format.indentWidth                 | 4        | 1–8                                             |
| format.lineEnding                  | lf       | lf / crlf / preserve                            |
| format.maxBlankLines               | 2        | 0–5                                             |
| format.finalNewline                | true     | Boolean                                         |
| format.arrayLayout                 | auto     | auto / multiline / preserve                     |
| format.dictionaryLayout            | auto     | auto / multiline / preserve                     |
| format.enumLayout                  | auto     | auto / multiline / preserve                     |
| format.dictionarySpacing           | compact  | compact / spaced                                |
| format.trailingCommaLayout         | auto     | auto / multiline                                |
| format.blankLinesAroundDefinitions | preserve | preserve or integer 0–5                         |
| format.lambdaBodyStyle             | auto     | auto / multiline                                |
| format.inlineCommentSpacing        | 1        | Integer 1–4                                     |

## Layout policies

All layout options keep the existing defaults and significant token spellings. They apply through inheritance, overrides and the npm sync/async text APIs; `explain-config` reports their effective values and sources.

- `arrayLayout`, `dictionaryLayout` and `enumLayout` independently control literals/members. `auto` wraps by width; `multiline` expands nonempty lists; `preserve` retains an existing multiline layout but still wraps long single-line input. Empty collections stay compact unless comments require lines. Collection type references such as `Dictionary[String, int]` remain indivisible.
- `dictionarySpacing: "spaced"` adds spaces inside nonempty single-line dictionary braces: `{ "key": 1 }` or `{ name = 1 }`. Empty dictionaries remain `{}`; multiline dictionaries, arrays and calls receive no brace padding.
- `trailingCommaLayout: "multiline"` expands nonempty delimited lists that already end in a comma, including literals, enums, call/annotation arguments, function/signal/lambda parameters and match patterns. It never adds or removes commas. A policy requiring expansion takes precedence over `auto`; block lambdas retain the indentation required by their enclosing statement.
- `blankLinesAroundDefinitions` accepts `"preserve"` or an integer from 0 to 5. A number sets the gap between a function or inner class and adjacent declarations in the same script/class scope. It takes precedence over `maxBlankLines` at those boundaries, including when the count is larger; other gaps remain capped by `maxBlankLines`. No gap is added at scope boundaries or inside function bodies. Leading comments and annotations stay together with their target declaration.
- `lambdaBodyStyle: "multiline"` forces block bodies. `auto` keeps the existing simple, comment-free single-statement inline behavior. Complex/commented bodies always use the required block layout.
- `inlineCommentSpacing` sets 1–4 spaces between code and a trailing `#` comment. Comment text and own-line comment indentation remain governed by their existing attachment rules.

For example:

```json
{
  "format": {
    "arrayLayout": "preserve",
    "dictionarySpacing": "spaced",
    "trailingCommaLayout": "multiline",
    "blankLinesAroundDefinitions": 2,
    "lambdaBodyStyle": "multiline",
    "inlineCommentSpacing": 2
  }
}
```

With this configuration, `var points = [1, 2,] # points` becomes:

```gdscript
var points = [
	1,
	2,
]  # points
```

Literal contents and formatter-off/on regions retain their original bytes regardless of these settings. See [script formatting](/reference/formatting) for validation and protected-region fallback behavior.

## Discovery, inheritance and overrides

For each input, gdfmt discovers `.gdfmtrc.json` or `.gdfmtrc.jsonc` from its directory up to `root: true` or the filesystem root. In each directory, `.gdfmtrc.json` takes precedence: when both exist, only `.gdfmtrc.json` is read, and errors in that file do not fall back to `.gdfmtrc.jsonc`. Missing virtual stdin paths use their existing ancestor while retaining the complete filename for matching. An explicit `--config` resolves relative to the working directory and replaces directory JSON/JSONC discovery; its own `extends` still applies. EditorConfig and local ignore files remain active.

```json
{
  "root": true,
  "extends": ["./base.json", "./team.json"],
  "ignore": ["generated/", "vendor/"],
  "overrides": [
    {
      "files": ["src/**/*.gd"],
      "exclude": ["src/generated/**"],
      "format": { "indentStyle": "space", "indentWidth": 2 }
    }
  ]
}
```

The precedence, from lowest to highest, is defaults, EditorConfig, all JSON/JSONC base fields, then all matching overrides. JSON/JSONC layers run ancestor first and nearest last. Each layer expands `extends` depth first, left to right, before applying itself. Shared parents are replayed in that order; cycles use canonical paths and are errors. `root` controls ancestor discovery only, and never blocks an explicit `extends` edge. Inheritance is limited to 32 active files, 128 applied layers and 16 MiB of configuration text; each individual configuration/ignore file has a 1 MiB limit. Only local files are loaded, with no network or package lookup.

Objects merge field by field. Missing values inherit, including through empty objects; explicit `false` and `0` survive. Ordinary arrays (`extends`, `ignore`) replace inherited arrays, including `[]`. `overrides` is the exception: arrays append, so `[]` adds no entries and does not clear inherited entries. Every base file and override is validated even when later settings would replace it.

`extends` paths, override globs and JSON/JSONC ignore patterns use the directory of the file that declares them, including inherited files. Overrides match the complete relative path, are case sensitive, and use `/` on every platform: `*.gd` matches direct children, `**/*.gd` also matches descendants. `*`, `?`, character classes and brace alternatives are supported. Absolute paths, `..` segments, backslashes, empty globs and malformed globs are errors. Each override requires `files`; `files: []` matches nothing, and any matching `exclude` vetoes that entry. Matching entries apply in order, with later fields winning. Overrides may contain only `files`, `exclude`, `language` and `format`.

Unknown fields, `null`, incorrect types, out-of-range values, unknown enum variants, and future schema versions are rejected. Errors identify the configuration file and field path.

```sh
gdfmt explain-config path/to/script.gd
```

`explain-config` accepts existing and virtual paths. It reports JSON/JSONC `files` in application order, `editorconfigFiles`, merged values, `ignored`, and the effective `sources` of each field. Defaults use `default`, JSON/JSONC overrides use `file#overrides[index]`, and EditorConfig uses `file:line (property)`. An explicit JSON/JSONC ignore list adds its declaring source as `ignore`.

## EditorConfig

EditorConfig discovery is independent of JSON/JSONC `root` and stops at its own `root = true`. Ancestor files and matching sections apply in order, including standard EditorConfig globs, case-insensitive property names/values and `unset`. Supported mappings are:

| EditorConfig property      | Formatter field       | Supported values                          |
| -------------------------- | --------------------- | ----------------------------------------- |
| `indent_style`             | `format.indentStyle`  | `tab`, `space`                            |
| `indent_size`, `tab_width` | `format.indentWidth`  | 1–8; `indent_size = tab` uses `tab_width` |
| `end_of_line`              | `format.lineEnding`   | `lf`, `crlf`                              |
| `insert_final_newline`     | `format.finalNewline` | `true`, `false`                           |
| `max_line_length`          | `format.lineWidth`    | 40–320                                    |

For tabs, `tab_width` takes precedence over numeric `indent_size`; for spaces, numeric `indent_size` takes precedence. The other supported numeric property is the fallback when the preferred value is absent or unsupported. `unset` removes an inherited property. Unsupported properties/values (including `charset`, `trim_trailing_whitespace`, `cr` and `max_line_length = off`) do not change formatter options or appear as effective sources. Malformed files and read failures are errors. gdfmt still requires UTF-8 and preserves literal/protected-region bytes; EditorConfig never authorizes semantic or encoding transformations.

## Ignore rules and input selection

Directory walks and explicit `.gd` inputs honor `.gitignore`, `.ignore`, `.gdfmtignore` and repository `.git/info/exclude`. Gitignore-style patterns include `!` negation, `/` anchoring and directory suffixes. At the same directory the priority is `.gdfmtignore` > `.ignore` > `.gitignore` > `.git/info/exclude`; nearer files take precedence over ancestors. `.gitignore` is also honored outside Git repositories. Machine-wide Git excludes are disabled. Git worktree marker files are tolerated; external Git common-directory excludes are not resolved. An excluded directory is pruned, so a negation inside it cannot restore descendants.

`.git`, `.godot`, `node_modules` and `target` directories are always excluded. Symlink entries are skipped while traversing, and explicit symlinks are rejected. Results are sorted and deduplicated. After local ignore filtering, every candidate's JSON/JSONC configuration is validated and its final `ignore` array is applied using gitignore syntax, independently of the local-ignore lists. A child JSON/JSONC `ignore: []` can replace an inherited configuration list; it cannot cancel local ignore files or built-in exclusions. Configuration ignores filter candidates after configuration resolution rather than pruning configuration discovery.

Explicit ignored files are skipped. Empty discovery, including when every input is ignored, exits 2. A `--stdin-filepath` that is ignored also exits 2 with no formatted output; it never writes a virtual file. Plain `format -` uses virtual `stdin.gd` in the working directory for these decisions. `explain-config` can still inspect an ignored path. File/check/diff/stdin use the same resolved layout and validated formatter core.

## Writes and current limits

All selected configurations and source reads are preflighted before the first file replacement. A source syntax/layout failure prevents writing that source; other valid files can still be formatted. Exit codes are 0 for success, 1 for syntax/format differences, and 2 for arguments/configuration/I/O/internal failures; a multi-file run chooses the highest code. `check` and `--diff` never write. Human errors go to stderr; JSON reports use stdout and keep stderr empty when reporting succeeds.

Writes use a temporary file in the source directory, preserve permissions and recheck the original file before atomic replacement. Read-only files and symlinks are refused, as are Unix hardlinks. Replacement is atomic per file, with best-effort concurrent-change detection; it is not a multi-file transaction. Forced termination may leave a temporary file behind.

GDScript quote conversion and comma insertion/removal are not implemented; `trailingCommaLayout` only changes layout. `lint`, `rules` and rule-plugin configuration remain outside formatter scope and produce unknown-field errors.

Language versions share an enum with the parser's `ParseOptions.language_version`. The CLI's `lex` performs lexical checks without version gates; `check` and formatting use the resolved grammar profile. See [language versions](/reference/language-versions) for the distinction between profiles and full engine compatibility.
