# nh-i18n — porting notes

neohugo langs/i18n + gohugoio/go-i18n/v2 fork (bundle, localizer, message templates, CLDR plural rules).

## Go file → Rust module

| Rust module | Go source(s) | Owner | Note |
|---|---|---|---|
| `i18n` | `langs/i18n/i18n.go` | T17 i18n |  |
| `translation_provider` | `langs/i18n/translationProvider.go` | T17 i18n |  |
| `goi18n::bundle` | — | T17 i18n | PORT gohugoio/go-i18n/v2 i18n/bundle.go |
| `goi18n::localizer` | — | T17 i18n | PORT i18n/localizer.go |
| `goi18n::message` | — | T17 i18n | PORT i18n/message.go + message_template.go |
| `goi18n::parse` | — | T17 i18n | PORT i18n/parse.go |
| `goi18n::template` | — | T17 i18n | PORT internal/template.go (text/template, no funcs, lazy parse) |
| `goi18n::plural` | — | T17 i18n | PORT internal/plural/{form,operands,rule,rules,rule_gen}.go |

## Dependencies

- nh-*: nh-common, nh-config, nh-parser, nh-langs, nh-hugofs, nh-helpers, nh-deps, nh-tpl
- Wave A (to add when available): gotemplate (text/template for message templates), go-fmt
- crates.io (justify each): none

## Deliberate deviations

_Wave B: list every deviation from the Go code here (README rule 1)._

## Known gaps

_Wave B: list unported / stubbed functionality here._
