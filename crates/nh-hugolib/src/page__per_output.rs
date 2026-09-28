//! Port of `hugolib/page__per_output.go`.
//!
//! Owner: Wave B task T22 (hugolib-content).

//! Go `hugolib/page__per_output.go`: per-output content (`.Content`, `.Plain`, `.Summary`, TOC,
//! `RenderString`) and the render-hook provider: for each hook type look up
//! `_markup/render-<type>` via `LookupPagesLayout` (CategoryMarkup, Variant1=type, output format of
//! the CURRENT output — `index.json` renders content in the json context and gets the embedded
//! `render-table.json.json` text template), wrapped in `HookRendererTemplate`.

use std::sync::{Arc, OnceLock};

use go_value::{HostCtx, Value};
use nh_common::Result;
use nh_markup::converter::hooks::{GetRendererFunc, HeadingRenderer, LinkRenderer, TableRenderer};
use nh_tplimpl::templatestore::{TemplInfo, TemplateStore};

/// Go: `pageContentOutput`.
pub struct PageContentOutput {
    /// Index of the output this content was rendered for.
    pub output_idx: usize,
    /// Go `renderHooks` (lazily resolved GetRendererFunc).
    pub render_hooks: OnceLock<GetRendererFunc>,
}

impl PageContentOutput {
    /// Go: `newPageContentOutput(po)`.
    // Go: hugolib/page__per_output.go:newPageContentOutput
    pub fn new(
        po: &Arc<crate::page__output::PageOutput>,
        output_idx: usize,
    ) -> Result<Arc<PageContentOutput>> {
        todo!()
    }

    /// Go: `Content(ctx)` -> `template.HTML`.
    // Go: hugolib/page__per_output.go:Content
    pub fn content(&self, ctx: HostCtx<'_>, p: &crate::page::PageHandle) -> Result<Value> {
        todo!()
    }

    // Go: hugolib/page__per_output.go:Plain
    pub fn plain(&self, ctx: HostCtx<'_>, p: &crate::page::PageHandle) -> Result<Value> {
        todo!()
    }

    /// Go: `RenderString(ctx, args...)` (markdownify: `display: inline`, fresh ID factory, TrimShortHTML).
    // Go: hugolib/page__per_output.go:RenderString
    pub fn render_string(
        &self,
        ctx: HostCtx<'_>,
        p: &crate::page::PageHandle,
        args: &[Value],
    ) -> Result<Value> {
        todo!()
    }

    /// Go: `getRenderHooks` / `renderHooks.getRenderer` construction.
    // Go: hugolib/page__per_output.go:initRenderHooks
    pub fn init_render_hooks(&self, p: &crate::page::PageHandle) -> GetRendererFunc {
        todo!()
    }
}

