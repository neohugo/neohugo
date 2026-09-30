# neohugo-build

Build orchestration (REWRITE_PLAN.md §2.6, §3). **State: T36** (the full §3 pipeline on T38's
entry point).

```rust
pub enum SinkKind { Disk /* default */, Memory }
pub struct BuildRequest { pub source: PathBuf, pub destination: Option<PathBuf>,
                          pub config_files: Vec<PathBuf> /* T37: --config */, pub cli: CliOverrides,
                          pub clock: Option<jiff::Timestamp>, pub sink: SinkKind, pub clean_destination: bool,
                          pub threads: Option<usize> /* T36: the render pool size */,
                          pub config: Option<Arc<Config>> /* T71: loaded by the caller (server) */,
                          pub live_reload: Option<LiveReload> /* T71: the server's script */ }
pub struct LiveReload { pub port: Option<u16> /* --liveReloadPort */ }  // url(&BaseUrl) -> UrlRef
pub fn build(r: BuildRequest) -> Result<BuildReport, BuildError>;
pub fn process_env() -> Vec<(String, String)>; // T37: HUGO_*, HOME, XDG_CACHE_HOME, TMPDIR, USER (the CLI's config loads too)
pub enum BuildError { Config, Vfs, Model, Template, Render, Publish, Resource, Pool, Diagnostics(Vec<Diagnostic>) }
pub struct Collision { pub path: OutputPath, pub winner: JobOrder, pub loser: JobOrder }
pub struct BuildReport { pub pages, pub outputs, pub aliases, pub resources, pub images, pub static_files: usize,
                         pub collisions: Vec<Collision>, pub diagnostics: Vec<Diagnostic>,
                         pub timings: Vec<(&'static str, Duration)>, pub memory: Option<Arc<MemorySink>>,
                         pub model: Option<Arc<Model>> /* T71: the rendered model (file → page) */ }
```

**`neohugo-rs server` (T71).** The server loads the configuration itself (once per
configuration change) with every language's base URL pointed at its listener and passes it as
`config` (then `source`, `config_files`, `cli` and `destination` are not read). With
`live_reload`, every language's `SiteLinks::livereload` is its base URL (the port replaced by
`LiveReload::port` when set), so the publisher puts the LiveReload script into every HTML page
but the alias redirects (`Output::alias`, set for alias, `page/1/` alias and language-redirect
jobs). `build` never sets it (the A-T bytes are unchanged). `model` lets the server find the
page of a changed content file (`--navigateToChanged`).

`Vfs` and `Pool` are beyond §2.6 (the Vfs has its own error type; the render pool may fail to
start).

## Phases (`src/lib.rs`, `src/waves.rs`, `src/deferred.rs`)

