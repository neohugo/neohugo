//! Port of `markup/goldmark/render_hooks.go`.
//!
//! Owner: Wave B task T06 (markup).

//! Go `markup/goldmark/render_hooks.go`: the hooked renderer for Link, AutoLink, Image and Heading
//! nodes and the hook-context objects passed to hook templates. `.Text` is the inner HTML captured
//! from the render buffer (see internal/render); `.Destination`/`.Title` are raw source bytes.

use std::sync::Arc;

use go_value::{GoString, Value};
use goldmark::ast::{self, Ast, AttrValue, AutoLinkType, NodeId, NodeValue, WalkStatus};
use goldmark::parser::OptionValue;
use goldmark::renderer::html::{self, Config as HtmlConfig};
use goldmark::util::{self, BufWriter};
use nh_common::types::hstring::Html;

use super::convert::{HugoNodeRenderer, HugoRegisterer, RenderFunc, to_goldmark_error};
use super::goldmark_config::Config as GoldmarkConfig;
use super::images;
use super::internal::render::{self, Context as RenderCtx};
use crate::converter::hooks::{LinkRenderer, Renderer, RendererType};
use crate::internal::attributes::{self, AttributesHolder};

/// Don't change this. This pattern is also used in the image render hooks.
const INTERNAL_ATTR_PREFIX: &[u8] = b"_h__";

type R = Result<WalkStatus, goldmark::Error>;

/// Go's `wrong number of args` check for the zero-argument context methods.
fn no_args(args: &[Value], name: &str) -> go_value::Result<()> {
    nh_common::object::args::exactly(args, 0, name)
}

/// Go: `linkContext` (template type `goldmark.linkContext`): `.Page`, `.PageInner`,
/// `.Destination`, `.Title`, `.Text`, `.PlainText` and the embedded `*AttributesHolder`'s
/// `.Attributes`, `.Options`, `.AttributesSlice`, `.OptionsSlice`.
#[derive(Clone)]
pub struct LinkContext {
    pub page: Value,
    pub page_inner: Value,
    pub destination: GoString,
    pub title: GoString,
    pub text: Html,
    pub plain_text: GoString,
    pub attributes: Arc<AttributesHolder>,
}

nh_common::go_methods!(LinkContext {
    "Destination" => |c, _x, a| { no_args(a, "Destination")?; Ok(Value::String(c.destination.clone())) },
    "Page" => |c, _x, a| { no_args(a, "Page")?; Ok(c.page.clone()) },
    "PageInner" => |c, _x, a| { no_args(a, "PageInner")?; Ok(c.page_inner.clone()) },
    "Text" => |c, _x, a| { no_args(a, "Text")?; Ok(c.text.value()) },
    "PlainText" => |c, _x, a| { no_args(a, "PlainText")?; Ok(Value::String(c.plain_text.clone())) },
    "Title" => |c, _x, a| { no_args(a, "Title")?; Ok(Value::String(c.title.clone())) },
    "Attributes" => |c, _x, a| { no_args(a, "Attributes")?; Ok(Value::map(c.attributes.attributes())) },
    "Options" => |c, _x, a| { no_args(a, "Options")?; Ok(Value::map(c.attributes.options())) },
    "AttributesSlice" => |c, _x, a| { no_args(a, "AttributesSlice")?; Ok(attributes::attribute_slice_value(c.attributes.attributes_slice())) },
    "OptionsSlice" => |c, _x, a| { no_args(a, "OptionsSlice")?; Ok(attributes::attribute_slice_value(c.attributes.options_slice())) },
});

impl go_value::Object for LinkContext {
    nh_common::object_basics!("goldmark.linkContext");
    fn kind(&self) -> go_value::Kind {
        go_value::Kind::Struct
    }
}

/// Go: `imageLinkContext`: linkContext + `.IsBlock`, `.Ordinal`.
#[derive(Clone)]
pub struct ImageLinkContext {
    pub link: LinkContext,
    pub ordinal: i64,
    pub is_block: bool,
}

