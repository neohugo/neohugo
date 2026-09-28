//! Port of `hugolib/shortcode_page.go` (+ the `ShortcodeWithPage` type of `hugolib/shortcode.go`).
//!
//! Owner: Wave B task T22 (hugolib-content).

//! Go `ShortcodeWithPage` (template data of a shortcode): `.Params`, `.Page`, `.Inner`,
//! `.InnerDeindent`, `.Get`, `.Name`, `.Ordinal`, `.Parent`, `.Position`, `.IsNamedParams`,
//! `.Site`, `.Ref`, `.RelRef`, `.Scratch`, `.Store`. `Params`, `Inner`, `Page`, `Parent`, `Name`,
//! `IsNamedParams` and `Ordinal` are exported struct FIELDS in Go (`field`), the rest methods.
//!
//! Also the shortcode renderers (Go `shortcodeRenderer`, `shortcodeRenderFunc`,
//! `prerenderedShortcode`) and the page wrappers handed to shortcodes and render hooks
//! (`newPageForShortcode`, `newPageForRenderHook`: `PageHandle` with a `PageWrapper`; their
//! method sets are T23's `tplapi/page_methods.rs`).

use std::sync::{Arc, OnceLock};

use go_value::{GoString, HostCtx, IntKind, MapType, Object, Value};
use nh_common::Result;
use nh_common::object::GoResult;
use nh_common::text::Position;
use nh_tpl::template::TplContext;

use crate::hugo_sites::HugoSites;
use crate::page::{PageHandle, PageId, PageWrapper};
use crate::page__output::{HugoSitesRef, upgrade_hs};

/// Go: `hugolib.ShortcodeWithPage`.
pub struct ShortcodeWithPage {
    /// Go `Params any`: `Invalid` (nil), a nil `[]string`, `[]interface {}` or
    /// `map[string]interface {}`.
    pub params: Value,
    /// Go `Inner template.HTML`: set after the nested shortcodes rendered (a nested shortcode
    /// sees its parent's zero `Inner` through `.Parent`, as in Go).
    pub inner: OnceLock<Value>,
    /// Go `Page page.Page`: the `pageForShortcode` wrapper.
    pub page: Value,
    pub parent: Option<Arc<ShortcodeWithPage>>,
    pub name: String,
    pub is_named_params: bool,
    /// Zero-based ordinal in relation to its parent. If the parent is the page itself, this
    /// ordinal will represent the position of this shortcode in the page content.
    pub ordinal: i64,
    /// Indentation before the opening shortcode in the source.
    pub indentation: String,
    /// Go `innerDeindentInit` + `innerDeindent`.
    inner_deindent: OnceLock<Value>,
    /// Go `posOffset`: the position in bytes in the source file (error logging).
    pub pos_offset: i64,
    /// Go `posInit` + `pos`.
    pos: OnceLock<Position>,
    /// Go `store` (created on first use in Go; the scratch is shared with nested shortcodes'
    /// `.Parent`, so it is created up front).
    pub scratch: Arc<nh_common::maps::scratch::Scratch>,
    /// The page (for `.Position` and `.Site`).
    pub(crate) hs: HugoSitesRef,
    pub(crate) p: PageId,
}

impl ShortcodeWithPage {
    /// The shortcode data of `sc` on page `p` (Go's `&ShortcodeWithPage{...}` literal in
    /// `doRenderShortcode`).
    pub(crate) fn new(
        h: &Arc<HugoSites>,
        p: PageId,
        sc: &crate::shortcode_parse::Shortcode,
        parent: Option<Arc<ShortcodeWithPage>>,
    ) -> ShortcodeWithPage {
        let is_named_params = matches!(sc.params, Value::Map(_));
        ShortcodeWithPage {
            params: sc.params.clone(),
            inner: OnceLock::new(),
            page: new_page_for_shortcode(h, p),
            parent,
            name: sc.name.clone(),
            is_named_params,
            ordinal: sc.ordinal,
            indentation: String::from_utf8_lossy(&sc.indentation).into_owned(),
            inner_deindent: OnceLock::new(),
            pos_offset: sc.pos,
            pos: OnceLock::new(),
            scratch: Arc::new(nh_common::maps::scratch::Scratch::new()),
            hs: h.self_ref.clone(),
            p,
        }
    }

