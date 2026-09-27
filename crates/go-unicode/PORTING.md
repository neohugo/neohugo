# go-unicode — porting notes

Port of Go's `unicode`, `unicode/utf8`, `unicode/utf16`, the Unicode-aware
parts of `strings` and `bytes`, and `strings.Replacer`, from **go1.27.1**
(`unicode.Version` = **17.0.0**). No floating point anywhere, so no FMA sites.

## Go file → Rust module map

| Go source (`$GOROOT/src/...`) | Rust |
|---|---|
| `unicode/tables.go` (all RangeTables, `Categories`, `CategoryAliases`, `Scripts`, `Properties`, `FoldCategory`, `FoldScript`, `CaseRanges`, `properties`, `asciiFold`, `caseOrbit`) | `src/tables.rs` — **generated** by `tools/go-oracle/go-unicode tables`, re-exported at the crate root |
| `unicode/casetables.go` (`TurkishCase`, `AzeriCase`) | `src/tables.rs` (generated) |
| `unicode/letter.go` (`RangeTable`, `Range16/32`, `CaseRange`, `SpecialCase`, `is16`, `is32`, `Is`, `isExcludingLatin`, `IsUpper/Lower/Title`, `lookupCaseRange`, `convertCase`, `to`, `To`, `ToUpper/Lower/Title`, `SpecialCase.To*`, `SimpleFold`) | `src/lib.rs` |
| `unicode/graphic.go` (`GraphicRanges`, `PrintRanges`, `IsGraphic`, `IsPrint`, `IsOneOf`, `In`, `IsControl`, `IsLetter`, `IsMark`, `IsNumber`, `IsPunct`, `IsSpace`, `IsSymbol`) | `src/lib.rs` |
| `unicode/digit.go` (`IsDigit`) | `src/lib.rs` |
| `unicode/utf8/utf8.go` (all) | `src/utf8.rs` |
| `runtime/utf8.go` `decoderune` (`for i, r := range s`), `[]rune(s)`, `string(runes)`, `string(r)` | `src/utf8.rs` (`runes`, `to_runes`, `from_runes`, `rune_to_string`) |
| `unicode/utf16/utf16.go` (all) | `src/utf16.rs` |
| `strings/strings.go` (all functions), `internal/stringslite` (`Index`, `HasPrefix`, `Cut*`...), `strings/compare.go` | `src/strings.rs` |
| `bytes/bytes.go` (all text functions) | `src/bytes.rs` |
| `strings/replace.go` (`NewReplacer`, generic trie / single string / byte / byte-string replacers) | `src/replacer.rs` |

Generator/oracle: `tools/go-oracle/go-unicode/` (`package main` in the neohugo
module, gofmt/vet clean):

- `genexports` → `exports_gen.go` (map of every exported `*unicode.RangeTable`).
- `tables` → parses `tables.go`/`casetables.go` with `go/parser`, **verifies
  every parsed table, map, alias, CaseRange and TurkishCase against the compiled
  `unicode` package** (and the unexported `properties`/`asciiFold`/`caseOrbit`
  through the exported predicates / `SimpleFold`), then writes `src/tables.rs`.
- `fixtures` → `tests/fixtures/{unicode_hashes.txt,unicode_meta.txt,utf8_hashes.txt,strings_vectors.bin.gz}`;
  with `-rich` it uses the rich fuzz generator (`fuzz.go`: random code points
  from all planes, every case-mapping/fold-orbit rune, spaces and separators,
  random bytes, truncated/overlong/surrogate/>MaxRune encodings) and with
  `-corpus DIR` it adds every line of the text files under DIR plus a
  byte-mutated copy of each line (real seeksnack text).
