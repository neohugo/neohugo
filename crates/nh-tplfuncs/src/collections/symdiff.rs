//! Port of `tpl/collections/symdiff.go`.
//!
//! Owner: Wave B task T18 (tplfuncs-data).

use go_value::Value;
use nh_common::object::GoResult;

use super::collections::Namespace;
use super::reflect_helpers::{
    SliceVal, as_slice, collect_identities, convert_value, indirect_interface, normalize,
};

impl Namespace {
    // Go: tpl/collections/symdiff.go:SymDiff
    /// SymDiff returns the symmetric difference of s1 and s2. Arguments must be either a slice
    /// or an array of comparable types.
    pub fn do_sym_diff(&self, s2: &Value, s1: &Value) -> GoResult<Value> {
        let ids1 = collect_identities(std::slice::from_ref(s1))?;
        let ids2 = collect_identities(std::slice::from_ref(s2))?;

        let mut slice: Option<SliceVal> = None;
        let mut out: Vec<Value> = Vec::new();
        let mut slice_elem_type = String::new();

        for (i, s) in [s1, s2].into_iter().enumerate() {
            match as_slice(s) {
                Some(v) => {
                    if i == 0 {
                        slice_elem_type = v.elem_type();
                        slice = Some(SliceVal {
                            ty: v.ty.clone(),
                            items: Vec::new(),
                            nil: false,
                        });
                    }

                    for item in &v.items {
                        let (ev, _) = indirect_interface(item);
                        let key = normalize(&ev);

                        // Append if the key is not in their intersection.
                        if ids1.contains(&key) != ids2.contains(&key) {
                            let v = convert_value(&ev, &slice_elem_type).map_err(|e| {
                                go_value::Error::new(format!(
                                    "symdiff: failed to convert value: {}",
                                    e.message()
                                ))
                            })?;
                            out.push(v);
                        }
                    }
                }
                None => {
                    return Err(go_value::Error::new(
                        "arguments to symdiff must be slices or arrays",
                    ));
                }
            }
        }

        Ok(slice.expect("s1 is a slice").with(out))
    }
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: tpl/collections/symdiff.go (66 lines; 0/1 funcs executed)
// OK L23-66: (ns *Namespace) SymDiff(s2, s1 any) (any, error)
// ---------------------------------------------------------------------------
