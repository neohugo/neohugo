# go-json: porting notes

Port of Go's `encoding/json` v1 API **as built by go1.27.1**, over
`go_value::Value`.

## The golden toolchain uses the jsonv2-backed implementation

go1.27.1 turns `GOEXPERIMENT=jsonv2` on by default
(`$GOROOT/src/internal/buildcfg/exp.go`: `JSONv2: true`). With it,
`encoding/json` compiles the `v2_*.go` files, which implement the v1 API on
top of `encoding/json/v2` (arshaling) and `encoding/json/jsontext` (syntax)
with `DefaultOptionsV1`. The classic `encode.go`/`decode.go`/`scanner.go`/
`indent.go`/`stream.go` files are excluded (`//go:build !goexperiment.jsonv2`).
The golden `neohugo-go` binary links 335 `encoding/json/jsontext` symbols.

This crate ports that stack. The two implementations produce different
bytes in several places. These differences matter to other crates and to
the specs:

| input | classic v1 | go1.27.1 (this crate) |
|---|---|---|
| invalid UTF-8 byte in a string | `\ufffd` escape | the raw U+FFFD character (`EF BF BD`) |
| byte `0x80` outside a string, syntax error | `invalid character '\u0080' ...` | `invalid character '\x80' ...` |
| `Compact` error offset | always 0 | the real offset |
| `1e400` decoded into `any` | nil, offset after the literal + 1 | +Inf is stored, offset after the literal |
| `{"a":1e400}` error text | `... into Go value of type float64` | `... into Go struct field .a of type float64` |
| `MarshalJSON` error type | `json.RawMessage`, `time.Time` | `*jsontext.Value`, `*time.Time` (pointer to receiver) |
| `time.Time` year out of range | `...: Time.MarshalJSON: year outside of range [0,9999]` | `...: year outside of range [0,9999]` |
| `Indent` with a non-space/tab prefix or indent | written as given | placeholder spaces, replaced afterwards (also after a trailing newline) |

`<`, `>`, `&` (HTML escaping), U+2028/U+2029, control characters,
sorted map keys and float formatting are the same in both implementations.
Specs that say invalid UTF-8 becomes `\ufffd` (for example
`template-engine.md` §8.6 and `i18n-lang-misc.md` §3.12) describe the classic
implementation. go1.27.1 emits the raw U+FFFD character.

## Go file → Rust module map

