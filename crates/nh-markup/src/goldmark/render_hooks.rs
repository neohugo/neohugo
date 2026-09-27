//! Port of `markup/goldmark/render_hooks.go`.
//!
//! Owner: Wave B task T06 (markup).


//! Go `markup/goldmark/render_hooks.go`: the hooked renderer for Link, AutoLink, Image and Heading
//! nodes and the hook-context objects passed to hook templates. `.Text` is the inner HTML captured
//! from the render buffer (see internal/render); `.Destination`/`.Title` are raw source bytes.

use std::any::Any;
use std::borrow::Cow;
use std::sync::Arc;

use go_value::{GoString, HostCtx, Map, Object, Value};
use nh_common::types::hstring::Html;

use crate::internal::attributes::AttributesHolder;

/// Go: `linkContext` (template type `goldmark.linkContext`): `.Page`, `.PageInner`,
/// `.Destination`, `.Title`, `.Text`, `.PlainText`, `.Attributes` (+ `Position`).
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
    "Page" => |c, _x, _a| Ok(c.page.clone()),
    "PageInner" => |c, _x, _a| Ok(c.page_inner.clone()),
    "Destination" => |c, _x, _a| Ok(Value::String(c.destination.clone())),
    "Title" => |c, _x, _a| Ok(Value::String(c.title.clone())),
    "Text" => |c, _x, _a| Ok(c.text.value()),
    "PlainText" => |c, _x, _a| Ok(Value::String(c.plain_text.clone())),
    "Attributes" => |c, _x, _a| Ok(Value::map(c.attributes.attributes())),
});

impl Object for LinkContext {
    nh_common::object_basics!("goldmark.linkContext");
}

/// Go: `imageLinkContext`: linkContext + `.IsBlock`, `.Ordinal`.
#[derive(Clone)]
pub struct ImageLinkContext {
    pub link: LinkContext,
    pub ordinal: i64,
    pub is_block: bool,
}

nh_common::go_methods!(ImageLinkContext {
    "Page" => |c, _x, _a| Ok(c.link.page.clone()),
    "PageInner" => |c, _x, _a| Ok(c.link.page_inner.clone()),
    "Destination" => |c, _x, _a| Ok(Value::String(c.link.destination.clone())),
    "Title" => |c, _x, _a| Ok(Value::String(c.link.title.clone())),
    "Text" => |c, _x, _a| Ok(c.link.text.value()),
    "PlainText" => |c, _x, _a| Ok(Value::String(c.link.plain_text.clone())),
    "Attributes" => |c, _x, _a| Ok(Value::map(c.link.attributes.attributes())),
    "IsBlock" => |c, _x, _a| Ok(Value::Bool(c.is_block)),
    "Ordinal" => |c, _x, _a| Ok(Value::int(c.ordinal)),
});

impl Object for ImageLinkContext {
    nh_common::object_basics!("goldmark.imageLinkContext");
}

/// Go: `headingContext`: `.Page`, `.PageInner`, `.Level`, `.Anchor`, `.Text`, `.PlainText`, `.Attributes`.
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
    "Page" => |c, _x, _a| Ok(c.page.clone()),
    "PageInner" => |c, _x, _a| Ok(c.page_inner.clone()),
    "Level" => |c, _x, _a| Ok(Value::int(c.level)),
    "Anchor" => |c, _x, _a| Ok(Value::String(c.anchor.clone())),
    "Text" => |c, _x, _a| Ok(c.text.value()),
    "PlainText" => |c, _x, _a| Ok(Value::String(c.plain_text.clone())),
    "Attributes" => |c, _x, _a| Ok(Value::map(c.attributes.attributes())),
});

impl Object for HeadingContext {
    nh_common::object_basics!("goldmark.headingContext");
}

/// Go: `render_hooks.go:autoLinkURL` — linkify `www.` URLs get `LinkifyProtocol`, emails `mailto:`.
pub fn auto_link_url(url: &[u8], protocol: Option<&[u8]>, linkify_protocol: &[u8], is_email: bool) -> Vec<u8> {
    todo!()
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: markup/goldmark/render_hooks.go (552 lines; 19/32 funcs executed)
//   types: linkContext, imageLinkContext, headingContext, hookedRenderer, links
// EX L36-44: newLinkRenderer(cfg goldmark_config.Config) renderer.NodeRenderer
// EX L46-48: newLinks(cfg goldmark_config.Config) goldmark.Extender
// EX L60-62: (ctx linkContext) Destination() string
// EX L64-66: (ctx linkContext) Page() any
// EX L68-70: (ctx linkContext) PageInner() any
// EX L72-74: (ctx linkContext) Text() hstring.HTML
//    L76-78: (ctx linkContext) PlainText() string
// EX L80-82: (ctx linkContext) Title() string
//    L90-92: (ctx imageLinkContext) IsBlock() bool
//    L94-96: (ctx imageLinkContext) Ordinal() int
//    L108-110: (ctx headingContext) Page() any
//    L112-114: (ctx headingContext) PageInner() any
// EX L116-118: (ctx headingContext) Level() int
// EX L120-122: (ctx headingContext) Anchor() string
// EX L124-126: (ctx headingContext) Text() hstring.HTML
//    L128-130: (ctx headingContext) PlainText() string
// EX L137-139: (r *hookedRenderer) SetOption(name renderer.OptionName, value any)
// EX L142-147: (r *hookedRenderer) RegisterFuncs(reg renderer.NodeRendererFuncRegisterer)
// EX L149-212: (r *hookedRenderer) renderImage(w util.BufWriter, source []byte, node ast.Node, entering bool) (ast.WalkStatus, error)
// EX L214-223: (r *hookedRenderer) filterInternalAttributes(attrs []ast.Attribute) []ast.Attribute
//    L227-253: (r *hookedRenderer) renderImageDefault(w util.BufWriter, source []byte, node ast.Node, entering bool) (ast.WalkStatus, error)
// EX L255-297: (r *hookedRenderer) renderLink(w util.BufWriter, source []byte, node ast.Node, entering bool) (ast.WalkStatus, error)
//    L300-310: (r *hookedRenderer) renderTexts(w util.BufWriter, source []byte, n ast.Node)
//    L313-328: (r *hookedRenderer) renderString(w util.BufWriter, source []byte, node ast.Node, entering bool) (ast.WalkStatus, error)
//    L331-370: (r *hookedRenderer) renderText(w util.BufWriter, source []byte, node ast.Node, entering bool) (ast.WalkStatus, error)
//    L374-392: (r *hookedRenderer) renderLinkDefault(w util.BufWriter, source []byte, node ast.Node, entering bool) (ast.WalkStatus, error)
// EX L394-437: (r *hookedRenderer) renderAutoLink(w util.BufWriter, source []byte, node ast.Node, entering bool) (ast.WalkStatus, error)
//    L441-464: (r *hookedRenderer) renderAutoLinkDefault(w util.BufWriter, source []byte, node ast.Node, entering bool) (ast.WalkStatus, error)
// EX L466-475: (r *hookedRenderer) autoLinkURL(n *ast.AutoLink, source []byte) []byte
// EX L477-524: (r *hookedRenderer) renderHeading(w util.BufWriter, source []byte, node ast.Node, entering bool) (ast.WalkStatus, error)
//    L526-541: (r *hookedRenderer) renderHeadingDefault(w util.BufWriter, source []byte, node ast.Node, entering bool) (ast.WalkStatus, error)
// EX L548-552: (e *links) Extend(m goldmark.Markdown)
// ---------------------------------------------------------------------------
