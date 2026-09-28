//! Port of `markup/goldmark/hugocontext/hugocontext.go`.
//!
//! Owner: Wave B task T06 (markup).

use std::sync::{Arc, LazyLock};

use goldmark::ast::{self, Ast, CustomNode, NodeId, NodeKind, NodeType, NodeValue, WalkStatus};
use goldmark::parser::{self, Context, InlineParser};
use goldmark::renderer::html::Config as HtmlConfig;
use goldmark::text::Reader;
use goldmark::util;
use goldmark::{Extender, Markdown};
use nh_common::loggers::Logger;

use super::convert::{HugoNodeRenderer, HugoRegisterer, RenderFunc};
use super::internal::render::{self, Context as RenderCtx};

// Go: markup/goldmark/hugocontext/hugocontext.go:New
pub fn new(logger: Option<Arc<Logger>>) -> HugoContextExtension {
    HugoContextExtension { logger }
}

/// Wrap wraps the given byte slice in a Hugo context that used to determine the correct Page
/// in .RenderShortcodes.
// Go: markup/goldmark/hugocontext/hugocontext.go:Wrap
pub fn wrap(b: &[u8], pid: u64) -> Vec<u8> {
    let mut buf = Vec::with_capacity(b.len() + 40);
    buf.extend_from_slice(HUGO_CTX_PREFIX);
    buf.extend_from_slice(b" pid=");
    buf.extend_from_slice(pid.to_string().as_bytes());
    buf.extend_from_slice(HUGO_CTX_END_DELIM);
    buf.push(b'\n');
    buf.extend_from_slice(b);
    // To make sure that we're able to parse it, make sure it ends with a newline.
    if !b.is_empty() && b[b.len() - 1] != b'\n' {
        buf.push(b'\n');
    }
    buf.extend_from_slice(HUGO_CTX_PREFIX);
    buf.extend_from_slice(HUGO_CTX_CLOSING_DELIM);
    buf.push(b'\n');
    buf
}

static KIND_HUGO_CONTEXT: LazyLock<NodeKind> = LazyLock::new(|| ast::new_node_kind("HugoContext"));

/// HugoContext is a node that represents a Hugo context.
#[derive(Debug, Default)]
pub struct HugoContext {
    pub closing: bool,
    /// Internal page ID. Not persisted.
    pub pid: u64,
}

impl HugoContext {
    // Go: markup/goldmark/hugocontext/hugocontext.go:parseAttrs
    fn parse_attrs(&mut self, attr_bytes: &[u8]) {
        for key_pair in attr_bytes.split(|&c| c == b' ') {
            let kv: Vec<&[u8]> = key_pair.split(|&c| c == b'=').collect();
            if kv.len() != 2 {
                continue;
            }
            if kv[0] == b"pid" {
                // Go: `pid, _ := strconv.ParseUint(val, 10, 64)` (0 on error).
                self.pid = go_strconv::parse_uint(kv[1], 10, 64).unwrap_or(0);
            }
        }
    }
}

