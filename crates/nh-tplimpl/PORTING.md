# nh-tplimpl — porting notes

neohugo tpl/tplimpl/** (template store, layout lookup, baseof, AST transforms, exec helper) + embedded templates; engine.rs is the single adaptation point to the Wave A gotemplate crate.

## Go file → Rust module

| Rust module | Go source(s) | Owner | Note |
|---|---|---|---|
| `engine` | — | T13 tplimpl | NEW: facade over the gotemplate crate (text/html template namespaces, parse trees, exec, escaper) |
| `templatestore` | `tpl/tplimpl/templatestore.go` | T13 tplimpl |  |
| `templates` | `tpl/tplimpl/templates.go` | T13 tplimpl |  |
| `templatetransform` | `tpl/tplimpl/templatetransform.go` | T13 tplimpl |  |
| `templatedescriptor` | `tpl/tplimpl/templatedescriptor.go` | T13 tplimpl |  |
| `template_funcs` | `tpl/tplimpl/template_funcs.go` | T13 tplimpl |  |
| `legacy` | `tpl/tplimpl/legacy.go` | T13 tplimpl |  |
| `template_info` | `tpl/tplimpl/template_info.go` | T13 tplimpl |  |
| `category` | `tpl/tplimpl/category_string.go`, `tpl/tplimpl/subcategory_string.go` | T13 tplimpl |  |
| `embedded` | — | T13 tplimpl | NEW: include_bytes! of tpl/tplimpl/embedded/templates/** (read from the Go tree) |

## Dependencies

- nh-*: nh-common, nh-config, nh-media, nh-hugofs, nh-markup, nh-helpers, nh-resource, nh-page, nh-doctree, nh-tpl, nh-langs
- Wave A (to add when available): gotemplate
- crates.io (justify each): none

## Deliberate deviations

_Wave B: list every deviation from the Go code here (README rule 1)._

## Known gaps

_Wave B: list unported / stubbed functionality here._