nh_common::go_methods!(ImageLinkContext {
    "Destination" => |c, x, a| c.link.go_call_method(x, "Destination", a).expect("method"),
    "Page" => |c, x, a| c.link.go_call_method(x, "Page", a).expect("method"),
    "PageInner" => |c, x, a| c.link.go_call_method(x, "PageInner", a).expect("method"),
    "Text" => |c, x, a| c.link.go_call_method(x, "Text", a).expect("method"),
    "PlainText" => |c, x, a| c.link.go_call_method(x, "PlainText", a).expect("method"),
    "Title" => |c, x, a| c.link.go_call_method(x, "Title", a).expect("method"),
    "Attributes" => |c, x, a| c.link.go_call_method(x, "Attributes", a).expect("method"),
    "Options" => |c, x, a| c.link.go_call_method(x, "Options", a).expect("method"),
    "AttributesSlice" => |c, x, a| c.link.go_call_method(x, "AttributesSlice", a).expect("method"),
    "OptionsSlice" => |c, x, a| c.link.go_call_method(x, "OptionsSlice", a).expect("method"),
    "IsBlock" => |c, _x, a| { no_args(a, "IsBlock")?; Ok(Value::Bool(c.is_block)) },
    "Ordinal" => |c, _x, a| { no_args(a, "Ordinal")?; Ok(Value::int(c.ordinal)) },
});

impl go_value::Object for ImageLinkContext {
    nh_common::object_basics!("goldmark.imageLinkContext");
    fn kind(&self) -> go_value::Kind {
        go_value::Kind::Struct
    }
}

/// Go: `headingContext`: `.Page`, `.PageInner`, `.Level`, `.Anchor`, `.Text`, `.PlainText` and the
/// embedded `*AttributesHolder`'s methods.
#[derive(Clone)]
pub struct HeadingContext {
    pub page: Value,
    pub page_inner: Value,
    pub level: i64,
    pub anchor: GoString,
    pub text: Html,
    pub plain_text: GoString,
    pub attributes: Arc<AttributesHolder>,
}

nh_common::go_methods!(HeadingContext {
    "Page" => |c, _x, a| { no_args(a, "Page")?; Ok(c.page.clone()) },
    "PageInner" => |c, _x, a| { no_args(a, "PageInner")?; Ok(c.page_inner.clone()) },
    "Level" => |c, _x, a| { no_args(a, "Level")?; Ok(Value::int(c.level)) },
    "Anchor" => |c, _x, a| { no_args(a, "Anchor")?; Ok(Value::String(c.anchor.clone())) },
    "Text" => |c, _x, a| { no_args(a, "Text")?; Ok(c.text.value()) },
    "PlainText" => |c, _x, a| { no_args(a, "PlainText")?; Ok(Value::String(c.plain_text.clone())) },
    "Attributes" => |c, _x, a| { no_args(a, "Attributes")?; Ok(Value::map(c.attributes.attributes())) },
    "Options" => |c, _x, a| { no_args(a, "Options")?; Ok(Value::map(c.attributes.options())) },
    "AttributesSlice" => |c, _x, a| { no_args(a, "AttributesSlice")?; Ok(attributes::attribute_slice_value(c.attributes.attributes_slice())) },
    "OptionsSlice" => |c, _x, a| { no_args(a, "OptionsSlice")?; Ok(attributes::attribute_slice_value(c.attributes.options_slice())) },
});

impl go_value::Object for HeadingContext {
    nh_common::object_basics!("goldmark.headingContext");
    fn kind(&self) -> go_value::Kind {
        go_value::Kind::Struct
    }
}

/// Go: `render_hooks.go:autoLinkURL` — linkify `www.` URLs get `LinkifyProtocol` (the protocol
/// is only set for links whose protocol the parser inserted).
// Go: markup/goldmark/render_hooks.go:autoLinkURL
pub fn auto_link_url(
    url: &[u8],
    protocol: Option<&[u8]>,
    linkify_protocol: &[u8],
    _is_email: bool,
) -> Vec<u8> {
    if let Some(p) = protocol
        && !p.is_empty()
        && p != linkify_protocol
    {
        // The CommonMark spec says "http" is the correct protocol for links,
        // but this doesn't make much sense (the fact that they should care about the rendered
        // output). Note that n.Protocol is not set if protocol is provided by user.
        let mut out = linkify_protocol.to_vec();
        out.extend_from_slice(&url[p.len()..]);
        return out;
    }
    url.to_vec()
}

