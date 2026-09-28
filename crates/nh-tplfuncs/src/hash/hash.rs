//! Port of `tpl/hash/hash.go`.
//!
//! Owner: Wave B task T18 (tplfuncs-data).

use std::sync::Arc;

use go_value::{HostCtx, Object, Value};
use nh_common::cast::caste;
use nh_common::object::{GoResult, args};
use nh_deps::deps::Deps;

/// Go: `hash.Namespace` (template value `*hash.Namespace`).
pub struct Namespace {
    pub d: Arc<Deps>,
}

impl Namespace {
    // Go: tpl/hash:New
    pub fn new(d: Arc<Deps>) -> Namespace {
        Namespace { d }
    }

    // Go: tpl/hash:FNV32a
    pub fn fnv32a(&self, _ctx: HostCtx<'_>, a: &[Value]) -> GoResult<Value> {
        args::exactly(a, 1, "FNV32a")?;
        // Go: tpl/hash/hash.go:FNV32a
        let conv = caste::to_string_e(&a[0])?;
        Ok(Value::int(crate::crypto::crypto::fnv32a(&conv) as i64))
    }

    // Go: tpl/hash:XxHash
    pub fn xx_hash(&self, _ctx: HostCtx<'_>, a: &[Value]) -> GoResult<Value> {
        args::exactly(a, 1, "XxHash")?;
        // Go: tpl/hash/hash.go:XxHash
        let conv = caste::to_string_e(&a[0])?;
        Ok(Value::string(
            nh_common::hashing::xxhash_from_string_hex_encoded(&conv),
        ))
    }
}

nh_common::go_methods!(Namespace {
    "FNV32a" => |n, ctx, a| n.fnv32a(ctx, a),
    "XxHash" => |n, ctx, a| n.xx_hash(ctx, a),
});

impl Object for Namespace {
    nh_common::object_basics!("*hash.Namespace");
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: tpl/hash/hash.go (85 lines; 2/4 funcs executed)
//   types: Namespace
// OK L28-30: New() *Namespace
// OK L36-44: (ns *Namespace) FNV32a(v any) (int, error)
// OK L47-54: (ns *Namespace) XxHash(v any) (string, error)
// OK L58-85: init() (hash/init.rs)
// ---------------------------------------------------------------------------
