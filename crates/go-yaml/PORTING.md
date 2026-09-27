# go-yaml — porting notes

Port of **gopkg.in/yaml.v2 v2.4.0** *decoding* (the YAML library neohugo's
`parser/metadecoders` uses for front matter, config, data files and
`transform.Unmarshal`; `langs/i18n` registers `yaml.Unmarshal` with go-i18n)
plus neohugo's YAML post-processing (`stringifyMapKeys`) and the conversion to
`go_value::Value`.

Verified: `parser/metadecoders/decoder.go` imports `yaml "gopkg.in/yaml.v2"`
and go.mod pins `gopkg.in/yaml.v2 v2.4.0` (yaml.v3 / goccy are only indirect
dependencies of other packages).

## Approach: full port, not unsafe-libyaml

yaml.v2 carries its own Go translation of libyaml (scannerc.go, parserc.go,
readerc.go) with Go-specific changes that are observable:

- the simple-key bookkeeping rewritten around `simple_keys_by_tok` and the
  1024-character / same-line rule,
- `max_flow_level` / `max_indents` limits ("exceeded max depth of 10000"),
- `yaml_parser_update_buffer` padding, the 512-byte raw / 1536-byte decoded
  buffers (reader errors surface lazily per 512-byte chunk; `is_bom` checks
  the *start of the buffer*, not the current position),
- the decoder's alias-expansion limit ("document contains excessive aliasing"),
- the Go `fail()` line-number convention (scanner errors +1, parser errors
  report the 0-based problem line, context line fallback).

unsafe-libyaml is a port of C libyaml 0.2.5, which differs in several of these
points, and using it would need `unsafe` outside a `*-sys` crate. The whole Go
pipeline is therefore ported line by line (~3,800 Go lines → ~3,900 Rust
lines), with Go function names and control flow kept.

## File map

| Go (yaml.v2@v2.4.0) | Rust |
|---|---|
| yamlh.go (parser types, token/event/state enums, tag constants), yamlprivateh.go (buffer sizes, character classes) | `src/yamlh.rs` |
| apic.go: `yaml_insert_token`, `yaml_parser_initialize`, `yaml_parser_set_input_string`, `yaml_string_read_handler` | `src/yamlh.rs`, `src/readerc.rs` |
| readerc.go | `src/readerc.rs` |
| scannerc.go | `src/scannerc.rs` |
| parserc.go | `src/parserc.rs` |
| decode.go (parser → node tree, decoder) + yaml.go (`unmarshal`, `fail`/`failf`, `TypeError`) | `src/decode.rs`, `src/lib.rs` |
| resolve.go | `src/resolve.rs` |
| encode.go, emitterc.go, writerc.go, sorter.go | not ported (decoding only) |
| neohugo parser/metadecoders/decoder.go: `UnmarshalToMap`, `Unmarshal`, YAML branch of `UnmarshalTo`, `stringifyMapKeys`; spf13/cast v1.9.2 `ToStringE` (key types yaml.v2 produces) | `src/metadecoders.rs` |
| time.Parse for the 4 layouts of `allowedTimestampFormats` (go1.27.1 time/format.go `parse`, `getnum`, `skip`, `atoi`, `parseNanoseconds`, `daysIn`) | `src/gostd.rs` |
| encoding/base64 `StdEncoding.DecodeString` (`decodeQuantum`) | `src/gostd.rs` |
| strconv ParseInt/ParseUint/ParseFloat/FormatFloat/Quote | `go-strconv` crate (thin wrappers in `src/gostd.rs`) |

## Public API

