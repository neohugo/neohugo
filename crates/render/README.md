# neohugo-render

The render session (REWRITE_PLAN.md §2.6, §3.2–3.4, §4.2–4.4). **State: T34** (the content
engine) **+ T36 wiring**: the site functions are T35's `neohugo_sitefuncs::register` (the T38
stubs are gone), and `neohugo-build` runs the phases with the jobs of `wave1`/`wave2`, the
targets of `target` and the deferred templates of `render_deferred`.

## API

```rust
pub use neohugo_nav::AliasPlan;
pub enum Job { Alias(AliasPlan), Page { page, format }, Pager { page, format, number },
               PagerAlias { page, format }, Standalone { page, format }, LanguageRedirect }
pub struct JobOrder { lang, format_rank, key_rank, sub }        // new(), lang()
pub struct Output { pub path, pub text, pub format, pub lang, pub is_html, pub order }
pub struct Project { pub vfs: Arc<Vfs>, pub layouts: Arc<LayoutStore> }
pub struct RenderOptions { pub clock: Clock }
impl Session {
    pub fn new(project, model: Arc<Model>, o: &RenderOptions) -> Result<Arc<Self>, RenderError>;
    /// T34: `new` plus extra functions registered after the site functions (tests).
    pub fn with_functions(project, model, o, extra: &dyn Fn(&mut tera::Tera, &Handles)) -> …;
    pub fn render_content(&self) -> Result<(), RenderError>;                  // C1, every variant
    pub fn freeze_views(&self) -> Result<(), RenderError>;                    // D
    pub fn render_job(&self, job: &Job) -> Result<Vec<Output>, RenderError>;  // E, pure: no I/O
    pub fn variants(&self) -> &[HookVariant]; pub fn page_stores(&self) -> &Arc<PageStores>;
    pub fn templates(&self) -> &Templates; // + model, views, diagnostics, order, wave1, wave2
    /// T36: the file a job writes without rendering it (`None`: it writes nothing).
    pub fn target(&self, job: &Job) -> Result<Option<OutputPath>, RenderError>;
    /// T36, phase E5: a `defer(...)` template with `data`, `site`, `hugo`, `__nh` (phase Deferred).
    pub fn render_deferred(&self, key: &str, d: &Deferred) -> Result<String, RenderError>;
    pub fn handles(&self) -> &Handles; // T36: the store, image queue, deferred registry, … for E4–E6
}
impl ContentRenderer for Session { content, fragments, render_shortcodes, render_markdown, render_template }
pub mod summary { manual, auto, plain, counts, unwrap_paragraph, Split, DIVIDER, DIVIDER_SOURCE }
pub enum RenderError { …, Content { page, source: Box<ContentError> } /* T34 */,
                       I18n, Vfs, Io, Deferred { key, template, source } /* T36 */ }
```

