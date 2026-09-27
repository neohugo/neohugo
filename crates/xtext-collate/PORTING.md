# xtext-collate — porting notes

Port of `golang.org/x/text@v0.26.0` (the version pinned in neohugo's
`go.mod`) collation runtime: `collate` + `internal/colltab`, with the CLDR 23 /
UCA 6.2.0 tables, plus the subsets of `unicode/norm` (Unicode 15.0.0 tables)
and `language` / `internal/language` / `internal/language/compact` /
`internal/tag` (CLDR 32 tables) that `collate.New(tag)` and the collation
iterators use. neohugo uses it through `langs.NewLanguage`
(`collate.New(language.Parse(lang))`, English on parse error) for template
`sort`, `Pages.ByTitle/ByLinkTitle`, `DefaultPageSort` tie-breaks and
`Taxonomy.Alphabetical` — all via `Collator.CompareString`.

No floating point anywhere: **no FMA sites**.

## Go file → Rust module map

| Go (x/text v0.26.0) | Rust |
|---|---|
| `collate/collate.go` (Collator, Buffer, Compare/CompareString, Key/KeyFromString, iter, compareLevel, keyFromElems, appendPrimary, processWeights) | `src/collate/mod.rs` |
| `collate/option.go` (newCollator, Option, setOptions, setFromTag, ldmlBool, option funcs, alternateHandling) | `src/collate/option.rs` (+ `new_collator` in `mod.rs`) |
| `collate/index.go` (getTable, tableIndex) | `src/collate/index.rs` |
| `collate/sort.go` (sorter, Lister, Sort, SortStrings) | `src/collate/sort.rs`, `Collator::sort/sort_strings` in `mod.rs` |
| `collate/tables.go` (generated: availableLocales, locales, varTop, main* tables) | `data/collate.bin` + `src/collate/tables.rs` |
| `internal/colltab/colltab.go` (MatchLang, parent) | `src/colltab/mod.rs` |
| `internal/colltab/collelem.go` | `src/colltab/collelem.rs` |
| `internal/colltab/contract.go` | `src/colltab/contract.rs` |
| `internal/colltab/iter.go` | `src/colltab/iter.rs` |
| `internal/colltab/numeric.go` | `src/colltab/numeric.rs` |
| `internal/colltab/table.go` | `src/colltab/table.rs` |
| `internal/colltab/trie.go` | `src/colltab/trie.rs` |
| `internal/colltab/weighter.go` | `src/colltab/weighter.rs` |
| `unicode/norm/forminfo.go` (Properties, compInfo, accessors), `trie.go` (sparseBlocks), `tables15.0.0.go` (nfc/nfkc tries, decomps, ccc; `lookup`/`lookupValue`), `normalize.go` (FirstBoundary/firstBoundary), `composition.go` (streamSafe.next/isMax, decomposeHangul), `input.go` (skipContinuationBytes) | `src/norm.rs` + `data/norm.bin` |
| `language/language.go` (CanonType, canonicalize, Canonicalize, Base, Raw, Parent, Extensions, TypeForKey, SetTypeForKey, makeTag), `language/parse.go` (Parse, Compose, update), `language/tags.go` (Make/MustParse) | `src/language/mod.rs` |
| `internal/language/language.go`, `lookup.go`, `parse.go`, `compose.go`, `match.go` (addTags, specializeRegion), `compact.go`, `tags.go`, `common.go`; `internal/tag/tag.go` | `src/language/internal.rs` |
| `internal/language/compact/compact.go`, `language.go` (FromTag, Make, Tag.Tag, Tag.Parent, LanguageID), `tables.go` (coreTags, specialTagsStr) | `src/language/compact.rs` |
| `internal/language/tables.go` (generated) | `data/language.bin` + `src/language/tables.rs` |
| `unicode` (`unicode.Is(unicode.Ideographic, r)`, `unicode.In(r, unicode.Nd)` — go1.27.1) and `unicode/utf8` (`DecodeRune`, `RuneStart`, `EncodeRune`) | `src/goutf8.rs` + `data/unicode.bin` |

