//! Port of `tpl/data/data.go`.
//!
//! Owner: Wave B task T19 (tplfuncs-host).
//!
//! STUB: `data.GetJSON`/`data.GetCSV` (deprecated since v0.123.0) fetch local files or remote
//! URLs through their own getjson/getcsv file caches; neither is on the seeksnack build path and
//! the port does no network access, so both log Go's deprecation and return an explicit
//! `neohugo-rs` error (HUGO_LAYER.md §1 rule 5). Use `resources.Get`/`resources.GetRemote` with
//! `transform.Unmarshal`.

use std::sync::Arc;

use go_value::{HostCtx, Object, Value};
use nh_common::object::{GoResult, args};
use nh_config::neohugo::neohugo::deprecate;
use nh_deps::deps::Deps;

/// Go: `data.Namespace` (template value `*data.Namespace`).
pub struct Namespace {
    pub d: Arc<Deps>,
}

impl Namespace {
    /// New returns a new instance of the data-namespaced template functions.
    // Go: tpl/data/data.go:New
    pub fn new(d: Arc<Deps>) -> Namespace {
        Namespace { d }
    }

    /// GetCSV expects the separator sep and one or n-parts of a URL to a resource which can
    /// either be a local or a remote one. STUB (see the module docs).
    // Go: tpl/data/data.go:GetCSV
    pub fn get_csv(&self, _ctx: HostCtx<'_>, a: &[Value]) -> GoResult<Value> {
        args::at_least(a, 1, "GetCSV")?;
        args::string(a, 0)?;
        deprecate(
            "data.GetCSV",
            "use resources.Get or resources.GetRemote with transform.Unmarshal.",
            "v0.123.0",
        );
        Err(go_value::Error::new(
            "neohugo-rs: data.GetCSV (getCSV) is not supported",
        ))
    }

    /// GetJSON expects one or n-parts of a URL in args to a resource which can either be a
    /// local or a remote one. STUB (see the module docs).
    // Go: tpl/data/data.go:GetJSON
    pub fn get_json(&self, _ctx: HostCtx<'_>, _a: &[Value]) -> GoResult<Value> {
        deprecate(
            "data.GetJSON",
            "use resources.Get or resources.GetRemote with transform.Unmarshal.",
            "v0.123.0",
        );
        Err(go_value::Error::new(
            "neohugo-rs: data.GetJSON (getJSON) is not supported",
        ))
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
// OK L43-50: New(deps *deps.Deps) *Namespace (the file caches are not needed by the stubs)
// STUB L67-103: (ns *Namespace) GetCSV(sep string, args ...any) (d [][]string, err error)
// STUB L108-141: (ns *Namespace) GetJSON(args ...any) (any, error)
// STUB L143-152: addDefaultHeaders(req *http.Request, accepts ...string)
// STUB L154-164: addUserProvidedHeaders(headers map[string]any, req *http.Request)
// STUB L166-175: hasHeaderValue(m http.Header, key, value string) bool
// STUB L177-180: hasHeaderKey(m http.Header, key string) bool
// STUB L182-196: toURLAndHeaders(urlParts []any) (string, map[string]any)
// STUB L199-209: parseCSV(c []byte, sep string) ([][]string, error)
// ---------------------------------------------------------------------------
