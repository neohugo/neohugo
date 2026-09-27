# nh-media — porting notes

neohugo media/* and output/* (media types, output formats).

## Go file → Rust module

| Rust module | Go source(s) | Owner | Note |
|---|---|---|---|
| `media::builtin` | `media/builtin.go` | T04 config-base-media |  |
| `media::config` | `media/config.go` | T04 config-base-media |  |
| `media::media_type` | `media/mediaType.go` | T04 config-base-media |  |
| `output::config` | `output/config.go` | T04 config-base-media |  |
| `output::output_format` | `output/outputFormat.go` | T04 config-base-media |  |

## Dependencies

- nh-*: nh-common, nh-config
- Wave A (to add when available): none
- crates.io (justify each): none

## Deliberate deviations

_Wave B: list every deviation from the Go code here (README rule 1)._

## Known gaps

_Wave B: list unported / stubbed functionality here._
