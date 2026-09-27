# go-fmt — porting notes

Port of Go's `fmt` printing functions as shipped in **go1.27.1** (the golden
toolchain): `src/fmt/print.go`, `src/fmt/format.go` and `Errorf` from
`src/fmt/errors.go`. The operands are [`go_value::Value`]s, which stand in
for `any` plus reflection. Numbers are formatted by `go-strconv` (Go's own
`AppendFloat`, `AppendQuote*`, `IsPrint` and `CanBackquote`), runes by
`go-unicode` (`utf8`), and `time.Time` by `go-time` (`String`, `GoString`).

## Go file → Rust module map

| Go (go1.27.1 `src/fmt/…`) | Rust | Notes |
|---|---|---|
| `format.go` (`fmt`, `fmtFlags`, `writePadding`, `pad`, `padString`, `fmtBoolean`, `fmtUnicode`, `fmtInteger`, `truncateString`, `truncate`, `fmtS`, `fmtBs`, `fmtSbx`, `fmtSx`, `fmtBx`, `fmtQ`, `fmtC`, `fmtQc`, `fmtFloat`) | `src/format.rs` (`Fmt`) | line by line |
| `print.go` constants, `pp`, `tooLarge`, `parsenum`, `intFromArg`, `parseArgNumber`, `argNumber`, `badArgNum`, `missingArg`, `doPrintf`, `doPrint`, `doPrintln` | `src/print.rs` (`Pp`) | line by line |
| `print.go` `badVerb`, `fmtBool`, `fmt0x64`, `fmtInteger`, `fmtFloat`, `fmtString`, `fmtBytes`, `fmtPointer`, `handleMethods`, `printArg`, `printValue`, `unknownType` | `src/print.rs` | reflection replaced by a match on `Value` (see below) |
| `print.go` `Sprintf`, `Appendf`, `Fprintf`, `Sprint`, `Append`, `Fprint`, `Sprintln`, `Appendln`, `Fprintln` | `src/lib.rs` | |
| `errors.go` `Errorf`/`errorf` | `src/lib.rs` `errorf` | returns the message and the wrapped operand indexes |
| `internal/fmtsort` | — | `Map` entries are already in byte order, and `Object::map_keys` is specified to be in fmtsort order (all keys are strings) |
| neohugo `resources/page/pages.go` `Pages.String`, `resources/page/taxonomy.go` `TaxonomyList.String`, Go `time.(*Location).String` on nil | `src/print.rs` (`pages_string`, `taxonomy_list_string`, `nil_location_string`) | default entries of the named-method registry (below) |

`Printf`/`Print`/`Println` (stdout), `State`/`FormatString`, `Formatter`
and the whole scanning half (`scan.go`) are not ported.

## Public API (for gotemplate and the nh-* crates)

Go strings are bytes, so formats are `impl AsRef<[u8]>` and the results are
`Vec<u8>`.

- `sprint(&[Value]) -> Vec<u8>`, `sprintln(&[Value]) -> Vec<u8>`,
  `sprintf(format, &[Value]) -> Vec<u8>`
- `append(&mut Vec<u8>, &[Value])`, `appendln`, `appendf(&mut Vec<u8>, format, &[Value])`
- `fprint(&mut impl io::Write, &[Value]) -> io::Result<usize>`, `fprintln`, `fprintf`
- `errorf(format, &[Value]) -> (Vec<u8>, Vec<usize>)`: `fmt.Errorf(...).Error()`
  and the indexes of the operands its `Unwrap` returns (one for a single
  `%w`, sorted/deduplicated for several, as in Go).
- `typed_nil_kind(&str) -> NilKind` and `NilKind`: how a `Value::TypedNil`
  type string is classified (below). These are re-exports of
  `go_value::typed_nil_kind`/`go_value::NilKind`, so host crates extend the
  classification with `go_value::register_named_kind` and every crate
  agrees.
- `register_named_method(type_name, NamedMethod::String(f) | NamedMethod::Error(f))`:
  the `String()`/`Error()` method of a named Go type that reaches fmt as a
  `Value::List` (`SliceType::Named`), `Value::Map` (`MapType::Named`) or
  `Value::TypedNil` (host objects use `Object::go_string`/`go_error`
  instead). `f(&Value) -> Option<Vec<u8>>` returns the method's result, or
  `None` when it panics on a nil pointer receiver (Go's `catchPanic` then
  prints an unpadded `<nil>`). Registered by default: `page.Pages`
  (`Pages(N)`), `page.TaxonomyList` (`TaxonomyList(N)`) and
  `*time.Location` (a nil location is `UTC`).

