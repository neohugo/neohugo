//! Port of `markup/goldmark/convert.go`.
//!
//! Owner: Wave B task T06 (markup).

//! Go `markup/goldmark/convert.go`: the goldmark instance (extensions in the exact order of
//! specs/markdown.md §3), `Parse`/`Render` with a per-document ID factory, and the typographer map.
//!
//! **Rendering.** Go hands its `*render.Context` to goldmark's `Renderer.Render` as the
//! `util.BufWriter`, and Hugo's node renderers type-assert it back to reach the render context
//! (hooks, the template context, the document). The Rust render context borrows the caller's
//! template context (`HostCtx<'a>`), which cannot travel through goldmark's `'static` node
//! renderer functions. So the converter builds the node renderer table itself, exactly as
//! goldmark's `renderer.Render` does (the same renderers, priorities, pdqsort tie order, options
//! and registration order), and walks the document with it: goldmark's renderers get the
//! context's buffer as their `util.BufWriter`, Hugo's renderers get the context itself.

use std::collections::HashMap;
use std::sync::Arc;

use goldmark::ast::{self, Ast, NodeId, NodeKind, WalkStatus};
use goldmark::extension::{self, TypographicPunctuation};
use goldmark::parser::{self, OptionValue, ParserOption};
use goldmark::renderer::html;
use goldmark::renderer::{
    self, NodeRenderer, NodeRendererFunc, NodeRendererFuncRegisterer, RendererOption,
};
use goldmark::util::{self, BufWriter as _, PrioritizedValue};
use goldmark::{Extender, Markdown};
use nh_common::Result;
use nh_common::herrors::{self, Error};
use nh_common::loggers::Logger;
use nh_common::text::Position;

use super::autoid::{IdFactory, sanitize_anchor_name};
use super::goldmark_config::Typographer;
use super::internal::render::{Context as RenderCtx, RenderContextDataHolder};
use super::toc::{TOC_ENABLE_KEY, TOC_RESULT_KEY, TocResult, new_toc_extension};
use super::{blockquotes, codeblocks, hugocontext, images, render_hooks, tables};
use crate::converter::converter::{
    Converter, DocumentContext, ParseRenderer, ParsedDoc, Provider, ProviderConfig, RenderContext,
    ResultParse, ResultRender, new_provider,
};
use crate::markup_config::Config as MarkupConfig;

/// Don't change this. This pattern is also used in the image render hooks.
#[allow(dead_code)]
const INTERNAL_ATTR_PREFIX: &str = "_h__";

/// Go: `goldmark.Provider` (`provide.New(cfg)`).
pub struct GoldmarkProvider;

impl GoldmarkProvider {
    /// Go: `provide.New(cfg converter.ProviderConfig) (converter.Provider, error)`.
    // Go: markup/goldmark/convert.go:New
    #[allow(clippy::new_ret_no_self)] // the skeleton's contract (Go returns the interface)
    pub fn new(cfg: ProviderConfig) -> Result<Arc<dyn Provider>> {
        let mcfg = cfg.markup_config();
        Self::new_from_config(mcfg, cfg.conf.enable_emoji(), cfg.logger.clone())
    }

    /// `New` with the parts of `converter.ProviderConfig` it reads: the markup config
    /// (`Conf.GetConfigSection("markup")`), `Conf.EnableEmoji()` and the logger.
    pub fn new_from_config(
        mcfg: Arc<MarkupConfig>,
        enable_emoji: bool,
        logger: Option<Arc<Logger>>,
    ) -> Result<Arc<dyn Provider>> {
        let md = Arc::new(new_markdown(&mcfg, enable_emoji, logger)?);

        Ok(new_provider(
            "goldmark",
            Arc::new(move |ctx: DocumentContext| {
                Ok(Arc::new(GoldmarkConverter {
                    md: md.clone(),
                    ctx,
                    mcfg: mcfg.clone(),
                }) as Arc<dyn Converter>)
            }),
        ))
    }
}

