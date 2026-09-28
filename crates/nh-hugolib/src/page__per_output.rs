//! Port of `hugolib/page__per_output.go` (+ `hugolib/site.go` hookRendererTemplate and the
//! content-converter helpers of `hugolib/page.go`/`page__meta.go` that rendering needs).
//!
//! Owner: Wave B task T22 (hugolib-content).

//! Go `hugolib/page__per_output.go`: per-output content (`.Content`, `.Plain`, `.Summary`, TOC,
//! `RenderString`) and the render-hook provider: for each hook type look up
//! `_markup/render-<type>` via `LookupPagesLayout` (CategoryMarkup, Variant1=type, output format of
//! the CURRENT output — `index.json` renders content in the json context and gets the embedded
//! `render-table.json.json` text template), wrapped in `HookRendererTemplate`.
//!
//! Also here (they belong to the content path; T23 owns the rest of page.go/page__meta.go):
//! `pageState.getContentConverter`, `pageMeta.newContentConverter` (T20 left it to T22),
//! `pathOrTitle`, `posFromInput`, `posOffset`, `parseError`, `wrapError`, `getPageInfoForError`.

use std::collections::{BTreeMap, HashMap};
use std::sync::atomic::{AtomicBool, AtomicI64, AtomicU32, Ordering};
use std::sync::{Arc, Mutex, OnceLock, Weak};

use go_value::{GoString, HostCtx, Value};
use nh_common::Result;
use nh_common::herrors::{Error, FilePos};
use nh_common::text::Position;
use nh_markup::converter::converter::{
    Converter, DocumentContext, NopConverter, ParsedDoc, RenderContext, ResultParse, ResultRender,
};
use nh_markup::converter::hooks::{
    BlockquoteRenderer, CodeBlockRenderer, GetRendererFunc, HeadingRenderer, LinkRenderer,
    PassthroughRenderer, Renderer, RendererType, TableRenderer,
};
use nh_markup::goldmark::goldmark_config::{
    RENDER_HOOK_USE_EMBEDDED_AUTO, RENDER_HOOK_USE_EMBEDDED_NEVER,
};
use nh_markup::tableofcontents::Fragments;
use nh_tpl::template::TplContext;
use nh_tplimpl::category::{Category, SubCategory};
use nh_tplimpl::templatestore::{TemplInfo, TemplateQuery, TemplateStore};

use crate::hugo_sites::HugoSites;
use crate::page::{PageHandle, PageId, PageState, PageWrapper};
use crate::page__content::{CachedContentScope, get_or_create_scope_for};
use crate::page__output::{HugoSitesRef, PageOutput, upgrade_hs};
use crate::shortcode_parse::pos_from_input;
use crate::template_exec::{ExecCall, ExecKind};

/// The template context of a `HostCtx` (the caller's, unchanged; a default context when the
/// caller passed something else).
pub(crate) fn tpl_ctx(ctx: HostCtx<'_>) -> TplContext {
    TplContext::from_host(ctx).cloned().unwrap_or_default()
}

/// Go: `pageContentOutput`.
pub struct PageContentOutput {
    /// Index of the output this content was rendered for (informational).
    pub output_idx: usize,
    /// Go `renderHooks` (lazily resolved GetRendererFunc; Go's `init sync.Once`).
    pub render_hooks: OnceLock<GetRendererFunc>,
    /// Go `po`.
    pub po: Arc<PageOutput>,
    /// Go `otherOutputs`: other pages involved in rendering of this page, typically included
    /// with `.RenderShortcodes` (pid -> content output).
    pub other_outputs: Mutex<BTreeMap<u64, Arc<PageContentOutput>>>,
    /// Go `contentRenderedVersion` (incremented on reset).
    pub content_rendered_version: AtomicU32,
    /// Go `contentRendered` (set on content render).
    pub content_rendered: AtomicBool,
    self_weak: Weak<PageContentOutput>,
}

impl PageContentOutput {
    /// Go: `newPageContentOutput(po)`.
    // Go: hugolib/page__per_output.go:newPageContentOutput
    pub fn new(po: &Arc<PageOutput>, output_idx: usize) -> Result<Arc<PageContentOutput>> {
        Ok(Arc::new_cyclic(|w| PageContentOutput {
            output_idx,
            render_hooks: OnceLock::new(),
            po: po.clone(),
            other_outputs: Mutex::new(BTreeMap::new()),
            content_rendered_version: AtomicU32::new(0),
            content_rendered: AtomicBool::new(false),
            self_weak: w.clone(),
        }))
    }

    /// This content output as an `Arc` (Go passes the pointer around).
    pub fn self_arc(&self) -> Option<Arc<PageContentOutput>> {
        self.self_weak.upgrade()
    }

    fn arc(&self) -> Arc<PageContentOutput> {
        self.self_arc()
            .expect("PageContentOutput is always in an Arc")
    }

    /// The frozen `HugoSites`.
    pub(crate) fn h(&self) -> Arc<HugoSites> {
        upgrade_hs(&self.po.hs)
    }

