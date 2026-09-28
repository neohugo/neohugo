//! Port of `markup/goldmark/blockquotes/blockquotes.go`.
//!
//! Owner: Wave B task T06 (markup).

//! Go `markup/goldmark/blockquotes`: Hugo's blockquote renderer (differs from goldmark's:
//! `"<blockquote>\n" + strings.TrimSpace(inner) + "</blockquote>\n"` without a hook), alert
//! detection regex `^<p>\[!([a-zA-Z]+)\](-|\+)?[^\S\r\n]?([^\n]*)\n?`.

use std::sync::Arc;

use go_value::{GoString, Value};
use goldmark::ast::{self, Ast, NodeId, WalkStatus};
use goldmark::parser::OptionValue;
use goldmark::renderer::html;
use goldmark::util::BufWriter;
use nh_common::types::hstring::Html;

use super::convert::{HugoNodeRenderer, HugoRegisterer, RenderFunc, file_error_from_pos};
use super::internal::render::{self, Context as RenderCtx, HookBase};
use crate::converter::hooks::{Renderer, RendererType};
use crate::internal::attributes::{self, AttributesHolder};

const TYPE_REGULAR: &str = "regular";
const TYPE_ALERT: &str = "alert";

/// Go: `blockquotes.New()` — the node renderer (priority 100) is registered by the converter.
pub(crate) fn new_html_renderer() -> HtmlRenderer {
    HtmlRenderer
}

/// Go: `blockquotes.htmlRenderer`.
pub(crate) struct HtmlRenderer;

impl HugoNodeRenderer for HtmlRenderer {
    // Go: markup/goldmark/blockquotes/blockquotes.go:RegisterFuncs
    fn register_funcs(self: Arc<Self>, reg: &mut HugoRegisterer) {
        reg.register(
            ast::KIND_BLOCKQUOTE,
            RenderFunc::hugo(move |w, s, a, n, e| self.render_blockquote(w, s, a, n, e)),
        );
    }

    fn set_option(&mut self, _name: &str, _value: &OptionValue) {}
}

impl HtmlRenderer {
    // Go: markup/goldmark/blockquotes/blockquotes.go:renderBlockquote
    fn render_blockquote(
        &self,
        ctx: &mut RenderCtx<'_>,
        src: &[u8],
        a: &Ast,
        n: NodeId,
        entering: bool,
    ) -> Result<WalkStatus, goldmark::Error> {
        if entering {
            // Store the current pos so we can capture the rendered text.
            ctx.push_pos(ctx.len());
            return Ok(WalkStatus::Continue);
        }

        let text = go_unicode::strings::trim_space(&ctx.pop_rendered_string()).to_vec();

        let ordinal = ctx.get_and_increment_ordinal(ast::KIND_BLOCKQUOTE);

        let mut typ = TYPE_REGULAR;
        let alert = resolve_block_quote_alert(&text);
        if !alert.typ.is_empty() {
            typ = TYPE_ALERT;
        }

        let renderer = ctx.get_renderer(RendererType::Blockquote, &Value::string(typ));
        let Some(renderer) = renderer else {
            return self.render_blockquote_default(&mut ctx.w, a, n, &text);
        };

        let mut text = text;
        if typ == TYPE_ALERT {
            // Parse the blockquote content to determine the alert text. The alert
            // text begins after the first newline, but we need to add an opening p
            // tag if the first line of the blockquote content does not have a
            // closing p tag. At some point we might want to move this to the
            // parser.
            match text.iter().position(|&c| c == b'\n') {
                Some(i) => {
                    let (before, after) = (&text[..i], &text[i + 1..]);
                    if before.ends_with(b"</p>") {
                        text = after.to_vec();
                    } else {
                        let mut t = b"<p>".to_vec();
                        t.extend_from_slice(after);
                        text = t;
                    }
                }
                None => text = Vec::new(),
            }
        }

        let cr = match renderer {
            Renderer::Blockquote(b) => b,
            _ => panic!(
                "interface conversion: renderer is not hooks.BlockquoteRenderer: missing method RenderBlockquote"
            ),
        };
        let resolver = {
            let cr = cr.clone();
            Some(Arc::new(move |sample: &[u8]| cr.resolve_position(sample))
                as render::PositionResolver)
        };

        let bqctx = BlockquoteContext {
            base: Arc::new(render::new_base_context(
                ctx, resolver, a, n, src, None, ordinal,
            )),
            typ: typ.to_string(),
            alert,
            text: Html(GoString::new(text)),
            attributes: Arc::new(attributes::new(
                a.attributes(n).unwrap_or(&[]),
                attributes::AttributesOwnerType::General,
            )),
        };
        let base = bqctx.base.clone();

        let cctx = ctx.render_context().ctx;
        if let Err(err) = cr.render_blockquote(cctx, &mut ctx.w.buf, &Value::object(bqctx)) {
            return Err(file_error_from_pos(err, base.position()));
        }

        Ok(WalkStatus::Continue)
    }

