# CLI reference

```sh
gdfmt [OPTIONS] [PATH]...
gdfmt <COMMAND> [OPTIONS]
```

With no command or paths, gdfmt formats the current directory. Paths are files or directories; recursive discovery selects `.gd` files. The CLI does not expand glob patterns itself.

## Commands

| Command                               | Behavior                                                               |
| ------------------------------------- | ---------------------------------------------------------------------- |
| `gdfmt [PATH]...`                     | Format files in place                                                  |
| `gdfmt format [PATH]...`              | Explicit form of the default command                                   |
| `gdfmt check [PATH]...`               | Check syntax and formatting without writing                            |
| `gdfmt check [PATH]... --syntax-only` | Check parser diagnostics only                                          |
| `gdfmt init`                          | Create `.gdfmtrc.json`; refuse if either configuration filename exists |
| `gdfmt explain-config [PATH]`         | Print resolved settings and their sources as JSON; defaults to `.`     |
| `gdfmt schema`                        | Print the configuration JSON Schema                                    |
| `gdfmt lex [PATH]...`                 | Inspect tokens and lexical diagnostics without writing                 |

`lex` is a debugging tool. Passing lexical checks does not establish valid script syntax; use `check --syntax-only` for that.

## Options

| Option                     | Behavior                                                                      |
| -------------------------- | ----------------------------------------------------------------------------- |
| `--check`                  | Check syntax and formatting without writing; alias for `check`                |
| `--diff`                   | Print formatting differences without writing                                  |
| `--config <FILE>`          | Use an explicit local JSON/JSONC configuration instead of directory discovery |
| `--stdin-filepath <PATH>`  | Read stdin and resolve settings using a virtual `.gd` path                    |
| `--reporter <human\|json>` | Select output format; defaults to `human`                                     |
| `--lsp`                    | Run the formatting language server over stdio                                 |
| `-h`, `--help`             | Print help; available on each command                                         |
| `-V`, `--version`          | Print the version                                                             |

`--check` and `--diff` are mutually exclusive. `--syntax-only` belongs to the `check` command and cannot be combined with `--diff`. Formatting modes and stdin paths apply to formatting/check commands.

`--config` applies to formatting, checks, `lex`, `explain-config` and LSP; it is
rejected for `init` and `schema`. `--lsp` cannot be combined with a subcommand,
paths, formatting modes, stdin paths or `--reporter=json`: its stdout uses framed
JSON-RPC. Invalid option combinations fail before configuration reads or writes.

Layout options belong in [configuration](/guide/configuration), rather than CLI flags. An explicit `--config` still allows that file's `extends`, EditorConfig and local ignore rules.

## Stdin

```sh
gdfmt format - < script.gd
gdfmt --stdin-filepath src/player.gd < script.gd
```

Plain `format -` uses virtual `stdin.gd` in the working directory. `--stdin-filepath` uses the complete virtual filename for configuration overrides and ignores; the file need not exist. It never writes that path. Successful formatting prints source to stdout. Invalid input produces no formatted source. Stdin cannot be mixed with file paths.

## Reporters

```sh
gdfmt check src --reporter json
gdfmt --diff src --reporter json
```

Human errors go to stderr. JSON reports use stdout and keep stderr empty when reporting succeeds. Reports include `schemaVersion: 1`, `spanEncoding: "utf8Bytes"`, a command, status, files and diagnostics. Formatting file entries include `changed`; syntax-only entries omit it. Stdin output and diffs appear in `output` and `diff` fields where applicable.

Diagnostic spans are half-open UTF-8 byte ranges `[start, end)`, not character or UTF-16 offsets. Virtual lexer tokens have zero-width spans. `schema` and `explain-config` emit JSON directly.

Help and version requests exit 0 and print plain text even with `--reporter=json`.
Other argument errors use JSON when that reporter was selected before `--`; tokens
after `--` are literal paths. `CLI_ARGUMENT` identifies argument parsing/option
validation, `CONFIG_INVALID` identifies project configuration failures, and
`CLI_IO` identifies path/file/stream failures. Host diagnostics contain `code`,
`category`, `severity` and `message`; source diagnostics additionally contain
`sourceId` and `span`. Message wording is human-readable, not a parsing interface.
`FMT001` denotes an internal formatter failure; malformed protection directives
use `FMT002` and exit 1 even though their diagnostic category is `configuration`.

## Exit codes

| Code | Meaning                                                                              |
| ---- | ------------------------------------------------------------------------------------ |
| `0`  | Success; checks found no errors or formatting differences                            |
| `1`  | Source errors, invalid formatter directives, or read-only formatting differences     |
| `2`  | Argument, configuration, I/O or internal validation failure; also no matching inputs |

A multi-file run uses the highest exit code. Configurations and input reads are preflighted before writes. A source error leaves that file unchanged; other valid files may still be formatted. Replacement is atomic per file, not across the whole batch. See [write behavior](/guide/configuration#writes-and-current-limits).
