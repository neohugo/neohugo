//! Port of `tpl/template.go`, `common/hcontext/context.go`.
//!
//! Owner: Wave B task T13 (tplimpl).

//! Go `tpl/template.go` + `common/hcontext`: the Rust stand-in for the `context.Context` that Go
//! threads through template execution. Passed to `go_value::Object::call_method` and to template
//! funcs as `HostCtx` (`&dyn Any`); host code downcasts with [`TplContext::from_host`].
//!
//! Values are owned/`Arc` so the context is `'static` (required by `dyn Any`) and cheap to clone
//! when a partial/hook derives a child context.

use std::any::Any;
use std::sync::Arc;

use go_unicode::utf8;
use go_value::{HostCtx, Value};

/// Go: `tpl.HugoDeferredTemplatePrefix` / `Suffix`.
pub const HUGO_DEFERRED_TEMPLATE_PREFIX: &str = "__hdeferred/";
pub const HUGO_DEFERRED_TEMPLATE_SUFFIX: &str = "__d=";

/// Go: `tpl.CurrentTemplateInfoCommonOps` of a base template (`CurrentTemplateInfoOps.Base()`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CurrentTemplateBase {
    /// Template name.
    pub name: String,
    /// Template source filename ("" for embedded templates).
    pub filename: String,
}

/// Go: `tpl.CurrentTemplateInfo` (for `templates.Current`, the 999-level recursion guard and
/// partialCached cycle detection).
#[derive(Clone, Debug)]
pub struct CurrentTemplateInfo {
    pub parent: Option<Arc<CurrentTemplateInfo>>,
    pub level: i64,
    /// Template name (Go `CurrentTemplateInfoOps.Name()`).
    pub name: String,
    /// Template source filename ("" for embedded templates).
    pub filename: String,
    /// Go `CurrentTemplateInfoOps.Base()`: the base template (baseof) the template was applied
    /// to, if any.
    pub base: Option<CurrentTemplateBase>,
}

/// Go: `tpl.CurrentTemplateInfos`.
pub type CurrentTemplateInfos = Vec<Arc<CurrentTemplateInfo>>;

/// Go: `(CurrentTemplateInfos).Reverse()` — a reversed copy.
// Go: tpl/template.go:Reverse
pub fn reverse_current_template_infos(c: &[Arc<CurrentTemplateInfo>]) -> CurrentTemplateInfos {
    if c.is_empty() {
        return c.to_vec();
    }
    let mut r = c.to_vec();
    r.reverse();
    r
}

impl CurrentTemplateInfo {
    /// Go: `Ancestors()` — the parents, nearest first.
    // Go: tpl/template.go:Ancestors
    pub fn ancestors(&self) -> CurrentTemplateInfos {
        let mut ancestors = Vec::new();
        let mut ti = self.parent.clone();
        while let Some(p) = ti {
            ti = p.parent.clone();
            ancestors.push(p);
        }
        ancestors
    }
}

/// The Go `context.Context` for template execution (keys of `tpl.Context` + `neohugo.Context`).
///
/// Go reads and writes these keys through `hcontext.ContextDispatcher`s
/// (`tpl.Context.Page.Get(ctx)`, `.Set(ctx, v)`); here they are fields: `Get` is a field read
/// (the zero value when unset, like `keyInContext.Get`) and `Set` is one of the `with_*`
/// constructors (a new context derived from the old one, like `context.WithValue`).
#[derive(Clone, Default)]
pub struct TplContext {
    /// Go: `tpl.Context.Page` — the page being rendered (a `page.Page` value), used by the `page`
    /// namespace, shortcodes and render hooks.
    pub page: Option<Value>,
    /// Go: `tpl.Context.CurrentTemplate`.
    pub current_template: Option<Arc<CurrentTemplateInfo>>,
    /// Go: `tpl.Context.IsInGoldmark`. Set ONLY by `prepareShortcode` for `{{% %}}` shortcodes in
    /// markdown (shortcode.go:327-332). Render hooks run with the caller's context unchanged
    /// (site.go:1504-1526) and must NOT set it: `RenderShortcodes` wraps its output in
    /// `hugocontext` when it is set (page__content.go:1119-1124).
    pub is_in_goldmark: bool,
    /// Go: `tpl.Context.DependencyScope` (identity tracking; kept for API shape).
    pub dependency_scope: i64,
    /// Go: `neohugo.Context.MarkupScope` (`.Content` rendering scope, e.g. "" or a custom scope).
    pub markup_scope: String,
    /// Host extension slot: nh-hugolib stores per-render state here (e.g. the page output index)
    /// without this crate knowing its type.
    pub host: Option<Arc<dyn Any + Send + Sync>>,
}

