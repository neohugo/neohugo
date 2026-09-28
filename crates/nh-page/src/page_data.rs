//! Port of `resources/page/page_data.go`.
//!
//! Owner: Wave B task T11 (page-api-paths).
//!
//! Go `page.Data` (`map[string]any` with a `Pages()` method; `.Data.Pages`, `.Data.Singular`,
//! `.Data.Plural`, `.Data.Term`, `.Data.<singular>`, `.Data.Terms`). Methods shadow keys.

use std::borrow::Cow;
use std::sync::Arc;

use go_value::{HostCtx, Map, MapType, Object, SliceType, Value};
use nh_common::object::{GoResult, NamedMethods};

use crate::page::{PAGES_TYPE, Pages, pages_to_value};

pub const DATA_TYPE: &str = "page.Data";

/// A new `page.Data` map value.
pub fn new_data(entries: Map) -> Value {
    Value::Map(Arc::new(Map {
        ty: MapType::Named(Arc::from(DATA_TYPE)),
        entries: entries.entries,
    }))
}

/// A `func() page.Pages` stored under the `pages` key (Go's lazy `.Data.Pages`).
#[derive(Clone)]
pub struct LazyPages(pub Arc<dyn Fn() -> Pages + Send + Sync>);

impl Object for LazyPages {
    fn type_name(&self) -> Cow<'_, str> {
        Cow::Borrowed("func() page.Pages")
    }
    fn kind(&self) -> go_value::Kind {
        go_value::Kind::Func
    }
    fn has_method(&self, _name: &str) -> bool {
        false
    }
    fn call_method(
        &self,
        _ctx: HostCtx<'_>,
        _name: &str,
        _args: &[Value],
    ) -> Option<GoResult<Value>> {
        None
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

/// Go: `Data.Pages()` — the `pages` key as `page.Pages` (missing -> a nil Pages; a
/// `func() Pages` is invoked). Any other value is Go's panic `%T is not Pages`, returned as an
/// error.
// Go: resources/page/page_data.go:Pages
pub fn try_data_pages(d: &Map) -> GoResult<Value> {
    let Some(v) = d.get(b"pages") else {
        return Ok(Value::TypedNil(Arc::from(PAGES_TYPE)));
    };

    match v {
        Value::List(l) if matches!(&l.ty, SliceType::Named(n) if &**n == PAGES_TYPE) => {
            Ok(v.clone())
        }
        Value::TypedNil(t) if &**t == PAGES_TYPE => Ok(v.clone()),
        Value::Object(o) if o.as_any().downcast_ref::<LazyPages>().is_some() => {
            let f = o.as_any().downcast_ref::<LazyPages>().expect("checked");
            Ok(pages_to_value(&(f.0)()))
        }
        _ => Err(go_value::Error::new(format!(
            "{} is not Pages",
            match v {
                Value::Invalid => Cow::Borrowed("<nil>"),
                _ => v.go_type_name(),
            }
        ))),
    }
}

/// [`try_data_pages`], panicking like Go on a `pages` value that is not Pages.
// Go: resources/page/page_data.go:Pages
pub fn data_pages(d: &Map) -> Value {
    match try_data_pages(d) {
        Ok(v) => v,
        Err(e) => panic!("{}", e.message()),
    }
}

pub fn data_has_method(name: &str) -> bool {
    name == "Pages"
}

pub fn data_call_method(
    _ctx: HostCtx<'_>,
    recv: &Value,
    name: &str,
    args: &[Value],
) -> Option<GoResult<Value>> {
    match (name, recv) {
        ("Pages", Value::Map(m)) => {
            Some(nh_common::object::args::exactly(args, 0, name).and_then(|_| try_data_pages(m)))
        }
        ("Pages", Value::TypedNil(_)) => Some(
            nh_common::object::args::exactly(args, 0, name)
                .map(|_| Value::TypedNil(Arc::from(PAGES_TYPE))),
        ),
        _ => None,
    }
}

pub const DATA_METHODS: NamedMethods = NamedMethods {
    has_method: data_has_method,
    call: data_call_method,
};

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: resources/page/page_data.go (42 lines; 1/1 funcs executed)
//   types: Data
// OK L28-42: (d Data) Pages() Pages
// ---------------------------------------------------------------------------