| Phase | What |
|---|---|
| A1–B6 | `config::load` (process `HUGO_*` env) → `Vfs::new` → `site::load_model` |
| B7, C0 | `LayoutStore::scan` → `Session::new` (site functions, i18n files, views, selections) |
| C1, D | `render_content` (every page, every hook variant) → `freeze_views` |
| E1 | static files: `sync_static_dir` on disk (`noTimes`, `noChmod`, `cleanDestinationDir`, multihost language directories), `sync_static` into memory |
| E2 | wave 1, **one sub-wave per language in language order** (`Session::wave1`) |
| E3 | wave 2 (`Session::wave2`): pagers 2..N and `page/1/` aliases of the paginations wave 1 recorded, the language redirect |
| E4 | `hugo_stats.json` (with `[build.buildStats] enable`): written to the project directory when changed (memory builds too, as Hugo's server does), and injected into the resource store at the asset path of every assets mount of that file (docs: `notwatching/hugo_stats.json`) |
| E5 | every `defer(...)` key rendered once (`Session::render_deferred`, in parallel over keys; `data`, `site`, `hugo`, `__nh` in phase `Deferred`); every post-process placeholder resolved (pending transforms run now, after the stats exist; placeholders inside deferred output too) → `Publisher::patch_held` (rewrite again, URL tokens again, minify, write) |
| E6 | URL tokens of `execute_as_template` results added, then `ResourceStore::publish(tokens)`: eager bundle files, `publish`ed resources, every resource named by a token; processed images through the image queue (`[caches.images]`) |
| E7 | sorted, de-duplicated diagnostics; errors fail the build (`BuildError::Diagnostics`) — after every file was written, as Go's build does |

**Render pool.** Every parallel phase runs on one rayon pool (`stack_size(16 MiB)`, `threads`,
else `RAYON_NUM_THREADS`, else the CPUs); parallel work is started only from outside it
(`RenderPool::run` debug-asserts `current_thread_index().is_none()`).

**Collisions** (`waves.rs`). Every job's target is known before rendering
(`Session::target`). A page (or pager) beats an alias; among jobs of one class the later
`JobOrder` wins; a job whose target an earlier wave claimed is compared with that file's job
(later languages win among equals). Each collision is a `target-collision` warning and a
`Collision`. Losing jobs are still rendered and their outputs dropped: Go renders every page,
so a losing list page still records its pagination and its pagers exist (they compete for their
own targets in wave 2).

**Publishing.** Each winning output goes to `Publisher::emit` from its render worker; errors are
reported in job order.

## Tests (`cargo test -p neohugo-build -- --nocapture`)

| Test | What | Result |
|---|---|---|
| `skeleton::testsite_{l1,bytes,contents}` | `sites.py make testsite` with `rust/sites/testsite/layouts` vs Go's `public/` (`tests/it/testsite-go.txtar`) | **55/55 files, 55/55 byte-identical** |
| `skeleton::testsite_for_the_server` | the testsite with a caller-loaded configuration (base URL `http://localhost:1313/`) and `live_reload` | the script right after `<head>` (after `<html>` in the 404 page) of every HTML page, none in aliases, `page/1/`, the language redirect, RSS and JSON; canonified links on the server URL; `model` set |
| `mini::mini_matches_the_go_tree` | the e2e `mini.txtar` site (en/th, hooks, shortcodes, pagination, taxonomies, menus, i18n, data, related, aliases, `defer`, `GetRemote` from the file cache + `unmarshal`, minify, fingerprint, Concat, ExecuteAsTemplate, FromString, PostProcess), Go layouts converted to Tera in the test, `--minify --clock`, vs the Go tree of `e2e.json.gz` | **53/53 files** (fingerprints normalised); no placeholder left, deferred footer everywhere, post-processed CSS linked and published, stats in the project directory |
| `edges::edge_trees_match_the_go_file_lists` | the 25 build oracles of `oracle/hugolib/build` (all but `build-errors`): cascade, i18n, multihost, ugly, taxonomy permalinks, aliases, collisions, custom alias template, disabled kinds/aliases/redirect, post-processing, stats, content dirs, Thai/punctuated paths, headless and build options, `content`/`seeksnack`/`shortcodes` (with `neohugo-render`'s shortcode and hook conversions), `docs` (948 pages, stub layouts) — disk builds vs the files Go wrote, and whether errors are reported | **2451/2451 files** over 25 sites; 2 accepted differences (below) |
| `docs_shortcodes::docs_cross_page_shortcodes` | docs' `include`, `glossary-term` and `quick-reference` converted to Tera on a docs-shaped site | renumbered placeholders around an include, `page_inner` of included links, a never-rendered glossary term, other sections' `.Content` and descriptions as a definition list |
| `images::referenced_images_are_processed_and_published` | a bundle image resized twice, one result printed | only the printed operation processed (through `resources/_gen/images`) and published; the bundle file published eagerly |
| `determinism::output_does_not_depend_on_threads` | A-DET: `mini`, `build-collide`, `asm-taxo`, `build-aliases` with 1, 8 and 8 threads | identical trees and collision lists |
| `smoke::smoke` (ignored) | `NEOHUGO_SITES=<dir>[:…]` builds real site directories and prints the report | — |

Accepted differences of the edge trees (`EXPECTED` in `tests/it/edges.rs`):
`build-postprocess` `js/main.js` (the conversion leaves out `js.Build`, no esbuild offline) and
`css/main.css` (the page prints the asset's `.Name`, `/css/main.css`, which is also its URL:
URL-token publishing publishes it, Go publishes only on `.RelPermalink`).

Deviations from Go kept on purpose: an earlier language's `FromString` target is the one
published (Go: the later language overwrites the file); a colliding target's winner is the
later `JobOrder` (Go: the last writer); deferred output is minified with its page (Go inserts it
into the minified file).
