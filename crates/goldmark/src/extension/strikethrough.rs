// Go: github.com/yuin/goldmark@v1.7.12/extension/strikethrough.go

use std::sync::{Arc, LazyLock};

use super::ast as east;
use crate::ast::{Ast, NodeId, WalkStatus};
use crate::parser::{self, Context, Delimiter, DelimiterProcessor, InlineParser, OptionValue};
use crate::renderer::html::{self, HtmlOption};
use crate::renderer::{self, NodeRenderer, NodeRendererFuncRegisterer};
use crate::text::Reader;
use crate::util::{self, BufWriter, BytesFilter};
use crate::{Extender, Markdown};

struct StrikethroughDelimiterProcessor;

impl DelimiterProcessor for StrikethroughDelimiterProcessor {
    // Go: extension/strikethrough.go:strikethroughDelimiterProcessor.IsDelimiter
    fn is_delimiter(&self, b: u8) -> bool {
        b == b'~'
    }

    // Go: extension/strikethrough.go:strikethroughDelimiterProcessor.CanOpenCloser
    fn can_open_closer(&self, opener: &Delimiter, closer: &Delimiter) -> bool {
        opener.char == closer.char
    }

    // Go: extension/strikethrough.go:strikethroughDelimiterProcessor.OnMatch
    fn on_match(&self, ast: &mut Ast, _consumes: i64) -> NodeId {
        east::new_strikethrough(ast)
    }
}

static DEFAULT_STRIKETHROUGH_DELIMITER_PROCESSOR: LazyLock<Arc<dyn DelimiterProcessor>> =
    LazyLock::new(|| Arc::new(StrikethroughDelimiterProcessor));

struct StrikethroughParser;

// Go: extension/strikethrough.go:NewStrikethroughParser
/// NewStrikethroughParser return a new InlineParser that parses
/// strikethrough expressions.
pub fn new_strikethrough_parser() -> Box<dyn InlineParser> {
    Box::new(StrikethroughParser)
}

impl InlineParser for StrikethroughParser {
    // Go: extension/strikethrough.go:strikethroughParser.Trigger
    fn trigger(&self) -> &[u8] {
        b"~"
    }

    // Go: extension/strikethrough.go:strikethroughParser.Parse
    fn parse<'a>(
        &self,
        ast: &mut Ast,
        _parent: NodeId,
        block: &mut dyn Reader<'a>,
        pc: &mut Context,
    ) -> Option<NodeId> {
        let before = block.precending_character();
        let (line, segment) = block.peek_line();
        let line = line.unwrap_or_default();
        let node = parser::scan_delimiter(
            &line,
            before,
            1,
            DEFAULT_STRIKETHROUGH_DELIMITER_PROCESSOR.clone(),
        );
        let mut node = match node {
            Some(n) if !(n.original_length > 2 || before == '~' as i32) => n,
            _ => return None,
        };
        node.segment = segment.with_stop(segment.start + node.original_length);
        block.advance(node.original_length);
        let id = ast.new_delimiter(node);
        pc.push_delimiter(ast, id);
        Some(id)
    }

    // Go: extension/strikethrough.go:strikethroughParser.CloseBlock(parent, pc) does not match
    // parser.CloseBlocker (which takes a text.Reader too), so Go never
    // registers it as a close blocker; it is a no-op anyway.
}

/// StrikethroughHTMLRenderer is a renderer.NodeRenderer implementation that
/// renders Strikethrough nodes.
pub struct StrikethroughHTMLRenderer {
    pub config: html::Config,
}

// Go: extension/strikethrough.go:NewStrikethroughHTMLRenderer
/// NewStrikethroughHTMLRenderer returns a new StrikethroughHTMLRenderer.
pub fn new_strikethrough_html_renderer(opts: Vec<Box<dyn HtmlOption>>) -> Box<dyn NodeRenderer> {
    let mut r = StrikethroughHTMLRenderer {
        config: html::new_config(),
    };
    for opt in opts {
        opt.set_html_option(&mut r.config);
    }
    Box::new(r)
}

/// StrikethroughAttributeFilter defines attribute names which dd elements can have.
pub static STRIKETHROUGH_ATTRIBUTE_FILTER: LazyLock<BytesFilter> =
    LazyLock::new(|| html::GLOBAL_ATTRIBUTE_FILTER.clone());

impl StrikethroughHTMLRenderer {
    // Go: extension/strikethrough.go:StrikethroughHTMLRenderer.renderStrikethrough
    fn render_strikethrough(
        &self,
        w: &mut dyn BufWriter,
        _source: &[u8],
        ast: &Ast,
        n: NodeId,
        entering: bool,
    ) -> Result<WalkStatus, crate::Error> {
        if entering {
            if ast.attributes(n).is_some() {
                w.write_string("<del");
                html::render_attributes(w, ast, n, Some(&STRIKETHROUGH_ATTRIBUTE_FILTER));
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
    // Go: extension/strikethrough.go:StrikethroughHTMLRenderer.RegisterFuncs
    fn register_funcs(self: Arc<Self>, reg: &mut dyn NodeRendererFuncRegisterer) {
        reg.register(
            *east::KIND_STRIKETHROUGH,
            html::bind(&self, StrikethroughHTMLRenderer::render_strikethrough),
        );
    }

    // Go: the embedded html.Config's SetOption
    fn set_option(&mut self, name: &str, value: &OptionValue) {
        self.config.set_option(name, value);
    }
}

/// Go: `type strikethrough struct{}`.
pub struct StrikethroughExt;

// Go: extension/strikethrough.go:Strikethrough
/// Strikethrough is an extension that allow you to use strikethrough expression like '~~text~~' .
pub fn strikethrough() -> Box<dyn Extender> {
    Box::new(StrikethroughExt)
}

impl Extender for StrikethroughExt {
    // Go: extension/strikethrough.go:strikethrough.Extend
    fn extend(&self, m: &mut Markdown) {
        m.parser()
            .add_options(vec![parser::with_inline_parsers(vec![util::prioritized(
                new_strikethrough_parser(),
                500,
            )])]);
        m.renderer()
            .add_options(vec![renderer::with_node_renderers(vec![
                util::prioritized(new_strikethrough_html_renderer(Vec::new()), 500),
            ])]);
    }
}