/// Go: `goldmarkConverter`.
pub struct GoldmarkConverter {
    /// The configured goldmark instance with Hugo's node renderer table.
    pub(crate) md: Arc<HugoMarkdown>,
    pub ctx: DocumentContext,
    pub(crate) mcfg: Arc<MarkupConfig>,
}

/// The parse result (Go: the `ast.Node` document of `parserResult.Doc()`).
pub struct ParsedDocument {
    pub ast: Ast,
    pub root: NodeId,
}

impl Converter for GoldmarkConverter {
    // Go: markup/goldmark/convert.go:Convert
    fn convert(&self, ctx: &RenderContext<'_>) -> Result<ResultRender> {
        let parse_result = self.parse(ctx)?;
        let mut render_result = self.render(ctx, &parse_result.doc)?;
        render_result.table_of_contents = parse_result.table_of_contents;
        Ok(render_result)
    }

    fn as_parse_renderer(&self) -> Option<&dyn ParseRenderer> {
        Some(self)
    }

    // Go: markup/goldmark/convert.go:SanitizeAnchorName
    fn sanitize_anchor_name(&self, s: &str) -> Option<String> {
        Some(sanitize_anchor_name(
            s,
            &self.mcfg.goldmark.parser.auto_id_type,
        ))
    }
}

impl ParseRenderer for GoldmarkConverter {
    // Go: markup/goldmark/convert.go:Parse
    fn parse(&self, ctx: &RenderContext<'_>) -> Result<ResultParse> {
        let mut pctx = self.new_parser_context(ctx);

        let doc = self.md.md.parse(ctx.src, &mut pctx);

        // Go: parserContext.TableOfContents (nil unless the TOC transformer ran).
        let toc = pctx
            .get_as::<TocResult>(*TOC_RESULT_KEY)
            .map(|t| t.0.clone());

        Ok(ResultParse {
            doc: Arc::new(ParsedDocument {
                ast: doc.ast,
                root: doc.root,
            }),
            table_of_contents: toc,
        })
    }

    // Go: markup/goldmark/convert.go:Render
    fn render(&self, ctx: &RenderContext<'_>, doc: &ParsedDoc) -> Result<ResultRender> {
        let n = doc
            .downcast_ref::<ParsedDocument>()
            .expect("interface conversion: interface {} is not ast.Node");

        let rcx = RenderContextDataHolder {
            rctx: ctx,
            dctx: &self.ctx,
        };

        let mut w = RenderCtx::new(rcx);

        self.md
            .render(&mut w, ctx.src, &n.ast, n.root)
            .map_err(from_goldmark_error)?;

        Ok(ResultRender {
            bytes: w.w.buf,
            table_of_contents: None,
        })
    }
}

impl GoldmarkConverter {
    // Go: markup/goldmark/convert.go:newParserContext
    fn new_parser_context(&self, rctx: &RenderContext<'_>) -> parser::Context {
        let mut ctx = parser::new_context(vec![parser::with_ids(Box::new(IdFactory::new(
            &self.mcfg.goldmark.parser.auto_id_type,
        )))]);
        ctx.set_value(*TOC_ENABLE_KEY, rctx.render_toc);
        ctx
    }
}

// ---------------------------------------------------------------------------
// The node renderer table (Go: goldmark's renderer.Render with Hugo's renderers).

/// A render function in the table: goldmark's (on the buffer) or Hugo's (on the context).
#[derive(Clone)]
pub(crate) enum RenderFunc {
    Goldmark(NodeRendererFunc),
    Hugo(HugoRenderFunc),
}

type HugoRenderFunc = Arc<
    dyn for<'a, 'b> Fn(&'b mut RenderCtx<'a>, &[u8], &Ast, NodeId, bool) -> Result2 + Send + Sync,
>;

type Result2 = std::result::Result<WalkStatus, goldmark::Error>;

impl RenderFunc {
    pub(crate) fn hugo<F>(f: F) -> RenderFunc
    where
        F: for<'a, 'b> Fn(&'b mut RenderCtx<'a>, &[u8], &Ast, NodeId, bool) -> Result2
            + Send
            + Sync
            + 'static,
    {
        RenderFunc::Hugo(Arc::new(f))
    }
}