/// Go: `hookedRenderer`.
pub(crate) struct HookedRenderer {
    linkify_protocol: Vec<u8>,
    config: HtmlConfig,
}

// Go: markup/goldmark/render_hooks.go:newLinkRenderer
pub(crate) fn new_link_renderer(cfg: &GoldmarkConfig) -> HookedRenderer {
    HookedRenderer {
        linkify_protocol: cfg.extensions.linkify_protocol.as_bytes().to_vec(),
        config: HtmlConfig {
            writer: html::DEFAULT_WRITER.clone(),
            ..html::new_config()
        },
    }
}

/// The link renderer returned for a hook type (Go type-asserts `h.(hooks.LinkRenderer)`).
fn link_renderer(r: Option<Renderer>) -> Option<Arc<dyn LinkRenderer>> {
    match r {
        None => None,
        Some(Renderer::Link(l)) => Some(l),
        Some(_) => panic!(
            "interface conversion: renderer is not hooks.LinkRenderer: missing method RenderLink"
        ),
    }
}

impl HookedRenderer {
    // Go: markup/goldmark/render_hooks.go:renderImage
    fn render_image(
        &self,
        ctx: &mut RenderCtx<'_>,
        source: &[u8],
        a: &Ast,
        node: NodeId,
        entering: bool,
    ) -> R {
        let n = a.image(node).expect("not an Image");
        let Some(lr) = link_renderer(ctx.get_renderer(RendererType::Image, &Value::Invalid)) else {
            return self.render_image_default(&mut ctx.w, source, a, node, entering);
        };

        if entering {
            // Store the current pos so we can capture the rendered text.
            ctx.push_pos(ctx.len());
            return Ok(WalkStatus::Continue);
        }

        let text = ctx.pop_rendered_string();

        let mut is_block = false;
        if let Some(b) = a.attribute_string(node, images::ATTR_IS_BLOCK) {
            match b {
                AttrValue::Bool(true) => is_block = true,
                AttrValue::Bool(false) => {}
                other => panic!(
                    "interface conversion: interface {{}} is {}, not bool",
                    other.go_type_name()
                ),
            }
        }
        let ordinal = images::image_ordinal(a, node).unwrap_or(0);

        // We use the attributes to signal from the parser whether the image is in
        // a block context or not.
        // We may find a better way to do that, but for now, we'll need to remove any
        // internal attributes before rendering.
        let attrs = self.filter_internal_attributes(a.attributes(node).unwrap_or(&[]));

        let (page, page_inner) = render::get_page_and_page_inner(ctx);

        let hctx = ImageLinkContext {
            link: LinkContext {
                page,
                page_inner,
                destination: GoString::new(n.destination.clone()),
                title: GoString::new(n.title.clone().unwrap_or_default()),
                text: Html(GoString::new(text)),
                plain_text: GoString::new(render::text_plain(a, node, source)),
                attributes: Arc::new(attributes::new(
                    &attrs,
                    attributes::AttributesOwnerType::General,
                )),
            },
            ordinal,
            is_block,
        };

        let cctx = ctx.render_context().ctx;
        let err = lr.render_link(cctx, &mut ctx.w.buf, &Value::object(hctx));

        err.map_err(to_goldmark_error)?;
        Ok(WalkStatus::Continue)
    }

    /// Go compacts the node's attribute slice in place; the port returns a filtered copy (the
    /// node is rendered once).
    // Go: markup/goldmark/render_hooks.go:filterInternalAttributes
    fn filter_internal_attributes(&self, attrs: &[ast::Attribute]) -> Vec<ast::Attribute> {
        attrs
            .iter()
            .filter(|x| !x.name.starts_with(INTERNAL_ATTR_PREFIX))
            .cloned()
            .collect()
    }