    /// Go: `Reset()`.
    // Go: hugolib/page__per_output.go:Reset
    pub fn reset(&self) {
        self.content_rendered_version.fetch_add(1, Ordering::SeqCst);
        self.content_rendered.store(false, Ordering::SeqCst);
        // Go also replaces `renderHooks`; the port never re-renders (no server mode).
    }

    /// Go: `c()` — the default markup scope of this output.
    // Go: hugolib/page__per_output.go:c
    pub fn c(&self) -> Arc<CachedContentScope> {
        get_or_create_scope_for(&self.h(), "", &self.arc())
    }

    /// Go: `Markup(opts...)` — the markup scope named by the (optional) argument.
    // Go: hugolib/page__per_output.go:Markup
    pub fn markup(&self, opts: &[Value]) -> Result<Arc<CachedContentScope>> {
        if opts.len() > 1 {
            return Err(Error::new("too many arguments, expected 0 or 1"));
        }
        let mut scope = String::new();
        if opts.len() == 1 {
            scope = String::from_utf8_lossy(nh_common::cast::caste::to_string(&opts[0]).as_bytes())
                .into_owned();
        }
        Ok(get_or_create_scope_for(&self.h(), &scope, &self.arc()))
    }

    /// Go: `Content(ctx)` -> `template.HTML` (as `any`).
    // Go: hugolib/page__per_output.go:Content
    pub fn content(&self, ctx: HostCtx<'_>, _p: &PageHandle) -> Result<Value> {
        self.c().content(ctx)
    }

    /// Go: `ContentWithoutSummary(ctx)`.
    // Go: hugolib/page__per_output.go:ContentWithoutSummary
    pub fn content_without_summary(&self, ctx: HostCtx<'_>) -> Result<Value> {
        self.c().content_without_summary(ctx)
    }

    /// Go: `TableOfContents(ctx)`.
    // Go: hugolib/page__per_output.go:TableOfContents
    pub fn table_of_contents(&self, ctx: HostCtx<'_>) -> Value {
        self.c().fragments_html(ctx)
    }

    /// Go: `Fragments(ctx)` (`None` = nil).
    // Go: hugolib/page__per_output.go:Fragments
    pub fn fragments(&self, ctx: HostCtx<'_>) -> Option<Arc<Fragments>> {
        self.c().fragments(ctx)
    }

    /// Go: `RenderShortcodes(ctx)`.
    // Go: hugolib/page__per_output.go:RenderShortcodes
    pub fn render_shortcodes(&self, ctx: HostCtx<'_>) -> Result<Value> {
        self.c().render_shortcodes(ctx)
    }

    /// Go: `Len(ctx)`.
    // Go: hugolib/page__per_output.go:Len
    pub fn len(&self, ctx: HostCtx<'_>) -> i64 {
        self.c().len(ctx)
    }

    /// Go: `fail(err)` — `h.FatalError(p.wrapError(err))`.
    // Go: hugolib/page__per_output.go:fail
    pub(crate) fn fail(&self, err: Error) {
        let h = self.h();
        let p = h.page(self.po.p);
        h.fatal_error_handler.fatal_error(p.wrap_error(err));
    }

    // Go: hugolib/page__per_output.go:Plain
    pub fn plain(&self, ctx: HostCtx<'_>, _p: &PageHandle) -> Result<Value> {
        Ok(Value::string(self.c().plain(ctx)))
    }

    /// Go: `PlainWords(ctx)` (`[]string`).
    // Go: hugolib/page__per_output.go:PlainWords
    pub fn plain_words(&self, ctx: HostCtx<'_>) -> Vec<GoString> {
        self.c().plain_words(ctx)
    }

    // Go: hugolib/page__per_output.go:ReadingTime
    pub fn reading_time(&self, ctx: HostCtx<'_>) -> i64 {
        self.c().reading_time(ctx)
    }

    // Go: hugolib/page__per_output.go:WordCount
    pub fn word_count(&self, ctx: HostCtx<'_>) -> i64 {
        self.c().word_count(ctx)
    }

    // Go: hugolib/page__per_output.go:FuzzyWordCount
    pub fn fuzzy_word_count(&self, ctx: HostCtx<'_>) -> i64 {
        self.c().fuzzy_word_count(ctx)
    }

    /// Go: `Summary(ctx)` — the summary text (`template.HTML`).
    // Go: hugolib/page__per_output.go:Summary
    pub fn summary(&self, ctx: HostCtx<'_>) -> Value {
        match self.c().summary(ctx) {
            Ok(s) => Value::html(s.text),
            Err(err) => {
                self.fail(err);
                Value::html(GoString::empty())
            }
        }
    }

    // Go: hugolib/page__per_output.go:Truncated
    pub fn truncated(&self, ctx: HostCtx<'_>) -> bool {
        match self.c().summary(ctx) {
            Ok(s) => s.truncated,
            Err(err) => {
                self.fail(err);
                false
            }
        }
    }

