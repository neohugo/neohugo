//! Port of `tpl/tplimplinit/tplimplinit.go`.
//!
//! Owner: Wave B task T19 (tplfuncs-host).


//! Go `tpl/tplimplinit.CreateFuncMap(d)`: all namespaces + aliases for one site (duplicates are a
//! bug). Also merges the text/template builtins not overridden by Hugo (`and`, `or`, `not`, `len`,
//! `html`, `urlquery`, `call`) and the html/template escaper funcs — that part is done by
//! nh-tplimpl `configureSiteStorage` when building the exec helper.

use std::sync::Arc;

use nh_deps::deps::Deps;
use nh_tplimpl::engine::FuncMap;

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
        crate::safe::init::namespace(d),
        crate::site::init::namespace(d),
        crate::strings::init::namespace(d),
        crate::templates::init::namespace(d),
        crate::time::init::namespace(d),
        crate::transform::init::namespace(d),
        crate::urls::init::namespace(d),
    ]
}

/// Go: `tplimplinit.CreateFuncMap(d)`. Extra non-namespace aliases: `return` (partials; a no-op
/// func, the real work is the AST rewrite), `try` (safe).
// Go: tpl/tplimplinit/tplimplinit.go:CreateFuncMap
pub fn create_func_map(d: &Arc<Deps>) -> FuncMap {
    todo!()
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: tpl/tplimplinit/tplimplinit.go (96 lines; 1/1 funcs executed)
// EX L60-96: CreateFuncMap(d *deps.Deps) map[string]any
// ---------------------------------------------------------------------------
