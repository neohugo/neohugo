# neohugo-view

Views and render state (REWRITE_PLAN.md §2.5). **State: T38 walking skeleton.** T33 turns the
views into the plan's full set; the render-state types below are frozen and T33/T34/T35 build
on exactly these signatures.

## Frozen by T38

```rust
pub const SCOPE_KEY: &str = "__nh";
pub const MAX_DEPTH: u16 = 64;
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Phase { Content, Layout, Deferred }
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum HookVariant { Html, Format(FormatId) }
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Stage { Expand, Fragments, Content(HookVariant) }

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct RenderScope {
    pub page: PageId, pub lang: LangIdx, pub format: FormatId, pub pager: Option<u32>,
    pub phase: Phase, pub variant: HookVariant, pub frame: Option<FrameId>, pub txn: Option<TxnId>,
    pub depth: u16, pub chain: Vec<(PageId, Stage)>,
}
impl RenderScope {
    pub fn layout(page: PageId, lang: LangIdx, format: FormatId, pager: Option<u32>) -> Self;
    pub fn from_state(s: &tera::State) -> tera::TeraResult<Option<Self>>;  // None: no `__nh`
    pub fn child(&self) -> Self;        // same page/format/pager, depth + 1, frame None
    pub fn too_deep(&self) -> bool;     // depth > MAX_DEPTH
    pub fn to_value(&self) -> tera::Value;
}

pub trait ContentRenderer: Send + Sync {
    fn content(&self, p: PageId, v: HookVariant, s: &RenderScope) -> Result<Arc<RenderedContent>, ContentError>;
    fn fragments(&self, p: PageId, s: &RenderScope) -> Result<Arc<Fragments>, ContentError>;   // neohugo_markup::Fragments
    fn render_shortcodes(&self, p: PageId, s: &RenderScope) -> Result<Arc<ExpandedSource>, ContentError>;
    fn render_markdown(&self, md: &str, o: RenderStringOptions, s: &RenderScope) -> Result<String, ContentError>;
    fn render_template(&self, t: &TemplateName, ctx: tera::Context, s: &RenderScope) -> Result<String, ContentError>;
}
```

The scope is a plain serde value in the context: `{{ __nh.page }}` works in templates, and
Tera's `Value` deserializer (enums as `{"Format": 1}` or `"Html"`) reads it back. No
thread-locals.

`RenderedContent { html, summary, truncated, plain, word_count, fuzzy_word_count,
reading_time, table_of_contents, fragments: Arc<Fragments> }`, `ExpandedSource { markdown,
placeholders: Vec<Arc<str>>, contexts: SourceContexts }`, `RenderStringOptions {
display_block }` and `ContentError` are the skeleton's shapes; T34 may add fields and variants.

## Skeleton parts (replaced)

| Item | Now | Replaced by |
|---|---|---|
| `interim::FlatSite` | the flat interim model copied out of T23b's `Model` (same page ids: made pages, titles, `Page.urls`, parents, lists, `.Sections`, translations, terms, node dates, `.Site.Taxonomies` via `listed_terms`); pager targets via `Model::pager_paths`; derives only prev/next in section and front matter alias files | T24 (aliases, pagination), T33 (views) |
| `ViewCache`, `ViewGeneration` | `new(flat)` builds the Meta generation, `freeze(&contents)` the Full ones; `generation(phase, variant)`, `summaries`, `links`, `sites`, `full(id)` (lazy, `OnceLock`) | T33 keeps this API over the real model |
| `views::*` | a subset of §2.5's views: what the testsite and embedded rss/sitemap/alias templates read | T33 (every documented key) |
| `PaginationRecorder`, `Recorded` | first call per (page, format) wins; `total_pages`, `page(n)`, `recorded()` | T35 (conflict error with both positions), T24 (`Pagination`) |

Not in the skeleton: `PageStores`, `DeferredRegistry`, menus, resources, related pages.

The planned dependency edges on `neohugo-nav` and `neohugo-resources` were dropped for the
skeleton (nothing uses them yet); T33 restores them.

## Tests (`cargo test -p neohugo-view`)

`scope`: the scope round-trips through a Tera render and is read back by a function; a render
without `__nh` has none; `child`/depth limit; the pagination recorder keeps the first call.
The views are exercised end to end by `neohugo-build`'s testsite test.
