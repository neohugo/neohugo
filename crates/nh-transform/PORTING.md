# nh-transform — porting notes

neohugo `transform/{chain,urlreplacers,livereloadinject,metainject}` and `minifiers/*` (the
tdewolff registry by media type). Wave B task T07 (transform-publisher).

## Go file → Rust module

| Rust module | Go source(s) | Owner | Note |
|---|---|---|---|
| `chain` | `transform/chain.go` | T07 transform-publisher | |
| `urlreplacers::absurl` | `transform/urlreplacers/absurl.go` | T07 transform-publisher | |
| `urlreplacers::absurlreplacer` | `transform/urlreplacers/absurlreplacer.go` | T07 transform-publisher | quirks kept, see below |
| `livereloadinject` | `transform/livereloadinject/livereloadinject.go` | T07 transform-publisher | ported (server only, never in a build) |
| `metainject` | `transform/metainject/hugogenerator.go` | T07 transform-publisher | ported (never active: neohugo inverted the flag) |
| `minifiers::config` | `minifiers/config.go` | T07 transform-publisher | |
| `minifiers::minifiers` | `minifiers/minifiers.go` | T07 transform-publisher | |

Every checklist entry is `OK`; nothing is stubbed.

## Public API

- `chain::{Chain, Transformer, FromTo}`: `Chain::apply(&[u8]) -> Result<Vec<u8>>`. A transformer
  reads `ft.from` (Go's `ft.From().Bytes()`) and appends to `ft.to`.
- `urlreplacers::absurl::{new_abs_url_transformer, new_abs_url_in_xml_transformer}`;
  `absurlreplacer::{replace_in_html, replace_in_xml}` now return `Result<()>` (the Go panic,
  below). No other crate used them.
- `minifiers::config::{MinifyConfig, TdewolffConfig, HtmlOptions, CssOptions, JsOptions,
  JsonOptions, SvgOptions, XmlOptions}` with Go's fields (including the unexported ones
  mapstructure matches but never sets), `decode_config(&Value)` (Go `DecodeConfig`),
  `default_config()` (Go `defaultConfig`; `MinifyConfig::default()` is Go's zero value, what
  mapstructure's `ZeroFields` would use). `TdewolffConfig` changed from the skeleton's flattened
  fields to one struct per Go minifier (no other crate used the fields); each has
  `minifier()` → the `tdewolff_minify`/`tdewolff_minify_js` options struct.
- `minifiers::minifiers::Client { m: Arc<tdewolff_minify::M>, minify_output }`:
  `Client::new(types, formats, &dyn AllProvider)` (reads the `minify` config section as
  `MinifyConfig`, like Go), `Client::from_config(types, formats, &MinifyConfig)` (new: the same
  without a provider), `transformer(&MediaType) -> Option<Transformer>`,
  `minify(&MediaType, &[u8]) -> Result<Vec<u8>>`.
- `livereloadinject::new_transformer(&go_url::Url)`, `metainject::{hugo_generator_transform,
  hugo_generator_transformer}` (new). The skeleton's `livereloadinject::new()` and
  `metainject::hugo_generator()` still return `None` (a build never has these transformers).

## Dependencies

- nh-*: nh-common, nh-config, nh-media.
- Wave A: tdewolff-minify, tdewolff-minify-js, tdewolff-parse, go-unicode, go-url, go-html.
- crates.io: none. Tests only: `serde_json`, `flate2` (decompression), `base64` (fixture byte
  fields) — all on the README list.

## Go behaviour reproduced on purpose

- absURL lexer (`absurlreplacer.go`): every quirk of spec output-publishing §3.3 — `nextPos`
  starts at Go's zero value 0 (content starting with a candidate is written several times),
  stale `nextPos` values are not rechecked (bytes can be duplicated), `srcset` whitespace is
  collapsed to single spaces and `srcset` must be quoted, unquoted `href=/x` is rewritten,
  prefixes are case-sensitive and not attribute-aware (`data-src=`, text, scripts), the
  2000-byte `srcset` guard, `root` stripping for a baseURL with a path, `&#34;`/`&#39;` as the XML
  quotes, and a relative (`relativeURLs`) path parsed with `url.Parse` for its root.
- The lexer works on the whole buffer (`ft.From().Bytes()`): Go's `Chain.Apply` reads the whole
  source before the first transformer, so the source's chunking never matters (the oracle
  checks one-byte and 7-byte readers give the same bytes).
