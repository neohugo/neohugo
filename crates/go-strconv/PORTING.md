# go-strconv — porting notes

Faithful port of Go's `strconv` as shipped in **go1.27.1** (the golden
toolchain). In 1.27 the numeric code lives in `src/internal/strconv`; the
public package `src/strconv` wraps it, adds `NumError`, and owns quoting
and `IsPrint`. The float algorithms are Go's current ones: "unrounded
scaling" (research.swtch.com/fp, `uscale.go`) for shortest and fixed
formatting and for parsing, with the multiprecision `decimal` fallback.
There is no Ryū or Eisel–Lemire code left in go1.27.1, so none is ported.
No Rust float formatting or parsing is used anywhere.

## Go file → Rust module map

| Go (go1.27.1 `src/…`) | Rust | Notes |
|---|---|---|
| `internal/strconv/uscale.go` | `src/internal/uscale.rs` | `unrounded`, `prescale`, `uscale`, `shortFloat[F]` (a `FloatParams` value selects F), `fixedWidthFloat`, `parseFloat64/32`, `pack64/32`, `setDigits`, `numDigits`, `log10Pow2`, `log2Pow10`, `skewed` |
| `internal/strconv/pow10tab.go` | `src/internal/pow10tab.rs` | Generated from the Go file by `tools/go-oracle/go-strconv/gentables.py` (696 entries, 1e-348..1e347) |
| `internal/strconv/ftoa.go` | `src/internal/ftoa.rs` | `FormatFloat`, `AppendFloat`, `ftoa32`/`ftoa64` (one body, see deviations), `fmtEFG`, `fmtB`, `fmtX` |
| `internal/strconv/atof.go` | `src/internal/atof.rs` | `special`, `readFloat`, `decimal.set`, `decimal.floatBits`, `atof64exact`/`atof32exact`, `atofHex`, `atof32`/`atof64`, `ParseFloat`, `parseFloatPrefix`, `optimize` |
| `internal/strconv/decimal.go` | `src/internal/decimal.rs` | `decimal` (800-digit buffer), `Assign`, `Shift`, `leftShift`/`rightShift`, `leftcheats`, `Round*`, `RoundedInteger`, `String` (as `Display`) |
| `internal/strconv/atoi.go` | `src/internal/atoi.rs` (+ `Error` in `internal/mod.rs`) | `ParseUint`, `ParseInt`, `Atoi`, `underscoreOK`, `Error` |
| `internal/strconv/itoa.go` | `src/internal/itoa.rs` | `FormatUint/Int`, `Itoa`, `AppendInt/Uint`, `formatBits`, `small`, `formatBase10`, `RuntimeFormatBase10` |
| `internal/strconv/atob.go` | `src/internal/atob.rs` | |
| `internal/strconv/atoc.go` | `src/internal/atoc.rs` | complex128 is `(f64, f64)` |
| `internal/strconv/ctoa.go` | `src/internal/ctoa.rs` | |
| `internal/strconv/deps.go` | `src/internal/deps.rs` | bits helpers, `inf`, `nan`, plus the `float32(x)` / `float64(x)` conversions (`f64_to_f32`, `f32_to_f64`) |
| `internal/strconv/export_test.go` | `internal::set_optimize`, `internal::export_test`, `Decimal::new` | test hooks |
| `strconv/number.go` | `src/lib.rs` | public wrappers, `NumError`, `toError`, `Error` (ErrRange/ErrSyntax/base/bitSize errors) |
| `strconv/quote.go` | `src/quote.rs` | all Quote*/AppendQuote*, `CanBackquote`, `UnquoteChar`, `Unquote`, `QuotedPrefix`, `IsPrint`, `IsGraphic`, `bsearch` |
| `strconv/isprint.go` | `src/isprint.rs` | Generated from the go1.27.1 file (Unicode 17.0.0): 412+132+566+120 entries + 16 `isGraphic` |
| `strconv/bytealg.go` | `index` in `src/quote.rs` | |
| `unicode/utf8` (subset) | `src/utf8.rs` (private) | `DecodeRuneInString`, `AppendRune`, `ValidRune`, `ValidString` with Go's rules. It is private so this crate stays a leaf; downstream crates should use `go-unicode` |

## Public API (for go-fmt, go-json, tdewolff-*, gotemplate)