Every ported function carries a `// Go: <path>:<Func>` comment.

## Data

`data/*.bin` are produced by `go run ./tools/go-oracle/xtext-collate gen`,
which parses the exact x/text source files resolved by the go command for
this module (`go/build` → module cache) and evaluates their literal tables
with `go/ast` + `go/constant` (the tables are unexported). `unicode.bin` is
dumped at run time from the golden toolchain's `unicode` package. Blob
format: `XTBLOB01` + sections (`name`, element width, count, LE data); read
once into `OnceLock` statics.

| blob | source | size |
|---|---|---|
| `collate.bin` | `collate/tables.go` (UnicodeVersion 6.2.0, CLDRVersion 23) | 1.25 MB |
| `norm.bin` | `unicode/norm/tables15.0.0.go` (`go1.21` build tag, Version 15.0.0) | 50 KB |
| `language.bin` | `internal/language/tables.go` (CLDR 32) + `compact/tables.go` | 34 KB |
| `unicode.bin` | go1.27.1 `unicode.Ideographic`, `unicode.Nd` (Unicode 17.0.0) | 1 KB |

## Public API (for downstream crates)

```rust
use xtext_collate::{Collator, Buffer, CollOption, LOOSE, NUMERIC, IGNORE_CASE,
                    IGNORE_DIACRITICS, IGNORE_WIDTH, FORCE, language};

// neohugo langs.NewLanguage: Parse(lang) ok -> New(tag), else New(English)
let mut c = Collator::for_hugo_language("th");
c.compare_string("a", "b");            // -1 / 0 / 1  (Go CompareString)
c.compare(b"a", b"b");                 // Go Compare on bytes (invalid UTF-8 ok)

Collator::new("de-u-co-phonebk");      // collate.New(language.Make(s))
Collator::with_options("en", &[LOOSE, NUMERIC]);
Collator::from_tag(&tag, &opts);       // collate.New(tag, opts...)
Collator::new_from_table(Box<dyn Weighter>, &opts);
Collator::match_index(&tag);           // colltab.MatchLang(tag, Supported())

let mut buf = Buffer::new();
let k: &[u8] = c.key(&mut buf, b"text");   // Key / key_from_string; buf.reset()
c.key_vec(b"text");                         // owned key
c.sort_strings(&mut v);                     // SortStrings (Go pdqsort via go-sort)
c.sort(&mut lister);                        // Sort(Lister)
xtext_collate::supported();                 // collate.Supported()
xtext_collate::options_from_tag(&tag);      // OptionsFromTag

language::parse(s) -> Result<Tag, LangError>      // language.Parse (Default canon)
language::make(s) -> Tag                          // language.Make
language::{RAW, ALL, DEFAULT, ...}.parse/make/canonicalize/compose
Tag::{string, raw, base, parent, extensions, type_for_key, set_type_for_key, compact_index}
language::english(), language::und()
xtext_collate::norm::Form::{Nfd,Nfkd,..}.properties(s) / first_boundary(s)   // x/text norm subset
```

`Collator` methods take `&mut self` (Go's collator is not concurrency-safe
either; neohugo wraps it in a mutex). `Collator: Send + Sync`.

## Deliberate deviations

1. **NFD comes from a port of x/text `unicode/norm`, not `unicode-normalization`.**
   colltab needs `NFD.Properties` (LeadCCC/TrailCCC/Size of possibly invalid
   or incomplete UTF-8), `NFD.FirstBoundary`, `NFKD.Properties(..).Decomposition()`
   and Hangul NFD. These are ported with x/text's own Unicode 15.0.0 tables.
   `unicode-normalization` 0.1.25 (Unicode 17.0.0) was checked against
   x/text for every code point (`tools/go-oracle/xtext-collate nfd-dump` vs a
   scratch Rust dumper): NFD/NFKD of **57 code points differ** (all characters
   with decompositions added in Unicode 16/17: U+A7F1, U+105C9, U+105E4,
   U+11383–U+113C8 (Tulu-Tigalari), U+16121–U+16128 (Gurung Khema),
   U+16D68–U+16D6A (Kirat Rai), U+1CCD6–U+1CCF9 (outlined letters/digits));
   it also has no API for the Properties/FirstBoundary semantics on invalid
   bytes. So the crate does not use it.
