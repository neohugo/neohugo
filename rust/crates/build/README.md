# neohugo-build

Build orchestration (REWRITE_PLAN.md §2.6, §3). **State: T38 walking skeleton**; T36 writes
the full pipeline on the same entry point.

```rust
pub enum SinkKind { Disk /* default */, Memory }
pub struct BuildRequest { pub source: PathBuf, pub destination: Option<PathBuf>, pub cli: CliOverrides,
                          pub clock: Option<jiff::Timestamp>, pub sink: SinkKind, pub clean_destination: bool }
pub fn build(r: BuildRequest) -> Result<BuildReport, BuildError>;
pub enum BuildError { Config, Vfs, Model, Template, Render, Publish, Diagnostics(Vec<Diagnostic>) }
pub struct Collision { pub path: OutputPath, pub winner: JobOrder, pub loser: JobOrder }
pub struct BuildReport { pub pages, pub outputs, pub aliases, pub resources, pub images: usize,
                         pub collisions: Vec<Collision>, pub diagnostics: Vec<Diagnostic>,
                         pub timings: Vec<(&'static str, Duration)>, pub memory: Option<Arc<MemorySink>> }
```

`BuildError::Resource` (§2.6) comes with T36's resources; `Vfs` is extra (the Vfs has its own
error type).

## What the skeleton runs

`config::load` (process `HUGO_*` env) → `Vfs::new` → `site::load_model` → `LayoutStore::scan`
→ `Session::new` → `render_content` → `freeze_views` → `sync_static` into the sink →
wave 1 per language in order and wave 2 (`Session::wave1`/`wave2`), each rendered with rayon,
sorted by `JobOrder`, collisions resolved (the later job wins, reported), then
`Publisher::emit` → `hugo_stats.json` in the project directory (disk builds only).

Not yet (T36): resources and images, the deferred wave and `patch_held`, URL-token
publishing, the render pool and its determinism gate.

The planned edges on `neohugo-resources`, `neohugo-images`, `neohugo-highlight` and
`neohugo-esbuild` (and `tracing`) were dropped for the skeleton; T36 restores them.

## L1 on the testsite (`cargo test -p neohugo-build -- --nocapture`)

`skeleton::testsite_l1` rebuilds the site of `tools/rust-port/i01/sites.py make testsite`
(`hugolib/testsite` + `testsite.txtar`) with the Tera layouts of `rust/sites/testsite/layouts`
into a `MemorySink` and compares the file list with the Go build's (`hugo -d public` of the same
site, embedded in the test): **55/55 files equal** (Go's 56th file is `hugo_stats.json` in the
project directory, which a memory build does not write). `NEOHUGO_T38_OUT=<dir>` writes the
files for a manual diff. `skeleton::testsite_contents` checks canonified links, pagination,
taxonomies, translations, JSON and RSS homes, aliases, the language redirect, sitemaps and
robots.txt.

Byte diff against Go (informational): 49 of 55 files identical. The rest: whitespace control
in the embedded `sitemap.xml`/`sitemapindex.xml` (`</url><url>` in Go), `html_escape` keeps
newlines where Go writes `&#xA;` (one RSS description), and `date(format="%b …")` is
localised in `nn` (`feb.` vs Go's always-English `Feb`).
