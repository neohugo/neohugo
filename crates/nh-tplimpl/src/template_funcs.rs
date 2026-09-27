//! Port of `tpl/tplimpl/template_funcs.go`.
//!
//! Owner: Wave B task T13 (tplimpl).


//! Go `tpl/tplimpl/template_funcs.go`: `templateExecHelper`.

use std::sync::{Arc, OnceLock};

use go_value::{HostCtx, Value};
use nh_common::object::NamedTypeRegistry;
use nh_page::site::SiteRef;

use crate::engine::{ExecHelper, FuncMap, TplFunc};

/// Go: `templateExecHelper`.
///
/// `mainsections` (Go `GetMethod`, template_funcs.go:88-114): the special case fires only when
/// the receiver IS the site's params map (pointer equality). Rust: `Value::Map(m)` with
/// `Arc::ptr_eq(m, <site params Arc>)`; nh-hugolib must hand out the SAME `Arc<Map>` for
/// `.Site.Params` every time (never rebuild it per call).
pub struct TemplateExecHelper {
    /// Hugo funcs + html escaper funcs + text builtins (Hugo wins on name clashes).
    pub funcs: Arc<FuncMap>,
    pub site: Arc<OnceLock<SiteRef>>,
    pub named_types: Arc<NamedTypeRegistry>,
}

impl ExecHelper for TemplateExecHelper {
    // Go: tpl/tplimpl/template_funcs.go:GetFunc
    fn get_func(&self, ctx: HostCtx<'_>, name: &str) -> Option<TplFunc> {
        self.funcs.get(name).cloned()
    }

    // Go: tpl/tplimpl/template_funcs.go:GetMethod (membership)
    fn has_method(&self, ctx: HostCtx<'_>, receiver: &Value, name: &str) -> bool {
        todo!()
    }

    // Go: tpl/tplimpl/template_funcs.go:GetMethod (+ evalCall)
    fn call_method(&self, ctx: HostCtx<'_>, receiver: &Value, name: &str, args: &[Value]) -> go_value::Result<Value> {
        todo!()
    }

    // Go: tpl/tplimpl/template_funcs.go:GetMapValue
    fn get_map_value(&self, ctx: HostCtx<'_>, receiver: &Value, key: &Value) -> Option<Value> {
        todo!()
    }

    /// Go: `hreflect.IsTruthfulValue` (common/hreflect/helpers.go:96-130), used by the fork's
    /// `isTrue` (texttemplate/hugo_template.go:434-436).
    // Go: common/hreflect/helpers.go:IsTruthfulValue
    fn is_true(&self, v: &Value) -> bool {
        nh_common::hreflect::is_truthful(v)
    }
}

/// Methods of `time.Time` reachable from templates (`.Format`, `.IsZero`, `.Year`, `.Unix`,
/// `.UTC`, `.Local`, `.In`, `.Before`, `.After`, `.Equal`, `.AddDate`, `.Month`, `.Day`, ...),
/// implemented over go-time. Used by `has_method`/`call_method` for `Value::Time` receivers.
pub fn time_has_method(name: &str) -> bool {
    todo!()
}

pub fn time_call_method(t: &go_value::Time, name: &str, args: &[Value]) -> Option<go_value::Result<Value>> {
    todo!()
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: tpl/tplimpl/template_funcs.go (175 lines; 5/6 funcs executed)
//   types: templateExecHelper
// EX L43-58: (t *templateExecHelper) GetFunc(ctx context.Context, tmpl texttemplate.Preparer, name string) (fn reflect.Value, firstArg reflect.Value, found bool)
// EX L60-68: (t *templateExecHelper) Init(ctx context.Context, tmpl texttemplate.Preparer)
// EX L70-84: (t *templateExecHelper) GetMapValue(ctx context.Context, tmpl texttemplate.Preparer, receiver, key reflect.Value) (reflect.Value, bool)
// EX L88-114: (t *templateExecHelper) GetMethod(ctx context.Context, tmpl texttemplate.Preparer, receiver reflect.Value, name string) (method reflect.Value, firs...
// EX L116-138: (t *templateExecHelper) OnCalled(ctx context.Context, tmpl texttemplate.Preparer, name string, args []reflect.Value, result reflect.Value)
//    L140-175: (t *templateExecHelper) trackDependencies(ctx context.Context, tmpl texttemplate.Preparer, name string, receiver reflect.Value) context.Context
// ---------------------------------------------------------------------------