2. `unicode.Ideographic` / `unicode.Nd` are dumped from go1.27.1 into
   `data/unicode.bin` (the `go-unicode` crate had no source when this was
   written). Same data; can be switched to `go-unicode` later.
   `utf8.DecodeRune` is a local port (Go semantics: invalid → U+FFFD, width 1).
3. Go's `[]byte`/`string` method pairs (`AppendNext`/`AppendNextString`,
   `lookup`/`lookupString`, `matchContraction`/`matchContractionString`,
   `scan` copies, `Compare`/`CompareString`, `Key`/`KeyFromString`) are
   verbatim copies in Go and collapse to one `&[u8]` implementation.
4. `Weighter` omits `Start`, `StartString`, `Domain` (`colltab.Table`
   panics "not implemented" for them). `Weighter::append_next` models Go's
   "return a new slice" by mutating a `Vec` (see numeric.rs header for the
   aliasing model of `numberConverter`).
5. Go would loop forever / panic on a few table-invariant violations that
   cannot occur with the shipped tables (`appendNext` returning 0 bytes
   inside the NFKD / interstitial loops, `w[i]` with nothing appended); the
   port advances by one byte / skips instead.
6. `Collator.Sort` stores keys as owned vectors instead of slices into a
   shared `Buffer` (identical bytes); sorting uses `go-sort` (Go pdqsort /
   `sort.Stable`) so tie order matches. `Reorder` (a Go `panic("TODO")`) is
   not provided.
7. Language layer: Go's slice aliasing in the BCP 47 scanner is modelled
   exactly (`Scanner` in `src/language/internal.rs`): `b` is `scan.b`,
   `tail` the rest of its backing array up to `cap` (Go's capacities: the
   32-byte inline array / size-class-rounded `[]byte(s)` for Parse,
   `make([]byte, n)` for Builder.Make, the fixed `buf` for SetTypeForKey;
   `append` growth via go1.27.1 `nextslicecap` + `roundupsize`). Go never
   clears bytes beyond `len` when the buffer shrinks, and `scan.token` is a
   slice into the array, so after `deleteRange` (-u key de-duplication) or
   the variant de-duplication `resizeRange`, Go reads the *shifted* buffer
   through the stale token (e.g. `uk-u-co-zhuyin-kc-false-co-search-a-BCD`
   reads the singleton at the old offset) — the port reads the same bytes.
   `SetTypeForKey` keeps using its `b`, which aliases the scanner array
   (stale tail bytes included: `Make("pa-CN").SetTypeForKey("kn",
   "ab-kn-cd")` is `pa-CN-u-ab-cd-kn-cd` in Go and in the port). Go
   `Parse`/`Compose` recover runtime panics as `ErrSyntax`; the port sets
   `Scanner.panicked` at every Go slice/index bound it can violate (M49
   lookup past the last bucket, reslice beyond cap, `copy` into `s[p:]` with
   p > len, `toLower` past len) and Parse / Compose return `Und, ErrSyntax`.
   `sort.Sort` calls in the parser use `go-sort` (exact Go order).
   The `-u-rg-XXzzzz` special case of `compact.Make` (separate language and
   locale ids) is implemented (`compact::CTag`), including `Tag.Parent` and
   `CompactIndex` (`LanguageID`) on it.
