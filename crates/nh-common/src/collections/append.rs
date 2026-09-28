//! Port of `common/collections/append.go`.
//!
//! Owner: Wave B task T01 (common-values).

use std::sync::Arc;

use go_value::{List, NilKind, Value};

use crate::herrors::{Error, Result};
use crate::hreflect::{
    assign_to, elem_type, is_slice_type, slice_elem_type, slice_type_from_name, type_assignable_to,
    type_of,
};

/// A slice value as Go's `reflect.Value` sees it: its type and elements.
struct SliceVal {
    ty: String,
    items: Vec<Value>,
}

impl SliceVal {
    fn into_value(self) -> Value {
        Value::List(Arc::new(List::new(
            slice_type_from_name(&self.ty),
            self.items,
        )))
    }
}

/// `reflect.ValueOf(v)` for a slice-kind value (a `List`, a nil slice, a `Kind::Slice` object).
fn as_slice(v: &Value) -> Option<SliceVal> {
    match v {
        Value::List(l) => Some(SliceVal {
            ty: l.ty.go_name().into_owned(),
            items: l.items.clone(),
        }),
        Value::TypedNil(t) if go_value::typed_nil_kind(t) == NilKind::Slice => Some(SliceVal {
            ty: t.to_string(),
            items: Vec::new(),
        }),
        Value::Object(o) if o.kind() == go_value::Kind::Slice => Some(SliceVal {
            ty: o.type_name().into_owned(),
            items: o.list().unwrap_or_default(),
        }),
        _ => None,
    }
}

/// The element type of the slice type `t` (`[]interface {}` elements when unknown).
fn slice_elem(t: &str) -> String {
    match elem_type(t) {
        Some(e) => e,
        None => slice_elem_type(&slice_type_from_name(t)),
    }
}

// Go: reflect.Append (panics if a value is not assignable to the element type)
fn reflect_append(tov: &mut SliceVal, tot: &str, x: &Value) -> Result<()> {
    match type_of(x) {
        Some(xt) if type_assignable_to(&xt, tot) => {
            tov.items.push(assign_to(x.clone(), tot));
            Ok(())
        }
        Some(xt) => Err(Error::new(format!(
            "reflect.Set: value of type {xt} is not assignable to type {tot}"
        ))),
        None => Err(Error::new(
            "reflect: call of reflect.Value.Type on zero Value",
        )),
    }
}