impl CustomNode for HugoContext {
    // Go: markup/goldmark/hugocontext/hugocontext.go:Dump
    fn dump_fields(&self, _source: &[u8]) -> Vec<(String, String)> {
        vec![("Pid".to_string(), self.pid.to_string())]
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

const HUGO_CTX_PREFIX: &[u8] = b"{{__hugo_ctx";
const HUGO_CTX_END_DELIM: &[u8] = b"}}";
const HUGO_CTX_CLOSING_DELIM: &[u8] = b"/}}";

/// Go `hugoCtxRe.ReplaceAll(b, nil)` with `hugoCtxRe = {{__hugo_ctx( pid=\d+)?/?}}\n?`
/// (a hand-written matcher; `\d` is ASCII in Go).
fn hugo_ctx_re_replace_all(b: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if let Some(n) = hugo_ctx_re_match_at(&b[i..]) {
            i += n;
        } else {
            out.push(b[i]);
            i += 1;
        }
    }
    out
}

/// The length of the leftmost-first match of `hugoCtxRe` at the start of `s`.
fn hugo_ctx_re_match_at(s: &[u8]) -> Option<usize> {
    if !s.starts_with(HUGO_CTX_PREFIX) {
        return None;
    }
    let rest = |mut j: usize| -> Option<usize> {
        if s.get(j) == Some(&b'/') {
            j += 1;
        }
        if !s[j.min(s.len())..].starts_with(b"}}") {
            return None;
        }
        j += 2;
        if s.get(j) == Some(&b'\n') {
            j += 1;
        }
        Some(j)
    };
    let j = HUGO_CTX_PREFIX.len();
    // `( pid=\d+)?` is greedy: try the group first.
    if s[j..].starts_with(b" pid=") {
        let mut k = j + 5;
        let start = k;
        while k < s.len() && s[k].is_ascii_digit() {
            k += 1;
        }
        if k > start
            && let Some(end) = rest(k)
        {
            return Some(end);
        }
    }
    rest(j)
}

struct HugoContextParser;

impl InlineParser for HugoContextParser {
    // Go: markup/goldmark/hugocontext/hugocontext.go:Trigger
    fn trigger(&self) -> &[u8] {
        b"{"
    }

    // Go: markup/goldmark/hugocontext/hugocontext.go:Parse
    fn parse<'a>(
        &self,
        ast: &mut Ast,
        _parent: NodeId,
        reader: &mut dyn Reader<'a>,
        _pc: &mut Context,
    ) -> Option<NodeId> {
        let (line, _) = reader.peek_line();
        let line = line?;
        if !line.starts_with(HUGO_CTX_PREFIX) {
            return None;
        }
        let end = go_unicode::bytes::index(&line, HUGO_CTX_END_DELIM);
        if end == -1 {
            return None;
        }
        let end = end as usize;

        reader.advance((end + HUGO_CTX_END_DELIM.len() + 1) as i64); // +1 for the newline

        if line[end - 1] == b'/' {
            return Some(ast.new_custom_node(
                *KIND_HUGO_CONTEXT,
                NodeType::Inline,
                Box::new(HugoContext {
                    closing: true,
                    pid: 0,
                }),
            ));
        }

        // Go: `line[len(hugoCtxPrefix)+1 : end]` (panics when end < len+1, as in Go).
        let attr_bytes = &line[HUGO_CTX_PREFIX.len() + 1..end];
        let mut h = HugoContext::default();
        h.parse_attrs(attr_bytes);
        Some(ast.new_custom_node(*KIND_HUGO_CONTEXT, NodeType::Inline, Box::new(h)))
    }
}

/// Go: `hugoContextRenderer` (renders HugoContext, RawHTML and HTMLBlock nodes).
pub(crate) struct HugoContextRenderer {
    logger: Option<Arc<Logger>>,
    config: HtmlConfig,
}

impl HugoContextRenderer {
    // Go: markup/goldmark/hugocontext/hugocontext.go:stripHugoCtx
    fn strip_hugo_ctx(&self, b: &[u8]) -> (Vec<u8>, bool) {
        if go_unicode::bytes::index(b, HUGO_CTX_PREFIX) == -1 {
            return (b.to_vec(), false);
        }
        (hugo_ctx_re_replace_all(b), true)
    }

    // Go: markup/goldmark/hugocontext/hugocontext.go:logRawHTMLEmittedWarn
    fn log_raw_html_emitted_warn(&self, w: &RenderCtx<'_>) {
        if let Some(l) = &self.logger {
            l.warnidf(
                nh_common::constants::WARN_GOLDMARK_RAW_HTML,
                format!(
                    "Raw HTML omitted while rendering {}; see https://gohugo.io/getting-started/configuration-markup/#rendererunsafe",
                    self.get_page(w)
                ),
            );
        }
    }

