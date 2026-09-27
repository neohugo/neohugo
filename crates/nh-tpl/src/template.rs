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

use go_value::{HostCtx, Value};

/// Go: `tpl.HugoDeferredTemplatePrefix` / `Suffix`.
pub const HUGO_DEFERRED_TEMPLATE_PREFIX: &str = "__hdeferred/";
pub const HUGO_DEFERRED_TEMPLATE_SUFFIX: &str = "__d=";

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
}

/// The Go `context.Context` for template execution (keys of `tpl.Context` + `neohugo.Context`).
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
    pub fn from_host<'a>(ctx: HostCtx<'a>) -> Option<&'a TplContext> {
        ctx.downcast_ref::<TplContext>()
    }

    /// As `HostCtx` for the engine / Object methods.
    pub fn as_host(&self) -> HostCtx<'_> {
        self
    }

    /// Go: `tpl.Context.Page.Set(ctx, p)`.
    pub fn with_page(&self, page: Value) -> TplContext {
        TplContext { page: Some(page), ..self.clone() }
    }

    /// Go: `tpl.Context.CurrentTemplate.Set(ctx, &CurrentTemplateInfo{Parent, Level+1, ti})`
    /// (done by `TemplateStore.ExecuteWithContext`).
    pub fn with_current_template(&self, name: &str, filename: &str) -> TplContext {
        let parent = self.current_template.clone();
        let level = parent.as_ref().map(|p| p.level + 1).unwrap_or(0);
        TplContext {
            current_template: Some(Arc::new(CurrentTemplateInfo {
                parent,
                level,
                name: name.to_string(),
                filename: filename.to_string(),
            })),
            ..self.clone()
        }
    }

    /// Go: `tpl.Context.IsInGoldmark.Set(ctx, true)` — only for `{{% %}}` shortcodes (see the
    /// field docs); never for render hooks.
    pub fn in_goldmark(&self) -> TplContext {
        TplContext { is_in_goldmark: true, ..self.clone() }
    }
}

/// Go: `tpl.StripHTML(s)` — tags stripped with the html/template `stripTags` state machine
/// (gotemplate crate), `</p>`/`<br>`/`<br />` -> newline placeholder, runs of `unicode.IsSpace`
/// collapsed to their first rune. Entities are NOT decoded.
// Go: tpl/template.go:StripHTML
pub fn strip_html(s: &[u8]) -> Vec<u8> {
    todo!("gotemplate::htmltemplate::strip_tags + go-unicode IsSpace")
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
// EX L73-81: init()
// EX L103-134: StripHTML(s string) string
//    L171-179: (c CurrentTemplateInfos) Reverse() CurrentTemplateInfos
//    L182-189: (ti *CurrentTemplateInfo) Ancestors() CurrentTemplateInfos
// Source: common/hcontext/context.go (46 lines; 3/3 funcs executed)
//   types: ContextDispatcher[T, keyInContext[T
// EX L25-29: NewContextDispatcher[T any, R comparable](key R) ContextDispatcher[T]
// EX L36-42: (f keyInContext[T, R]) Get(ctx context.Context) T
// EX L44-46: (f keyInContext[T, R]) Set(ctx context.Context, value T) context.Context
// ---------------------------------------------------------------------------