Go `int` → `i64`, Go `rune` → `go_strconv::Rune` (`i32`, so invalid code
points work as they do in Go), complex128 → `(f64, f64)`. Parse and quote
inputs are `impl AsRef<[u8]>`, because Go strings are bytes. Formatting and
quoting return `String`: formatter output is always ASCII and quoter output
is always valid UTF-8. `unquote` returns `Vec<u8>`. `append_*(dst: &mut Vec<u8>, …)`
matches Go's `AppendX(dst, …) []byte`.

Crate root (package `strconv`):

- `format_float(f, fmt: u8, prec: i64, bit_size: i64) -> String`, `append_float(&mut Vec<u8>, …)`
- `parse_float(s, bit_size) -> Result<f64, NumError>`
- `parse_int(s, base, bit_size) -> Result<i64, NumError>`, `parse_uint(…) -> Result<u64, NumError>`, `atoi(s) -> Result<i64, NumError>`
- `format_int(i64, base)`, `format_uint(u64, base)`, `itoa(i64)`, `append_int`, `append_uint`
- `parse_bool`, `format_bool`, `append_bool`, `parse_complex`, `format_complex`
- `quote`, `quote_to_ascii`, `quote_to_graphic`, `quote_rune`, `quote_rune_to_ascii`, `quote_rune_to_graphic`, and each `append_*` variant
- `unquote(s) -> Result<Vec<u8>, Error>`, `unquote_char(s: &[u8], quote: u8) -> Result<(Rune, bool, &[u8]), Error>`, `quoted_prefix(&[u8]) -> Result<&[u8], Error>`, `can_backquote`
- `is_print(Rune)`, `is_graphic(Rune)`, `INT_SIZE`
- `NumError { func: &'static str, num: Vec<u8>, err: Error }`. Its `Display` is Go's `(*NumError).Error()` byte for byte, e.g. `strconv.ParseInt: parsing "x": invalid syntax`, and `source()` stands in for `Unwrap`.
- `Error::{Range, Syntax, Base(i64), BitSize(i64)}`. `Display` gives Go's messages (`value out of range`, `invalid syntax`, `invalid base N`, `invalid bit size N`).

`go_strconv::internal` (package `internal/strconv`) keeps Go's `(value, error)`
shape as `(T, Option<internal::Error>)`. Use it when Go callers read the value
that comes back with an error: `±Inf` with ErrRange from ParseFloat, the
clamped value from ParseInt/ParseUint, or `parse_float_prefix` (consumed
length). It also exports `ftoa32`/`ftoa64` (format an `f32` directly),
`f64_to_f32`/`f32_to_f64` (Go's arm64 conversions, including NaN payloads),
and `Decimal`.

Panics, as in Go: `format_float`/`append_float` with bitSize ∉ {32, 64},
`format_int`/`format_uint`/`append_*` with base ∉ [2, 36] (base 10 is always
valid), and `format_complex` with bitSize ∉ {64, 128}. These are programmer
errors in Go too. No input string can cause a panic. One extra panic (see
deviation 9): `format_float`/`format_complex` with a non-ASCII `fmt` byte.

## Deliberate deviations (none change output bytes)

1. `ftoa32`/`ftoa64` are a single function `ftoa(dst, bits, &FloatInfoF, …)`.
   Go keeps two identical copies of one generic body only because of a
   compiler issue (go.dev/issue/79547). `shortFloat[F]` takes a `FloatParams`
   value in place of a type parameter.
2. Go's `optimize` package variable (SetOptimize test hook) is a
   **thread-local** `Cell<bool>`, so tests that switch to the slow path
   cannot race with other test threads. Production code never changes it.
3. `atofHex`'s unused `s` parameter was dropped. `flt == &float32info` is
   `std::ptr::eq`.
4. Go `goto overflow` / `goto out` in `decimal.floatBits` became a labelled
   block with a flag.
5. `float32(x)` and `float64(x32)` go through explicit helpers
   (`deps::f64_to_f32`/`f32_to_f64`). For numbers they are Rust `as`, which
   is IEEE round-to-nearest-even, the same as FCVT. For NaN they reproduce
   darwin/arm64 FCVT exactly: sign kept, payload truncated or extended, quiet
   bit set. Rust leaves NaN payloads of `as` casts unspecified. This matters
   because `ParseFloat("nan", 32)` returns bits `0x7FF8000000000000`, while
   bitSize 64 returns `0x7FF8000000000001`. The `nanconv` oracle test checks
   this against the Go runtime.
6. `utf8.ValidString`'s word-at-a-time ASCII skip is left out. It is a
   speed-only path and does not change results.
7. `formatBits`'s string/append modes are one append-to-`Vec` routine.
8. `NumError.num` is an owned `Vec<u8>`, the equivalent of Go's
   `stringslite.Clone`.
9. `format_float` / `format_complex` return `String`. For a finite value and
   a `fmt` byte >= 0x80, Go returns `"%" + fmt` (e.g. `"%\xff"`), which is
   not valid UTF-8, so these two functions panic with an explicit message
   instead. `append_float` / `append_complex` return Go's exact bytes
   (tested by `non_ascii_fmt_byte` and the `fmtbytes` family, which covers
   all 256 fmt bytes). Go's own float verbs never pass such a byte (fmt's
   `%v`/`%g`/... use `b e E f g G x X` only), so no parity path hits this.

