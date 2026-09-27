# nh-allconfig — porting notes

neohugo config/allconfig (load, decode all sections, compile, per-language configs), hugolib/segments, deploy/deployconfig stub.

## Go file → Rust module

| Rust module | Go source(s) | Owner | Note |
|---|---|---|---|
| `allconfig` | `config/allconfig/allconfig.go` | T09 allconfig-modules |  |
| `alldecoders` | `config/allconfig/alldecoders.go` | T09 allconfig-modules |  |
| `configlanguage` | `config/allconfig/configlanguage.go` | T09 allconfig-modules |  |
| `load` | `config/allconfig/load.go` | T09 allconfig-modules |  |
| `segments` | `hugolib/segments/segments.go` | T09 allconfig-modules |  |
| `deployconfig` | `deploy/deployconfig/deployConfig.go` | T09 allconfig-modules | STUB (decode only) |

## Dependencies

- nh-*: nh-common, nh-parser, nh-langs, nh-config, nh-media, nh-hugofs, nh-markup, nh-transform, nh-helpers, nh-images, nh-page
- Wave A (to add when available): go-time
- crates.io (justify each): none

## Deliberate deviations

_Wave B: list every deviation from the Go code here (README rule 1)._

## Known gaps

_Wave B: list unported / stubbed functionality here._