| Go (go1.27.1 `src/encoding/json/…`) | Rust | Notes |
|---|---|---|
| `internal/jsonflags/flags.go` | `src/jsonflags.rs` | All flags, same bit layout (43 bits) |
| `internal/jsonopts/options.go` (+ option constructors from `jsontext/options.go`, `v2/options.go`, `v2_options.go`) | `src/jsonopts.rs` | `Options` interface → `Opt` enum; `Marshalers`/`Unmarshalers` values not ported |
| `internal/jsonwire/wire.go` | `src/jsonwire/mod.rs` | `QuoteRune`, `NewInvalidCharacterError`, `NewInvalidEscapeSequenceError`, `InvalidTextError`, `TruncatePointer` (`CompareUTF16`, `TrimSuffix*` not needed) |
| `internal/jsonwire/decode.go` | `src/jsonwire/decode.rs` | complete except `ParseUint` (only used by `Token.Int/Uint`) |
| `internal/jsonwire/encode.go` | `src/jsonwire/encode.rs` | complete |
| `jsontext/state.go` | `src/jsontext/state.rs` | state machine, `objectNameStack`, pointers; duplicate-name namespaces not ported |
| `jsontext/decode.go` | `src/jsontext/decode.rs` | complete for the v1 paths |
| `jsontext/encode.go` | `src/jsontext/encode.rs` | buffered encoders only (see deviations) |
| `jsontext/errors.go` | `src/jsontext/errors.rs` + `src/goerr.rs` | |
| `jsontext/token.go` | `src/jsontext/token.rs` | constructors and accessors the v1 API reaches |
| `jsontext/value.go` (`AppendFormat`, `Value.Kind`) | `src/jsontext/mod.rs`, `encode.rs:value_kind` | |
| `jsontext/pools.go`, `export.go` | constructors `new_buffered`/`new_streaming` | pooling does not affect output |
| `v2/arshal.go` | `src/arshal/mod.rs` | `Marshal`, `marshalEncode`, `Unmarshal`, `unmarshalDecode` |
| `v2/arshal_default.go` | `src/arshal/default.rs` | bool, string, int, uint, float, `[]byte`, slice, map, struct, interface arshalers |
| `v2/arshal_any.go` | `src/arshal/any.rs` | marshal fast paths (the unmarshal fast path is disabled under v1) |
| `v2/arshal_methods.go` | `src/arshal/methods.rs` | `Marshaler`, `TextMarshaler`, `MarshalerTo`/`UnmarshalerFrom` (json.Number) |
| `v2/arshal_time.go` | `src/arshal/time.rs` | `time.Time` marshal, RFC 3339 (no format tags) |
| `v2/errors.go` | `src/arshal/errors.rs` | `SemanticError` and its constructors |
| `v2/fields.go` | `src/encode_struct.rs` (host-side) | hosts resolve fields and tags into `JsonStruct` |
| `v2_encode.go` | `src/v2_encode.rs`, `src/error.rs` | `Marshal`, `MarshalIndent`, error types |
| `v2_decode.go` | `src/v2_decode.rs`, `src/arshal/methods.rs` (`Number`) | `Unmarshal` into `*any` and `*map[string]any` |
| `v2_indent.go` | `src/v2_indent.rs` | `HTMLEscape`, `Compact`, `Indent` |
| `v2_scanner.go` | `src/v2_scanner.rs` | `Valid`, `SyntaxError`, `transformSyntacticError` |
| `v2_stream.go` | `src/v2_stream.rs`, `src/arshal/methods.rs` (`RawMessage`) | `Decoder` (Decode/Token/More/InputOffset/Buffered/UseNumber), `Encoder` |
| `v2_inject.go` | `src/v2_inject.rs` | `transformMarshalError`, `transformUnmarshalError` |
| (none) | `src/stack.rs` | stack-depth guard (see deviations) |

## Public API (for downstream crates)

```rust
// Marshal (Go json.Marshal / MarshalIndent; HTML escaping on)
go_json::marshal(&Value) -> Result<Vec<u8>, go_json::Error>
go_json::marshal_indent(&Value, prefix, indent) -> Result<Vec<u8>, Error>
go_json::marshal_with(&Value, escape_html: bool) -> Result<Vec<u8>, Error> // Encoder bytes without the '\n'

// Encoder (Go json.NewEncoder): jsonify = set_escape_html + set_indent + encode, then drop the '\n'
let mut enc = go_json::Encoder::new(Vec::new());
enc.set_escape_html(false); enc.set_indent("", "  "); enc.encode(&v)?; let out = enc.into_inner();

// Unmarshal (Go json.Unmarshal into `var v any` / `m := make(map[string]any)`)
go_json::unmarshal(&[u8]) -> Result<Value, Error>
go_json::unmarshal_partial(&[u8]) -> (Value, Option<Error>)      // what Go leaves in v, and the error
go_json::unmarshal_map(&[u8]) -> Result<Value, Error>            // null → TypedNil("map[string]interface {}")
go_json::unmarshal_map_partial(&[u8]) -> (Value, Option<Error>)

// Streaming decoder (Go json.NewDecoder)
let mut dec = go_json::Decoder::new(reader);           // any std::io::Read
dec.use_number(); dec.decode() / decode_map() / decode_partial();
dec.token() -> Result<go_json::Token, Error>            // Token::Delim(b'[') | Token::Value(v)
dec.more(); dec.input_offset(); dec.buffered();

// Syntax helpers
go_json::valid(&[u8]) -> bool
go_json::compact(&mut Vec<u8>, &[u8]) -> Result<(), Error>
go_json::indent(&mut Vec<u8>, &[u8], prefix, indent) -> Result<(), Error>
go_json::html_escape(&mut Vec<u8>, &[u8])

// Host types
go_json::Number(GoString)                  // json.Number (an Object; Decoder::use_number produces it)
go_json::RawMessage(Option<Vec<u8>>)       // json.RawMessage = jsontext.Value (None = nil → null)
go_json::JsonStruct { type_name, fields: Vec<JsonField> }   // a Go struct without MarshalJSON
go_json::JsonField { name, value, omit_empty, omit_zero, string, interface_typed }

// Errors: Display is Go's err.Error() byte for byte
go_json::Error::{Syntax(SyntaxError{msg, offset}), UnmarshalType(UnmarshalTypeError{value, type_name, offset, struct_name, field, err}),
                 UnsupportedType{type_name}, UnsupportedValue{str}, Marshaler(MarshalerError{type_name, err, source_func}),
                 Message(String), Eof, UnexpectedEof, Io(String)}
Error::offset() -> Option<i64>   // SyntaxError/UnmarshalTypeError offset (Hugo herrors positions)
```

