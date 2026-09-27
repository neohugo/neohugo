//! Validates the extension (plugin) API against Go: the test extension of
//! tools/go-oracle/goldmark/plugin.go re-implemented on the Rust API, plus a
//! port of goldmark's extension.Strikethrough (delimiter processor + custom
//! inline node + node renderer at priority 500). The oracle renders the
//! same inputs with the Go versions ("plugin" / "plugin-unsafe" configs).
//!
//! This file doubles as the reference example for extension authors (Hugo's
//! hugocontext/attributes/render hooks follow the same patterns).

// Mirrors the Go code (`t.Segment.Len() == 0`).
#![allow(clippy::len_zero)]

mod common;

use std::any::Any;
use std::sync::{Arc, LazyLock};

use goldmark::ast::{self, Ast, CustomNode, NodeId, NodeKind, NodeType, NodeValue, WalkStatus};
use goldmark::parser::{
    self, AstTransformer, BlockParser, Context, Delimiter, DelimiterProcessor, InlineParser,
    OptionValue, State,
};
use goldmark::renderer::html;
use goldmark::renderer::{self, NodeRenderer, NodeRendererFuncRegisterer};
use goldmark::text::Reader;
use goldmark::util::{self, BufWriter};
use goldmark::{Extender, Markdown};

// ---------------------------------------------------------------------------
// extension.Strikethrough (github.com/yuin/goldmark@v1.7.12/extension/strikethrough.go)

static KIND_STRIKETHROUGH: LazyLock<NodeKind> =
    LazyLock::new(|| ast::new_node_kind("Strikethrough"));

#[derive(Debug)]
struct Strikethrough;

impl CustomNode for Strikethrough {
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

struct StrikethroughDelimiterProcessor;

impl DelimiterProcessor for StrikethroughDelimiterProcessor {
    fn is_delimiter(&self, b: u8) -> bool {
        b == b'~'
    }
    fn can_open_closer(&self, opener: &Delimiter, closer: &Delimiter) -> bool {
        opener.char == closer.char
    }
    fn on_match(&self, ast: &mut Ast, _consumes: i64) -> NodeId {
        ast.new_custom_node(
            *KIND_STRIKETHROUGH,
            NodeType::Inline,
            Box::new(Strikethrough),
        )
    }
}

static STRIKE_PROCESSOR: LazyLock<Arc<dyn DelimiterProcessor>> =
    LazyLock::new(|| Arc::new(StrikethroughDelimiterProcessor));

struct StrikethroughParser;

impl InlineParser for StrikethroughParser {
    fn trigger(&self) -> &[u8] {
        b"~"
    }
    fn parse<'a>(
        &self,
        ast: &mut Ast,
        _parent: NodeId,
        block: &mut dyn Reader<'a>,
        pc: &mut Context,
    ) -> Option<NodeId> {
        let before = block.precending_character();
        let (line, segment) = block.peek_line();
        let line = line?;
        let mut node = parser::scan_delimiter(&line, before, 1, STRIKE_PROCESSOR.clone())?;
        if node.original_length > 2 || before == '~' as i32 {
            return None;
        }
        node.segment = segment.with_stop(segment.start + node.original_length);
        block.advance(node.original_length);
        let id = ast.new_delimiter(node);
        pc.push_delimiter(ast, id);
        Some(id)
    }
}

struct StrikethroughHTMLRenderer {
    #[allow(dead_code)]
    config: html::Config,
}

impl StrikethroughHTMLRenderer {
    fn render(
        &self,
        w: &mut dyn BufWriter,
        _s: &[u8],
        ast: &Ast,
        n: NodeId,
        entering: bool,
    ) -> Result<WalkStatus, goldmark::Error> {
        if entering {
            if ast.attributes(n).is_some() {
                w.write_string("<del");
                html::render_attributes(w, ast, n, Some(&html::GLOBAL_ATTRIBUTE_FILTER));
                w.write_byte(b'>');
            } else {
                w.write_string("<del>");
            }
        } else {
            w.write_string("</del>");
        }
        Ok(WalkStatus::Continue)
    }
}