```rust
// yaml.Unmarshal(data, &v) with v interface{} (first document; empty → Nil)
go_yaml::unmarshal(data: &[u8]) -> Result<Yaml, Error>
go_yaml::unmarshal_with(data, strict: bool) -> Result<Yaml, Error>   // UnmarshalStrict
// m := make(map[string]interface{}); yaml.Unmarshal(data, &m)  (None = nil map after `null`)
go_yaml::unmarshal_str_map(data) -> Result<Option<StrMap>, Error>
// yaml.NewDecoder(bytes.NewReader(data)).Decode(&v) repeatedly (multi-document)
go_yaml::Decoder::new(data).decode() -> Option<Result<Yaml, Error>>

enum Yaml { Nil, Bool, Int(i64) /* Go int */, Uint64(u64), Float64(f64), String(Vec<u8>), Seq(Vec<Yaml>), Map(IfaceMap) }
IfaceMap  // map[interface{}]interface{}: insert/get with Go key equality, iter() in document order
StrMap    // map[string]interface{}: BTreeMap order
Error     // message_bytes() == Go err.Error() byte for byte; kind(): Fatal | Type(messages)

// neohugo parser/metadecoders, YAML format:
go_yaml::metadecoders::unmarshal_to_map(data) -> Result<Option<go_value::Map>, MetaError> // UnmarshalToMap (front matter, config)
go_yaml::metadecoders::unmarshal(data) -> Result<go_value::Value, MetaError>              // Unmarshal (data files, transform.Unmarshal)
go_yaml::metadecoders::to_value(&Yaml) -> go_value::Value                                  // stringifyMapKeys + conversion
go_yaml::metadecoders::cast_to_string(&Yaml) -> Vec<u8>                                    // cast.ToStringE for keys
```

Value mapping to `go_value`: `map[string]interface{}` → `Value::Map(MapType::StringAny)`,
`[]interface{}` → `Value::List(SliceType::Any)`, `int` → `Value::Int(_, IntKind::Int)`,
`uint64` → `Value::Uint(_, UintKind::Uint64)`, `float64` → `Value::Float(_, F64)`,
`bool`, `string` → `Value::String`, nil → `Value::Invalid`.

Semantics worth knowing downstream (all as in Go):

- `UnmarshalToMap` decodes the top level straight into `map[string]interface{}`,
  so top-level keys keep their source text (`true: x` → key `"true"`,
  `0x10: x` → `"0x10"`, `1.50: x` → `"1.50"`), while nested map keys go
  through `cast.ToStringE` of the resolved value (`0x10` → `"16"`, `1.50` →
  `"1.5"`, `~` → `""`).
- Timestamps decoded into `interface{}` stay strings; integers are Go `int`
  (never `int64`), `uint64` only above MaxInt64; `08` is the float 8.
- A `null` document gives a nil map (`Ok(None)`) from `UnmarshalToMap`, an
  empty document an empty map; `Unmarshal` of empty input gives an empty map.
- Merge keys: a later `<<` overrides earlier explicit keys, sequence merges
  are applied last-to-first. Duplicate keys: last wins (non-strict).
- `.nan` has Go's `math.NaN()` bits `0x7FF8000000000001` (`GO_NAN_BITS`),
  which matters when a value is hashed.
- `MetaError` renders `failed to unmarshal YAML: <yaml error>`; the herrors
  `toFileError` decoration (file name, position) is left to the herrors port.

## Deliberate deviations

- **Node tree construction is iterative** (decode.go's `parser.parse` is
  recursive). Events are consumed in the same order and anchor registration,
  alias resolution ("unknown anchor"), `expect` checks happen at the same
  points, so results and errors are identical; the change avoids native stack
  overflow on documents nested 10,000+ levels.