8. Public `Tag` equality is Go `==` on `compact.Tag`: tags are normalised
   through `compact.FromTag` exactly like `makeTag`, including the Go quirk
   where a script id > 255 overflows the 8-bit `CompactCoreInfo` field
   (`language.Make("pa-Zzzz").String() == "pa-Arab"`).

## Verification (all against the real Go code, go1.27.1 + x/text v0.26.0)

Checked-in fixtures (`tests/fixtures`, 4.4 MB; the original set is
regenerated with `go run ./tools/go-oracle/xtext-collate fixtures -site
<files>`, the adversarial set with the `pairs`/`enum`/`settype` commands
shown below, `tags-fuzz.tsv` is a selection of `tagfuzz` output); random
corpora are regenerated in Rust with a bit-identical splitmix64 generator and
compared through SHA-256 digests of all keys and 3 comparisons per string.
`colltab.MatchLang` is called in the oracle through `go:linkname`.

* `tags.tsv`: 30,456 tags (every supported locale in 4 spellings, all 676
  2-letter and 17,576 3-letter codes, 95 locales × 39 regions / 11 scripts /
  collation keywords, grandfathered/legacy/private-use/extension tags, 8,000
  fuzzed tags): `language.Parse` error text + result, `Make`,
  `All.Canonicalize`, `Base()` + confidence, `Parent()`, MatchLang index and
  a key hash of 180 probe strings under `collate.New(Make(s))` — **all match**.
* Random corpus (3,000 strings) × 33 option/tag configs, random corpus (800)
  × all 95 supported locales, combining-mark stress corpus (300) × 128
  configs (33 + 95) — keys and comparisons match.
* Per-rune keys (every 7th code point) for und and th; norm properties
  (NFD Size/CCC/LeadCCC/TrailCCC/BoundaryBefore/After/Decomposition, NFKD
  Size/Decomposition, FirstBoundary) for all 1,112,064 scalars + 65,664
  1–2-byte invalid sequences + 50,000 random strings — match.
* Site strings (2,802 distinct: `i18n-lang-misc/coll/bytes-{en,th}.txt`,
  `content-model/colltest/strings.json`, every list/token of
  `templates-inventory/sortdiff.txt`) × {und, en, th, th+Loose,
  en+Numeric}: per-string key hash, `sort.SliceStable` order, digests — match.
  The six `i18n-lang-misc/coll/xtext-{und,en,th}-data{en,th}.txt` orders are
  reproduced byte-for-byte.
* Ported Go unit tests: TestColElem, TestImplicit, TestUpdateTertiary,
  TestLookupContraction, TestDoNorm, TestNumericAppendNext,
  TestNumericOverflow, TestLookupTrie, TestMatchLang (colltab);
  TestProcessWeights, TestKeyFromElems, TestNumeric, TestOptions,
  TestAlternateSortTypes, TestSort and the Example* outputs (collate).
  TestNonDigits / TestNumericCompare are covered against Go's actual
  results by `pairs-numeric.txt` (the Go tests filter by Unicode 6.2
  `rangetable.Assigned`, which is not ported).
* Adversarial near-equal comparison pairs (`pairs.go`; Hugo only calls
  `CompareString`, and random pairs are decided at the first primary):
  base strings from Thai syllable/word generators, title-like mixes, the
  random generator, site strings, combining-mark stress and long
  concatenations (up to ~3 KB, > 512 elements), mutated 1-3 times (case,
  inserted/removed/reordered marks, Thai tone/vowel/prevowel swaps, digit
  script and width variants, compatibility forms, ignorables and controls,
  NFC/NFD/NFKC/NFKD, contraction boundaries, invalid UTF-8 cuts,
  prefixes). `Compare(a,b)`, `Compare(b,a)` (Go uses CompareString for one
  direction) and all keys, under 43 configs (33 main + en/th+loose,numeric,
  en+numeric, th-TH, en-US, th level2/shifted/posix/kb/kc) + all 95
  locales: checked-in `pairs.txt` 1,500 pairs x 138 configs; numeric
  `pairs-numeric.txt` (TestNumericCompare over every go1.27.1 Nd range and
  block of ten, TestNonDigits over unicode.N, long/leading-zero/marked
  digit runs; every 29th of 131,513 pairs) x 138 configs.