impl NodeRenderer for StrikethroughHTMLRenderer {
    fn register_funcs(self: Arc<Self>, reg: &mut dyn NodeRendererFuncRegisterer) {
        reg.register(
            *KIND_STRIKETHROUGH,
            html::bind(&self, StrikethroughHTMLRenderer::render),
        );
    }
}

struct StrikethroughExt;

impl Extender for StrikethroughExt {
    fn extend(&self, m: &mut Markdown) {
        m.parser()
            .add_options(vec![parser::with_inline_parsers(vec![util::prioritized(
                Box::new(StrikethroughParser) as Box<dyn InlineParser>,
                500,
            )])]);
        m.renderer()
            .add_options(vec![renderer::with_node_renderers(vec![
                util::prioritized(
                    Box::new(StrikethroughHTMLRenderer {
                        config: html::new_config(),
                    }) as Box<dyn NodeRenderer>,
                    500,
                ),
            ])]);
    }
}

// ---------------------------------------------------------------------------
// The oracle's test extension (plugin.go).

static KIND_MUSTACHE: LazyLock<NodeKind> = LazyLock::new(|| ast::new_node_kind("Mustache"));
static KIND_NOTE: LazyLock<NodeKind> = LazyLock::new(|| ast::new_node_kind("Note"));

#[derive(Debug)]
struct Mustache {
    name: Vec<u8>,
}

impl CustomNode for Mustache {
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

#[derive(Debug)]
struct Note;

impl CustomNode for Note {
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

struct MustacheParser;

impl InlineParser for MustacheParser {
    fn trigger(&self) -> &[u8] {
        b"{"
    }
    fn parse<'a>(
        &self,
        ast: &mut Ast,
        _parent: NodeId,
        reader: &mut dyn Reader<'a>,
        _pc: &mut Context,
    ) -> Option<NodeId> {
        let (line, _) = reader.peek_line();
        let line = line.unwrap_or_default();
        if !line.starts_with(b"{{") {
            return None;
        }
        let end = line.windows(2).position(|w| w == b"}}")?;
        reader.advance(end as i64 + 2);
        Some(ast.new_custom_node(
            *KIND_MUSTACHE,
            NodeType::Inline,
            Box::new(Mustache {
                name: line[2..end].to_vec(),
            }),
        ))
    }
}

struct NoteParser;

impl NoteParser {
    fn process<'a>(&self, reader: &mut dyn Reader<'a>) -> bool {
        let (line, _) = reader.peek_line();
        let line = line.unwrap_or_default();
        let lo = reader.line_offset();
        let (w, pos) = util::indent_width(&line, lo);
        let pos = pos as usize;
        if w > 3 || pos + 1 >= line.len() || line[pos] != b'%' || line[pos + 1] != b'%' {
            return false;
        }
        reader.advance(pos as i64 + 2);
        true
    }
}

impl BlockParser for NoteParser {
    fn trigger(&self) -> Option<&[u8]> {
        Some(b"%")
    }
    fn open<'a>(
        &self,
        ast: &mut Ast,
        _parent: NodeId,
        reader: &mut dyn Reader<'a>,
        _pc: &mut Context,
    ) -> (Option<NodeId>, State) {
        if self.process(reader) {
            return (
                Some(ast.new_custom_node(*KIND_NOTE, NodeType::Block, Box::new(Note))),
                State::HAS_CHILDREN,
            );
        }
        (None, State::NO_CHILDREN)
    }
    fn continue_<'a>(
        &self,
        _ast: &mut Ast,
        _node: NodeId,
        reader: &mut dyn Reader<'a>,
        _pc: &mut Context,
    ) -> State {
        if self.process(reader) {
            return State::CONTINUE | State::HAS_CHILDREN;
        }
        State::CLOSE
    }
    fn close<'a>(
        &self,
        _ast: &mut Ast,
        _node: NodeId,
        _reader: &mut dyn Reader<'a>,
        _pc: &mut Context,
    ) {
    }
    fn can_interrupt_paragraph(&self) -> bool {
        true
    }
    fn can_accept_indented_line(&self) -> bool {
        false
    }
}

