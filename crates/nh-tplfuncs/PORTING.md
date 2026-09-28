# nh-tplfuncs — porting notes

neohugo tpl/<namespace>/** template functions (all namespaces; seeksnack's set fully, the rest as stubs), registry, tplimplinit.

## Go file → Rust module

| Rust module | Go source(s) | Owner | Note |
|---|---|---|---|
| `internal::registry` | `tpl/internal/templatefuncsRegistry.go` | T19 tplfuncs-host |  |
| `internal::resourcehelpers` | `tpl/internal/resourcehelpers/helpers.go` | T15 resource-factories |  |
| `tplimplinit` | `tpl/tplimplinit/tplimplinit.go` | T19 tplfuncs-host |  |
| `cast::init` | `tpl/cast/init.go` | T18 tplfuncs-data | ported |
| `cast::cast` | `tpl/cast/cast.go` | T18 tplfuncs-data | ported |
| `cast::docshelper` | `tpl/cast/docshelper.go` | T18 tplfuncs-data | STUB (`hugo gen docshelper` only): explicit unsupported error |
| `collections::init` | `tpl/collections/init.go` | T18 tplfuncs-data | ported |
| `collections::append` | `tpl/collections/append.go` | T18 tplfuncs-data | ported |
| `collections::apply` | `tpl/collections/apply.go` | T18 tplfuncs-data | ported |
| `collections::collections` | `tpl/collections/collections.go` | T18 tplfuncs-data | ported |
| `collections::complement` | `tpl/collections/complement.go` | T18 tplfuncs-data | ported |
| `collections::index` | `tpl/collections/index.go` | T18 tplfuncs-data | ported |
| `collections::merge` | `tpl/collections/merge.go` | T18 tplfuncs-data | ported |
| `collections::querify` | `tpl/collections/querify.go` | T18 tplfuncs-data | ported |
| `collections::reflect_helpers` | `tpl/collections/reflect_helpers.go` | T18 tplfuncs-data | ported |
| `collections::sort` | `tpl/collections/sort.go` | T18 tplfuncs-data | ported |
| `collections::symdiff` | `tpl/collections/symdiff.go` | T18 tplfuncs-data | ported |
| `collections::where_` | `tpl/collections/where.go` | T18 tplfuncs-data | ported |
| `compare::init` | `tpl/compare/init.go` | T18 tplfuncs-data | ported |
| `compare::compare` | `tpl/compare/compare.go` | T18 tplfuncs-data | ported |
| `crypto::init` | `tpl/crypto/init.go` | T18 tplfuncs-data | ported |
| `crypto::crypto` | `tpl/crypto/crypto.go` | T18 tplfuncs-data | ported |
| `encoding::init` | `tpl/encoding/init.go` | T18 tplfuncs-data | ported |
| `encoding::encoding` | `tpl/encoding/encoding.go` | T18 tplfuncs-data | ported |
| `fmt::init` | `tpl/fmt/init.go` | T18 tplfuncs-data | ported |
| `fmt::fmt` | `tpl/fmt/fmt.go` | T18 tplfuncs-data | ported |
| `hash::init` | `tpl/hash/init.go` | T18 tplfuncs-data | ported |
| `hash::hash` | `tpl/hash/hash.go` | T18 tplfuncs-data | ported |
| `math::init` | `tpl/math/init.go` | T18 tplfuncs-data | ported |
| `math::math` | `tpl/math/math.go` | T18 tplfuncs-data | ported |
| `math::round` | `tpl/math/round.go` | T18 tplfuncs-data | ported |
| `math::gomath` | Go `math` (go1.27.1, arm64 build): `Exp` (asm), `Log`, `Pow`, `Sin`, `Cos`, `Tan`, `Asin`, `Acos`, `Atan`, `Atan2`, `Max`/`Min` (asm), `Frexp`, `Ldexp`, `Modf` | T18 tplfuncs-data | NEW: copy of `crates/gift/src/gomath.rs` + Tan, Asin, Acos, Atan, Atan2, Min, arm64 default NaN |
| `reflect::init` | `tpl/reflect/init.go` | T18 tplfuncs-data | ported |
| `reflect::reflect` | `tpl/reflect/reflect.go` | T18 tplfuncs-data | ported |
| `safe::init` | `tpl/safe/init.go` | T18 tplfuncs-data | ported |
| `safe::safe` | `tpl/safe/safe.go` | T18 tplfuncs-data | ported |
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
- Wave A (landed): go-value, go-fmt, go-json, go-url, go-time, go-html, go-strconv, go-sort, go-unicode (T18: Go `strings` helpers such as EqualFold, TrimSpace and ToLower). xtext-collate is reached through nh-langs (`Language::collator1`).
- crates.io (T18):
  - `md-5 0.10`: MD5 for `crypto.MD5` and `crypto.HMAC "md5"` (pure digest, output identical to Go's `crypto/md5`).
  - `sha2 0.10`: SHA-256 and SHA-512 for `crypto.SHA256` and `crypto.HMAC "sha256"/"sha512"`.
  - SHA-1 (no offline crate), HMAC, FNV-32a, hex and base64 (Go's `decodeQuantum` with its error offsets) are written out in `crypto.rs` and `encoding.rs`. XxHash comes from nh-common hashing.
  - dev-only: `serde_json` (fixture reader) and `flate2` (rust_backend; gunzips the oracle fixtures).
- Go `regexp` is `nh_common::goregexp`, a port of Go's package (no crates.io regex): `where … "like"` uses it through `hstrings::get_or_compile_regexp` (Go's cache); T19's `findRE`, `replaceRE`, `findRESubmatch` and `strings.*` regexp functions should too (`find_all`, `find_all_submatch`, `replace_all`/`replace_all_literal`, `split` over the template's byte strings).

## Deliberate deviations

- Go map iteration order is random. Where it can be seen, Rust uses sorted key order:
  - `sort` of a map with tied keys;
  - `merge`'s case-insensitive key lookup;
  - `querify`'s choice of error when several values are bad.
  The oracle records these cases as `unordered` or `nondet`, and the Rust result must be one of Go's possible results.
- `shuffle` and `math.Rand` use a private xorshift generator, not Go's `math/rand`. The oracle checks only permutation or range.
- `collections.Apply` does not check a function's signature for assignability (Go's `reflect.Value.Call` does). Functions come from the template store's `get_func` or from the namespaces' func map (test seam).
- `where`'s `evaluateSubElem` approximates Go's static types (method result and map element types come from the `Object` and value model). The arity error of a method call is mapped to Go's "… requires more than 1 parameter" text.
- A `*hugolib.pageWithOrdinal` wrapper's identity (in `uniq`/`intersect`/`in`/`union`) is its `Arc` pointer. `pageWithWeight0` is a value type and compares through `Unwrapv`.
- Named string types other than `hstring.HTML`, `json.Number` and `neohugo.VersionString` become plain `string` when `after`, `first` or `last` rebuild a slice of them.
- `fmt.Errorf/Warnf/Erroridf/Warnidf/Warnmf/Errormf` log through `Deps::log`. The `*mf` variants append ` key=value` fields in Go's field order.
- Arity errors use the Go method name, not the template alias.
- `compare.LtCollate` from a template receives a nil collator, as in Go, so it uses the default comparer.
- `math` follows the Go linux/arm64 build (see `math::gomath`):
  - FMA contraction at the sites the Go compiler fuses (tan, atan, asin);
  - arm64 default NaN `0x7FF8000000000000`;
  - `int(NaN/±Inf)` as arm64 converts it.
  The x86-64 Go build gives different bits for these values.
- `try` (alias registered by `tplimplinit`) is left to T19.

## Known gaps

- `cast::docshelper` is an explicit stub: `docs_provider()` returns "neohugo-rs: docshelper (hugo gen docshelper) is not supported".

## Verification

- `tests/data.rs` runs the Go oracle fixtures `tests/fixtures/data/*.json.gz`:
  - 94 topics, 515,023 cases over 673 shared values and 25 page entries;
  - every function of cast, collections (plus the site-deps and Thai-collator variants), compare, crypto, encoding, fmt, hash, math, reflect and safe;
  - the `*_test.go` tables;
  - pages from an in-memory hugolib build, including regular, section and home pages, the `pageWithWeight0` term pages and the `*pageWithOrdinal` GetTerms results.
- Each case records the value, the Go type/kind and the error text. Printed pointer addresses are masked.
- Result: 0 failures and no known divergences. The 1,256 cases that T18 had to mask now match Go: 48 `where … "like"` cases (the Go regexp port), 16 `lt`/`le`/`gt`/`ge` comparisons of a `neohugo.VersionString` with `uint64` max (nh-config's `Version` fields are Go `int`s now and keep `strconv.Atoi`'s clamped value) and 1,192 cast errors of pointer objects (nh-common's cast dereferences them like spf13/cast). 2 nondet and 16 unordered cases are accepted as sets, and the math counter is checked separately.
- `tests/go_tables.rs` holds the Go test tables transcribed literally.
- Regenerate the fixtures (they must come out byte for byte the same). The first step needs Go and the module cache; the second needs qemu-user-static:

  ```
  GOTOOLCHAIN=go1.27.1 go run ./tools/go-oracle/nh-tplfuncs/arm64build -o /tmp/data.arm64
  qemu-aarch64-static /tmp/data.arm64 -out crates/nh-tplfuncs/tests/fixtures/data
  ```

  `arm64build` cross-compiles `tools/go-oracle/nh-tplfuncs/data` for linux/arm64 with CGO off. A temporary `-modfile` swaps in local copies of gowebp and golibsass that carry `!cgo` stub files.
