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
| (none) | `src/stack.rs` | stack-depth guard of `printValue`'s recursion (deviation 9) |
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
- `go_value::Object::underlying() -> Option<Value>` (added by the red-team
  pass): a host object that stands for a **named basic type** (`time.Month`,
  `time.Weekday`, `time.Duration`, `hstring.HTML`, `neohugo.VersionString`,
  ...) returns its value converted to the underlying type
  (`Value::Int(9, IntKind::Int)` for `time.September`, `Value::String` for
  an `hstring.HTML`). fmt then uses that kind wherever Go reflects on it
  (see the table below). Its `String`/`Error`/`GoString` methods stay
  `go_string`/`go_error`/`go_go_string`.

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
| `Object` with `underlying()` = `Bool`/`Int`/`Uint`/`Float`/`String`/`Safe` | a named basic type: methods first (`handleMethods`, as for every non-basic type), then the underlying kind in `printValue` (`%02d` of a `time.Month` → `09`, `%#v` of a named string → `"x"`, bad verbs `%!t(time.Month=9)`); `Sprint` counts a string kind as a string; a `*` width/precision accepts an integer kind; `%p` is a bad verb. Other `underlying()` values are ignored |

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
9. **Recursion depth.** Go's goroutine stacks grow (to 1 GB), so Go prints
   values nested 100,000 levels deep. `printValue`'s recursion
   (`print_value`) runs under `stack::guard` (`src/stack.rs`, a copy of
   go-json's): after about 256 KiB of stack it continues on a scoped helper
   thread with an 8 MiB stack. Output is unchanged. Host methods
   (`go_string`, `go_error`, `struct_fields`, `list`, `map_get`,
   registered `NamedMethod`s ...) of deeply nested values may therefore run
   on a helper thread, so they must not depend on thread-locals. Dropping
   such a value recurses in Rust's drop glue (the caller's concern; the
   tests leak them).

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
  and red-team generators only produce valid UTF-8 error messages for that
  reason (`String` results, registered `NamedMethod`s and `GoString`s may
  hold any bytes and are tested with invalid UTF-8).
- Named basic types (`time.Month`, `time.Weekday`, `time.Duration`,
  `neohugo.VersionString`, `hstring.HTML` …) print like Go only if the host
  object implements `Object::underlying` (added by the red-team pass; see
  its report). Without it they are struct objects: `%02d` of a
  `time.Month` prints `{}` instead of `09`, and `Sprint` puts spaces
  around a named string. go-value's other consumers (gotemplate's
  comparisons and `printf` argument checks, go-json, go-hashstructure) do
  not consult `underlying` yet.
- Map keys are strings only (the value model normalises them), so Go's
  `fmtsort` ordering of mixed key kinds, NaN keys, and non-string keys in
  general cannot arise.
- A bad verb on a `time.Time` whose location is not UTC prints the
  `*time.Location` pointee in Go (`badVerb` re-enters `printValue` at depth
  0 for the unexported `loc` field: `%!t(*time.Location=&{Europe/London
  [{LMT -75 false} …] … 0xc…})`); the port prints `&{}` there. Go's output
  always contains the `cacheZone` pointer, so it is not reproducible
  anyway.
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
- Red-team pass (third; `tools/go-oracle/go-fmt/redteam.go`,
  `tests/redteam.rs`): `redteam_cases.txt` (3,077 cases, seed 1: nested
  operands with nil-receiver hooks, named slice/map types with
  `String`/`Error`, named basic types, fixed zones with arbitrary names;
  formats with flags in any order, malformed indexes, star widths, odd
  verbs; Sprint/Sprintln/Errorf), `redteam_matrix.txt` (`formats.txt` over
  60 random nested operands: 1,106,855 outputs), `flagperm_matrix.txt`
  (every flag string of length 0-3 in every order × 25 verbs × 2 widths ×
  2 precisions, 15,600 formats built by the test, over `values.txt` plus
  50 hook/named-type operands: 4,379,796 outputs), `deep.txt` (values
  10,000-100,000 levels deep, on a 2 MiB thread) and the Go-verified
  `named_basic_types` table. See "Red-team report (third pass)".
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

