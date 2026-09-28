//! Port of `tpl/site/init.go`.
//!
//! Owner: Wave B task T19 (tplfuncs-host).

use std::sync::Arc;

use go_value::{HostCtx, Value};
use nh_deps::deps::Deps;
use nh_tplimpl::engine::TplFunc;

use crate::internal::registry::TemplateFuncsNamespace;

/// `site` returns the current `page.Site` (deps.site).
pub const ALIASES: &[(&str, &[&str])] = &[];

/// Go wraps `d.Site` once, when the func map is created; the Rust site is bound later
/// (HUGO_LAYER.md §4.2), so the (already wrapped) `SiteRef` is read when `site` is called.
// Go: tpl/site/init.go:init
pub fn namespace(d: &Arc<Deps>) -> TemplateFuncsNamespace {
    let d = d.clone();
    let context: TplFunc = Arc::new(move |_ctx: HostCtx<'_>, _args: &[Value]| {
        // We just add the Site as the namespace here. No method mappings.
        match d.site.get() {
            Some(s) => Ok(s.to_value()),
            // Go: `page.WrapSite(nil)` is a nil `*page.siteWrapper`.
            None => Ok(Value::TypedNil(Arc::from("*page.siteWrapper"))),
        }
    });
    TemplateFuncsNamespace::new("site", context)
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: tpl/site/init.go (42 lines; 1/1 funcs executed)
// OK L28-42: init()
// ---------------------------------------------------------------------------