### How `Value` maps to Go types

Go picks an arshaler per `reflect.Type`. The port picks one per `Value`
variant: `arshal::marshal_concrete` is the dynamic type's arshaler.
`arshal::Static` records whether the static type at a position is an
interface: map values of `map[string]any`/`maps.Params`/named maps,
elements of `[]any` and named slices, and `JsonField::interface_typed`
fields. This matters for `omitempty`, `omitzero` and the `string` tag, as it
does in Go.

- `Invalid` → nil interface; `TypedNil(_)` → nil pointer/map/slice (`null`).
- `Int`/`Uint`/`Float(F32|F64)` → the numeric kinds (float32 uses 32-bit
  formatting and cut-offs).
- `String` → `string`; `Safe(k, s)` → the named string type `k.go_name()`.
- `Time` → `time.Time` (v2 time arshaler).
- `List` with `SliceType::Uint8` → `[]byte` (base64); other lists → slices.
- `Map` → `map[string]…` (keys sorted bytewise, the `Deterministic` order).
  `MapType::Params` is Hugo's `maps.Params`: `omitzero` calls its `IsZero`
  method (zero when empty or when `_merge` is its only key).
- `Object`: `Number` → `json.Number` (`MarshalJSONTo`); `JsonStruct` → the
  struct arshaler; `marshal_json()` → `json.Marshaler` (the bytes are
  validated and reformatted with the encoder's escaping); `marshal_text()` →
  `encoding.TextMarshaler`; otherwise by `kind()`: `Map` (`map_keys`/`map_get`),
  `Slice` (`list`), `Func` (UnsupportedTypeError), `Ptr`/`Struct`/`Interface`
  (`struct_fields`, fields of concrete type without tags).
- Marshaler error types are the receiver's pointer type (Go uses
  `reflect.TypeOf(va.Addr().Interface())`): `"*" + type_name()` unless the
  type name already starts with `*`.

Hosts must report `marshal_json`/`marshal_text` only when Go would call the
method. Under `CallMethodsWithLegacySemantics`, Go skips pointer-receiver
methods on values that are not addressable (for example a struct value held
in an interface).

## Deliberate deviations (none change output)

1. **Recursion.** Go goroutine stacks grow, so Go's recursive arshalers and
   `reformatValue` handle 10000-deep values. In the port,
   `jsontext::decode::consume_value` (validation, generic over the reader)
   is iterative with an explicit frame stack. The recursive marshal,
   unmarshal and reformat functions call `stack::guard`, which continues the
   recursion on a scoped helper thread (8 MiB stack) once about 256 KiB of
   stack has been used. `tests/deep.rs` runs 10000-deep inputs on a 2 MiB
   thread in debug builds.
