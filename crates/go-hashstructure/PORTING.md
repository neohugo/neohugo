# go-hashstructure — porting notes

Port of **github.com/gohugoio/hashstructure v0.5.0** (the version in neohugo's
go.mod) and of neohugo's **common/hashing** package. These hashes reach the
golden output as processed-image file names (`_hu_<HashStringHex>`), image
filter keys, the imaging-config `SourceHash`, and the GetRemote file-cache
names.

## File map

| Go | Rust |
|---|---|
| hashstructure@v0.5.0/hashstructure.go (`Hash`, `walker.visit`, `hashUpdateOrdered/Unordered`, `hashFinishUnordered`, `HashOptions`) | `src/hashstructure.rs` |
| hashstructure@v0.5.0/include.go (`Includable`, `IncludableMap`, `Hashable`) | `src/include.rs` |
| hashstructure@v0.5.0/errors.go (`ErrNotStringer`) + the other errors `Hash` returns | `src/errors.rs` |
| reflect (`Value` kinds, `IsZero`, addressability, method sets) | `src/value.rs` (`HashValue`, `GoStruct`, `GoMap`, `is_zero`, `natural_stringer`) |
| reflect/type.go `StructTag.Get/Lookup` | `src/structtag.rs` (unquoting via `go-strconv`) |
| hash/fnv `New64` (FNV-1), cespare/xxhash/v2 `New` (via `xxhash-rust`, XXH64 seed 0) | `src/hasher.rs` |
| neohugo common/hashing/hashing.go | `src/hashing.rs` |
| time.Time.MarshalBinary, Time.String | `go-time` crate (`GoTimeExt`) |

## Public API

```rust
// hashstructure.Hash(v, opts); opts None = FNV + tag "hash" (defaults are filled into Some(opts) like Go)
go_hashstructure::hash(v: &HashValue, opts: Option<&mut HashOptions>) -> Result<u64, Error>
HashOptions { hasher: Option<Box<dyn Hasher64>>, tag_name, zero_nil, ignore_zero_value, slices_as_sets, use_stringer }
XxHash64, Fnv64: Hasher64

// neohugo common/hashing (xxhash options)
hashing::hash_string(&[HashValue]) -> Result<String, Error>      // HashString  (decimal)
hashing::hash_string_hex(&[HashValue]) -> Result<String, Error>  // HashStringHex (lower-case, unpadded)
hashing::hash_uint64(&[HashValue]) -> Result<u64, Error>         // HashUint64 (toHashable per argument)
hashing::hash(&[HashValue]) -> Result<u64, Error>                // Hash
hashing::xxhash_from_reader(&mut dyn Read, ReaderKind) -> io::Result<(u64, i64)>
hashing::xxhash_from_reader_hex_encoded, xxhash_from_string, xxhash_from_string_hex_encoded, md5_from_string_hex_encoded

// The value model
enum HashValue { Nil, Bool, Int(i64, IntKind), Uint(u64, UintKind), Float(f64, FloatKind), Complex64, Complex128,
                 String(GoString), Time(go_value::Time), Array(Vec), Slice(Option<Vec>), Map(GoMap), Struct(GoStruct),
                 Ptr(Option<Box>, Option<Box> /*pointee zero for ZeroNil*/), Interface(Box), Unsupported{kind, nil},
                 Value(go_value::Value) /* converted on the fly */ }
GoStruct::new("filter").field("Options", ...).tagged("C", r#"hash:"set""#, ...).unexported("x", ...).blank(...)
        .with_hashable(Receiver::Pointer, Arc<dyn Hashable>).with_includable(..).with_include_map(..).with_key("k")
GoMap::new(entries) / GoMap::nil() / .with_include_map(..)
register_object::<T: go_value::Object>(|t| HashValue)   // how a host type looks to hashstructure
```

Where Go hashing panics (`HashUint64` on error, `String()` through a nil
`*time.Time`), the Rust API returns `Err` (`Error::GoPanic` for the latter).

### Mapping `go_value::Value` → reflect

