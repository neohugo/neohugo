//! Port of `tpl/data/data.go`.
//!
//! Owner: Wave B task T19 (tplfuncs-host).

use std::any::Any;
use std::sync::Arc;

use go_value::{HostCtx, Object, Value};
use nh_common::object::GoResult;
use nh_deps::deps::Deps;

/// Go: `data.Namespace` (template value `*data.Namespace`).
pub struct Namespace {
    pub d: Arc<Deps>,
}

impl Namespace {
    // Go: tpl/data:New
    pub fn new(d: Arc<Deps>) -> Namespace {
        Namespace { d }
    }

    // Go: tpl/data:GetCSV
    pub fn get_csv(&self, ctx: HostCtx<'_>, args: &[Value]) -> GoResult<Value> {
        todo!()
    }

    // Go: tpl/data:GetJSON
    pub fn get_json(&self, ctx: HostCtx<'_>, args: &[Value]) -> GoResult<Value> {
        todo!()
    }
}

nh_common::go_methods!(Namespace {
    "GetCSV" => |n, ctx, a| n.get_csv(ctx, a),
    "GetJSON" => |n, ctx, a| n.get_json(ctx, a),
});

impl Object for Namespace {
    nh_common::object_basics!("*data.Namespace");
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: tpl/data/data.go (209 lines; 1/9 funcs executed)
//   types: Namespace
// EX L43-50: New(deps *deps.Deps) *Namespace
//    L67-103: (ns *Namespace) GetCSV(sep string, args ...any) (d [][]string, err error)
//    L108-141: (ns *Namespace) GetJSON(args ...any) (any, error)
//    L143-152: addDefaultHeaders(req *http.Request, accepts ...string)
//    L154-164: addUserProvidedHeaders(headers map[string]any, req *http.Request)
//    L166-175: hasHeaderValue(m http.Header, key, value string) bool
//    L177-180: hasHeaderKey(m http.Header, key string) bool
//    L182-196: toURLAndHeaders(urlParts []any) (string, map[string]any)
//    L199-209: parseCSV(c []byte, sep string) ([][]string, error)
// ---------------------------------------------------------------------------
