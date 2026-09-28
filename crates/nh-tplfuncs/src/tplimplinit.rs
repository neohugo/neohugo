//! Port of `tpl/tplimplinit/tplimplinit.go`.
//!
//! Owner: Wave B task T19 (tplfuncs-host).

//! Go `tpl/tplimplinit.CreateFuncMap(d)`: all namespaces + aliases for one site (duplicates are a
//! bug). The text/template builtins not overridden by Hugo (`and`, `or`, `not`, `len`, `html`,
//! `urlquery`, `call`) and the html/template escaper funcs are added by the template engine
//! (nh-tplimpl), as in Go.
//!
//! Go registers each namespace from the `init()` of its package; `tplimplinit` imports them all,
//! so the registry holds them in import order. The port lists the constructors explicitly
//! ([`namespaces`]), in the same order.

use std::sync::Arc;

use go_value::{HostCtx, Value};
use nh_deps::deps::Deps;
use nh_tplimpl::engine::{FuncMap, TplFunc};

use crate::internal::registry::TemplateFuncsNamespace;

/// All namespace constructors (Go: package init order in tplimplinit imports).
pub fn namespaces(d: &Arc<Deps>) -> Vec<TemplateFuncsNamespace> {
    vec![
        crate::cast::init::namespace(d),
        crate::collections::init::namespace(d),
        crate::compare::init::namespace(d),
        crate::crypto::init::namespace(d),
        crate::css::init::namespace(d),
        crate::data::init::namespace(d),
        crate::debug::init::namespace(d),
        crate::diagrams::init::namespace(d),
        crate::encoding::init::namespace(d),
        crate::fmt::init::namespace(d),
        crate::hash::init::namespace(d),
        crate::hugo::init::namespace(d),
        crate::images::init::namespace(d),
        crate::inflect::init::namespace(d),
        crate::js::init::namespace(d),
        crate::lang::init::namespace(d),
        crate::math::init::namespace(d),
        crate::openapi3::init::namespace(d),
        crate::os::init::namespace(d),
        crate::page::init::namespace(d),
        crate::partials::init::namespace(d),
        crate::path::init::namespace(d),
        crate::reflect::init::namespace(d),
        crate::resources::init::namespace(d),
        with_try(crate::safe::init::namespace(d)),
        crate::site::init::namespace(d),
        crate::strings::init::namespace(d),
        crate::templates::init::namespace(d),
        crate::time::init::namespace(d),
        crate::transform::init::namespace(d),
        crate::urls::init::namespace(d),
    ]
}

/// Go's `tpl/safe` init also maps `func(v any) (any, error) { return v, nil }` to `try` (the
/// engine wraps the call's result or error in a `TryValue`). T18's `safe::init::ALIASES` holds
/// the method aliases; the anonymous func is added here.
// Go: tpl/safe/init.go:init
fn with_try(mut ns: TemplateFuncsNamespace) -> TemplateFuncsNamespace {
    let f: TplFunc = Arc::new(|_ctx: HostCtx<'_>, args: &[Value]| {
        nh_common::object::args::exactly(args, 1, "try")?;
        Ok(args[0].clone())
    });
    ns.add_method_mapping(f, &["try"]);
    ns
}

/// Go: `tplimplinit.CreateFuncMap(d)` — the namespace funcs (`funcMap[ns.Name] = ns.Context`)
/// and their aliases. A duplicate name panics, as in Go. Go's `OnCreated` hooks (only the
/// `resources` namespace has one, to reach the css and js namespaces) are resolved inside that
/// namespace (`resources::Namespace::css_ns`/`js_ns`), which shares the css and js clients of
/// this site's namespaces (`css::css::Namespace::new` / `js::js::Namespace::new` reuse them).
// Go: tpl/tplimplinit/tplimplinit.go:CreateFuncMap
pub fn create_func_map(d: &Arc<Deps>) -> FuncMap {
    let mut func_map = FuncMap::new();

    // Merge the namespace funcs
    for ns in namespaces(d) {
        if func_map.contains_key(ns.name) {
            panic!("{} is a duplicate template func", ns.name);
        }
        func_map.insert(ns.name.to_string(), ns.context.clone());
        for (alias, f) in ns.aliases {
            if func_map.contains_key(&alias) {
                panic!("{alias} is a duplicate template func");
            }
            func_map.insert(alias, f);
        }
    }

    func_map
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: tpl/tplimplinit/tplimplinit.go (96 lines; 1/1 funcs executed)
// OK L60-96: CreateFuncMap(d *deps.Deps) map[string]any
// ---------------------------------------------------------------------------