    /// Go `Inner` (the zero `template.HTML` until set).
    pub fn inner_value(&self) -> Value {
        self.inner
            .get()
            .cloned()
            .unwrap_or_else(|| Value::html(GoString::empty()))
    }

    /// Go: `InnerDeindent()` — the (potentially de-indented) inner content.
    // Go: hugolib/shortcode.go:InnerDeindent
    pub fn inner_deindent(&self) -> Value {
        if self.indentation.is_empty() {
            return self.inner_value();
        }
        self.inner_deindent
            .get_or_init(|| {
                let inner = self.inner_value();
                let inner = value_bytes(&inner);
                let mut b: Vec<u8> = Vec::new();
                let ind = self.indentation.as_bytes();
                visit_lines_after_bytes(&inner, |s| {
                    if s.starts_with(ind) {
                        b.extend_from_slice(&s[ind.len()..]);
                    } else {
                        b.extend_from_slice(s);
                    }
                });
                Value::html(b)
            })
            .clone()
    }

    /// Go: `Position()` — this shortcode's detailed position (expensive; error situations).
    // Go: hugolib/shortcode.go:Position
    pub fn position(&self) -> Position {
        self.pos
            .get_or_init(|| {
                let h = upgrade_hs(&self.hs);
                let ps = h.page(self.p);
                ps.pos_offset(self.pos_offset)
            })
            .clone()
    }

    /// Go: `Site()` — `scp.Page.Site()`.
    // Go: hugolib/shortcode.go:Site
    pub fn site(&self) -> Value {
        let h = upgrade_hs(&self.hs);
        let ps = h.page(self.p);
        match h.sites[ps.site_idx].deps.site.get() {
            Some(s) => Value::Object(Arc::new(s.clone())),
            None => Value::TypedNil("page.Site".into()),
        }
    }

    /// Go: `Get(key)` — a positional or named parameter.
    // Go: hugolib/shortcode.go:Get
    pub fn get(&self, key: &Value) -> GoResult<Value> {
        let len = match &self.params {
            Value::Invalid => return Ok(Value::Invalid),
            Value::TypedNil(_) => 0,
            Value::List(l) => l.items.len(),
            Value::Map(m) => m.entries.len(),
            _ => 0,
        };
        if len == 0 {
            return Ok(Value::Invalid);
        }

        match key {
            Value::Int(i, _) => {
                match &self.params {
                    // We treat this as a non error, so people can do similar to
                    // {{ $myParam := .Get "myParam" | default .Get 0 }}
                    // Without having to do additional checks.
                    Value::Map(_) => Ok(Value::Invalid),
                    Value::List(l) => {
                        let idx = *i;
                        let ln = l.items.len() as i64;
                        if idx > ln - 1 {
                            return Ok(Value::string(""));
                        }
                        if idx < 0 {
                            return Err(go_value::Error::new("reflect: slice index out of range"));
                        }
                        Ok(interface_value(&l.items[idx as usize]))
                    }
                    _ => Err(zero_value_interface()),
                }
            }
            Value::String(s) => match &self.params {
                Value::Map(m) => match m.get(s.as_bytes()) {
                    Some(v) => Ok(interface_value(v)),
                    None => Ok(Value::string("")),
                },
                // We treat this as a non error (see above).
                Value::List(_) => Ok(Value::Invalid),
                _ => Err(zero_value_interface()),
            },
            _ => Err(zero_value_interface()),
        }
    }

    /// Go: `Unwrapv()` — the page.
    // Go: hugolib/shortcode.go:Unwrapv
    pub fn unwrapv(&self) -> Value {
        self.page.clone()
    }
}

/// `reflect.Value.Interface()` of an element of an `[]interface{}`/`map[string]interface{}`:
/// a nil element is the untyped nil.
fn interface_value(v: &Value) -> Value {
    v.clone()
}

/// Go: `x.Interface()` on the zero `reflect.Value` (an unsupported key type) panics.
fn zero_value_interface() -> go_value::Error {
    go_value::Error::new("reflect: call of reflect.Value.Interface on zero Value")
}

