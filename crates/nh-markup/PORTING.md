# nh-markup — porting notes

neohugo markup/**: converter + hooks API, goldmark glue (render hooks, autoid, attributes, tables, blockquotes, hugocontext, images, TOC), highlight config (+ stub highlighter), markup_config; asciidoc/pandoc/rst/org stubs.

## Go file → Rust module

| Rust module | Go source(s) | Owner | Note |
|---|---|---|---|
| `markup` | `markup/markup.go` | T06 markup |  |
| `converter::converter` | `markup/converter/converter.go` | T06 markup |  |
| `converter::hooks` | `markup/converter/hooks/hooks.go` | T06 markup |  |
| `markup_config` | `markup/markup_config/config.go` | T06 markup |  |
| `tableofcontents` | `markup/tableofcontents/tableofcontents.go` | T06 markup |  |
| `highlight::config` | `markup/highlight/config.go` | T06 markup |  |
| `highlight::highlight` | `markup/highlight/highlight.go` | T06 markup | STUB highlighter: fenced code rendering returns an explicit unsupported error |
| `internal::attributes` | `markup/internal/attributes/attributes.go` | T06 markup |  |
| `goldmark::convert` | `markup/goldmark/convert.go` | T06 markup |  |
| `goldmark::render_hooks` | `markup/goldmark/render_hooks.go` | T06 markup |  |
| `goldmark::autoid` | `markup/goldmark/autoid.go` | T06 markup |  |
| `goldmark::toc` | `markup/goldmark/toc.go` | T06 markup |  |
| `goldmark::goldmark_config` | `markup/goldmark/goldmark_config/config.go` | T06 markup |  |
| `goldmark::blockquotes` | `markup/goldmark/blockquotes/blockquotes.go` | T06 markup |  |
| `goldmark::codeblocks` | `markup/goldmark/codeblocks/render.go` | T06 markup |  |
| `goldmark::hugocontext` | `markup/goldmark/hugocontext/hugocontext.go` | T06 markup |  |
| `goldmark::images` | `markup/goldmark/images/transform.go` | T06 markup |  |
| `goldmark::tables` | `markup/goldmark/tables/tables.go` | T06 markup |  |
| `goldmark::passthrough` | `markup/goldmark/passthrough/passthrough.go` | T06 markup | STUB (passthrough disabled) |
| `goldmark::internal::extensions::attributes` | `markup/goldmark/internal/extensions/attributes/attributes.go` | T06 markup |  |
| `goldmark::internal::render` | `markup/goldmark/internal/render/context.go` | T06 markup |  |
| `asciidocext` | `markup/asciidocext/convert.go` | T06 markup | STUB |
| `pandoc` | `markup/pandoc/convert.go` | T06 markup | STUB |
| `rst` | `markup/rst/convert.go` | T06 markup | STUB |
| `org` | `markup/org/convert.go` | T06 markup | STUB |
| `blackfriday` | `markup/blackfriday/anchors.go` | T06 markup |  |

## Dependencies

- nh-*: nh-common, nh-config, nh-media
- Wave A (to add when available): goldmark, go-html, go-unicode
- crates.io (justify each): regex (blockquote alert, hugocontext; with Go \s rewritten)

## Deliberate deviations

_Wave B: list every deviation from the Go code here (README rule 1)._

## Known gaps

_Wave B: list unported / stubbed functionality here._
