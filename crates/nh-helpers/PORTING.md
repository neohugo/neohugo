# nh-helpers — porting notes

neohugo helpers/*, source/*, cache/filecache, cache/httpcache (+ gohugoio/httpcache + net/http response-dump subset).

## Go file → Rust module

| Rust module | Go source(s) | Owner | Note |
|---|---|---|---|
| `content` | `helpers/content.go` | T08 helpers-source-cache |  |
| `general` | `helpers/general.go` | T08 helpers-source-cache |  |
| `path` | `helpers/path.go` | T08 helpers-source-cache |  |
| `pathspec` | `helpers/pathspec.go` | T08 helpers-source-cache |  |
| `processing_stats` | `helpers/processing_stats.go` | T08 helpers-source-cache |  |
| `url` | `helpers/url.go` | T08 helpers-source-cache |  |
| `emoji` | `helpers/emoji.go` | T08 helpers-source-cache | STUB (enableEmoji=false) |
| `source::file_info` | `source/fileInfo.go` | T08 helpers-source-cache |  |
| `source::source_spec` | `source/sourceSpec.go` | T08 helpers-source-cache |  |
| `cache::filecache::filecache` | `cache/filecache/filecache.go` | T08 helpers-source-cache |  |
| `cache::filecache::filecache_config` | `cache/filecache/filecache_config.go` | T08 helpers-source-cache |  |
| `cache::httpcache::httpcache` | `cache/httpcache/httpcache.go` | T08 helpers-source-cache |  |
| `cache::httpcache::transport` | — | T08 helpers-source-cache | NEW: gohugoio/httpcache subset (cached-response read path, AlwaysUseCachedResponse) + Go http.ReadResponse / DumpResponse compatible parser |

## Dependencies

- nh-*: nh-common, nh-config, nh-media, nh-hugofs, nh-markup
- Wave A (to add when available): go-url, go-time, go-strconv
- crates.io (justify each): md-5 (source.File.UniqueID)

## Deliberate deviations

_Wave B: list every deviation from the Go code here (README rule 1)._

## Known gaps

_Wave B: list unported / stubbed functionality here._
