//! Port of `tpl/time/init.go`.
//!
//! Owner: Wave B task T19 (tplfuncs-host).

use std::sync::Arc;

use go_value::{HostCtx, Object, Value};
use nh_deps::deps::Deps;
use nh_tplimpl::engine::TplFunc;

use crate::internal::registry::{TemplateFuncsNamespace, alias_func};

/// Go: `tpl/time` init — namespace `time` and its aliases.
pub const ALIASES: &[(&str, &[&str])] = &[
    ("Format", &["dateFormat"]),
    ("Now", &["now"]),
    ("Duration", &["duration"]),
];

// Go: tpl/time/init.go:init
pub fn namespace(d: &Arc<Deps>) -> TemplateFuncsNamespace {
    let ctx = Arc::new(super::time::Namespace::new(d.clone()));
    let obj: Arc<dyn Object> = ctx.clone();
    let ns_value = Value::Object(obj.clone());

    let context: TplFunc = Arc::new(move |_cctx: HostCtx<'_>, args: &[Value]| {
        // Handle overlapping "time" namespace and func.
        //
        // If no args are passed to `time`, assume namespace usage and return namespace context.
        //
        // If args are passed, call AsTime().
        match args.len() {
            0 => Ok(ns_value.clone()),
            1 => ctx.as_time_impl(&args[0], &[]),
            2 => ctx.as_time_impl(&args[0], &args[1..]),
            // 3 or more arguments. Currently not supported.
            _ => Err(go_value::Error::new("invalid arguments supplied to `time`")),
        }
    });

    let mut ns = TemplateFuncsNamespace::new("time", context);
    for (method, names) in ALIASES {
        for alias in names.iter() {
            ns.add_method_mapping(alias_func(obj.clone(), method, alias), &[alias]);
        }
    }
    ns
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: tpl/time/init.go (96 lines; 1/1 funcs executed)
// OK L27-96: init()
// ---------------------------------------------------------------------------