impl TplContext {
    /// Downcast a `HostCtx` passed through the template engine.
    // Go: common/hcontext/context.go:Get
    pub fn from_host<'a>(ctx: HostCtx<'a>) -> Option<&'a TplContext> {
        ctx.downcast_ref::<TplContext>()
    }

    /// As `HostCtx` for the engine / Object methods.
    pub fn as_host(&self) -> HostCtx<'_> {
        self
    }

    /// Go: `tpl.Context.Page.Set(ctx, p)`.
    // Go: common/hcontext/context.go:Set
    pub fn with_page(&self, page: Value) -> TplContext {
        TplContext {
            page: Some(page),
            ..self.clone()
        }
    }

    /// Go: `tpl.Context.CurrentTemplate.Set(ctx, &CurrentTemplateInfo{Parent, Level+1, ti})`
    /// (done by `TemplateStore.ExecuteWithContext`), for a template without a base template.
    pub fn with_current_template(&self, name: &str, filename: &str) -> TplContext {
        self.with_current_template_info(name, filename, None)
    }

    /// Go: `tpl.Context.CurrentTemplate.Set(ctx, &CurrentTemplateInfo{Parent: parent, Level:
    /// level, CurrentTemplateInfoOps: ti})` (templatestore.go:497-508): the level is the parent's
    /// plus one, or 0 at the top.
    pub fn with_current_template_info(
        &self,
        name: &str,
        filename: &str,
        base: Option<CurrentTemplateBase>,
    ) -> TplContext {
        let parent = self.current_template.clone();
        let level = parent.as_ref().map(|p| p.level + 1).unwrap_or(0);
        TplContext {
            current_template: Some(Arc::new(CurrentTemplateInfo {
                parent,
                level,
                name: name.to_string(),
                filename: filename.to_string(),
                base,
            })),
            ..self.clone()
        }
    }

    /// Go: `tpl.Context.IsInGoldmark.Set(ctx, true)` — only for `{{% %}}` shortcodes (see the
    /// field docs); never for render hooks.
    pub fn in_goldmark(&self) -> TplContext {
        TplContext {
            is_in_goldmark: true,
            ..self.clone()
        }
    }

    /// Go: `neohugo.Context.MarkupScope.Set(ctx, scope)`.
    pub fn with_markup_scope(&self, scope: &str) -> TplContext {
        TplContext {
            markup_scope: scope.to_string(),
            ..self.clone()
        }
    }
}

/// Go: `neohugo.GetMarkupScope(ctx)` reads `neohugo.Context.MarkupScope` from the context. The
/// context stand-in is [`TplContext`], so nh-config's reader is registered from here
/// (`nh_config::neohugo::neohugo::set_markup_scope_getter`). Idempotent. Called by
/// `nh_tplimpl::templatestore::TemplateStore::new`; a host that reads the scope before any
/// store exists calls it itself.
pub fn register_markup_scope_getter() {
    nh_config::neohugo::neohugo::set_markup_scope_getter(markup_scope_of);
}

/// The markup scope of a template context ("" for another context, as Go's zero value).
fn markup_scope_of(ctx: HostCtx<'_>) -> String {
    match TplContext::from_host(ctx) {
        Some(c) => c.markup_scope.clone(),
        None => String::new(),
    }
}

// Go: tpl/template.go:hugoNewLinePlaceholder
const HUGO_NEW_LINE_PLACEHOLDER: &[u8] = b"___hugonl_";

