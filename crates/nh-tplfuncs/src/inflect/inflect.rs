//! Port of `tpl/inflect/inflect.go`.
//!
//! Owner: Wave B task T19 (tplfuncs-host).

use std::sync::Arc;

use go_value::{GoString, HostCtx, IntKind, Object, Value};
use nh_common::cast::caste;
use nh_common::flect;
use nh_common::object::{GoResult, args};
use nh_deps::deps::Deps;

// Parity notes: `humanize` via flect (ordinalize ints / numeric strings; drops Thai combining marks).

/// Go: `inflect.Namespace` (template value `*inflect.Namespace`).
pub struct Namespace {
    pub d: Arc<Deps>,
}

fn sv(b: impl Into<GoString>) -> Value {
    Value::String(b.into())
}

impl Namespace {
    /// New returns a new instance of the inflect-namespaced template functions (Go's `New()`
    /// takes no deps; the skeleton keeps them).
    // Go: tpl/inflect/inflect.go:New
    pub fn new(d: Arc<Deps>) -> Namespace {
        Namespace { d }
    }

    /// Humanize returns the humanized form of v.
    ///
    /// If v is either an integer or a string containing an integer value, the behavior is to
    /// add the appropriate ordinal.
    // Go: tpl/inflect/inflect.go:Humanize
    pub fn humanize(&self, _ctx: HostCtx<'_>, a: &[Value]) -> GoResult<Value> {
        args::exactly(a, 1, "Humanize")?;
        let v = &a[0];
        let word = caste::to_string_e(v)?;

        if word.is_empty() {
            return Ok(sv(""));
        }

        let ok = matches!(v, Value::Int(_, IntKind::Int)); // original param was literal int value
        let is_int = go_strconv::atoi(word.as_bytes()).is_ok(); // original param was string containing an int value
        if ok || is_int {
            return Ok(sv(flect::ordinalize_bytes(word.as_bytes())));
        }

        let s = flect::try_humanize_bytes(word.as_bytes())?;
        let lower = go_unicode::strings::to_lower(&s).into_owned();
        Ok(sv(flect::try_humanize_bytes(&lower)?))
    }

    /// Pluralize returns the plural form of the single word in v.
    // Go: tpl/inflect/inflect.go:Pluralize
    pub fn pluralize(&self, _ctx: HostCtx<'_>, a: &[Value]) -> GoResult<Value> {
        args::exactly(a, 1, "Pluralize")?;
        let word = caste::to_string_e(&a[0])?;
        Ok(sv(flect::pluralize_bytes(word.as_bytes())))
    }

    /// Singularize returns the singular form of a single word in v.
    // Go: tpl/inflect/inflect.go:Singularize
    pub fn singularize(&self, _ctx: HostCtx<'_>, a: &[Value]) -> GoResult<Value> {
        args::exactly(a, 1, "Singularize")?;
        let word = caste::to_string_e(&a[0])?;
        Ok(sv(flect::singularize_bytes(word.as_bytes())))
    }
}

nh_common::go_methods!(Namespace {
    "Humanize" => |n, ctx, a| n.humanize(ctx, a),
    "Pluralize" => |n, ctx, a| n.pluralize(ctx, a),
    "Singularize" => |n, ctx, a| n.singularize(ctx, a),
});

impl Object for Namespace {
    nh_common::object_basics!("*inflect.Namespace");
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: tpl/inflect/inflect.go (75 lines; 2/4 funcs executed)
//   types: Namespace
// OK L26-28: New() *Namespace
// OK L37-55: (ns *Namespace) Humanize(v any) (string, error)
// OK L58-65: (ns *Namespace) Pluralize(v any) (string, error)
// OK L68-75: (ns *Namespace) Singularize(v any) (string, error)
// ---------------------------------------------------------------------------
