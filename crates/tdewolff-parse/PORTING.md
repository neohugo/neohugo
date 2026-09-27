# tdewolff-parse — porting notes

Byte-exact port of `github.com/tdewolff/parse/v2@v2.8.1` (the version pinned
in neohugo's `go.mod`), **without** the `js` package (ported separately in
`crates/tdewolff-parse-js`, which depends on this crate) and without
`binary.go`/`binary_unix.go` (not used by minify or neohugo).

Golden toolchain: go1.27.1 darwin/arm64.

## Go file → Rust module map

| Go (`parse/v2@v2.8.1/…`) | Rust |
|---|---|
| `input.go` | `src/input.rs` (`Input`, `GoReader`, `IoReader`, `read_all` = `io.ReadAll`) |
| `util.go` | `src/util.rs` (`copy`, `to_lower`, `equal_fold`, `printable`, `is_whitespace`, `is_newline`, `is_all_whitespace`, `trim_whitespace`, `Indenter`) |
| `common.go` | `src/common.rs` (`number`, `dimension`, `mediatype`, `data_uri`, `quote_entity`, `replace_multiple_whitespace`, `replace_entities`, `replace_multiple_whitespace_and_entities`, `URL_ENCODING_TABLE`, `DATA_URI_ENCODING_TABLE`, `encode_url`, `decode_url`, `append_escape`) |
| `position.go` | `src/position.rs` (`position`, `position_input`) |
| `error.go` | `src/error.rs` (`Error` = `*parse.Error`, `new_error`, `new_error_lexer`; `GoError` models Go `error` values) |
| `buffer/buffer.go` | `src/buffer/mod.rs` (`DEFAULT_BUF_SIZE`, `MIN_BUF`) |
| `buffer/lexer.go` | `src/buffer/lexer.rs` (`buffer::Lexer`) |
| `buffer/reader.go` | `src/buffer/reader.rs` (`buffer::Reader`, implements `GoReader` with `Bytes()`) |
| `buffer/writer.go` | `src/buffer/writer.rs` (`buffer::Writer`) |
| `buffer/streamlexer.go` | `src/buffer/streamlexer.rs` (`buffer::StreamLexer`, private `bufferPool`) |
| `strconv/float.go` | `src/strconv/float.rs` (`parse_float`, `append_float`) |
| `strconv/int.go` | `src/strconv/int.rs` (`parse_int`, `parse_uint`, `append_int`, `len_int`, `len_uint`) |
| `strconv/decimal.go` | `src/strconv/decimal.rs` (`parse_decimal`, `append_decimal`) |
| `strconv/number.go` | `src/strconv/number.rs` (`parse_number`, `append_number`) |
| `html/lex.go` | `src/html/lex.rs` (`html::Lexer`, `TokenType`, template delimiters) |
| `html/util.go` | `src/html/util.rs` (`escape_attr_val`) |
| `html/hash.go` | `src/html/hash.rs` (generated tables ported literally) |
| `css/lex.go` | `src/css/lex.rs` (`css::Lexer`, `TokenType`) |
| `css/parse.go` | `src/css/parse.rs` (`css::Parser`, `GrammarType`, `Token`, `State`) |
| `css/util.go` | `src/css/util.rs` (`is_ident`, `is_url_unquoted`, `hsl2rgb`) |
| `css/hash.go` | `src/css/hash.rs` (generated tables ported literally) |
| `xml/lex.go` | `src/xml/lex.rs` (`xml::Lexer`, `TokenType`) |
| `xml/util.go` | `src/xml/util.rs` (`escape_attr_val`, `escape_cdata_val`) |
| `json/parse.go` | `src/json/parse.rs` (`json::Parser`, `GrammarType`, `State`) |

Go standard library pieces ported locally (exact go1.27.1 semantics):

| Go | Rust | why |
|---|---|---|
| `runtime.growslice`/`roundupsize` for `[]byte` | `src/gobytes.rs` (`grow_cap`) | `append` capacity decides whether later appends write in place (aliasing) |
| `encoding/base64` `StdEncoding.Decode`/`DecodedLen` | `src/base64.rs` | `DataURI`: skips `\r\n`, non-strict trailing bits, exact `CorruptInputError` offsets |
| `math.Pow10` | `src/gomath.rs` | product of two table entries, not always correctly rounded |
| `io.ReadAll` (go1.27.1 chunked version) | `src/input.rs` (`read_all`) | `NewInput` for readers without `Bytes()`; result capacity decides NUL-in-place vs copy |
| `unicode/utf8` (DecodeRune, RuneLen, AppendRune, `[]rune(s)`, `string(rs)`) and `unicode.IsGraphic` | `src/utf8.rs` | thin facade over the shared `go-unicode` crate |