## FMA sites

None. `go tool objdump` of the oracle binary (built with go1.27.1,
darwin/arm64) over every `internal/strconv.*` and `strconv.*` symbol shows no
`FMADD`/`FMSUB`/`FNMADD`/`FNMSUB`. The only float instructions are the
`FMUL`/`FDIV` in `atof64exact`/`atof32exact` (atof.go:455-497, inlined into
`atof32`/`atof64`). The Rust code uses plain `*`/`/` there.

## Known Go quirks that are reproduced on purpose

- The slow path (`decimal.set`) caps the digit count at 800, so an integer
  literal with more than 800 digits and no `.` gets the wrong decimal point
  on the slow path (`999…9e-800` with 801 nines → `1`). Go only reaches the
  slow path when the fast path is ambiguous, or with `optimize=false`. The
  `atof_vectors` fixture checks the Rust slow path against Go's slow path.
  The oracle gets Go's slow path by linking to `internal/strconv.optimize`.
- `special()`'s `+`/`-` case falls through into the `inf` case, so `+nan`
  is a syntax error.

## Tests and verification (all numbers are from `cargo test`)

- `tests/go_tables.rs` (44 tests) ports the Go test tables and tests from
  `ftoa_test`, `atof_test` (fast and slow paths), `atoi_test`, `itoa_test`,
  `decimal_test`, `fp_test` (Paxson `testfp.txt`, and `atof1k.txt` /
  `ftoa1k.txt` fast vs slow, copied to `tests/fixtures/go-testdata/`),
  `math_test`, `atob_test`, `atoc_test`, `ctoa_test`, `quote_test`,
  `strconv_test` and `number_test`. `TestRoundTrip32` covers 2.15M float32
  values, and the random fast==slow tests use 100K values.
- `tests/oracle.rs` checks Go-produced fixtures (`tools/go-oracle/go-strconv`):
  - `hash_chunks`: 582 chunk hashes (FNV-1a over every result) of
    deterministic SplitMix64-driven tests. The generators are ported line by
    line in `tests/common/mod.rs`. Coverage:
    - `ftoa64` / `ftoa32`: 1,048,576 float64 and 1,048,576 float32 values.
      Each value goes through all 8 formats `b e E f g G x X`, with prec -1
      plus two random precs per format (22 FormatFloat calls), and 8
      ParseFloat round trips (bitSize 64 and 32) of its `g`/`e`/`f`/`x`
      outputs.
    - `ftoa64as32`: float64 values formatted with bitSize 32.
    - `ftoagrid64/32`: a dense grid of 8 formats × 56 precisions (-1…40, 50…1100).
    - `atofgen`: 1,048,576 generated literal strings (decimal, hex,
      underscores, inf/nan spellings, junk, 700–1100-digit mantissas). Each is
      parsed at bitSize 64 and 32 (value bits plus the full NumError text),
      plus 262,144 ParseComplex inputs.
    - `atoigen`: 524,288 integer strings × 10 (base, bitSize) combinations
      for ParseInt and ParseUint, plus Atoi, ParseBool, and
      FormatInt/FormatUint/Itoa in random bases.
    - `quoteall`: every code point 0…0x10FFFF plus invalid runes through
      QuoteRune*, IsPrint, IsGraphic, Quote*, CanBackquote, Unquote,
      QuotedPrefix, UnquoteChar, and round trips.
    - `quotegen`: 524,288 random byte strings (invalid UTF-8, escapes,
      quotes) through the same quoting functions.
    - `nanconv`: float32↔float64 NaN payload and rounding conversions.
    - `ftoaslow64/32` and `atofslow`: 163,840 values/strings run with
      `optimize=false` (bignum formatting and decimal parsing), compared
      with Go's slow path.
  - `ftoa_vectors` (17,875 lines, fast and slow), `atof_vectors` (11,834
    inputs, including exact float64/float32 midpoints ± perturbations,
    800-digit boundaries and hex boundary cases), `atoi_vectors` (2,500),
    `quote_vectors` (2,500), `numerror` (33): readable explicit vectors.
  - `hash_chunks_extended` (`#[ignore]`) takes `GO_STRCONV_HASHES=<file>`
    produced by `oracle -mode hashes -scale 16`. It covers 16.7M float64,
    16.7M float32, 16.7M parse strings, 8.4M integer strings and 8.4M
    quote strings. It passed: 5,202/5,202 chunks match (about 3 minutes with
    `--release`; re-run by the verification pass: 5,202/5,202 again).