/// The bytes of a string-like value (`template.HTML`, `string`).
pub(crate) fn value_bytes(v: &Value) -> Vec<u8> {
    match v {
        Value::String(s) | Value::Safe(_, s) => s.as_bytes().to_vec(),
        _ => Vec::new(),
    }
}

/// Go: `text.VisitLinesAfter` over bytes (lines including their newline).
// Go: common/text/transform.go:VisitLinesAfter
pub(crate) fn visit_lines_after_bytes(s: &[u8], mut f: impl FnMut(&[u8])) {
    let mut s = s;
    while let Some(h) = s.iter().position(|&b| b == b'\n') {
        f(&s[..h + 1]);
        s = &s[h + 1..];
    }
    if !s.is_empty() {
        f(s);
    }
}

nh_common::go_methods!(ShortcodeWithPage {
    "InnerDeindent" => |s, _c, a| {
        nh_common::object::args::exactly(a, 0, "InnerDeindent")?;
        Ok(s.inner_deindent())
    },
    "Position" => |s, _c, a| {
        nh_common::object::args::exactly(a, 0, "Position")?;
        Ok(nh_markup::converter::hooks::position_value(s.position()))
    },
    "Site" => |s, _c, a| {
        nh_common::object::args::exactly(a, 0, "Site")?;
        Ok(s.site())
    },
    "Ref" => |s, _c, a| {
        nh_common::object::args::exactly(a, 1, "Ref")?;
        let m = ref_args(&a[0])?;
        Ok(Value::string(nh_page::page::Page::ref_(&page_of(s), &m)?))
    },
    "RelRef" => |s, _c, a| {
        nh_common::object::args::exactly(a, 1, "RelRef")?;
        let m = ref_args(&a[0])?;
        Ok(Value::string(nh_page::page::Page::rel_ref(&page_of(s), &m)?))
    },
    "Store" => |s, _c, a| {
        nh_common::object::args::exactly(a, 0, "Store")?;
        Ok(Value::Object(s.scratch.clone()))
    },
    "Scratch" => |s, _c, a| {
        nh_common::object::args::exactly(a, 0, "Scratch")?;
        Ok(Value::Object(s.scratch.clone()))
    },
    "Get" => |s, _c, a| {
        nh_common::object::args::exactly(a, 1, "Get")?;
        s.get(&a[0])
    },
    "Unwrapv" => |s, _c, a| {
        nh_common::object::args::exactly(a, 0, "Unwrapv")?;
        Ok(s.unwrapv())
    },
});

/// The page behind the shortcode's `.Page` (Go `scp.Page`, used by `RefFrom`/`RelRefFrom`).
fn page_of(s: &ShortcodeWithPage) -> PageHandle {
    PageHandle {
        h: upgrade_hs(&s.hs),
        id: s.p,
        wrapper: PageWrapper::ForShortcode,
    }
}

/// The `map[string]any` argument of `Ref`/`RelRef`.
fn ref_args(v: &Value) -> GoResult<go_value::Map> {
    match v {
        Value::Map(m) => Ok((**m).clone()),
        Value::Invalid => Ok(go_value::Map::new(MapType::StringAny)),
        _ => Err(go_value::Error::new(format!(
            "wrong type for value; expected map[string]interface {{}}; got {}",
            v.go_type_name()
        ))),
    }
}

impl Object for ShortcodeWithPage {
    nh_common::object_basics!("*hugolib.ShortcodeWithPage");

    fn field(&self, name: &str) -> Option<Value> {
        match name {
            "Params" => Some(self.params.clone()),
            "Inner" => Some(self.inner_value()),
            "Page" => Some(self.page.clone()),
            "Parent" => Some(match &self.parent {
                Some(p) => Value::Object(p.clone()),
                None => Value::TypedNil("*hugolib.ShortcodeWithPage".into()),
            }),
            "Name" => Some(Value::string(self.name.as_str())),
            "IsNamedParams" => Some(Value::Bool(self.is_named_params)),
            "Ordinal" => Some(Value::int(self.ordinal)),
            _ => None,
        }
    }
}

/// Go: `shortcodeRenderer` — typically used to delay rendering of inner shortcodes marked with
/// placeholders in the content. Returns the rendered bytes and whether the shortcode (or one of
/// its children) has template variations per output format.
pub trait ShortcodeRenderer: Send + Sync {
    fn render_shortcode(&self, ctx: &TplContext) -> Result<(Vec<u8>, bool)>;