## Public API for downstream crates (minify, parse-js)

### `GoBytes` — Go `[]byte`

All byte slices handed out by this crate are `GoBytes`: a `(array, off, len,
cap)` header over a shared `Rc<[Cell<u8>]>` backing array (or `nil`, or a
read-only `&'static [u8]` for Go package-level `[]byte("…")` globals). Every
alias sees every write, exactly like Go. Translation guide:

| Go | Rust |
|---|---|
| `nil` / `[]byte{}` | `GoBytes::nil()` / `GoBytes::empty()` |
| `var x = []byte("…")` (global) | `GoBytes::from_static(b"…")` (writing into it panics) |
| `make([]byte, n, c)` | `GoBytes::make(n, c)` |
| `append([]byte(nil), s...)` | `GoBytes::nil().append(s)` |
| `len(b)`, `cap(b)`, `b == nil` | `b.len()`, `b.cap()`, `b.is_nil()` |
| `b[i]`, `b[i] = c` | `b.at(i)`, `b.set(i, c)` (bounds-checked against `len` like Go) |
| `b[i:j]`, `b[i:]`, `b[:j]`, `b[i:j:k]` | `b.slice(i, j)`, `b.slice_from(i)`, `b.slice_to(j)`, `b.slice3(i, j, k)` |
| `append(b, x...)` (x a Rust slice) | `b = b.append(x)` |
| `append(b, y...)` (y a Go slice, may alias b) | `b = b.append_bytes(&y)` |
| `append(b, c)` | `b = b.append_byte(c)` |
| `copy(dst, src)` | `dst.copy_from(&src)` (memmove) / `dst.copy_from_slice(&[u8])` |
| `string(b)`, `w.Write(b)` | `b.to_vec()`, `b.write_to(&mut out)` |
| `bytes.Equal/HasPrefix/HasSuffix/IndexByte/LastIndexByte/Index/Contains/Compare` | `==` / `has_prefix` / `has_suffix` / `index_byte` / `last_index_byte` / `index` / `contains` / `compare` |

`append` uses go1.27.1's `growslice` capacities (size classes), so "append in
place vs. reallocate" matches Go. `GoBytes` is `!Send` (one minify call = one
thread).

Pure scanning functions take `&B where B: ByteView + ?Sized` so they accept
`[u8]`, `Vec<u8>`, `str` or `GoBytes` without copying (`number`, `dimension`,
`equal_fold`, `is_all_whitespace`, `quote_entity`, `strconv::parse_*`,
`html::to_hash`, `css::to_hash`). `SubView` is `b[off:]` as a `ByteView`.

### `Input` — `*parse.Input`

