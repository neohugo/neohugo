//! Port of `resources/page/taxonomy.go`.
//!
//! Owner: Wave B task T12 (page-collections).

//! Go `page.TaxonomyList` (`map[string]Taxonomy`), `page.Taxonomy` (`map[string]WeightedPages`),
//! `OrderedTaxonomy`. Template values are named maps; `range` iterates keys in byte order.

use std::collections::BTreeMap;
use std::sync::Arc;

use go_value::{HostCtx, Value};
use nh_common::object::{GoResult, NamedMethods};

use crate::page::PageRef;
use crate::weighted::WeightedPages;

pub const TAXONOMY_LIST_TYPE: &str = "page.TaxonomyList";
pub const TAXONOMY_TYPE: &str = "page.Taxonomy";

/// Go: `page.Taxonomy` — term key (`strings.ToLower(m.term)`) -> weighted pages.
pub type Taxonomy = BTreeMap<String, WeightedPages>;

/// Go: `page.TaxonomyList` — plural -> taxonomy.
pub type TaxonomyList = Arc<BTreeMap<String, Taxonomy>>;

/// Go: `page.OrderedTaxonomyEntry`.
#[derive(Clone)]
pub struct OrderedTaxonomyEntry {
    pub name: String,
    pub weighted_pages: WeightedPages,
}

/// Go: `page.OrderedTaxonomy`.
pub type OrderedTaxonomy = Vec<OrderedTaxonomyEntry>;

/// Go: `Taxonomy.Get(key)` — lower-cases the key.
// Go: resources/page/taxonomy.go:Get
pub fn taxonomy_get(t: &Taxonomy, key: &str) -> Option<WeightedPages> {
    todo!()
}

/// Go: `Taxonomy.Alphabetical()` (collator) / `ByCount()` (count desc, then `compare.LessStrings`).
// Go: resources/page/taxonomy.go:Alphabetical
pub fn alphabetical(t: &Taxonomy) -> OrderedTaxonomy {
    todo!()
}

// Go: resources/page/taxonomy.go:ByCount
pub fn by_count(t: &Taxonomy) -> OrderedTaxonomy {
    todo!()
}

/// Template values.
pub fn taxonomy_list_to_value(tl: &TaxonomyList) -> Value {
    todo!("Map Named(page.TaxonomyList) of Map Named(page.Taxonomy) of WeightedPages lists")
}

pub fn taxonomy_has_method(name: &str) -> bool {
    matches!(
        name,
        "Get" | "Count" | "TaxonomyArray" | "Alphabetical" | "ByCount" | "Page"
    )
}

pub fn taxonomy_call_method(
    ctx: HostCtx<'_>,
    recv: &Value,
    name: &str,
    args: &[Value],
) -> Option<GoResult<Value>> {
    todo!()
}

pub const TAXONOMY_METHODS: NamedMethods = NamedMethods {
    has_method: taxonomy_has_method,
    call: taxonomy_call_method,
};

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: resources/page/taxonomy.go (177 lines; 1/16 funcs executed)
//   types: TaxonomyList, Taxonomy, OrderedTaxonomy, OrderedTaxonomyEntry, orderedTaxonomySorter, oiBy
//    L29-31: (tl TaxonomyList) String() string
//    L46-51: (t OrderedTaxonomy) getOneOPage() Page
// EX L61-63: (i Taxonomy) Get(key string) WeightedPages
//    L66-66: (i Taxonomy) Count(key string) int
//    L69-77: (i Taxonomy) TaxonomyArray() OrderedTaxonomy
//    L80-95: (i Taxonomy) Alphabetical() OrderedTaxonomy
//    L99-113: (i Taxonomy) ByCount() OrderedTaxonomy
//    L116-121: (i Taxonomy) Page() Page
//    L124-126: (ie OrderedTaxonomyEntry) Pages() Pages
//    L129-131: (ie OrderedTaxonomyEntry) Count() int
//    L134-136: (ie OrderedTaxonomyEntry) Term() string
//    L139-145: (t OrderedTaxonomy) Reverse() OrderedTaxonomy
//    L156-162: (by oiBy) Sort(taxonomy OrderedTaxonomy)
//    L165-167: (s *orderedTaxonomySorter) Len() int
//    L170-172: (s *orderedTaxonomySorter) Swap(i, j int)
//    L175-177: (s *orderedTaxonomySorter) Less(i, j int) bool
// ---------------------------------------------------------------------------
