# tdewolff-minify — porting notes

Byte-exact port of `github.com/tdewolff/minify/v2@v2.23.8` (the version
pinned in neohugo's `go.mod`) **without** the `js` minifier, which is ported
separately in `crates/tdewolff-minify-js` and plugs into [`M`] through the
[`Minifier`] trait like any other minifier.

Golden toolchain: go1.27.1 darwin/arm64. Built on `crates/tdewolff-parse`
(`GoBytes`, `Input`, lexers/parsers, `GoError`), `go-strconv`
(`ParseFloat(s, 32)`, `AppendFloat(f, 'g', -1, 64)`) and `go-sort`
(`sort.Slice`).

## Go file → Rust module map

| Go (`minify/v2@v2.23.8/…`) | Rust |
|---|---|
| `minify.go` | `src/lib.rs` (`M`, `Minifier`, `MinifierFunc`, `Writer`, `Regexp`, `Url`, `err_not_exist`/`is_err_not_exist`, `M::{add, add_func, add_regexp, add_func_regexp, match_, minify, minify_mimetype, bytes, string}`) |
| `common.go` | `src/common.rs` (`mediatype`, `data_uri`, `decimal`, `number`, `update_error_position`, `EPSILON`, `MAX_INT`, `MIN_INT`; Go's `base64.StdEncoding.Encode` as `base64_std_encode`) |
| `html/html.go` | `src/html/mod.rs` (`html::Minifier`, `html::minify`, `EntitiesMap`, `TextRevEntitiesMap`, template delimiters) |
| `html/buffer.go` | `src/html/buffer.rs` (`Token`, `TokenBuffer`) |
| `html/table.go` | `src/html/table.rs` (traits, `tag_map`, `attr_map`, `js_mimetypes`; **generated**) and `src/html/entities.rs` (`EntitiesMap` data, 1090 entries sorted; **generated**) |
| `html/hash.go` | `src/html/hash.rs` (**generated**, perfect hash ported literally, 265 names) |
| `css/css.go` | `src/css/mod.rs` (`css::Minifier`, `css::minify`, `css::Token`, the grammar/selector/declaration/token/property minifiers) |
| `css/util.go` | `src/css/util.rs` (`remove_markup_newlines`, `rgb_to_token`) |
| `css/table.go` | `src/css/table.rs` (`optional_zero_dimension`, `shorten_color_hex`, `shorten_color_name`; **generated**; `PropertyOverrides` is unused by the minifier and not ported) |
| `css/hash.go` | `src/css/hash.rs` (**generated**, 420 names) |
| `json/json.go` | `src/json.rs` |
| `svg/svg.go` | `src/svg/mod.rs` (`svg::Minifier`, `svg::minify`, `shortenDimension`, `printTag`, `skipTag`) |
| `svg/buffer.go` | `src/svg/buffer.rs` |
| `svg/pathdata.go` | `src/svg/pathdata.rs` (`PathData`, `PathDataState`) |
| `svg/table.go` | `src/svg/table.rs` (`color_attr_map`; **generated**) |
| `svg/hash.go` | `src/svg/hash.rs` (**generated**, 112 names) |
| `xml/xml.go` | `src/xml/mod.rs` |
| `xml/buffer.go` | `src/xml/buffer.rs` |
| `xml/table.go` | `src/xml/table.rs` (hand-ported, 3+2 entries; the generator checks it against Go) |
| `Peek`/`Shift` of the three `buffer.go` | `src/tokbuf.rs` (`GoTokenBuf`: the identical algorithm, generic over the token type) |

Generated files come from `tools/go-oracle/tdewolff-minify tables` (Go AST +
go/types over the module sources; every hash constant is checked against
`ToHash`/`String()` of the real package). Regenerate everything with
`SCRATCH=… tools/go-oracle/tdewolff-minify/gen.sh`.

## Public API for downstream crates (nh-minifiers, tdewolff-minify-js)

```rust
use tdewolff_minify::{M, Minifier, MinifierFunc, Regexp, Writer, GoBytes, GoError, GoReader, Params};

let mut m = M::new();
m.add("text/css", Arc::new(css::Minifier { keep_css2: true, ..Default::default() }));
m.add_regexp(Regexp::must_compile(r"^(application|text)/(x-|(ld|manifest)\+)?json$"), Arc::new(json::Minifier::default()));
m.add_func("text/x", |m, w, r, params| { … });          // Go AddFunc
m.minify("text/html", &mut out /* Vec<u8> */, &mut reader /* GoReader */)?;
let (mimetype_or_pattern, params, minifier) = m.match_("text/html; charset=utf-8"); // Go Match
let (out, err) = m.bytes("image/svg+xml", GoBytes::from_slice(data)); // Go Bytes (returns input on error)
```

* `Minifier` (Go `minify.Minifier`): `fn minify(&self, m: &M, w: &mut dyn
  Writer, r: &mut dyn GoReader, params: Option<&Params>) -> Result<(), GoError>`.
  `Send + Sync`; `M` is `Send + Sync` and can be shared between threads (each
  minify call keeps its `GoBytes` on its own thread).
* Readers are `tdewolff_parse::GoReader`; pass a
  `tdewolff_parse::buffer::Reader::new(GoBytes)` (Go `buffer.NewReader`) so the
  minifier works **in place on the caller's bytes** exactly like Go (`Bytes()`
  is used, the NUL sentinel goes into spare capacity and is restored).
* `Writer` (Go `io.Writer`): implemented for `Vec<u8>` and
  `tdewolff_parse::buffer::Writer`. `write_go(&GoBytes)` writes a Go slice.
* Options structs have Go's fields in snake_case:
  `html::Minifier { keep_comments, keep_conditional_comments, keep_special_comments, keep_default_attr_vals, keep_document_tags, keep_end_tags, keep_quotes, keep_whitespace, template_delims: [String; 2] }`,
  `css::Minifier { keep_css2, precision, inline }`,
  `json::Minifier { precision, keep_numbers }`,
  `svg::Minifier { keep_comments, precision, inline }`,
  `xml::Minifier { keep_whitespace }`. Go's unexported `newPrecision` is
  computed inside `Minify` (as Go does on its copy).
* Package-level `Minify` functions: `html::minify`, `css::minify`,
  `json::minify`, `svg::minify`, `xml::minify` (default options).
* Helpers for the js port: `number(GoBytes, prec) -> GoBytes`,
  `decimal`, `data_uri(&M, GoBytes)`, `mediatype(GoBytes)`,
  `update_error_position(GoError, &Input, offset)`, `EPSILON`,
  `err_not_exist()`/`is_err_not_exist(&GoError)`, `RestoreGuard` (Go
  `defer z.Restore()`), `param_is(params, key, value)`
  (`params["inline"] == "1"`). All byte helpers work in place like Go.
* Tables: `html::EntitiesMap`/`TextRevEntitiesMap` and
  `xml::EntitiesMap`/`TextRevEntitiesMap` implement
  `tdewolff_parse::{EntityMap, RevEntityMap}`; `css::shorten_color_hex`,
  `css::shorten_color_name`, `css::to_hash`, `html::to_hash`, `svg::to_hash`
  and all Go hash constants (`html::Http_Equiv`, `css::Font_Face`, …).
* `svg::PathData::new(&svg::Minifier)` / `shorten_path_data(GoBytes)` (Go
  `NewPathData`/`ShortenPathData`; `newPrecision` is 0 unless created by
  `Minify`, exactly as in Go).

## Deliberate deviations (API shape only; bytes are identical)

* `[]byte` → `GoBytes` everywhere a Go slice may alias the input or another
  buffer; Go `nil` checks are `GoBytes::is_nil()`. Package-level `[]byte`
  globals that end up in token data are materialised as fresh `cap == len`
  copies (Go shares one array but never writes through it on these paths).
* `M.Match` is `M::match_` (`match` is a keyword) and returns the minifier as
  `Option<Arc<dyn Minifier>>`.
* `M.AddRegexp` takes a `Regexp`, a wrapper over the `regex` crate
  (`regex::bytes`, unanchored `is_match`). Justification (README rule 2): Go
  RE2 and Rust `regex` implement the same syntax and leftmost-first semantics
  for the mimetype patterns used by neohugo and upstream
  (`^(application|text)/(x-)?(java|ecma)script$`,
  `^(application|text)/(x-|(ld|manifest)\+)?json$`, `[/+]xml$`, …); the only
  known difference is that `.` in Rust's Unicode mode does not match an
  invalid UTF-8 byte while Go matches it as U+FFFD — irrelevant for ASCII
  mimetypes. The regexp only decides *which* minifier runs; it never produces
  output bytes.
* `M.URL` is `Option<Url { scheme }>` (only `Scheme` is read, by HTML URL
  attributes).
* `ErrNotExist` is `GoError::Other("minifier does not exist for mimetype")`;
  identity comparison becomes message comparison (`is_err_not_exist`).
* HTML `KeepConditionalComments`: Go prints the deprecation line to stdout
  once per `*Minifier` (it flips the flag on the receiver); the Rust
  `Minifier` is immutable (`&self`), so the line is printed on every call.
  neohugo removes the key before decoding, so it never prints either way.
* `TokenBuffer.Attributes` returns buffer indices (`Vec<Option<usize>>`,
  dereferenced with `token_mut`) instead of `[]*Token`.
* `AddCmd`/`AddCmdRegexp`, `M.Reader`, `M.Writer`, `M.ResponseWriter`,
  `Middleware*` and the `Warning` logger are not ported (not used by neohugo).

## Go semantics that needed care

* In-place rewrites and aliasing are emulated with `GoBytes`: `Number`/
  `Decimal` (write past the returned window), `Mediatype`, `DataURI`
  (in-place percent-decoding, `EncodeURL` appending into spare capacity of
  the caller's token), `minifyColor`, `minifyDimension` (`dim` read after
  `append(num, dim...)` moved it), `removeMarkupNewlines`, the viewBox and
  `shortenDimension` rewrites, path data (`copyNumber` rewrites `…00` to
  `…e2` inside the input, the output is copied over the input it is still
  parsing), HTML end-tag truncation and `http:` lowercasing, the shared
  4-byte buffer of `background-position` right/bottom rewrites.
* `[]Token` slice surgery in `minifyProperty` (`append(values[:i], …)`,
  windows passed to recursive calls) is ported with `Vec` splices; every
  call site was checked to read only the window prefix the callee returns.
* CSS `parseDeclaration` returns Go `nil` for "no tokens" only while the
  reused `tokenBuffer` is still nil (first declarations of a stylesheet) —
  ported (`token_buffer_nil`), since nil selects the raw-token path.
* Go integer wrap-around (`Number` exponents, unicode ranges, `100-n`) uses
  `wrapping_*`; `int(math.Pow(16, n))` is exact/`+Inf`→saturating;
  `fmt.Sprintf("%X")` of negative ints prints a sign.
* `sort.Slice` of unicode ranges uses `go_sort::sort_by` (pdqsort, identical
  tie order).
* `byte(f)` in `rgbToToken` is `FCVTZSDW` (float64 → int32, saturating)
  followed by the low byte: `(x as i32) as u8`.

## FMA sites (darwin/arm64, verified with `go tool objdump`)

`go tool objdump -s tdewolff/minify/v2 <oracle> | grep -E 'FMADD|FMSUB|FNMADD|FNMSUB'`
(non-js packages) → 3 instructions, all in one function:

| Go site | instruction | Rust |
|---|---|---|
| `css/util.go:39` `byte((r * 255.0) + 0.5)` (×3: r, g, b) | FMADDD | `x.mul_add(255.0, 0.5)` in `css::util::rgb_to_token` |

Not fused: `svg/pathdata.go` `2*p.x-p.cx` (exact either way), all other
float code (only additions, divisions, `math.Mod`/`Modf`/`Abs`). HSL2RGB's
fused sites live in `tdewolff-parse`. A search over 3.6M `hsl()` and 100k
`rgb(p%)` inputs found no input where the fusion changes the output byte, so
the fixtures cannot distinguish it; it is ported anyway.

## Tests

Oracle: `tools/go-oracle/tdewolff-minify` (package main in the neohugo
module; `gen.sh` regenerates everything). Modes: `tables`, `fixtures DIR
[SCALE]`, `fuzz`, `digests`, `nested`, `tree`, `stdin`. Checked-in fixtures
(2.1 MB, gzip TSV) in `tests/fixtures/`, read with `flate2` (decompression
only).

* `tests/upstream.rs` — the upstream `_test.go` tables, extracted literally
  (911 rows: html ×9 tests, css ×3, json ×2, svg ×4 + path data ×2, xml ×2,
  common ×6); each row is run with the same setup as the Go test and must
  equal Go's output; 909 rows equal the upstream expectation, the 2 others
  need the js minifier (`TestHTMLCSSJS`, run without js). Plus hand-ported
  `minify_test.go` (Minify/Bytes/String/Add/Match/Wildcard), reader/writer
  error tables of all five packages, nested-minifier error propagation,
  `TestSpecialTagClosing`, `Token.String`, and the three `buffer_test.go`.
* `tests/fixtures.rs` — `literals` (every upstream test literal through 21
  configurations × mediatypes: 14942 records), `fuzz` (random mutations of
  literals and site corpora: 9648), `structured` (grammar-shaped CSS
  declarations, SVG path data, HTML tags/attributes: 4500), `units`
  (`Number` ×7 precisions, `Decimal` ×5, `Mediatype`, `DataURI` ×3 configs,
  `PathData` ×3 precisions: 85075). Each record checks output bytes, the
  error string and the input buffer after the call (in-place rewrites).
  Configurations include neohugo's seeksnack setup, all option combinations
  of the upstream tests, `M.URL` http/https, template delimiters, and
  stand-in nested minifiers (copy, error, trimming "js", a js minifier
  returning a positioned `*parse.Error`).
* `tests/corpus.rs` (`#[ignore]`, needs the corpora): the 4910
  .html/.xml/.json files of `golden/nominify`, the 2672 pristine site
  sources (templates, SCSS/CSS, SVG, JSON), the recorded nested calls of a
  real build (`corpus2`: 3 CSS, 1719 JSON-LD, 4 SVG data-URI, the postcss
  resource), and golden parity with a js stand-in.

Results (2026-09-27, debug with overflow checks and release):

* checked-in fixtures: 114,165 / 114,165 records; upstream 911/911 rows;
  the 17 other tests of `upstream.rs` pass.
* 30× set (`$SCRATCH/work/tdewolff-minify/fixtures-full`): literals
  14,942/14,942, fuzz 289,460/289,460, structured 135,000/135,000, units
  1,351,331/1,351,331; a second fuzz seed (257,301 records) also 100%.
* golden/nominify through the seeksnack config without js: **4910/4910**
  digests equal Go (205 MB of input); pristine sources 2672/2672;
  corpus2 nested 1727/1727 equal Go and the recorded outputs.
* golden `--minify` parity (js calls answered from the 6 recorded Go js
  outputs): **4900/4910 files byte-identical to golden/canonical**; the other
  10 are 5 static files neohugo publishes unminified and 5 files whose
  pre-minify input differs between Go builds (term-title collisions:
  "Lay's"/"Lays", "INS 322(i)"/"INS 322i", "5'-"/"5-"), not minifier output.

## Known gaps

* The js minifier (separate crate). Upstream table rows that need it (2 rows
  of `TestHTMLCSSJS`; `TestMinifyErrorPropagation`) are only checked against
  Go-without-js.
* `AddCmd*`, streaming `Reader`/`Writer`/`ResponseWriter`/`Middleware` (not
  used by neohugo).