    /// Go: `renderShortcodeString` (the same bytes; Go converts them to a string).
    fn render_shortcode_string(&self, ctx: &TplContext) -> Result<(Vec<u8>, bool)> {
        self.render_shortcode(ctx)
    }
}

/// Go: `shortcodeRenderFunc`.
pub struct ShortcodeRenderFunc(
    pub Arc<dyn Fn(&TplContext) -> Result<(Vec<u8>, bool)> + Send + Sync>,
);

impl ShortcodeRenderer for ShortcodeRenderFunc {
    // Go: hugolib/shortcode_page.go:renderShortcode
    fn render_shortcode(&self, ctx: &TplContext) -> Result<(Vec<u8>, bool)> {
        (self.0)(ctx)
    }
}

/// Go: `prerenderedShortcode`.
#[derive(Clone, Default)]
pub struct PrerenderedShortcode {
    pub s: Vec<u8>,
    pub has_variants: bool,
}

impl ShortcodeRenderer for PrerenderedShortcode {
    // Go: hugolib/shortcode_page.go:renderShortcode
    fn render_shortcode(&self, _ctx: &TplContext) -> Result<(Vec<u8>, bool)> {
        Ok((self.s.clone(), self.has_variants))
    }
}

/// Go: `zeroShortcode`.
pub fn zero_shortcode() -> PrerenderedShortcode {
    PrerenderedShortcode::default()
}

/// Go: `newPageForShortcode(p)` — the `.Page` of a shortcode (`*hugolib.pageForShortcode`:
/// `TableOfContents` returns the TOC placeholder, `Content` etc. are the nop page's).
// Go: hugolib/shortcode_page.go:newPageForShortcode
pub fn new_page_for_shortcode(h: &Arc<HugoSites>, p: PageId) -> Value {
    PageHandle {
        h: h.clone(),
        id: p,
        wrapper: PageWrapper::ForShortcode,
    }
    .page_ref()
    .to_value()
}

/// Go: `newPageForRenderHook(p)` — the `.Page` of a render hook (`*hugolib.pageForRenderHooks`).
// Go: hugolib/shortcode_page.go:newPageForRenderHook
pub fn new_page_for_render_hook(h: &Arc<HugoSites>, p: PageId) -> Value {
    PageHandle {
        h: h.clone(),
        id: p,
        wrapper: PageWrapper::ForRenderHooks,
    }
    .page_ref()
    .to_value()
}

/// Go: `createShortcodePlaceholder("TOC", 0, 0)` — the TableOfContents placeholder passed to
/// Goldmark etc. (Go `tocShortcodePlaceholder`).
pub fn toc_shortcode_placeholder() -> String {
    crate::shortcode_parse::create_shortcode_placeholder("TOC", 0, 0)
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: hugolib/shortcode_page.go (131 lines; 5/11 funcs executed)
//   types: shortcodeRenderer, shortcodeRenderFunc, prerenderedShortcode, pageForShortcode, pageForRenderHooks
// OK L36-38: (f shortcodeRenderFunc) renderShortcode(ctx context.Context) ([]byte, bool, error)
// OK L40-43: (f shortcodeRenderFunc) renderShortcodeString(ctx context.Context) (string, bool, error)
// OK L50-52: (p prerenderedShortcode) renderShortcode(context.Context) ([]byte, bool, error)
// OK L54-56: (p prerenderedShortcode) renderShortcodeString(context.Context) (string, bool, error)
// OK L80-89: newPageForShortcode(p *pageState) page.Page
//    L92-94: (p *pageForShortcode) Unwrapv() any                                  (T23: page_methods)
//    L96-98: (p *pageForShortcode) String() string                                (T23: page_methods)
//    L100-102: (p *pageForShortcode) TableOfContents(context.Context) template.HTML  (T23: page_methods)
// OK L115-123: newPageForRenderHook(p *pageState) page.Page
//    L125-127: (p *pageForRenderHooks) Unwrapv() any                              (T23: page_methods)
//    L129-131: (p *pageForRenderHooks) String() string                            (T23: page_methods)
// ---------------------------------------------------------------------------
