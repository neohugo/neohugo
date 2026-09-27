//! Port of `hugolib/shortcode_page.go`.
//!
//! Owner: Wave B task T22 (hugolib-content).


//! Go `ShortcodeWithPage` (template data of a shortcode): `.Params`, `.Page`, `.Inner`,
//! `.InnerDeindent`, `.Get`, `.Name`, `.Ordinal`, `.Parent`, `.Position`, `.IsNamedParams`,
//! `.Site`, `.Ref`, `.RelRef`, `.Scratch`, `.Store`. `.Params` is a FIELD (`[]interface{}` or
//! `map[string]interface{}`).

use std::sync::Arc;

use go_value::{HostCtx, Object, Value};

/// Go: `hugolib.ShortcodeWithPage`.
pub struct ShortcodeWithPage {
    pub params: Value,
    pub inner: Value,
    pub page: Value,
    pub parent: Option<Arc<ShortcodeWithPage>>,
    pub name: String,
    pub is_named_params: bool,
    pub ordinal: i64,
    pub indentation: String,
    pub scratch: Arc<nh_common::maps::scratch::Scratch>,
}

nh_common::go_methods!(ShortcodeWithPage {
    "Get" => |s, _c, a| todo!(),
    "Page" => |s, _c, _a| Ok(s.page.clone()),
    "Inner" => |s, _c, _a| Ok(s.inner.clone()),
    "InnerDeindent" => |s, _c, _a| todo!(),
    "Name" => |s, _c, _a| Ok(Value::string(s.name.as_str())),
    "Ordinal" => |s, _c, _a| Ok(Value::int(s.ordinal)),
    "Parent" => |s, _c, _a| todo!(),
    "IsNamedParams" => |s, _c, _a| Ok(Value::Bool(s.is_named_params)),
    "Site" => |s, _c, _a| todo!(),
    "Ref" => |s, _c, a| todo!(),
    "RelRef" => |s, _c, a| todo!(),
    "Scratch" => |s, _c, _a| Ok(Value::Object(s.scratch.clone())),
    "Store" => |s, _c, _a| Ok(Value::Object(s.scratch.clone())),
});

impl Object for ShortcodeWithPage {
    nh_common::object_basics!("*hugolib.ShortcodeWithPage");

    fn field(&self, name: &str) -> Option<Value> {
        match name {
            "Params" => Some(self.params.clone()),
            _ => None,
        }
    }
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: hugolib/shortcode_page.go (131 lines; 5/11 funcs executed)
//   types: shortcodeRenderer, shortcodeRenderFunc, prerenderedShortcode, pageForShortcode, pageForRenderHooks
// EX L36-38: (f shortcodeRenderFunc) renderShortcode(ctx context.Context) ([]byte, bool, error)
//    L40-43: (f shortcodeRenderFunc) renderShortcodeString(ctx context.Context) (string, bool, error)
// EX L50-52: (p prerenderedShortcode) renderShortcode(context.Context) ([]byte, bool, error)
//    L54-56: (p prerenderedShortcode) renderShortcodeString(context.Context) (string, bool, error)
// EX L80-89: newPageForShortcode(p *pageState) page.Page
// EX L92-94: (p *pageForShortcode) Unwrapv() any
//    L96-98: (p *pageForShortcode) String() string
//    L100-102: (p *pageForShortcode) TableOfContents(context.Context) template.HTML
// EX L115-123: newPageForRenderHook(p *pageState) page.Page
//    L125-127: (p *pageForRenderHooks) Unwrapv() any
//    L129-131: (p *pageForRenderHooks) String() string
// ---------------------------------------------------------------------------
