//! Module `funcmap`.
//!
//! NEW: the template func map factory the `neohugo-rs` binary hands to `NewHugoSites`.
//!
//! Owner: Wave B task T25 (commands-cli).
//!
//! Production must use Go's `tplimplinit.CreateFuncMap` (`nh_tplfuncs::tplimplinit::
//! create_func_map`, T19). At this task's base that function and three of its namespaces
//! (`hugo`, `page`, `site`) are still `todo!()`, so the binary uses [`interim_func_map_factory`]:
//! `CreateFuncMap`'s loop over every other namespace (their real implementations; functions
//! that T19 has not ported yet still panic with `todo!()` when a template calls them), plus
//! Go's `return`/`try` mappings, `site` and `hugo` exactly as Go's namespaces define them
//! (`WrapSite(d.Site)`, `d.Site.Hugo()`), and `page` bound to a function that fails with an
//! explicit error. **I01:** once T19 lands, make [`production_func_map_factory`] return `None`
//! (hugolib then calls `tplimplinit::create_func_map`) and delete the interim factory.

use std::sync::Arc;

use nh_deps::deps::Deps;
use nh_hugolib::hugo_sites::FuncMapFactory;
use nh_tplfuncs::internal::registry::TemplateFuncsNamespace;
use nh_tplimpl::engine::{FuncMap, TplFunc};

/// The func map factory of the `neohugo-rs` binary (`None` = hugolib's default,
/// `tplimplinit::create_func_map`).
pub fn production_func_map_factory() -> Option<FuncMapFactory> {
    Some(interim_func_map_factory())
}

/// The namespaces whose constructors are ported at this task's base, in Go's registration order
/// (`tplimplinit` imports), without `hugo`, `page` and `site` (added by the factory).
fn ported_namespaces(d: &Arc<Deps>) -> Vec<TemplateFuncsNamespace> {
    use nh_tplfuncs as t;
    vec![
        t::cast::init::namespace(d),
        t::collections::init::namespace(d),
        t::compare::init::namespace(d),
        t::crypto::init::namespace(d),
        t::css::init::namespace(d),
        t::data::init::namespace(d),
        t::debug::init::namespace(d),
        t::diagrams::init::namespace(d),
        t::encoding::init::namespace(d),
        t::fmt::init::namespace(d),
        t::hash::init::namespace(d),
        t::images::init::namespace(d),
        t::inflect::init::namespace(d),
        t::js::init::namespace(d),
        t::lang::init::namespace(d),
        t::math::init::namespace(d),
        t::openapi3::init::namespace(d),
        t::os::init::namespace(d),
        t::partials::init::namespace(d),
        t::path::init::namespace(d),
        t::reflect::init::namespace(d),
        t::resources::init::namespace(d),
        t::safe::init::namespace(d),
        t::strings::init::namespace(d),
        t::templates::init::namespace(d),
        t::time::init::namespace(d),
        t::transform::init::namespace(d),
        t::urls::init::namespace(d),
    ]
}

/// See the module docs.
// Go: tpl/tplimplinit/tplimplinit.go:CreateFuncMap (the loop; `page` stubbed)
pub fn interim_func_map_factory() -> FuncMapFactory {
    Arc::new(|d: &Arc<Deps>| {
        let mut func_map = FuncMap::new();
        for ns in ported_namespaces(d) {
            func_map.insert(ns.name.to_string(), ns.context.clone());
            for (alias, f) in ns.aliases {
                func_map.insert(alias, f);
            }
        }
        // Go's extra method mappings of the partials and safe namespaces (not in their Rust
        // ALIASES tables): `return` (a no-op; templatetransform removes it) and `try` (the
        // engine handles the keyword; the func returns its argument).
        let ret: TplFunc = Arc::new(|_ctx, _args| Ok(go_value::Value::string("")));
        func_map.insert("return".to_string(), ret);
        let try_f: TplFunc = Arc::new(|_ctx, args: &[go_value::Value]| {
            Ok(args.first().cloned().unwrap_or(go_value::Value::Invalid))
        });
        func_map.insert("try".to_string(), try_f);
        // Go tpl/site/init.go: the namespace is `page.WrapSite(d.Site)` (read when called: the
        // port sets `Deps.site` when the sites are frozen, before any template runs).
        let dd = d.clone();
        let site: TplFunc = Arc::new(move |_ctx, _args| Ok(dd.site().to_value()));
        func_map.insert("site".to_string(), site);
        // Go tpl/hugo/init.go: the namespace is `d.Site.Hugo()`.
        let dd = d.clone();
        let hugo: TplFunc = Arc::new(move |_ctx, _args| {
            Ok(nh_config::neohugo::neohugo::hugo_info_value(
                &dd.site().0.hugo(),
            ))
        });
        func_map.insert("hugo".to_string(), hugo);
        // Go tpl/page/init.go (the current page of the template context): T19.
        let page: TplFunc = Arc::new(|_ctx, _args| {
            Err(go_value::Error::new(
                "neohugo-rs: the page template function is not ported yet (T19)",
            ))
        });
        func_map.insert("page".to_string(), page);
        func_map
    })
}
