# neohugo-render

The render session (REWRITE_PLAN.md §2.6, §3.2–3.3). **State: T38 walking skeleton.** The job
and session signatures below are frozen; T34 (content engine), T35 (site functions) and T36
(orchestration) replace the bodies.

## Frozen by T38

```rust
pub use neohugo_nav::AliasPlan;   // { from: OutputPath, to: PageId, format: FormatId, kind: AliasKind }
pub enum Job {
    Alias(AliasPlan),
    Page { page: PageId, format: FormatId },               // pager 1 when paginated
    Pager { page: PageId, format: FormatId, number: u32 }, // wave 2, number ≥ 2
    PagerAlias { page: PageId, format: FormatId },         // …/page/1/ → node (HTML formats only)
    Standalone { page: PageId, format: FormatId },         // 404, sitemap, sitemapindex, robots
    LanguageRedirect,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct JobOrder { lang: LangIdx, format_rank: u8, key_rank: u32, sub: u8 }
impl JobOrder { pub const fn new(lang, format_rank, key_rank, sub) -> Self; pub const fn lang(self) -> LangIdx; }
pub struct Output { pub path: OutputPath, pub text: String, pub format: FormatId, pub lang: LangIdx,
                    pub is_html: bool, pub order: JobOrder }

pub struct Project { pub vfs: Arc<Vfs>, pub layouts: Arc<LayoutStore> }
pub struct RenderOptions { pub clock: Clock }                // Default: system clock
impl Session {
    pub fn new(project: Project, model: Arc<Model>, o: &RenderOptions) -> Result<Arc<Self>, RenderError>;
    pub fn render_content(&self) -> Result<(), RenderError>;                  // C1
    pub fn freeze_views(&self) -> Result<(), RenderError>;                    // D
    pub fn render_job(&self, job: &Job) -> Result<Vec<Output>, RenderError>;  // E, pure: no I/O
}
impl ContentRenderer for Session { … }
pub enum RenderError { Target(Box<TargetError>), View(Box<ViewError>), Nav(Box<NavError>),
                       Template(TemplateError), Markup { file, message },
                       Render { template, page, source: Box<tera::Error> }, NoOutput { page, format },
                       Phase(&'static str) }
```

`Output` is the render side of a file; `neohugo-publish::Output { path, text, format, lang }`
is the publisher's, and `neohugo-build` converts one into the other after resolving target
collisions by `JobOrder` (publish does not depend on render, §2.3).

Calling the phases out of order (`freeze_views` before `render_content`, `render_job` before
`freeze_views`) is `RenderError::Phase`.

## Skeleton parts (replaced)

- `Session::new`: resource store (`StoreConfig::from_config`, no image queue yet) and menus
  (`neohugo_nav::build_menus` over `NavSite`) → `ViewCache::new` (T33's views over the model)
  → alias plan (`neohugo_nav::page_aliases`) → layout selection for every (page, format)
  (a page without a layout is a warning; standalone pages without one are skipped) →
  `layouts::load` with `funcs::register_placeholders`, `funcs::register_pure` and the stubs.
  No renderer slot yet: T35 adds it in the plan's order (slot empty → `sitefuncs::register`
  → `layouts::load` → `Arc::new(Session)` → `slot.set(Arc::downgrade(&session))`).
- **Content** (`content.rs`): Markdown through `neohugo_markup::render` with `NoHooks` and no
  highlighter, HTML content passed through, `<!--more-->` / front matter `summary` / automatic
  summary, plain text, word counts, reading time, TOC. No shortcodes, hooks, memo cells or
  per-format variants; CJK word counting off.
- **Stub site functions** (`stubs.rs`; every other site-bound name is a kwargs-checking
  placeholder from `neohugo_funcs::register_placeholders`):

  | name | stub |
  |---|---|
  | `paginator()` | records the scope page's default list (`neohugo_nav::default_pagination_list`) with `pagination.pagerSize` in the view crate's `PaginationRecorder`, first call wins; returns pager `__nh.pager` (1 when unset) as a `PagerView` (`neohugo_view::pager_view`) |
  | `rel_url`, `abs_url` | `SiteUrls::{rel_url, abs_url}` of the scope's language |
  | `deref` | the full value of `p.id` in the scope's generation |
  | `by_lastmod` | stable sort by `lastmod.unix` |

- **Contexts**: layout jobs get `page` (the full value, `ViewGeneration::page_value`), `site`, `hugo`, `lang`, `output_format`,
  `__nh` (+ `sites` for the sitemap index); aliases get `permalink`, `page` (link), `site`,
  `hugo` (`alias.html` of the earliest origin).
- **Job planning** (not frozen, T36's): `Session::wave1(lang)` (aliases, pages × formats,
  standalone pages; robots.txt and the sitemap index with the first language) and
  `Session::wave2()` (pager aliases and pagers 2..N from the recorder, then the language
  redirect), both sorted by `JobOrder` (lang, format id, page id, sub-order by job kind).

The planned edges on `neohugo-sitefuncs` and `neohugo-pageparser` were dropped for the
skeleton; T34/T35 restore them.

## Tests (`cargo test -p neohugo-render`)

`session`: a small site through all phases: phase-order errors, wave 1 job kinds and
outputs, Markdown content, a missing 404 layout gives no output, the home paginator recorded
in wave 1 gives the `page/1/` alias and pager 2 in wave 2 (with `__nh.pager = 2`).