2. **Duplicate-name namespaces** (`objectNamespaceStack`) are not ported.
   Every v1 entry point sets `AllowDuplicateNames`, which disables them. The
   related `!AllowDuplicateNames` branches are omitted and commented.
3. **JSON Pointers of `SyntacticError`s** (`pointerSuffixError`, and the
   pointer from `AppendStackPointer` in `wrapSyntacticError`) are not
   tracked. `transformSyntacticError` drops them. The decoder's
   `objectNameStack` *is* ported, because `SemanticError` pointers become
   `UnmarshalTypeError.Field`.
4. **Encoder-side object names** (`Names` on `encoderState`) are not
   tracked. They only feed the JSON Pointer of marshal errors, which
   `transformMarshalError` discards.
5. **Buffered encoders only.** The v1 `Encoder` marshals into a buffered
   `jsontext.Encoder` and writes the bytes itself. The writer, flushing and
   `bufStats` of `jsontext.Encoder` are not ported.
6. **`Decoder` buffer.** Go reads into `buf[len:cap]`. The port tracks
   Go's `cap` (`buf_cap`) and the exact growth policy, and hands the reader a
   scratch slice of exactly `cap-len` bytes. The same amount is therefore read
   ahead (`Buffered()` matches Go), and the buffer is not zeroed on every
   read. `Ok(0)` from a Rust reader is `io.EOF`, and `Interrupted` is retried.
7. **`SemanticError.Error()`** always says "cannot". Go picks "cannot" or
   "unable to" per process by ranging over a map. The v1 API only shows this
   text through host errors that embed a SemanticError.
8. **String interning** (`makeString`/`stringCache`) and **cycle
   detection** (`visitPointer`) are not ported. Interning does not affect
   output, and `Value` trees cannot be cyclic.
9. **`appendIndent` placeholder fix-up** (`v2_indent.go:99-111`). When the
   prefix or indent holds a byte other than space or tab, Go formats with
   placeholder spaces. A deferred function then walks the bytes it appended
   and, after each `'\n'`, overwrites the run of spaces with the prefix and
   then with copies of the indent. With an empty indent, the loop at line
   106, `for len(spaces) > 0 { spaces = spaces[copy(spaces, invalidIndent):] }`,
   never ends once a run is longer than the prefix. The bytes it walks are:
   - on success, the formatted value plus the trailing whitespace of `src`,
     for example `Indent(dst, "1\n  ", "a", "")`;
   - on a syntax error, `src` itself. `jsontext.AppendFormat` returns
     `append(dst, src...)` with the error, and the deferred function runs
     although `appendIndent` then returns `dst[:dstLen]`. So any invalid
     `src` with a `'\n'` followed by more spaces than the prefix is long
     never returns, for example `Indent(dst, "[\n  1,]", "a", "")`.

   The port returns what Go computes before the fix-up: the syntax error
   with `dst` unchanged (Go returns exactly that for the same call with the
   prefix replaced by spaces), or the output with the fill stopped after the
   prefix (`1\na `). `MarshalIndent` and `Encoder.SetIndent` never get
   there, because marshal output is valid and has no trailing whitespace.
   `tests/redteam.rs` (`indent_inputs_that_hang_go_terminate`) pins both
   cases.
10. **Unmarshal targets** are `*any` and `*map[string]any` (Hugo's only
    targets). Interface targets always start as nil, so
    `MergeWithLegacySemantics` on interfaces is a no-op.

## FMA sites

None. `go tool objdump -s encoding/json` of the oracle binary (go1.27.1,
darwin/arm64) covers 383 functions in `encoding/json`, `…/v2`,
`…/jsontext` and `…/internal/*` and contains no
`FMADD`/`FMSUB`/`FNMADD`/`FNMSUB`. Float text comes from `go-strconv`
(`AppendFloat`). This crate only compares floats and takes absolute values.

