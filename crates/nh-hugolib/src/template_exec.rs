//! Module `template_exec`.
//!
//! NEW: the single dispatch point for every template execution nh-hugolib performs (test seam)
//!
//! Owner: Wave B task T20 (hugolib-capture), crate lead of nh-hugolib.


//! Every template execution that nh-hugolib itself starts goes through [`execute`]: pages and
//! pagers (`renderAndWritePage`, T23/T24), aliases (T24), markdown render hooks
//! (`HookRendererTemplate`, T22) and shortcodes (`renderShortcodeWithPage`, T22).
//!
//! In a normal build `HugoSites.template_executor` is `None` and [`execute`] calls the site's
//! `TemplateStore::execute_with_context` with the caller's context UNCHANGED (Go passes `cctx`
//! straight through, site.go:1504-1526). Tests install a [`TemplateExecutor`] to break the
//! dependency on the whole template function set (HUGO_LAYER.md §11.1):
//! * T22 replays hook and shortcode outputs recorded by a Go oracle, keyed by [`ExecCall`];
//! * T24 records the render order with a stub that writes the template name.
//!
//! Functions executed by templates (partials, `resources.ExecuteAsTemplate`, `markdownify`, ...)
//! do NOT come through here: they call the template store directly.

use std::sync::Arc;

use go_value::Value;
use nh_common::Result;
use nh_tpl::template::TplContext;
use nh_tplimpl::templatestore::TemplInfo;

use crate::hugo_sites::HugoSites;
use crate::page::PageId;

/// What is being executed (the replay key of the T22/T24 test executors).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum ExecKind {
    /// A page (or a pager page 1) rendered by `renderAndWritePage`.
    Page,
    /// Pager N >= 2 of a paginated page (`renderPaginator`).
    Pager(u32),
    /// An alias redirect page (`renderAliases`, page/1 aliases, main-language redirect).
    Alias,
    /// A markdown render hook: "link", "image", "heading", "table" (Go `hooks.RendererType`).
    Hook(&'static str),
    /// A shortcode, by name.
    Shortcode(String),
}

/// The identity of one template execution, as recorded by the Go oracles.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ExecCall {
    /// The page being rendered / whose content is being rendered (None for site-level aliases).
    pub page: Option<PageId>,
    /// Output format name (`html`, `json`, ...) of the page output that renders.
    pub output_format: String,
    pub kind: ExecKind,
    /// 0-based count of earlier calls with the same (page, output_format, kind) in this build.
    pub ordinal: u32,
}

/// Test seam: replaces template execution for everything nh-hugolib renders itself.
pub trait TemplateExecutor: Send + Sync {
    fn execute(&self, ctx: &TplContext, templ: &Arc<TemplInfo>, w: &mut Vec<u8>, data: &Value, call: &ExecCall) -> Result<()>;
}

/// Executes `templ` for site `site_idx`: through `h.template_executor` when set, else through
/// the site's template store (`ExecuteWithContext`). The context is passed through unchanged.
pub fn execute(
    h: &Arc<HugoSites>,
    site_idx: usize,
    ctx: &TplContext,
    templ: &Arc<TemplInfo>,
    w: &mut Vec<u8>,
    data: &Value,
    call: &ExecCall,
) -> Result<()> {
    todo!()
}