- Mutation check: a one-character change in `append_escaped_rune`
  (`r < 0x10000` → `r < 0x1000`) made many `quoteall` chunks fail, and
  `GO_STRCONV_DUMP_DIR` + `oracle -mode dump` located the diverging rune
  (U+1000).

### Independent adversarial verification (`tests/adv.rs`)

A second, independently written oracle (`tools/go-oracle/go-strconv/adv.go`,
`oracle -mode advhashes`) drives 16 families of structured and exhaustive
inputs; `tests/adv.rs` mirrors it and checks `tests/fixtures/adv_hashes.txt`
(1,637 chunk hashes). Inputs are built without the code under test: exact
midpoints come from Go `math/big` and from a small bignum in the Rust test,
and "nearest float to a short decimal" comes from Go's and Rust std's
(correctly rounded) parsers.

| family | what | size |
|---|---|---|
| `f32all` | every float32 bit pattern: shortest `e`, ParseFloat(…, 32) round trip, `e` prec 8 and 5 | 2^32 values |
| `f64pow2` | every float64 exponent × {0,1,2,3,2^51,2^52−1,2^52−2} × ±: all 8 formats, `e`/`g` 0..20, `f` 9 precs, `x` 0..14, bitSize 32, 6 round trips | 28,672 values, ~2.3M calls |
| `ties` | exact decimal ties n·2^−k (k 1..64, odd n ≤ 2001) at every `e`/`f`/`g` prec, both bit sizes; (10j+5)·10^q integers | 64K + 64K values, ~13M calls |
| `nearshort` | floats nearest to random 1–17 digit decimals (10^−350..10^309) and ±1, ±2 ulp neighbours; float32 likewise | 2.6M values |
| `midpoints` | exact decimal midpoints below/above random hard floats (subnormal, min normal, max finite, powers of two …), plain / sci / +1e−k / −1e−k / truncated 17,19,20,25 digits / underscored / padded / signed / `E+`, parsed at 64 and 32 | ~49K midpoints, ~1.1M parses |
| `longties` | midpoints extended past the 800-digit decimal buffer (820 zeros + 1, 820 nines, 790 leading zeros …) | ~4K midpoints, ~48K parses |
| `hexmid` | hex midpoints: exact, trailing zero digits, sticky bit past 16 digits, just below, fractional point, underscores, upper case, `0x0.00…`, exponent ±5000 | 130K midpoints, 2.6M parses |
| `quote2` / `quote3` | every string of 0–3 bytes through Quote, QuoteToASCII, QuoteToGraphic, CanBackquote, Unquote and QuotedPrefix with 4 quote styles, UnquoteChar with 3 quotes | 16.8M strings |
| `quote4` | first byte 0..255 × 3 bytes from 16 UTF-8 boundary / special values | 1M strings |
| `unqesc` | `\U` over 0..0x10FFFF+, `\u` over 0..0xFFFF (3 quote styles), UnquoteChar, `\x`, octal 0..0o777, backslash + every byte | ~3.6M Unquote + 1.1M UnquoteChar |
| `wildrunes` | IsPrint/IsGraphic/QuoteRune* for runes with every high half (negative and > MaxRune) × 24 special low halves (Go truncates to uint16) | 1.6M runes |
| `atoiall` | every string of length ≤ 4 over `01789abfxXoO_+-z` through ParseInt/ParseUint (11 bases × 11 bit sizes incl. invalid), Atoi, ParseBool | 69,905 strings, 17M calls |
| `atoibound` | ±(2^k ± {0,1,2}) for k = bits−1, bits, every bit size, in bases 2/8/10/16/36 with/without prefix and underscores, at base b/0 and bitSize bits/64/0 | ~283K calls |
| `fmtbytes` | every fmt byte 0..255 through AppendFloat (and FormatFloat for ASCII) at 10 precs incl. −5, 400, 1100, both bit sizes | 164K calls |
| `complex` | FormatComplex over 8 formats × 4 precs × both sizes, ParseComplex round trip | 16K values |