- **Deep documents decode on a big-stack thread** (`with_stack`). The
  decision uses `NodeParser::decode_depth`, the decoder's recursion depth
  *including alias expansion* (an alias re-enters its anchored subtree, so
  a chain of 60 anchors that each nest 190 levels and alias the previous
  one recurses 11,400 levels although the tree is only 190 deep); above 200
  levels the decoder runs on a thread with `64 MB + 64 KB × depth` of stack,
  capped at 1 GB (Go's goroutine stack limit). Go's goroutine stacks grow;
  the decoder stays recursive as in Go. `Yaml` drops iteratively.
  `Yaml::clone`, `metadecoders::to_value` (run inside the big-stack thread)
  and `go_value::Value`'s own `Drop` are recursive: a caller that keeps a
  `go_value::Value` nested ~10,000+ levels deep needs a large stack to drop
  it (go-value crate; measured: an 11,400-deep value drops fine on a 2 MB
  stack in release builds but not in debug builds).
- **Only the decode targets neohugo uses are supported**: `interface{}`,
  `map[string]interface{}` (top level) and `string` (its keys). Struct
  targets, `Unmarshaler`/`TextUnmarshaler`, `MapSlice` are not ported
  (`prepare` is therefore a no-op).
- `peek_token` copies the scalar fields of the head token and the byte
  fields are taken from the queue right before `skip_token` (Go returns a
  pointer into the queue). In `yaml_parser_parse_flow_sequence_entry_mapping_value`
  Go keeps using a pointer to the (skipped) VALUE token, which Go's queue
  compaction can overwrite in rare cases; the Rust port uses the VALUE token's
  mark. This can only change the line number of a TypeError on the implicit
  empty value of `[a: ]`-style entries, which cannot produce a TypeError for
  the supported targets.
- `yaml_insert_token` compacts the queue on a different schedule than Go's
  `len == cap` rule (no observable effect: positions are head-relative).
- Buffer reads past Go's `len(buffer)` return stale bytes instead of
  panicking (Go would crash; yaml.v2 v2.4.0 never does it — its fuzz-crasher
  test cases are part of the tests).
- Map-key collisions after `stringifyMapKeys` (`1` and `"1"`, `1` and `1.0`,
  `true` and `"true"` in the same nested map): Go's winner depends on random
  map iteration order (measured: last-in-document wins ~7/8 of runs). The
  port always lets the later entry win. The oracle marks such cases `nondet`.
- `%#v` in "invalid map key" / strict-mode messages: keys of *different*
  dynamic types are ordered by Go's fmtsort by type-descriptor address, which
  depends on the linker; the port uses the order observed in go1.27.1
  darwin/arm64 binaries (nil, string, uint64, int, float64, bool). The
  differential test accepts a permutation there.

## FMA sites

- decode.go:346 `allowedAliasRatio`: `0.99 - 0.89*(float64(decodeCount-low)/range)`
  compiles to `FMSUBD` in go1.27.1 darwin/arm64 (checked with
  `go tool objdump` of the oracle binary *and* of `bin/neohugo-go`, where it
  is inlined into `(*decoder).unmarshal`). Ported as
  `(-0.89f64).mul_add(x, 0.99)`. No other float arithmetic.

## Tests and evidence

Oracle: `tools/go-oracle/go-yaml` (`corpus` builds corpora, `run` decodes them
with the real yaml.v2 and metadecoders, `time` times files). Each case records
four results: yaml.Unmarshal into `interface{}`, into `map[string]interface{}`,
`metadecoders.UnmarshalToMap(YAML)` and `metadecoders.Unmarshal(YAML)`, as a
canonical typed dump (float bits, byte-escaped strings, sorted maps) or the
exact error message.

Checked-in (gzip, `tests/fixtures`, ~1 MB): yaml.v2 decode/encode test-table
strings + scaled limit tests (499 cases), all 222 seeksnack YAML front
matters/`*.yml` files except `static/admin/config.yml` (anchor-heavy, 13 MB
dump; run from scratch), 7,394 adversarial cases (YAML 1.1 scalars × 29
contexts, tags, anchors, merges, directives, encodings, BOMs, buffer
boundaries, long keys), 10,000 fuzz cases. Full scratch corpora
(`GO_YAML_FIXTURE_DIR`): 68,116 + 300,000 (200k mutation fuzz + 100k
grammar-generated) cases — **all identical** (368,116 cases × 4 decoders).
`tests/limits.rs` ports limit_test.go at full size (1 MB inputs), the
unmarshalErrorTests messages, fuzz crashers and decoderTests.

### Independent verification (round 2)

An adversarial re-verification added `go-yaml verify` (tools/go-oracle/go-yaml/verify.go),
nine generators aimed at what the first-round corpora covered thinly:
`maps` (maps of 1–30 entries whose keys collide under Go map equality:
`1`/`"1"`/`1.0`/`0x1`/`01`, `0.0`/`-0.0`, several `.nan`, `~`/`null`/`""`,
YAML 1.1 bools, `!!binary`, timestamps, plus `<<` merges of maps and
sequences), `numbers` (random numerals with signs, base prefixes, leading
zeros, underscores, exponents −350…350, int64/uint64 bounds, hex floats,
under `!!int`/`!!float` too), `floatkeys` (random float bit patterns as
nested keys → `cast.ToStringE`/`FormatFloat('f', -1)`), `timestamps`
(random dates/times/fractions/zones around every `time.Parse` rule),
`unicode` (NEL/LS/PS/BOM/C1/surrogates/noncharacters in every scalar
style and in keys, all escapes), `blocks` (indentation/chomping indicators,
tabs, blank lines, document markers inside block scalars), `soup` (random
token sequences), `anchors` (anchors/aliases/merges of anchors, aliases as
keys) and `encodings` (UTF-8/UTF-16LE/BE with and without BOM, lone
surrogates, invalid bytes around the 512/1536-byte reader buffers, simple
keys of 1015–1034 multi-byte characters).

Results (all four decoders per case): verify generators 160,000 (8 × 20,000,
default seed) + 800,000 (8 × 100,000, seed 777) + 30,000 `encodings`
(seed 99); a second first-round corpus with new seeds (300,000
mutation-fuzz + 300,000 grammar-generated, seed 12345); the full
seeksnack corpus including `static/admin/config.yml` (223), adversarial
(7,394) and yaml.v2 test tables (499) — **1,598,116 cases, 0 mismatches
after the fixes below**. Checked in: `tests/fixtures/verify.fixture.gz` (7,207 cases:
800 per generator + regressions), `tests/limits.rs`
(`alias_chain_on_small_stack`, `alias_cycle_and_excess`) and the unit test
`tests::iface_map_index_after_nan_key`.

Bugs found and fixed in this round:

1. `IfaceMap::insert` (Go map key equality): when the 9th entry — the one
   that switches lookups to the hash index — had a NaN key, the index was
   never built, so later duplicates of the first 8 keys were appended
   instead of replacing them (Go `m[k] = v` replaces). Example:
   `{a: 1, b: 2, c: 3, d: 4, e: 5, f: 6, g: 7, h: 8, .nan: 9, a: 10}` gave
   two `a` entries (Go: one, `a: 10`); in strict mode duplicates were not
   reported. Fixed by building the index whenever the entry count crosses
   the threshold.
2. Stack overflow (process abort) on alias chains: `with_stack` only
   looked at the parsed nesting depth, so a 23 KB document
   (`a1: &a1 [[…190…*a0…]]`, … 60 anchors) recursed 11,400 levels on the
   caller's thread and aborted on a 2 MB thread (Go decodes it). Fixed with
   `decode_depth` (above).

## Known gaps

- Encoding (yaml.Marshal) is not ported (neohugo uses it only in
  `parser/frontmatter.go` for `hugo new`/`convert`, not in builds).
- Struct/typed decode targets are not ported (see deviations).
- `Decoder` only reads from a byte slice (Go's `io.Reader` read errors are
  not modelled).
- `yaml.Decoder` after a parse error: Go panics on the next `Decode`
  ("attempted to parse unknown event"); the port returns that message as an
  error (neohugo does not use `yaml.Decoder`).
