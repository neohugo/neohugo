//! Port of `tpl/cast/cast.go`.
//!
//! Owner: Wave B task T18 (tplfuncs-data).

use std::sync::Arc;

use go_value::{HostCtx, Object, SafeKind, Value};
use nh_common::cast::caste;
use nh_common::object::{GoResult, args};
use nh_deps::deps::Deps;

/// Go: `cast.Namespace` (template value `*cast.Namespace`).
pub struct Namespace {
    pub d: Arc<Deps>,
}

impl Namespace {
    // Go: tpl/cast:New
    pub fn new(d: Arc<Deps>) -> Namespace {
        Namespace { d }
    }

    // Go: tpl/cast:ToFloat
    pub fn to_float(&self, _ctx: HostCtx<'_>, a: &[Value]) -> GoResult<Value> {
        args::exactly(a, 1, "ToFloat")?;
        // Go: tpl/cast/cast.go:ToFloat
        let v = convert_template_to_string(&a[0]);
        Ok(Value::float64(caste::to_float64_e(&v)?))
    }

    // Go: tpl/cast:ToInt
    pub fn to_int(&self, _ctx: HostCtx<'_>, a: &[Value]) -> GoResult<Value> {
        args::exactly(a, 1, "ToInt")?;
        // Go: tpl/cast/cast.go:ToInt
        let v = convert_template_to_string(&a[0]);
        Ok(Value::int(caste::to_int_e(&v)?))
    }

    // Go: tpl/cast:ToString
    pub fn to_string(&self, _ctx: HostCtx<'_>, a: &[Value]) -> GoResult<Value> {
        args::exactly(a, 1, "ToString")?;
        // Go: tpl/cast/cast.go:ToString
        Ok(Value::String(caste::to_string_e(&a[0])?))
    }
}

// Go: tpl/cast/cast.go:convertTemplateToString
fn convert_template_to_string(v: &Value) -> Value {
    match v {
        Value::Safe(
            SafeKind::Html | SafeKind::Css | SafeKind::HtmlAttr | SafeKind::Js | SafeKind::JsStr,
            s,
        ) => Value::String(s.clone()),
        _ => v.clone(),
    }
}

nh_common::go_methods!(Namespace {
    "ToFloat" => |n, ctx, a| n.to_float(ctx, a),
    "ToInt" => |n, ctx, a| n.to_int(ctx, a),
    "ToString" => |n, ctx, a| n.to_string(ctx, a),
});

impl Object for Namespace {
    nh_common::object_basics!("*cast.Namespace");
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: tpl/cast/cast.go (62 lines; 1/5 funcs executed)
//   types: Namespace
// OK L24-26: New() *Namespace
// OK L32-35: (ns *Namespace) ToInt(v any) (int, error)
// OK L38-40: (ns *Namespace) ToString(v any) (string, error)
// OK L43-46: (ns *Namespace) ToFloat(v any) (float64, error)
// OK L48-62: convertTemplateToString(v any) any
// ---------------------------------------------------------------------------