    /// Fall back to the default Goldmark render funcs.
    // Go: markup/goldmark/render_hooks.go:renderImageDefault
    fn render_image_default(
        &self,
        w: &mut dyn BufWriter,
        source: &[u8],
        a: &Ast,
        node: NodeId,
        entering: bool,
    ) -> R {
        if !entering {
            return Ok(WalkStatus::Continue);
        }
        let n = a.image(node).expect("not an Image");
        w.write_string("<img src=\"");
        if self.config.unsafe_ || !html::is_dangerous_url(&n.destination) {
            w.write(&util::escape_html(&util::url_escape(&n.destination, true)));
        }
        w.write_string("\" alt=\"");
        self.render_texts(w, source, a, node);
        w.write_byte(b'"');
        if let Some(title) = &n.title {
            w.write_string(" title=\"");
            self.config.writer.write(w, title);
            w.write_byte(b'"');
        }
        if a.attributes(node).is_some() {
            html::render_attributes(w, a, node, Some(&html::IMAGE_ATTRIBUTE_FILTER));
        }
        if self.config.xhtml {
            w.write_string(" />");
        } else {
            w.write_string(">");
        }
        Ok(WalkStatus::SkipChildren)
    }

    // Go: markup/goldmark/render_hooks.go:renderLink
    fn render_link(
        &self,
        ctx: &mut RenderCtx<'_>,
        source: &[u8],
        a: &Ast,
        node: NodeId,
        entering: bool,
    ) -> R {
        let n = a.link(node).expect("not a Link");
        let Some(lr) = link_renderer(ctx.get_renderer(RendererType::Link, &Value::Invalid)) else {
            return self.render_link_default(&mut ctx.w, source, a, node, entering);
        };

        if entering {
            // Store the current pos so we can capture the rendered text.
            ctx.push_pos(ctx.len());
            return Ok(WalkStatus::Continue);
        }

        let text = ctx.pop_rendered_string();

        let (page, page_inner) = render::get_page_and_page_inner(ctx);

        let hctx = LinkContext {
            page,
            page_inner,
            destination: GoString::new(n.destination.clone()),
            title: GoString::new(n.title.clone().unwrap_or_default()),
            text: Html(GoString::new(text)),
            plain_text: GoString::new(render::text_plain(a, node, source)),
            attributes: attributes::empty(),
        };

        let cctx = ctx.render_context().ctx;
        lr.render_link(cctx, &mut ctx.w.buf, &Value::object(hctx))
            .map_err(to_goldmark_error)?;
        Ok(WalkStatus::Continue)
    }

    /// Borrowed from Goldmark's HTML renderer (an explicit stack instead of Go's recursion).
    // Go: markup/goldmark/render_hooks.go:renderTexts
    fn render_texts(&self, w: &mut dyn BufWriter, source: &[u8], a: &Ast, n: NodeId) {
        let mut stack: Vec<Option<NodeId>> = vec![a.first_child(n)];
        while let Some(top) = stack.last_mut() {
            let Some(c) = *top else {
                stack.pop();
                continue;
            };
            *top = a.next_sibling(c);
            match a.value(c) {
                NodeValue::String(_) => {
                    let _ = self.render_string(w, source, a, c, true);
                }
                NodeValue::Text(_) => {
                    let _ = self.render_text(w, source, a, c, true);
                }
                _ => stack.push(a.first_child(c)),
            }
        }
    }

    /// Borrowed from Goldmark's HTML renderer.
    // Go: markup/goldmark/render_hooks.go:renderString
    fn render_string(
        &self,
        w: &mut dyn BufWriter,
        _source: &[u8],
        a: &Ast,
        node: NodeId,
        entering: bool,
    ) -> R {
        if !entering {
            return Ok(WalkStatus::Continue);
        }
        let n = a.string_node(node).expect("not a String");
        if n.is_code() {
            w.write(&n.value);
        } else if n.is_raw() {
            self.config.writer.raw_write(w, &n.value);
        } else {
            self.config.writer.write(w, &n.value);
        }
        Ok(WalkStatus::Continue)
    }

