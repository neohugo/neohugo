# nh-tplfuncs — porting notes

neohugo tpl/<namespace>/** template functions (all namespaces; seeksnack's set fully, the rest as stubs), registry, tplimplinit.

## Go file → Rust module

| Rust module | Go source(s) | Owner | Note |
|---|---|---|---|
| `internal::registry` | `tpl/internal/templatefuncsRegistry.go` | T19 tplfuncs-host | ported (+ `method_func`, `alias_func`) |
| `internal::resourcehelpers` | `tpl/internal/resourcehelpers/helpers.go` | T15 resource-factories | ported (+ Go-shaped `resolve_args_go`, `resolve_if_first_arg_is_string_go`, `transformer_from_value`) |
| `tplimplinit` | `tpl/tplimplinit/tplimplinit.go` | T19 tplfuncs-host | ported (explicit constructor list in Go import order; `try`) |
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
| `css::init` | `tpl/css/init.go` | T19 tplfuncs-host | ported |
| `css::css` | `tpl/css/css.go` | T19 tplfuncs-host | ported; `Sass` with transpiler `dartsass` is an explicit error |
| `data::init` | `tpl/data/init.go` | T19 tplfuncs-host | ported |
| `data::data` | `tpl/data/data.go` | T19 tplfuncs-host | STUB: `GetJSON`/`GetCSV` (deprecated, network) are explicit errors after the deprecation log |
| `data::resources` | `tpl/data/resources.go` | T19 tplfuncs-host | STUB (fetching for `data::data`) |
| `debug::init` | `tpl/debug/init.go` | T19 tplfuncs-host | ported |
| `debug::debug` | `tpl/debug/debug.go` | T19 tplfuncs-host | ported |
| `diagrams::init` | `tpl/diagrams/init.go` | T19 tplfuncs-host | ported |
| `diagrams::diagrams` | `tpl/diagrams/diagrams.go` | T19 tplfuncs-host | ported |
| `diagrams::goat` | `tpl/diagrams/goat.go` | T19 tplfuncs-host | STUB: `bep/goat` |
| `hugo::init` | `tpl/hugo/init.go` | T19 tplfuncs-host | ported |
| `images::init` | `tpl/images/init.go` | T19 tplfuncs-host | ported |
| `images::images` | `tpl/images/images.go` | T19 tplfuncs-host | ported; STUB `QR` (rsc.io/qr); `Text`, `Dither` and GIF `Config` fail in nh-images |
| `inflect::init` | `tpl/inflect/init.go` | T19 tplfuncs-host | ported |
| `inflect::inflect` | `tpl/inflect/inflect.go` | T19 tplfuncs-host | ported |
| `js::init` | `tpl/js/init.go` | T19 tplfuncs-host | ported |
| `js::js` | `tpl/js/js.go` | T19 tplfuncs-host | ported; STUB `Batch`; `Babel` reaches T16's stub client |
| `lang::init` | `tpl/lang/init.go` | T19 tplfuncs-host | ported |
| `lang::lang` | `tpl/lang/lang.go` | T19 tplfuncs-host | ported |
| `openapi3::init` | `tpl/openapi/openapi3/init.go` | T19 tplfuncs-host | ported |
| `openapi3::openapi3` | `tpl/openapi/openapi3/openapi3.go` | T19 tplfuncs-host | STUB: `Unmarshal` (kin-openapi); the resource checks before it are ported |
| `os::init` | `tpl/os/init.go` | T19 tplfuncs-host | ported |
| `os::os` | `tpl/os/os.go` | T19 tplfuncs-host | ported |
| `page::init` | `tpl/page/init.go` | T19 tplfuncs-host | ported |
| `partials::init` | `tpl/partials/init.go` | T19 tplfuncs-host | ported |
| `partials::partials` | `tpl/partials/partials.go` | T19 tplfuncs-host | ported |
| `path::init` | `tpl/path/init.go` | T19 tplfuncs-host | ported |
| `path::path` | `tpl/path/path.go` | T19 tplfuncs-host | ported |
| `resources::init` | `tpl/resources/init.go` | T15 resource-factories | ported |
| `resources::resources` | `tpl/resources/resources.go` | T15 resource-factories | ported |
| `site::init` | `tpl/site/init.go` | T19 tplfuncs-host | ported |
| `strings::init` | `tpl/strings/init.go` | T19 tplfuncs-host | ported |
| `strings::strings` | `tpl/strings/strings.go` | T19 tplfuncs-host | ported (+ `strings::diff`, go-internal `diff` v1.14.1 for `Diff`) |
| `strings::regexp` | `tpl/strings/regexp.go` | T19 tplfuncs-host | ported |
| `strings::truncate` | `tpl/strings/truncate.go` | T19 tplfuncs-host | ported |
| `templates::init` | `tpl/templates/init.go` | T19 tplfuncs-host | ported |
| `templates::templates` | `tpl/templates/templates.go` | T19 tplfuncs-host | ported (`DoDefer`: I01) |
| `time::init` | `tpl/time/init.go` | T19 tplfuncs-host | ported |
| `time::time` | `tpl/time/time.go` | T19 tplfuncs-host | ported |
| `transform::init` | `tpl/transform/init.go` | T19 tplfuncs-host | ported |
| `transform::transform` | `tpl/transform/transform.go` | T19 tplfuncs-host | ported; STUB `Highlight`, `HighlightCodeBlock`, `CanHighlight` (Chroma), `ToMath` (KaTeX), `PortableText`; `Emojify` reaches the nh-helpers stub |
| `transform::unmarshal` | `tpl/transform/unmarshal.go` | T19 tplfuncs-host | ported |
| `transform::remarshal` | `tpl/transform/remarshal.go` | T19 tplfuncs-host | ported |
| `urls::init` | `tpl/urls/init.go` | T19 tplfuncs-host | ported |
| `urls::urls` | `tpl/urls/urls.go` | T19 tplfuncs-host | ported |

## Dependencies

- nh-*: nh-common, nh-config, nh-media, nh-langs, nh-hugofs, nh-helpers, nh-markup, nh-resource, nh-images, nh-page, nh-allconfig, nh-tpl, nh-tplimpl, nh-resources, nh-resource-transformers, nh-parser, nh-deps
- Wave A (landed): go-value, go-fmt, go-json, go-url, go-time, go-html, go-strconv, go-sort, go-unicode (T18: Go `strings` helpers such as EqualFold, TrimSpace and ToLower). xtext-collate is reached through nh-langs (`Language::collator1`).
- crates.io (T18):
  - `md-5 0.10`: MD5 for `crypto.MD5` and `crypto.HMAC "md5"` (pure digest, output identical to Go's `crypto/md5`).
  - `sha2 0.10`: SHA-256 and SHA-512 for `crypto.SHA256` and `crypto.HMAC "sha256"/"sha512"`.
  - SHA-1 (no offline crate), HMAC, FNV-32a, hex and base64 (Go's `decodeQuantum` with its error offsets) are written out in `crypto.rs` and `encoding.rs`. XxHash comes from nh-common hashing.
  - dev-only: `serde_json` (fixture reader) and `flate2` (rust_backend; gunzips the oracle fixtures).
- Go `regexp` is `nh_common::goregexp`, a port of Go's package (no crates.io regex): `where … "like"` uses it through `hstrings::get_or_compile_regexp` (Go's cache); T19's `findRE`, `replaceRE`, `findRESubmatch` and `strings.*` regexp functions should too (`find_all`, `find_all_submatch`, `replace_all`/`replace_all_literal`, `split` over the template's byte strings). T19's `strings` functions do.
- T19 adds `go-path` (Go `path`, for `path.*`/`urls.JoinPath`) and `goldmark` (`util.VisualizeSpaces` for `debug.VisualizeSpaces`).
- dev-only (T19): `nh-hugolib`. This is a dev-dependency cycle (nh-hugolib depends on nh-tplfuncs); it works because the integration tests link a single copy of each crate. `tests/host.rs` builds real sites with it (process + assemble + freeze).
- `tests/resources.rs` (T15) replays `tools/go-oracle/nh-resource-transformers` scripts through `resources::resources`; its notes and counts are in `crates/nh-resource-transformers/PORTING.md`.

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
- Arity errors of a namespace method use the Go method name. Through a func map alias (`internal::registry::alias_func`) they name the alias, as text/template does.
- `compare.LtCollate` from a template receives a nil collator, as in Go, so it uses the default comparer.
- `math` follows the Go linux/arm64 build (see `math::gomath`):
  - FMA contraction at the sites the Go compiler fuses (tan, atan, asin);
  - arm64 default NaN `0x7FF8000000000000`;
  - `int(NaN/±Inf)` as arm64 converts it.
  The x86-64 Go build gives different bits for these values.
- T19 (host namespaces):
  - `tplimplinit::namespaces` lists the constructors explicitly in Go's import order (Go fills the registry from package `init()`s). `create_func_map` panics on a duplicate name, as Go does. `try` is added to the `safe` namespace (exactly one argument, returned as is); `.Err.Data` is not available because go_value errors carry only a message.
  - The `hugo` and `site` funcs read the site lazily from `Deps::site` (a `OnceLock`): the func map is built before the site exists. With no site, `site` is a typed nil `*page.siteWrapper` and `hugo` is an error.
  - `partial`, `partialCached` and `return` share one namespace object. The partial cache is an LRU of 1000 entries (Go's lazycache: the placeholder goes in before the partial runs and is removed on error), cleared on build start.
  - The js clients are shared per `Deps` through the dynacache partition `/tmpl/js/clients` (keyed by the `Arc<Deps>` pointer), so `resources.Babel` and `js.Build` use the same clients.
  - Go mutates the option dicts passed to `transform.Unmarshal` (`decodeDecoder`) and `transform.Remarshal` (`applyMarshalTypes`). Template maps are immutable in the port, so the caller's dict is not changed.
  - `templates.Current.Ancestors` of a top-level template is an empty list, not a typed nil (the same in templates: `len` is 0, `range` runs 0 times).
  - Go panics reached through a template (`strings.Repeat` overflow and makeslice, `SliceString`, `Truncate` bounds, `lang.FormatNumberCustom` slices, `urls.JoinPath` of nothing) are returned as errors with Go's runtime error text.

## Known gaps

- `cast::docshelper` is an explicit stub: `docs_provider()` returns "neohugo-rs: docshelper (hugo gen docshelper) is not supported".
- T19 stubs. Each returns `neohugo-rs: ... is not supported`:
  - `diagrams.Goat`, `openapi3.Unmarshal`, `getJSON`/`getCSV`, `images.QR`, `js.Batch`;
  - `transform.Highlight`, `HighlightCodeBlock`, `CanHighlight`, `ToMath`, `PortableText`;
- These reach stubs in other crates:
  - `transform.Emojify` (nh-helpers emoji table);
  - `images.Text`, `images.Dither` and `images.Config` of a GIF (nh-images);
  - `css.Sass` with Dart Sass.
- Known divergences caused by other crates: none remain. The earlier ones were fixed in their
  crates: invalid UTF-8 through `strings.Title` and the `urls` functions (nh-config's
  `create_title_bytes`, nh-helpers' `abs_url_bytes`/`rel_url_bytes`/`urlize_bytes`),
  `os.ReadFile` of a directory (nh-hugofs' `OsFile` read errors are Go's `*PathError`), CSV
  decoding (nh-parser; Go's Unmarshal cache then returns the cached CSV document as Go does),
  and the `deprecated: ` log field (nh-config's `deprecate`).

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

  `arm64build` cross-compiles `tools/go-oracle/nh-tplfuncs/data` (or the package given with `-pkg`) for linux/arm64 with CGO off. A temporary `-modfile` swaps in local copies of gowebp and golibsass that carry `!cgo` stub files.
- `tests/host.rs` (T19) runs the Go oracle fixtures `tests/fixtures/host/*.json.gz` (53 files) from `tools/go-oracle/nh-tplfuncs/host`:
  - `sites.json.gz` holds 11 sites (en, th, sub, canon, multi, multisub, multihost, ap, chicago, gostyle, firstupper), the clock (Hugo's `--clock`, 2021-11-17T20:34:10+07:00), and each site's log and collected errors after the cases.
  - The Rust side writes the same sites to a temp dir, builds them with nh-hugolib (process + assemble + freeze, the real func map), and replays every case in order through the func map or namespace method. Case arguments can be pages of the site or nested calls. The typed result or error text must be Go's.
  - Site paths are masked as `/SITE`, and printed addresses are masked.
  - 51 topics and 56,060 cases:
    - css, debug, go_tables (the `*_test.go` tables), hugo;
    - images_config, images_filter(s), inflect, js;
    - lang_custom, lang_format, lang_merge, lang_translate;
    - os, partials, partials_cached, path;
    - strings_* (aliases, binary, diff, regexp, repeat, replace, slice, split, title, truncate, unary), stubs, templates;
    - time_astime, time_duration, time_format, time_in, time_now;
    - transform_markdownify, transform_remarshal, transform_text, transform_unmarshal;
    - urls, urls_ref, and urls_<site><n> for every site and language.
  - `funcnames` is Go's `CreateFuncMap` name set (155 names). Every site's func map must equal it and T20's `crates/nh-hugolib/tests/fixtures/funcnames` fixture.
  - Result: 0 differences.
    - 127 cases hit a listed stub and must fail with the `neohugo-rs:` error (`STUBS`).
    - No known divergences remain (54 before the cross-crate fixes).
    - `time.Now` is checked against the clock.
    - 7 of the 12 js cases (`js.Build`) need the pinned esbuild (`NEOHUGO_ESBUILD_BINARY`, `tools/esbuild/build.sh`). Without it they are skipped, and so are their JSBUILD errors.
- Regenerate the host fixtures (they must come out byte for byte the same). Everything except css runs as linux/arm64 under qemu, because float formatting must match the arm64 build. css runs natively, because libsass needs cgo:

  ```
  GOTOOLCHAIN=go1.27.1 go run ./tools/go-oracle/nh-tplfuncs/arm64build -pkg ./tools/go-oracle/nh-tplfuncs/host -o /tmp/host.arm64
  qemu-aarch64-static /tmp/host.arm64 -skip css -out crates/nh-tplfuncs/tests/fixtures/host
  GOTOOLCHAIN=go1.27.1 go run ./tools/go-oracle/nh-tplfuncs/host -only css -out crates/nh-tplfuncs/tests/fixtures/host
  ```
