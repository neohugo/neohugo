# nh-parser — porting notes

neohugo parser/* : metadecoders (YAML via go-yaml, TOML via toml crate + go-toml/v2 type mapping, JSON via go-json), pageparser lexer.

## Go file → Rust module

| Rust module | Go source(s) | Owner | Note |
|---|---|---|---|
| `metadecoders::decoder` | `parser/metadecoders/decoder.go` | T03 parser-langs |  |
| `metadecoders::format` | `parser/metadecoders/format.go` | T03 parser-langs |  |
| `metadecoders::toml` | — | T03 parser-langs | NEW: pelletier/go-toml/v2@v2.2.4 Unmarshal-into-any semantics on top of the toml crate |
| `pageparser::item` | `parser/pageparser/item.go`, `parser/pageparser/itemtype_string.go` | T03 parser-langs |  |
| `pageparser::pagelexer` | `parser/pageparser/pagelexer.go` | T03 parser-langs |  |
| `pageparser::pagelexer_intro` | `parser/pageparser/pagelexer_intro.go` | T03 parser-langs |  |
| `pageparser::pagelexer_shortcode` | `parser/pageparser/pagelexer_shortcode.go` | T03 parser-langs |  |
| `pageparser::pageparser` | `parser/pageparser/pageparser.go` | T03 parser-langs |  |
| `frontmatter` | `parser/frontmatter.go`, `parser/lowercase_camel_json.go` | T03 parser-langs |  |

## Dependencies

- nh-*: nh-common
- Wave A (to add when available): go-yaml, go-json, go-strconv, go-time, go-unicode
- crates.io (justify each): toml (parse only; values mapped to go-toml/v2 Go types)

## Deliberate deviations

_Wave B: list every deviation from the Go code here (README rule 1)._

## Known gaps

_Wave B: list unported / stubbed functionality here._