    /// The page for log messages (Go formats it with `%q`; the port quotes its `String()` or
    /// its type name: log text only).
    // Go: markup/goldmark/hugocontext/hugocontext.go:getPage
    fn get_page(&self, w: &RenderCtx<'_>) -> String {
        let (p, _) = render::get_page_and_page_inner(w);
        let s = match &p {
            go_value::Value::Object(o) => o
                .go_string()
                .map(|s| s.to_str_lossy().into_owned())
                .unwrap_or_else(|| o.type_name().into_owned()),
            go_value::Value::String(s) => s.to_str_lossy().into_owned(),
            other => other.go_type_name().into_owned(),
        };
        go_strconv::quote(s.as_bytes())
    }

    // Go: markup/goldmark/hugocontext/hugocontext.go:isHTMLComment
    fn is_html_comment(&self, b: &[u8]) -> bool {
        b.len() > 4 && b[0] == b'<' && b[1] == b'!' && b[2] == b'-' && b[3] == b'-'
    }

    /// HTML rendering based on Goldmark implementation.
    // Go: markup/goldmark/hugocontext/hugocontext.go:renderHTMLBlock
    fn render_html_block(
        &self,
        w: &mut RenderCtx<'_>,
        source: &[u8],
        ast: &Ast,
        node: NodeId,
        entering: bool,
    ) -> Result<WalkStatus, goldmark::Error> {
        let NodeValue::HTMLBlock(n) = ast.value(node) else {
            unreachable!("HTMLBlock renderer on another node")
        };
        let lines = ast.lines(node);

        if entering {
            if self.config.unsafe_ {
                let l = lines.len();
                for i in 0..l {
                    let line = lines.at(i);
                    let linev = line.value(source);
                    let (linev, stripped) = self.strip_hugo_ctx(&linev);
                    if stripped && let Some(l) = &self.logger {
                        l.warnidf(
                            nh_common::constants::WARN_RENDER_SHORTCODES_IN_HTML,
                            format!(
                                ".RenderShortcodes detected inside HTML block in {}; this may not be what you intended, see https://gohugo.io/methods/page/rendershortcodes/#limitations",
                                self.get_page(w)
                            ),
                        );
                    }
                    self.config.writer.secure_write(&mut w.w, &linev);
                }
            } else {
                let l = lines.at(0);
                let v = l.value(source);
                if !self.is_html_comment(&v) {
                    self.log_raw_html_emitted_warn(w);
                    w.w.buf.extend_from_slice(b"<!-- raw HTML omitted -->\n");
                }
            }
        } else if n.has_closure() {
            if self.config.unsafe_ {
                let closure = n.closure_line;
                self.config
                    .writer
                    .secure_write(&mut w.w, &closure.value(source));
            } else {
                let l = lines.at(0);
                let v = l.value(source);
                if !self.is_html_comment(&v) {
                    w.w.buf.extend_from_slice(b"<!-- raw HTML omitted -->\n");
                }
            }
        }
        Ok(WalkStatus::Continue)
    }

    // Go: markup/goldmark/hugocontext/hugocontext.go:renderRawHTML
    fn render_raw_html(
        &self,
        w: &mut RenderCtx<'_>,
        source: &[u8],
        ast: &Ast,
        node: NodeId,
        entering: bool,
    ) -> Result<WalkStatus, goldmark::Error> {
        if !entering {
            return Ok(WalkStatus::SkipChildren);
        }
        let n = ast.raw_html(node).expect("RawHTML node");
        let l = n.segments.len();
        if self.config.unsafe_ {
            for i in 0..l {
                let segment = n.segments.at(i);
                w.w.buf.extend_from_slice(&segment.value(source));
            }
            return Ok(WalkStatus::SkipChildren);
        }
        let segment = n.segments.at(0);
        let v = segment.value(source);
        if !self.is_html_comment(&v) {
            self.log_raw_html_emitted_warn(w);
            w.w.buf.extend_from_slice(b"<!-- raw HTML omitted -->");
        }
        Ok(WalkStatus::SkipChildren)
    }