## Known gaps

- No decoding into Go structs, typed slices or typed maps. Hosts decode into
  `Value` and map fields themselves. `fields.go` (tag parsing, embedding,
  case-insensitive matching) is not ported. Hosts give `JsonStruct` the
  resolved names and options.
- Not ported: `format` struct tag options, `time.Duration`, `Marshalers`/
  `Unmarshalers` options, `Token.Int`/`Token.Uint`, and `jsontext.Value`
  canonicalization/reordering. v1 never reaches them.
- Host objects of kind `Ptr`/`Struct`/`Interface` without `struct_fields`
  (and without `marshal_json`/`marshal_text`) produce
  `json: unsupported type: <type>`. Go would reflect over their fields.
- `Object` maps (`Kind::Map`) are assumed to have string keys. A host with
  integer or `TextMarshaler` keys must return them formatted the way Go
  writes them (decimal, `MarshalText`). The port then sorts them bytewise,
  as Go sorts the written names.
- `Value::Map` and `Value::List` of named types carry no methods, apart
  from `maps.Params.IsZero`. A named map or slice type with `MarshalJSON`,
  `MarshalText` or `IsZero` must be an `Object`. For example, `exif.Tags`
  (`map[string]any`) has a `MarshalJSON` that uses Hugo's typed-map codec.
- `omitzero` on a `JsonStruct` field without `Object::is_zero` checks the
  listed fields only. Go's `reflect.Value.IsZero` also looks at unexported
  and `json:"-"` fields.
- Host methods (`marshal_json`, `marshal_text`, `map_get`, `map_keys`,
  `list`, `struct_fields`, `is_zero`) run on a `stack::guard` helper thread
  once the nesting has used about 256 KiB of stack since the first go-json
  frame on the thread. So they must not depend on thread-locals. The guard
  only counts the stack used since that frame, so a caller must still leave
  some stack for the first 256 KiB.
- Dropping a `Value` nested thousands of levels deep recurses in Rust's
  drop glue. This is the caller's concern: `tests/deep.rs` has an
  iterative `drop_deep` helper.

## Tests and verification

- `tests/oracle.rs` replays fixtures written by `tools/go-oracle/go-json`
  (`go run ./tools/go-oracle/go-json -mode <mode> -out …`). Every
  comparison is byte-exact and includes error kind, message and offset:
  - `encode.rec.gz` (seed 1): 3,846 values: every float64/float32 special,
    ±each, all 256 single bytes, adversarial strings, and 3,000 random nested
    values with all `Value` kinds, marshalers, text marshalers, plain structs
    and tagged structs (omitempty/omitzero/string, interface- and
    concretely-typed fields), NaN/Inf, bad times and bad Numbers. Each value
    is checked for `Marshal`, `Encoder` without HTML escaping (and
    `marshal_with(v, false)`), `MarshalIndent` and `Encoder` with
    `SetIndent`, using 8 prefix/indent pairs including invalid characters.
  - `text.rec.gz` (seed 2): 3,152 JSON texts: hand-written edge cases, random
    valid documents, streams of several values and random mutations. Each
    text is checked for `Valid`, `Compact`, `Marshal(RawMessage)`, `Indent`,
    `HTMLEscape`, `Unmarshal` into `any` and into `map[string]any` (with the
    partial value on error), a `Decoder.Decode` stream (random chunked
    reader, `UseNumber`, `InputOffset`, `Buffered`), a `Token`+`More` stream,
    and a random interleaving of `Token`/`Decode`/`More`/`InputOffset`.
  - `real.rec.gz`: 55 real inputs: the golden `index.json` (364 KB), 51
    YouTube API responses from the Hugo file cache (HTTP headers stripped),
    and the 3 seeksnack `data/**/*.json`. Each input is checked for
    `Unmarshal` (any/map), `Marshal`, `MarshalIndent`, `Encoder` (no HTML
    escaping, 2-space indent, as in hugo_stats.json), `Encoder` (HTML
    escaping, as in jsonify), `Indent`, `Compact`, and streaming reads with
    chunk sizes 1, 7, 100, 4096 and 1 MiB.
  - `golden_index_json_roundtrip`: decoding the golden `index.json` and
    re-encoding it the way `jsonify` does gives identical bytes.
  - Small slices of the red-team modes (see below), in the same two record
    formats: `hugoencode.rec.gz` (2,000 Hugo-shaped values, the
    `maps.Params` regression), `advencode.rec.gz` (764 values),
    `exhaustencode.rec.gz` (75 batches of up to 256 runes/strings),
    `exhausttext.rec.gz` (3,702 enumerated inputs), `numtext.rec.gz` (400),
    `bigtext.rec.gz` (10 documents up to 400 KB) and `vartext.rec.gz` (100).
  - With `GO_JSON_FIXTURES=<dir>`, each test reads its file from `<dir>`
    and is skipped when `<dir>` does not have it.
