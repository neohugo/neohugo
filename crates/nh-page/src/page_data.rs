//! Port of `resources/page/page_data.go`.
//!
//! Owner: Wave B task T11 (page-api-paths).


//! Go `page.Data` (`map[string]any` with a `Pages()` method; `.Data.Pages`, `.Data.Singular`,
//! `.Data.Plural`, `.Data.Term`, `.Data.<singular>`, `.Data.Terms`). Methods shadow keys.

use std::sync::Arc;

use go_value::{HostCtx, Map, MapType, Value};
use nh_common::object::{GoResult, NamedMethods};

pub const DATA_TYPE: &str = "page.Data";

/// A new `page.Data` map value.
pub fn new_data(entries: Map) -> Value {
    Value::Map(Arc::new(Map { ty: MapType::Named(Arc::from(DATA_TYPE)), entries: entries.entries }))
}

/// Go: `Data.Pages()` — the `pages` key as `page.Pages` (nil -> empty Pages).
// Go: resources/page/page_data.go:Pages
pub fn data_pages(d: &Map) -> Value {
    todo!()
}

pub fn data_has_method(name: &str) -> bool {
    name == "Pages"
}

pub fn data_call_method(_ctx: HostCtx<'_>, recv: &Value, name: &str, _args: &[Value]) -> Option<GoResult<Value>> {
    match (name, recv) {
        ("Pages", Value::Map(m)) => Some(Ok(data_pages(m))),
        _ => None,
    }
}

pub const DATA_METHODS: NamedMethods = NamedMethods { has_method: data_has_method, call: data_call_method };

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: resources/page/page_data.go (42 lines; 1/1 funcs executed)
//   types: Data
// EX L28-42: (d Data) Pages() Pages
// ---------------------------------------------------------------------------
