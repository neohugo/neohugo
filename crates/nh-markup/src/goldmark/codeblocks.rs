//! Port of `markup/goldmark/codeblocks/render.go`.
//!
//! Owner: Wave B task T06 (markup).

//! Go `markup/goldmark/codeblocks`: fenced code blocks are always rendered through a code block
//! renderer: the page's `render-codeblock` hook, or the highlighter (Chroma). The highlighter
//! is an explicit unsupported error in the port (`highlight::highlight`), so only hooks render.

use std::sync::Arc;

use go_value::{GoString, Value};
use goldmark::ast::{self, Ast, NodeId, WalkStatus};
use goldmark::parser::OptionValue;
use nh_common::herrors::Error;

use super::convert::{
    HugoNodeRenderer, HugoRegisterer, RenderFunc, file_error_from_pos, to_goldmark_error,
};
use super::internal::render::{self, Context as RenderCtx, HookBase};
use crate::converter::hooks::{Renderer, RendererType};
use crate::highlight::chromalexers;
use crate::internal::attributes::{self, AttributesHolder, AttributesOwnerType};

/// Go: `codeblocks.New()` — the node renderer (priority 100) is registered by the converter.
pub(crate) fn new_html_renderer() -> HtmlRenderer {
    HtmlRenderer
}

/// Go: `codeblocks.htmlRenderer`.
pub(crate) struct HtmlRenderer;

impl HugoNodeRenderer for HtmlRenderer {
    // Go: markup/goldmark/codeblocks/render.go:RegisterFuncs
    fn register_funcs(self: Arc<Self>, reg: &mut HugoRegisterer) {
        reg.register(
            ast::KIND_FENCED_CODE_BLOCK,
            RenderFunc::hugo(move |w, s, a, n, e| self.render_code_block(w, s, a, n, e)),
        );
    }

    fn set_option(&mut self, _name: &str, _value: &OptionValue) {}
}

impl HtmlRenderer {
    // Go: markup/goldmark/codeblocks/render.go:renderCodeBlock
    fn render_code_block(
        &self,
        ctx: &mut RenderCtx<'_>,
        src: &[u8],
        a: &Ast,
        node: NodeId,
        entering: bool,
    ) -> Result<WalkStatus, goldmark::Error> {
        if entering {
            return Ok(WalkStatus::Continue);
        }

        let lang = get_lang(a, node, src);
        let renderer = ctx.get_renderer(
            RendererType::CodeBlock,
            &Value::String(GoString::new(lang.clone())),
        );
        let Some(renderer) = renderer else {
            return Err(to_goldmark_error(Error::new(format!(
                "no code renderer found for {}",
                go_quote(&lang)
            ))));
        };
        let cr = match renderer {
            Renderer::CodeBlock(c) => c,
            _ => panic!(
                "interface conversion: renderer is not hooks.CodeBlockRenderer: missing method RenderCodeblock"
            ),
        };

        let ordinal = ctx.get_and_increment_ordinal(ast::KIND_FENCED_CODE_BLOCK);

        let mut buff: Vec<u8> = Vec::new();

        let lines = a.lines(node);
        for i in 0..lines.len() {
            let line = lines.at(i);
            buff.extend_from_slice(&line.value(src));
        }

        let s = chomp(&buff).to_vec();

        let info: Option<Vec<u8>> = a.fenced_code_block(node).and_then(|f| f.info).map(|i| {
            a.text_node(i)
                .expect("Text")
                .segment
                .value(src)
                .into_owned()
        });

        let mut attrtp = AttributesOwnerType::CodeBlockCustom;
        if cr.is_default_code_block_renderer() || chromalexers::get(&lang) {
            // We say that this is a Chroma code block if it's the default code block renderer
            // or if the language is supported by Chroma.
            attrtp = AttributesOwnerType::CodeBlockChroma;
        }

        let attrs = match get_attributes(a, node, info.as_deref()) {
            Ok(attrs) => attrs,
            Err((attr_str, err)) => {
                // Go: `&herrors.TextSegmentError{Err: err, Segment: attrStr}`.
                let _ = attr_str;
                return Err(to_goldmark_error(err));
            }
        };

        let resolver = {
            let cr = cr.clone();
            Some(Arc::new(move |sample: &[u8]| cr.resolve_position(sample))
                as render::PositionResolver)
        };

        let cbctx = CodeBlockContext {
            base: Arc::new(render::new_base_context(
                ctx,
                resolver,
                a,
                node,
                src,
                Some(s.clone()),
                ordinal,
            )),
            lang: GoString::new(lang),
            code: GoString::new(s),
            attributes: Arc::new(attributes::new(&attrs, attrtp)),
        };
        let base = cbctx.base.clone();

        let cctx = ctx.render_context().ctx;
        if let Err(err) = cr.render_codeblock(cctx, &mut ctx.w.buf, &Value::object(cbctx)) {
            return Err(file_error_from_pos(err, base.position()));
        }

        Ok(WalkStatus::Continue)
    }
}

/// Go: `codeBlockContext` (template type `*codeblocks.codeBlockContext`): the
/// `hooks.BaseContext` (`.Page`, `.PageInner`, `.Ordinal`, `.Position`), `.Type`, `.Inner` and the
/// `*AttributesHolder` methods (`.Attributes`, `.Options`, ...).
#[derive(Clone)]
pub struct CodeBlockContext {
    pub base: Arc<HookBase>,
    pub lang: GoString,
    pub code: GoString,
    pub attributes: Arc<AttributesHolder>,
}