## How reflection maps onto the value model

| Value | Go equivalent in `printArg`/`printValue` |
|---|---|
| `Invalid` at top level (or in `EXTRA`) | `arg == nil`: `%v`/`%T` → `<nil>` (padded), other verbs `%!x(<nil>)` |
| `Invalid` inside a List/Map/struct field | a nil element of the container's element type: `[]interface {}`/`map[string]interface {}`/fields → nil interface (`<nil>`, `%#v` `interface {}(nil)`); `[]*T` → nil pointer; `[]map[..]..` → nil map; element types come from the `SliceType`/`MapType` name |
| `TypedNil(t)` | classified by `typed_nil_kind(t)` (go-value's registry): `*…` pointer, `[]…` slice, `map[…` map, `func(` func, `chan ` chan, known Hugo named slices (`page.Pages`, `resource.Resources`, `langs.Languages`, …) and maps (`maps.Params`, `page.Taxonomy`, …) plus whatever hosts register, **anything else is an interface type**, whose nil is a nil `any` (prints exactly like `Invalid`). A nil `[]uint8` takes `printArg`'s `[]byte` path. A registered `NamedMethod` is called on it (`Pages(0)`, `UTC`, or `<nil>` for a nil-receiver panic). |
| `TypedNil(t)` of an interface type inside a `List`/`Map` | a nil interface stored in an interface-typed element is that element type's nil (Go has no nil `error` inside an `interface {}`), so `[]interface {}` prints `interface {}(nil)` under `%#v`; as a struct field it keeps its own type (it is the field's declared type) |
| `Bool`, `Int(_, kind)`, `Uint(_, kind)` | the corresponding basic kinds (`uint64(int64)` sign extension; `%#v` of unsigned prints `0x…`) |
| `Float(_, F32)` / `F64` | `fmtFloat(v, 32/64, …)`; a float32 is formatted with 32-bit shortest digits by `go-strconv` |
| `String` | `string` (fast path) |
| `Safe(kind, s)` | a named string type (`template.HTML` …): prints like a string, `%T` is the type, counts as a string for `Sprint`'s spacing rule |
| `List` with `SliceType::Uint8` (or `Named("[]uint8")`) | `[]byte` (`printArg` fast path, `%#v` → `[]byte{0x1, …}`); nested, `%s %q %x %X` print the bytes and `%#v` uses `[]uint8` |
| `List` | slice: `[a b]`, `%#v` `T{a, b}`; a named type with a registered `NamedMethod` (`page.Pages`) prints its `String()` for `%v %s %x %X %q` |
| `Map` | map: `map[k:v …]`, `%#v` `T{"k":v, …}`; `page.TaxonomyList` prints `TaxonomyList(N)` |
| `Time` | `time.Time`: `%v %s %x %X %q` use `String()`, `%#v` uses `GoString()`, other verbs print the struct `{wall ext loc}` (`wall` = nanoseconds, `ext` = seconds since year 1, `loc` = nil pointer for UTC); the fields are unexported, so no methods are called on them (a nil `loc` prints `<nil>`/`0`, not `UTC`) |
| `Object`, `go_error()` / `go_string()` / `go_go_string()` | `error` / `fmt.Stringer` / `fmt.GoStringer`, consulted by `handleMethods` (error first) for `%v %s %x %X %q` (GoStringer for `%#v`), at depth 0 and for nested values |
| `Object` `Kind::Ptr` / `Kind::Interface` | pointer to struct: `&{fields}` at depth 0 (the pointee has an empty method set), `0x<identity>` when nested (Go prints an address there too) |
| `Object` `Kind::Struct` | struct `{fields}` from `struct_fields()` (`%+v`/`%#v` add names, `%#v` adds the type) |
| `Object` `Kind::Map` / `Kind::Slice` | map via `map_keys`/`map_get`, slice via `list()` |
| `Object` `Kind::Func` | func pointer (`0x<identity>`) |

## Deliberate deviations (none change output for representable values)