/// Go `renderer.NodeRenderer` for Hugo's renderers (they render with the render context).
pub(crate) trait HugoNodeRenderer: Send + Sync {
    /// RegisterFuncs registers the render functions.
    fn register_funcs(self: Arc<Self>, reg: &mut HugoRegisterer);
    /// SetOptioner.SetOption.
    fn set_option(&mut self, name: &str, value: &OptionValue);
}

/// The registry both kinds of node renderers register into (Go's `renderer.Register` into
/// `nodeRendererFuncsTmp`, later registrations win).
pub(crate) struct HugoRegisterer {
    funcs: HashMap<NodeKind, RenderFunc>,
    max_kind: usize,
}

impl HugoRegisterer {
    // Go: renderer/renderer.go:renderer.Register
    pub(crate) fn register(&mut self, kind: NodeKind, f: RenderFunc) {
        self.funcs.insert(kind, f);
        if kind.0 as usize > self.max_kind {
            self.max_kind = kind.0 as usize;
        }
    }
}

impl NodeRendererFuncRegisterer for HugoRegisterer {
    fn register(&mut self, kind: NodeKind, f: NodeRendererFunc) {
        HugoRegisterer::register(self, kind, RenderFunc::Goldmark(f));
    }
}

/// A node renderer of either kind (Go's `util.PrioritizedValue` of a `renderer.NodeRenderer`).
enum RendererEntry {
    Goldmark(Box<dyn NodeRenderer>),
    Hugo(Box<dyn HugoNodeRenderer>),
}

/// Go: `renderer.renderer` after its lazy init: the render funcs by node kind.
// Go: renderer/renderer.go:renderer.Render (the init part)
fn build_render_funcs(
    mut node_renderers: Vec<PrioritizedValue<RendererEntry>>,
    options: &HashMap<String, OptionValue>,
) -> Vec<Option<RenderFunc>> {
    util::sort_prioritized(&mut node_renderers);
    let mut reg = HugoRegisterer {
        funcs: HashMap::new(),
        max_kind: 0,
    };
    let l = node_renderers.len();
    let mut nrs: Vec<Option<RendererEntry>> =
        node_renderers.into_iter().map(|v| Some(v.value)).collect();
    for i in (0..l).rev() {
        match nrs[i].take().expect("renderer entry") {
            RendererEntry::Goldmark(mut nr) => {
                for (oname, ovalue) in options {
                    nr.set_option(oname, ovalue);
                }
                let nr: Arc<dyn NodeRenderer> = Arc::from(nr);
                nr.register_funcs(&mut reg);
            }
            RendererEntry::Hugo(mut nr) => {
                for (oname, ovalue) in options {
                    nr.set_option(oname, ovalue);
                }
                let nr: Arc<dyn HugoNodeRenderer> = Arc::from(nr);
                nr.register_funcs(&mut reg);
            }
        }
    }
    let mut funcs: Vec<Option<RenderFunc>> = vec![None; reg.max_kind + 1];
    for (kind, f) in reg.funcs {
        funcs[kind.0 as usize] = Some(f);
    }
    funcs
}

/// The configured goldmark instance (Go: `goldmark.Markdown` from `newMarkdown`) with the node
/// renderer table of its renderer.
pub(crate) struct HugoMarkdown {
    pub(crate) md: Markdown,
    funcs: Vec<Option<RenderFunc>>,
}

impl HugoMarkdown {
    /// Go: `c.md.Renderer().Render(w, source, n)`.
    // Go: renderer/renderer.go:renderer.Render
    fn render(
        &self,
        w: &mut RenderCtx<'_>,
        source: &[u8],
        a: &Ast,
        n: NodeId,
    ) -> std::result::Result<(), goldmark::Error> {
        let funcs = &self.funcs;
        ast::walk_ref(a, n, &mut |a, n, entering| {
            let mut s = WalkStatus::Continue;
            let kind = a.kind(n).0 as usize;
            // Go indexes the slice by kind and panics past its end.
            let Some(f) = funcs.get(kind) else {
                panic!(
                    "runtime error: index out of range [{kind}] with length {}",
                    funcs.len()
                );
            };
            if let Some(f) = f {
                s = match f {
                    RenderFunc::Goldmark(f) => f(&mut w.w, source, a, n, entering)?,
                    RenderFunc::Hugo(f) => f(w, source, a, n, entering)?,
                };
            }
            Ok(s)
        })?;
        w.w.flush()
    }
}