    /// Code borrowed from goldmark's html renderer.
    // Go: markup/goldmark/blockquotes/blockquotes.go:renderBlockquoteDefault
    fn render_blockquote_default(
        &self,
        w: &mut dyn BufWriter,
        a: &Ast,
        n: NodeId,
        text: &[u8],
    ) -> Result<WalkStatus, goldmark::Error> {
        if a.attributes(n).is_some() {
            w.write_string("<blockquote");
            html::render_attributes(w, a, n, Some(&html::BLOCKQUOTE_ATTRIBUTE_FILTER));
            w.write_byte(b'>');
        } else {
            w.write_string("<blockquote>\n");
        }

        w.write(text);

        w.write_string("</blockquote>\n");
        Ok(WalkStatus::Continue)
    }
}

/// Go: `blockquoteContext` (template type `*blockquotes.blockquoteContext`): the
/// `hooks.BaseContext` (`.Page`, `.PageInner`, `.Ordinal`, `.Position`), `.Text`, `.Type`,
/// `.AlertType`, `.AlertTitle`, `.AlertSign` and the `*AttributesHolder` methods.
#[derive(Clone)]
pub struct BlockquoteContext {
    pub base: Arc<HookBase>,
    pub typ: String,
    pub alert: BlockQuoteAlert,
    pub text: Html,
    pub attributes: Arc<AttributesHolder>,
}

fn no_args(args: &[Value], name: &str) -> go_value::Result<()> {
    nh_common::object::args::exactly(args, 0, name)
}

nh_common::go_methods!(BlockquoteContext {
    "Page" => |c, _x, a| { no_args(a, "Page")?; Ok(c.base.page()) },
    "PageInner" => |c, _x, a| { no_args(a, "PageInner")?; Ok(c.base.page_inner()) },
    "Ordinal" => |c, _x, a| { no_args(a, "Ordinal")?; Ok(Value::int(c.base.ordinal())) },
    "Position" => |c, _x, a| { no_args(a, "Position")?; Ok(crate::converter::hooks::position_value(c.base.position())) },
    "Text" => |c, _x, a| { no_args(a, "Text")?; Ok(c.text.value()) },
    "Type" => |c, _x, a| { no_args(a, "Type")?; Ok(Value::string(c.typ.as_str())) },
    "AlertType" => |c, _x, a| { no_args(a, "AlertType")?; Ok(Value::String(c.alert.typ.clone())) },
    "AlertTitle" => |c, _x, a| { no_args(a, "AlertTitle")?; Ok(Html(c.alert.title.clone()).value()) },
    "AlertSign" => |c, _x, a| { no_args(a, "AlertSign")?; Ok(Value::String(c.alert.sign.clone())) },
    "Attributes" => |c, _x, a| { no_args(a, "Attributes")?; Ok(Value::map(c.attributes.attributes())) },
    "Options" => |c, _x, a| { no_args(a, "Options")?; Ok(Value::map(c.attributes.options())) },
    "AttributesSlice" => |c, _x, a| { no_args(a, "AttributesSlice")?; Ok(attributes::attribute_slice_value(c.attributes.attributes_slice())) },
    "OptionsSlice" => |c, _x, a| { no_args(a, "OptionsSlice")?; Ok(attributes::attribute_slice_value(c.attributes.options_slice())) },
});

impl go_value::Object for BlockquoteContext {
    nh_common::object_basics!("*blockquotes.blockquoteContext");
}

/// Go: `blockQuoteAlert`. Blockquote alert syntax was introduced by GitHub, but is also used
/// by Obsidian which also support some extended attributes: More types, alert titles and a +/-
/// sign for folding.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct BlockQuoteAlert {
    pub typ: GoString,
    pub sign: GoString,
    pub title: GoString,
}

// Go: markup/goldmark/blockquotes/blockquotes.go:resolveBlockQuoteAlert
fn resolve_block_quote_alert(s: &[u8]) -> BlockQuoteAlert {
    if let Some(m) = block_quote_alert_re(s) {
        let title = go_unicode::strings::trim_space(m.2);
        let title = title.strip_suffix(b"</p>").unwrap_or(title);
        return BlockQuoteAlert {
            typ: GoString::new(go_unicode::strings::to_lower(m.0).into_owned()),
            sign: GoString::new(m.1.to_vec()),
            title: GoString::new(title.to_vec()),
        };
    }

    BlockQuoteAlert::default()
}