    /// Borrowed from Goldmark's HTML renderer (without the east-asian line break handling,
    /// which Hugo comments out).
    // Go: markup/goldmark/render_hooks.go:renderText
    fn render_text(
        &self,
        w: &mut dyn BufWriter,
        source: &[u8],
        a: &Ast,
        node: NodeId,
        entering: bool,
    ) -> R {
        if !entering {
            return Ok(WalkStatus::Continue);
        }
        let n = a.text_node(node).expect("not a Text");
        let segment = n.segment;
        if n.is_raw() {
            self.config.writer.raw_write(w, &segment.value(source));
        } else {
            let value = segment.value(source);
            self.config.writer.write(w, &value);
            if n.hard_line_break() || (n.soft_line_break() && self.config.hard_wraps) {
                if self.config.xhtml {
                    w.write_string("<br />\n");
                } else {
                    w.write_string("<br>\n");
                }
            } else if n.soft_line_break() {
                w.write_byte(b'\n');
            }
        }
        Ok(WalkStatus::Continue)
    }

    /// Fall back to the default Goldmark render funcs.
    // Go: markup/goldmark/render_hooks.go:renderLinkDefault
    fn render_link_default(
        &self,
        w: &mut dyn BufWriter,
        _source: &[u8],
        a: &Ast,
        node: NodeId,
        entering: bool,
    ) -> R {
        let n = a.link(node).expect("not a Link");
        if entering {
            w.write_string("<a href=\"");
            if self.config.unsafe_ || !html::is_dangerous_url(&n.destination) {
                w.write(&util::escape_html(&util::url_escape(&n.destination, true)));
            }
            w.write_byte(b'"');
            if let Some(title) = &n.title {
                w.write_string(" title=\"");
                self.config.writer.write(w, title);
                w.write_byte(b'"');
            }
            w.write_byte(b'>');
        } else {
            w.write_string("</a>");
        }
        Ok(WalkStatus::Continue)
    }

    // Go: markup/goldmark/render_hooks.go:renderAutoLink
    fn render_auto_link(
        &self,
        ctx: &mut RenderCtx<'_>,
        source: &[u8],
        a: &Ast,
        node: NodeId,
        entering: bool,
    ) -> R {
        if !entering {
            return Ok(WalkStatus::Continue);
        }

        let n = a.auto_link(node).expect("not an AutoLink");
        let Some(lr) = link_renderer(ctx.get_renderer(RendererType::Link, &Value::Invalid)) else {
            return self.render_auto_link_default(&mut ctx.w, source, a, node, entering);
        };

        let mut url = self.auto_link_url(a, n, source);
        let label = a.auto_link_label(n, source);
        if n.auto_link_type == AutoLinkType::Email
            && !go_unicode::strings::to_lower(&url).starts_with(b"mailto:")
        {
            let mut u = b"mailto:".to_vec();
            u.extend_from_slice(&url);
            url = u;
        }

        let (page, page_inner) = render::get_page_and_page_inner(ctx);

        let hctx = LinkContext {
            page,
            page_inner,
            destination: GoString::new(url),
            title: GoString::empty(),
            text: Html(GoString::new(label.clone())),
            plain_text: GoString::new(label),
            attributes: attributes::empty(),
        };

        let cctx = ctx.render_context().ctx;
        lr.render_link(cctx, &mut ctx.w.buf, &Value::object(hctx))
            .map_err(to_goldmark_error)?;
        Ok(WalkStatus::Continue)
    }

    /// Fall back to the default Goldmark render funcs.
    // Go: markup/goldmark/render_hooks.go:renderAutoLinkDefault
    fn render_auto_link_default(
        &self,
        w: &mut dyn BufWriter,
        source: &[u8],
        a: &Ast,
        node: NodeId,
        entering: bool,
    ) -> R {
        let n = a.auto_link(node).expect("not an AutoLink");
        if !entering {
            return Ok(WalkStatus::Continue);
        }

        w.write_string("<a href=\"");
        let url = self.auto_link_url(a, n, source);
        let label = a.auto_link_label(n, source);
        if n.auto_link_type == AutoLinkType::Email
            && !go_unicode::bytes::to_lower(&url).starts_with(b"mailto:")
        {
            w.write_string("mailto:");
        }
        w.write(&util::escape_html(&util::url_escape(&url, false)));
        if a.attributes(node).is_some() {
            w.write_byte(b'"');
            html::render_attributes(w, a, node, Some(&html::LINK_ATTRIBUTE_FILTER));
            w.write_byte(b'>');
        } else {
            w.write_string("\">");
        }
        w.write(&util::escape_html(&label));
        w.write_string("</a>");
        Ok(WalkStatus::Continue)
    }

