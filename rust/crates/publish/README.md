# neohugo-publish

Sinks, canonify, minify dispatch, `hugo_stats.json`, URL-token extraction, held outputs and the
static sync (REWRITE_PLAN.md §2.6, §3.4; phases E1, E2 output, E4 and the `patch_held` half of
E5).

| API | What |
|---|---|
| `DiskSink { root }`, `MemorySink { files: DashMap<OutputPath, Arc<[u8]>> }` | `base::Sink`; the disk sink creates parent directories and truncates; the memory sink has `get`, `text`, `paths` (sorted) and `write_to(dir)` |
| `PublishSettings::from_config(&Config)` | per-language `SiteLinks` (base URL, `canonifyURLs`, `relativeURLs`), output formats and media types, the `Minifier` when `minifyOutput` is set, `[build.buildStats]`, optional LiveReload URL (`serve`) |
| `Publisher::new(settings, Arc<dyn Sink>, Arc<Diagnostics>)` | shared by the render workers (`Send + Sync`) |
| `Publisher::emit(Output { path, text, format, lang })` | canonify / relative URLs (RSS always, HTML when configured) → LiveReload script (HTML, `serve`) → stats (HTML) → URL tokens → hold when a `__nh_defer_` / `__nh_pp_` placeholder is present, else minify by media type and write. Empty text writes nothing (`Emitted::Empty`). A minifier error writes the output unminified with a `minify-output` warning |
| `Publisher::patch_held(&BTreeMap<placeholder, text>)` | replaces the placeholders of every held output, extracts its URL tokens again, minifies and writes (rayon, outside renders); a placeholder left over is `PublishError::UnresolvedPlaceholder` |
| `Publisher::add_tokens_from(text)`, `url_tokens()` | `execute_as_template` results; the sorted `UrlTokens` so far |
| `Publisher::stats() -> HugoStats`, `HugoStats::{to_json, write_if_changed}` | `hugo_stats.json`: sorted lists, `null` when disabled or empty, two-space JSON with a final newline, written only when changed |
| `UrlRewriter::{absolute, relative, new}.rewrite(bytes, Quoting)` | the canonify rewriter (also `rewrite_str`); `canonify::dotted_path_to_root` |
| `HtmlElements::collect(html)`, `StatsCollector` | the tags, classes and ids of HTML |
| `UrlTokens::{extract, iter, contains}` | URL-shaped words after decoding HTML references and JSON escapes (srcset lists, unquoted attributes, `./` / `../` resolved against the output) |
| `sync_static_dir(&Vfs, root, &StaticSyncOptions)` | phase E1 on disk: rewrite only changed files, copy permissions and modification times (`noChmod`, `noTimes`), `cleanDestinationDir` (keeps `.`-directories), multihost language directories; returns the file count |
| `sync_static(&Vfs, &dyn Sink, &StaticSyncOptions)` | the same files into any sink (memory builds) |
| `livereload::{script, inject}` | the LiveReload `<script>` placed at the start of the head |

**URL-token seam.** `neohugo-resources` does not depend on this crate (§2.3). The store's
`publish` takes any `IntoIterator<Item = &str>`; `neohugo-build` passes `publisher.url_tokens().iter()`
(or `&UrlTokens`, which is `IntoIterator<Item = &str>`). Tokens are decoded but not
canonicalised: the store reduces them (percent-decoding, host, query) to its own URL index.

## Rules

- **Canonify** (`canonify.rs`): a candidate follows `src=`, `href=`, `url=`, `action=` or
  `srcset=` (case-sensitive substrings); after the first four an optional quote and a
  root-relative URL (`/x`, not `//x`), after `srcset=` a quoted list starting with one (at most
  2000 bytes up to the closing quote; every root-relative candidate is rewritten and white
  space collapses to single spaces). The prefix's own path (`docs/`) is dropped when the URL
  repeats it. HTML quotes are `"` `'`, RSS quotes `&#34;` `&#39;`. Aliases are HTML outputs.
