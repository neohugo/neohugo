# nh-tpl — porting notes

neohugo tpl/template.go: template-execution context (context.Context replacement), CurrentTemplateInfo, StripHTML.

## Go file → Rust module

| Rust module | Go source(s) | Owner | Note |
|---|---|---|---|
| `template` | `tpl/template.go`, `common/hcontext/context.go` | T13 tplimpl |  |

## Dependencies

- nh-*: nh-common, nh-langs, nh-config (registers the `neohugo.GetMarkupScope` reader)
- Wave A: gotemplate (`html::strip_tags`), go-unicode, go-value
- crates.io: none (dev only: serde_json, flate2 for the gz JSON fixture)

## Deliberate deviations

- `context.Context` values are fields of `TplContext` (`current_template`, markup scope, …) instead of keyed context values. `with_current_template_info(name, filename, base)` replaces Go's construction of `CurrentTemplateInfo` from a `*TemplInfo` (whose `Base()` becomes the `base: Option<CurrentTemplateBase>` field), so nh-tpl does not depend on nh-tplimpl.
- `register_markup_scope_getter()` installs the reader in `nh_config::neohugo::neohugo::set_markup_scope_getter` (Go reads the context key directly). `nh_tplimpl::TemplateStore::new` calls it.

## Known gaps

None.

## Verification

`tests/striphtml.rs` checks `strip_html` against `tools/go-oracle/nh-tpl/striphtml` (4462 cases: edge cases, every 1- and 2-byte string over HTML-significant bytes, 4000 seeded random HTML-ish strings, slices of docs/content; invalid UTF-8 included), plus the markup scope getter and current-template levels. Regenerate: `GOTOOLCHAIN=go1.27.1 go run ./tools/go-oracle/nh-tpl/striphtml`.