- `gotests` → `tests/fixtures/go_test_tables.txt` (Go's own `_test.go` tables).
- `adversarial` → `tests/fixtures/adversarial_hashes.txt` (`adversarial.go`,
  see Verification).

Regenerate everything (from the repo root):

```sh
go run ./tools/go-oracle/go-unicode genexports
go run ./tools/go-oracle/go-unicode tables
go run ./tools/go-oracle/go-unicode fixtures
go run ./tools/go-oracle/go-unicode fixtures -only vectors -rich -n 900 -seed 20260928 -vec strings_vectors_rich.bin.gz
go run ./tools/go-oracle/go-unicode gotests
go run ./tools/go-oracle/go-unicode adversarial
```

All outputs are deterministic (re-running reproduces the checked-in files
byte for byte).

## Public API (for downstream crates)

Types: `Rune = i32` (Go `rune`), `RangeTable { r16: &'static [Range16], r32: &'static [Range32], latin_offset: usize }`,
`Range16 {lo, hi, stride}`, `Range32 {lo, hi, stride}`, `CaseRange {lo, hi, delta: [Rune; 3]}`,
`SpecialCase<'a>(pub &'a [CaseRange])`, `FoldPair`.

Crate root (`unicode`):

- constants `MAX_RUNE`, `REPLACEMENT_CHAR`, `MAX_ASCII`, `MAX_LATIN1`,
  `UPPER_CASE`/`LOWER_CASE`/`TITLE_CASE`/`MAX_CASE` (`i64`, Go `int`), `UPPER_LOWER`, `VERSION`.
- every exported table as `pub static NAME: &RangeTable` with the Go name
  upper-cased: `LU`, `LL`, `LT`, `LM`, `LO`, `L`/`LETTER`, `M`/`MARK`, `MN`,
  `MC`, `ME`, `N`/`NUMBER`, `ND`/`DIGIT`, `NL`, `NO`, `P`/`PUNCT`, `PC`…`PS`,
  `S`/`SYMBOL`, `SC`…`SO`, `Z`/`SPACE`, `ZS`, `ZL`, `ZP`, `C`/`OTHER`, `CC`…`CS`,
  `LC`, `UPPER`, `LOWER`, `TITLE`, all scripts (`HAN`, `THAI`, `LATIN`,
  `HIRAGANA`, `KATAKANA`, `HANGUL`, `SIGNWRITING`, …), all properties
  (`WHITE_SPACE`, `OTHER_ID_START`, `OTHER_ID_CONTINUE`, `PATTERN_WHITE_SPACE`,
  `STERM`, …).
- maps as sorted slices: `CATEGORIES`, `SCRIPTS`, `PROPERTIES`,
  `FOLD_CATEGORY`, `FOLD_SCRIPT` (`&[(&str, &RangeTable)]`),
  `CATEGORY_ALIASES` (`&[(&str, &str)]`); `lookup(map, name) -> Option<_>` is Go's `m[name]`.
- `CASE_RANGES`, `TURKISH_CASE`, `AZERI_CASE`, `GRAPHIC_RANGES`, `PRINT_RANGES`.
- functions `is`, `r#in` (Go `In`), `is_one_of`, `is_letter`, `is_digit`,
  `is_number`, `is_mark`, `is_space`, `is_punct`, `is_symbol`, `is_print`,
  `is_graphic`, `is_control`, `is_upper`, `is_lower`, `is_title`, `to`,
  `to_upper`, `to_lower`, `to_title`, `simple_fold`,
  `SpecialCase::{to_upper,to_lower,to_title}`.

`utf8`: `RUNE_ERROR`, `RUNE_SELF`, `MAX_RUNE`, `UTF_MAX`, `decode_rune`,
`decode_rune_in_string`, `decode_last_rune`, `decode_last_rune_in_string`
→ `(Rune, usize)`; `encode_rune(&mut [u8], r) -> usize`;
`append_rune(&mut Vec<u8>, r)`; `rune_len -> isize` (-1 invalid);
`rune_count`, `rune_count_in_string -> usize`; `full_rune`, `full_rune_in_string`,
`valid`, `valid_string`, `valid_rune`, `rune_start`; Go language conversions
`runes(s)` (iterator of `(byte_index, Rune)` = `for i, r := range s`),
`to_runes` (`[]rune(s)`), `from_runes` (`string(rs)`), `rune_to_string` (`string(r)`).

`utf16`: `is_surrogate`, `decode_rune`, `encode_rune`, `rune_len`, `encode`,
`append_rune`, `decode`.

`strings` (Go `string` = `&[u8]`): `count`, `contains`, `contains_any`,
`contains_rune`, `contains_func`, `index`, `last_index`, `index_byte`,
`last_index_byte`, `index_rune`, `index_any`, `last_index_any`, `index_func`,
`last_index_func`, `split`, `split_n`, `split_after`, `split_after_n`,
`fields`, `fields_func`, `join`, `has_prefix`, `has_suffix`, `map`, `repeat`,
`to_upper`, `to_lower`, `to_title`, `to_upper_special`, `to_lower_special`,
`to_title_special`, `to_valid_utf8`, `title`, `trim`, `trim_left`,
`trim_right`, `trim_func`, `trim_left_func`, `trim_right_func`, `trim_space`,
`trim_prefix`, `trim_suffix`, `replace`, `replace_all`, `equal_fold`, `cut`,
`cut_prefix`, `cut_suffix`, `cut_last`, `compare`; `&str` conveniences
`to_lower_str`, `to_upper_str`, `to_title_str`, `title_str`, `map_str`,
`trim_space_str`, `trim_func_str`, `trim_left_func_str`, `trim_right_func_str`,
`fields_str`, `fields_func_str`, `equal_fold_str`.
Index results are `isize` (-1 = not found); sub-string results borrow the
input; `map`/`to_*`/`to_valid_utf8`/`replace`/`title` return
`Cow<[u8]>`, `Borrowed` exactly when Go returns its input unchanged.

`bytes`: the same set with `bytes` semantics (always-copying results as
`Vec<u8>`), plus `equal`, `runes`, `clone`.

`replacer::Replacer::new(&[old, new, ...])`, `.replace(s) -> Cow<[u8]>`,
`.write_string(&mut Vec<u8>, s) -> usize`; test support mirroring Go's
`export_test.go` (`#[doc(hidden)]`): `.algorithm()` (Go type name of the
picked algorithm) and `.print_trie()` (Go `PrintTrie`).

Predicates/mappings take `impl FnMut(Rune) -> bool` / `impl FnMut(Rune) -> Rune`,
so `go_unicode::is_space` etc. can be passed directly.

## Deliberate deviations (none change output bytes)

1. **Types.** `rune` → `i32`. Go `int` results that can be negative (indexes,
   `RuneLen`) → `isize`; sizes and counts → `usize`; the `_case` argument of
   `To` and the case constants → `i64`; `n`/`count` arguments → `isize`.
2. **RangeTable slices are `&'static`** so tables are `static` items like Go
   package variables; a table built at run time must be leaked (none are in
   the dependency tree).
3. **Maps** are sorted slices + `lookup` (Go map iteration order is random
   anyway; sorted is what any output-affecting caller would do).
4. **Names.** Exported tables are the Go names upper-cased (`SignWriting` →
   `SIGNWRITING`, `STerm` → `STERM`). The unexported `properties` table is
   `LATIN1_PROPERTIES` (it would collide with the `Properties` map) and the
   `pp` bit is `P_PRINT` (it would collide with `pP`). `unicode.In` is `r#in`
   (`in` is a keyword). Private tables are `TAB_<NAME>` / `FOLD_<NAME>`.
5. **Go `string` vs `[]byte` entry points** of utf8 (`DecodeRune` /
   `DecodeRuneInString`, ...) both take `&[u8]` and share one implementation
   (the Go pairs are identical algorithms). `AppendRune` extends a
   `&mut Vec<u8>` instead of returning a new slice.
6. **Pure optimisations not ported:** the word-at-a-time ASCII skip in
   `utf8.Valid`; the last-byte scan with brute-force/Rabin-Karp fallbacks in
   `IndexRune`/`Index`/`LastIndex`/`Count` (replaced by `memchr`/`memmem`,
   which return the same first/last occurrence); `strings.Repeat`'s chunked
   doubling; Boyer-Moore `stringFinder` in the single-string Replacer
   (memmem, same first occurrence); `byteStringReplacer`'s Count-based size
   pre-computation (kept, only affects capacity).
7. **nil vs empty.** `bytes` functions that return `nil` in Go return an empty
   slice/`Vec`.
8. **Replacer** is built eagerly in `new` (Go builds lazily with `sync.Once`);
   the trie uses arena indices instead of pointers; `WriteString` appends to a
   `Vec<u8>` (no I/O errors).
9. **Panics kept from Go** (programmer errors, never input-dependent in Go
   code that checks first): `repeat` with negative count / overflow (and,
   like Go's `makeslice: len out of range`, a result too large to allocate
   panics via `try_reserve_exact` instead of aborting),
   `Replacer::new` with an odd argument count, `encode_rune` into a too-short
   buffer, division by a zero stride in a malformed user RangeTable.
10. **Arithmetic:** `convertCase` uses wrapping add/sub (Go int32 wraps; only
    reachable with a malformed user `SpecialCase`).

## Crates used

- `memchr` (allowed list) — byte / substring search.
- dev-only `flate2` (decompression of the checked-in gzip fixture only).

## Verification (all against go1.27.1)

- `tests/unicode_exhaustive.rs`: for **every code point 0..=0x10FFFF plus 10
  invalid runes** (negative, > MaxRune): all 13 `Is*` predicates, `In`/`IsOneOf`
  over `GraphicRanges`/`PrintRanges`/a custom list, `Is` for **every table in
  `Categories` (38), `Scripts` (174), `Properties` (39), `FoldCategory` (6),
  `FoldScript` (3) and all 262 exported variables**, `ToUpper/ToLower/ToTitle`,
  `SimpleFold`, `To` with cases -1..3, TurkishCase/AzeriCase mappings,
  utf8 `RuneLen`/`ValidRune`/`EncodeRune`/`AppendRune`/`string(rune)`, utf16
  `RuneLen`/`EncodeRune`/`AppendRune`/`Encode`/`IsSurrogate`, and a utf16
  `DecodeRune` window: **563 hash lines, 0 mismatches** (~600M evaluations).
  Plus metadata: Version, all 42 `CategoryAliases`, map sizes, CaseRanges length.
- `tests/utf8_exhaustive.rs`: DecodeRune, DecodeLastRune, FullRune, Valid,
  RuneCount (and their `InString` twins) and `for range` decoding on **all
  1- and 2-byte sequences, all 8.4M 3-byte sequences with a non-ASCII lead,
  5.3M 4-byte and 6.7M 5-byte sequences over boundary byte classes: 0 mismatches**.
- `tests/strings_vectors.rs`: 175,078 checked-in vectors (1,521 random byte
  strings built from ASCII, Unicode spaces, case-mapping edge cases, multi-byte
  runes, U+FFFD and invalid/truncated/overlong/surrogate bytes) over ~120
  `strings`/`bytes`/`utf8`/`utf16`/Replacer functions: **0 mismatches**; also
  run on two extra 4.6M-vector corpora (seeds 1 and 777, in
  `$SCRATCH/work/go-unicode/big{1,2}.bin.gz`, `GO_UNICODE_VECTORS=...`):
  **9.2M vectors, 0 mismatches**. The test also checks that `Cow` results
  borrow only when the output equals the input.
- `tests/go_test_tables.rs`: Go's own test tables (letter/digit/script,
  utf8, utf16, strings and bytes `_test.go`), 74 tables (1,267 rows) extracted by the
  oracle's `gotests` mode and replayed with the Go tests' assertions:
  all pass. 5 table rows are not extracted (`RepeatTests` rows built with
  `make`/long package-level strings; ported by hand in `go_inline_tests.rs`).