struct MustacheTransformer;

impl AstTransformer for MustacheTransformer {
    fn transform<'a>(
        &self,
        ast: &mut Ast,
        doc: NodeId,
        _reader: &mut dyn Reader<'a>,
        _pc: &mut Context,
    ) {
        let _ = ast::walk(ast, doc, &mut |ast, n, entering| {
            if !entering || ast.kind(n) != *KIND_MUSTACHE {
                return Ok(WalkStatus::Continue);
            }
            let p = ast.parent(n).unwrap();
            if matches!(ast.value(p), NodeValue::Paragraph) {
                if ast.child_count(p) == 1 {
                    let pp = ast.parent(p).unwrap();
                    ast.replace_child(pp, p, n);
                } else if let Some(t) = ast.previous_sibling(n)
                    && let Some(tt) = ast.text_node(t)
                    && tt.soft_line_break()
                {
                    if tt.segment.len() == 0 {
                        ast.remove_child(p, t);
                    } else {
                        ast.text_node_mut(t).unwrap().set_soft_line_break(false);
                    }
                }
            }
            Ok(WalkStatus::Continue)
        });
    }
}

struct PluginRenderer {
    config: html::Config,
}

impl NodeRenderer for PluginRenderer {
    fn set_option(&mut self, name: &str, value: &OptionValue) {
        self.config.set_option(name, value);
    }

    fn register_funcs(self: Arc<Self>, reg: &mut dyn NodeRendererFuncRegisterer) {
        let r = self.clone();
        reg.register(
            *KIND_MUSTACHE,
            Arc::new(move |w, _src, ast, n, entering| {
                if entering {
                    w.write_string("<span class=\"m\">");
                    r.config
                        .writer
                        .write(w, &ast.custom::<Mustache>(n).unwrap().name);
                    w.write_string("</span>");
                }
                Ok(WalkStatus::Continue)
            }),
        );
        reg.register(
            *KIND_NOTE,
            Arc::new(|w, _src, _ast, _n, entering| {
                w.write_string(if entering { "<aside>\n" } else { "</aside>\n" });
                Ok(WalkStatus::Continue)
            }),
        );
        let r = self.clone();
        reg.register(
            ast::KIND_RAW_HTML,
            Arc::new(move |w, src, ast, n, entering| {
                if !entering {
                    return Ok(WalkStatus::SkipChildren);
                }
                if r.config.unsafe_ {
                    w.write_string("[raw:");
                    let segs = &ast.raw_html(n).unwrap().segments;
                    for i in 0..segs.len() {
                        w.write(&segs.at(i).value(src));
                    }
                    w.write_string("]");
                } else {
                    w.write_string("[omitted]");
                }
                Ok(WalkStatus::SkipChildren)
            }),
        );
    }
}

struct PluginExt;

impl Extender for PluginExt {
    fn extend(&self, m: &mut Markdown) {
        m.parser().add_options(vec![
            parser::with_inline_parsers(vec![util::prioritized(
                Box::new(MustacheParser) as Box<dyn InlineParser>,
                50,
            )]),
            parser::with_block_parsers(vec![util::prioritized(
                Box::new(NoteParser) as Box<dyn BlockParser>,
                850,
            )]),
            parser::with_ast_transformers(vec![util::prioritized(
                Box::new(MustacheTransformer) as Box<dyn AstTransformer>,
                10,
            )]),
        ]);
        m.renderer()
            .add_options(vec![renderer::with_node_renderers(vec![
                util::prioritized(
                    Box::new(PluginRenderer {
                        config: html::new_config(),
                    }) as Box<dyn NodeRenderer>,
                    50,
                ),
            ])]);
    }
}

fn plugin_markdown(unsafe_: bool) -> Markdown {
    let mut opts = vec![
        goldmark::with_extensions(vec![Box::new(StrikethroughExt), Box::new(PluginExt)]),
        goldmark::with_parser_options(vec![
            parser::with_attribute(),
            Box::new(parser::with_auto_heading_id()),
        ]),
    ];
    if unsafe_ {
        opts.push(goldmark::with_renderer_options(vec![Box::new(
            html::with_unsafe(),
        )]));
    }
    goldmark::new(opts)
}