/// The renderer options of `newMarkdown` (Go builds the slice once and passes it twice; the
/// Rust options are consumed, so they are built per use).
fn renderer_options(mcfg: &MarkupConfig) -> Vec<Box<dyn RendererOption>> {
    let cfg = &mcfg.goldmark;
    let mut renderer_options: Vec<Box<dyn RendererOption>> = Vec::new();

    if cfg.renderer.hard_wraps {
        renderer_options.push(Box::new(html::with_hard_wraps()));
    }

    if cfg.renderer.xhtml {
        renderer_options.push(Box::new(html::with_xhtml()));
    }

    if cfg.renderer.unsafe_ {
        renderer_options.push(Box::new(html::with_unsafe()));
    }
    renderer_options
}

fn unsupported(what: &str) -> Error {
    Error::new(format!("neohugo-rs: {what} is not supported"))
}

// Go: markup/goldmark/convert.go:newMarkdown
pub(crate) fn new_markdown(
    mcfg: &MarkupConfig,
    enable_emoji: bool,
    logger: Option<Arc<Logger>>,
) -> Result<HugoMarkdown> {
    let cfg = &mcfg.goldmark;

    let renderer_options = renderer_options(mcfg);

    let mut toc_renderer_options = self::renderer_options(mcfg);
    toc_renderer_options.push(renderer::with_node_renderers(vec![util::prioritized(
        extension::new_strikethrough_html_renderer(Vec::new()),
        500,
    )]));
    // Go also adds goldmark-emoji's renderer (priority 200); it only renders emoji nodes,
    // which only exist with the emoji extension (an explicit error below).

    // The node renderers of the main renderer, in the order Go registers them: the default
    // HTML renderer, then each extension's in extension order.
    let mut node_renderers: Vec<PrioritizedValue<RendererEntry>> = vec![util::prioritized(
        RendererEntry::Goldmark(html::new_renderer(Vec::new())),
        1000,
    )];
    // The renderer options: WithRendererOptions, then the CJK extension's.
    let mut renderer_config = renderer::new_config();

    let hugo_ctx = hugocontext::new(logger);
    node_renderers.push(util::prioritized(
        RendererEntry::Hugo(Box::new(hugo_ctx.node_renderer())),
        50,
    ));
    // newLinks(cfg)
    node_renderers.push(util::prioritized(
        RendererEntry::Hugo(Box::new(render_hooks::new_link_renderer(cfg))),
        100,
    ));
    // blockquotes.New()
    node_renderers.push(util::prioritized(
        RendererEntry::Hugo(Box::new(blockquotes::new_html_renderer())),
        100,
    ));

    let mut extensions: Vec<Box<dyn Extender>> = vec![
        Box::new(hugo_ctx),
        // newLinks and blockquotes.New only add node renderers (above).
        Box::new(new_toc_extension(toc_renderer_options)),
    ];
    let mut parser_options: Vec<Box<dyn ParserOption>> = Vec::new();

    extensions.push(images::new(
        cfg.parser.wrap_stand_alone_image_within_paragraph,
    ));

    // extras.New(...): with every extra disabled it adds nothing.
    let extras = &cfg.extensions.extras;
    if extras.delete.enable
        || extras.insert.enable
        || extras.mark.enable
        || extras.subscript.enable
        || extras.superscript.enable
    {
        return Err(unsupported(
            "goldmark extras (delete, insert, mark, subscript, superscript)",
        ));
    }

    if mcfg.highlight.code_fences {
        node_renderers.push(util::prioritized(
            RendererEntry::Hugo(Box::new(codeblocks::new_html_renderer())),
            100,
        ));
    }

    if cfg.extensions.table {
        extensions.push(extension::table());
        node_renderers.push(util::prioritized(
            RendererEntry::Goldmark(extension::new_table_html_renderer(&[])),
            500,
        ));
        node_renderers.push(util::prioritized(
            RendererEntry::Hugo(Box::new(tables::new_html_renderer())),
            100,
        ));
    }

    if cfg.extensions.strikethrough {
        extensions.push(extension::strikethrough());
        node_renderers.push(util::prioritized(
            RendererEntry::Goldmark(extension::new_strikethrough_html_renderer(Vec::new())),
            500,
        ));
    }

    if cfg.extensions.linkify {
        extensions.push(extension::linkify());
    }

    if cfg.extensions.task_list {
        extensions.push(extension::task_list());
        node_renderers.push(util::prioritized(
            RendererEntry::Goldmark(extension::new_task_check_box_html_renderer(Vec::new())),
            500,
        ));
    }

    if !cfg.extensions.typographer.disable {
        let t = extension::new_typographer(vec![extension::with_typographic_substitutions(
            &to_typographic_punctuation_map(&cfg.extensions.typographer),
        )]);
        extensions.push(t);
    }

    if cfg.extensions.definition_list {
        extensions.push(extension::definition_list());
        node_renderers.push(util::prioritized(
            RendererEntry::Goldmark(extension::new_definition_list_html_renderer(Vec::new())),
            500,
        ));
    }

    if cfg.extensions.footnote {
        extensions.push(extension::footnote());
        node_renderers.push(util::prioritized(
            RendererEntry::Goldmark(extension::new_footnote_html_renderer(&[])),
            500,
        ));
    }

    let mut cjk_renderer_options: Vec<Box<dyn RendererOption>> = Vec::new();
    if cfg.extensions.cjk.enable {
        let mut opts: Vec<extension::CJKOption> = Vec::new();
        let mut style = html::EastAsianLineBreaks::None;
        if cfg.extensions.cjk.east_asian_line_breaks {
            if cfg.extensions.cjk.east_asian_line_breaks_style == "css3draft" {
                opts.push(extension::with_east_asian_line_breaks(&[
                    extension::EastAsianLineBreaks::Css3Draft,
                ]));
                style = html::EastAsianLineBreaks::Css3Draft;
            } else {
                opts.push(extension::with_east_asian_line_breaks(&[]));
                style = html::EastAsianLineBreaks::Simple;
            }
        }

        if cfg.extensions.cjk.escaped_space {
            opts.push(extension::with_escaped_space());
        }
        // The renderer options the CJK extension adds to the renderer (extension/cjk.go).
        cjk_renderer_options.push(Box::new(html::with_east_asian_line_breaks(style)));
        if cfg.extensions.cjk.escaped_space {
            cjk_renderer_options.push(Box::new(html::with_writer(html::new_writer(vec![
                html::with_escaped_space(),
            ]))));
        }
        let c = extension::new_cjk(opts);
        extensions.push(c);
    }

    if cfg.extensions.passthrough.enable {
        return Err(unsupported("the goldmark passthrough extension"));
    }

    if enable_emoji {
        return Err(unsupported("enableEmoji (goldmark-emoji)"));
    }

    if cfg.parser.attribute.title {
        parser_options.push(parser::with_attribute());
    }

    if cfg.parser.attribute.block
        || cfg.parser.auto_heading_id
        || cfg.parser.auto_definition_term_id
    {
        extensions.push(super::internal::extensions::attributes::new(
            cfg.parser.clone(),
        ));
    }

    // The main renderer's options (WithRendererOptions, then the CJK extension's).
    for opt in self::renderer_options(mcfg) {
        opt.set_config(&mut renderer_config);
    }
    for opt in cjk_renderer_options {
        opt.set_config(&mut renderer_config);
    }

    let md = goldmark::new(vec![
        goldmark::with_extensions(extensions),
        goldmark::with_parser_options(parser_options),
        goldmark::with_renderer_options(renderer_options),
    ]);

    let funcs = build_render_funcs(node_renderers, &renderer_config.options);

    Ok(HugoMarkdown { md, funcs })
}

