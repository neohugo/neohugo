# nh-transform — porting notes

neohugo transform/{chain,urlreplacers} and minifiers/* (tdewolff registry by media type).

## Go file → Rust module

| Rust module | Go source(s) | Owner | Note |
|---|---|---|---|
| `chain` | `transform/chain.go` | T07 transform-publisher |  |
| `urlreplacers::absurl` | `transform/urlreplacers/absurl.go` | T07 transform-publisher |  |
| `urlreplacers::absurlreplacer` | `transform/urlreplacers/absurlreplacer.go` | T07 transform-publisher |  |
| `livereloadinject` | `transform/livereloadinject/livereloadinject.go` | T07 transform-publisher | STUB (server only) |
| `metainject` | `transform/metainject/hugogenerator.go` | T07 transform-publisher | STUB (never active: neohugo inverted the flag) |
| `minifiers::config` | `minifiers/config.go` | T07 transform-publisher |  |
| `minifiers::minifiers` | `minifiers/minifiers.go` | T07 transform-publisher |  |

## Dependencies

- nh-*: nh-common, nh-config, nh-media
- Wave A (to add when available): tdewolff-minify, tdewolff-minify-js, tdewolff-parse
- crates.io (justify each): memchr / aho-corasick (optional, byte search)

## Deliberate deviations

_Wave B: list every deviation from the Go code here (README rule 1)._

## Known gaps

_Wave B: list unported / stubbed functionality here._
