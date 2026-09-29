# nh-helpers — porting notes

neohugo helpers/*, source/*, cache/filecache, cache/httpcache (+ gohugoio/httpcache + net/http response-dump subset).

Every ported function carries a `// Go: <path>:<Func>` line; every checklist line in `src/` is `OK`
(148 entries, no `EX` left).

## Go file → Rust module

| Rust module | Go source(s) | Owner | Note |
|---|---|---|---|
| `content` | `helpers/content.go` | T08 helpers-source-cache |  |
| `general` | `helpers/general.go` | T08 helpers-source-cache | `TCPListen`, `PrintFs` are explicit-error stubs (not on the build path) |
| `path` | `helpers/path.go` | T08 helpers-source-cache | `GetCacheDir` has an env seam (`get_cache_dir_env`) |
| `pathspec` | `helpers/pathspec.go` | T08 helpers-source-cache |  |
| `processing_stats` | `helpers/processing_stats.go` | T08 helpers-source-cache | table rendered by `tablewriter` |
| `url` | `helpers/url.go` | T08 helpers-source-cache |  |
| `emoji` | `helpers/emoji.go` | T08 helpers-source-cache | STUB: explicit unsupported error (enableEmoji=false; Go executes 0 lines of it) |
| `source::file_info` | `source/fileInfo.go` | T08 helpers-source-cache | `FileObject` = the template-facing `*source.File` |
| `source::source_spec` | `source/sourceSpec.go` | T08 helpers-source-cache |  |
| `cache::filecache::filecache` | `cache/filecache/filecache.go` | T08 helpers-source-cache |  |
| `cache::filecache::filecache_config` | `cache/filecache/filecache_config.go` | T08 helpers-source-cache |  |
| `cache::httpcache::httpcache` | `cache/httpcache/httpcache.go` | T08 helpers-source-cache |  |
| `cache::httpcache::transport` | `github.com/gohugoio/httpcache@v0.8.0` `httpcache.go` | T08 helpers-source-cache | `Transport.RoundTrip` cached-response path; network and store paths return explicit errors |
| `cache::httpcache::http` | `net/http` `ReadResponse`/`readTransfer`/chunked + `net/textproto` `ReadMIMEHeader` | T08 helpers-source-cache | NEW: the subset needed to read `httputil.DumpResponse` bytes back (Go error texts) |
| `tablewriter` (+ `tablewriter::{runewidth,uniseg}`) | `github.com/olekukonko/tablewriter@v1.0.8`, `github.com/mattn/go-runewidth@v0.0.16`, `github.com/rivo/uniseg@v0.2.0` | T08 helpers-source-cache | NEW: only the configuration `ProcessingStatsTable` uses |

## Dependencies

- nh-*: nh-common, nh-config, nh-media, nh-hugofs, nh-markup; dev: nh-langs.
- Wave A: go-value, go-url, go-time, go-strconv, go-path, go-unicode.
- crates.io runtime: none. MD5 (`source.File.UniqueID`) comes through `nh_common::hashing`.
- crates.io dev-only: `serde_json` (fixture reader), `flate2` with `rust_backend` (gunzip of
  fixtures), `sha2` (sha256 of cached response bodies in the httpcache test).
- `src/tablewriter/runewidth_tables.rs` and `uniseg_tables.rs` are mechanical conversions of the
  Go tables (`go-runewidth` `runewidth_table.go` minus the unused `neutral`/`emoji` tables;
  `uniseg` `properties.go`). They are verified against Go by the `rune_width_matches_go` and
  `graphemes_match_go` unit tests (every code point, both EastAsian settings).

## Deliberate deviations

- **Go string bytes in the URL helpers.** `PathSpec::urlize_bytes`, `abs_url_bytes`,
  `rel_url_bytes`, `url_escape_bytes`, `make_path_bytes`, `make_path_sanitized_bytes` and
  `general::get_title_bytes_func`/`first_upper_bytes` take and return bytes (invalid UTF-8 as in
  Go); the `&str` functions are wrappers (`rel_url` panics when the decoded base path is not
  UTF-8: use `rel_url_bytes`). The tplfuncs `urls` and `strings.Title` functions use the byte
  versions (the 46 invalid-UTF-8 host cases now match Go).

- **Result-returning APIs.** Go functions returning `(T, error)` become `Result<T>`; where Go also
  returns a meaningful partial value together with a non-nil error, the Rust port returns only the
  error. The tests normalize Go results that carry an error to the error alone. Exceptions that keep
  the Go shape: `make_path_relative -> (String, Option<Error>)`, `extract_toc -> (Vec<u8>, Option<Vec<u8>>)`,
  `HttpCache::get -> (Option<Vec<u8>>, bool)`.
- **Panicking Go functions** (`URLEscape`, `IsAbsURL`) have a panicking form plus a `try_*`
  form. `MakePath`/`MakePathSanitized` never fail; their `try_*` forms are kept and always
  return `Ok`.
- **`removePathAccents`** (`RemovePathAccents: true`): `MakePath` calls nh-common's
  `text::remove_accents_string` (the port of `text.RemoveAccentsString` over x/text `norm`,
  `transform` and `runes`; nh-common deviations 27, 27a).
- **Map order.** Go's random map iteration shows up in `filecache.DecodeConfig` error choice when
  several cache entries are invalid; Rust iterates in byte order. The oracle marks those 23 cases
  `nondet` and the test only requires that Rust returns one of Go's possible errors.
- **`ContentSpec` content types.** Go reads `cfg.GetConfigSection("contentTypes")`. The Rust
  `ContentSpec::new` decodes the default content types; `ContentSpec::new_with_content_types`
  takes the decoded set explicitly (see requests, nh-config).
- **`httpcache.PollConfig.MarshalJSON`.** Go distinguishes nil vs empty glob slices; the Rust struct
  holds `Vec`s, so both marshal as Go's non-nil form. Hugo only marshals configs built by
  `DecodeConfig`, which never yields nil after defaults.
- **HTTP header values** are `Vec<u8>` (Go strings may hold invalid UTF-8). The declared-trailer map
  keeps Go's nil value slices (printed as `null`).
- **Trailer keys.** Go's `canonicalMIMEHeaderKey` mutates the key in place, so the error text for a
  malformed header line quotes the canonical key; reproduced on purpose.
- **Transport.** The inner `RoundTripper` defaults to `NoNetwork` (explicit error). The store path
  (a response that Go would write into the cache) returns an explicit unsupported error, since it can
  only be reached after a network fetch. The 304 merge, `stale-if-error`, `only-if-cached`, and the
  `Delete` when `!ShouldCache`/`!canStore` are ported.
- **`GetCacheDir`.** `get_cache_dir_env(fs, dir, &CacheDirEnv{getenv, is_test})` makes the
  environment (`NETLIFY`, `XDG_CACHE_HOME`, `HOME`, `TMPDIR`, `USER`) injectable for the oracle
  tests; `get_cache_dir` uses the process environment.
- **tablewriter** is ported only for the configuration Hugo's `ProcessingStatsTable` sets
  (MaxWidth 70, header auto-format + center, rows right-aligned, first column left, box-drawing
  separators). ANSI stripping, wrapping and width code follow v1.0.8. Column widths follow
  go-runewidth's `EastAsianWidth` read from `RUNEWIDTH_EASTASIAN`/locale at first use, as in Go.
- **No deprecation log** for the `Lang` accessor on `source.File` (logging belongs to the caller).

## Known gaps

- `emoji::emojify` returns an explicit unsupported error (enableEmoji=false on every site in scope).
- `general::tcp_listen` (server only), `general::print_fs` (debug helper) and `path::is_empty`
  (no caller on the build path) return explicit unsupported errors.
- httpcache `Transport`: no network, no store (see deviations). `source::FileObject.FileInfo` and
  `.Open` template methods return unsupported errors.

## Requests to other crates

- (Done) nh-hugofs' `OsFile` now returns Go's `read <path>: <errno>` `*fs.PathError`s itself;
  `filecache::read_file`'s reconstruction of it is kept for other files.
- **nh-config**: a way for `ContentSpec::new` to get the decoded content types from the config
  provider (an `as_any` downcast on the provider, or a `"contentTypes"` config section).
- **T09 / nh-allconfig**: Go's `LoadConfig` lets `HUGO_CACHEDIR` override the `cacheDir` config
  value (even when set to ""), then calls `helpers.GetCacheDir`; use `get_cache_dir`.
  `filecache::decode_config` now takes the fs as first argument (as Go). Register the `caches`
  and `httpCacheCompiled` sections.
- **T15 (resources / getresource) and other users**: the `Transport` and `Caches` API shape
  (`Caches::get`, `Cache::as_http_cache`, `Transport::new(cache)` with Go-named fields).
- **T20 (hugolib deps)**: `SourceSpec::new(ps, filter, fs)` and
  `ContentSpec::new_with_content_types(cfg, exec, logger, content_types)`.

## Verification

Oracles live in `tools/go-oracle/nh-helpers/<topic>/main.go` and call neohugo's real packages.
Fixtures are gzipped JSON in `tests/fixtures/<topic>/`; `cargo test` needs neither Go nor network.

| Topic | Inputs | Result |
|---|---|---|
| pathspec | docs/ + hugolib/testsite strings and paths, the architecture-core-data config dumps, adversarial strings, multihost and non-multihost; the `removePathAccents` setup also gets 33 accent strings (precomposed and decomposed accents, Greek, Cyrillic, Vietnamese, Hangul, Thai, Devanagari, Hebrew and Arabic marks, a CGJ, a 31-mark run, Zalgo text) and the non-ASCII corpus strings | 945,039 checks pass, 8,007 of them (`MakePath`, `MakePathSanitized`, `URLize`) through `removePathAccents` |
| general | corpus strings, byte counts, readers, ~400 ProcessingStats tables, every code point's rune width, grapheme splits | ~24k cases pass |
| srcfile | synthetic source trees on OsFs and MemMapFs | 2,378 File dumps, 9,696 IgnoreFile checks pass |
| httpcache | Go `DumpResponse` output, truncations and seeded mutations; Go-made cache entries | 6,066 ReadResponse reads, 2,480 RoundTrips pass (70 store-path cases are the expected unsupported error); config decode passes |
| filecache | cache configs, GetCacheDir environments, cache operations | 104 DecodeConfig (23 nondet), 120 GetCacheDir, 1,440 ops and NewCaches pass |
| go_tables | Go's own `_test.go` tables (URLize, AbsURL, RelURL, MakePath* (incl. the removeAccents rows), MakeTitle, ContentSpec, ExtractTOC, GlobMatcher, filecache) | 16 tests pass |

Regenerate (each must reproduce byte for byte):

```sh
export GOTOOLCHAIN=go1.27.1
go run ./tools/go-oracle/nh-helpers/pathspec
go run ./tools/go-oracle/nh-helpers/general    # general.json.gz + widths.json.gz
go run ./tools/go-oracle/nh-helpers/srcfile
go run ./tools/go-oracle/nh-helpers/httpcache
go run ./tools/go-oracle/nh-helpers/filecache
```
