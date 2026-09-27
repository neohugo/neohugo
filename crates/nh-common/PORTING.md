# nh-common — porting notes

neohugo common/* + compare + resources/kinds + hugofs/files + hugofs/glob + identity(stub) + cache/dynacache, plus ports of spf13/cast, gobuffalo/flect, jdkato/prose/transform, gohugoio/locales (en, th).

## Go file → Rust module

| Rust module | Go source(s) | Owner | Note |
|---|---|---|---|
| `object` | — | T01 common-values | NEW: go_methods! macro, argument helpers, NamedTypeRegistry (replaces reflect method lookup) |
| `herrors` | `common/herrors/errors.go`, `common/herrors/file_error.go`, `common/herrors/error_locator.go`, `common/herrors/line_number_extractors.go` | T01 common-values |  |
| `constants` | `common/constants/constants.go` | T01 common-values |  |
| `collections::append` | `common/collections/append.go` | T01 common-values |  |
| `collections::collections` | `common/collections/collections.go` | T01 common-values |  |
| `collections::order` | `common/collections/order.go` | T01 common-values |  |
| `collections::slice` | `common/collections/slice.go` | T01 common-values |  |
| `collections::stack` | `common/collections/stack.go` | T01 common-values |  |
| `hashing` | `common/hashing/hashing.go` | T01 common-values |  |
| `hreflect` | `common/hreflect/helpers.go` | T01 common-values |  |
| `htime` | `common/htime/time.go` | T01 common-values | plus github.com/bep/clocks (Start/System) |
| `maps::cache` | `common/maps/cache.go` | T01 common-values |  |
| `maps::maps` | `common/maps/maps.go` | T01 common-values |  |
| `maps::ordered` | `common/maps/ordered.go` | T01 common-values |  |
| `maps::params` | `common/maps/params.go` | T01 common-values |  |
| `maps::scratch` | `common/maps/scratch.go` | T01 common-values |  |
| `math` | `common/math/math.go` | T01 common-values |  |
| `predicate` | `common/predicate/predicate.go` | T01 common-values |  |
| `types::types` | `common/types/types.go`, `common/types/closer.go`, `common/types/evictingqueue.go` | T01 common-values |  |
| `types::convert` | `common/types/convert.go` | T01 common-values |  |
| `types::hstring` | `common/types/hstring/stringtypes.go` | T01 common-values |  |
| `types::css` | `common/types/css/csstypes.go` | T01 common-values |  |
| `compare` | `compare/compare.go`, `compare/compare_strings.go` | T01 common-values |  |
| `identity` | `identity/identity.go` | T01 common-values | STUB: dependency tracking is not needed for a one-shot build |
| `dynacache` | `cache/dynacache/dynacache.go` | T01 common-values | SIMPLIFIED: get-or-create partitions, no eviction |
| `cast::caste` | — | T26 common-thirdparty-ports | PORT spf13/cast@v1.9.2 caste.go/basic.go/number.go/slice.go/map.go (ToStringE, ToIntE, ToInt64E, ToFloat64E, ToBoolE, ToStringSliceE, ToIntSliceE, ToStringMapE ...) |
| `cast::time` | — | T26 common-thirdparty-ports | PORT spf13/cast@v1.9.2 time.go + internal/time.go (ToTimeInDefaultLocationE, 24 layouts) |
| `locales` | — | T26 common-thirdparty-ports | PORT gohugoio/locales + gohugoio/localescompressed subset: Translator trait + generated en/th tables |
| `paths::path` | `common/paths/path.go` | T02 common-paths-text |  |
| `paths::pathparser` | `common/paths/pathparser.go`, `common/paths/type_string.go` | T02 common-paths-text |  |
| `paths::url` | `common/paths/url.go` | T02 common-paths-text |  |
| `urls` | `common/urls/baseURL.go`, `common/urls/ref.go` | T02 common-paths-text |  |
| `text` | `common/text/position.go`, `common/text/transform.go` | T02 common-paths-text |  |
| `hstrings` | `common/hstrings/strings.go` | T02 common-paths-text |  |
| `hugio` | `common/hugio/copy.go`, `common/hugio/hasBytesWriter.go`, `common/hugio/readers.go`, `common/hugio/writers.go` | T02 common-paths-text |  |
| `loggers` | `common/loggers/logger.go`, `common/loggers/loggerglobal.go`, `common/loggers/handlerdefault.go`, `common/loggers/handlersmisc.go`, `common/loggers/handlerterminal.go` | T02 common-paths-text | MINIMAL: levels + counters + stderr output; ERROR count fails the build |
| `kinds` | `resources/kinds/kinds.go` | T02 common-paths-text |  |
| `files` | `hugofs/files/classifier.go` | T02 common-paths-text |  |
| `glob::glob` | `hugofs/glob/glob.go` | T02 common-paths-text | plus a gobwas/glob port (only the syntax Hugo uses) |
| `glob::filename_filter` | `hugofs/glob/filename_filter.go` | T02 common-paths-text |  |
| `flect` | — | T26 common-thirdparty-ports | PORT gobuffalo/flect@v1.0.3: ident.go, humanize.go, titleize.go, ordinalize.go, pluralize.go, plural_rules.go, acronyms.go, custom_data.go, flect.go, rule.go, capitalize.go |
| `prose` | — | T26 common-thirdparty-ports | PORT jdkato/prose@v1.2.1 transform/title.go (AP style, including the rune-count/byte-offset bug) |

## Dependencies

- nh-*: none
- Wave A (to add when available): go-unicode, go-strconv, go-fmt, go-time, go-url, go-path, go-sort, go-hashstructure, go-json, go-html
- crates.io (justify each): xxhash-rust (xxh64), md-5, unicode-normalization (NFC only)

## Deliberate deviations

_Wave B: list every deviation from the Go code here (README rule 1)._

## Known gaps

_Wave B: list unported / stubbed functionality here._