    // Go: markup/goldmark/hugocontext/hugocontext.go:handleHugoContext
    fn handle_hugo_context(
        &self,
        w: &mut RenderCtx<'_>,
        _source: &[u8],
        ast: &Ast,
        node: NodeId,
        entering: bool,
    ) -> Result<WalkStatus, goldmark::Error> {
        if !entering {
            return Ok(WalkStatus::Continue);
        }

        let hctx = ast.custom::<HugoContext>(node).expect("HugoContext node");
        if hctx.closing {
            let _ = w.pop_pid();
        } else {
            w.push_pid(hctx.pid);
        }
        Ok(WalkStatus::Continue)
    }
}

impl HugoNodeRenderer for HugoContextRenderer {
    // Go: markup/goldmark/hugocontext/hugocontext.go:SetOption
    fn set_option(&mut self, name: &str, value: &parser::OptionValue) {
        self.config.set_option(name, value);
    }

    // Go: markup/goldmark/hugocontext/hugocontext.go:RegisterFuncs
    fn register_funcs(self: Arc<Self>, reg: &mut HugoRegisterer) {
        let r = self.clone();
        reg.register(
            *KIND_HUGO_CONTEXT,
            RenderFunc::hugo(move |w, s, a, n, e| r.handle_hugo_context(w, s, a, n, e)),
        );
        let r = self.clone();
        reg.register(
            ast::KIND_RAW_HTML,
            RenderFunc::hugo(move |w, s, a, n, e| r.render_raw_html(w, s, a, n, e)),
        );
        let r = self;
        reg.register(
            ast::KIND_HTML_BLOCK,
            RenderFunc::hugo(move |w, s, a, n, e| r.render_html_block(w, s, a, n, e)),
        );
    }
}

struct HugoContextTransformer;

impl parser::AstTransformer for HugoContextTransformer {
    // Go: markup/goldmark/hugocontext/hugocontext.go:Transform
    fn transform<'a>(
        &self,
        a: &mut Ast,
        root: NodeId,
        _reader: &mut dyn Reader<'a>,
        _pc: &mut Context,
    ) {
        let _ = ast::walk(a, root, &mut |a, n, entering| {
            let s = WalkStatus::Continue;
            if !entering || a.kind(n) != *KIND_HUGO_CONTEXT {
                return Ok(s);
            }

            if let Some(p) = a.parent(n)
                && a.kind(p) == ast::KIND_PARAGRAPH
            {
                if a.child_count(p) == 1 {
                    // Avoid empty paragraphs.
                    let pp = a.parent(p).expect("paragraph parent");
                    a.replace_child(pp, p, n);
                } else if let Some(t) = a.previous_sibling(n)
                    && a.kind(t) == ast::KIND_TEXT
                {
                    // Remove the newline produced by the Hugo context markers.
                    let text = a.text_node(t).expect("Text node");
                    if text.soft_line_break() {
                        if text.segment.is_empty() {
                            a.remove_child(p, t);
                        } else {
                            a.text_node_mut(t)
                                .expect("Text node")
                                .set_soft_line_break(false);
                        }
                    }
                }
            }

            Ok(s)
        });
    }
}

/// Go: `hugoContextExtension`.
pub struct HugoContextExtension {
    logger: Option<Arc<Logger>>,
}

impl HugoContextExtension {
    /// The node renderer Go adds to `m.Renderer()` (priority 50); the port's converter
    /// dispatches Hugo's renderers itself (see `convert`).
    pub(crate) fn node_renderer(&self) -> HugoContextRenderer {
        HugoContextRenderer {
            logger: self.logger.clone(),
            config: goldmark::renderer::html::Config {
                writer: goldmark::renderer::html::DEFAULT_WRITER.clone(),
                ..goldmark::renderer::html::new_config()
            },
        }
    }
}