1. **Buffer ownership.** Go's `fmt` holds a pointer to the printer's buffer;
   here `Fmt` owns it (`fmt.buf`) and `Pp` writes through it.
2. **No `sync.Pool`, no `panicking`/`catchPanic`.** Host methods cannot
   panic in the value model, so the `%!v(PANIC=…)` path does not exist.
3. **`arg`/`value` pairs.** Go passes both `arg any` and `value
   reflect.Value` down to `badVerb`; the Rust functions take
   `(Option<&Value>, Option<&Value>)` with the same meaning, so `badVerb`
   chooses `printArg` vs `printValue` exactly like Go.
4. **`fmtFloat` re-slicing.** `num = num[1:]` is an index offset into the
   `AppendFloat` buffer until the sharp-flag block, then a split.
5. **`fmtInteger` / `fmtUnicode`** use a 68-byte stack buffer (Go's
   `intbuf`) and allocate only when width/precision need more, like Go.
6. **Pointee of a `Kind::Ptr` object.** Go runs `handleMethods` again on the
   dereferenced struct at depth 1; there the method set is the value
   receivers only. The value model cannot tell value receivers from pointer
   receivers, so the pointee is modelled with an empty method set (pointer
   receivers). This only matters for `%w` (a value-receiver error type would
   take a different path in `Errorf("%#w")`).
7. **Struct fields.** A field's declared type is unknown: a nil field
   (`Value::Invalid`) is printed as a nil `interface {}` field, a typed nil
   keeps its own type. Host `struct_fields` are exported (methods are
   consulted, Go `CanInterface`); `time.Time`'s `wall`/`ext`/`loc` are not.
8. **Named-type methods.** Go finds `String`/`Error` on named slice/map
   types by reflection; the value model only has the type name, so they
   come from a registry (`register_named_method`) with the neohugo
   `page.Pages`/`page.TaxonomyList` methods and the nil `*time.Location`
   case built in.

## Known gaps (model limitations)

