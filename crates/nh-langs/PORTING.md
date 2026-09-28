# nh-langs — porting notes

neohugo langs/{language,config}.go: Language, Languages, collators (x/text collate CLDR 23), locale
translators. Owner and crate lead: Wave B task T03 (parser-langs).

## Go file → Rust module

| Rust module | Go source(s) | Status / note |
|---|---|---|
| `language` | `langs/language.go` | ported (all functions) |
| `config` | `langs/config.go` + `github.com/mitchellh/mapstructure@v1.5.1-0.20231216201459-8508981c8b6c` `WeakDecode` for `map[string]LanguageConfig` (`decodeMap`, `decodeMapFromMap`, `decodeStruct`, `decodeStructFromMap`, `decodeString`, `decodeInt`, `decodeBool`, `Error`) | ported |

Every GO PORTING CHECKLIST entry is `OK`.

## Dependencies

- nh-*: nh-common (htime, locales, maps, hreflect, object).
- Wave A: go-value, xtext-collate, go-time, go-fmt (`%v` in mapstructure errors), go-strconv,
  go-unicode.
- crates.io: none. dev: `serde_json`, `flate2` (`rust_backend`, gunzip of fixtures).

## Collators

`Language::new` is a faithful port of `langs.NewLanguage`: `language.Parse(lang)`; on success both
collators are `collate.New(tag)` (xtext-collate `Collator::from_tag`), otherwise
`collate.New(language.English)`. Every language gets collators built from its own tag: `th` is
never replaced by `en` (HUGO_LAYER.md §10.10), and the oracle checks both tags.
`Language::tag` is the string of the tag `language.Parse` returned (also on error, like Go).
The location is `time.LoadLocation(timeZone)`: UTC for `""`.

## Deliberate deviations

1. **Collator locking.** Go's `langs.Collator` embeds a `sync.Mutex` that callers lock around a
   sort. `Collator::compare_strings` locks for each call; `Collator::lock()` returns a guard
   (`CollatorGuard::compare_strings`) for callers that hold the lock for a whole sort (Go's
   `coll.Lock()`; `pages_sort`, template `sort`). Results are the same.
2. **Unported locale translators.** nh-common ports the `gohugoio/locales` tables of `en` and `th`
   only. Go creates a `Language` for every configured language, disabled or not (seeksnack:
   fr, pl, pt, de, es, zh-cn, zh-tw, ja, nl), and `localescompressed.GetTranslator` returns a
   translator for each. For a locale Go knows but nh-common does not port, `Language::new` stores
   an `UnportedTranslator`: its `locale()` is Go's `Locale()` name (lookup key with CLDR casing
   restored: `en_US`, `zh_Hant`, `es_419`), and every other method fails with the explicit
   `neohugo-rs: locale … is not supported` message (formatting methods panic, the `try_*` ones
   return the error). Only enabled languages format dates, so seeksnack never reaches it. A locale
   Go does not know falls back to the default content language, then `en`, as in Go.
3. `Language::new` returns only the error when the time zone is invalid (Go returns the language
   too; Hugo's caller discards it).
4. `Language::params` starts as an empty `maps.Params` (Go: nil until `SetParams`).
5. **mapstructure** is ported for the one target type `map[string]LanguageConfig`. A field
   without an exact key is matched case-insensitively; when several keys fold to the field name
   Go picks one in random map order, the port the first in byte order (Hugo's config keys are
   lower-cased, so this cannot happen there). Map keys that are not valid UTF-8 are converted
   lossily (`BTreeMap<String, _>`, the skeleton's type). `json.Number` inputs (not produced by
   Hugo's decoders) are not handled.
6. `DeprecationFunc` (injected by hugolib, unused here) is not ported.

## Known gaps / requests

- nh-common (T26): the `gohugoio/locales` tables of the disabled seeksnack languages, or a
  canonical-name lookup, would replace `UnportedTranslator` (deviation 2).

## Verification

| topic | inputs | Rust test | checks |
|---|---|---|---|
| `language/language.json.gz` | `DecodeConfig` of the committed seeksnack language maps (`docs/rust-port/specs/architecture-core-data/config-{en,th}.json`, as JSON-decoded maps and as `maps.Params`) and 24 adversarial maps (weights as strings/hex/floats/bools/±Inf/NaN/±1e20/uint64 max, nil values, wrong kinds, `[]byte`, mixed-case keys, `_merge`, empty); `NewLanguage` for 110 language keys (the 11 seeksnack ones, BCP 47 edge cases: private use, grandfathered, deprecated, extensions, malformed, case variants) × 2 time zones, 8 more time zones (valid, invalid, path-like) and default-content-language fallbacks | `tests/language.rs` | 262 cases: decoded configs or mapstructure's error text; tag, `LanguageCode`, `String`, translator `Locale`, location, error text, and 400 collation signs per language from both collators |
| `collate/collate.json.gz` | 4,949 strings: the T26 corpus (`tools/go-oracle/nh-common/corpus`) plus Thai (the specs' Thai words, with Latin affixes, and 1,500 seeded random Thai strings with pre-posed vowels, tone marks, Thai digits, mixed scripts) | `tests/collate.rs` | 341,271 pairs (all pairs of every 8th string + 150,000 LCG pairs) × `en` and `th`: `GetCollator1(l).CompareStrings` signs, plus the `sort.SliceStable` order of all strings per tag |

`tests/go_tables.rs` ports `language_test.go` (`TestCollator`, concurrent use) and checks the
`*langs.Language` template API and the `Languages` helpers.

Results: 0 mismatches for both tags.

Platform: mapstructure converts a float weight with `int64(f)`, which differs between arm64 and
amd64 for out-of-range values (±1e20, ±Inf, NaN; Rust `as` matches arm64). The checked-in
fixtures come from arm64 builds under qemu and record `"goarch":"arm64"`; amd64 builds give the
same collate fixture and a language fixture that differs only in those cases:

```sh
export GOTOOLCHAIN=go1.27.1
GOARCH=arm64 CGO_ENABLED=0 go build -o /tmp/language-arm64 ./tools/go-oracle/nh-langs/language
GOARCH=arm64 CGO_ENABLED=0 go build -o /tmp/collate-arm64 ./tools/go-oracle/nh-langs/collate
qemu-aarch64-static /tmp/language-arm64 -root . -out crates/nh-langs/tests/fixtures/language
qemu-aarch64-static /tmp/collate-arm64 -root . -out crates/nh-langs/tests/fixtures/collate
```

The fixtures regenerate byte for byte.