Results (go1.27.1 oracle vs this crate): **1,637/1,637 chunks match**, both
with `--release` (`adv_hashes_full`, about 7 min on 12 idle cores) and in
the test profile with overflow checks on (18 min on a loaded machine), so
no input in these families makes an integer operation overflow either. The default `adv_hashes` runs all
families except `f32all` (11 of 1,024 chunks sampled) and `quote3` (16 of
256 chunks sampled). The only discrepancy found is deviation 9 above (a
panic where Go returns a non-UTF-8 string); no output-byte difference was
found anywhere.

Real-site corpus (`tests/corpus.rs`, `oracle -mode corpus`):
`corpus_numbers` runs the 5,648 distinct numeric tokens found in the
seeksnack sources and the golden Go output (plain integers of up to 6
digits are taken from the sources only; e.g. `+.05`, `116e97`, `0x1f`) through ParseFloat 64/32, FormatFloat `g`/`f`/`e` -1 and
`f` 2 of the result, `g` -1 at 32 bits, ParseInt base 10/0, Atoi and
ParseUint base 0: all match. `corpus_quote` (needs
`GO_STRCONV_SITE=<pristine-seeksnack>`) quotes every line of the 340 text
source files (1.0 MB: Quote/QuoteToASCII/QuoteToGraphic/CanBackquote,
Unquote round trip, Unquote/QuotedPrefix of the raw line): all 340 file
hashes match.

Mutation check of the adversarial suite: 9 of 10 single-line mutations were
caught by it (round-half-even → half-up in `Unrounded::round`; no `d+1`
truncation check in `atof64`; BOM rule in `CanBackquote`; trailing
underscore in `underscoreOK`; skewed branch in `shortFloat`; round-to-even in
`shouldRoundUp`; hex sticky bit; `fixedWidthFloat`'s `f` loop; `\U` validity).
The 10th (`>` → `>=` in `fmtX` rounding) is caught by `hash_chunks` and
`test_ftoa` instead.

`Cargo.toml` sets `[profile.test] opt-level = 3`, so a plain `cargo test`
runs the million-value checks in about 10–30 s. Overflow checks stay on,
so every run also checks that no integer operation overflows unexpectedly.

Regenerating fixtures (needs go1.27.1). The `strconvslow` tag enables the
`go:linkname` hook. Without the tag the oracle still builds, so
`go build ./...` keeps working, but it will not generate fixtures:

```
go build -tags strconvslow -ldflags=-checklinkname=0 -o "$TMPDIR/oracle" ./tools/go-oracle/go-strconv
"$TMPDIR/oracle" -mode hashes  -out crates/go-strconv/tests/fixtures/hashes.txt
"$TMPDIR/oracle" -mode vectors -dir crates/go-strconv/tests/fixtures
"$TMPDIR/oracle" -mode advhashes -out crates/go-strconv/tests/fixtures/adv_hashes.txt   # ~10 min (f32all)
"$TMPDIR/oracle" -mode advhashes -only midpoints,hexmid -out crates/go-strconv/tests/fixtures/adv_hashes.txt  # recompute some families
"$TMPDIR/oracle" -mode advdump -test midpoints -chunk 3 > go.txt   # compare with GO_STRCONV_DUMP_DIR output of tests/adv.rs
"$TMPDIR/oracle" -mode corpus -site <pristine-seeksnack> -golden <golden/nominify> -dir crates/go-strconv/tests/fixtures
python3 tools/go-oracle/go-strconv/gentables.py && (cd crates/go-strconv && cargo fmt)   # pow10tab.rs, isprint.rs
```

## Known gaps

- None in the functions listed. `strconv` is fully ported, including
  ParseComplex/FormatComplex and QuotedPrefix.
- Precision values near `i64::MAX` overflow: debug builds panic on the
  overflow, release builds wrap like Go. Either way Go and Rust would try to
  allocate an unbounded buffer, so these inputs are not supported.
- The full Paxson testbase files (~10 MB, fetched over the network by Go's
  `-testbase` flag) are not used. The 1k samples Go ships are used instead.
