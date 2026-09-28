//! Port of `resources/page/pages.go`.
//!
//! Owner: Wave B task T12 (page-collections).

//! Methods of the named slice type `page.Pages` for templates (`.Reverse`, `.Related`, `.ByTitle`,
//! `.GroupBy*`, `.Len`, `.Next`, `.Prev`, `.Limit`, ...), registered in the NamedTypeRegistry.

use go_value::{HostCtx, Value};
use nh_common::object::{GoResult, NamedMethods};

use crate::page::{PageRef, Pages};

/// Go: `Pages.Len()`.
pub fn len(p: &Pages) -> i64 {
    p.len() as i64
}

pub fn pages_has_method(name: &str) -> bool {
    matches!(
        name,
        "Len"
            | "Reverse"
            | "Related"
            | "RelatedIndices"
            | "RelatedTo"
            | "ByWeight"
            | "ByTitle"
            | "ByLinkTitle"
            | "ByDate"
            | "ByPublishDate"
            | "ByExpiryDate"
            | "ByLastmod"
            | "ByLength"
            | "ByLanguage"
            | "ByParam"
            | "Limit"
            | "GroupBy"
            | "GroupByParam"
            | "GroupByDate"
            | "GroupByPublishDate"
            | "GroupByExpiryDate"
            | "GroupByLastmod"
            | "GroupByParamDate"
            | "Next"
            | "Prev"
            | "MergeByLanguage"
            | "MergeByLanguageInterface"
            | "ToResources"
            | "ProbablyEq"
            | "String"
    )
}

/// Dispatch of `page.Pages` methods (receiver: a `page.Pages` list value).
pub fn pages_call_method(
    ctx: HostCtx<'_>,
    recv: &Value,
    name: &str,
    args: &[Value],
) -> Option<GoResult<Value>> {
    todo!()
}

pub const PAGES_METHODS: NamedMethods = NamedMethods {
    has_method: pages_has_method,
    call: pages_call_method,
};

/// Go: `Pages.ProbablyEq(other)`.
pub fn probably_eq(a: &Pages, b: &Pages) -> bool {
    todo!()
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: resources/page/pages.go (140 lines; 2/7 funcs executed)
//   types: Pages, PagesFactory
//    L30-32: (ps Pages) String() string
//    L35-40: (ps Pages) shuffle()
//    L44-50: (pages Pages) ToResources() resource.Resources
// EX L53-88: ToPages(seq any) (Pages, error)
//    L92-98: (p Pages) Group(key any, in any) (any, error)
// EX L101-103: (p Pages) Len() int
//    L107-131: (pages Pages) ProbablyEq(other any) bool
// ---------------------------------------------------------------------------