impl nh_page::page_lazy_contentprovider::OutputFormatContentProvider for PageContentOutput {
    // Go: hugolib/page__per_output.go:Plain
    fn plain(&self, ctx: HostCtx<'_>) -> Result<go_value::GoString> {
        todo!()
    }
    // Go: hugolib/page__per_output.go:Content
    fn content(&self, ctx: HostCtx<'_>) -> Result<Value> {
        todo!()
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
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
}

impl LinkRenderer for HookRendererTemplate {
    // Go: hugolib/site.go:RenderLink
    fn render_link(&self, cctx: HostCtx<'_>, w: &mut Vec<u8>, ctx: &Value) -> Result<()> {
        todo!()
    }
}

impl HeadingRenderer for HookRendererTemplate {
    // Go: hugolib/site.go:RenderHeading
    fn render_heading(&self, cctx: HostCtx<'_>, w: &mut Vec<u8>, ctx: &Value) -> Result<()> {
        todo!()
    }
}

impl TableRenderer for HookRendererTemplate {
    // Go: hugolib/site.go:RenderTable
    fn render_table(&self, cctx: HostCtx<'_>, w: &mut Vec<u8>, ctx: &Value) -> Result<()> {
        todo!()
    }
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: hugolib/page__per_output.go (499 lines; 13/31 funcs executed)
//   types: renderHooks, pageContentOutput, pagePerOutputProviders, targetPather, targetPathsHolder
// EX L66-73: newPageContentOutput(po *pageOutput) (*pageContentOutput, error)
//    L95-97: (pco *pageContentOutput) trackDependency(idp identity.IdentityProvider)
//    L99-106: (pco *pageContentOutput) Reset()
//    L108-127: (pco *pageContentOutput) Render(ctx context.Context, layout ...string) (template.HTML, error)
//    L129-131: (pco *pageContentOutput) Fragments(ctx context.Context) *tableofcontents.Fragments
//    L133-135: (pco *pageContentOutput) RenderShortcodes(ctx context.Context) (template.HTML, error)
//    L137-146: (pco *pageContentOutput) Markup(opts ...any) page.Markup
// EX L148-150: (pco *pageContentOutput) c() page.Markup
// EX L152-158: (pco *pageContentOutput) Content(ctx context.Context) (any, error)
//    L160-166: (pco *pageContentOutput) ContentWithoutSummary(ctx context.Context) (template.HTML, error)
//    L168-170: (pco *pageContentOutput) TableOfContents(ctx context.Context) template.HTML
//    L172-174: (pco *pageContentOutput) Len(ctx context.Context) int
// EX L176-182: (pco *pageContentOutput) mustRender(ctx context.Context) page.Content
//    L184-186: (pco *pageContentOutput) fail(err error)
// EX L188-190: (pco *pageContentOutput) Plain(ctx context.Context) string
//    L192-194: (pco *pageContentOutput) PlainWords(ctx context.Context) []string
//    L196-198: (pco *pageContentOutput) ReadingTime(ctx context.Context) int
//    L200-202: (pco *pageContentOutput) WordCount(ctx context.Context) int
//    L204-206: (pco *pageContentOutput) FuzzyWordCount(ctx context.Context) int
//    L208-214: (pco *pageContentOutput) Summary(ctx context.Context) template.HTML
//    L216-222: (pco *pageContentOutput) Truncated(ctx context.Context) bool
// EX L224-226: (pco *pageContentOutput) RenderString(ctx context.Context, args ...any) (template.HTML, error)
// EX L228-398: (pco *pageContentOutput) initRenderHooks() error
// EX L400-405: (pco *pageContentOutput) getContentConverter() (converter.Converter, error)
// EX L407-413: (cp *pageContentOutput) ParseAndRenderContent(ctx context.Context, content []byte, renderTOC bool) (converter.ResultRender, error)
// EX L415-432: (pco *pageContentOutput) ParseContent(ctx context.Context, content []byte) (converter.ResultParse, bool, error)
// EX L434-451: (pco *pageContentOutput) RenderContent(ctx context.Context, content []byte, doc any) (converter.ResultRender, bool, error)
// EX L453-462: (pco *pageContentOutput) renderContentWithConverter(ctx context.Context, c converter.Converter, content []byte, renderTOC bool) (converter.ResultRe...
//    L484-486: (t targetPathsHolder) getRelURL() string
// EX L488-490: (t targetPathsHolder) targetPaths() page.TargetPaths
//    L492-499: executeToString(ctx context.Context, h *tplimpl.TemplateStore, templ *tplimpl.TemplInfo, data any) (string, error)
// Source: hugolib/site.go (hookRendererTemplate only)
//   types: hookRendererTemplate
// EX L1504-1506: (hr hookRendererTemplate) RenderLink(cctx context.Context, w io.Writer, ctx hooks.LinkContext) error
// EX L1508-1510: (hr hookRendererTemplate) RenderHeading(cctx context.Context, w io.Writer, ctx hooks.HeadingContext) error
//    L1512-1514: (hr hookRendererTemplate) RenderCodeblock(cctx context.Context, w hugio.FlexiWriter, ctx hooks.CodeblockContext) error
//    L1516-1518: (hr hookRendererTemplate) RenderPassthrough(cctx context.Context, w io.Writer, ctx hooks.PassthroughContext) error
//    L1520-1522: (hr hookRendererTemplate) RenderBlockquote(cctx context.Context, w hugio.FlexiWriter, ctx hooks.BlockquoteContext) error
// EX L1524-1526: (hr hookRendererTemplate) RenderTable(cctx context.Context, w hugio.FlexiWriter, ctx hooks.TableContext) error
//    L1528-1530: (hr hookRendererTemplate) ResolvePosition(ctx any) text.Position
//    L1532-1534: (hr hookRendererTemplate) IsDefaultCodeBlockRenderer() bool
// ---------------------------------------------------------------------------