    /// Go: `RenderString(ctx, args...)` (markdownify: `display: inline`, fresh ID factory, TrimShortHTML).
    // Go: hugolib/page__per_output.go:RenderString
    pub fn render_string(
        &self,
        ctx: HostCtx<'_>,
        _p: &crate::page::PageHandle,
        args: &[Value],
    ) -> Result<Value> {
        self.c().render_string(ctx, args)
    }

    /// Go: `Render(ctx, layout...)` — executes the page's layout template `layout` with the
    /// page as data (`executeToString`).
    // Go: hugolib/page__per_output.go:Render
    pub fn render(&self, ctx: HostCtx<'_>, layout: &[String]) -> Result<Value> {
        if layout.is_empty() {
            return Err(Error::new("no layout given"));
        }
        let h = self.h();
        let p = h.page(self.po.p);
        let (dir, mut d) = p
            .current_output()
            .get_internal_template_base_path_and_descriptor(p);
        d.layout_from_user = layout[0].clone();
        d.layout_from_user_must_match = true;
        let q = TemplateQuery {
            path: dir,
            name: String::new(),
            category: Category::Layout,
            desc: d,
            consider: None,
        };
        let store = h.sites[p.site_idx].deps.get_template_store();
        let Some(templ) = store.lookup_pages_layout(&q) else {
            return Ok(Value::html(GoString::empty()));
        };
        // Make sure to send the *pageState and not the *pageContentOutput to the template.
        let data = PageHandle {
            h: h.clone(),
            id: self.po.p,
            wrapper: PageWrapper::None,
        }
        .page_ref()
        .to_value();
        let res = execute_to_string(&tpl_ctx(ctx), store, &templ, &data).map_err(|err| {
            p.wrap_error(err.wrap(format!("failed to execute template {}", templ.name())))
        })?;
        Ok(Value::html(res))
    }

    /// Go: `getRenderHooks` / `renderHooks.getRenderer` construction (`initRenderHooks`).
    // Go: hugolib/page__per_output.go:initRenderHooks
    pub fn init_render_hooks(&self, _p: &crate::page::PageHandle) -> GetRendererFunc {
        self.init_render_hooks_internal().clone()
    }

    /// Go: `initRenderHooks()` — runs once (Go `renderHooks.init.Do`): moves the page's template
    /// variations state from 0 to 1 and creates the renderer lookup.
    // Go: hugolib/page__per_output.go:initRenderHooks
    pub(crate) fn init_render_hooks_internal(&self) -> &GetRendererFunc {
        self.render_hooks.get_or_init(|| {
            let h = self.h();
            let ps = h.page(self.po.p);
            if ps
                .page_output_template_variations_state
                .load(Ordering::SeqCst)
                == 0
            {
                ps.page_output_template_variations_state
                    .store(1, Ordering::SeqCst);
            }
            new_get_renderer(self.po.clone())
        })
    }

    /// Go: `getContentConverter()` — `initRenderHooks` + the page's converter.
    // Go: hugolib/page__per_output.go:getContentConverter
    pub fn get_content_converter(&self) -> Result<Arc<dyn Converter>> {
        self.init_render_hooks_internal();
        let h = self.h();
        Ok(h.page(self.po.p).get_content_converter(&h, &self.po))
    }

    fn render_context<'a>(
        &self,
        ctx: &'a TplContext,
        src: &'a [u8],
        render_toc: bool,
    ) -> RenderContext<'a> {
        RenderContext {
            ctx: ctx.as_host(),
            src,
            render_toc,
            get_renderer: self.render_hooks.get().cloned(),
        }
    }

    /// Go: `ParseAndRenderContent(ctx, content, renderTOC)`.
    // Go: hugolib/page__per_output.go:ParseAndRenderContent
    pub fn parse_and_render_content(
        &self,
        ctx: &TplContext,
        content: &[u8],
        render_toc: bool,
    ) -> Result<ResultRender> {
        let c = self.get_content_converter()?;
        self.render_content_with_converter(ctx, &*c, content, render_toc)
    }

    /// Go: `ParseContent(ctx, content)` — `None` when the converter cannot parse separately
    /// (Go's `ok == false`).
    // Go: hugolib/page__per_output.go:ParseContent
    pub fn parse_content(&self, ctx: &TplContext, content: &[u8]) -> Result<Option<ResultParse>> {
        let c = self.get_content_converter()?;
        let Some(p) = c.as_parse_renderer() else {
            return Ok(None);
        };
        let rctx = self.render_context(ctx, content, true);
        p.parse(&rctx).map(Some)
    }

    /// Go: `RenderContent(ctx, content, doc)` — `None` when the converter cannot render a
    /// parsed document.
    // Go: hugolib/page__per_output.go:RenderContent
    pub fn render_content(
        &self,
        ctx: &TplContext,
        content: &[u8],
        doc: &ParsedDoc,
    ) -> Result<Option<ResultRender>> {
        let c = self.get_content_converter()?;
        let Some(p) = c.as_parse_renderer() else {
            return Ok(None);
        };
        let rctx = self.render_context(ctx, content, true);
        p.render(&rctx, doc).map(Some)
    }

    /// Go: `renderContentWithConverter(ctx, c, content, renderTOC)`.
    // Go: hugolib/page__per_output.go:renderContentWithConverter
    pub fn render_content_with_converter(
        &self,
        ctx: &TplContext,
        c: &dyn Converter,
        content: &[u8],
        render_toc: bool,
    ) -> Result<ResultRender> {
        let rctx = self.render_context(ctx, content, render_toc);
        c.convert(&rctx)
    }
}

