//! Module `tplapi::objects`.
//!
//! NEW: template values of the types page methods return that no lower crate models as a
//! template object: `*tableofcontents.Fragments` (`.Fragments`), `*tableofcontents.Heading`,
//! `tableofcontents.Headings` and `collections.SortedStringSlice` (`.Fragments.Identifiers`).
//!
//! Owner: Wave B task T23 (hugolib-site).

use std::borrow::Cow;
use std::sync::Arc;

use go_value::{HostCtx, List, Map, MapType, Object, SliceType, Value};
use nh_common::object::{GoResult, NamedMethods, args};
use nh_markup::tableofcontents::{Fragments, Heading, Headings};

use super::values::{arity, nil_of};

/// Go type string of `tableofcontents.Headings`.
pub const HEADINGS_TYPE: &str = "tableofcontents.Headings";
/// Go type string of `collections.SortedStringSlice`.
pub const SORTED_STRING_SLICE_TYPE: &str = "collections.SortedStringSlice";

/// `*tableofcontents.Fragments` (a nil pointer is `nil_of("*tableofcontents.Fragments")`).
pub struct FragmentsObject(pub Arc<Fragments>);

/// A `.Fragments` result.
pub fn fragments_value(f: Option<Arc<Fragments>>) -> Value {
    match f {
        Some(f) => Value::object(FragmentsObject(f)),
        None => nil_of("*tableofcontents.Fragments"),
    }
}

nh_common::go_methods!(FragmentsObject {
    // Go: markup/tableofcontents/tableofcontents.go:ToHTML
    "ToHTML" => |f, _c, a| {
        arity(a, 3, false, "ToHTML")?;
        let ordered = args::bool(a, 2)?;
        let b = f.0.to_html_values(&a[0], &a[1], ordered)?;
        Ok(Value::html(b))
    },
});

impl Object for FragmentsObject {
    nh_common::object_basics!("*tableofcontents.Fragments");

    fn field(&self, name: &str) -> Option<Value> {
        match name {
            "Headings" => Some(headings_value(&self.0.headings)),
            "Identifiers" => Some(if self.0.identifiers.is_empty() {
                nil_of(SORTED_STRING_SLICE_TYPE)
            } else {
                Value::list(
                    SliceType::Named(Arc::from(SORTED_STRING_SLICE_TYPE)),
                    self.0
                        .identifiers
                        .iter()
                        .map(|s| Value::String(s.clone()))
                        .collect(),
                )
            }),
            "HeadingsMap" => {
                let mut m = Map::new(MapType::Named(Arc::from(
                    "map[string]*tableofcontents.Heading",
                )));
                for (k, v) in &self.0.headings_map {
                    m.insert(k.clone(), Value::object(HeadingObject(v.clone())));
                }
                Some(Value::map(m))
            }
            _ => None,
        }
    }

    fn identity(&self) -> usize {
        Arc::as_ptr(&self.0) as usize
    }
}

/// `*tableofcontents.Heading`.
pub struct HeadingObject(pub Arc<Heading>);

nh_common::go_methods!(HeadingObject {
    // Go: markup/tableofcontents/tableofcontents.go:IsZero
    "IsZero" => |h, _c, a| {
        arity(a, 0, false, "IsZero")?;
        Ok(Value::Bool(h.0.id.is_empty() && h.0.title.is_empty()))
    },
});

impl Object for HeadingObject {
    nh_common::object_basics!("*tableofcontents.Heading");

    fn field(&self, name: &str) -> Option<Value> {
        match name {
            "ID" => Some(Value::String(self.0.id.clone())),
            "Level" => Some(Value::int(self.0.level)),
            "Title" => Some(Value::String(self.0.title.clone())),
            "Headings" => Some(headings_value(&self.0.headings)),
            _ => None,
        }
    }

    fn is_zero(&self) -> Option<bool> {
        Some(self.0.id.is_empty() && self.0.title.is_empty())
    }

    fn identity(&self) -> usize {
        Arc::as_ptr(&self.0) as usize
    }
}

/// `tableofcontents.Headings` (nil when empty: Go builds it by appending).
pub fn headings_value(h: &Headings) -> Value {
    if h.is_empty() {
        return nil_of(HEADINGS_TYPE);
    }
    Value::List(Arc::new(List::new(
        SliceType::Named(Arc::from(HEADINGS_TYPE)),
        h.iter()
            .map(|x| Value::object(HeadingObject(x.clone())))
            .collect(),
    )))
}

fn headings_has_method(name: &str) -> bool {
    name == "FilterBy"
}

fn headings_call(
    _ctx: HostCtx<'_>,
    _recv: &Value,
    name: &str,
    a: &[Value],
) -> Option<GoResult<Value>> {
    match name {
        // Go `FilterBy(fn func(*Heading) bool)`: templates cannot pass a Go func.
        "FilterBy" => Some(arity(a, 1, false, name).and_then(|_| {
            Err(nh_common::object::args::wrong_type(
                "func(*tableofcontents.Heading) bool",
                &a[0],
            ))
        })),
        _ => None,
    }
}

/// The methods of `tableofcontents.Headings`.
pub const HEADINGS_METHODS: NamedMethods = NamedMethods {
    has_method: headings_has_method,
    call: headings_call,
};

fn sss_has_method(name: &str) -> bool {
    matches!(name, "Contains" | "Count")
}

fn sss_call(_ctx: HostCtx<'_>, recv: &Value, name: &str, a: &[Value]) -> Option<GoResult<Value>> {
    if !sss_has_method(name) {
        return None;
    }
    let items: Vec<go_value::GoString> = match recv {
        Value::List(l) => l
            .items
            .iter()
            .filter_map(|v| match v {
                Value::String(s) => Some(s.clone()),
                _ => None,
            })
            .collect(),
        _ => Vec::new(),
    };
    let ss = nh_common::collections::slice::SortedStringSlice(items);
    Some((|| {
        arity(a, 1, false, name)?;
        let s = args::string(a, 0)?;
        Ok(match name {
            "Contains" => Value::Bool(ss.contains(s.as_bytes())),
            _ => Value::int(ss.count(s.as_bytes()) as i64),
        })
    })())
}

/// The methods of `collections.SortedStringSlice`.
pub const SORTED_STRING_SLICE_METHODS: NamedMethods = NamedMethods {
    has_method: sss_has_method,
    call: sss_call,
};

/// The type name of an object for error messages.
pub fn type_name_of(v: &Value) -> Cow<'_, str> {
    match v {
        Value::Invalid => Cow::Borrowed("<nil>"),
        _ => v.go_type_name(),
    }
}