* Exhaustive enumerations (`enum.go`): every string of length <= 2 over
  the Thai block + 7 neighbours (8,931) and over 38 Latin contraction
  letters/marks/ignorables (1,483) x 138 configs; <= 3 over 25
  starters/marks of mixed ccc (16,276) x 43 configs — keys and 3
  comparisons per string.
* Fuzzed language tags (`tagfuzz.go`, same checks as tags.tsv):
  `tags-fuzz.tsv` 3,618 lines = the regression inputs of the bugs below
  (`tagFuzzRegressions`: duplicate `-u` keys / duplicate variants followed
  by another extension, `-u-rg-XXzzzz`, `pa-Zzzz`) + the first 3,600 tags of
  seed 12 (`oracle tagfuzz -regressions -n 3600 -seed 12`);
  `SetTypeForKey` (`settype.tsv`, 3,261 lines of adversarial key/values,
  `oracle settype -n 400`).
* `tags.tsv` and `tags-fuzz.tsv` match the repository root `.gitignore`
  pattern `tags*` (ctags); `tests/fixtures/.gitignore` re-includes them
  (they were missing from the first commit, so `tags_match_go` /
  `fuzzed_tags_match_go` failed on a fresh checkout).
* Exact regeneration commands of the checked-in fixtures (run from the repo
  root; all reproduce byte-for-byte): `oracle fixtures -dir D -site SITE`
  (tags.tsv, digests.txt, site-strings.hex, site-keys.txt; SITE is a text
  file with the strings of `site-strings.hex` one per line — the original
  Hugo test-data inputs are not in this repository), `oracle pairs -n 1500
  -seed 7 -site tests/fixtures/site-strings.hex`, `oracle pairs -kind
  numeric -every 29`, `oracle enum -alpha thai|latin -maxlen 2 -locales`,
  `oracle enum -alpha marks -maxlen 3`, `oracle settype -n 400`, and
  `oracle gen` for `data/*.bin`.
* Scratch runs (`big_corpus` ignored test, corpora outside the repo):
  3,000,000 random strings × 33 configs; 300,000 random × 128 configs
  (33 + all 95 locales); 200,000 combining-mark stress strings × 128
  configs; keys of **every** code point (1,112,064) × 128 configs — all
  digests match (0 mismatches).
  Adversarial verification (second agent): 240,000 adversarial pairs x
  128 configs (`big_pairs`); 131,513 numeric + 126,756 non-digit pairs x
  43 configs; exhaustive Thai length <= 3 (839,515 strings) x 43 configs,
  Latin length <= 3 (56,355) x 138 configs, marks length <= 4 (406,901) x
  43 configs (`big_enum`); 800,000 fuzzed tags (`big_tags`, also in a debug
  build) and 330,666 SetTypeForKey lines (`big_settype`) — 0 mismatches
  after the fixes below.
* Platform independence (linux/amd64, go1.27.1, rustc 1.94.1; fixtures
  were first made on darwin/arm64): `data/*.bin` and every checked-in
  fixture regenerate byte-identically, and the documented large runs were
  repeated — 3,000,000 random (seed 21) x 33 configs, 300,000 (seed 22) x
  128, 200,000 stress (seed 23) x 128, every code point x 128, 4 x 60,000
  pairs (seeds 101-104) x 138, numeric 131,513 + non-digit 126,756 x 43,
  enum thai<=3 / latin<=3 (+locales) / marks<=4, 800,000 tags (seeds
  12/13/14), 330,666 SetTypeForKey lines — 0 mismatches.
