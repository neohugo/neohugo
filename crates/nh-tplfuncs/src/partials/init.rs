//! Port of `tpl/partials/init.go`.
//!
//! Owner: Wave B task T19 (tplfuncs-host).

use std::sync::Arc;

use go_value::{HostCtx, Object, Value};
use nh_deps::deps::Deps;
use nh_tplimpl::engine::TplFunc;

use crate::internal::registry::{TemplateFuncsNamespace, alias_func};

/// Go: `tpl/partials` init — namespace `partials` and its aliases (plus the `return` func,
/// see [`namespace`]).
pub const ALIASES: &[(&str, &[&str])] = &[
    ("Include", &["partial"]),
    ("IncludeCached", &["partialCached"]),
];

// Go: tpl/partials/init.go:init
pub fn namespace(d: &Arc<Deps>) -> TemplateFuncsNamespace {
    let obj: Arc<dyn Object> = Arc::new(super::partials::Namespace::new(d.clone()));
    let mut ns = TemplateFuncsNamespace::from_object("partials", obj.clone(), &[]);

    ns.add_method_mapping(alias_func(obj.clone(), "Include", "partial"), &["partial"]);

    // TODO(bep) we need the return to be a valid identifiers, but should consider another way
    // of adding it. (Go: `func() string { return "" }`; the template transformer rewrites the
    // `return` statement, this func only makes the name known to the parser.)
    let ret: TplFunc = Arc::new(|_ctx: HostCtx<'_>, a: &[Value]| {
        nh_common::object::args::exactly(a, 0, "return")?;
        Ok(Value::string(""))
    });
    ns.add_method_mapping(ret, &["return"]);

    ns.add_method_mapping(
        alias_func(obj, "IncludeCached", "partialCached"),
        &["partialCached"],
    );

    ns
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: tpl/partials/init.go (57 lines; 1/1 funcs executed)
// OK L25-57: init()
// ---------------------------------------------------------------------------