- `fmt.Formatter` is not modelled for host objects (no Hugo type
  implements it). `%!v(PANIC=…)` does not exist: host methods cannot panic,
  and a registered `NamedMethod` returning `None` always prints `<nil>`
  (Go's nil-pointer-receiver case).
- A nil pointer whose type has a `String`/`Error` method prints like a
  plain nil pointer (`%s` → `%!s(*T=<nil>)`) unless the type is registered
  with `register_named_method` (Go calls the method on the nil receiver).
- A `TypedNil` of a named slice/map type that is neither built into nor
  registered with `go_value::register_named_kind` is treated as an
  interface nil (`<nil>` instead of `[]`/`map[]`).
- `Object::go_error` returns a Rust `String`, so an error message with
  invalid UTF-8 cannot be expressed (Go prints its raw bytes). The fuzz
  generator only produces valid UTF-8 error messages for that reason.
- Named basic types with methods (`time.Month`, `time.Weekday`,
  `time.Duration`, `neohugo.VersionString`, `hstring.HTML` …) have no
  representation: as `Object`s their `String` works, but `%d` of a
  `time.Month` (Go `09` for `%02d`) and `Sprint`'s "is a string" spacing
  rule for named string types cannot be reproduced. This needs an
  "underlying basic value" in go-value (see the report).
- Struct printing shows only what `struct_fields()` returns (exported
  fields); Go also prints unexported fields. `struct_fields() == None`
  prints `{}`.
- No `complex64`/`complex128`, arrays, channels or unsafe pointers in the
  value model, so `fmtComplex` and array printing are not ported.
- `%p` and nested pointers print `Object::identity()` / the `Arc` address:
  never byte-identical to Go (Go prints real addresses, which are
  nondeterministic anyway).
- `time.Time` has no monotonic reading, so `String()` never has the
  ` m=+…` suffix and the `{wall ext loc}` struct form assumes
  `hasMonotonic == 0`.

## FMA sites

None. `fmt` does no floating-point arithmetic of its own (floats go straight
to `strconv.AppendFloat`). `go tool objdump -s '^fmt\.'` of the oracle
binary (go1.27.1, darwin/arm64, 59 functions) contains no
`FMADD`/`FMSUB`/`FNMADD`/`FNMSUB`.

## Tests and verification

`tools/go-oracle/go-fmt` generates the fixtures in `tests/fixtures/` from
the real Go `fmt`:

- `values.txt` (241 operands, value spec language defined in
  `tools/go-oracle/go-fmt/spec.go` and parsed by `tests/common/mod.rs`):
  nil, bools, every int/uint kind incl. limits and rune edge cases, float64
  and float32 edge values (±0, NaN, ±Inf, subnormals, 1e21, 2.675 …),
  strings with Unicode, invalid UTF-8, control chars, quotes and backquotes,
  all `template.*` types, `[]byte`, typed nils (slice/map/pointer/func/chan
  and interface), slices of 20 element types (incl. nils and nested
  containers), maps (`map[string]interface {}`, `map[string]string`,
  `maps.Params`, named maps, nil values), times (UTC, nil loc, fixed zones,
  IANA zones), and host objects (pointer/value Stringers, errors, error +
  Stringer, plain structs and pointers to them, map-like and slice-like
  objects).
- `formats.txt` (19,235 formats): 25 verbs (`v d s q x X t b o O c U e E f
  F g G T p % w z ! é`) × all 32 flag subsets of `+-# 0` × widths
  {none,1,5,13} × precisions {none, `.`, .0, .2, .5, .11}, plus 35 malformed
  or multi-directive formats (`%`, `%!v`, `%[2]v`, `%1000001v`, `%\xffv` …).
- `matrix.txt`: one FNV-1a hash per operand over all its outputs.
  `tests/oracle.rs::matrix_hashes` compares **4,488,872 Sprintf outputs**
  (241 × 19,235 minus 146,763 whose Go output contains a pointer address,
  detected by formatting two independently allocated copies of each
  operand). Passing: 241/241 operands.
  `GO_FMT_DUMP_DIR=<dir>` writes the Rust outputs of a failing operand,
  comparable with `oracle -mode dump -value <index>`.
- `cases.txt` (9,944 cases): 2,481 random `Sprint`/`Sprintln` operand lists
  (the spacing rule), 6,863 random multi-directive `Sprintf` formats (star
  width/precision from ints/uints/non-ints/out-of-range values, explicit
  argument indexes before/after width and precision, BADINDEX, MISSING,
  EXTRA, NOVERB, BADWIDTH, BADPREC, `%%`, non-ASCII verbs) and 600
  `Errorf` cases with `%w` (message and wrapped operand indexes). All pass.
- `fmttests.txt`: the oracle extracts the declarations and tables of
  `$GOROOT/src/fmt/fmt_test.go` (go1.27.1) into a temporary program, runs
  `fmtTests`, `reorderTests` and `startests`, and keeps every entry whose
  operands the value model can express (595 entries: 540 fmtTests, 32
  reorderTests, 23 startests; 199 use types the
  model does not have — renamed basic types, arrays, complex numbers,
  channels, Formatters, structs with unexported fields — and 66 print
  pointers or `fmt_test.*` type names). All 595 pass.
- `fuzz_cases.txt`, `fuzz_formats.txt`, `fuzz_matrix.txt`
  (`tools/go-oracle/go-fmt/fuzz.go`, seed 1): 5,880 random cases (random
  nested operands of every kind — lists of 29 slice types, maps of 14 map
  types, host objects incl. GoStringers, times in 16 zones, strings built
  from control/quoting/non-printable/invalid-UTF-8 atoms, `page.Pages`,
  `page.TaxonomyList`, typed nils — with random 1–4 directive formats,
  plus Sprint/Sprintln/Errorf) and a dense single-operand matrix of 18,324
  formats (18 verbs × 14 flag sets × 6 widths up to 70 × 12 precisions up
  to .66, plus precisions .100–.1100 and widths 200/300) over 150 random
  floats, float32s, integers, strings and byte slices: 2,748,600 outputs.
  `tests/fuzz.rs`; `GO_FMT_FUZZ_DIR=<dir>` runs the same tests on
  `oracle -mode fuzz -dir <dir> -seed N -n … -nmatrix … [-kinds float|int|str]`
  output, `GO_FMT_FAIL_FILE=<file>` writes every failing case.
- `model_cases.txt` (548 cases): hand-picked operands for the value-model
  mappings (named collection Stringers, interface nils in containers,
  GoStringer objects, nil `*time.Location`, time fields) × 14 formats, and
  46 malformed formats (bad argument indexes `%[01]d %[+1]d %[]d %[1`,
  stars after indexes, `%[1]%`, `%.%`, NUL verbs …) with 3, 1 and 0
  operands.
- Large runs outside the fixtures (all pass): seeds 5–7 with 100,000 cases
  and 600 matrix operands each (293,515 cases, 32.9M matrix outputs), and
  kind-restricted matrices: 4,000 floats/float32s (73.3M outputs), 1,500
  integers (27.5M), 1,500 strings/byte slices (27.5M).
- `tests/go_tables.rs`: hand ports of `TestFmtInterface`, `TestBlank`,
  `TestBlankln`, `TestStructPrinter`, `TestSlicePrinter`, `TestMapPrinter`,
  `TestEmptyMap`, `TestNilDoesNotBecomeTyped`, `TestAppendf`,
  `TestAppend`, `TestAppendln`, `TestParsenum` (in `src/lib.rs`),
  `errors_test.go:TestErrorf` (all 19 entries, text and wrapped operands),
  `stringer_test.go:TestStringer` (named basic types as struct objects), the
  named-type method registry, the huge
  star-width entries of `startests` (outputs > 2000 bytes are not stored in
  fixtures), the Hugo printing rules of the template-engine spec §8.2,
  `typed_nil_kind` and `Errorf`.
- Mutation check: changing any one of `digits = 6` (`fmtFloat` `#`), the
  `% x` width computation in `fmtSbx`, or the sign allowance in
  `fmtInteger`'s zero padding makes all three oracle tests fail (39–107
  matrix operands, 3–75 cases, 2–20 table entries each).

