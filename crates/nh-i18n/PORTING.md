# nh-i18n — porting notes

neohugo langs/i18n + gohugoio/go-i18n/v2 fork (bundle, localizer, message templates, CLDR plural rules).
Wave B task T17. Status: **ported**; all Go-oracle fixtures identical (one documented
divergence in warning text, deviation 6).

## Go file → Rust module

| Rust module | Go source(s) | Owner | Note |
|---|---|---|---|
| `i18n` | `langs/i18n/i18n.go` | T17 i18n | `Translator` (`Func`, `initFuncs`), `intCount`, `getPluralCount`, `toPluralCountValue` |
| `translation_provider` | `langs/i18n/translationProvider.go` | T17 i18n | `NewResource`/`CloneResource` set `Deps.translate`; `addTranslationFile`, `errWithFileContext` |
| `goi18n::bundle` | go-i18n `i18n/bundle.go` | T17 i18n | `Bundle`, go-i18n's art-aware `matcher` |
| `goi18n::localizer` | go-i18n `i18n/localizer.go` | T17 i18n | `LocalizeWithTag`, `getMessageTemplate` (default-language fallback), errors |
| `goi18n::message` | go-i18n `i18n/message.go` + `message_template.go` | T17 i18n | `NewMessage`, `stringMap`, `isMessage`, `MessageTemplate` |
| `goi18n::parse` | go-i18n `i18n/parse.go` | T17 i18n | `ParseMessageFileBytes` with Hugo's unmarshalers, `recGetMessages`, `parsePath` |
| `goi18n::template` | go-i18n `internal/template.go` | T17 i18n | message templates on the `gotemplate` text engine, lazy parse, fast path |
| `goi18n::plural` | go-i18n `internal/plural/{form,operands,rule,rules}.go` | T17 i18n | |
| `goi18n::plural::rule_gen` | go-i18n `internal/plural/rule_gen.go` | T17 i18n | **generated** |
| `xlanguage` | `golang.org/x/text@v0.26.0` `language/match.go`, `language/parse.go` (`ParseAcceptLanguage`), `language/language.go` (`Tag.Script`), `internal/language/match.go` (`addTags`), `internal/language/language.go` (`Region.Contains`, `VariantOrPrivateUseTags`), `lookup.go` (`SuppressScript`) | T17 i18n | the matcher go-i18n uses; parsing/canonicalization/`Parent`/`Base` come from `xtext-collate` |
| `xlanguage::tables` | x/text `internal/language/tables.go`, `language/tables.go` | T17 i18n | **generated** |

go-i18n version: `github.com/gohugoio/go-i18n/v2@v2.1.3-0.20230805085216-e63c13218d0e` (go.mod).
Every ported function has a `// Go: <path>:<Func>` marker; each module ends with its GO PORTING
CHECKLIST (all `EX` entries of the skeleton are `OK`).

## Generated tables

`tools/go-oracle/nh-i18n/gentables` (run from the repository root with `GOTOOLCHAIN=go1.27.1`)
writes both generated files; `#[rustfmt::skip]` keeps them as generated. Rerunning it must not
change them (verified).

- `src/goi18n/plural/rule_gen.rs`: go/ast translation of `DefaultRules()` in
  `internal/plural/rule_gen.go`. Every `addPluralRules` call becomes a Rust call in the same
  order (later rules for the same canonical tag replace earlier ones, as in Go; e.g. `in`→`id`),
  every `PluralFormFunc` a Rust fn whose conditions are the Go expressions (`&&`, `||`, `!`, `%`,
  `intEqualsAny`, `intInRange`, `ops.NEqualsAny`…) with the CLDR comments.
- `src/xlanguage/tables.rs`: x/text's `likelyLang`, `likelyLangList`, `likelyScript`,
  `likelyRegion`, `likelyRegionList`, `likelyRegionGroup`, `regionContainment`,
  `regionInclusion`, `regionInclusionBits`, `suppressScript`, `AliasMap`+`AliasTypes`,
  `regionToGroups`, `paradigmLocales`, `matchLang`, `matchScript`, `matchRegion`,
  `nRegionGroups`, read from the Go sources with go/ast. x/text indexes them by internal ids; the
  generator decodes every id with x/text's own `language.Base/Script/Region.String()` (id 0 →
  `""`) and fails on duplicate keys, so the string keys are in bijection with the ids. List
  indices (`flags & isList`) stay numbers.

## Dependencies

- nh-*: nh-common, nh-config, nh-parser (TOML decoder), nh-langs, nh-hugofs (Walkway, BaseFs),
  nh-helpers (source.File, PathSpec, SourceSpec), nh-deps, nh-tpl.
