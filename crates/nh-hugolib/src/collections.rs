//! Port of `hugolib/collections.go`.
//!
//! Owner: Wave B task T21 (hugolib-assemble).

//! Go `hugolib/collections.go`: `pageState.Slice(items)` (collections.Slicer -> page.Pages),
//! `Group(key, pages)`.

use go_value::Value;
use nh_common::Result;
use nh_page::page::{pages_from_value, pages_to_value};

use crate::page::PageHandle;

/// Go: `(p *pageState) Slice(items)` — `page.ToPages(items)` (collections.Slicer: `slice`
/// of pages gives a `page.Pages`).
// Go: hugolib/collections.go:Slice
pub fn slice(_p: &PageHandle, items: &Value) -> Result<Value> {
    Ok(pages_to_value(&pages_from_value(items)?))
}

/// Go: `(p *pageState) Group(key, in)` — a `page.PageGroup` of the pages (collections.Grouper).
// Go: hugolib/collections.go:Group
pub fn group(_p: &PageHandle, key: &Value, input: &Value) -> Result<Value> {
    let pages = pages_from_value(input)?;
    Ok(Value::object(nh_page::pagegroup::PageGroup {
        key: key.clone(),
        pages,
    }))
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: hugolib/collections.go (46 lines; 1/2 funcs executed)
// OK L31-33: (p *pageState) Slice(items any) (any, error)
// OK L40-46: (p *pageState) Group(key any, in any) (any, error)
// ---------------------------------------------------------------------------
