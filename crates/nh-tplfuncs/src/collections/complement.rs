//! Port of `tpl/collections/complement.go`.
//!
//! Owner: Wave B task T18 (tplfuncs-data).

use go_value::Value;
use nh_common::object::GoResult;

use super::collections::Namespace;
use super::reflect_helpers::{as_slice, collect_identities, indirect_interface, normalize};

impl Namespace {
    // Go: tpl/collections/complement.go:Complement
    /// Complement gives the elements in the last element of ls that are not in any of the
    /// others. All elements of ls must be slices or arrays of comparable types.
    pub fn do_complement(&self, ls: &[Value]) -> GoResult<Value> {
        if ls.len() < 2 {
            return Err(go_value::Error::new(
                "complement needs at least two arguments",
            ));
        }

        let universe = &ls[ls.len() - 1];
        let as_ = &ls[..ls.len() - 1];

        let aset = collect_identities(as_)?;

        match as_slice(universe) {
            Some(v) => {
                let mut sl = Vec::new();
                for item in &v.items {
                    let (ev, _) = indirect_interface(item);
                    if !aset.contains(&normalize(&ev)) {
                        sl.push(ev);
                    }
                }
                Ok(v.with(sl))
            }
            None => Err(go_value::Error::new(
                "arguments to complement must be slices or arrays",
            )),
        }
    }
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: tpl/collections/complement.go (57 lines; 0/1 funcs executed)
// OK L30-57: (ns *Namespace) Complement(ls ...any) (any, error)
// ---------------------------------------------------------------------------