This is the previous red-team's pass (see the report below for how it was
re-checked). Bugs found and fixed (each has regression cases in `model_cases.txt`,
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

## Red-team report (third pass)

HANDOFF §5 item 2 listed go-fmt as "red-team finished its fixes, but no
final report". This section is that report: what the previous pass did,
an independent re-check of it, the extension of its coverage, and the two
divergences this pass found and fixed.

### What the previous red-team did

Git cannot separate it from the port. The whole crate, its oracle and the
second pass landed in one commit (`0a7ac06b`, "add ported crates awaiting
red-team"). Later commits only exported `named_method` for gotemplate's
`jsValEscaper` (`de563915`) and fixed golangci-lint findings without
changing output (`c67c8afb`). From the code and the section above, the
second pass:

- added the value-model inputs: `model_cases.txt` (named collection
  Stringers, interface nils in containers, GoStringers, nil
  `*time.Location`, time fields, malformed indexes) and the randomized
  `-mode fuzz` generator (`fuzz.go`: nested operands of every kind, formats
  of 1-4 directives, Sprint/Sprintln/Errorf, the dense 18,324-format
  single-operand matrix, `-kinds float|int|str`);
- fixed the six value-model bugs listed above;
- ran seeds 5-7 (293,515 cases, 32.9M matrix outputs) and the
  kind-restricted matrices (73.3M + 27.5M + 27.5M outputs). All passed.

This pass re-checked that work:

- All nine of its fixtures regenerate byte for byte, on linux/amd64 and on
  linux/arm64 under qemu.
- Every test passes.
- Its `fuzz` mode, run at eight fresh seeds with 3-4x larger counts and
  with fresh kind-restricted matrices, finds 0 differences (table below).
- Its six fixes hold. `model_cases.txt` passes, and the red-team corpora
  exercise all of those paths again: Pages/TaxonomyList and their typed
  nils inside lists, maps and struct fields; nil `*time.Location`; nested
  GoStringers; interface nils in `[]error`, `[]fmt.Stringer` and
  `map[string]error`; registered kinds.

### Bugs found and fixed

1. **Deeply nested values overflowed the Rust stack.** This was an abort,
   not a panic.
   - **Input:** a `[]interface {}` nested 10,000 deep around `1`, printed
     with `Sprintf("%v", v)` on a 2 MiB thread.
   - **Go 1.27.1:** prints `[[[…1…]]]` (20,001 bytes). Go also prints
     100,000 levels; it fails only near 1,000,000, at its 1 GB goroutine
     stack limit.
   - **Rust:** `fatal runtime error: stack overflow, aborting` (SIGABRT),
     at 10,000 levels on 2 MiB and at 100,000 levels on 8 MiB.
   - **Root cause:** `src/print.rs` recurses through `print_value` ->
     `print_value_kind` -> `print_slice`/`print_map`/`print_struct`/
     `print_object` -> `print_elem`/`print_field` -> `print_value`, which
     is several frames per level on a fixed-size thread stack. Hugo reaches
     such values through data files (encoding/json allows 10,000 levels)
     and through values built by templates.
   - **Fix:** `src/stack.rs` (go-json's guard) wraps `print_value`
     (deviation 9).
   - **Tests** (in `tests/redteam.rs`):
     - `deep_nesting_on_a_small_stack` uses `deep.txt`: lists, maps,
       structs, named slices/maps, `Kind::Slice`/`Kind::Map` objects and a
       mix, 10,000 to 100,000 levels deep, printed with `%v %+v %#v %d %s
       %x %q %10.3v`. It compares the length and FNV-1a hash with Go's
       output. Without the fix it aborts.
     - `deep_stringers_on_a_small_stack`.
2. **Named basic types could not be expressed.** This is the known gap
   "needs an underlying basic value in go-value". A `time.Month`,
   `time.Duration` or `hstring.HTML` could only be a struct-kind object
   with `go_string`, which printed differently from Go:

   | input | Go | Rust before the fix |
   |---|---|---|
   | `Sprintf("%02d", time.Month(9))` | `09` | `{}` |
   | `%#v` of `hstring.HTML("x")` | `"x"` | `hstring.HTML{}` |
   | `%d` of `hstring.HTML("x")` | `%!d(hstring.HTML=x)` | `{}` |
   | `Sprint(hstring.HTML("a"), 1)` | `a1` (a string kind) | `a 1` |
   | `Sprintf("%*d", time.Month(5), 3)` | `    3` | `%!(BADWIDTH)3` |

   - **Root cause:** `print_value_kind` sent every `Object` to
     `print_object`. `do_print` (its string test) only knew
     `Value::String`/`Safe`, and `int_from_arg` only knew
     `Value::Int`/`Uint`.
   - **Fix in go-value:** a new trait method `Object::underlying() ->
     Option<Value>`. It defaults to `None`, so no other crate needs a
     change.
   - **Fix in go-fmt:** `basic_underlying` now drives `printValue`'s kind
     switch (print.go:777-792), `doPrint`'s string test (print.go:1188),
     `intFromArg` (print.go:928-940) and `fmtPointer`'s kind check
     (print.go:544-550).
   - **Before and after:** on a red-team corpus with named basic types
     (seed 302), 6,845 of 28,416 cases and 39 of 300 matrix operands
     differed before the fix. After the fix, none did.
   - **Tests:** `tests/redteam.rs` `named_basic_types` (25 outputs checked
     against Go), and the `nb:` operands of `redteam_cases.txt` and
     `flagperm_matrix.txt`.
   - **Follow-up for the Hugo layer:** it must implement `underlying` for
     its named basic types: `time.Month` and `time.Weekday` -> `Int`,
     `time.Duration` -> `Int64`, `hstring.HTML` and
     `neohugo.VersionString` -> `String`.

The oracle itself needed care, and future runs should keep these
safeguards:

- The spec decoder now rejects integers that do not fit their kind. A
  mistyped `int16:-1000001` had silently become -16961 in Go.
- A `*` width of 1e6 pads every leaf of a container, and a bad verb on a
  `time.Time` prints the hundreds of fields of its `*time.Location`. So the
  red-team generator:
  - bounds operands to 200 leaves (24 for the matrix, because of
    `%1000001v`);
  - uses the 1e6 star values only with scalar operands;
  - is run under `ulimit -v 6000000`. One early run without these bounds
    was OOM-killed at 13 GB.

### What was run

The Rust side always ran in the test profile (`opt-level = 3` with
overflow checks). "Outputs" counts hashed single-operand matrix outputs.

| run | generator | seeds | cases | outputs | differences |
|---|---|---|---|---|---|
| checked-in fixtures (9 existing + 4 new) | `-mode vectors` | 1 | 20,044 (+ 80 deep) | 12.7M | 0 |
| previous pass's generator, fresh seeds | `-mode fuzz -n 300000 -nmatrix 1000` | 21-28 | 2,346,999 | 146.4M | 0 |
| its kind-restricted matrices | `-mode fuzz -kinds float` (4,000 operands each), `int` and `str` (3,000 each) | 31, 32; 33; 34 | - | 146.6M + 55.0M + 55.0M | 0 |
| red-team, first generator | `-mode redteam` | 101 (20k), 201 and 202 (300k, 1,500 operands) | 583,833 | 52.5M | 0 |
| red-team with named basic types | `-mode redteam` | 301, 302 (30k each) | 56,789 | 8.5M | 0 after fix 2 |
| red-team, final generator | `-mode redteam -n 300000 -nmatrix 1500` | 231-238 (and 205 at 20k) | 2,328,510 | 218.7M | 0 |
| red-team, 12 levels deep | `-mode redteam -depth 12 -n 300000 -nmatrix 1500` | 241, 242 | 577,453 | 54.2M | 0 |
| flag permutations (0-4 flags in every order, 78,100 formats × 291 operands) | `-mode flagperm -n 100000` | 251 | 96,238 | 21.9M | 0 |
| deep values | `deep.txt` plus probes up to 100,000 levels | - | 80 | - | 0 after fix 1 |
| gotemplate on the changed go-fmt | gotemplate `rtpairs pairs`, `rtpairs fmt`, `rtexec` | -, -, 7 (60k templates) | 544,644 + 28,044 + 120,000 | - | 0 beyond gotemplate's listed deviations |
| Go amd64 vs Go arm64 (qemu) | `vectors`; `fuzz -kinds float -n 20000 -nmatrix 600`; `redteam -n 30000 -nmatrix 150` | 1; 61; 62 | all 13 fixtures and both corpora | 11.0M; 2.7M | identical |

In total that is 5,989,822 go-fmt cases and 758.7M matrix outputs outside
the fixtures, with 0 differences once the two fixes were in.

**What the red-team generator produces** (`tools/go-oracle/go-fmt/redteam.go`).

Formats have 1-6 pieces:

- 0-8 flags, in any order and repeated;
- zero-prefixed widths;
- `*` and `[n]*` widths and precisions, taken from ints, uints, values at
  the 1e6 limit, non-ints and named integer types;
- argument indexes that are good, zero, negative, out of range or
  malformed (`[`, `[]`, `[x]`, `[-1]`, `[+1]`, `[01]`, `[1`, `[ 1]`,
  `[1]]`, `[99999999999]`);
- widths of 9-13 digits, where parsenum gives up and swallows the rest of
  the format;
- about 90 verbs: every class of ASCII letter, punctuation, NUL and control
  bytes, `\xff`, truncated and surrogate UTF-8, and U+2028.

Operands are nested up to 6 levels (12 with `-depth 12`). They include
everything `fuzz.go` generates, plus:

- strings of random bytes and of random code points;
- times in fixed zones with arbitrary valid-UTF-8 names (quotes,
  backslashes, NUL, non-ASCII);
- the nil-receiver hook types: with `*NilPanicStr`/`*NilPanicErr` the
  method panics on nil and fmt prints `<nil>`; with `*NilOKStr`/`*NilOKErr`
  it returns a string;
- `main.SS`, a named slice whose value-receiver `String` result needs
  quoting;
- `main.SM`, a named map with a value-receiver `Error`, also wrapped by
  `%w`;
- `[]fmt.Stringer`, `[]error` and `map[string]error` holding those types;
- named basic types: `time.Month`, `time.Weekday`, `time.Duration`, named
  string and float types with `String`, a named string with `Error`, and
  method-less named `int8`, `uint16`, `float32`, `bool` and `string`.

One case in three is `Sprint` or `Sprintln` over operand lists that mix
strings, named string types and everything else; this is the rule behind
gotemplate's `print` and `println` spacing. One case in ten is `Errorf`.

### Divergences left

None were found for values the model can express. The model limits (see
"Known gaps") remain:

- `fmt.Formatter`, and panics in non-nil receivers (`%!v(PANIC=String
  method: …)`), do not exist in the value model.
- `go_error` cannot hold invalid UTF-8.
- Map keys are strings only.
- Complex numbers, arrays, channels and unexported struct fields are not
  modelled.
- Addresses are never byte-identical: `%p`, nested pointers, and the
  `*time.Location` printed for a bad verb on a `time.Time`.
- Named basic types print like Go only if the host implements
  `underlying`.

### Regenerating

```sh
export GOTOOLCHAIN=go1.27.1
go build -o "$TMPDIR/oracle" ./tools/go-oracle/go-fmt
# All 13 fixtures (about 25 s):
"$TMPDIR/oracle" -mode vectors -dir crates/go-fmt/tests/fixtures -scratch "$TMPDIR"
# Large runs outside the repository (about 90 MB per 300k-case directory $D):
( ulimit -v 6000000; "$TMPDIR/oracle" -mode fuzz     -dir "$D" -seed 21  -n 300000 -nmatrix 1000 )
( ulimit -v 6000000; "$TMPDIR/oracle" -mode fuzz     -dir "$D" -seed 31  -n 2000   -nmatrix 4000 -kinds float )
( ulimit -v 6000000; "$TMPDIR/oracle" -mode redteam  -dir "$D" -seed 231 -n 300000 -nmatrix 1500 )   # -depth 12
( ulimit -v 6000000; "$TMPDIR/oracle" -mode flagperm -dir "$D" -seed 251 -n 100000 )
cd crates/go-fmt && GO_FMT_FUZZ_DIR="$D" GO_FMT_FAIL_FILE="$D/failures.txt" \
  cargo test --test fuzz -- fuzz_cases fuzz_matrix
# arm64 Go (reproduces darwin/arm64; every fixture is identical to amd64):
GOARCH=arm64 CGO_ENABLED=0 go build -o "$TMPDIR/oracle-arm64" ./tools/go-oracle/go-fmt
qemu-aarch64-static "$TMPDIR/oracle-arm64" -mode vectors -dir "$D" -scratch "$TMPDIR"
```

`GO_FMT_RT_DEBUG=1` makes `-mode redteam` log every case before it runs,
which finds a case that exhausts memory.