* Extended campaign, brand-new seeds not used by any checked-in fixture or
  the runs above (linux/amd64): 3,000,000 random strings (seed 9001) x 33
  configs, 300,000 locale strings (seed 9002) x 128 configs, 300,000
  stress strings (seed 9003) x 128 configs, 200,000 `th`-config strings
  (seed 9004), 5 x 60,000 adversarial pairs (seeds 9101-9105, one without
  `-site`) x 138 configs = 300,000 pairs, 8,931-string Thai enum (`-alpha
  thai -maxlen 2 -locales -dump th`) x 138 configs, 3 x 300,000 fuzzed tags
  (seeds 9201-9203) = 900,000 tags, 2 x ~329,700 SetTypeForKey lines
  (seeds 9301-9302) = 659,348 lines — 0 mismatches in every category.

Large runs: `go build -o $S/oracle ./tools/go-oracle/xtext-collate &&
$S/oracle corpus -n 3000000 -seed 21 -out $S/a.txt` (also `-kind stress`,
`-locales`, `-runes`), then
`XTEXT_COLLATE_CORPUS=$S/a.txt,... cargo test --release -- --ignored big_corpus`.
Likewise `oracle pairs -n 60000 -seed 101 -site tests/fixtures/site-strings.hex
-out P` (`-kind numeric|nondigits`, `-every K`) → `XTEXT_COLLATE_PAIRS=P ...
big_pairs`; `oracle enum -alpha thai|latin|marks -maxlen N [-locales] [-dump
CONFIG] -out E` → `XTEXT_COLLATE_ENUM=E ... big_enum`; `oracle tagfuzz -n
300000 -seed 12 -out T` → `XTEXT_COLLATE_TAGS=T ... big_tags`; `oracle
settype -n 40000 -out U` → `XTEXT_COLLATE_SETTYPE=U ... big_settype`.

## Bugs found by adversarial verification (fixed)

1. `language.Parse`/`Make` panicked (Rust slice index out of range in
   `Scanner::token`) on tags with a duplicated `-u` key or duplicated
   variants followed by another extension, e.g.
   `hr-u-ka-noignore-ka-shifted-x-PRIV-u-co`,
   `uk-u-co-zhuyin-kc-false-co-search-a-BCD`,
   `ru-PT-valencia-u-va-posix-ka-posix-ka-posix-a-BCD`,
   `mo_u_co_search_kn_false_co_UNIHAN_A_bcd` (22 of 300,000 fuzzed tags).
   Go reads the shifted buffer through its stale token; the scanner now
   models Go's backing array (deviation 7) and matches Go on all of them.
2. `SetTypeForKey` zero-filled where Go reads stale scanner bytes
   (`...SetTypeForKey("kn", "ab-kn-cd")`); fixed by the same model.
3. `-u-rg-XXzzzz` tags: `Make("th-TH-u-rg-thzzzz").String()` is `th-TH` in
   Go (exact language/locale ids), the port kept the extension and got the
   parent wrong; `compact.Make`/`Tag`/`Parent`/`LanguageID` are now ported.

None of these affect collation of the seeksnack languages (`en`, `th`);
the collation core (collate/colltab/norm) had no discrepancy in any run.

## Known gaps

* `collate/build` (table builder), `collate/tools`, `maketables.go` are not
  ported (runtime only); Go tests that need built tables (TestKey,
  TestCompare, TestGetColElems, TestAppendNext, reg_test/CLDR regression
  files) are not ported.
* The rest of `x/text/language` (Matcher, Coverage, `Script()`/`Region()`
  inference APIs, ParseAcceptLanguage, CompactIndex tables beyond FromTag)
  is not ported.
* Go panics that are not recovered (e.g. inside `compact.FromTag`'s
  `Builder.Make` outside Parse/Compose) are not reproduced; none is
  reachable from tags produced by `Parse`.
* `Collator::compare_string` takes `&str`; Go strings holding invalid UTF-8
  must go through `Collator::compare(&[u8], &[u8])` (identical semantics).