impl nh_page::page_lazy_contentprovider::OutputFormatContentProvider for PageContentOutput {
    // Go: hugolib/page__per_output.go:Plain
    fn plain(&self, ctx: HostCtx<'_>) -> Result<go_value::GoString> {
        Ok(self.c().plain(ctx))
    }
    // Go: hugolib/page__per_output.go:Content
    fn content(&self, ctx: HostCtx<'_>) -> Result<Value> {
        self.c().content(ctx)
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

/// The key of the render hook cache (Go `cacheKey{tp, id, f}`; `f` is the content output's
/// format, fixed per cache).
type RenderCacheKey = (RendererType, Option<Vec<u8>>);

/// Go: the `pco.renderHooks.getRenderer` closure of `initRenderHooks` (page__per_output.go:
/// 267-396).
fn new_get_renderer(po: Arc<PageOutput>) -> GetRendererFunc {
    let render_cache: Mutex<HashMap<RenderCacheKey, Renderer>> = Mutex::new(HashMap::new());
    Arc::new(move |tp: RendererType, id: &Value| -> Option<Renderer> {
        let mut render_cache = render_cache.lock().unwrap_or_else(|e| e.into_inner());

        let id_key = match id {
            Value::String(s) => Some(s.as_bytes().to_vec()),
            _ => None,
        };
        let key = (tp, id_key.clone());
        if let Some(r) = render_cache.get(&key) {
            return Some(r.clone());
        }

        let h = upgrade_hs(&po.hs);
        let ps = h.page(po.p);
        let s = &h.sites[po.site_idx];

        // Inherit the descriptor from the page/current output format.
        // This allows for fine-grained control of the template used for
        // rendering of e.g. links.
        let (base, mut layout_descriptor) = ps
            .current_output()
            .get_internal_template_base_path_and_descriptor(ps);

        let id_string = id_key
            .as_ref()
            .map(|b| String::from_utf8_lossy(b).into_owned());
        match tp {
            RendererType::Link => layout_descriptor.variant1 = "link".into(),
            RendererType::Image => layout_descriptor.variant1 = "image".into(),
            RendererType::Heading => layout_descriptor.variant1 = "heading".into(),
            RendererType::Passthrough => {
                layout_descriptor.variant1 = "passthrough".into();
                if let Some(id) = &id_string {
                    layout_descriptor.variant2 = id.clone();
                }
            }
            RendererType::Blockquote => {
                layout_descriptor.variant1 = "blockquote".into();
                if let Some(id) = &id_string {
                    layout_descriptor.variant2 = id.clone();
                }
            }
            RendererType::Table => layout_descriptor.variant1 = "table".into(),
            RendererType::CodeBlock => {
                layout_descriptor.variant1 = "codeblock".into();
                if let Some(id) = &id_string {
                    layout_descriptor.variant2 = id.clone();
                }
            }
        }

        let render_hook_config = &s.conf.markup.goldmark.render_hooks;
        // For multilingual single-host sites, "auto" becomes "fallback"
        // earlier in the process.
        let ignore_embedded = match layout_descriptor.variant1.as_str() {
            "link" => {
                render_hook_config.link.use_embedded == RENDER_HOOK_USE_EMBEDDED_NEVER
                    || render_hook_config.link.use_embedded == RENDER_HOOK_USE_EMBEDDED_AUTO
            }
            "image" => {
                render_hook_config.image.use_embedded == RENDER_HOOK_USE_EMBEDDED_NEVER
                    || render_hook_config.image.use_embedded == RENDER_HOOK_USE_EMBEDDED_AUTO
            }
            _ => false,
        };

        let candidates = s.render_formats.clone();
        let num_candidates_found = Arc::new(AtomicI64::new(0));
        let consider = {
            let variant1 = layout_descriptor.variant1.clone();
            let variant2 = layout_descriptor.variant2.clone();
            let num = num_candidates_found.clone();
            let hs = po.hs.clone();
            let pid = po.p;
            move |candidate: &TemplInfo| -> bool {
                let d = candidate.d();
                if variant1 != d.variant1 {
                    return false;
                }

                if !variant2.is_empty() && !d.variant2.is_empty() && variant2 != d.variant2 {
                    return false;
                }

                if ignore_embedded && candidate.sub_category() == SubCategory::Embedded {
                    // Don't consider the embedded hook templates.
                    return false;
                }

                let h = upgrade_hs(&hs);
                if h.page(pid)
                    .page_output_template_variations_state
                    .load(Ordering::SeqCst)
                    > 1
                {
                    return true;
                }

                if d.output_format.is_empty() || candidates.get_by_name(&d.output_format).is_some()
                {
                    num.fetch_add(1, Ordering::SeqCst);
                }

                true
            }
        };

        let default_output_format_matches =
            layout_descriptor.output_format == s.conf.root.default_output_format;
        let q = TemplateQuery {
            path: base,
            name: String::new(),
            category: Category::Markup,
            desc: layout_descriptor,
            consider: Some(Arc::new(consider)),
        };
        let store = s.deps.get_template_store();
        let templ = store.lookup_pages_layout(&q);
        let found1 = templ.is_some();

        if !found1 && default_output_format_matches {
            num_candidates_found.fetch_add(1, Ordering::SeqCst);
        }

        if num_candidates_found.load(Ordering::SeqCst) > 1 {
            // More than one output format candidate found for this hook temoplate,
            // so we cannot reuse the same rendered content.
            ps.incr_page_output_template_variation();
        }

        let Some(templ) = templ else {
            if tp == RendererType::CodeBlock {
                // No user provided template for code blocks, so we use the native Go version -- which is also faster.
                let r = Renderer::CodeBlock(
                    s.deps
                        .content_spec()
                        .converters
                        .get_highlighter()
                        .as_code_block_renderer(),
                );
                render_cache.insert(key, r.clone());
                return Some(r);
            }
            return None;
        };

        let kind: &'static str = match tp {
            RendererType::Link => "link",
            RendererType::Image => "image",
            RendererType::Heading => "heading",
            RendererType::CodeBlock => "codeblock",
            RendererType::Passthrough => "passthrough",
            RendererType::Blockquote => "blockquote",
            RendererType::Table => "table",
        };

        let hr = Arc::new(HookRendererTemplate {
            template_handler: store.clone(),
            templ,
            hs: po.hs.clone(),
            site_idx: po.site_idx,
            page: po.p,
            output_format: po.f.name.clone(),
            kind,
        });
        let r = match tp {
            RendererType::Link | RendererType::Image => Renderer::Link(hr),
            RendererType::Heading => Renderer::Heading(hr),
            RendererType::CodeBlock => Renderer::CodeBlock(hr),
            RendererType::Passthrough => Renderer::Passthrough(hr),
            RendererType::Blockquote => Renderer::Blockquote(hr),
            RendererType::Table => Renderer::Table(hr),
        };
        render_cache.insert(key, r.clone());
        Some(r)
    })
}

/// Go: `hookRendererTemplate` (site.go:1494-1534): executes a hook template with the hook
/// context as data. The template context is the caller's `cctx`, passed through UNCHANGED (Go
/// `hr.templateHandler.ExecuteWithContext(cctx, hr.templ, w, ctx)`): no page is set on it and
/// `is_in_goldmark` is NOT set (that flag is only for `{{% %}}` shortcodes, shortcode.go:331; if
/// it leaked into hooks, `RenderShortcodes` would wrap its output in `hugocontext`).
/// Execution goes through `crate::template_exec::execute` with `ExecKind::Hook(<type>)`, so the
/// T22 tests can replay Go-recorded hook outputs.
pub struct HookRendererTemplate {
    pub template_handler: TemplateStore,
    pub templ: Arc<TemplInfo>,
    /// The page whose content output resolved the hook (Go's `resolvePosition` closure and the
    /// `template_exec` key).
    pub(crate) hs: HugoSitesRef,
    pub(crate) site_idx: usize,
    pub(crate) page: PageId,
    /// The content output's format name (Go `pco.po.f.Name`).
    pub(crate) output_format: String,
    /// The hook type (`hooks.RendererType`'s variant name: link, image, heading, ...).
    pub(crate) kind: &'static str,
}

impl HookRendererTemplate {
    /// Go: `hr.templateHandler.ExecuteWithContext(cctx, hr.templ, w, ctx)`.
    fn execute(&self, cctx: HostCtx<'_>, w: &mut Vec<u8>, ctx: &Value) -> Result<()> {
        let h = upgrade_hs(&self.hs);
        let call = ExecCall {
            page: Some(self.page),
            output_format: self.output_format.clone(),
            kind: ExecKind::Hook(self.kind),
            ordinal: 0,
        };
        match TplContext::from_host(cctx) {
            Some(tctx) => {
                crate::template_exec::execute(&h, self.site_idx, tctx, &self.templ, w, ctx, &call)
            }
            None => crate::template_exec::execute(
                &h,
                self.site_idx,
                &TplContext::default(),
                &self.templ,
                w,
                ctx,
                &call,
            ),
        }
    }