// Go: tpl/template.go:stripHTMLReplacerPre
/// `strings.NewReplacer("\n", " ", "</p>", hugoNewLinePlaceholder, "<br>",
/// hugoNewLinePlaceholder, "<br />", hugoNewLinePlaceholder).Replace(s)`: left to right, without
/// overlaps; at each position the first pair (in argument order) whose old string matches wins.
fn strip_html_replacer_pre(s: &[u8]) -> Vec<u8> {
    let mut pre = Vec::with_capacity(s.len());
    let mut i = 0;
    while i < s.len() {
        let rest = &s[i..];
        if rest[0] == b'\n' {
            pre.push(b' ');
            i += 1;
        } else if rest.starts_with(b"</p>") || rest.starts_with(b"<br>") {
            pre.extend_from_slice(HUGO_NEW_LINE_PLACEHOLDER);
            i += 4;
        } else if rest.starts_with(b"<br />") {
            pre.extend_from_slice(HUGO_NEW_LINE_PLACEHOLDER);
            i += 6;
        } else {
            pre.push(rest[0]);
            i += 1;
        }
    }
    pre
}

/// Go: `tpl.StripHTML(s)` — tags stripped with the html/template `stripTags` state machine
/// (gotemplate crate), `</p>`/`<br>`/`<br />` -> newline placeholder, runs of `unicode.IsSpace`
/// collapsed to their first rune. Entities are NOT decoded.
// Go: tpl/template.go:StripHTML
pub fn strip_html(s: &[u8]) -> Vec<u8> {
    // Shortcut strings with no tags in them
    if !s.iter().any(|&c| c == b'<' || c == b'>') {
        return s.to_vec();
    }

    let pre = strip_html_replacer_pre(s);
    let pre_replaced = pre != s;

    let mut s = gotemplate::html::strip_tags(&pre);

    if pre_replaced {
        s = go_unicode::strings::replace_all(&s, HUGO_NEW_LINE_PLACEHOLDER, b"\n").into_owned();
    }

    let mut was_space = false;
    let mut b = Vec::with_capacity(s.len());
    let mut i = 0;
    while i < s.len() {
        // Go: `for _, r := range s` decodes an invalid byte as U+FFFD (width 1), and
        // `WriteRune(utf8.RuneError)` writes its 3-byte encoding.
        let (r, size) = utf8::decode_rune(&s[i..]);
        let is_space = go_unicode::is_space(r);
        if !is_space || !was_space {
            utf8::append_rune(&mut b, r);
        }
        was_space = is_space;
        i += size;
    }

    if !b.is_empty() {
        s = b;
    }

    s
}

/// Go: `tpl.DeferredExecution` (templates.Defer; unused by seeksnack).
#[derive(Default)]
pub struct DeferredExecution {
    pub template_path: String,
    pub data: Option<Value>,
    pub executed: bool,
    pub result: String,
}

/// Go: `tpl.RenderingContext` (site index + output index; keys deferred executions).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct RenderingContext {
    pub site_index: usize,
    pub site_out_idx: usize,
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: tpl/template.go (189 lines; 2/4 funcs executed)
//   types: Template, RenderingContext, (group), page, site, DeferredExecution, CurrentTemplateInfoOps,
//          CurrentTemplateInfoCommonOps, CurrentTemplateInfo, CurrentTemplateInfos
// OK L73-81: init() (GetDependencyManagerInCurrentScope: watch-mode identity tracking, not ported)
// OK L103-134: StripHTML(s string) string
// OK L171-179: (c CurrentTemplateInfos) Reverse() CurrentTemplateInfos
// OK L182-189: (ti *CurrentTemplateInfo) Ancestors() CurrentTemplateInfos
// Source: common/hcontext/context.go (46 lines; 3/3 funcs executed)
//   types: ContextDispatcher[T, keyInContext[T
// OK L25-29: NewContextDispatcher[T any, R comparable](key R) ContextDispatcher[T] (TplContext fields)
// OK L36-42: (f keyInContext[T, R]) Get(ctx context.Context) T (field read; TplContext::from_host)
// OK L44-46: (f keyInContext[T, R]) Set(ctx context.Context, value T) context.Context (with_*)
// ---------------------------------------------------------------------------
