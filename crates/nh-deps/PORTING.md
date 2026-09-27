# nh-deps — porting notes

neohugo deps/deps.go: per-site dependency container (the Rust context struct).

## Go file → Rust module

| Rust module | Go source(s) | Owner | Note |
|---|---|---|---|
| `deps` | `deps/deps.go` | T20 hugolib-capture |  |

## Dependencies

- nh-*: nh-common, nh-config, nh-media, nh-hugofs, nh-helpers, nh-resource, nh-page, nh-allconfig, nh-tpl, nh-tplimpl, nh-resources
- Wave A (to add when available): none
- crates.io (justify each): none

## Deliberate deviations

_Wave B: list every deviation from the Go code here (README rule 1)._

## Known gaps

_Wave B: list unported / stubbed functionality here._