// Go: common/collections/append.go:Append
/// Append appends from to a slice to and returns the resulting slice. If `to` is a nil slice
/// (or empty), `from` decides the type ([`super::slice::slice`]). A `from` that is a single
/// slice of the same element type is appended element-wise; values that are not assignable to
/// the element type turn the result into a `[]interface {}`. The input slice is never modified.
/// The result type is visible to `%T`, `jsonify` and later `range`/`sort`:
/// `[]interface{}` + map -> `[]map[string]interface {}`, `[]interface{}` + page -> `page.Pages`
/// (via the page's `Slice` method).
pub fn append(ctx: go_value::HostCtx<'_>, to: &Value, from: &[Value]) -> Result<Value> {
    if from.is_empty() {
        return Ok(to.clone());
    }
    let (tov_value, to_is_nil) = indirect(to);

    let mut to_is_nil = to_is_nil || to.is_invalid();
    let mut tot = String::new();
    let mut tov = SliceVal {
        ty: String::new(),
        items: Vec::new(),
    };

    if !to_is_nil {
        // Create a copy of tov, so we don't modify the original.
        let Some(copy) = as_slice(&tov_value) else {
            return Err(Error::new(format!(
                "expected a slice, got {}",
                to.go_type_name()
            )));
        };
        tov = copy;

        tot = slice_elem(&tov.ty);
        if is_slice_type(&tot) {
            let totvt = slice_elem(&tot);
            for f in from {
                let Some(fromt) = type_of(f) else {
                    return Err(Error::new(
                        "reflect: call of reflect.Value.Type on zero Value",
                    ));
                };
                let mut fromt = fromt.into_owned();
                if is_slice_type(&fromt) {
                    fromt = slice_elem(&fromt);
                }
                if totvt != fromt {
                    return Err(Error::new(format!(
                        "cannot append slice of {fromt} to slice of {totvt}"
                    )));
                }
            }
            for f in from {
                reflect_append(&mut tov, &tot, f)?;
            }
            return Ok(tov.into_value());
        }

        to_is_nil = tov.items.is_empty();

        if from.len() == 1 {
            let fromv = &from[0];
            let Some(fromt) = type_of(fromv) else {
                // from[0] is nil
                return Ok(append_to_interface_slice_from_values(&tov, None));
            };
            let mut fromt = fromt.into_owned();
            let from_slice = as_slice(fromv);
            if is_slice_type(&fromt) {
                fromt = slice_elem(&fromt);
            }
            if let Some(from_slice) = from_slice {
                if to_is_nil {
                    // If we get nil []string, we just return the []string
                    return Ok(fromv.clone());
                }

                // If we get []string []string, we append the from slice to to
                if tot == fromt {
                    tov.items.extend(from_slice.items);
                    return Ok(tov.into_value());
                } else if !type_assignable_to(&fromt, &tot) {
                    // Fall back to a []interface{} slice.
                    return Ok(append_to_interface_slice_from_values(
                        &tov,
                        Some(&from_slice),
                    ));
                }
            }
        }
    }

    if to_is_nil {
        return Ok(super::slice::slice(ctx, from));
    }

    for f in from {
        let assignable = type_of(f).is_some_and(|ft| type_assignable_to(&ft, &tot));
        if !assignable {
            // Fall back to a []interface{} slice.
            let (orig, _) = indirect(to);
            let orig = as_slice(&orig).unwrap_or(SliceVal {
                ty: String::new(),
                items: Vec::new(),
            });
            return Ok(append_to_interface_slice(&orig, from));
        }
        reflect_append(&mut tov, &tot, f)?;
    }

    Ok(tov.into_value())
}

// Go: common/collections/append.go:appendToInterfaceSliceFromValues
/// The elements of both slices in a `[]interface {}`; a nil (`None`) second slice contributes
/// one nil element.
fn append_to_interface_slice_from_values(slice1: &SliceVal, slice2: Option<&SliceVal>) -> Value {
    let mut tos = slice1.items.clone();
    match slice2 {
        None => tos.push(Value::Invalid),
        Some(s) => tos.extend(s.items.iter().cloned()),
    }
    Value::any_list(tos)
}

// Go: common/collections/append.go:appendToInterfaceSlice
fn append_to_interface_slice(tov: &SliceVal, from: &[Value]) -> Value {
    let mut tos = tov.items.clone();
    tos.extend(from.iter().cloned());
    Value::any_list(tos)
}

// Go: common/collections/append.go:indirect
/// indirect is borrowed from the Go stdlib: `reflect.Value.Elem` through pointers and interfaces.
/// In the value model the only pointers or interfaces that can be dereferenced are typed nils,
/// which report `isNil`; every other value is already the dereferenced value.
fn indirect(v: &Value) -> (Value, bool) {
    if let Value::TypedNil(t) = v
        && matches!(
            go_value::typed_nil_kind(t),
            NilKind::Ptr | NilKind::Interface
        )
    {
        return (v.clone(), true);
    }
    (v.clone(), false)
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: common/collections/append.go (152 lines; 2/4 funcs executed)
// OK L24-110: Append(to any, from ...any) (any, error)
// OK L112-126: appendToInterfaceSliceFromValues(slice1, slice2 reflect.Value) ([]any, error)
// OK L128-138: appendToInterfaceSlice(tov reflect.Value, from ...any) ([]any, error)
// OK L142-152: indirect(v reflect.Value) (rv reflect.Value, isNil bool)
// ---------------------------------------------------------------------------
