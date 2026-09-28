# nh-common — porting notes

neohugo common/* + compare + resources/kinds + hugofs/files + hugofs/glob + identity(stub) +
cache/dynacache, plus ports of spf13/cast, gobuffalo/flect, jdkato/prose/transform and
gohugoio/locales (en, th).

Crate lead: T01 (common-values). T26 (common-thirdparty-ports) and T02 (common-paths-text) own
the modules marked below.

## Go file → Rust module

| Rust module | Go source(s) | Owner | Status / note |
|---|---|---|---|
| `object` | — | T01 | NEW, frozen API (see "The template-API foundation") |
| `herrors` | `common/herrors/errors.go`, `file_error.go`, `error_locator.go`, `line_number_extractors.go` | T01 | ported; simplified error model (deviation 9); error-context excerpts not ported |
| `constants` | `common/constants/constants.go` | T01 | ported |
| `collections::append` | `common/collections/append.go` | T01 | ported (reflect emulation through `hreflect`) |
| `collections::collections` | `common/collections/collections.go` | T01 | types only |
| `collections::order` | `common/collections/order.go` | T01 | types only |
| `collections::slice` | `common/collections/slice.go` | T01 | ported |
| `collections::stack` | `common/collections/stack.go` | T01 | ported |
| `hashing` | `common/hashing/hashing.go` | T01 | ported over `go-hashstructure` |
| `hreflect` | `common/hreflect/helpers.go` | T01 | ported + the reflect type helpers (`type_of`, `elem_type`, `type_assignable_to`, `assign_to`, `slice_of`) and the named-type registries |
| `htime` | `common/htime/time.go` + `github.com/bep/clocks@v0.5.0` (Start, System) | T01 | ported; `LocationRef` = `*time.Location` template value |
| `maps::cache` | `common/maps/cache.go` | T01 | ported (lock-free compute, deviation 3) |
| `maps::maps` | `common/maps/maps.go` | T01 | ported; `KeyRenamer` uses a private gobwas/glob subset (deviation 6) |
| `maps::ordered` | `common/maps/ordered.go` | T01 | ported |
| `maps::params` | `common/maps/params.go` (+ `ToParamsAndPrepare` from maps.go) | T01 | ported; `PARAMS_METHODS` for the named-type registry |
| `maps::scratch` | `common/maps/scratch.go` | T01 | ported; `*maps.Scratch` object via `go_methods!` |
| `math` | `common/math/math.go` | T01 | ported (arm64 NaN, deviation 8) |
| `predicate` | `common/predicate/predicate.go` | T01 | ported |
| `types::types` | `common/types/types.go`, `closer.go`, `evictingqueue.go` | T01 | ported |
| `types::convert` | `common/types/convert.go` | T01 | ported |
| `types::hstring` | `common/types/hstring/stringtypes.go` | T01 | ported; named basic type with `underlying` |
| `types::css` | `common/types/css/csstypes.go` | T01 | named basic types with `underlying` |
| `compare` | `compare/compare.go`, `compare/compare_strings.go` | T01 | ported |
| `identity` | `identity/identity.go` | T01 | STUB: no dependency tracking (one-shot build); `CleanString`, `StringIdentity`, `IncrementByOne` ported |
| `dynacache` | `cache/dynacache/dynacache.go` | T01 | SIMPLIFIED: get-or-create partitions, first writer wins, no eviction |
| `cast::caste` | spf13/cast@v1.9.2 caste.go/basic.go/number.go/slice.go/map.go | T26 | ported |
| `cast::time` | spf13/cast@v1.9.2 time.go + internal/time.go | T26 | ported |
| `locales` | gohugoio/localescompressed v1.0.1 + locales v0.14.0 (en, th) | T26 | ported; other locales are an explicit unsupported error |
| `flect` | gobuffalo/flect@v1.0.3 | T26 | ported |
| `prose` | jdkato/prose@v1.2.1 transform/title.go | T26 | ported |
| `paths::path` | `common/paths/path.go` | T02 | ported (all functions) |
| `paths::pathparser` | `common/paths/pathparser.go`, `type_string.go` | T02 | ported (all functions; deviations 21-23) |
| `paths::url` | `common/paths/url.go` (+ `helpers/url.go` `IsAbsURL`, placed here by the skeleton) | T02 | ported (unix `UrlFromFilename`/`UrlStringToFilename`; deviations 24-25) |
| `urls` | `common/urls/baseURL.go`, `ref.go` | T02 | ported (deviation 24) |
| `text` | `common/text/position.go`, `transform.go` | T02 | ported (deviation 27) |
| `text::norm` (+ generated `text/norm/tables15.rs`) | `golang.org/x/text@v0.26.0/unicode/norm`: `normalize.go`, `composition.go`, `forminfo.go`, `input.go`, `transform.go`, `trie.go`, the NFC trie of `tables15.0.0.go` (Unicode 15.0.0) | — | NFC and NFD ported (moved from nh-hugofs `nfc.rs`; deviation 27a) |
| `text::xtransform` | `golang.org/x/text@v0.26.0/transform/transform.go` | — | `Transformer`, `Chain`, `String`, `Bytes`, `RemoveFunc` ported (deviation 27a) |
| `text::runes` | `golang.org/x/text@v0.26.0/runes/runes.go` | — | `Set`, `In`, `NotIn`, `Predicate`, `Remove` ported (deviation 27a) |
| `hstrings` | `common/hstrings/strings.go` | T02 | ported; `GetOrCompileRegexp` STUB (deviation 28) |
| `hugio` | `common/hugio/*.go` | T02 | ported (deviation 29); `CopyFile`/`CopyDir` STUB |
| `loggers` | `common/loggers/*.go` | T02 | MINIMAL: levels, counters, distinct/suppressed entries, stderr (deviation 30) |
| `kinds` | `resources/kinds/kinds.go` | T02 | ported |
| `files` | `hugofs/files/classifier.go` | T02 | ported |
| `glob::glob` | `hugofs/glob/glob.go` | T02 | ported |
| `glob::gobwas` | `github.com/gobwas/glob@v0.2.3` (glob.go, syntax, compiler, match, util) | T02 | ported in full, upstream quirks included (deviation 26) |
| `glob::filename_filter` | `hugofs/glob/filename_filter.go` | T02 | ported (deviation 31) |

## Dependencies

- nh-*: none (nh-common is layer 0).
- Wave A: go-value, go-unicode, go-strconv, go-fmt, go-time, go-url, go-path, go-sort,
  go-hashstructure, go-json, go-html.
- crates.io: none directly. XXH64 (`xxhash-rust`) and MD5 (`md-5`) come through go-hashstructure
  (both allowed by README rule 2).
- dev-dependencies: `serde_json` (fixture reader), `flate2` with the `rust_backend` feature
  (decompression of the gzip-compressed oracle fixtures only; allowed by README rule 2).

## The template-API foundation (`object`, frozen)

Public items, stable for every nh-* crate:

- `GoResult<T>` = `go_value::Result<T>`; re-exports `HostCtx`, `Object`, `Value`.
- `go_methods!(Type { "Name" => |s, ctx, args| ..., ... })` generates `Type::GO_METHODS`,
  `Type::go_has_method(name)` and `Type::go_call_method(&self, ctx, name, args)`;
  `go_methods!(Type {})` declares a type without methods. Bodies are non-capturing closures
  `fn(&Self, HostCtx, &[Value]) -> GoResult<Value>`.
- `object_basics!("<%T spelling>")` inside `impl Object for T` gives `type_name`, `has_method`,
  `call_method`, `as_any`; everything else (`kind`, `field`, `is_zero`, `go_string`,
  `underlying`, ...) is written next to it.
- `args`: `exactly`/`at_least` (Go's `wrong number of args for %s: want [at least ]%d got %d`),
  `get`/`opt`/`rest` (`any`/`...any`), and Go's `validateType` for typed parameters with Go's
  texts (`wrong type for value; expected %s; got %s`, `invalid value; expected %s`): `string`,
  `bool`, `int` exact; `int64`, `float64`, `named_string` also accept an `int`/`string` literal;
  `strings` for `...string`; `wrong_type`, `invalid_value` for custom checks.
- `bad_results_error(name, n)`: Go's `can't call method/function %q with %d results` for methods
  whose Go result count is not callable from templates (they still shadow keys and fields).
- `NamedMethods { has_method, call }` and `NamedTypeRegistry` (`register`, `lookup`,
  `named_type_of` — `MapType::Params` is `maps.Params` — `call`, `has_method`) for methods of named
  slice/map types. `maps::params::PARAMS_METHODS` is registered by
  `nh_hugolib::tplapi::named_types::registry()`.
- `ToValue`, `string_slice`.

Companion registries in `hreflect` (for Go reflection that other crates' types take part in):

- `register_named_elem(named, elem)`: the element type of a named slice/map (`page.Pages` →
  `page.Page`). All neohugo named slice/map types are registered by default.
- `register_interface(iface, |type_name| bool)`: which dynamic types implement a named interface.
  **nh-page/nh-resource must register `page.Page` and `resource.Resource`** (including the
  wrapper types) so that `Scratch.Add`/`append` keep `page.Pages`/`resource.Resources` typed as Go
  does (`[]interface{}` + page → `page.Pages`, then `page.Pages` + page → `page.Pages`).

Named basic types are objects that implement `Object::underlying`: `hstring.HTML`,
`maps.ParamsMergeStrategy`, `css.QuotedString`, `css.UnquotedString` here. `hreflect` (kinds,
truthiness), `math`, `hashing`, go-fmt and T26's cast read it.

## Deliberate deviations

1. **Map aliasing.** Go maps are references: `PrepareParams`, `SetParams`, `MergeParams`,
   `KeyRenamer.Rename`, `ConvertFloat64WithNoDecimalsToInt` mutate nested maps (and `[]any`
   elements) in place, `MergeParams` inserts src's nested maps into dst (sharing them), and
   `Scratch.SetInMap`/`DeleteInMap` mutate the stored map. The value model's maps are `Arc<Map>`
   values updated copy-on-write (`Arc::make_mut`), so a mutation is never visible through another
   holder of the same nested map (a template variable holding an earlier `.Scratch.Get` result,
   a config map shared by two parents). `Scratch.Values` returns a snapshot. No seeksnack path is
   known to depend on the sharing.
2. **Map order.** Where Go's result depends on randomized map iteration, the port uses byte
   order: `PrepareParams` with several non-lower-case keys folding to one key keeps the last in
   byte order (Go: a random one; one non-lower-case key plus its lower-case form is deterministic
   and ported exactly); `PrepareParamsClone` with fold-equal keys; `LookupEqualFold` and
   `MergeShallow` take the first fold-equal key in byte order; `KeyRenamer` renames in byte
   order. The oracle marks these cases `nondet` (17 of 27,419 checks).
3. **Caches never compute under a lock** (HUGO_LAYER.md §4.8). `maps.Cache.GetOrCreate` (Go:
   under the write lock) and `dynacache.Partition.GetOrCreate` (Go: lazycache, other callers of
   the key wait) run `create` unlocked and store the result only if the key is still absent; the
   first stored value is returned to every caller. Sequential rendering gives Go's result.
   Errors are not cached (as in Go). `InitAndGet` runs `init` under the lock like Go (it cannot
   re-enter). dynacache: no eviction, no memory-pressure resizing, no stale-value or rebuild
   clearing, and `GetOrCreateWitTimeout` does not enforce the timeout (Go fails the build with
   `timeout after %s`).
4. **`maps.Params` template methods.** `DeleteMergeStrategy` reports whether `_merge` is present
   but cannot delete it from the shared, immutable map value. `GetMergeStrategy` (two results)
   and `SetMergeStrategy` (none) are Go's `evalCall` errors, as in Go. `ParamsMergeStrategy` is an
   enum of the three valid values; `MergeParamsWithStrategy` takes Go's arbitrary strategy string.
5. **Arguments.** The engine evaluates every argument as for `any` (contract C3/C7), so the
   `int64`, `float64` and named-string helpers also accept `int`/`string` from a variable, where
   Go's `validateType` fails. This only matters for templates that fail in Go.
6. **KeyRenamer glob.** `maps::maps` compiles its patterns with a private subset of
   gobwas/glob v0.2.3 (`*`, `**`, `?`, one `[lo-hi]` range or a `[...]` list with `!`,
   `{a,b}` alternatives, `\` escapes) instead of T02's `glob` module, which is not ported yet.
   Integration may switch to `crate::glob`.
7. **Clock.** `htime::set_clock` installs the process clock once (Go assigns `htime.Clock`).
   `StartClock::now` is `time.Now().Add(offset)` in the Local location, exactly like
   `clocks.Start` (the skeleton kept the start time's location).
8. **arm64 NaN.** `DoArithmetic` returns arm64's default NaN (`0x7FF8000000000000`) for an
   invalid float operation (`Inf-Inf`, `0*Inf`, `Inf/Inf`); x86-64 hardware yields
   `0xFFF8000000000000` (the golden build ran on arm64). NaN operands propagate identically.
9. **Errors.** One `herrors::Error` type (message, `ErrorKind`, optional position) replaces Go's
   wrapped error chains; `errors.Is` checks are `ErrorKind` checks (`NotExist`, `Exist`,
   `FeatureNotAvailable`, `Timeout`, ...). File errors take their line/column from the message
   with Go's four line-number extractors (hand-written matchers equivalent to the regexps);
   `UpdateContent` (source excerpt, chroma lexer) and the fs-reading constructors are not ported
   (they only decorate a failing build's message). `Recover`/`PrintStackTrace` print Rust
   backtraces.
10. **identity** is a stub: managers are no-ops, identities are strings. `hashing.toHashable`
    therefore only rewrites `Key()` providers (`Object::hash_key`), not `IdentityProvider`s.
11. **reflect emulation.** A `TypedNil` of interface kind is an untyped nil (`reflect.TypeOf`
    = nil, kind Invalid), as a nil interface converted to `any` is in Go. `IsContextType` is
    always false (methods receive the context separately). `AsTime`/`ToTimeInDefaultLocationE`
    call an object's `AsTime` method with one argument, an `htime::LocationRef`
    (`*time.Location`); T03's go-toml local dates implement it (`htime::location_arg`).
    Struct-kind objects compare by identity in `compare.Eq` (Go compares struct values field by
    field); uncomparable operands (slices, maps) give `false` where Go panics.
12. **hashing** hashes named basic objects through their `underlying` value (as hashstructure's
    reflection does) and registers nh-common's named types with go-hashstructure.
    `xxhash_from_reader` takes a `ReaderKind`, because Go's returned size depends on whether the
    reader implements `io.WriterTo` (`strings.Reader`: the length; `*os.File`: 0).
13. **nil vs empty.** Functions returning `Map`/`Vec` (the map conversions, `ToSliceStringMap`,
    `ToStringSlicePreserveString`, `ToSliceAny`) cannot return Go's nil map/slice; they return an
    empty one (same as T26's cast, below). `collections.Append`'s pointer-to-slice input
    (`&tstSlicers`) has no value-model equivalent.
14. **hstring.HTML** no longer claims `types.Zeroer` (the skeleton's `is_zero`); its truthiness
    comes from its underlying string, as in Go.

T26 (cast, locales, flect, prose):

15. cast results do not distinguish nil from empty slices and maps.
16. `map[interface{}]…` inputs cannot be represented (Hugo converts YAML maps to string keys).
17. flect `LoadInflections` applies entries in byte order, not Go's random map order; this only
    matters for invalid entries.
18. flect custom data (`inflections.json`) is read on first use, not at process start.
19. 26 cast error texts differ, because go-fmt prints a `json.Number` inside a slice as
    `json.Number{}`. Also (found by the T01 oracle): for a non-nil pointer object, Go's cast
    error message dereferences the pointer (`main.pg{id:"p1"} of type main.pg`); the port prints
    the pointer (`&main.pg{...} of type *main.pg`). Error texts only.
20. Go panics are exposed as `try_*` variants that return errors; the plain variants panic like
    Go (flect `Humanize` of an empty result, locales slice panics, `hashing::hash_*`).

T02 (paths, urls, text, hstrings, hugio, loggers, kinds, files, glob):

21. **Paths are `&str`.** `PathParser.Parse` and the path helpers take UTF-8 (the Rust fs layer has
    `String` paths); `Sanitize`, `MakeTitle`, `PathEscape` and `URLEscape` also have byte forms.
    Where Go slices a string mid-rune and returns invalid UTF-8, the port panics: `BaseRel` against
    an owner that is not an ancestor (Go: `p.Base()[len(ob)+1:]`; Hugo only passes the owning
    bundle) and `BaseNoLeadingSlash`/`PathNoLeadingSlash` of a `TrimLeadingSlash` copy (Go's
    `[1:]` on a path without its slash; no Hugo caller). The oracle has 60 such `BaseRel` cases,
    all with synthetic layout owners.
22. **`Path.Unnormalized()`** is `Option<Box<Path>>` with `None` = Go's self pointer, so
    `ModifyPathBundleTypeResource` is visible through it exactly as in Go, and `ForType` /
    `TrimLeadingSlash` copies point at the original (Go copies the pointer). The unnormalized
    path's own `Unnormalized()` is nil in Go and itself here.
23. Go's pooled parse (`ParseIdentity`, `ParseBaseAndBaseNameNoIdentifier`) keeps stale
    `identifiersUnknown` from the pool; nothing reads them; the port allocates.
24. **Non-UTF-8 URL paths.** A baseURL whose percent-decoded path is not valid UTF-8 (`%ff`) is an
    explicit `neohugo-rs:` error (Go accepts it; 1 oracle input). `AddContextRoot` with such a
    base panics with an explicit message (`try_add_context_root` returns Go's bytes).
    `url.URL.String()` of a parsed `&str` is always UTF-8.
25. Go panics kept: `PathEscape`, `URLEscape`, `MakePermalink`, `AddContextRoot` panic on a
    `url.Parse` error (and `MakePermalink` on an absolute link) like Go; the `try_*` forms return
    the panic value as an error. A nil `IsOutputFormat`/`IsContentExt` panics when called, like a
    nil Go func (the `PathParser` callbacks and `LanguageIndex` are `Option`s: Go's nil).
26. **gobwas/glob** is ported in full (lexer, parser, compiler with its tree minimisation and
    matcher gluing, all 19 matchers with `Match`/`Index`/`Len`/`String`) over Go string bytes.
    Upstream bugs are reproduced on purpose: rune counts used as byte offsets (`BTree`, `Row`),
    `Row` cutting a multi-byte rune, `EveryOf.Len`, `LastIndexAnyRunes` searching the whole
    string, U+0000 ending a pattern, U+FFFD rejected by the lexer. Two Go panics are reproduced:
    `Row` slicing past its input (`a{,}` on "a"; 594 panics in the match matrix) and
    `LastIndexAnyRunes` with a non-ASCII separator. The segment pools are not ported (they only
    recycle buffers).
27. `text.RemoveAccents`/`RemoveAccentsString` are ported over the x/text ports of 27a:
    `transform.Bytes`/`transform.String` of `Chain(norm.NFD, runes.Remove(runes.In(unicode.Mn)),
    norm.NFC)`, errors dropped as in Go. `remove_accents(&[u8]) -> Vec<u8>` and
    `remove_accents_string_bytes(&[u8]) -> Vec<u8>` take Go string bytes (invalid UTF-8 becomes
    U+FFFD); `remove_accents_string(&str) -> String` relies on the result being valid UTF-8 (true
    for any input: the NFC stage only sees `runes.Remove` output). Go takes the chain from a
    `sync.Pool`; the port builds one per call (`transform.Bytes`/`String` reset it first, so this
    is equivalent). `unicode.Mn` is go-unicode's table (Go 1.27.1, Unicode 17.0.0), as in the Go
    build; only `norm` has x/text's 15.0.0 data. `Position.String` never adds ANSI colours
    (message text only).
27a. **x/text `norm`, `transform`, `runes`.** Function-by-function ports (NEW; no Hugo owner,
    moved here from nh-hugofs, whose `nfc::nfc_string` re-exports `text::norm::nfc_string`). The
    tables (`src/text/norm/tables15.rs`: `ccc`, `decomps` and its section constants, `nfcValues`,
    `nfcIndex`, `nfcSparseOffset`, `nfcSparseValues`, the dense block count of
    `nfcTrie.lookupValue` and `recompMapPacked` as sorted pairs) are generated by
    `tools/go-oracle/nh-common/norm`, which parses x/text's `tables15.0.0.go` from the module
    cache, so the trie lookups, `compInfo` flags, stream-safe counts (a CGJ after 30
    non-starters, `ss` reset at every `Transform` call), the quick-check spans, the reorder
    buffer and x/text's `combine` (keys truncated to 16 bits) are x/text's own. Not ported (no
    caller): NFKC/NFKD (the `nfkc` trie), `Iter`, `Reader`/`Writer`, `Append`/`AppendString`,
    `IsNormal`, `Span`, the boundary functions; transform's `Reader`, `Writer`, `Append`, `Nop`,
    `Discard`; runes' `If`, `Map`, `ReplaceIllFormed` and `Span` methods. Go's `(nDst, nSrc,
    err)` is a tuple with `Option<TransformError>`. `Form::string(&str)` expects its result to
    be valid UTF-8 (normalization keeps valid UTF-8 valid); `Form::string_bytes`/`bytes` take Go
    string bytes. `unicode-normalization` (README rule 2) is not used: it has a newer Unicode
    version than x/text v0.26.0 and does not reproduce x/text's stream-safe segmentation.
28. `hstrings.GetOrCompileRegexp` returns an explicit unsupported error (no Go regexp port;
    non-EX). `StringEqualFold.Eq` is `eq_any` (clippy: `eq` shadows `PartialEq`).
29. `hugio`: closing is dropping; `StringReadSeeker::read_string` is Go's `StringReader` fast path
    on the concrete type (`read_all` of a `dyn Read` reads everything, the same bytes for a fresh
    reader); `NewOpenReadSeekCloser` exists for byte content (a fresh cursor per open);
    `CopyFile`/`CopyDir` (afero) return an explicit unsupported error. `HasBytesWriter` keeps
    Go's quirks (it searches the whole buffer, stale half included, and skips the rest of a
    `Write` after the first match).
30. `loggers` is MINIMAL: callers format messages (no Go `fmt` verbs), no ANSI colours, no
    `HandlerPost`/panic-on-warning hook, and distinct entries are keyed by (level, message,
    statement id) instead of a hash of level, message and fields. Kept from Go: the level filter
    (entries below it are neither printed nor counted), `ignoreLogs` statement ids (dropped
    before counting), distinct entries at or above the distinct level (dropped before counting),
    per-level counters (`LoggCount`), whitespace trimming, the `Erroridf`/`Warnidf` suppress hint,
    `Errors()` with `StoreErrors`, and the global logger (`Log`, `SetGlobalLogger`).
31. `FilenameFilter::new(&[String], &[String])` treats an empty slice as Go's nil; a non-nil empty
    Go slice gives a filter that matches everything, like no filter. `new_opt` keeps the
    distinction.

## Known gaps and requests to other crates

- **go-json** does not read `Object::underlying`: an `hstring.HTML` would not encode as a JSON
  string. `ParamsMergeStrategy` implements `marshal_json` as a workaround; `hstring.HTML` does not
  (a JSON string of invalid UTF-8 would need go-json's own replacement). go-json should encode a
  named basic object through `underlying`.
- **gotemplate's** default `is_truthful_value` and the comparisons of T18's `eq`/`lt`/... must read
  `underlying` for named basic objects (Hugo's helper uses `hreflect::is_truthful`, which does).
- **T03**: go-toml local dates must implement the `AsTime` method (argument: `LocationRef`).
- **T11/T12**: register `page.Page`/`resource.Resource` with `hreflect::register_interface`.
- **T01 / integration**: `KeyRenamer` can now use `crate::glob::gobwas::compile(&lower, &['/'])`
  (Go: `glob.Compile(strings.ToLower(key), '/')`) instead of its private subset (deviation 6). T02
  did not change `maps`; the gobwas port reproduces Go's compiled trees and `Match` over the glob
  fixture (1,407 patterns × 5 separator sets incl. `/`).
- **T02 skeleton changes** (no other crate used these items): `PathParser`'s fields are `Option`s
  (Go's nil map/funcs; build it with a struct literal and `..Default::default()`);
  `Path::is_bundle` includes `ContentData` like Go (the skeleton's `matches!` did not);
  `BaseURL.url` is a `go_url::Url` (was its `String()`); `FilenameFilter`'s fields are Go's
  (crate-private) with `new_opt`, `new_for_inclusion_func`, and the free functions `match_opt` and
  `append` for Go's nil receiver; `Glob` wraps the compiled matcher (`matches_bytes`, `matcher`);
  `text::remove_accents(&[u8]) -> Vec<u8>` and `text::remove_accents_string(&str) -> String`
  (were `Result`s while they were stubs; only nh-helpers calls them); `text::puts("")` is `""` like Go;
  `Logger` filters by level before counting (the skeleton counted every `errorf`).
- herrors file-error context, identity dependency tracking, dynacache eviction/clearing: not
  needed for a one-shot build (non-EX).

## Verification (T01)

Go oracles in `tools/go-oracle/nh-common/` (package `main` in the neohugo module; shared typed-JSON
encoder `goval`, corpus from T26's `corpus`) write `crates/nh-common/tests/fixtures/<topic>/`.
`cargo test` needs neither Go nor the network.

| topic | inputs | Rust test | checks |
|---|---|---|---|
| `values/params*.json.gz` | the decoded seeksnack config dumps `docs/rust-port/specs/architecture-core-data/config-{en,th,mounts,en-printzero}.json` + case-mixed variants (8), the front matter of every content file under `docs/content`, `hugolib/testsite/content`, `create/skeletons` decoded by neohugo's pageparser (909 distinct maps), 13 adversarial maps (case-mixed/colliding keys, nested maps of every type, `_merge` keys, every value kind, floats at the int64 edges) | `tests/values.rs::params` | 27,419 (17 `nondet`, deviation 2): PrepareParams, PrepareParamsClone, ToParamsAndPrepare, CleanConfigStringMap, IsZero, ConvertFloat64WithNoDecimalsToInt, ToStringMapStringE, KeyRenamer (Hugo's `menu` alias + glob syntax), GetNestedParam (`.`, `/`), GetNested, GetNestedParamFn, MergeParams, SetParams, MergeParamsWithStrategy × 5, MergeShallow, LookupEqualFold, ToSliceStringMap, ToStringMapE |
| `values/scratch.json` | 23 op sequences (Add on int/int64/uint/float/string/HTML/bool/pages/dicts/typed slices/nil/Duration/maps; Set/Get/Delete/Values; SetInMap/DeleteInMap/GetSortedMapValues incl. Go's panics) | `tests/values.rs::scratch` (through the `go_methods!` table) | 204 |
| `values/collections.json` | Append: 26 `to` × 29 `from` lists; Slice: 28 arg lists (typed slices, Slicers returning a named slice, Params, HTML, nil) | `tests/values.rs::collections` | 782 |
| `values/reflect.json` | 90 values of every kind | `tests/values.rs::reflect` | 990: IsTruthful, IsSlice, IsMap, IsNil, IsValid, IsNumber, ToSliceAny, ToStringSlicePreserveStringE, ToStringE, TypeToString, ToDurationE |
| `values/htime.json` | ToTimeInDefaultLocationE: 22 inputs × 3 zones; TimeFormatter.Format: en/th × 14 instants × 31 layouts | `tests/values.rs::htime` | 934 |
| `math/math.json.gz` | DoArithmetic: 56 operands (every int/uint/float kind with edge values, NaN/±Inf/−0, strings, HTML types, named types, bool, nil, time, slice, maps)² × `+ - * / % ^` | `tests/math.rs` | 18,816 |
| `hashing/hashing.json.gz` | HashString/HashStringHex/HashUint64/Hash of 44 typed values (Params with ParamsMergeStrategy, hstring.HTML, Key() providers, times, NaN), their pairs and all at once; XXHashFromString, XxHashFromStringHexEncoded, MD5FromStringHexEncoded, XXHashFromReader (WriterTo and plain readers) over the 3,227-string corpus and 9 long inputs around the 48 KiB buffer | `tests/hashing.rs::oracle` | 13,288 |
| `compare/compare.json.gz` | the corpus (3,252 strings incl. fold edge cases) sorted with `sort.SliceStable`+`LessStrings`; `Strings` for neighbours, case variants and 20,000 random pairs; Eq/ProbablyEq over 13 values incl. an Eqer | `tests/compare.rs::oracle` | 46,183 + the sort order |

Results: 0 mismatches. Go's own `*_test.go` tables are ported in `tests/go_tables.rs` (maps,
params, scratch, ordered, append, slice, stack, types, convert, evictingqueue, hstring,
predicate), `tests/math.rs`, `tests/hashing.rs`, `tests/compare.rs` (+ `compareFold` in
`src/compare.rs`) and `tests/misc.rs` (dynacache, herrors, identity), plus `object` unit tests.

Hashing vectors (`tests/hashing.rs::seeksnack_vectors`): imaging config SourceHash
`4bf645f71319dd1d` (from a config map prepared by `ToParamsAndPrepare`, so `_merge` holds
`ParamsMergeStrategy` objects), `key[resize 600x480]` `7bfd4638d4eb3be2` (and the 3 other keys),
the watermark target `3bf49ff914f6e68c`, `filterKey(Overlay wm600x480, 0, 0)`
`1682858112077426900`, the A/B/C target names. Two inputs are private: `xxhash64(watermark.png)`
= `6519743917224815147` (the site's asset) enters through the watermark's `Key()`, and the
GetRemote key `iIsZs0m-BVU → 5844198154546968338` needs the site's API key; the same key code path
is checked on the 51 public-key URIs of go-hashstructure's fixture (`tests/hashing.rs::remote_keys`).

Platform: `ConvertFloat64WithNoDecimalsToInt` (2^63 saturates to MaxInt64 on arm64 only),
`types.ToDurationE` of ±Inf/1e300 (cast's float→int), and `DoArithmetic`'s NaN sign differ between
arm64 and amd64 Go (1 + 2 + 20 fixture lines). Every fixture comes from an arm64 build and records
`"goarch":"arm64"`:

```sh
export GOTOOLCHAIN=go1.27.1
for t in values math hashing compare; do
  GOARCH=arm64 CGO_ENABLED=0 go build -o /tmp/$t-arm64 ./tools/go-oracle/nh-common/$t
done
qemu-aarch64-static /tmp/values-arm64 -root . -out crates/nh-common/tests/fixtures/values
qemu-aarch64-static /tmp/math-arm64 -out crates/nh-common/tests/fixtures/math/math.json.gz
qemu-aarch64-static /tmp/hashing-arm64 -root . -out crates/nh-common/tests/fixtures/hashing/hashing.json.gz
qemu-aarch64-static /tmp/compare-arm64 -root . -out crates/nh-common/tests/fixtures/compare/compare.json.gz
```

The fixtures regenerate byte for byte (gzip at best compression, no header name or time;
order-dependent results are detected with 400 runs when a map has fold-equal keys).

## Verification (T02)

Go oracles `tools/go-oracle/nh-common/{paths,urls,glob,textmisc}` write
`tests/fixtures/{paths,urls,glob,textmisc}/`; the Rust tests are `tests/{paths,urls,glob,textmisc}.rs`
(shared helpers in `tests/t02support`). The seeksnack site is private, so the inputs are this
repository's Hugo sites, the embedded templates, the seeksnack paths quoted in
`docs/rust-port/specs`, synthetic sweeps and the string corpus of `../corpus`.

| topic | inputs | Rust test | checks |
|---|---|---|---|
| `paths/pathparser.json.gz` | 14,513 `Parse(component, path)` calls with 3 parsers: `seeksnack` (the real `ContentPathParser` of `allconfig` loaded from `config-en.json`: en + th, 9 disabled languages, the site's output formats and media types), `test` (pathparser_test.go's) and `nolang` (nil `LanguageIndex`). Inputs: every file and directory of `docs/{content/en,layouts,assets,data,static,archetypes}`, `hugolib/testsite`, `create/skeletons` and `tpl/tplimpl/embedded/templates` (2,937); the specs' seeksnack content and asset paths and 58 layouts (282); a sweep of 12 directories × 164 names (bundles in every case, `.en`/`.th`/disabled/unknown languages, output-format/kind/layout/baseof identifiers, multiple dots, no extension, dot files, unicode, spaces, uppercase) over all 8 components (6,793); slash forms (287); the corpus as term keys and layout names (4,214). Every call to `IsOutputFormat`/`IsContentExt`/`IsLangDisabled` is recorded and replayed (a call Go never made fails the test). | `paths.rs::pathparser_matches_go` | 1,180,820: every exported accessor (37) of the path and of `Unnormalized()` (or its identity), 10 of `TrimLeadingSlash()`, `ForType`, `PathRel`/`BaseRel`, `ModifyPathBundleTypeResource`, `ParseIdentity`, `ParseBaseAndBaseNameNoIdentifier`, `NormalizePathStringBasic`, `HasExt` |
| `paths/strings.json.gz` | 4,990 strings: the corpus (3,214, text helpers only), the repository/spec/sweep paths and 54 adversarial URLs | `paths.rs::string_helpers_match_go` | 124,159: every path.go/url.go helper (`Sanitize`, `MakeTitle`, `PathEscape`, `URLEscape`, `AddContextRoot` × 4 bases, `MakePermalink` × 3 hosts, `GetRelativePath`, `CommonDirPath`, `Uglify`, `PrettifyURL*`, …) |
| `urls/baseurl.json.gz` | 3,276 base URLs: seeksnack's, Hugo's test ones, 54 adversarial (opaque, ports, userinfo, IPv6, escapes, unicode) and the corpus | `urls.rs::base_url_matches_go` | 3,867: every `BaseURL` field and method, `WithProtocol` × 9, `WithPort` × 4 (1 deviation 24) |
| `glob/compile.json.gz`, `glob/match.json.gz` | 1,407 patterns (upstream and Hugo patterns, 1,500 random token soups from a fixed seed) × 5 separator sets × 274 inputs | `glob.rs` | 8,441 compiled trees (`String()`) and errors; 2,179,944 `Match` results incl. 594 Go panics; `GetGlob` |
| `glob/filter.json.gz` | 18 `FilenameFilter` configurations (Go's tests, mount include/exclude globs, the single-file inclusion func, `Append` chains, bad globs) × 2,802 file/dir names from `docs/`; the hugofs/glob path helpers | `glob.rs::filename_filter_matches_go` | 65,375 |
| `textmisc/textmisc.json.gz` | kinds, component folders, text (`Chomp`, `Puts`, `VisitLinesAfter`, `Position.String`), `EqualFold`/`InSlicEqualFold` triples, 600 `HasBytesWriter` streams cut into random writes | `textmisc.rs` | 16,975 cases |

Results: 0 differences outside deviations 21 and 24. Go's tests are ported:
`pathparser_test.go`, `path_test.go`, `url_test.go` (`tests/paths.rs`), `baseURL_test.go`
(`tests/urls.rs`), hugofs `glob_test.go`, `filename_filter_test.go` and gobwas `glob_test.go`
(`tests/glob.rs`), `kinds_test.go`, `classifier_test.go`, `position_test.go`,
`transform_test.go`, `strings_test.go`, `hasBytesWriter_test.go` (`tests/textmisc.rs`).
Mutations of `IsBundle`, `Sanitize`'s `%XX` bound, `BTree`'s rune step and `Row`'s rune cut each
fail the oracle tests.

Nothing here depends on the platform (no floats; unix path rules), so these fixtures carry no
`goarch`. They regenerate byte for byte on linux/amd64 and from linux/arm64 builds of `urls`,
`glob` and `textmisc` run under `qemu-aarch64-static`. `paths` imports `allconfig`, which needs
cgo (libwebp), so its arm64 check replayed the recorded seeksnack callbacks instead: same bytes.

```sh
export GOTOOLCHAIN=go1.27.1
go run ./tools/go-oracle/nh-common/paths -root . -out crates/nh-common/tests/fixtures/paths
go run ./tools/go-oracle/nh-common/urls -root . -out crates/nh-common/tests/fixtures/urls
go run ./tools/go-oracle/nh-common/glob -root . -out crates/nh-common/tests/fixtures/glob
go run ./tools/go-oracle/nh-common/textmisc -root . -out crates/nh-common/tests/fixtures/textmisc
```

## Verification (x/text norm, RemoveAccents)

`tools/go-oracle/nh-common/norm` writes `src/text/norm/tables15.rs` (see deviation 27a) and
`tests/fixtures/norm/`; the Rust test is `tests/norm.rs`. Each case records, where they differ
from their fallback, `norm.NFC/NFD.String`, `Form.Bytes`, `transform.String(norm.NFC/NFD, s)`,
`text.RemoveAccents` and `text.RemoveAccentsString`; the test runs all eight on every input.

| topic | inputs | Rust test | checks |
|---|---|---|---|
| `norm/runes.json.gz` | every code point alone (the 15,274 that any function changes are listed; the test checks that the other 1,096,790 are unchanged by all eight); every non-starter, backward-combining rune and decomposable (and every 5th Hangul syllable) in 4 contexts (`a_b`, `e_◌́`, `ọ_`, `ᄀ_ᆨ`); every byte 0x80–0xFF and 32 invalid sequences (truncated, overlong, surrogates, > U+10FFFF) between letters, marks and jamo | `norm.rs::runes_match_go` | 36,949 cases, 9,069,912 results |
| `norm/strings.json.gz` | the former nh-hugofs `nfc` inputs (24,333: NFD of every decomposable, Hangul, Thai, file names, 20,000 seeded mixes); Hugo's `TestRemoveAccents` strings and adversarial ones (CGJ, 31+ mark runs, 5,000 marks, 3,000 invalid bytes, long Hangul); 12,000 seeded random strings of starters, marks, Mn, decomposables (precomposed and NFD), Hangul syllables and jamo, CGJ, invalid UTF-8, runs of 25–75 marks and of 25–75 non-Mn non-starters (Hangul V/T jamo, Mc marks with a combining class, which survive `runes.Remove`); 40 random strings up to 6 KB and 240 mark/jamo/invalid runs at every offset around 128 (`transform.String`'s chunk) and 4,096 (`Chain`'s buffers); the docs/ site's content strings (the nh-common corpus and every non-ASCII line of `docs/content/**/*.md`) | `norm.rs::strings_match_go` | 40,777 cases, 326,216 results |

Results: 0 differences. Every `RemoveAccentsString` result is valid UTF-8. Mutation checks:
changing the stream-safe limit (30 → 31) fails 19,364 string results. `transform.String`'s
128-byte chunks and `Chain`'s 4,096-byte buffers do not change any result in the fixtures (x/text
keeps its segmentation independent of them), so their mutations pass; they are ported as Go has
them. Go's `TestRemoveAccents` is `norm.rs::go_test_remove_accents` (and in `tests/textmisc.rs`).

Nothing here depends on the platform (no floats). The tables and fixtures regenerate byte for
byte on linux/amd64:

```sh
export GOTOOLCHAIN=go1.27.1
go run ./tools/go-oracle/nh-common/norm -root . -tables crates/nh-common/src/text/norm/tables15.rs -out crates/nh-common/tests/fixtures/norm
```