- `tests/redteam.rs`: the second red-team's regressions, with the go1.27.1
  output they quote: `maps.Params` under `omitzero`, the `Indent` inputs
  that never return in Go, and the marshal depth limit.
- `tests/go_tables.rs` ports Go's `TestValid`, `TestCompactAndIndent`,
  `TestCompactSeparators`, `TestIndentErrors`, `TestEncoder`,
  `TestEncoderIndent`, `TestEncoderSetEscapeHTML`, `TestHTMLEscape`,
  `TestEncodeString` and `TestDecodeInStream`.
- `tests/deep.rs`: 10000-deep arrays and objects through
  `unmarshal`/`marshal`/`marshal_indent`/`compact`/`Decoder` on a 2 MiB
  thread; 10001 levels gives Go's "exceeded max depth" error at offset
  10001.

## Red-team, second pass (2026-09-27)

Oracle: go1.27.1 on linux/amd64, and linux/arm64 under
`qemu-aarch64-static` for a subset. Every corpus below was replayed with
`GO_JSON_FIXTURES=<dir> cargo test --release --test oracle -- --exact
{text,encode}_fixture` (the corpus is named `text.rec.gz` or
`encode.rec.gz` in `<dir>`).

### The `-mode advtext` hang

Go's `json.Indent` itself never returns on some inputs (deviation 9 above).
`adv.go` already avoided the trailing-whitespace case, but it did not know
that on a syntax error the placeholder fix-up walks `src`. About one
`advtext` input in 20,000 was an invalid document with a `'\n'` followed by
spaces, drawn together with a non-space prefix and an empty indent, so most
large runs hung (goroutine dump: `appendIndent.func1`,
`v2_indent.go:106`, called from the error return at line 123).

`indentHangs` now predicts the hang exactly: it runs the same `Indent` with
the placeholder spaces (which takes no fix-up path) and walks its output, or
`src` when that call fails, looking for a `'\n'` followed by more spaces than
the prefix is long. Checked against Go on 300,000 targeted inputs: all
200,095 accepted pairs returned, and 40 of 40 sampled rejected pairs hung.
`pickAdvIndent` draws another pair when the predicate is true (about one
pair per 20,000 cases), and the oracle logs how many it re-drew. As a safety
net, `textCase` calls `Indent` through `indentWatched`, which stops the
oracle with the arguments if a call has not returned after a minute.

### Modes