`Invalid` → nil interface; `Bool/Int/Uint/Float` keep their Go kind (int is
8 bytes, `float32` 4 bytes); `String`/`Safe` → string kind; `Time` →
`time.Time`; `List` → slice (elements of `[]interface{}` and named slice
types are interface values); `Map` → map (values of `map[string]interface{}`,
`maps.Params` and named maps are interface values; `map[string]string`
values are strings); `TypedNil(t)` → nil slice for `[]…`, nil map for
`map[…]`/`maps.Params`, func/chan for `func…`/`chan…`, otherwise a nil
pointer (hashes as `int(0)`). `Object` → registered conversion if any, else
by `Object::kind()`: `Ptr`/`Interface` → pointer to a struct named
`bare_type_name(type_name())` (`*hugolib.pageState` → `pageState`) with the
`struct_fields()` as exported untagged fields (`fmt.Stringer` from
`Object::go_string` of field values), `Struct` → that struct by value,
`Map` → `map_keys`/`map_get`, `Slice` → `list()`, `Func` → func.
`Object::hash_key()` is neohugo's `keyer` for `toHashable`.

## Semantics reproduced (all verified against Go)

- Kinds: `int` hashes as int64; int8/16/32, uint8/16/32 at natural size;
  bool as int8; float32/64 and complex64/128 as IEEE bits (LE);
  `uintptr` fails (`binary.Write: some values are not fixed-sized in type uintptr`);
  func/chan fail (`unknown kind to hash: func`); `time.Time` via
  `MarshalBinary` (v1/v2 formats, UTC = -1, `unexpected zone offset` error
  for -1 minute offsets, named zones with transitions and TZ extend rules).
- Slices: ordered combine from 0 (`[]` and nil → 0); `hash:"set"` → XOR +
  finish; `SlicesAsSets` → XOR without finish (as in Go). Arrays: ordered.
- Maps: XOR of ordered(k, v) then finish; nil and empty maps are equal;
  `IncludableMap` from the map type (field "") or the containing struct.
- Structs: start from `H(TypeName)` (bare name, `""` for unnamed types);
  `hash:"ignore"`/`"-"`, unexported fields, `IgnoreZeroValue`, `Includable`
  skip a field *without* the finish step; blank `_` fields of a
  non-addressable struct skip the body but *do* run the finish step;
  `hash:"string"`/`UseStringer` use `fmt.Stringer` (incl. `time.Time`,
  `*time.Time`); `Hashable` overrides. Value vs pointer receivers follow
  Go's addressability rules (pointers, slice elements and map entry copies
  are addressable; interface contents, top-level values and struct fields of
  non-addressable structs are not).
- `ZeroNil`: a nil pointer hashes as the zero value of the last pointee type
  (`unknown kind to hash: ptr/interface` when that is a pointer/interface).
- `IsZero` is go1.27.1's: `== 0` for numbers (so `-0.0` is zero, NaN not),
  nil for slices/maps/pointers/interfaces/funcs, blank struct fields ignored.
- neohugo `HashUint64(vs…)`: one argument → `toHashable(v)`, several →
  `[]interface{}{toHashable(v)…}`; `HashString` decimal, `HashStringHex`
  lower-case unpadded hex.
- `XXHashFromReader`: the returned *size* depends on the Go reader type:
  readers with a real `io.WriterTo` (strings/bytes readers) return the total,
  anything else — including `*os.File`, whose `WriteTo` falls back to the
  hasher's `ReadFrom` — returns the byte count of the last `Read`, i.e. 0.
  neohugo's resource hashing therefore records size 0 (verified with
  neohugo's own code on `watermark.png`: `6519743917224815147 0`). Callers
  choose with `ReaderKind`.

## Deliberate deviations / gaps

- Go's map iteration order is random; when several entries of one map make
  hashing fail, Go's reported error varies. The port visits entries in the
  given order (the hash itself is order-independent).
- `TypedNil` types are classified by their type string; a typed nil of a
  *named* slice/map type (e.g. `page.Pages(nil)`) cannot be recognised and
  hashes like a nil pointer. Hosts should use an empty `List`/`Map` with the
  named type, or register the type.
- `identity.IdentityProvider` (`toHashable` → `GetIdentity()`) has no hook
  in `go_value::Object`; callers must substitute it before hashing (neohugo
  does not pass identity providers to the hashing calls that reach output).
- `Object` fields are treated as exported and untagged; hash tags and
  `Hashable`/`Includable` on host types need `register_object`.
- Keyer (`Key() string`) receivers are not distinguished (value vs pointer).
- `time.Time` values carry no monotonic reading (`go_value` does not model
  it), which only matters for `IsZero`/`String()` of `time.Now()` values.
- `Unsupported{nil}` only feeds `IsZero`.
- `binary.Write`'s error for a *named* `uintptr` type names that type
  (`… in type main.MyPtr`); the port always says `uintptr` (error text only).