impl Extender for HugoContextExtension {
    // Go: markup/goldmark/hugocontext/hugocontext.go:Extend
    fn extend(&self, m: &mut Markdown) {
        m.parser().add_options(vec![
            parser::with_inline_parsers(vec![util::prioritized(
                Box::new(HugoContextParser) as Box<dyn InlineParser>,
                50,
            )]),
            parser::with_ast_transformers(vec![util::prioritized(
                Box::new(HugoContextTransformer) as Box<dyn parser::AstTransformer>,
                10,
            )]),
        ]);
        // The renderer part (priority 50) is registered by the converter: see
        // `HugoContextExtension::node_renderer`.
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Go: markup/goldmark/hugocontext/hugocontext_test.go:TestWrap
    #[test]
    fn wrap_test() {
        assert_eq!(
            wrap(b"test", 42),
            b"{{__hugo_ctx pid=42}}\ntest\n{{__hugo_ctx/}}\n"
        );
    }

    #[test]
    fn hugo_ctx_re() {
        assert_eq!(
            hugo_ctx_re_replace_all(b"a{{__hugo_ctx pid=1}}\nb{{__hugo_ctx/}}c"),
            b"abc"
        );
        assert_eq!(
            hugo_ctx_re_replace_all(b"{{__hugo_ctx pid=}}x"),
            b"{{__hugo_ctx pid=}}x"
        );
        assert_eq!(hugo_ctx_re_replace_all(b"{{__hugo_ctx}}\n\nx"), b"\nx");
        assert_eq!(hugo_ctx_re_replace_all(b"{{__hugo_ctx pid=12/}}x"), b"x");
        assert_eq!(
            hugo_ctx_re_replace_all(b"{{__hugo_ctx pid=1 }}"),
            b"{{__hugo_ctx pid=1 }}"
        );
    }
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: markup/goldmark/hugocontext/hugocontext.go (317 lines; 10/18 funcs executed)
//   types: HugoContext, hugoContextParser, hugoContextRenderer, hugoContextTransformer, hugoContextExtension
// OK L35-37: New(logger loggers.Logger) goldmark.Extender
// OK L41-58: Wrap(b []byte, pid uint64) string
// OK L73-77: (n *HugoContext) Dump(source []byte, level int)
// OK L79-94: (n *HugoContext) parseAttrs(attrBytes []byte)
// OK L96-98: (h *HugoContext) Kind() ast.NodeKind
// OK L111-113: (a *hugoContextParser) Trigger() []byte
// OK L115-135: (s *hugoContextParser) Parse(parent ast.Node, reader text.Reader, pc parser.Context) ast.Node
// OK L142-144: (r *hugoContextRenderer) SetOption(name renderer.OptionName, value any)
// OK L146-150: (r *hugoContextRenderer) RegisterFuncs(reg renderer.NodeRendererFuncRegisterer)
// OK L152-157: (r *hugoContextRenderer) stripHugoCtx(b []byte) ([]byte, bool)
// OK L159-161: (r *hugoContextRenderer) logRawHTMLEmittedWarn(w util.BufWriter)
// OK L163-170: (r *hugoContextRenderer) getPage(w util.BufWriter) any
// OK L172-174: (r *hugoContextRenderer) isHTMLComment(b []byte) bool
// OK L177-218: (r *hugoContextRenderer) renderHTMLBlock( w util.BufWriter, source []byte, node ast.Node, entering bool, ) (ast.WalkStatus, error)
// OK L220-242: (r *hugoContextRenderer) renderRawHTML( w util.BufWriter, source []byte, node ast.Node, entering bool, ) (ast.WalkStatus, error)
// OK L244-260: (r *hugoContextRenderer) handleHugoContext(w util.BufWriter, source []byte, node ast.Node, entering bool) (ast.WalkStatus, error)
// OK L266-293: (a *hugoContextTransformer) Transform(n *ast.Document, reader text.Reader, pc parser.Context)
// OK L299-317: (a *hugoContextExtension) Extend(m goldmark.Markdown)
// ---------------------------------------------------------------------------
