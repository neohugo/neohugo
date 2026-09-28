//! Port of `resources/page/weighted.go`.
//!
//! Owner: Wave B task T12 (page-collections).

use std::any::Any;
use std::borrow::Cow;
use std::sync::Arc;

use go_value::{HostCtx, Object, SliceType, Value};
use nh_common::object::{GoResult, NamedMethods};

use crate::page::{PageRef, Pages};

pub const WEIGHTED_PAGES_TYPE: &str = "page.WeightedPages";

/// Go: `page.WeightedPage` (`Weight` field, embedded `Page` -> page methods are promoted, `owner`).
#[derive(Clone)]
pub struct WeightedPage {
    pub weight: i64,
    pub page: PageRef,
    pub owner: Option<PageRef>,
}

impl WeightedPage {
    // Go: resources/page/weighted.go:NewWeightedPage
    pub fn new(weight: i64, p: PageRef, owner: Option<PageRef>) -> Self {
        WeightedPage {
            weight,
            page: p,
            owner,
        }
    }
}

impl Object for WeightedPage {
    fn type_name(&self) -> Cow<'_, str> {
        Cow::Borrowed("page.WeightedPage")
    }
    fn kind(&self) -> go_value::Kind {
        go_value::Kind::Struct
    }
    /// Promoted methods of the embedded Page, plus `Slice`, `String`.
    fn has_method(&self, name: &str) -> bool {
        name == "Slice" || self.page.has_method(name)
    }
    fn call_method(&self, ctx: HostCtx<'_>, name: &str, args: &[Value]) -> Option<GoResult<Value>> {
        self.page.call_method(ctx, name, args)
    }
    fn field(&self, name: &str) -> Option<Value> {
        match name {
            "Weight" => Some(Value::int(self.weight)),
            "Page" => Some(self.page.to_value()),
            _ => None,
        }
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// Go: `page.WeightedPages`.
pub type WeightedPages = Vec<WeightedPage>;

/// Go: `WeightedPages.Pages()`.
// Go: resources/page/weighted.go:Pages
pub fn weighted_pages_pages(wp: &WeightedPages) -> Pages {
    wp.iter().map(|w| w.page.clone()).collect()
}

/// Go: `WeightedPages.Sort()` — `sort.Stable` by (weight, then DefaultPageSort).
// Go: resources/page/weighted.go:Sort
pub fn sort(wp: &mut WeightedPages) {
    todo!()
}

pub fn weighted_pages_to_value(wp: &WeightedPages) -> Value {
    Value::list(
        SliceType::Named(Arc::from(WEIGHTED_PAGES_TYPE)),
        wp.iter().map(|w| Value::object(w.clone())).collect(),
    )
}

pub fn weighted_pages_has_method(name: &str) -> bool {
    matches!(
        name,
        "Page" | "Pages" | "Next" | "Prev" | "Len" | "Count" | "Sort"
    )
}

pub fn weighted_pages_call_method(
    ctx: HostCtx<'_>,
    recv: &Value,
    name: &str,
    args: &[Value],
) -> Option<GoResult<Value>> {
    todo!()
}

pub const WEIGHTED_PAGES_METHODS: NamedMethods = NamedMethods {
    has_method: weighted_pages_has_method,
    call: weighted_pages_call_method,
};

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: resources/page/weighted.go (138 lines; 5/12 funcs executed)
//   types: WeightedPages, WeightedPage
//    L31-44: (p WeightedPages) Page() Page
// EX L58-60: NewWeightedPage(weight int, p Page, owner Page) WeightedPage
//    L62-64: (w WeightedPage) String() string
//    L68-85: (p WeightedPage) Slice(in any) (any, error)
//    L88-94: (wp WeightedPages) Pages() Pages
//    L98-108: (wp WeightedPages) Next(cur Page) Page
//    L112-122: (wp WeightedPages) Prev(cur Page) Page
// EX L124-124: (wp WeightedPages) Len() int
// EX L125-125: (wp WeightedPages) Swap(i, j int)
// EX L128-128: (wp WeightedPages) Sort()
//    L131-131: (wp WeightedPages) Count() int
// EX L133-138: (wp WeightedPages) Less(i, j int) bool
// ---------------------------------------------------------------------------