`Input` is a cloneable *handle* (`Rc`): cloning shares position and buffer,
like Go code sharing one `*parse.Input` between a lexer and its caller (e.g.
minify's `TokenBuffer` calls `z.Offset()` while the lexer advances it).
Constructors: `Input::new(Option<&mut dyn GoReader>)` (uses `bytes()` when the
reader has one, else reads all), `Input::new_bytes(GoBytes)` (writes the NUL
sentinel into spare capacity and restores it on `restore()`, else copies),
`Input::new_string(&[u8])`. Methods: `peek(usize)`, `peek_at(isize)` (Go
`Peek` with a negative offset, as the js lexer's `l.r.Peek(-1)`), `peek_rune`,
`move_(isize)` (Go `Move`; `move` is a keyword), `move_rune`, `pos`, `rewind`,
`lexeme`, `skip`, `shift`, `offset`, `bytes`, `len`, `reset`, `err`,
`peek_err`, `has_err` (= `Err() != nil` without cloning), `restore`.
`read_all` is go1.27.1's `io.ReadAll` (chunked reads, one right-sized copy at
the end), so the capacity of the buffer `Input::new` gets from a reader
without `Bytes()` is Go's too.

Callers that hand a Go `*bytes.Buffer`/`*buffer.Reader` to `NewInput` must
pass a `GoReader` whose `bytes()` returns a `GoBytes` sharing the caller's
backing array: `NewInputBytes` then writes the NUL sentinel into the spare
capacity of *that* array (e.g. into the enclosing HTML buffer when minify
passes `buffer.NewReader(t.Data)` of a sub-slice) until `restore()`.

### Errors

`GoError` models Go `error` values: `Eof` (`io.EOF`), `Parse(Box<Error>)`
(`*parse.Error`), `BadDataUri`, `Base64CorruptInput(n)`, `Other(msg)`.
`GoError::error_bytes()` is Go's `err.Error()` byte for byte. `Option<GoError>`
is a nilable Go `error`; `is_eof(&opt)` tests for `io.EOF`. `Error` fields are
`message`, `line`, `column`, `context`; `new_error(r, offset, msg)`,
`new_error_lexer(&Input, msg)`. Go's `fmt.Sprintf(message, a...)` is done by
the caller (message is already formatted bytes).

### Lexers / parsers

`html::Lexer::{new, new_template}`, `css::Lexer::new`,
`css::Parser::new(input, is_inline)`, `xml::Lexer::new`, `json::Parser::new`;
`next()` returns `(TokenType, GoBytes)` (css parser: `(GrammarType,
TokenType, GoBytes)`, json: `(GrammarType, GoBytes)`). Accessors as in Go
(`text`, `attr_key`, `attr_val`, `has_template`, `err`, `values`, `offset`,
`state`, `has_parse_error`, `input()`). Token/grammar types are Rust enums
with Go's names and discriminants (`html::StartTagToken`, `css::DeclarationGrammar`,
…, re-exported at module level); `token_type_string(n)` etc. give Go's
`String()` for any integer. Hashes are `html::Hash(u32)`/`css::Hash(u32)`
with Go-named constants (`html::Script`, `css::Font_Face`) and `to_hash`.

Entity replacement takes `&dyn EntityMap` (Go `map[string][]byte`) and
`&dyn RevEntityMap` (Go `map[byte][]byte`); both are implemented for
`HashMap`/`BTreeMap`; `NilMap` is a nil map. Minify's generated
`EntitiesMap`/`TextRevEntitiesMap` tables implement these traits.

## Deliberate deviations (API shape only; bytes are identical)

- `[]byte` → `GoBytes`; `*parse.Input` → `Input` handle; lexers are plain
  structs owning an `Input` clone (`&mut self` methods).
- `io.Reader` → `GoReader` trait (`read` returns `(n, Option<GoError>)`,
  optional `bytes()` for `interface{ Bytes() []byte }`); `IoReader` adapts
  `std::io::Read`.
- `buffer::StreamLexer` borrows its reader (`&mut dyn GoReader`).
  `shift_len`/`free` are `isize` because Go's `ShiftLen` becomes negative
  after a buffer swap.
- `Indenter<W: std::io::Write>` is generic; Go's "wrap an Indenter again adds
  up the indentation" is `Indenter::nested`. Its `std::io::Write::write`
  returns `b.len()`; the Go count (including indentation) is `write_go`.
- `Mediatype` params: `Option<BTreeMap<Vec<u8>, Vec<u8>>>` (nil map = `None`).
- `Printable` returns bytes; `Hash.String()` returns `&'static [u8]`.
- `css::Parser::values()` returns `&[Token]` (Go returns the internal buffer
  that the next grammar overwrites); `values_mut()` for callers that write
  through it.
- `[]byte(string)` conversions: `GoBytes::from_slice` uses `cap == len`; Go
  may round capacity up (or use a 32-byte stack buffer). Only observable if
  code appends to such a slice while another alias exists; not the case in
  parse. `string_to_bytes_cap(n)` gives Go's heap capacity if needed.
- Where Go would panic (index out of range), Rust panics too; no Go code path
  in parse panics on the tested inputs.

## FMA sites (darwin/arm64, verified with `go tool objdump`)

All fused multiply-adds in the non-js parse packages of the oracle binary
(`go tool objdump -s tdewolff/parse/v2 oracle | grep -E 'FMADD|FMSUB|FNMADD|FNMSUB'`
→ 10 instructions), and the same in the minify harness binary (HSL2RGB is
called, not inlined, by `minify/css`):

