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
9. **`appendIndent` placeholder fix-up**: Go loops forever when the prefix
   contains a non-space/tab character, the indent is empty, and a trailing
   newline in `src` is followed by more spaces than the prefix length. The
   port stops.
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
- `Object` maps (`Kind::Map`) are assumed to have string keys.
- Dropping a `Value` nested thousands of levels deep recurses in Rust's
  drop glue. This is the caller's concern: `tests/deep.rs` has an
  iterative `drop_deep` helper.

## Tests and verification

- `tests/oracle.rs` replays fixtures written by `tools/go-oracle/go-json`
  (`go run ./tools/go-oracle/go-json -mode {encode|text|real} -out …`).
  Every comparison is byte-exact and includes error kind, message and offset:
  - `encode.rec.gz` (seed 1): 3,846 values: every float64/float32 special,
    ±each, all 256 single bytes, adversarial strings, and 3,000 random nested
    values with all `Value` kinds, marshalers, text marshalers, plain structs
    and tagged structs (omitempty/omitzero/string, interface- and
    concretely-typed fields), NaN/Inf, bad times and bad Numbers. Each value
    is checked for `Marshal`, `Encoder` without HTML escaping,
    `MarshalIndent` and `Encoder` with `SetIndent`, using 8 prefix/indent
    pairs including invalid characters.
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
  - Larger corpora, run outside the repository through
    `GO_JSON_FIXTURES=<dir> cargo test --release --test oracle`: 100,846
    encode cases (seed 11) and 100,152 text cases (seed 12), 0 mismatches.
- `tests/go_tables.rs` ports Go's `TestValid`, `TestCompactAndIndent`,
  `TestCompactSeparators`, `TestIndentErrors`, `TestEncoder`,
  `TestEncoderIndent`, `TestEncoderSetEscapeHTML`, `TestHTMLEscape`,
  `TestEncodeString` and `TestDecodeInStream`.
- `tests/deep.rs`: 10000-deep arrays and objects through
  `unmarshal`/`marshal`/`marshal_indent`/`compact`/`Decoder` on a 2 MiB
  thread; 10001 levels gives Go's "exceeded max depth" error at offset
  10001.

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
```

The large corpora used seeds 11 (encode) and 12 (text) with `-n 100000`.
Run them with `GO_JSON_FIXTURES=<dir with encode/text/real.rec.gz> cargo test --release --test oracle`.