    // Go: markup/goldmark/render_hooks.go:autoLinkURL
    fn auto_link_url(&self, a: &Ast, n: &ast::AutoLink, source: &[u8]) -> Vec<u8> {
        let url = a.auto_link_url(n, source);
        auto_link_url(&url, n.protocol.as_deref(), &self.linkify_protocol, false)
    }

    // Go: markup/goldmark/render_hooks.go:renderHeading
    fn render_heading(
        &self,
        ctx: &mut RenderCtx<'_>,
        source: &[u8],
        a: &Ast,
        node: NodeId,
        entering: bool,
    ) -> R {
        let n = a.heading(node).expect("not a Heading");
        let hr = match ctx.get_renderer(RendererType::Heading, &Value::Invalid) {
            None => return self.render_heading_default(&mut ctx.w, source, a, node, entering),
            Some(Renderer::Heading(h)) => h,
            Some(_) => panic!(
                "interface conversion: renderer is not hooks.HeadingRenderer: missing method RenderHeading"
            ),
        };

        if entering {
            // Store the current pos so we can capture the rendered text.
            ctx.push_pos(ctx.len());
            return Ok(WalkStatus::Continue);
        }

        let text = ctx.pop_rendered_string();

        let mut anchor: Vec<u8> = Vec::new();
        if let Some(AttrValue::Bytes(b)) = a.attribute_string(node, "id") {
            anchor = b.clone();
        }

        let (page, page_inner) = render::get_page_and_page_inner(ctx);

        let hctx = HeadingContext {
            page,
            page_inner,
            level: n.level,
            anchor: GoString::new(anchor),
            text: Html(GoString::new(text)),
            plain_text: GoString::new(render::text_plain(a, node, source)),
            attributes: Arc::new(attributes::new(
                a.attributes(node).unwrap_or(&[]),
                attributes::AttributesOwnerType::General,
            )),
        };

        let cctx = ctx.render_context().ctx;
        hr.render_heading(cctx, &mut ctx.w.buf, &Value::object(hctx))
            .map_err(to_goldmark_error)?;
        Ok(WalkStatus::Continue)
    }

    // Go: markup/goldmark/render_hooks.go:renderHeadingDefault
    fn render_heading_default(
        &self,
        w: &mut dyn BufWriter,
        _source: &[u8],
        a: &Ast,
        node: NodeId,
        entering: bool,
    ) -> R {
        let n = a.heading(node).expect("not a Heading");
        if entering {
            w.write_string("<h");
            w.write_byte(b"0123456"[n.level as usize]);
            if let Some(attrs) = a.attributes(node) {
                attributes::render_ast_attributes(w, attrs);
            }
            w.write_byte(b'>');
        } else {
            w.write_string("</h");
            w.write_byte(b"0123456"[n.level as usize]);
            w.write_string(">\n");
        }
        Ok(WalkStatus::Continue)
    }
}

impl HugoNodeRenderer for HookedRenderer {
    // Go: markup/goldmark/render_hooks.go:SetOption
    fn set_option(&mut self, name: &str, value: &OptionValue) {
        self.config.set_option(name, value);
    }

