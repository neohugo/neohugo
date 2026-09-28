//! Port of `tpl/hugo/init.go`.
//!
//! Owner: Wave B task T19 (tplfuncs-host).

use std::sync::Arc;

use go_value::{HostCtx, Value};
use nh_deps::deps::Deps;
use nh_tplimpl::engine::TplFunc;

use crate::internal::registry::TemplateFuncsNamespace;

/// `hugo` returns `neohugo.HugoInfo` (`hugo.Environment` is a struct field).
pub const ALIASES: &[(&str, &[&str])] = &[];

/// Go panics when the deps have no site (`panic("no site in deps")`) while the func map is
/// created; the Rust site is bound after the func map exists (HUGO_LAYER.md §4.2), so the site is
/// read when `hugo` is called and its absence is that call's error.
// Go: tpl/hugo/init.go:init
pub fn namespace(d: &Arc<Deps>) -> TemplateFuncsNamespace {
    let d = d.clone();
    let context: TplFunc = Arc::new(move |_ctx: HostCtx<'_>, _args: &[Value]| {
        // We just add the Hugo struct as the namespace here. No method mappings.
        match d.site.get() {
            Some(s) => Ok(Value::object(s.0.hugo())),
            None => Err(go_value::Error::new("no site in deps")),
        }
    });
    TemplateFuncsNamespace::new("hugo", context)
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: tpl/hugo/init.go (44 lines; 1/1 funcs executed)
// OK L26-44: init()
// ---------------------------------------------------------------------------