| mode | record format | inputs |
|---|---|---|
| `advtext`, `advencode` | text, encode | the first red-team's adversarial generators (`adv.go`) |
| `exhausttext` | text | every 1- and 2-byte input; every 3-byte input over 48 bytes; every 1- and 2-byte string body, `\X` escape (also in names) and `\uXXXX` unit; every third unit after 3 high surrogates; 3/4-byte UTF-8 with 12 continuation bytes; every number-like string up to 4 bytes over `0-9-+.eE` and 5 bytes over `01-+.eE9`; every byte after each prefix of null/true/false (473,829 inputs; `-parts`/`-part` split them) |
| `exhaustencode` | encode | every rune U+0000–U+10FFFF (surrogates as their 3-byte encodings), every 1- and 2-byte string, as `[]string` values and `map[string]any` keys, 256 per case, and every 2-byte `template.HTML` (9,474 cases) |
| `numtext` | text | numbers with up to 20,000 mantissa digits, runs of zeros/nines, halfway patterns, long fractions, exponents with leading zeros and around strconv's 10,000 cap, alone, in arrays and objects, and in pairs |
| `bigtext` | text | 4–400 KB documents (arrays of adversarial documents, long strings with escapes, many long names, deep nesting around long values, truncations and mutations), streamed with chunk sizes around 64, 4096 and 8192 |
| `vartext` | text | `advtext` inputs and some `bigtext` documents streamed through readers whose read sizes follow a random cycle (a third field of `streamargs`) |
| `hugoencode` | encode | `maps.Params` (with and without `_merge`), dicts, `[]any`, index.json-style `[]map[string]any`, page text and `template.HTML`, dates, and structs with `maps.Params`/`any` fields under `omitzero`/`omitempty` |

`hugo.go` imports `common/maps`, so `maps.Params` is the real Hugo type
(vdump `mP`).

### Runs

All runs reported 0 mismatches after the fix below. Seeds are `-seed`
values (for the exhaust modes, the part numbers).

| mode | seeds × cases | total |
|---|---|---|
| advtext | 101–120 × 100,000 (+ hand texts and truncations); 1, 2, 3, 5 × 20,000; 1000 × 2,000 | 2,102,858 |
| advencode | 201–226 × 100,000 (+264 float batches each) | 2,606,864 |
| text | 401–410 × 100,000 (+152 hand texts each) | 1,001,520 |
| encode | 501–510 × 100,000 (+846 specials each) | 1,008,460 |
| exhausttext | parts 0–7 of 8: the whole enumeration | 473,829 |
| exhaustencode | parts 0–1 of 2: the whole enumeration | 9,474 (2.4M strings) |
| numtext | 601 × 2,000; 602–611 × 100,000 | 1,002,000 |
| bigtext | 701 × 200; 702–711 × 5,000 | 50,200 |
| vartext | 1001 × 5,000; 1002–1011 × 100,000 | 1,005,000 |
| hugoencode | 801 × 20,000; 802–811 × 100,000 | 1,020,000 |
| arm64 (qemu) | advencode 901 × 100,000; numtext 902 × 50,000; advtext 903 × 20,000 | 170,966 |

That is 10,451,171 cases. An earlier run of the text enumeration, whose
3-byte alphabet listed `u` twice, added 480,886 more, also with 0
mismatches.

### Bug found

`maps.Params` under `omitzero`. Go's `omitzero` calls a field type's
`IsZero` method (`v2/fields.go:219-236`), and Hugo's `maps.Params` has one:
empty, or `_merge` as the only key, is zero. `arshal::default::is_zero`
treated every non-nil map as non-zero, so

```go
type S struct{ Z maps.Params `json:"z,omitzero"` }
json.Marshal(S{maps.Params{"_merge": "deep"}}) // Go: {}   port: {"z":{"_merge":"deep"}}
```

(same for `maps.Params{}`). Fixed in `src/arshal/default.rs` (`is_zero`,
`params_is_zero`, the same logic as `gotemplate/src/text/hugo.rs`). No Hugo
struct uses `omitzero` today, so the golden site did not show it.
Regressions: `tests/redteam.rs` `params_omitzero_calls_is_zero` and
`hugoencode.rec.gz` (without the fix, 3,104 of the 20,000 cases of seed 801
differ).

### Coverage

