# nh-tpl — porting notes

neohugo tpl/template.go: template-execution context (context.Context replacement), CurrentTemplateInfo, StripHTML.

## Go file → Rust module

| Rust module | Go source(s) | Owner | Note |
|---|---|---|---|
| `template` | `tpl/template.go`, `common/hcontext/context.go` | T13 tplimpl |  |

## Dependencies

- nh-*: nh-common, nh-langs
- Wave A (to add when available): gotemplate (htmltemplate::strip_tags), go-unicode
- crates.io (justify each): none

## Deliberate deviations

_Wave B: list every deviation from the Go code here (README rule 1)._

## Known gaps

_Wave B: list unported / stubbed functionality here._