/// Go: `toTypographicPunctuationMap` (every value, even an empty one, is a substitution).
// Go: markup/goldmark/convert.go:toTypographicPunctuationMap
fn to_typographic_punctuation_map(t: &Typographer) -> Vec<(TypographicPunctuation, Vec<u8>)> {
    vec![
        (
            TypographicPunctuation::LeftSingleQuote,
            t.left_single_quote.as_bytes().to_vec(),
        ),
        (
            TypographicPunctuation::RightSingleQuote,
            t.right_single_quote.as_bytes().to_vec(),
        ),
        (
            TypographicPunctuation::LeftDoubleQuote,
            t.left_double_quote.as_bytes().to_vec(),
        ),
        (
            TypographicPunctuation::RightDoubleQuote,
            t.right_double_quote.as_bytes().to_vec(),
        ),
        (
            TypographicPunctuation::EnDash,
            t.en_dash.as_bytes().to_vec(),
        ),
        (
            TypographicPunctuation::EmDash,
            t.em_dash.as_bytes().to_vec(),
        ),
        (
            TypographicPunctuation::Ellipsis,
            t.ellipsis.as_bytes().to_vec(),
        ),
        (
            TypographicPunctuation::LeftAngleQuote,
            t.left_angle_quote.as_bytes().to_vec(),
        ),
        (
            TypographicPunctuation::RightAngleQuote,
            t.right_angle_quote.as_bytes().to_vec(),
        ),
        (
            TypographicPunctuation::Apostrophe,
            t.apostrophe.as_bytes().to_vec(),
        ),
    ]
}