`llvm-cov` line coverage of `src/` over the first pass's tests and fixtures plus 30,000
`advtext`, 30,000 `advencode`, a quarter of each enumeration, 20,000 `text`
and 20,000 `encode` cases: 85.7%. The lines not reached are v2-only options
(`SpaceAfterComma`, `Multiline` in the struct arshaler, number
canonicalization, `ReformatString`'s re-quoting path, UTF-16 escapes), the
`!ReportErrorsWithLegacySemantics` branches, `SemanticError.Error` and
`SyntacticError.Error` (the v1 layer rewrites both), `unmarshal_number`
paths other than a plain number, and host `Object`s of `Kind::Map`, `Slice`
and `Ptr`. Probes compared with Go by hand: JSON Pointer escaping of `~`
and `/` in `UnmarshalTypeError.Field` (`{"a/~b":{"~0":2e400}}` gives
`.a~1~0b.~00`), `Decoder::decode_map` over streams with nulls, arrays,
overflows and truncation, and `Marshal`/`MarshalIndent`/`Encoder` at
depths 9999–10001 (arrays, maps, `RawMessage`, `MarshalJSON` output,
empty maps/slices/structs). All matched.

## Regenerating the fixtures

From the repository root (go1.27.1, default GOEXPERIMENT):

```sh
go run ./tools/go-oracle/go-json -mode encode -n 3000 -seed 1 -out crates/go-json/tests/fixtures/encode.rec.gz
go run ./tools/go-oracle/go-json -mode text   -n 3000 -seed 2 -out crates/go-json/tests/fixtures/text.rec.gz
# real inputs, staged under names the tests recognise:
#   golden-index.json      <- golden/canonical/index.json
#   youtube-<cache key>    <- ~/Library/Caches/hugo_cache/seeksnack/filecache/getresource/*
#   data-<path with _>     <- pristine-seeksnack/data/**/*.json
go run ./tools/go-oracle/go-json -mode real -out crates/go-json/tests/fixtures/real.rec.gz <staged files>
go run ./tools/go-oracle/go-json -mode hugoencode    -n 2000 -seed 21 -out crates/go-json/tests/fixtures/hugoencode.rec.gz
go run ./tools/go-oracle/go-json -mode numtext       -n 400  -seed 22 -out crates/go-json/tests/fixtures/numtext.rec.gz
go run ./tools/go-oracle/go-json -mode exhausttext   -parts 128 -part 0 -seed 23 -out crates/go-json/tests/fixtures/exhausttext.rec.gz
go run ./tools/go-oracle/go-json -mode advencode     -n 500  -seed 25 -out crates/go-json/tests/fixtures/advencode.rec.gz
go run ./tools/go-oracle/go-json -mode bigtext       -n 10   -seed 26 -out crates/go-json/tests/fixtures/bigtext.rec.gz
go run ./tools/go-oracle/go-json -mode exhaustencode -parts 128 -part 0 -seed 27 -out crates/go-json/tests/fixtures/exhaustencode.rec.gz
go run ./tools/go-oracle/go-json -mode vartext       -n 100  -seed 28 -out crates/go-json/tests/fixtures/vartext.rec.gz
```

`real.rec.gz` can also be rebuilt from itself: write each case's `input`
record to a file named by its `name` record and pass the files in the order
of the cases. With the oracle at this commit, all ten files regenerate byte
for byte on linux/amd64 and on linux/arm64 (qemu). The generators use no
platform-dependent float arithmetic.

Large corpora go outside the repository, e.g.
`-mode advtext -n 100000 -seed 101 -out <dir>/text.rec.gz`, then
`GO_JSON_FIXTURES=<dir> cargo test --release --test oracle -- --exact text_fixture`
(`encode_fixture` for the encode formats). The first pass ran seeds 11
(encode) and 12 (text) with `-n 100000`: 100,846 and 100,152 cases, 0
mismatches.
