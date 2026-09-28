//! Port of `tpl/page/init.go`.
//!
//! Owner: Wave B task T19 (tplfuncs-host).

use std::sync::Arc;

use go_value::{HostCtx, Value};
use nh_deps::deps::Deps;
use nh_tpl::template::TplContext;
use nh_tplimpl::engine::TplFunc;

use crate::internal::registry::TemplateFuncsNamespace;

/// `page` returns the page in the context (`tpl.Context.Page`), or nil.
pub const ALIASES: &[(&str, &[&str])] = &[];

// Go: tpl/page/init.go:init
pub fn namespace(_d: &Arc<Deps>) -> TemplateFuncsNamespace {
    let context: TplFunc = Arc::new(|ctx: HostCtx<'_>, _args: &[Value]| {
        match TplContext::from_host(ctx).and_then(|c| c.page.clone()) {
            // The multilingual sitemap does not have a page as its context.
            None => Ok(Value::Invalid),
            Some(p) => Ok(p),
        }
    });
    TemplateFuncsNamespace::new("page", context)
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: tpl/page/init.go (49 lines; 1/1 funcs executed)
// OK L30-49: init()
// ---------------------------------------------------------------------------
