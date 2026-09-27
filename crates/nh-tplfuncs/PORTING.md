# nh-tplfuncs — porting notes

neohugo tpl/<namespace>/** template functions (all namespaces; seeksnack's set fully, the rest as stubs), registry, tplimplinit.

## Go file → Rust module

| Rust module | Go source(s) | Owner | Note |
|---|---|---|---|
| `internal::registry` | `tpl/internal/templatefuncsRegistry.go` | T19 tplfuncs-host |  |
| `internal::resourcehelpers` | `tpl/internal/resourcehelpers/helpers.go` | T15 resource-factories |  |
| `tplimplinit` | `tpl/tplimplinit/tplimplinit.go` | T19 tplfuncs-host |  |
| `cast::init` | `tpl/cast/init.go` | T18 tplfuncs-data |  |
| `cast::cast` | `tpl/cast/cast.go` | T18 tplfuncs-data |  |
| `cast::docshelper` | `tpl/cast/docshelper.go` | T18 tplfuncs-data | not needed (docs) |
| `collections::init` | `tpl/collections/init.go` | T18 tplfuncs-data |  |
| `collections::append` | `tpl/collections/append.go` | T18 tplfuncs-data |  |
| `collections::apply` | `tpl/collections/apply.go` | T18 tplfuncs-data |  |
| `collections::collections` | `tpl/collections/collections.go` | T18 tplfuncs-data |  |
| `collections::complement` | `tpl/collections/complement.go` | T18 tplfuncs-data |  |
| `collections::index` | `tpl/collections/index.go` | T18 tplfuncs-data |  |
| `collections::merge` | `tpl/collections/merge.go` | T18 tplfuncs-data |  |
| `collections::querify` | `tpl/collections/querify.go` | T18 tplfuncs-data |  |
| `collections::reflect_helpers` | `tpl/collections/reflect_helpers.go` | T18 tplfuncs-data |  |
| `collections::sort` | `tpl/collections/sort.go` | T18 tplfuncs-data |  |
| `collections::symdiff` | `tpl/collections/symdiff.go` | T18 tplfuncs-data |  |
| `collections::where_` | `tpl/collections/where.go` | T18 tplfuncs-data |  |
| `compare::init` | `tpl/compare/init.go` | T18 tplfuncs-data |  |
| `compare::compare` | `tpl/compare/compare.go` | T18 tplfuncs-data |  |
| `crypto::init` | `tpl/crypto/init.go` | T18 tplfuncs-data |  |
| `crypto::crypto` | `tpl/crypto/crypto.go` | T18 tplfuncs-data |  |
| `encoding::init` | `tpl/encoding/init.go` | T18 tplfuncs-data |  |
| `encoding::encoding` | `tpl/encoding/encoding.go` | T18 tplfuncs-data |  |
| `fmt::init` | `tpl/fmt/init.go` | T18 tplfuncs-data |  |
| `fmt::fmt` | `tpl/fmt/fmt.go` | T18 tplfuncs-data |  |
| `hash::init` | `tpl/hash/init.go` | T18 tplfuncs-data |  |
| `hash::hash` | `tpl/hash/hash.go` | T18 tplfuncs-data |  |
| `math::init` | `tpl/math/init.go` | T18 tplfuncs-data |  |
| `math::math` | `tpl/math/math.go` | T18 tplfuncs-data |  |
| `math::round` | `tpl/math/round.go` | T18 tplfuncs-data |  |
| `reflect::init` | `tpl/reflect/init.go` | T18 tplfuncs-data |  |
| `reflect::reflect` | `tpl/reflect/reflect.go` | T18 tplfuncs-data |  |
| `safe::init` | `tpl/safe/init.go` | T18 tplfuncs-data |  |
| `safe::safe` | `tpl/safe/safe.go` | T18 tplfuncs-data |  |
| `css::init` | `tpl/css/init.go` | T19 tplfuncs-host |  |
| `css::css` | `tpl/css/css.go` | T19 tplfuncs-host |  |
| `data::init` | `tpl/data/init.go` | T19 tplfuncs-host |  |
| `data::data` | `tpl/data/data.go` | T19 tplfuncs-host |  |
| `data::resources` | `tpl/data/resources.go` | T19 tplfuncs-host |  |
| `debug::init` | `tpl/debug/init.go` | T19 tplfuncs-host |  |
| `debug::debug` | `tpl/debug/debug.go` | T19 tplfuncs-host |  |
| `diagrams::init` | `tpl/diagrams/init.go` | T19 tplfuncs-host |  |
| `diagrams::diagrams` | `tpl/diagrams/diagrams.go` | T19 tplfuncs-host |  |
| `diagrams::goat` | `tpl/diagrams/goat.go` | T19 tplfuncs-host |  |
| `hugo::init` | `tpl/hugo/init.go` | T19 tplfuncs-host |  |
| `images::init` | `tpl/images/init.go` | T19 tplfuncs-host |  |
| `images::images` | `tpl/images/images.go` | T19 tplfuncs-host |  |
| `inflect::init` | `tpl/inflect/init.go` | T19 tplfuncs-host |  |
| `inflect::inflect` | `tpl/inflect/inflect.go` | T19 tplfuncs-host |  |
| `js::init` | `tpl/js/init.go` | T19 tplfuncs-host |  |
| `js::js` | `tpl/js/js.go` | T19 tplfuncs-host |  |
| `lang::init` | `tpl/lang/init.go` | T19 tplfuncs-host |  |
| `lang::lang` | `tpl/lang/lang.go` | T19 tplfuncs-host |  |
| `openapi3::init` | `tpl/openapi/openapi3/init.go` | T19 tplfuncs-host |  |
| `openapi3::openapi3` | `tpl/openapi/openapi3/openapi3.go` | T19 tplfuncs-host |  |
| `os::init` | `tpl/os/init.go` | T19 tplfuncs-host |  |
| `os::os` | `tpl/os/os.go` | T19 tplfuncs-host |  |
| `page::init` | `tpl/page/init.go` | T19 tplfuncs-host |  |
| `partials::init` | `tpl/partials/init.go` | T19 tplfuncs-host |  |
| `partials::partials` | `tpl/partials/partials.go` | T19 tplfuncs-host |  |
| `path::init` | `tpl/path/init.go` | T19 tplfuncs-host |  |
| `path::path` | `tpl/path/path.go` | T19 tplfuncs-host |  |
| `resources::init` | `tpl/resources/init.go` | T15 resource-factories |  |
| `resources::resources` | `tpl/resources/resources.go` | T15 resource-factories |  |
| `site::init` | `tpl/site/init.go` | T19 tplfuncs-host |  |
| `strings::init` | `tpl/strings/init.go` | T19 tplfuncs-host |  |
| `strings::strings` | `tpl/strings/strings.go` | T19 tplfuncs-host |  |
| `strings::regexp` | `tpl/strings/regexp.go` | T19 tplfuncs-host |  |
| `strings::truncate` | `tpl/strings/truncate.go` | T19 tplfuncs-host |  |
| `templates::init` | `tpl/templates/init.go` | T19 tplfuncs-host |  |
| `templates::templates` | `tpl/templates/templates.go` | T19 tplfuncs-host |  |
| `time::init` | `tpl/time/init.go` | T19 tplfuncs-host |  |
| `time::time` | `tpl/time/time.go` | T19 tplfuncs-host |  |
| `transform::init` | `tpl/transform/init.go` | T19 tplfuncs-host |  |
| `transform::transform` | `tpl/transform/transform.go` | T19 tplfuncs-host |  |
| `transform::unmarshal` | `tpl/transform/unmarshal.go` | T19 tplfuncs-host |  |
| `transform::remarshal` | `tpl/transform/remarshal.go` | T19 tplfuncs-host |  |
| `urls::init` | `tpl/urls/init.go` | T19 tplfuncs-host |  |
| `urls::urls` | `tpl/urls/urls.go` | T19 tplfuncs-host |  |

## Dependencies

- nh-*: nh-common, nh-config, nh-media, nh-langs, nh-hugofs, nh-helpers, nh-markup, nh-resource, nh-images, nh-page, nh-allconfig, nh-tpl, nh-tplimpl, nh-resources, nh-resource-transformers, nh-parser, nh-deps
- Wave A (to add when available): go-fmt, go-json, go-url, go-time, go-html, go-strconv, go-sort, xtext-collate (via nh-langs)
- crates.io (justify each): md-5, sha1, sha2, base64, hex, crc32fast, regex (findRE/replaceRE; not exercised)

## Deliberate deviations

_Wave B: list every deviation from the Go code here (README rule 1)._

## Known gaps

_Wave B: list unported / stubbed functionality here._