| Go site | instruction | Rust |
|---|---|---|
| `strconv/decimal.go:86` `f += 0.5` after `f *= Pow10(dec)` | FMADDD | `f.mul_add(p, 0.5)` (comparison uses rounded `f*p`) |
| `strconv/decimal.go:88` `f -= 0.5` | FNMSUBD | `f.mul_add(p, -0.5)` |
| `strconv/float.go:105` `exp10 -= 1.0` after `exp10 := float64(exp2)*log2` | FNMSUBD | `(exp2 as f64).mul_add(LOG2, -1.0)` |
| `css/util.go:26` `m2 = l + s - l*s` | FMSUBD | `(-l).mul_add(s, l + s)` |
| `css/util.go:40` `m1 + (m2-m1)*h*6.0` (×3, inlined hue2rgb) | FMADDD | `((m2 - m1) * h).mul_add(6.0, m1)` |
| `css/util.go:44` `m1 + (m2-m1)*(2.0/3.0-h)*6.0` (×3) | FMADDD | `((m2 - m1) * (2.0 / 3.0 - h)).mul_add(6.0, m1)` |

Not fused: `css/util.go:28` `l*2 - m2` (compiled as `l+l`), all of
`ParseFloat`/`ParseDecimal`/`Pow10` (only MUL/DIV). Checked that the
fixtures catch a missing fusion: unfusing AppendDecimal fails 32 checked-in
(438 of the 30× set) records, unfusing HSL2RGB fails 116 (1728). The
`float64exp` fusion is not observable (the truncated result never changes)
but is ported anyway.

## Tests

- Oracle: `tools/go-oracle/tdewolff-parse` (package main in the neohugo
  module; `gen.sh` regenerates everything, `LARGE=1` also the big sets).
  Modes: `fixtures DIR [SCALE]`, `corpus KIND ROOT OUT EXT...`,
  `extract HTMLROOT OUTDIR`, `dump KIND FILE`,
  `fuzz OUTDIR N SEED EXHAUST MAXWIN ROOT...` (stream digests of generated
  inputs), `fnfuzz DIR N SEED` (randomized root/strconv/misc records),
  `corpusnums DIR ROOT...` (Parse* records for every number in a corpus).
- `tests/streams.rs`: full token/grammar streams (offset, type, data, text,
  attr value, template flag, per-grammar values, final error string, and the
  input buffer after lexing — which checks in-place mutation) for every string
  literal of upstream's `_test.go` tables plus deterministic fuzz splices:
  html (+ Go-template lexer), css lexer, css parser (stylesheet and inline),
  xml, json.
- `tests/fuzz.rs`: the same stream serialization, FNV-1a digested, for
  `fuzz` sets. Groups/kinds: html (`html`, `htmltmpl` = `{{ }}`, `htmlphp` =
  `<? ?>`), css (`csslex`, `css`, `cssinline`), `xml`, `json`. Inputs:
  exhaustive enumeration of all short strings over per-group special-character
  alphabets after ~15–30 interesting prefixes (`<script><!--`, `<svg a="`,
  `<![CDATA[`, `url(`, `u+`, a backslash, `a{--x:`, `<!DOCTYPE x [`, `{"a":`, …),
  random alphabet strings, random bytes, fragment splices and mutated windows
  of the seeksnack corpora. `fuzz_checked_in` runs the small checked-in set
  (tests/fixtures/fuzz, ~7.6k inputs × kinds); `fuzz_digests` (ignored) runs
  `TDEWOLFF_PARSE_FUZZ=<dir>`. Failing inputs are dumped to files; compare
  with `cargo run --example dump -- KIND FILE` vs the oracle's `dump`.
- `tests/root.rs`, `tests/strconv.rs`, `tests/misc.rs`: every root/strconv
  function on literals + fuzz (outputs *and* the input buffer after the call,
  for in-place functions), Position/NewError strings, Printable/IsGraphic
  over all graphic-range boundaries, encoding tables, `math.Pow10` −400…400,
  HSL2RGB bits, escaping helpers with a shared buffer, perfect hashes,
  `String()` names, `growslice` capacities, `io.ReadAll` capacities; plus the
  upstream tables ported literally (strconv, css util) and hand-ported API
  tests (Input incl. `peek_at`, buffer Lexer/Reader/Writer/StreamLexer/
  bufferPool, html/css/json offsets, errors, in-place lowercasing). Each
  `*_fixtures` test has a `*_fnfuzz` twin over randomized records
  (tests/fixtures/fnfuzz, or `TDEWOLFF_PARSE_FNFUZZ=<dir>`): entity soups
  with every HTML entity name (+ upper-cased/truncated), numeric/hex
  references incl. overflow, data URIs with (in)valid base64/`\r\n`/padding,
  mediatypes, percent-encoding, whitespace runs, number strings with long
  digit runs/extreme exponents/hundreds of leading zeros/group symbols,
  random/subnormal/near-power-of-ten floats, attribute values for the
  escapers, CSS identifiers with escapes.
