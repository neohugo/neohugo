//! Port of `tpl/internal/templatefuncsRegistry.go`.
//!
//! Owner: Wave B task T19 (tplfuncs-host).


//! Go `tpl/internal/templatefuncsRegistry.go` (registry part): each namespace contributes its
//! namespace object (`funcMap[ns] = ns.Context`, returning the namespace struct) and aliases
//! (`funcMap[alias] = method`).

use std::sync::Arc;

use go_value::{HostCtx, Object, Value};
use nh_tplimpl::engine::TplFunc;

/// Go: `internal.TemplateFuncsNamespace`.
pub struct TemplateFuncsNamespace {
    pub name: &'static str,
    /// Go `Context func(ctx, args...) (any, error)` — returns the namespace object (template
    /// `resources.Get ...` = call `resources` with no args, then method `Get`).
    pub context: TplFunc,
    /// alias -> implementation (Go `MethodMappings`).
    pub aliases: Vec<(String, TplFunc)>,
}

impl TemplateFuncsNamespace {
    /// Namespace backed by an `Object` whose Go methods implement the aliases.
    pub fn from_object(name: &'static str, obj: Arc<dyn Object>, aliases: &[(&str, &[&str])]) -> TemplateFuncsNamespace {
        let ns_value = Value::Object(obj.clone());
        let context: TplFunc = Arc::new(move |_ctx: HostCtx<'_>, _args: &[Value]| Ok(ns_value.clone()));
        let mut out = Vec::new();
        for (method, names) in aliases {
            for alias in names.iter() {
                let obj = obj.clone();
                let method = method.to_string();
                let f: TplFunc = Arc::new(move |ctx: HostCtx<'_>, args: &[Value]| {
                    obj.call_method(ctx, &method, args)
                        .unwrap_or_else(|| Err(go_value::Error::new(format!("method {method} not found"))))
                });
                out.push((alias.to_string(), f));
            }
        }
        TemplateFuncsNamespace { name, context, aliases: out }
    }
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: tpl/internal/templatefuncsRegistry.go (325 lines; 3/8 funcs executed)
//   types: TemplateFuncsNamespace, TemplateFuncsNamespaces, TemplateFuncMethodMapping, goDocFunc, methodGoDocInfo
// EX L42-44: AddTemplateFuncsNamespace(ns func(d *deps.Deps) *TemplateFuncsNamespace)
// EX L65-94: (t *TemplateFuncsNamespace) AddMethodMapping(m any, aliases []string, examples [][2]string)
// EX L115-121: methodToName(m any) string
//    L131-150: (t goDocFunc) toJSON() ([]byte, error)
//    L153-166: (namespaces TemplateFuncsNamespaces) ToMap() map[string]any
//    L169-191: (namespaces TemplateFuncsNamespaces) MarshalJSON() ([]byte, error)
//    L197-251: (t *TemplateFuncsNamespace) toJSON(ctx context.Context) ([]byte, error)
//    L263-325: getGetTplPackagesGoDoc() map[string]map[string]methodGoDocInfo
// ---------------------------------------------------------------------------