fn no_args(args: &[Value], name: &str) -> go_value::Result<()> {
    nh_common::object::args::exactly(args, 0, name)
}

nh_common::go_methods!(CodeBlockContext {
    "Page" => |c, _x, a| { no_args(a, "Page")?; Ok(c.base.page()) },
    "PageInner" => |c, _x, a| { no_args(a, "PageInner")?; Ok(c.base.page_inner()) },
    "Ordinal" => |c, _x, a| { no_args(a, "Ordinal")?; Ok(Value::int(c.base.ordinal())) },
    "Position" => |c, _x, a| { no_args(a, "Position")?; Ok(crate::converter::hooks::position_value(c.base.position())) },
    "Type" => |c, _x, a| { no_args(a, "Type")?; Ok(Value::String(c.lang.clone())) },
    "Inner" => |c, _x, a| { no_args(a, "Inner")?; Ok(Value::String(c.code.clone())) },
    "Attributes" => |c, _x, a| { no_args(a, "Attributes")?; Ok(Value::map(c.attributes.attributes())) },
    "Options" => |c, _x, a| { no_args(a, "Options")?; Ok(Value::map(c.attributes.options())) },
    "AttributesSlice" => |c, _x, a| { no_args(a, "AttributesSlice")?; Ok(attributes::attribute_slice_value(c.attributes.attributes_slice())) },
    "OptionsSlice" => |c, _x, a| { no_args(a, "OptionsSlice")?; Ok(attributes::attribute_slice_value(c.attributes.options_slice())) },
});

impl go_value::Object for CodeBlockContext {
    nh_common::object_basics!("*codeblocks.codeBlockContext");
}

/// Go `htext.Chomp`: trailing `\n` and `\r` removed.
fn chomp(s: &[u8]) -> &[u8] {
    let mut end = s.len();
    while end > 0 && (s[end - 1] == b'\n' || s[end - 1] == b'\r') {
        end -= 1;
    }
    &s[..end]
}

/// Go `%q` of a string.
fn go_quote(s: &[u8]) -> String {
    go_strconv::quote(s)
}

// Go: markup/goldmark/codeblocks/render.go:getLang
fn get_lang(a: &Ast, node: NodeId, src: &[u8]) -> Vec<u8> {
    let lang_with_attributes = a.fenced_code_block_language(node, src).unwrap_or_default();
    match lang_with_attributes.iter().position(|&c| c == b'{') {
        Some(i) => lang_with_attributes[..i].to_vec(),
        None => lang_with_attributes,
    }
}

/// On failure: the attribute string and the error (Go returns both).
// Go: markup/goldmark/codeblocks/render.go:getAttributes
fn get_attributes(
    a: &Ast,
    node: NodeId,
    infostr: Option<&[u8]>,
) -> Result<Vec<ast::Attribute>, (Vec<u8>, Error)> {
    if let Some(attrs) = a.attributes(node) {
        return Ok(attrs.to_vec());
    }
    if let Some(infostr) = infostr {
        let mut attr_start_idx: i64 = -1;
        let mut attr_end_idx: i64 = -1;

        for (idx, &char) in infostr.iter().enumerate() {
            if attr_end_idx == -1 && char == b'{' {
                attr_start_idx = idx as i64;
            }
            if attr_start_idx != -1 && char == b'}' {
                attr_end_idx = idx as i64;
                break;
            }
        }

        if attr_start_idx != -1 && attr_end_idx != -1 {
            // dummy node for storing attributes
            let mut dummy = Ast::new();
            let n = dummy.new_text_block();
            let attr_str = &infostr[attr_start_idx as usize..(attr_end_idx + 1) as usize];
            let mut reader = goldmark::text::new_reader(attr_str);
            if let Some(attrs) = goldmark::parser::parse_attributes(&mut reader) {
                for attr in attrs {
                    dummy.set_attribute(n, &attr.name, attr.value.clone());
                }
                return Ok(dummy.attributes(n).map(|a| a.to_vec()).unwrap_or_default());
            } else {
                return Err((
                    attr_str.to_vec(),
                    Error::new(
                        "failed to parse Markdown attributes; you may need to quote the values",
                    ),
                ));
            }
        }
    }
    Ok(Vec::new())
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: markup/goldmark/codeblocks/render.go (179 lines; 4/9 funcs executed)
//   types: (group), codeBlockContext
// OK L41-43: New() goldmark.Extender (the renderer is registered by the converter)
// OK L45-49: (e *codeBlocksExtension) Extend(m goldmark.Markdown)
// OK L51-54: newHTMLRenderer() renderer.NodeRenderer
// OK L56-58: (r *htmlRenderer) RegisterFuncs(reg renderer.NodeRendererFuncRegisterer)
// OK L60-123: (r *htmlRenderer) renderCodeBlock(w util.BufWriter, src []byte, node ast.Node, entering bool) (ast.WalkStatus, error)
// OK L133-135: (c *codeBlockContext) Type() string
// OK L137-139: (c *codeBlockContext) Inner() string
// OK L141-145: getLang(node *ast.FencedCodeBlock, src []byte) string
// OK L147-179: getAttributes(node *ast.FencedCodeBlock, infostr []byte) ([]ast.Attribute, string, error)
// ---------------------------------------------------------------------------