- `tests/corpus.rs` (`#[ignore]`, needs the corpora): FNV-1a digests of the
  stream serialization for golden/nominify and golden/canonical html+xml+json,
  corpus2 css/css-resource/svg/json (`.in` and `.out`), CSS/JSON extracted
  from the HTML, and the pristine site sources (bootstrap css/scss, 2048 svgs,
  305 json, 157 Go-template html lexed with and without template delimiters).
- Checked-in fixtures: the scale-1 set (~5 MB, 75k records) + fuzz (0.5 MB) +
  fnfuzz (0.85 MB); `gen.sh` also writes a 30× set to
  `$SCRATCH/work/tdewolff-parse/fixtures-full` (536k records;
  `TDEWOLFF_PARSE_FIXTURES=… cargo test --release`).

Results of the original port (2026-09-27): all tests (+ the ignored corpus
test) pass in debug and release on both fixture sets; corpus: 18050/18050
files produce identical streams (golden html 3427+3427, golden xml 1481+1481,
corpus2 json 3438, embedded JSON-LD 1718, pristine svg/xml 2050, json 305,
css/scss 160 (+35 inline), html 157+157, …), also in debug mode
(integer-overflow checks on).

### Independent adversarial verification (2026-09-27)

Second agent; side-by-side review of every ported Go function against
`parse/v2@v2.8.1` and go1.27.1 (`io`, `encoding/base64`, `math`, `runtime`
growslice), plus new oracle modes and differential runs:

| run | inputs / records | result |
|---|---|---|
| `fuzz` seed 11 (N=100k, default exhaustive) | 3,843,916 streams | identical (release **and** debug/overflow-checked) |
| `fuzz` seed 12 (N=300k, windows ≤3000 B) | 5,443,916 streams | identical |
| `fuzz` all-256-byte suffixes of length ≤2 after every prefix | 11,974,326 streams | identical |
| `fnfuzz` seeds 21/33/34 (N=50k/200k/60k) | 3.3M + 16.1M + 5.1M records | identical (also debug) |
| `corpusnums` (every number in golden + pristine + corpus2 + embedded) | 223,335 numbers × 7 parsers | identical |
| corpus digests | 18,050 files | identical |
| 30× fixture set (regenerated) | 536k records | identical |
| checked-in sets | 75k + 7.6k×kinds + 13k records | identical |

Mutation check: injecting two one-line bugs (html `--!>` comment end, css
`\f` newline) made the small fuzz set fail 495 streams, so the digests do
catch real divergences. FMA sites re-verified with `go tool objdump` of the
current oracle (same 10 instructions as below).

Changes made by the verifier:

- `read_all` (`io.ReadAll`) re-ported from go1.27.1: the previous port
  followed the old append-growth implementation, so the capacity of the
  buffer `Input::new`/`buffer::Lexer::new` build from a reader without
  `Bytes()` differed from Go (e.g. 5000 bytes: Go cap 5376). Not observable
  through the lexers (the Input buffer is private and lexemes are
  cap-limited), but now exact; covered by `readall` records.
- Added `Input::peek_at(isize)` (Go `Peek` with negative offsets; the js
  lexer uses `Peek(-1)`/`Peek(-2)`, which `peek(usize)` cannot express).
- New checked-in regression sets (fuzz/, fnfuzz/), `parsenumber3` records
  (multi-byte and U+FFFD group/decimal symbols), `examples/dump.rs`.

No output-affecting discrepancy was found in the lexers, parsers, strconv,
common, util, position/error, escaping, hash or base64 code.

## Known gaps

- Every consumer must go through `GoBytes`; there is no zero-copy `&[u8]`
  view of a `GoBytes` (interior mutability), so read-only helpers take
  `ByteView` or copy with `to_vec()`.

- `binary.go` / `binary_unix.go` (BinaryReader/Writer, mmap readers) are not
  ported (unused by minify/neohugo).
- `js` package: separate crate.
- `html::Lexer::new_template` takes `[&str; 2]` (UTF-8) where Go takes
  `[2]string` (any bytes); Hugo uses no template delimiters (minify always
  calls `NewTemplateLexer` with `{"", ""}`, equivalent to `new`).
- `buffer::StreamLexer::peek` takes `usize`; Go's `Peek(-n)` (unused) would
  go through `read` with a negative position.