- Wave A: gotemplate (text/template), go-fmt (`%#v`/`%T` in errors), go-strconv, go-unicode
  (`strings.ToLower`, `EqualFold`, `TrimSpace`), go-sort (`sort.Stable` of
  `ParseAcceptLanguage`), go-yaml (yaml.v2 `Unmarshal` into `interface{}`), go-json
  (`encoding/json`), xtext-collate (x/text `language`: `Parse`, `Make`, `Canonicalize`, `Parent`,
  `Base`, `String`).
- crates.io: none.
- dev: serde_json (fixture reader), flate2 (gzip fixtures; decompression only), nh-allconfig
  (loads the oracle sites like Go's `testconfig`).

## Deliberate deviations

1. **Go map order.** go-i18n ranges over Go maps in `recGetMessages` (the order of a file's
   messages: only visible when two keys produce the same message id), `stringMap` (a message with
   both `translation` and `other`), `unmarshalInterface` (keys differing only in case, e.g. `other`
   and `Other`), and Hugo's `getPluralCount` takes the first map key that EqualFolds `Count`. Go's
   result is random in these cases; the port iterates in byte order. The parse oracle parses every
   input 50 times and would drop unstable ones (none of the 161 inputs is).
2. **x/text ids → subtag strings.** `xlanguage` keys the matcher's index and tables by subtag
   strings instead of `Language`/`Script`/`Region` ids (see "Generated tables"). The ids of a
   tag come from `xtext-collate`'s public `Tag::raw()` (id 0 = `Default`).
   `VariantOrPrivateUseTags` and `IsPrivateUse` are computed from the canonical tag string
   (`str[pVariant:pExt]`, or the whole `x-…` tag).
3. **`matcher.Match` returns the index and confidence only.** Go also builds the matched tag
   (with the wanted region as `-u-rg-` and the wanted extensions); go-i18n discards it
   (`_, i, _ := Match(...)`). The Localizer computes the match once (Go matches on every call;
   the bundle is immutable once shared, so the result is the same).
4. **No `page.PageWithContext`.** Hugo wraps a `page.Page` passed as template data so that the
   message template's method calls get the `context.Context`. The Rust engine passes the host
   context to every method call, so the page is passed as is; only `%T`/`printf "%v"` of `.` in
   a message would differ (Go prints a struct with pointers there anyway).
5. **Message templates run on `gotemplate`.** go-i18n uses the **stdlib** `text/template`
   (go1.27.1), not Hugo's fork. The port executes the gotemplate text engine (go1.24 fork) with a
   helper that restores stdlib behaviour: no functions besides the builtins, exact map keys
   (also for `maps.Params`), Go's `isTrue` instead of Hugo's `IsTruthfulValue`. Differences
   between the go1.24 fork and the go1.27 stdlib that a message template could reach were not
   audited beyond the fixtures.
6. **Static types in template errors (warning text only).** Go's `can't evaluate field X in type
   T` names the receiver's static type: `interface {}` for a `map[string]any` value, an `any`
   field or an `any` method result. The gotemplate value model has no static types and names the
   dynamic type (`int`, `float64`, …). The translated string is the same (`""` or the
   placeholder); only the "Failed to get translated string" warning differs. The translate test
   accepts exactly these 21 calls (`{{ .Count.Foo }}`) and fails on any other difference. A fix
   belongs in gotemplate.
7. **Go runtime panics.** Go panics inside the translate func for `i18n "x" ""`
   (`newOperandsString` indexes `s[0]`), a `Count` that is an untyped nil (`reflect.TypeOf(nil).Kind()`)
   and a nil pointer as data (`reflect.Value.Type` on the zero Value); text/template turns the
   panic into the error of the `i18n`/`T` call. `Translator::func_e` returns these as errors
   with Go's panic text; `Translator::func` (the `nh_deps::deps::TranslateFunc` stored in
   `Deps.translate`, which returns a `String`) panics with the same text. See "Requests".
8. **`errWithFileContext`** keeps Go's message and position (a `*toml.DecodeError`'s position is
   carried in `ParseError::toml_position`, like Go's `extractFileTypePos`), but does not attach
   the error-context lines (`UpdateContent`), which only matter for error display.
9. **Strings.** Message texts and outputs are Rust `String`s. Invalid UTF-8 could only come from
   a YAML `!!binary` value or a data value printed by a message; it is replaced (U+FFFD).
10. **Unmarshalers.** `RegisterUnmarshalFunc` is not ported; `ParseMessageFileBytes` has Hugo's
    four (`toml`, `yaml`, `yml`, `json`) built in. `LocalizeConfig.Funcs` is not supported (Hugo
    never passes funcs). `Rule.plural_forms` is a `Vec` in generated order (Go: a set).
11. **Skeleton signatures changed** (no other crate used them): `Translator::new` takes the logger
    (Go's third argument); `Bundle`, `Rules`, `Localizer` use `xtext_collate::language::Tag`
    instead of `String` tags; `Localizer` owns an `Arc<Bundle>` (no lifetime);
    `localize_with_tag` returns the Go tag; `LocalizeError` has Go's error kinds;
    `Bundle::parse_message_file_bytes` returns `ParseError`. Added: `Translator::func_e`,
    `Translator::languages`, `TranslationProvider::translator`, `get_plural_count` panics like Go
    (the `_e` variant is crate-private and used by the translate funcs).

## Verification (Go oracles, go1.27.1; `cargo test` needs neither Go nor the network)

| test | oracle | inputs | result |
|---|---|---|---|
| `tests/plural.rs` | `tools/go-oracle/nh-i18n/plural` | 212 CLDR locale ids of `rule_gen.go` × 224 counts (Go ints −1…120, powers of ten, int64/int8/uint/bool/float64/nil, decimal and invalid strings) = 47,488 localizations, plus the 16,922 CLDR samples of `rule_gen_test.go` (expanded like `expandExamples`, both the Go result and the test's expected form checked) | identical |
| `tests/matcher.rs` | `tools/go-oracle/nh-i18n/matcher` | 38,856 `NewMatcher(supported).Match(want...)` (1,500 random supported lists of 1–7 of 159 tag strings: regional/script variants, deprecated and macro codes, grandfathered, `art-x-`, `x-`, `und-…`, variants, extensions; every pool tag × every pool tag), 97,717 go-i18n localizations over 600 bundles (which bundle tag answers, incl. the art matcher), 179 `ParseAcceptLanguage` | identical; a mutation of the region-group tie-breaker is caught (183 differences) |
| `tests/parse.rs` | `tools/go-oracle/nh-i18n/parse` | 161 message files: TOML/YAML/JSON, flat/table/nested/dotted keys, reserved keys in other cases, custom ids, delimiters, v1 `id`/`translation` arrays, HTML, Thai, empty files, every error path (texts with `%#v`/`%T`), 19 paths; plus the files of go-i18n's own `TestParseMessageFileBytes`/`TestJSON`/`TestYAML`/`TestTOML`/`TestV1Format`/`TestV1FlatFormat` | identical |
| `tests/translate.rs` | `tools/go-oracle/nh-i18n/translate` | 61 sites, 3,378 `Deps.translate` calls through `NewResource`/`CloneResource`: en+th (seeksnack-like messages with `{{ .Context }}`, plurals, HTML, custom delimiters, parse and exec errors), 65 arguments (ints, 1.5, `"1"`, `""`, maps with `Count`/`count`/`Context`, `maps.Params`, structs with a `Count` field or method, value and pointer, nil pointer, `time.Month`, `template.HTML`, bool, slice), placeholders/printI18nWarnings on and off, th as default language, en th fr pl ar ja ru cy ga pt with all six plural forms, TOML/YAML/JSON + v1 files, theme overlay, the `i18n_integration_test.go` sites, unknown languages (`art-x-`), regional bundles (`pt` answered by `pt-BR` with Go's `%!s(<nil>)` warning), ignoreFiles, no i18n, 7 broken files, and every `i18nTests` (placeholders off/on) and `TestPlural` case of `langs/i18n/i18n_test.go` | outputs, Go panics, NewResource errors and log entries identical, except the 21 warning texts of deviation 6 |
| `tests/go_tables.rs` | — | `TestGetPluralCount` of `i18n_test.go` | ported |

Fixtures: 512 KB gzipped JSON; every fixture and both generated files regenerate byte for byte.

## Known gaps

- go-i18n's `localizer_test.go` table (`TestLocalizer_Localize`, mostly `DefaultMessage`/`Funcs`
  paths Hugo does not use) is not ported; `DefaultMessage` is ported but only covered by reading.
- `language.MatchStrings`/`Comprehends`, `Tag.Region`, `minimize` are not ported (go-i18n does not
  use them).

## Requests to other crates

- **nh-deps (T20):** `TranslateFunc` returns `String`, so the Go panics of deviation 7 can only
  become a Rust panic there. Suggest `Arc<dyn Fn(HostCtx, &str, &Value) -> Result<String>>`
  (then `Translator::func_e` fits directly); until then T19 can call
  `TranslationProvider::translator()?.func_e(lang)` to get Go's template error.
- **gotemplate:** static receiver types for `can't evaluate field` errors (deviation 6).