- **Stats** (`stats.rs`): html5gum tokens; every start tag (lower-cased), `id` values,
  `class` and `*transition*` attribute words, and Vue/Alpine `:class` bindings (object keys, or
  single-quoted words). The content of `pre`, `textarea`, `script` and `style` is skipped.
  Collected before minification.
- **Held outputs**: the plan's placeholder prefixes (`PLACEHOLDER_PREFIXES`). Replacement text
  is inserted after canonify (as Hugo inserts deferred output into published files) and
  minified with the page.

## Acceptance evidence

`cargo test -p neohugo-publish` (lib 6, it 17):

- **canonify** — `oracle/transform/absurl/cases.jsonl.gz`: 94,180 cases, 93,178 exact, 1,002
  accepted in 3 Go-quirk classes (below), 0 unexplained; the upstream `absurlreplacer_test.go`
  tables exact.
- **LiveReload** — `oracle/transform/absurl/inject.jsonl.gz`: 175/175 livereload records
  exact; 14 generator-tag records accepted (never injected), 11 equal.
- **stats collector** — `oracle/publisher/collector/collector.jsonl.gz`: the upstream
  `TestClassCollector` documents 170/170 exact; html5lib documents 1,662/1,708; hand-written
  documents 185/265; element strings 28,673/36,795; element documents 8,527/12,265; random
  documents 2,288/4,000; multi-write streams 765/1,260; groups 7/25 (56,488 checks, every
  difference classified in `expected_diffs.toml`, 0 unexplained). The 3,022 `closed` records test Go's private `isClosedByTag`
  and have no counterpart.
- **golden stats** — `tools/rust-port/golden/hugo_stats.json` (seeksnack) and
  `docs/hugo_stats.json` round-trip byte for byte through `HugoStats::to_json` (format,
  sorting, `null`). Scanning the golden HTML itself needs the Go build outputs (T01 golden
  trees), which are not in the repository.
- **static sync** — `oracle/commands/staticcopy/staticcopy.json.gz`: 12 cases, 179 checks
  (count + every entry's bytes, mode, mtime), 163 exact, 16 accepted, 0 unexplained;
  `sync_static` into a `MemorySink` (plain and multihost).
- **publisher** — canonify per format (HTML with `canonifyURLs`, RSS always with entity quotes,
  JSON never), `relativeURLs`, minify dispatch (HTML, JSON, `text/plain` untouched, invalid
  JSON published with a warning), empty outputs, held outputs patched then re-scanned (the
  PostProcess URL becomes a token) and minified, unresolved placeholders, URL tokens (entities,
  JSON escapes, srcset, `&`/`'` paths), stats of HTML outputs only, identical tokens/stats/paths
  with 1 and 4 threads, disk and memory sinks.

## Accepted deviations

All counted in `expected_diffs.toml` (a changed count fails the tests):

| Family | Classes |
|---|---|
| absurl | `go-panic` 93, `leading-candidate` 855 (Go's prefix positions start at 0, so a document starting with `/x` gets the base written up to 4 times), `prefix-inside-rewrite` 54 (stale positions jump back into written input) |
| inject | `generator-tag-never-injected` 14 |
| collector-* | x/net/html tree-builder and Go-scanner artefacts: markup declarations as tags, table/head/frame elements dropped in a body context, `<prefix>`-style raw-text skips, quotes tracked across tags, repeated attributes, Unicode tag names, NUL and numeric references, characters split across writes, invalid UTF-8 |
| staticcopy | `static-mount-precedence` 4 (Hugo: the later static mount overwrites; neohugo-vfs: the first mount wins), `symlinks-not-followed` 5 (neohugo-vfs skips symbolic links below a mount root), `empty-dirs-not-copied` 6, `missing-static-dir` 1 |

Not implemented here: `Output` carries no `JobOrder` (that type lives in `render`, which this
crate does not depend on; collisions are ordered by `build`).