/// `blockQuoteAlertRe.FindStringSubmatch(s)` with
/// `^<p>\[!([a-zA-Z]+)\](-|\+)?[^\S\r\n]?([^\n]*)\n?` (a hand-written matcher; Go's `\s` is
/// `[\t\n\f\r ]`, so `[^\S\r\n]` is `[\t\f ]`). Returns the three groups.
fn block_quote_alert_re(s: &[u8]) -> Option<(&[u8], &[u8], &[u8])> {
    let rest = s.strip_prefix(b"<p>[!")?;
    let n = rest.iter().take_while(|c| c.is_ascii_alphabetic()).count();
    if n == 0 {
        return None;
    }
    let typ = &rest[..n];
    let mut rest = rest[n..].strip_prefix(b"]")?;
    let mut sign: &[u8] = b"";
    if let Some(&c) = rest.first()
        && (c == b'-' || c == b'+')
    {
        sign = &rest[..1];
        rest = &rest[1..];
    }
    if let Some(&c) = rest.first()
        && (c == b'\t' || c == 0x0c || c == b' ')
    {
        rest = &rest[1..];
    }
    let end = rest.iter().position(|&c| c == b'\n').unwrap_or(rest.len());
    Some((typ, sign, &rest[..end]))
}

#[cfg(test)]
mod tests {
    use super::*;

    // Go: markup/goldmark/blockquotes/blockquotes_test.go:TestResolveBlockQuoteAlert
    #[test]
    fn resolve_alert() {
        let alert = |typ: &str, sign: &str, title: &str| BlockQuoteAlert {
            typ: GoString::new(typ.as_bytes().to_vec()),
            sign: GoString::new(sign.as_bytes().to_vec()),
            title: GoString::new(title.as_bytes().to_vec()),
        };
        let tests = [
            ("[!NOTE]", alert("note", "", "")),
            ("[!FaQ]", alert("faq", "", "")),
            ("[!NOTE]+", alert("note", "+", "")),
            ("[!NOTE]-", alert("note", "-", "")),
            (
                "[!NOTE] This is a note",
                alert("note", "", "This is a note"),
            ),
            (
                "[!NOTE]+ This is a note",
                alert("note", "+", "This is a note"),
            ),
            (
                "[!NOTE]+ This is a title\nThis is not.",
                alert("note", "+", "This is a title"),
            ),
            ("[!NOTE]\nThis is not.", alert("note", "", "")),
        ];
        for (i, (input, expected)) in tests.iter().enumerate() {
            let s = format!("<p>{input}</p>");
            assert_eq!(
                resolve_block_quote_alert(s.as_bytes()),
                *expected,
                "Test {i}"
            );
        }
        assert_eq!(
            resolve_block_quote_alert(b"<p>[!]</p>"),
            BlockQuoteAlert::default()
        );
        assert_eq!(
            resolve_block_quote_alert(b"<p> [!NOTE]</p>"),
            BlockQuoteAlert::default()
        );
        // `[^\S\r\n]` is `[\t\f ]`: an NBSP is not consumed there, but the title is
        // `strings.TrimSpace`d.
        assert_eq!(
            resolve_block_quote_alert("<p>[!NOTE]\u{a0}x</p>".as_bytes()),
            alert("note", "", "x")
        );
    }
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: markup/goldmark/blockquotes/blockquotes.go (196 lines; 7/12 funcs executed)
//   types: (group), blockquoteContext, blockQuoteAlert
// OK L37-39: New() goldmark.Extender (the renderer is registered by the converter)
// OK L41-45: (e *blockquotesExtension) Extend(m goldmark.Markdown)
// OK L47-50: newHTMLRenderer() renderer.NodeRenderer
// OK L52-54: (r *htmlRenderer) RegisterFuncs(reg renderer.NodeRendererFuncRegisterer)
// OK L61-125: (r *htmlRenderer) renderBlockquote(w util.BufWriter, src []byte, node ast.Node, entering bool) (ast.WalkStatus, error)
// OK L128-143: (r *htmlRenderer) renderBlockquoteDefault( w util.BufWriter, n ast.Node, text string, ) (ast.WalkStatus, error)
// OK L153-155: (c *blockquoteContext) Type() string
// OK L157-159: (c *blockquoteContext) AlertType() string
// OK L161-163: (c *blockquoteContext) AlertTitle() hstring.HTML
// OK L165-167: (c *blockquoteContext) AlertSign() string
// OK L169-171: (c *blockquoteContext) Text() hstring.HTML
// OK L175-188: resolveBlockQuoteAlert(s string) blockQuoteAlert
// ---------------------------------------------------------------------------