Not ported: `TestComplexFormatting` (no complex numbers), `TestPanics`,
`TestBadVerbRecursion` (panicking methods / `%p`), `TestFormatterPrintln`,
`TestFormatterFlags`, `TestFlagParser` (Formatter), `TestCountMallocs`,
`TestIsSpace` (scanning).

## Adversarial verification (second pass)

Bugs found and fixed (each has regression cases in `model_cases.txt`,
`fuzz_cases.txt` and `tests/go_tables.rs`; 139 of the 410 value-model
cases of `model_cases.txt` failed before the fixes):

1. `page.Pages` and `page.TaxonomyList` (named slice/map types with a
   value-receiver `String`) printed their elements: `%v` of a Pages with 3
   pages gave `[… … …]` instead of `Pages(3)`, a nil Pages `[]` instead of
   `Pages(0)` (also inside lists/maps/struct fields, and for `%s %q %x %X`
   and `Sprint`). Fixed with the `NamedMethod` registry.
2. Nil `*time.Location` printed `<nil>`/`%!s(*time.Location=<nil>)`
   instead of Go's `UTC` for `%v %s %q %x %X`.
3. `Object::go_go_string` (GoStringer) was ignored: `%#v` printed the
   struct (`&main.GS{S:"x"}`) instead of the `GoString()` result, at depth
   0 and nested.
4. A typed nil of an interface type (`error`, `page.Page`, …) inside an
   interface-typed container printed its own type under `%#v`
   (`[]interface {}{error(nil)}`); Go converts it to the element type's nil
   (`interface {}(nil)`).
5. `typed_nil_kind` was a private copy that ignored go-value's
   `register_named_kind` registry, so host-registered named slice/map types
   printed as `<nil>`; it is now go-value's function.
6. A `Value::List` tagged `SliceType::Named("[]uint8")` missed `printArg`'s
   `[]byte` fast path (`%#v` gave `[]uint8{…}` instead of `[]byte{…}`).

Nothing else differed: the core formatter (`format.go`) and `doPrintf`
matched Go on every generated input, including all float64/float32
verbs with precisions up to 1100, widths up to 300, `%U`/`%x`/`%d` beyond
the 68-byte `intbuf`, and all error forms (`%!v(…)`, `MISSING`,
`BADINDEX`, `BADWIDTH`, `BADPREC`, `NOVERB`, `EXTRA`).

Regenerating the fixtures (needs go1.27.1; takes about 15 s):

```
go build -o "$TMPDIR/oracle" ./tools/go-oracle/go-fmt
"$TMPDIR/oracle" -mode vectors -dir crates/go-fmt/tests/fixtures -scratch "$TMPDIR"
```

`Cargo.toml` sets `[profile.test] opt-level = 3` so the 4.5M-output matrix
runs in a few seconds.
