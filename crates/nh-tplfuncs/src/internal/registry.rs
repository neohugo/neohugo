//! Port of `tpl/internal/templatefuncsRegistry.go`.
//!
//! Owner: Wave B task T19 (tplfuncs-host).

//! Go `tpl/internal/templatefuncsRegistry.go` (registry part): each namespace contributes its
//! namespace object (`funcMap[ns] = ns.Context`, returning the namespace struct) and aliases
//! (`funcMap[alias] = method`).
//!
//! Go registers the namespaces from `init()` functions into the package-level
//! `TemplateFuncsNamespaceRegistry`; the port has no `init()`: `tplimplinit::namespaces` is the
//! explicit, ordered list of constructors (Go's `AddTemplateFuncsNamespace` calls, in the import
//! order of `tpl/tplimplinit`).
//!
//! Go's `MethodMappings` is a map keyed by the method name, so its iteration order is random;
//! the order only decides which duplicate alias panics first in `CreateFuncMap`, and Hugo has no
//! duplicate. The port keeps the aliases in their `AddMethodMapping` order.

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

/// A template func that calls `method` on `obj` (Go's method value `ctx.Method`).
pub fn method_func(obj: Arc<dyn Object>, method: &str) -> TplFunc {
    let method = method.to_string();
    Arc::new(move |ctx: HostCtx<'_>, args: &[Value]| {
        obj.call_method(ctx, &method, args)
            .unwrap_or_else(|| Err(go_value::Error::new(format!("method {method} not found"))))
    })
}

/// The func map entry `alias` for `method` of `obj`. text/template checks the argument count
/// of a function against its signature and names the function as the template called it (the
/// alias); the methods check their own arguments and name themselves, so the arity error of
/// the method is reported under the alias here.
pub fn alias_func(obj: Arc<dyn Object>, method: &str, alias: &str) -> TplFunc {
    let method = method.to_string();
    let from = format!("wrong number of args for {method}: ");
    let to = format!("wrong number of args for {alias}: ");
    Arc::new(
        move |ctx: HostCtx<'_>, args: &[Value]| match obj.call_method(ctx, &method, args) {
            Some(Err(e)) if e.message().starts_with(&from) => Err(go_value::Error::new(format!(
                "{to}{}",
                &e.message()[from.len()..]
            ))),
            Some(r) => r,
            None => Err(go_value::Error::new(format!("method {method} not found"))),
        },
    )
}

impl TemplateFuncsNamespace {
    /// A namespace with the given `Context` func and no method mappings (Go's struct literal
    /// before any `AddMethodMapping`).
    pub fn new(name: &'static str, context: TplFunc) -> TemplateFuncsNamespace {
        TemplateFuncsNamespace {
            name,
            context,
            aliases: Vec::new(),
        }
    }

    /// Go: `AddMethodMapping(m, aliases, examples)` — `m` is registered under every alias
    /// (the examples are documentation only). Go panics on an empty alias.
    // Go: tpl/internal/templatefuncsRegistry.go:AddMethodMapping
    pub fn add_method_mapping(&mut self, m: TplFunc, aliases: &[&str]) {
        for a in aliases {
            if a.is_empty() {
                panic!("{}: Empty alias", self.name);
            }
            self.aliases.push((a.to_string(), m.clone()));
        }
    }

    /// Namespace backed by an `Object` whose Go methods implement the aliases (Go:
    /// `Context: func(cctx context.Context, args ...any) (any, error) { return ctx, nil }` plus
    /// one `AddMethodMapping(ctx.Method, aliases, ...)` per entry).
    pub fn from_object(
        name: &'static str,
        obj: Arc<dyn Object>,
        aliases: &[(&str, &[&str])],
    ) -> TemplateFuncsNamespace {
        let ns_value = Value::Object(obj.clone());
        let context: TplFunc =
            Arc::new(move |_ctx: HostCtx<'_>, _args: &[Value]| Ok(ns_value.clone()));
        let mut ns = TemplateFuncsNamespace::new(name, context);
        for (method, names) in aliases {
            for alias in names.iter() {
                ns.add_method_mapping(alias_func(obj.clone(), method, alias), &[alias]);
            }
        }
        ns
    }
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: tpl/internal/templatefuncsRegistry.go (325 lines; 3/8 funcs executed)
//   types: TemplateFuncsNamespace, TemplateFuncsNamespaces, TemplateFuncMethodMapping, goDocFunc, methodGoDocInfo
// OK L42-44: AddTemplateFuncsNamespace(ns func(d *deps.Deps) *TemplateFuncsNamespace) (the explicit list `tplimplinit::namespaces`)
// OK L65-94: (t *TemplateFuncsNamespace) AddMethodMapping(m any, aliases []string, examples [][2]string)
// OK L115-121: methodToName(m any) string (runtime reflection: the method name is written at each mapping)
//    L131-150: (t goDocFunc) toJSON() ([]byte, error) (`hugo gen docshelper` only; not ported)
//    L153-166: (namespaces TemplateFuncsNamespaces) ToMap() map[string]any (docshelper only; not ported)
//    L169-191: (namespaces TemplateFuncsNamespaces) MarshalJSON() ([]byte, error) (docshelper only; not ported)
//    L197-251: (t *TemplateFuncsNamespace) toJSON(ctx context.Context) ([]byte, error) (docshelper only; not ported)
//    L263-325: getGetTplPackagesGoDoc() map[string]map[string]methodGoDocInfo (docshelper only; not ported)
// ---------------------------------------------------------------------------