    /// Go: the `resolvePosition` closure of `initRenderHooks` (page__per_output.go:249-265):
    /// the position of the `PositionerSourceTarget` in the page source, moved up one line to the
    /// code fence delimiter.
    // Go: hugolib/site.go:ResolvePosition
    fn resolve(&self, target: &[u8]) -> Position {
        let h = upgrade_hs(&self.hs);
        let p = h.page(self.page);
        let source: &[u8] = p.content.as_ref().map(|c| c.must_source()).unwrap_or(&[]);
        let offset = go_unicode::strings::index(source, target) as i64;
        let mut pos = p.pos_from_input(source, offset);
        if pos.line_number > 0 {
            // Move up to the code fence delimiter.
            // This is in line with how we report on shortcodes.
            pos.line_number -= 1;
        }
        pos
    }
}

impl LinkRenderer for HookRendererTemplate {
    // Go: hugolib/site.go:RenderLink
    fn render_link(&self, cctx: HostCtx<'_>, w: &mut Vec<u8>, ctx: &Value) -> Result<()> {
        self.execute(cctx, w, ctx)
    }
}

impl HeadingRenderer for HookRendererTemplate {
    // Go: hugolib/site.go:RenderHeading
    fn render_heading(&self, cctx: HostCtx<'_>, w: &mut Vec<u8>, ctx: &Value) -> Result<()> {
        self.execute(cctx, w, ctx)
    }
}

impl TableRenderer for HookRendererTemplate {
    // Go: hugolib/site.go:RenderTable
    fn render_table(&self, cctx: HostCtx<'_>, w: &mut Vec<u8>, ctx: &Value) -> Result<()> {
        self.execute(cctx, w, ctx)
    }
    // Go: hugolib/site.go:ResolvePosition
    fn resolve_position(&self, positioner_source_target: &[u8]) -> Option<Position> {
        Some(self.resolve(positioner_source_target))
    }
}

impl CodeBlockRenderer for HookRendererTemplate {
    // Go: hugolib/site.go:RenderCodeblock
    fn render_codeblock(&self, cctx: HostCtx<'_>, w: &mut Vec<u8>, ctx: &Value) -> Result<()> {
        self.execute(cctx, w, ctx)
    }
    // Go: hugolib/site.go:IsDefaultCodeBlockRenderer
    fn is_default_code_block_renderer(&self) -> bool {
        false
    }
    // Go: hugolib/site.go:ResolvePosition
    fn resolve_position(&self, positioner_source_target: &[u8]) -> Option<Position> {
        Some(self.resolve(positioner_source_target))
    }
}

impl BlockquoteRenderer for HookRendererTemplate {
    // Go: hugolib/site.go:RenderBlockquote
    fn render_blockquote(&self, cctx: HostCtx<'_>, w: &mut Vec<u8>, ctx: &Value) -> Result<()> {
        self.execute(cctx, w, ctx)
    }
    // Go: hugolib/site.go:ResolvePosition
    fn resolve_position(&self, positioner_source_target: &[u8]) -> Option<Position> {
        Some(self.resolve(positioner_source_target))
    }
}

impl PassthroughRenderer for HookRendererTemplate {
    // Go: hugolib/site.go:RenderPassthrough
    fn render_passthrough(&self, cctx: HostCtx<'_>, w: &mut Vec<u8>, ctx: &Value) -> Result<()> {
        self.execute(cctx, w, ctx)
    }
    // Go: hugolib/site.go:ResolvePosition
    fn resolve_position(&self, positioner_source_target: &[u8]) -> Option<Position> {
        Some(self.resolve(positioner_source_target))
    }
}

/// Go: `executeToString(ctx, h, templ, data)`.
// Go: hugolib/page__per_output.go:executeToString
pub fn execute_to_string(
    ctx: &TplContext,
    h: &TemplateStore,
    templ: &Arc<TemplInfo>,
    data: &Value,
) -> Result<Vec<u8>> {
    let mut b = Vec::new();
    h.execute_with_context(ctx, templ, &mut b, data)?;
    Ok(b)
}

// ---------------------------------------------------------------------------
// hugolib/page.go + page__meta.go helpers used by content rendering.

impl PageState {
    /// Go: `HasShortcode(name)` — whether the page's content (or content it rendered or
    /// included: `RenderString`, `.RenderShortcodes`) uses the shortcode `name`. T23's method
    /// table calls this (page.go is T23's; the name set lives in the shortcode handler).
    // Go: hugolib/page.go:HasShortcode
    pub fn has_shortcode(&self, name: &str) -> bool {
        match &self.content {
            None => false,
            Some(c) => c.shortcode_state.has_name(name),
        }
    }