- `tests/adversarial.rs` (independent adversarial verification, fixture from
  the oracle's `adversarial` mode, 816 hash lines, **0 mismatches**):
  - `struct/*` (527 lines): the exact structure (every R16/R32 entry and
    LatinOffset) of every table in Categories/Scripts/Properties/FoldCategory/
    FoldScript and every exported table, GraphicRanges/PrintRanges, and every
    CaseRanges/TurkishCase/AzeriCase entry (not only membership).
  - `orbit/all`: the SimpleFold orbit walked from every code point.
  - `<domain>/<func>` (5 domains × 54 functions = 270 lines, ~100M calls):
    strings/bytes ToUpper/ToLower/ToTitle/Title/To*Special(Turkish)/TrimSpace/
    Fields/FieldsFunc/EqualFold (vs ToUpper, ToLower, ToTitle, Map(SimpleFold),
    ToUpperSpecial)/ToValidUTF8/Map (identity and a drop/grow/fold mapping)/
    TrimFunc/Trim/TrimLeft/TrimRight/IndexRune(RuneError)/IndexFunc/
    LastIndexFunc/Split("")/SplitN("",2)/IndexAny/LastIndexAny/RuneCount/
    []rune/bytes.Runes over: every code point `string(r)` (+ invalid runes);
    code points in `" "+r+"a"+r+NBSP`, `U+2000 r \xff r _ r U+3000` and
    truncated-encoding contexts; every byte string of length ≤ 2, 53³ length-3
    and 20⁴ length-4 byte-class strings.
  - `foldpair/*` (10 lines): EqualFold (both argument orders, bytes, with
    ASCII/invalid context, length mismatch) of every code point against its
    whole fold orbit, its To*/Turkish mappings, r±1, r^0x20, r^1, K, ſ, U+FFFD;
    and all 133K ordered pairs of 365 short valid/invalid strings.
  - `long/*` (8 lines): 300 long haystacks (up to 6000 pieces from 2-5 piece
    alphabets with many partial matches) driving Go's IndexByte/IndexRune
    cutovers, Rabin-Karp, Boyer-Moore and the four Replacer algorithms:
    IndexRune/Index/LastIndex/Count/IndexAny/LastIndexAny/Replace/Split/
    NewReplacer/case mapping/EqualFold/TrimSpace/Fields/ToValidUTF8/Map.
  - plus a check that the `&str` conveniences equal the byte functions.
- `tests/strings_vectors.rs` also replays the checked-in `-rich` corpus
  (`strings_vectors_rich.bin.gz`, 106,010 vectors): **0 mismatches**. Extra
  corpora in `$SCRATCH/work/go-unicode/verify/`: `rich_big.bin.gz` (2,880,894
  vectors, seed 4242) and `corpus_big.bin.gz` (4,607,198 vectors from 20,000
  seeksnack text lines + mutated copies): **0 mismatches**.
- `tests/go_inline_tests.rs`: Go tests whose data is inline in the test
  function, ported by hand: TestReplacer (78 cases, Replace and WriteString),
  TestPickAlgorithm, TestGenericTrieBuilding, TestIndexRune (strings and
  bytes), TestMap, toValidUTF8Tests, TestSpecialCase, TestCutLast,
  compareTests, TestCaseConsistency, TestTurkishCase, TestLetterOptimizations,
  TestDigitOptimization, TestLatinOffset, TestSpecialCaseNoMapping,
  TestNegativeRune, TestSimpleFold(-42), the 5 RepeatTests rows the extractor
  skips and TestRepeatCatchesOverflow: all pass.

Mutation checks (temporarily corrupting a fixture hash, or the
`lastIndexFunc` ASCII fast path; and, on a scratch copy of the crate, making
`_` a Title separator, dropping Map's RuneError re-encoding, or not resetting
ToValidUTF8's run state) make the tests fail as expected.

## Known gaps

- Not ported: `strings`/`bytes` iterator functions (`Lines`, `SplitSeq`,
  `SplitAfterSeq`, `FieldsSeq`, `FieldsFuncSeq`; unused by neohugo, goldmark
  and tdewolff), `strings.Builder`, `strings.Reader`, `bytes.Buffer`,
  `bytes.Reader` (use `Vec<u8>` / slices), `strings.Clone`,
  `bytes.MinRead`-style I/O helpers.
- `unicode.RangeTable` values constructed at run time need `'static` slices.
