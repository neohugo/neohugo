# nh-publisher — porting notes

neohugo publisher/* (DestinationPublisher, transformer chain order, htmlElementsCollector) + x/net/html subset.

## Go file → Rust module

| Rust module | Go source(s) | Owner | Note |
|---|---|---|---|
| `publisher` | `publisher/publisher.go` | T07 transform-publisher |  |
| `html_elements_collector` | `publisher/htmlElementsCollector.go` | T07 transform-publisher |  |
| `xnethtml` | — | T07 transform-publisher | NEW: golang.org/x/net/html tokenizer + tree builder subset used by parseHTMLElement (incl. in-body quirks for th/caption/col/colgroup/frame/image) |

## Dependencies

- nh-*: nh-common, nh-config, nh-media, nh-hugofs, nh-helpers, nh-transform
- Wave A (to add when available): go-unicode, go-json
- crates.io (justify each): none

## Deliberate deviations

_Wave B: list every deviation from the Go code here (README rule 1)._

## Known gaps

_Wave B: list unported / stubbed functionality here._