// ---------------------------------------------------------------------------
// Errors crossing goldmark's walk.

/// A hook error carried through goldmark's walk as a `goldmark::Error`.
#[derive(Debug)]
struct HookError(Error);

impl std::fmt::Display for HookError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(&self.0, f)
    }
}

impl std::error::Error for HookError {}

pub(crate) fn to_goldmark_error(err: Error) -> goldmark::Error {
    Box::new(HookError(err))
}

fn from_goldmark_error(err: goldmark::Error) -> Error {
    match err.downcast::<HookError>() {
        Ok(h) => h.0,
        Err(e) => Error::new(e.to_string()),
    }
}

/// Go: `herrors.NewFileErrorFromPos(err, ctx.Position())`.
pub(crate) fn file_error_from_pos(err: Error, pos: Position) -> goldmark::Error {
    to_goldmark_error(herrors::new_file_error_from_pos(
        err,
        herrors::FilePos {
            filename: pos.filename,
            line: pos.line_number,
            column: pos.column_number,
        },
    ))
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: markup/goldmark/convert.go (327 lines; 10/11 funcs executed)
//   types: provide, goldmarkConverter, parserResult, renderResult, converterResult, tableOfContentsProvider,
//          parserContext
// OK L55-68: (p provide) New(cfg converter.ProviderConfig) (converter.Provider, error)
// OK L80-82: (c *goldmarkConverter) SanitizeAnchorName(s string) string
// OK L84-212: newMarkdown(pcfg converter.ProviderConfig) goldmark.Markdown
// OK L219-221: (p parserResult) Doc() any
// OK L223-225: (p parserResult) TableOfContents() *tableofcontents.Fragments
// OK L240-253: (c *goldmarkConverter) Parse(ctx converter.RenderContext) (converter.ResultParse, error)
// OK L255-276: (c *goldmarkConverter) Render(ctx converter.RenderContext, doc any) (converter.ResultRender, error)
// OK L278-291: (c *goldmarkConverter) Convert(ctx converter.RenderContext) (converter.ResultRender, error)
// OK L293-299: (c *goldmarkConverter) newParserContext(rctx converter.RenderContext) *parserContext
// OK L305-310: (p *parserContext) TableOfContents() *tableofcontents.Fragments
// OK L314-327: toTypographicPunctuationMap(t goldmark_config.Typographer) map[extension.TypographicPunctuation][]byte
// ---------------------------------------------------------------------------
