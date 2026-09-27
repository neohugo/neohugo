# nh-resource-transformers — porting notes

neohugo resources/resource_factories/{create,bundler} and resources/resource_transformers/{integrity,minifier,templates,js,cssjs,tocss}.

## Go file → Rust module

| Rust module | Go source(s) | Owner | Note |
|---|---|---|---|
| `resource_factories::create::create` | `resources/resource_factories/create/create.go` | T15 resource-factories |  |
| `resource_factories::create::remote` | `resources/resource_factories/create/remote.go` | T15 resource-factories |  |
| `resource_factories::bundler` | `resources/resource_factories/bundler/bundler.go` | T15 resource-factories |  |
| `resource_transformers::integrity` | `resources/resource_transformers/integrity/integrity.go` | T15 resource-factories |  |
| `resource_transformers::minifier` | `resources/resource_transformers/minifier/minify.go` | T15 resource-factories |  |
| `resource_transformers::templates` | `resources/resource_transformers/templates/execute_as_template.go` | T15 resource-factories |  |
| `resource_transformers::js::build` | `resources/resource_transformers/js/build.go` | T16 js-css-pipeline |  |
| `resource_transformers::js::transform` | `resources/resource_transformers/js/transform.go` | T16 js-css-pipeline |  |
| `resource_transformers::cssjs::postcss` | `resources/resource_transformers/cssjs/postcss.go` | T16 js-css-pipeline |  |
| `resource_transformers::cssjs::inline_imports` | `resources/resource_transformers/cssjs/inline_imports.go` | T16 js-css-pipeline | optional (inlineImports=false) |
| `resource_transformers::cssjs::tailwindcss` | `resources/resource_transformers/cssjs/tailwindcss.go` | T16 js-css-pipeline | STUB |
| `resource_transformers::babel` | `resources/resource_transformers/babel/babel.go` | T16 js-css-pipeline | STUB |
| `resource_transformers::tocss::scss::client` | `resources/resource_transformers/tocss/scss/client.go` | T16 js-css-pipeline |  |
| `resource_transformers::tocss::scss::client_extended` | `resources/resource_transformers/tocss/scss/client_extended.go` | T16 js-css-pipeline |  |
| `resource_transformers::tocss::scss::tocss` | `resources/resource_transformers/tocss/scss/tocss.go` | T16 js-css-pipeline |  |
| `resource_transformers::tocss::sass::helpers` | `resources/resource_transformers/tocss/sass/helpers.go` | T16 js-css-pipeline |  |

## Dependencies

- nh-*: nh-common, nh-config, nh-media, nh-hugofs, nh-helpers, nh-resource, nh-resources, nh-transform, nh-tplimpl, nh-tpl, nh-esbuild
- Wave A (to add when available): libsass-sys, go-hashstructure
- crates.io (justify each): sha2, md-5, base64, hex, regex (tocss import regexes)

## Deliberate deviations

_Wave B: list every deviation from the Go code here (README rule 1)._

## Known gaps

_Wave B: list unported / stubbed functionality here._
