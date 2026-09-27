# nh-commands — porting notes

neohugo commands/* + main.go (bin `neohugo`): flags -> config, build, version/env/config commands, static copy (spf13/fsync).

## Go file → Rust module

| Rust module | Go source(s) | Owner | Note |
|---|---|---|---|
| `commandeer` | `commands/commandeer.go` | T25 commands-cli |  |
| `commands` | `commands/commands.go` | T25 commands-cli |  |
| `helpers` | `commands/helpers.go` | T25 commands-cli |  |
| `hugobuilder` | `commands/hugobuilder.go` | T25 commands-cli |  |
| `config` | `commands/config.go` | T25 commands-cli |  |
| `env` | `commands/env.go` | T25 commands-cli |  |
| `fsync` | — | T25 commands-cli | NEW: spf13/fsync@v0.10.1 Syncer (copy-if-different, chmod, mtimes) |

## Dependencies

- nh-*: nh-common, nh-config, nh-allconfig, nh-hugofs, nh-helpers, nh-hugolib, nh-deps
- Wave A (to add when available): none
- crates.io (justify each): clap (CLI parsing; no output-byte impact)

## Deliberate deviations

_Wave B: list every deviation from the Go code here (README rule 1)._

## Known gaps

_Wave B: list unported / stubbed functionality here._