    /// RegisterFuncs implements NodeRenderer.RegisterFuncs.
    // Go: markup/goldmark/render_hooks.go:RegisterFuncs
    fn register_funcs(self: Arc<Self>, reg: &mut HugoRegisterer) {
        let r = self.clone();
        reg.register(
            ast::KIND_LINK,
            RenderFunc::hugo(move |w, s, a, n, e| r.render_link(w, s, a, n, e)),
        );
        let r = self.clone();
        reg.register(
            ast::KIND_AUTO_LINK,
            RenderFunc::hugo(move |w, s, a, n, e| r.render_auto_link(w, s, a, n, e)),
        );
        let r = self.clone();
        reg.register(
            ast::KIND_IMAGE,
            RenderFunc::hugo(move |w, s, a, n, e| r.render_image(w, s, a, n, e)),
        );
        let r = self;
        reg.register(
            ast::KIND_HEADING,
            RenderFunc::hugo(move |w, s, a, n, e| r.render_heading(w, s, a, n, e)),
        );
    }
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: markup/goldmark/render_hooks.go (552 lines; 19/32 funcs executed)
//   types: linkContext, imageLinkContext, headingContext, hookedRenderer, links
// OK L36-44: newLinkRenderer(cfg goldmark_config.Config) renderer.NodeRenderer
// OK L46-48: newLinks(cfg goldmark_config.Config) goldmark.Extender
// OK L60-62: (ctx linkContext) Destination() string
// OK L64-66: (ctx linkContext) Page() any
// OK L68-70: (ctx linkContext) PageInner() any
// OK L72-74: (ctx linkContext) Text() hstring.HTML
// OK L76-78: (ctx linkContext) PlainText() string
// OK L80-82: (ctx linkContext) Title() string
// OK L90-92: (ctx imageLinkContext) IsBlock() bool
// OK L94-96: (ctx imageLinkContext) Ordinal() int
// OK L108-110: (ctx headingContext) Page() any
// OK L112-114: (ctx headingContext) PageInner() any
// OK L116-118: (ctx headingContext) Level() int
// OK L120-122: (ctx headingContext) Anchor() string
// OK L124-126: (ctx headingContext) Text() hstring.HTML
// OK L128-130: (ctx headingContext) PlainText() string
// OK L137-139: (r *hookedRenderer) SetOption(name renderer.OptionName, value any)
// OK L142-147: (r *hookedRenderer) RegisterFuncs(reg renderer.NodeRendererFuncRegisterer)
// OK L149-212: (r *hookedRenderer) renderImage(w util.BufWriter, source []byte, node ast.Node, entering bool) (ast.WalkStatus, error)
// OK L214-223: (r *hookedRenderer) filterInternalAttributes(attrs []ast.Attribute) []ast.Attribute
// OK L227-253: (r *hookedRenderer) renderImageDefault(w util.BufWriter, source []byte, node ast.Node, entering bool) (ast.WalkStatus, error)
// OK L255-297: (r *hookedRenderer) renderLink(w util.BufWriter, source []byte, node ast.Node, entering bool) (ast.WalkStatus, error)
// OK L300-310: (r *hookedRenderer) renderTexts(w util.BufWriter, source []byte, n ast.Node)
// OK L313-328: (r *hookedRenderer) renderString(w util.BufWriter, source []byte, node ast.Node, entering bool) (ast.WalkStatus, error)
// OK L331-370: (r *hookedRenderer) renderText(w util.BufWriter, source []byte, node ast.Node, entering bool) (ast.WalkStatus, error)
// OK L374-392: (r *hookedRenderer) renderLinkDefault(w util.BufWriter, source []byte, node ast.Node, entering bool) (ast.WalkStatus, error)
// OK L394-437: (r *hookedRenderer) renderAutoLink(w util.BufWriter, source []byte, node ast.Node, entering bool) (ast.WalkStatus, error)
// OK L441-464: (r *hookedRenderer) renderAutoLinkDefault(w util.BufWriter, source []byte, node ast.Node, entering bool) (ast.WalkStatus, error)
// OK L466-475: (r *hookedRenderer) autoLinkURL(n *ast.AutoLink, source []byte) []byte
// OK L477-524: (r *hookedRenderer) renderHeading(w util.BufWriter, source []byte, node ast.Node, entering bool) (ast.WalkStatus, error)
// OK L526-541: (r *hookedRenderer) renderHeadingDefault(w util.BufWriter, source []byte, node ast.Node, entering bool) (ast.WalkStatus, error)
// OK L548-552: (e *links) Extend(m goldmark.Markdown)
// ---------------------------------------------------------------------------
