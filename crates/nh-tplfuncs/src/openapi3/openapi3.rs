//! Port of `tpl/openapi/openapi3/openapi3.go`.
//!
//! Owner: Wave B task T19 (tplfuncs-host).
//!
//! STUB: the OpenAPI document model and loader (`getkin/kin-openapi`, whose `ResolveRefsIn`
//! may fetch remote references) are not ported. `Unmarshal` checks its argument like Go (the
//! resource, its key and its media type) and then returns an explicit `neohugo-rs` error.

use std::sync::Arc;

use go_value::{HostCtx, Object, Value};
use nh_common::object::{GoResult, args};
use nh_deps::deps::Deps;
use nh_parser::metadecoders::format::{Format, format_from_strings};

/// Go: `openapi3.Namespace` (template value `*openapi3.Namespace`).
pub struct Namespace {
    pub d: Arc<Deps>,
}

fn gerr(msg: impl Into<String>) -> go_value::Error {
    go_value::Error::new(msg)
}

impl Namespace {
    /// New returns a new instance of the openapi3-namespaced template functions.
    // Go: tpl/openapi/openapi3/openapi3.go:New
    pub fn new(d: Arc<Deps>) -> Namespace {
        Namespace { d }
    }

    /// Unmarshal unmarshals the given resource into an OpenAPI 3 document. STUB (see the module
    /// docs).
    // Go: tpl/openapi/openapi3/openapi3.go:Unmarshal
    pub fn unmarshal(&self, _ctx: HostCtx<'_>, a: &[Value]) -> GoResult<Value> {
        args::exactly(a, 1, "Unmarshal")?;
        let r = match &a[0] {
            Value::Invalid => {
                return Err(gerr(
                    "runtime error: invalid memory address or nil pointer dereference",
                ));
            }
            v => nh_resource::resourcetypes::resource_from_value_any(v)
                .filter(|r| r.read_seek_closer().is_some())
                .ok_or_else(|| args::wrong_type("resource.UnmarshableResource", v))?,
        };
        let key = r.key();
        if key.is_empty() {
            return Err(gerr("no Key set in Resource"));
        }

        let mt = r.media_type();
        let suffixes = mt.suffixes();
        let suffixes: Vec<&str> = suffixes.iter().map(|s| s.as_str()).collect();
        if format_from_strings(&suffixes) == Format::Unknown {
            return Err(gerr(format!(
                "MIME {} not supported",
                go_strconv::quote(mt.typ.as_bytes())
            )));
        }

        Err(gerr(
            "neohugo-rs: openapi3.Unmarshal (kin-openapi) is not supported",
        ))
    }
}

nh_common::go_methods!(Namespace {
    "Unmarshal" => |n, ctx, a| n.unmarshal(ctx, a),
});

impl Object for Namespace {
    nh_common::object_basics!("*openapi3.Namespace");
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: tpl/openapi/openapi3/openapi3.go (100 lines; 1/3 funcs executed)
//   types: Namespace, OpenAPIDocument
// OK L33-38: New(deps *deps.Deps) *Namespace
// STUB L52-54: (o *OpenAPIDocument) GetIdentityGroup() identity.Identity
// STUB L57-100: (ns *Namespace) Unmarshal(r resource.UnmarshableResource) (*OpenAPIDocument, error)
// ---------------------------------------------------------------------------