`Session::new` follows §2.6: the renderer slot (`Arc<OnceLock<Weak<dyn ContentRenderer>>>`) is
created empty → `Handles` (the session's named mutable state: page stores, pagination
recorder, deferred registry, frames, partial cache, related cache; **one `ImageQueue` with the
`[caches.images]` file cache, shared with `StoreConfig::from_config`**; **the translations of
every i18n file** of the project and its themes, lowest precedence first; **one `Highlight`**,
also used for the fences of every language whose `[markup.highlight]` is the default site's) →
`register_placeholders`, `register_pure`, `neohugo_sitefuncs::register`, `extra` →
`layouts::load` → `Arc::new(Session)` → the templates slot and the renderer slot are set.

**Jobs (T36).** `wave1(lang)`: front matter aliases, pages × formats, standalone pages
(robots.txt and the sitemap index with the first language). `wave2()`: from the recorded
paginations the `page/1/` aliases (HTML formats, unless `pagination.disableAliases`) and pagers
2..N, then the language redirect of `neohugo_nav::language_redirect` (`/en/` → `/`, or `/` →
`/en/` with `defaultContentLanguageInSubdir`; none with `disableDefaultLanguageRedirect`).

## Content phase (C1)

Per page with a content file (bundled content pages included) and per **hook variant**
(`Html`, plus `Format(F)` for each output format F with a `_markup/*.<F>.*` hook; a layout job
for F uses `Format(F)` if it exists):

1. **Expand** (`shortcode.rs`): `neohugo_pageparser::parse_body` with the template store as
   `InnerOracle` (`uses_variable(tpl, "inner" | "inner_deindent")`), then every call is run
   through its Tera template (looked up with the variant's format, so `fmt.rss.xml` serves the
   RSS variant) with `page` (Meta full value), `site`, `hugo`, `lang`, `shortcode`
   (`ShortcodeView`: typed `args`, `params` list or map, `is_named_params`, `ordinal` per
   nesting level, `parent`, `position` — `"file:line:col"`, quoted as Go's `.Position` prints),
   `inner` / `inner_deindent` (safe) and `__nh`. Render hooks get `position` in the same form.
   - `{{% %}}` output is spliced into the Markdown; `{{< >}}` output becomes `NHSC<n>X`.
   - Nested calls run first, into the parent's `inner`. The inner of the outermost `{{% %}}`
     is raw; a nested `{{% %}}` inner is rendered as Markdown (a one-line inner loses its
     `<p>`). `$_hugo_config` v1 is not reproduced (D5).
   - A call without inner content whose tag is indented gets the indentation on its further
     output lines (Hugo); `inner_deindent` removes the call's indentation from inner lines.
   - Inline shortcodes (`security.enableInlineShortcodes`): the body is a Tera template
     (`render_str`), reused by later self-closed calls; disabled, they print nothing.
   - The summary divider becomes its own paragraph (`summary::DIVIDER_SOURCE`).
2. **Fragments** (its own memo stage): `neohugo_markup::fragments` of the `Html` expansion,
   parse only, no hooks. A page's own fragments asked while its shortcodes run (a TOC
   shortcode) are parsed from the body with the calls left out and are not memoised, so
   `{{< toc >}}` on its own page is not a cycle.
3. **Render**: comrak through `neohugo_markup::render` with `TeraHooks` (`hooks.rs`), HTML
   content passed through. Hooks: `_markup/render-<kind>[-<variant>]` of the variant's format,
   else the HTML format's; the embedded table hook is left to markup's native output; context
   `page`, `page_inner` (from the source-context spans), `site`, `hugo`, `lang`, `__nh` and the
   `HOOK_FIELDS` flattened (`text` and cell texts safe; `alert_sign` as `+`/`-`/``). Under
   `CodeFences::Hooked`, a fence no hook handles goes to `neohugo_highlight::Highlight`
   (built per language at its first fence).
4. **Swap and derive**: placeholders swapped (a `<p>` holding only a placeholder is removed),
   in the HTML, the TOC and the fragments; summary (manual divider, else front matter
   `summary` rendered like `markdownify`, else automatic), `.Plain`, word count, fuzzy word
   count, reading time (`summary.rs`).

### Memo cells (`memo.rs`)

`ContentStore { expanded: variant → IdVec<PageId, Memo>, frags: IdVec<PageId, Memo>,
content: variant → IdVec<PageId, Memo> }`, created with the variants known after template
load. `get_or_compute(cell, key, scope, place, stores, cycle, f)`:

- a set cell returns its value; **it never blocks** (`OnceLock::get`, then `set`);
- a key already in `scope.chain` is `ContentError::Cycle`, naming the chain's files and
  stages from the repeated key back to itself (`a.md content (html) → b.md shortcodes → …`);
- otherwise the caller computes with a child scope: page = the key's page, its language, the
  variant's format, phase `Content`, `chain + key`, and a **new page-store transaction**. The
  first `set` wins and **commits** its transaction; a loser returns the winner's value and
  **discards** its writes; a failing computation discards its writes.

Cross-page requests (`page_content`, `page_fragments`, `render_shortcodes`, `markdownify` with
`page=`) all go through the `ContentRenderer` with the caller's scope.

### `render_shortcodes` contract (for T35)

`ContentRenderer::render_shortcodes(q, scope)` returns an `ExpandedSource` whose `markdown` the
site function prints as is (safe):

- in the **content phase** it is an inclusion token `NHRS<n>X`. The expanding page replaces it
  in `{{% %}}` output by q's expanded Markdown, **appends q's placeholders to its own table and
  renumbers** q's tokens, shifts q's context spans and adds a span for the included text (so
  hooks there get `page_inner = q`). In `{{< >}}` output, hook output or `markdownify` input it
  becomes q's text with q's placeholders resolved.
- elsewhere (layouts) it is q's source with the shortcode outputs in place.

## Tests (`cargo test -p neohugo-render -- --nocapture`)

The suites run with **test doubles** of the T35 functions they need (`tests/it/fakes.rs`:
`markdownify`, `page_content`, `page_toc`, `render_shortcodes`, `store_set`/`store_get`,
`ref`/`rel_ref`, `get_page`, `get_resource`, `get_asset`), registered with
`Session::with_functions`.

| Acceptance (T34 row of §8.2) | Evidence |
|---|---|
| shortcode semantics on R's edge pages | content oracle `seeksnack` (R's edge pages: `about.md` and the koala bundle with every shortcode form, escapes, Thai params, v1, `%` vs `<`, nesting, indentation, `ref`/`relref`, HTML page, bundled pages, JSON table hook): **921/924 values equal, 3 accepted**; `reconstruction::r_shortcodes_on_r_pages`: R's own shortcodes (txtar) converted to Tera (`R_SHORTCODES`) over R's pages (`parent`, `name`, `modalImage`, ordinal, `page`); `engine::shortcode_semantics`: param typing, `arg` defaults and styles, ordinal/parent/grandparent, `%` vs `<`, escapes, indentation, inline shortcodes; content oracle `shortcodes` (p00–p19 syntax matrix): **276/280, 4 accepted** |
| per-page placeholder renumbering through `render_shortcodes` | `engine::includes_renumber_placeholders_and_set_page_inner` (A's `{{< >}}` before and after an include of B, B's own placeholders) |
| `page_inner` spans | the same test: a link hook prints `page_inner.title` and `page.title` for A's own and B's included links |
| cross-page memo: cycle test, forced two-thread no-deadlock test | `engine::cycles_are_errors` (A ↔ B through `page_content`: an error naming both files; own-page TOC is not a cycle); `engine::two_threads_never_deadlock_and_commit_once` (a `Barrier` in a shortcode holds two threads inside A's and B's computations while each needs the other's fragments; both finish, with a 60 s watchdog) |
| buffered store writes committed once | the same test with both threads computing A: pointer-equal results, a counter written from the shortcode is 1; `engine::store_writes_follow_the_winner` (nothing before C1, committed after, layout writes direct) |
| hooks via Tera; JSON variant | content oracle `content` (blockquote, codeblock, heading, image, link hooks, `render-table.json.json` → `Format(json)`, `render-heading.rss.xml` → `Format(rss)`, `fmt.rss.xml` in the RSS variant): **480/483 equal, 3 accepted**; `engine::json_variant_through_a_layout_job` (a JSON layout job prints the `Format(json)` content) |
| HTML content; bundled content resources | oracle pages `/posts/markup-html`, `/posts/html-page`, `/blog/html-page` (shortcodes, divider, auto summary in HTML) and `/bundle/sub.md`, `…/sub/index.md`, `…/notes.md` (bundled pages); `engine::c1_renders_bundled_pages_and_html_content` (C1 → frozen Full value) |
| summary oracle | `summary::summary_oracle`: `oracle/page/summary/{build,adversarial}` (Hugo's summary of the rendered HTML of every page of the Go builds, variants and 6,000 adversarial calls): **11,537/11,537** Markdown/HTML cases equal (`.Summary`, `.Content`, `.Truncated`); 7,196 cases of external markups (AsciiDoc, RST, Pandoc, Org), non-UTF-8 input or Go panics not applicable |

`skeleton::testsite_bytes` of `neohugo-build` stays **55/55 byte-identical**.

### Accepted differences (reviewed in `tests/it/oracle.rs`)

- A summary divider as the first text of a body is a divider (pageparser's `divider_at_start`):
  manual, empty summary, truncated (Hugo: front matter type, not truncated). 2 pages × 3
  formats.
- `$_hugo_config` version 1 (`legacytag`, p02) is not reproduced (D5): its `{{% %}}` output is
  Markdown.

### Hugo rules kept

The automatic summary follows Hugo's counting (it decides where real summaries end): words that
look like tags or attributes do not count; a paragraph is counted without its last character
and with the `>` of the previous `</p>`; `.Truncated` is true when anything, even a newline,
follows the cut. The manual divider grows to its paragraph (walking back over white space and
`<div>` wrappers to the `<p…>` start tag) and takes one following newline.

## Notes and limits

- Store writes of a page are made once per variant computation (each variant's cell commits
  its own transaction): a counter incremented by a shortcode counts the variants.
- C1 renders every page with a content file, also pages with `build.render = never`; a content
  error there fails the build where Hugo, rendering lazily, would not notice.
- The deferred template of a key renders with the default language's `site` (a `Deferred`
  records no language).
