# nh-parser — porting notes

neohugo parser/* : metadecoders (YAML via go-yaml, TOML via a port of pelletier/go-toml/v2, JSON via
go-json), pageparser lexer. Owner and crate lead: Wave B task T03 (parser-langs).

## Go file → Rust module

| Rust module | Go source(s) | Status / note |
|---|---|---|
| `metadecoders::decoder` | `parser/metadecoders/decoder.go` | ported; ORG decoding STUB (explicit `neohugo-rs:` error) |
| `metadecoders::csv` | go1.27.1 `encoding/csv/reader.go` | NEW: the reader subset `unmarshalCSV` uses (`Comma`, `Comment`, `LazyQuotes`, `ReadAll`) |
| `metadecoders::xml` | go1.27.1 `encoding/xml/xml.go` | NEW: the strict `Decoder.Token` path (no `CharsetReader`/`Entity`/`AutoClose`), the name tables, `EscapeText` |
| `metadecoders::mxj` | `github.com/clbanning/mxj/v2@v2.7.0` `xml.go`, `anyxml.go`, `misc.go` | NEW: `NewMapXml` + `Map.Root` and `AnyXmlIndent` of a map, with mxj's default settings |
| `metadecoders::toml::marshaler` | `github.com/pelletier/go-toml/v2@v2.2.4/marshaler.go` | NEW: the `Encoder` with `SetIndentTables(true)` over map/slice/scalar/time/TextMarshaler values (no structs) |
| `metadecoders::format` | `parser/metadecoders/format.go` | ported (all functions) |
| `metadecoders::toml` (+ `toml/*`) | `github.com/pelletier/go-toml/v2@v2.2.4`: `unstable/{ast,builder,kind,parser,scanner}.go` (`toml/parser.rs`), `internal/characters` (`toml/characters.rs`), `decode.go` (`toml/decode.rs`), `errors.go` (`toml/errors.rs`), `internal/tracker/seen.go` (`toml/tracker.rs`), `localtime.go` (`toml.rs`), `unmarshaler.go` for `any`/`map[string]any` targets (`toml/unmarshaler.rs`) | NEW: full port (decision below) |
| `pageparser::item` | `parser/pageparser/item.go`, `itemtype_string.go` | ported (all functions) |
| `pageparser::pagelexer` | `parser/pageparser/pagelexer.go` | ported (all functions) |
| `pageparser::pagelexer_intro` | `parser/pageparser/pagelexer_intro.go` | ported (all functions, JSON and org front matter included) |
| `pageparser::pagelexer_shortcode` | `parser/pageparser/pagelexer_shortcode.go` | ported (all functions) |
| `pageparser::pageparser` | `parser/pageparser/pageparser.go` | ported (all functions) |
| `frontmatter` | `parser/frontmatter.go`, `parser/lowercase_camel_json.go` | ported: JSON (go-json), YAML (go-yaml's port of yaml.v2 `Marshal`), TOML (`toml::marshaler`), XML (`mxj`), the three JSON marshallers |

Every GO PORTING CHECKLIST entry is `OK` except the ORG stubs (`parseORGDate`, `unmarshalORG`:
`STUB`). No `EX` entries remain.

## Dependencies

- nh-*: nh-common.
- Wave A: go-value, go-yaml, go-json, go-strconv, go-time, go-unicode, go-path, go-fmt (`%v` of
  the mxj encoder's values).
- crates.io: none. The `toml` crate is not used (see below).
- dev: `serde_json` (fixture reader), `flate2` with `rust_backend` (gunzip of fixtures; README
  rule 2), `go-fmt` (a `%v` check of the go-toml objects).

## TOML: a full port of go-toml v2.2.4, not the `toml` crate

The plan allowed either a type mapping over the `toml` crate or a port of go-toml's parser. The
port was chosen because only it gives Go-identical results on every input the oracle generates:

- **Values and types.** go-toml's scanner decides what a token is (`scanDateTime`'s
  digit/colon/dash heuristics, one space followed by a digit, `inf`/`nan` inside numbers,
  `0` followed by another digit becoming the integer `0` plus a syntax error on the next
  expression), keeps fractional seconds beyond 9 digits by truncating them, accepts second `60`
  (normalised by `time.Date`), gives `+nan`/`-nan` Go's positive `math.NaN()` bits, uses
  `time.FixedZone("", offset)` for non-zero offsets and `time.UTC` for `Z` *and* `±00:00`, and
  returns `toml.LocalDate/LocalTime/LocalDateTime` values with their own `String()` precision
  rules. A mapping over another parser would have to re-derive each of these.
- **Errors.** go-toml's texts (`toml: expected character =`, `toml: key a is already defined`,
  the tracker's swapped arguments in `toml: key table already exists as a a,  but should be an
  array table`), the `DecodeError` position and the human-readable context block come from its
  own code paths. Several are plain `fmt.Errorf` errors without a position (the seen-key tracker,
  type mismatches), the others `*DecodeError`s located by pointer arithmetic on sub-slices.
- **Ordering.** go-toml parses one top-level expression at a time and decodes it before parsing
  the next one, so a semantic error (duplicate key) earlier in a document wins over a syntax error
  later in it. The port keeps the lazy `NextExpression` loop.
- The `toml` crate is also not in the offline registry.

The oracle checks 29,435 documents in the repository fixture (value + Go type, or error text,
position and human context) and 314,767 in the larger out-of-repo run, with 0 differences.

## Deliberate deviations

1. **Error positions and Go's zero-capacity slices.** go-toml locates errors with
   `danger.SubsliceOffset`. Go gives an empty tail slice `b[len(b):]` of a full slice
   (`len == cap`) the base pointer of `b`, not its end, so for a document whose buffer has no
   spare capacity an error at the very end (e.g. `expected = after a key, but the document ends
   there`) is reported at the start of the last token. Hugo's TOML input always has spare
   capacity (`afero.ReadFile` grows a `bytes.Buffer` by `bytes.MinRead`; front matter is a
   sub-slice of the page followed by its closing delimiter), and the port reproduces that case:
   the oracle passes documents with spare capacity. Only `UnmarshalStringTo` (HUGO_* map
   overrides) and `transform.Unmarshal` of a string, whose `[]byte(s)` may have `len == cap`, can
   see the other position, and only in an error message.
2. **go-toml's reflection** is replaced by a small value tree (`toml/unmarshaler.rs`): each Go
   function receives the place Go's `reflect.Value` points at and whether that value is of
   `Interface` kind. Go's returned replacement values are stored in place; every Go path writes its
   replacement back to the same place and Go maps are references, so the result is the same. On
   error Go also leaves a partially filled map in the caller's variable; the port returns only the
   error (Hugo discards the map). Struct targets, `TextUnmarshaler`, strict mode and the
   unmarshaler interface are not ported (no neohugo caller). Go panics that the tracker makes
   unreachable (`unhandled part`) return an error with the panic text; the others
   (`date time should have a timezone`, `invalid base`) panic like Go.
3. **Error model.** Errors are `nh_common::Error` values (deviation 9 of nh-common). The message is
   Go's cause: `unmarshal failed: toml: …`, `failed to unmarshal YAML: …`, `unmarshal failed:
   <encoding/json error>`. For TOML the `FilePos` is the `DecodeError` position with the file name
   `_stream.toml`; for the others nh-common's line-number extraction. Go's `UpdateContent`
   (source excerpt, offset → line) is not ported. `metadecoders::toml::Error` keeps go-toml's
   error bytes, position and human text exactly.
4. **ORG** decoding returns `neohugo-rs: … is not supported` (seeksnack has none; HUGO_LAYER.md
   §1 rule 5). The lexer handles org front matter fully; decoding it fails.
   The encoders take go_value values; values Go would reach only through reflection (structs
   without `MarshalText`, funcs, channels, typed nil pointers) give a `neohugo-rs:` error. XML
   error texts are converted lossily into `nh_common::Error` (like YAML's).
   Go picks the invalid attribute an mxj error names, and the order of map keys yaml.v2's
   `keyList` gives for keys it cannot order consistently ("01", "1.5", "0x1F"), from Go's random
   map order; the port starts from byte order. The oracle records every result Go gave in 200
   runs (`alts`, 6 of 3,698 encode cases); the port's must be one of them.
5. **Nil maps and nil data.** Go's `UnmarshalToMap(nil)` (no front matter) differs from an empty
   non-nil slice (JSON: `unexpected end of JSON input`), and a `null` YAML/JSON document gives a
   nil map. `Decoder::unmarshal_to_map_opt`/`unmarshal_to_map_nilable` keep both distinctions;
   `unmarshal_to_map` takes non-nil data and returns an empty map for Go's nil map.
   `ContentFrontMatter` gained `front_matter_nil` (no other crate constructs it).
6. **Item errors.** `Item::err` holds the lossy UTF-8 text; the new `Item::err_bytes` holds Go's
   bytes (a `%s` of the input can be invalid UTF-8). `Item::val_str` is lossy; parity code uses
   `val`/`val_bytes`. `ignoreEscapesAndEmit` does not store its `isString` argument in the item,
   as in Go. `Iterator::current` before the first `next_item` panics (Go indexes `items[-1]`).
7. `stringify_map_keys` is the identity on `go_value::Value` (maps always have string keys):
   go-yaml's `metadecoders::to_value` applies Go's `stringifyMapKeys` with `cast.ToStringE` while
   converting yaml.v2 values, and the oracle checks the result through the decoders.
8. `UnmarshalFileToMap` takes a `read_file` closure instead of an `afero.Fs`.
9. `Format` is an enum; Go's `Format` is a string, so an unknown format name (`Format("bogus")`)
   cannot be represented (`Format::Unknown` is Go's `""`).
10. `TomlLocal` (the skeleton's enum) now wraps `LocalDate`/`LocalTime`/`LocalDateTime` structs
    with Go's `int` fields as `i64`; no other crate used it. The objects implement `AsTime(loc)`
    with an `nh_common::htime::LocationRef` argument (T01's `AsTimeProvider` request), `String`,
    `MarshalText`, the struct fields and `%v`. Like other struct objects they compare by identity
    in `compare.Eq` (nh-common deviation 11).

## Known gaps

- ORG decoding (stub, above).

## Verification

Go oracles (package `main` in the neohugo module) write gzip-compressed typed JSON (shared
encoder `tools/go-oracle/nh-parser/tval`, an extension of T01's `goval` with the go-toml local
types) to `tests/fixtures/<topic>/`. `cargo test` needs neither Go nor the network. The seeksnack
site is private, so the inputs substitute for it.

| topic | inputs | Rust test | checks |
|---|---|---|---|
| `pageparser/pages.json.gz` | 5,540 pages: every `.md/.markdown/.html/.gotmpl` file under `docs/content`, `hugolib/testsite`, `create/skeletons` (980); the 291 string literals of `parser/pageparser/*_test.go`; the 218 seeksnack YAML front matters kept in go-yaml's fixture, wrapped in `---` with a shortcode body; 51 hand-written front matter shapes (YAML/TOML/JSON/org, BOM, CRLF, unterminated, every value type); 4,000 seeded token soups of shortcode syntax (nested, inline, self-closing, unclosed, comments, quoted/raw/escaped params, `{{% %}}` vs `{{< >}}`, summary dividers, unicode, invalid UTF-8) | `tests/pageparser.rs` | 141,869 items × (type, low, high, first byte, isString, segments, error, `Pos`, `ValTyped`, `ToString`, `Val`) for 3 configs (default, `NoFrontMatter`, `NoSummaryDivider`), `IsProbablySourceOfItems`, `LineNumber` of every item, `Consume`/`IsValueNext`, `HasShortcode`, and `ParseFrontMatterAndContent` (format, content offset, 4,555 decoded front matters with Go types, error texts): 211,919 checks |
| `metadecoders/decode.json.gz` | 29,435 documents: every `.toml/.yaml/.yml/.json` file of the repository (`docs/hugo.toml`, `docs/data/**`, test data: 46); every string literal of go-toml's tests (the toml-test suite, `unmarshaler_test.go`, `errors_test.go`, …) and its fuzz corpus; 221 hand-written TOML documents (ints at the int64 bounds, all bases, floats, inf/nan, offset/local dates and times, fractional seconds, leap seconds, nested tables, arrays of tables, dotted keys, inline tables, multiline strings, escapes, invalid documents), YAML and JSON documents; every prefix of the small valid documents (20,094), seeded byte substitutions, concatenations, 3,000 TOML token soups | `tests/metadecoders.rs` | `UnmarshalToMap` and `Unmarshal` (value + Go type or error cause) for every document; for TOML also `toml.Unmarshal` (value, or `Error()`, `DecodeError.Position()` and `String()`): 109,474 checks |
| `formats/csv.json.gz` | 552 CSV documents (hand-written, the `Input`s of go1.27.1's `encoding/csv/reader_test.go`, 300 seeded token soups) × 16 decoder configurations (delimiters `,` `;` tab `é` `\|` and invalid ones, comments, lazy quotes, target types map/slice/bogus) | `tests/formats.rs` | 8,832 cases: `Unmarshal` and `UnmarshalToMap` (typed value or error): 17,664 checks |
| `formats/xml.json.gz` | 2,242 XML documents: hand-written, the string literals of go1.27.1's `encoding/xml` tests and mxj's tests and `.xml` files, prefixes and byte substitutions of the small ones, 400 seeded token soups | `tests/formats.rs` | `Unmarshal` and `UnmarshalToMap`: 4,484 checks |
| `formats/encode.json.gz` | 3,698 maps decoded from hand-written YAML/TOML/JSON, the repository's data files and 1,800 random documents (random keys and values serialized with yaml.v2 / JSON: tricky strings and keys, ints, uint64 max, floats incl. inf/NaN/5e-324, nil, nested maps and lists, invalid UTF-8), each with and without `transform.Remarshal`'s `applyMarshalTypes` | `tests/formats.rs` | `InterfaceToConfig` to YAML, TOML, XML and JSON: 14,792 checks (12,846 outputs) |
| `metadecoders/misc.json.gz` | format names, content strings × 2 delimiters, decoder options, empty data × formats × target types, `UnmarshalStringTo` 24 strings × 10 target types | `tests/metadecoders.rs` | 332 |

Go's own tests are ported in `tests/go_tables.rs` (`decoder_test.go`, `format_test.go`,
`frontmatter_test.go`, `lowercase_camel_json_test.go`, go-toml's `localtime_test.go`);
`tests/toml_objects.rs` checks the go-toml objects through `nh_common::htime` (`AsTime`), fields,
methods and `%v`. The pageparser test inputs are the lexer's own test literals.

Results: 0 mismatches. A larger run outside the repository (`-soups 300000 -seed 7` for both
oracles: 314,767 documents / 1,249,028 checks and 301,540 pages / 6,258,791 items / 10,048,503
checks, read through `NH_PARSER_DECODE` / `NH_PARSER_PAGES`) also gave 0 mismatches.

Nothing here is platform dependent (no float arithmetic beyond `strconv` parsing; go-toml and
the lexer are integer code). The checked-in fixtures come from linux/arm64 builds run under
`qemu-aarch64-static`, and linux/amd64 builds produce the same bytes:

```sh
export GOTOOLCHAIN=go1.27.1
GOARCH=arm64 CGO_ENABLED=0 go build -o /tmp/pageparser-arm64 ./tools/go-oracle/nh-parser/pageparser
GOARCH=arm64 CGO_ENABLED=0 go build -o /tmp/metadecoders-arm64 ./tools/go-oracle/nh-parser/metadecoders
qemu-aarch64-static /tmp/pageparser-arm64 -root . -out crates/nh-parser/tests/fixtures/pageparser
qemu-aarch64-static /tmp/metadecoders-arm64 -root . -gomodcache "$(go env GOMODCACHE)" \
  -out crates/nh-parser/tests/fixtures/metadecoders
# or natively: go run ./tools/go-oracle/nh-parser/{pageparser,metadecoders} -root . -out …
# larger corpora outside the repo:
go run ./tools/go-oracle/nh-parser/metadecoders -root . -soups 300000 -seed 7 -out /tmp/md
NH_PARSER_DECODE=/tmp/md/decode.json.gz cargo test --release --test metadecoders
```

The fixtures regenerate byte for byte (gzip at best compression without name or time).

The `formats` fixtures come from `tools/go-oracle/nh-parser/formats`, run as linux/arm64 under
qemu (`applyMarshalTypes`' `int64(2^63)` saturates on arm64, the golden platform, and gives
`MinInt64` on amd64: the amd64 run differs in the encode cases of 9223372036854775807). The
fixtures regenerate byte for byte:

```sh
GOARCH=arm64 CGO_ENABLED=0 go build -o /tmp/formats-arm64 ./tools/go-oracle/nh-parser/formats
qemu-aarch64-static /tmp/formats-arm64 -goroot "$(go env GOROOT)" -out crates/nh-parser/tests/fixtures/formats
```
