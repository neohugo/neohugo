//! Port of `markup/goldmark/blockquotes/blockquotes.go`.
//!
//! Owner: Wave B task T06 (markup).


//! Go `markup/goldmark/blockquotes`: Hugo's blockquote renderer (differs from goldmark's:
//! `"<blockquote>\n" + strings.TrimSpace(inner) + "</blockquote>\n"` without a hook), alert
//! detection regex `^<p>\[!([a-zA-Z]+)\](-|\+)?[^\S\r\n]?([^\n]*)\n?`.

use go_value::{GoString, Object, Value};
use nh_common::types::hstring::Html;

/// Go: `blockquoteContext`.
#[derive(Clone)]
pub struct BlockquoteContext {
    pub page: Value,
    pub page_inner: Value,
    pub ordinal: i64,
    pub text: Html,
    /// "regular" or "alert".
    pub typ: String,
    pub alert_type: String,
    pub alert_title: Html,
    pub alert_sign: String,
}

nh_common::go_methods!(BlockquoteContext {
    "Page" => |c, _x, _a| Ok(c.page.clone()),
    "PageInner" => |c, _x, _a| Ok(c.page_inner.clone()),
    "Ordinal" => |c, _x, _a| Ok(Value::int(c.ordinal)),
    "Text" => |c, _x, _a| Ok(c.text.value()),
    "Type" => |c, _x, _a| Ok(Value::string(c.typ.as_str())),
    "AlertType" => |c, _x, _a| Ok(Value::string(c.alert_type.as_str())),
    "AlertTitle" => |c, _x, _a| Ok(c.alert_title.value()),
    "AlertSign" => |c, _x, _a| Ok(Value::string(c.alert_sign.as_str())),
});

impl Object for BlockquoteContext {
    nh_common::object_basics!("*blockquotes.blockquoteContext");
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: markup/goldmark/blockquotes/blockquotes.go (196 lines; 7/12 funcs executed)
//   types: (group), blockquoteContext, blockQuoteAlert
// EX L37-39: New() goldmark.Extender
// EX L41-45: (e *blockquotesExtension) Extend(m goldmark.Markdown)
// EX L47-50: newHTMLRenderer() renderer.NodeRenderer
// EX L52-54: (r *htmlRenderer) RegisterFuncs(reg renderer.NodeRendererFuncRegisterer)
// EX L61-125: (r *htmlRenderer) renderBlockquote(w util.BufWriter, src []byte, node ast.Node, entering bool) (ast.WalkStatus, error)
// EX L128-143: (r *htmlRenderer) renderBlockquoteDefault( w util.BufWriter, n ast.Node, text string, ) (ast.WalkStatus, error)
//    L153-155: (c *blockquoteContext) Type() string
//    L157-159: (c *blockquoteContext) AlertType() string
//    L161-163: (c *blockquoteContext) AlertTitle() hstring.HTML
//    L165-167: (c *blockquoteContext) AlertSign() string
//    L169-171: (c *blockquoteContext) Text() hstring.HTML
// EX L175-188: resolveBlockQuoteAlert(s string) blockQuoteAlert
// ---------------------------------------------------------------------------