#[test]
fn plugin_extension_matches_go() {
    let recs = common::fixture("plugin.gmf.gz");
    check_plugin(&recs);
    assert!(recs.len() > 4000);
}

/// `GOLDMARK_PLUGIN_FUZZ=path.gmf cargo test --release --test plugin_api -- --ignored`
/// (corpora from `goldmark fuzz -mode plugin -cfg plugin,plugin-unsafe`).
#[test]
#[ignore]
fn external_plugin_fuzz() {
    let Ok(paths) = std::env::var("GOLDMARK_PLUGIN_FUZZ") else {
        return;
    };
    for p in paths.split(',') {
        let recs = common::parse_gmf(&common::read_file(std::path::Path::new(p)));
        check_plugin(&recs);
        eprintln!("{p}: {} records identical", recs.len());
    }
}

fn check_plugin(recs: &[common::Record]) {
    let mds = [plugin_markdown(false), plugin_markdown(true)];
    let prev = std::panic::take_hook();
    std::panic::set_hook(Box::new(|_| {}));
    let mut fails = Vec::new();
    for r in recs {
        let md = match r.str("cfg").as_str() {
            "plugin" => &mds[0],
            "plugin-unsafe" => &mds[1],
            c => panic!("{c}"),
        };
        let got = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let mut out: Vec<u8> = Vec::new();
            md.convert(r.get("md"), &mut out).unwrap();
            out
        }))
        .unwrap_or_else(|_| b"PANIC".to_vec());
        let want = r.get("html");
        let ok = if want.starts_with(b"PANIC: ") {
            got == b"PANIC"
        } else {
            got == want
        };
        if !ok {
            fails.push(format!(
                "{} md={:?}\n want={:?}\n got ={:?}",
                r.name,
                String::from_utf8_lossy(r.get("md")),
                String::from_utf8_lossy(want),
                String::from_utf8_lossy(&got)
            ));
        }
    }
    std::panic::set_hook(prev);
    assert!(
        fails.is_empty(),
        "{}/{} differ:\n{}",
        fails.len(),
        recs.len(),
        fails[..fails.len().min(10)].join("\n")
    );
}

#[test]
fn custom_writer_downcast() {
    // Hugo's renderers downcast the writer (`w.(*render.Context)`).
    struct Ctx {
        buf: Vec<u8>,
        seen: usize,
    }
    impl BufWriter for Ctx {
        fn write(&mut self, p: &[u8]) {
            self.buf.extend_from_slice(p);
        }
        fn available(&self) -> i64 {
            i64::MAX
        }
        fn buffered(&self) -> i64 {
            self.buf.len() as i64
        }
        fn flush(&mut self) -> Result<(), goldmark::Error> {
            Ok(())
        }
        fn as_any_mut(&mut self) -> &mut dyn Any {
            self
        }
    }
    struct Counter;
    impl NodeRenderer for Counter {
        fn register_funcs(self: Arc<Self>, reg: &mut dyn NodeRendererFuncRegisterer) {
            reg.register(
                ast::KIND_EMPHASIS,
                Arc::new(|w, _s, _a, _n, entering| {
                    if entering {
                        let ctx = w.as_any_mut().downcast_mut::<Ctx>().unwrap();
                        ctx.seen += 1;
                    }
                    Ok(WalkStatus::Continue)
                }),
            );
        }
    }
    let md = goldmark::new(vec![goldmark::with_renderer_options(vec![
        renderer::with_node_renderers(vec![util::prioritized(
            Box::new(Counter) as Box<dyn NodeRenderer>,
            100,
        )]),
    ])]);
    let mut ctx = Ctx {
        buf: Vec::new(),
        seen: 0,
    };
    md.convert(b"*a* **b**", &mut ctx).unwrap();
    assert_eq!(ctx.seen, 2);
    assert_eq!(String::from_utf8(ctx.buf).unwrap(), "<p>a b</p>\n");
}