- `float32` values are carried widened to `f64` (`go_value` model); a
  float32 *signaling* NaN would come back quieted (arm64 `FCVT`), changing
  its hash. Go only produces such values via `math.Float32frombits`.

## FMA sites

None: hashstructure and common/hashing do no float arithmetic.

## Tests and evidence

Oracle `tools/go-oracle/go-hashstructure` (uses the real hashstructure,
xxhash and neohugo packages, including `resources/images.Filters` for the
filter structs and `maps.Params`):

- `random.fixture`: random typed values built with reflect (all scalar
  kinds with NaN/±Inf/-0/subnormals, time.Time in UTC, fixed zones incl.
  second offsets and invalid -1 min, Asia/Bangkok, America/New_York,
  Europe/Dublin, Australia/Lord_Howe loaded from the same TZif bytes on both
  sides; slices, arrays, maps with interface/float/bool keys, pointers,
  interfaces, unnamed structs with random `hash` tags). 7 variants each: xx,
  FNV, SlicesAsSets, ZeroNil, IgnoreZeroValue, UseStringer and
  `hashing.HashUint64(v, v)`. **330,000 values × 7 all identical** (30k +
  300k seeds; 12k checked in); the only tolerance is the Go-random choice
  of error when several map entries fail.
- `named.fixture`: 43 hand-written cases × 7 variants + 14 helper results
  (named structs, tags, blank fields, Stringer, Hashable/Includable with value
  and pointer receivers, IncludableMap on structs and map types, keyer,
  func/chan/uintptr errors, nested pointers and interfaces, times,
  `maps.Params`, the exact neohugo Overlay/Opacity/Process filter keys,
  `HashString` forms, xxhash/MD5 helpers, reader sizes).
- GetRemote: `hashing.HashString(uri, map[string]any(nil))` for all 51
  YouTube video ids of seeksnack equals the 51 file names in the golden
  build's `getresource` file cache (scratch fixture: it contains the site's
  API key; the checked-in fixture uses a placeholder key).
- `tests/vectors.rs`: hashstructure_test.go `TestHash_golden` (FNV),
  neohugo hashing_test.go, and the image vectors of specs/images.md §4.5:
  imaging config `SourceHash` 4bf645f71319dd1d, resize keys, watermark name
  `3bf49ff914f6e68c`, filter key 1682858112077426900, and the golden names
  A/B/C for collon_strawberry_package.jpg.

### Independent verification (round 2)

- `values.fixture` (new, `-values N` in the oracle, tools/go-oracle/go-hashstructure/values.go;
  test `tests/values_model.rs`): random values of the Go types the
  `go_value::Value` model maps to — `map[string]interface{}`,
  `maps.Params`, `map[string]string`, `[]interface{}`, `[]string`, `[]int`,
  `[]int64`, `[]float64`, `[]bool`, `[]uint8`, `[]map[string]interface{}`,
  typed nils (`[]string(nil)`, `maps.Params(nil)`, `(*time.Time)(nil)`,
  `func()(nil)`, …), `template.HTML`, every int/uint/float kind incl.
  `uintptr`, `time.Time` in UTC/fixed/TZif zones — hashed in Go and, on the
  Rust side, built as a `go_value::Value` and hashed through
  `HashValue::Value` (the `from_value` mapping downstream crates use).
  Five variants: `hashstructure.Hash` with xxhash and with defaults (FNV),
  `hashing.HashString(v)`, `hashing.HashString("k", v, v)`,
  `hashing.HashStringHex([]any{v})`. **220,000 values × 5 variants
  identical** (20,000 + 200,000 seeds; 4,000 checked in).
- random.fixture re-run with a new seed: **300,000 values × 7 variants
  identical**.
- named.fixture: 22 new hand-written cases for addressability contexts
  (pointer-receiver `Hashable`/`Includable` and blank `_` fields reached
  through top-level arrays, pointers to arrays, arrays inside slices,
  slices, `[]any`, map values, fields of addressable and non-addressable
  anonymous structs, interface fields), `hash:"string"` on `time.Time`,
  `*time.Time`, nil `*time.Time` (Go panic), `any` holding a time or an
  int (ErrNotStringer), and `IgnoreZeroValue` on a struct mixing zero
  pointers/empty-but-non-nil slices and maps/nil interface/blank-field
  struct/`[2]float64{-0, 0}`/zero time/`[0]int`: **469 results identical**.
- No discrepancy was found in this crate.