    /// Go: `pathOrTitle()` — the filename, else the path, else the title.
    // Go: hugolib/page.go:pathOrTitle
    pub fn path_or_title(&self) -> String {
        if let Some(f) = &self.meta.f {
            return f.filename().to_string();
        }
        let p = self.meta.path();
        if !p.is_empty() {
            return p;
        }
        self.meta.title().to_string()
    }

    /// Go: `posFromInput(input, offset)` with `pathOrTitle()` as the filename.
    // Go: hugolib/page.go:posFromInput
    pub fn pos_from_input(&self, input: &[u8], offset: i64) -> Position {
        pos_from_input(&self.path_or_title(), input, offset)
    }

    /// Go: `posOffset(offset)` — the position of a byte offset in the page source.
    // Go: hugolib/page.go:posOffset
    pub fn pos_offset(&self, offset: i64) -> Position {
        let source: &[u8] = self
            .content
            .as_ref()
            .map(|c| c.must_source())
            .unwrap_or(&[]);
        self.pos_from_input(source, offset)
    }

    /// Go: `parseError(err, input, offset)` — a file error at the offset's position.
    // Go: hugolib/page.go:parseError
    pub fn parse_error(&self, err: Error, input: &[u8], offset: i64) -> Error {
        let pos = pos_from_input("", input, offset);
        let filename = self
            .meta
            .f
            .as_ref()
            .map(|f| f.filename().to_string())
            .unwrap_or_default();
        nh_common::herrors::new_file_error_from_pos(
            err,
            FilePos {
                filename,
                line: pos.line_number,
                column: pos.column_number,
            },
        )
    }

