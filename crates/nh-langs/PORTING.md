# nh-langs — porting notes

neohugo langs/{language,config}.go: Language, Languages, collators (x/text collate CLDR 23), locale translators.

## Go file → Rust module

| Rust module | Go source(s) | Owner | Note |
|---|---|---|---|
| `language` | `langs/language.go` | T03 parser-langs |  |
| `config` | `langs/config.go` | T03 parser-langs |  |

## Dependencies

- nh-*: nh-common
- Wave A (to add when available): xtext-collate, go-time
- crates.io (justify each): none

## Deliberate deviations

_Wave B: list every deviation from the Go code here (README rule 1)._

## Known gaps

_Wave B: list unported / stubbed functionality here._
