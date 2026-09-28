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
| `paths::path` | `common/paths/path.go` | T02 | skeleton |
| `paths::pathparser` | `common/paths/pathparser.go`, `type_string.go` | T02 | skeleton |
| `paths::url` | `common/paths/url.go` | T02 | skeleton |
| `urls` | `common/urls/baseURL.go`, `ref.go` | T02 | skeleton |
| `text` | `common/text/position.go`, `transform.go` | T02 | skeleton |
| `hstrings` | `common/hstrings/strings.go` | T02 | skeleton |
| `hugio` | `common/hugio/*.go` | T02 | skeleton |
| `loggers` | `common/loggers/*.go` | T02 | skeleton (MINIMAL) |
| `kinds` | `resources/kinds/kinds.go` | T02 | skeleton |
| `files` | `hugofs/files/classifier.go` | T02 | skeleton |
| `glob::glob` | `hugofs/glob/glob.go` + gobwas/glob subset | T02 | skeleton |
| `glob::filename_filter` | `hugofs/glob/filename_filter.go` | T02 | skeleton |

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

## Known gaps and requests to other crates

- **go-json** does not read `Object::underlying`: an `hstring.HTML` would not encode as a JSON
  string. `ParamsMergeStrategy` implements `marshal_json` as a workaround; `hstring.HTML` does not
  (a JSON string of invalid UTF-8 would need go-json's own replacement). go-json should encode a
  named basic object through `underlying`.
- **gotemplate's** default `is_truthful_value` and the comparisons of T18's `eq`/`lt`/... must read
  `underlying` for named basic objects (Hugo's helper uses `hreflect::is_truthful`, which does).
- **T03**: go-toml local dates must implement the `AsTime` method (argument: `LocationRef`).
- **T11/T12**: register `page.Page`/`resource.Resource` with `hreflect::register_interface`.
- **T02**: `KeyRenamer` could use `crate::glob` once ported (deviation 6).
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
