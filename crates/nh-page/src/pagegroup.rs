//! Port of `resources/page/pagegroup.go`.
//!
//! Owner: Wave B task T12 (page-collections).


use go_value::Value;
use nh_common::Result;

use crate::page::Pages;

/// Go: `page.PageGroup` (`.Key`, `.Pages`).
#[derive(Clone)]
pub struct PageGroup {
    pub key: Value,
    pub pages: Pages,
}

/// Go: `page.PagesGroup`.
pub type PagesGroup = Vec<PageGroup>;

/// Go: `Pages.GroupBy(ctx, key, order...)` etc.
// Go: resources/page/pagegroup.go:GroupBy
pub fn group_by(p: &Pages, key: &str, order: &[String]) -> Result<PagesGroup> {
    todo!()
}

/// Go: `page.ToPagesGroup(seq)`.
// Go: resources/page/pagegroup.go:ToPagesGroup
pub fn to_pages_group(v: &Value) -> Result<Option<PagesGroup>> {
    todo!()
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: resources/page/pagegroup.go (461 lines; 1/19 funcs executed)
//   types: PageGroup, mapKeyValues, mapKeyByInt, mapKeyByStr, PagesGroup
//    L53-53: (v mapKeyValues) Len() int
//    L54-54: (v mapKeyValues) Swap(i, j int)
//    L58-58: (s mapKeyByInt) Less(i, j int) bool
//    L65-67: (s mapKeyByStr) Less(i, j int) bool
//    L69-91: sortKeys(examplePage Page, v []reflect.Value, order string) []reflect.Value
//    L98-104: (p PagesGroup) Reverse() PagesGroup
//    L114-180: (p Pages) GroupBy(ctx context.Context, key string, order ...string) (PagesGroup, error)
//    L184-230: (p Pages) GroupByParam(key string, order ...string) (PagesGroup, error)
//    L232-270: (p Pages) groupByDateField(format string, sorter func(p Pages) Pages, getDate func(p Page) time.Time, order ...string) (PagesGroup, error)
//    L276-284: (p Pages) GroupByDate(format string, order ...string) (PagesGroup, error)
//    L290-298: (p Pages) GroupByPublishDate(format string, order ...string) (PagesGroup, error)
//    L304-312: (p Pages) GroupByExpiryDate(format string, order ...string) (PagesGroup, error)
//    L318-326: (p Pages) GroupByLastmod(format string, order ...string) (PagesGroup, error)
//    L332-365: (p Pages) GroupByParamDate(key string, format string, order ...string) (PagesGroup, error)
//    L369-380: (p PageGroup) ProbablyEq(other any) bool
//    L384-401: (p PageGroup) Slice(in any) (any, error)
//    L404-410: (psg PagesGroup) Len() int
//    L413-430: (psg PagesGroup) ProbablyEq(other any) bool
// EX L433-461: ToPagesGroup(seq any) (PagesGroup, bool, error)
// ---------------------------------------------------------------------------