- `DecodeConfig`: the `decimal` → `precision` (only if > 0 and precision unset) and
  `keepConditionalComments` → `keepSpecialComments` renames are applied to the nested maps only
  when Go's `maps.ToStringMap` returns the same map (a `map[string]interface {}` or
  `maps.Params`); a `map[string]string` gets a copy in Go, so the rename is lost there too. Then
  mapstructure's `WeakDecode` (nh-config's port) with Go's error texts.
- `minifiers.New`: the registration order and the two regexps (`^(application|text)/(x-)?(java|
  ecma)script$`, `^(application|text)/(x-|(ld|manifest)\+)?json$`), the `noopMinifier` for
  disabled types, every `IsHTML` output format's media type gets the HTML minifier.
- `livereloadinject`/`metainject` regexps are matched by hand with RE2 semantics: `\s` is
  `[\t\n\f\r ]`; `(?i)` on `doctype`, `html`, `head`, `meta`, `name`, `generator` is ASCII case
  folding (none of their letters has a non-ASCII simple fold).

## Deliberate deviations

- **Go panics become errors.** A content starting with a quote followed by a candidate (e.g.
  `"/x"`) makes Go's lexer panic (`slice bounds out of range [2:1]` in `emit`, after the
  zero-`nextPos` rewind); the Go build crashes. The port returns
  `Err("runtime error: slice bounds out of range [2:1]")` from the transformer (README rule 9);
  the oracle records 93 such inputs and the test requires the error with Go's text.
- `Chain::apply` returns a failing transformer's error as it is. Go writes the failing step's
  input to a temp file and wraps the error in a `herrors.FileError` naming that (random) file;
  the publisher then wraps it in `failed to process "<target>": …` (ported in nh-publisher).
- `Client::transformer`/`minify` copy the input into a `GoBytes` (Go minifies the pooled buffer
  in place); the output bytes are the same.
- The HTML minifier's `KeepConditionalComments` deprecation line is printed per call (see
  tdewolff-minify PORTING.md); neohugo's config loader removes that key, so it never prints.
- Stack depth: the minifiers recurse like Go's (JS parser/printer, nested minifier calls);
  callers minifying untrusted input run on a large stack (the tests use 256 MiB; see the
  tdewolff-minify(-js) PORTING.md).

## Tests and oracles

| test | oracle | cases |
|---|---|---|
| `tests/absurl.rs` | `tools/go-oracle/nh-transform/absurl` → `fixtures/absurl/cases.jsonl.gz` | 94,180: the upstream `absurlreplacer_test.go` tables and the spec §3.3 vectors × 25 base paths × HTML/XML, a prefix × quote × value × suffix grid (13 × 11 × 26 × 11 per kind: quoted/unquoted/`&#34;`/`&#39;`/`&quot;`, protocol-relative, root paths, invalid UTF-8, `>` and spaces in values), 10,000 random documents, `srcset` around the 2000-byte guard; 93 Go panics; plus the upstream expectations verbatim |
| `tests/inject.rs` | same oracle (`inject.go`) → `fixtures/absurl/inject.jsonl.gz` | 200: livereload (7 base URLs) and generator tag over 25 documents |
| `tests/minify.rs` | `tools/go-oracle/nh-transform/minify` (overlay, **linux/arm64 under qemu**) → `fixtures/minify/minify.jsonl.gz` | 29 `DecodeConfig` inputs (lower/camel case keys, both renames, `map[string]string`/`maps.Params` nesting, weak conversions, 4 decode errors) and 1,100 client cases: 4 configs (default, customised, all disabled, minifyOutput off) × media types (the default ones, three custom ones, regexp-only ones like `application/ld+json`, `text/x-javascript`, and unregistered ones) × the docs site's CSS/JS/SVG/JSON assets and hand-written snippets (colours, precision, errors); `Transformer` (nil or output/error) and `Client.Minify` (output/error) |

nh-publisher's `tests/site.rs` checks the whole chain (absURL + minify through the publisher)
on the real site builds.

Regenerating (from the repo root, `GOTOOLCHAIN=go1.27.1`):

```sh
go run ./tools/go-oracle/nh-transform/absurl
NH_T07_ORACLE_ARCH=arm64 go run ./tools/go-oracle/nh-transform/minify   # needs qemu-aarch64-static
```

Both regenerate their `.gz` files byte for byte (gzip without name/time; the random inputs have
fixed seeds). The minify fixture must come from arm64 (CSS/SVG colour conversions fuse FMAs,
tdewolff-minify PORTING.md §FMA).

## Known gaps

None in scope. `transform.BytesReader`/streaming readers are not modelled (every Go caller
passes a `*bytes.Buffer`).