    /// Go: `wrapError(err)` — `pageMeta.wrapError(err, sourceFs)`: `"%q: %w"` with the path
    /// for pages without a file, else the file info added to the error
    /// (`hugofs.AddFileInfoToError`: the filename as the error position when it has none).
    // Go: hugolib/page.go:wrapError
    pub fn wrap_error(&self, err: Error) -> Error {
        match &self.meta.f {
            None => err.wrap(go_strconv::quote(self.meta.path().as_bytes())),
            Some(f) => {
                if err.pos().is_some() {
                    return err;
                }
                nh_common::herrors::new_file_error_from_name(err, f.filename())
            }
        }
    }

    /// Go: `getPageInfoForError()`.
    // Go: hugolib/page.go:getPageInfoForError
    pub fn get_page_info_for_error(&self) -> String {
        let mut s = format!(
            "kind: {}, path: {}",
            go_strconv::quote(self.meta.kind().as_bytes()),
            go_strconv::quote(self.meta.path().as_bytes())
        );
        if let Some(f) = &self.meta.f {
            s.push_str(&format!(
                ", file: {}",
                go_strconv::quote(f.filename().as_bytes())
            ));
        }
        s
    }

    /// Go: `getContentConverter()` — created once (Go `contentConverterInit`; kept on the page
    /// output `po`, see `PageOutput::content_converter`): the converter of the content's markup
    /// (`html` content uses markdown, for shortcode inner content only). A creation error is
    /// logged and the nop converter used.
    // Go: hugolib/page.go:getContentConverter
    pub fn get_content_converter(&self, h: &Arc<HugoSites>, po: &PageOutput) -> Arc<dyn Converter> {
        po.content_converter
            .get_or_init(|| {
                let mt = &self.meta.page_config.content_media_type;
                if mt.is_zero() {
                    panic!("ContentMediaType not set");
                }
                let mut markup = mt.sub_type.clone();

                if markup == "html" {
                    // Only used for shortcode inner content.
                    markup = "markdown".to_string();
                }
                match new_content_converter(h, self, &markup) {
                    Ok(c) => c,
                    Err((c, err)) => {
                        h.sites[self.site_idx].deps.log.errorf(format!(
                            "Failed to create content converter: {}",
                            err.message()
                        ));
                        c
                    }
                }
            })
            .clone()
    }
}

/// Go: `(p *pageMeta) newContentConverter(ps, markup)` — the converter for `markup` with the
/// page as the document (`newPageForRenderHook`) and the hugocontext document lookup (the page
/// itself for its own pid, else the pages whose `.RenderShortcodes` output was included).
/// On error Go returns the nop converter with the error.
// Go: hugolib/page__meta.go:newContentConverter
pub fn new_content_converter(
    h: &Arc<HugoSites>,
    ps: &PageState,
    markup: &str,
) -> std::result::Result<Arc<dyn Converter>, (Arc<dyn Converter>, Error)> {
    let nop: Arc<dyn Converter> = Arc::new(NopConverter);
    let cp = h.sites[ps.site_idx]
        .deps
        .content_spec()
        .converters
        .get(markup);
    let Some(cp) = cp else {
        return Err((
            nop,
            Error::new(format!(
                "no content renderer found for markup {}, page: {}",
                go_strconv::quote(markup.as_bytes()),
                ps.get_page_info_for_error()
            )),
        ));
    };

    let mut id = String::new();
    let mut filename = String::new();
    let path;
    if let Some(f) = &ps.meta.f {
        id = f.unique_id().to_string();
        filename = f.filename().to_string();
        path = f.path();
    } else {
        path = ps.meta.path();
    }

    let doc = crate::shortcode_page::new_page_for_render_hook(h, ps.id);

    let hs = h.self_ref.clone();
    let pid = ps.pid;
    let page_id = ps.id;
    let doc2 = doc.clone();
    let document_lookup = move |id: u64| -> Value {
        if id == pid {
            // This prevents infinite recursion in some cases.
            return doc2.clone();
        }
        let h = upgrade_hs(&hs);
        let Some(pco) = h.page(page_id).current_output().pco() else {
            return Value::Invalid;
        };
        let v = pco
            .other_outputs
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .get(&id)
            .cloned();
        match v {
            Some(v) => PageHandle {
                h: h.clone(),
                id: v.po.p,
                wrapper: PageWrapper::None,
            }
            .page_ref()
            .to_value(),
            None => Value::Invalid,
        }
    };

    match cp.new_converter(DocumentContext {
        document: doc,
        document_lookup: Some(Arc::new(document_lookup)),
        document_id: id,
        document_name: path,
        filename,
    }) {
        Ok(c) => Ok(c),
        Err(err) => Err((nop, err)),
    }
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: hugolib/page__per_output.go (499 lines; 13/31 funcs executed)
//   types: renderHooks, pageContentOutput, pagePerOutputProviders, targetPather, targetPathsHolder
// OK L66-73: newPageContentOutput(po *pageOutput) (*pageContentOutput, error)
//    L95-97: (pco *pageContentOutput) trackDependency(idp identity.IdentityProvider)   (no identity tracking)
// OK L99-106: (pco *pageContentOutput) Reset()
// OK L108-127: (pco *pageContentOutput) Render(ctx context.Context, layout ...string) (template.HTML, error)
// OK L129-131: (pco *pageContentOutput) Fragments(ctx context.Context) *tableofcontents.Fragments
// OK L133-135: (pco *pageContentOutput) RenderShortcodes(ctx context.Context) (template.HTML, error)
// OK L137-146: (pco *pageContentOutput) Markup(opts ...any) page.Markup
// OK L148-150: (pco *pageContentOutput) c() page.Markup
// OK L152-158: (pco *pageContentOutput) Content(ctx context.Context) (any, error)
// OK L160-166: (pco *pageContentOutput) ContentWithoutSummary(ctx context.Context) (template.HTML, error)
// OK L168-170: (pco *pageContentOutput) TableOfContents(ctx context.Context) template.HTML
// OK L172-174: (pco *pageContentOutput) Len(ctx context.Context) int
// OK L176-182: (pco *pageContentOutput) mustRender(ctx context.Context) page.Content
// OK L184-186: (pco *pageContentOutput) fail(err error)
// OK L188-190: (pco *pageContentOutput) Plain(ctx context.Context) string
// OK L192-194: (pco *pageContentOutput) PlainWords(ctx context.Context) []string
// OK L196-198: (pco *pageContentOutput) ReadingTime(ctx context.Context) int
// OK L200-202: (pco *pageContentOutput) WordCount(ctx context.Context) int
// OK L204-206: (pco *pageContentOutput) FuzzyWordCount(ctx context.Context) int
// OK L208-214: (pco *pageContentOutput) Summary(ctx context.Context) template.HTML
// OK L216-222: (pco *pageContentOutput) Truncated(ctx context.Context) bool
// OK L224-226: (pco *pageContentOutput) RenderString(ctx context.Context, args ...any) (template.HTML, error)
// OK L228-398: (pco *pageContentOutput) initRenderHooks() error
// OK L400-405: (pco *pageContentOutput) getContentConverter() (converter.Converter, error)
// OK L407-413: (cp *pageContentOutput) ParseAndRenderContent(ctx context.Context, content []byte, renderTOC bool) (converter.ResultRender, error)
// OK L415-432: (pco *pageContentOutput) ParseContent(ctx context.Context, content []byte) (converter.ResultParse, bool, error)
// OK L434-451: (pco *pageContentOutput) RenderContent(ctx context.Context, content []byte, doc any) (converter.ResultRender, bool, error)
// OK L453-462: (pco *pageContentOutput) renderContentWithConverter(ctx context.Context, c converter.Converter, content []byte, renderTOC bool) (converter.ResultRe...
//    L484-486: (t targetPathsHolder) getRelURL() string                           (T21: page__paths.rs field)
// EX L488-490: (t targetPathsHolder) targetPaths() page.TargetPaths               (T21: the `paths` field of page__paths.rs TargetPathsHolder)
// OK L492-499: executeToString(ctx context.Context, h *tplimpl.TemplateStore, templ *tplimpl.TemplInfo, data any) (string, error)
// Source: hugolib/site.go (hookRendererTemplate only)
//   types: hookRendererTemplate
// OK L1504-1506: (hr hookRendererTemplate) RenderLink(cctx context.Context, w io.Writer, ctx hooks.LinkContext) error
// OK L1508-1510: (hr hookRendererTemplate) RenderHeading(cctx context.Context, w io.Writer, ctx hooks.HeadingContext) error
// OK L1512-1514: (hr hookRendererTemplate) RenderCodeblock(cctx context.Context, w hugio.FlexiWriter, ctx hooks.CodeblockContext) error
// OK L1516-1518: (hr hookRendererTemplate) RenderPassthrough(cctx context.Context, w io.Writer, ctx hooks.PassthroughContext) error
// OK L1520-1522: (hr hookRendererTemplate) RenderBlockquote(cctx context.Context, w hugio.FlexiWriter, ctx hooks.BlockquoteContext) error
// OK L1524-1526: (hr hookRendererTemplate) RenderTable(cctx context.Context, w hugio.FlexiWriter, ctx hooks.TableContext) error
// OK L1528-1530: (hr hookRendererTemplate) ResolvePosition(ctx any) text.Position
// OK L1532-1534: (hr hookRendererTemplate) IsDefaultCodeBlockRenderer() bool
// ---------------------------------------------------------------------------
