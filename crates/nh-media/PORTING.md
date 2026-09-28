# nh-media — porting notes

neohugo media/* and output/* (media types, output formats). Owner and crate lead: Wave B task
T04 (config-base-media).

## Go file → Rust module

| Rust module | Go source(s) | Status / note |
|---|---|---|
| `media::builtin` | `media/builtin.go` | ported |
| `media::config` | `media/config.go` | ported |
| `media::media_type` | `media/mediaType.go` | ported |
| `media::sniff` | Go `net/http/sniff.go` (`DetectContentType`) | NEW: used by `FromContent` |
| `output::config` | `output/config.go` | ported |
| `output::output_format` | `output/outputFormat.go` | ported |

Every GO PORTING CHECKLIST entry is `OK`.

## Dependencies

- nh-*: nh-common, nh-config (`decode` = mapstructure, `ConfigNamespace`).
- Wave A: go-value, go-json, go-path, go-sort (Go's `sort.Sort`: pdqsort, so the order of
  `Formats` is Go's also where `Less` is not a strict weak order), go-strconv, go-unicode.
- crates.io: none. dev: `serde_json`, `flate2` (`rust_backend`), nh-parser
  (`ReplacingJSONMarshaller` for the config dump check), go-time (fixture support).

## Go behaviour reproduced on purpose

- The lookups that report ambiguity (`Types.GetBySuffix`, `GetByMainSubType`,
  `GetBySubType`, `GetByType` through `GetByMainSubType`, `Formats.GetBySuffix`,
  `FromFilename`) return Go's named results: the first match with `found == false`. The
  `*_found` methods return `(value, found)` like Go; the `Option` methods return `None`.
- `FromContent` with no extension hints returns the zero Type for every detected type (Go's
  `found` is still true from `GetByType` and the zero `mm` is not text).
- `DecodeTypes` and `DecodeContentTypes` wrap errors in `failed to decode media types: `;
  `output.DecodeConfig`'s decode errors are `failed to decode output format configuration: `
  plus mapstructure's text (`error decoding '': media type "x" not found` from the hook).
- `output.decode`'s hook replacing a `mediaType` string in a `map[string]string` input panics
  in Go (`reflect.Value.SetMapIndex: value of type media.Type is not assignable to type
  string`); the port panics with the same message.

## Deliberate deviations

1. **Map order.** Go iterates the config maps in random order: with several bad keys the error
   reported is random, `output.DecodeConfig` appends new formats in random order before an
   unstable sort (visible with negative weights, where `Formats.Less` is not a strict weak
   order), and `DecodeContentTypes` reports a random unknown key. The port uses byte order. The
   oracles run each decode 100 times and do not compare the cases whose results vary (`nondet`).
2. **Map aliasing.** Go's `output.decode` hook replaces the `mediaType` string in the caller's
   map in place (visible in the config after `DecodeConfig`); the port's hook changes a copy.
   No Hugo code reads the config map afterwards.
3. `ContentTypes` keeps its extension set in a `BTreeSet` (Go: a map), and `MediaType` fields
   are `String` (non-UTF-8 input is converted lossily), as in nh-config.

## Verification

| topic | inputs | Rust test | checks |
|---|---|---|---|
| `media/media.json.gz` | `media.Builtin`, `DefaultTypes`, `DefaultContentTypes`; `DecodeTypes` of 34 fixed inputs (seeksnack: no `[mediaTypes]`, via `GetStringMap` like allconfig; the `mediatypes` sections of `config-{en,th}.json`; `docs/hugo.toml`; custom types; case-mixed keys, `_merge`, invalid keys and values, typed maps, non-map inputs) and 1,500 seeded random inputs, directly and through a provider; `DecodeContentTypes` of 15 fixed × 2 type sets and 500 random inputs; the `Types` queries for 102 probes × 2 type sets; the `Type` methods; `FromString`/`FromStringAndExt` for 20 strings × 6 extension lists; `FromContent` for 96 contents × 20 hint lists × 2 type sets; `http.DetectContentType` for 2,921 byte strings | `tests/media.rs` | 2,471 cases compared (135 nondet): type dumps, SourceHash, the config dump (`MarshalJSON` and `hugo config` JSON with and without zero values), error texts |
| `output/output.json.gz` | the built-in formats and `DefaultFormats` (with `BaseFilename`, `IsZero`, `MarshalJSON`); `DecodeConfig` of 46 fixed inputs × 2 type sets (seeksnack: no `[outputFormats]`; the `outputformats` sections of `config-{en,th}.json`; `docs/hugo.toml`; overrides of built-in formats; zero, equal, negative, string and float weights; bad and case-mixed `mediaType`; typed maps, non-map inputs) and 1,500 seeded random inputs, directly and through a provider (`Get("outputformats")`); the `Formats` queries and `FromFilename` on the default and a custom set | `tests/output.rs` | 1,653 cases compared (60 nondet): format dumps in sort order, SourceHash, the config dump, error texts and panics |

Results: 0 mismatches. `tests/config_dump.rs` checks that the seeksnack build's `hugo config`
JSON of `mediatypes`, `contenttypes` and `outputformats` (no custom media types or output
formats, decoded through a provider like allconfig) equals the committed golden dumps
`config-en.json`, `config-th.json` and `config-en-printzero.json`.

Regenerate (platform independent):

```sh
export GOTOOLCHAIN=go1.27.1
go run ./tools/go-oracle/nh-media/media -root . -out crates/nh-media/tests/fixtures/media
go run ./tools/go-oracle/nh-media/output -root . -out crates/nh-media/tests/fixtures/output
```

The fixtures regenerate byte for byte.
