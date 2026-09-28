//! Port of `resources/page/taxonomy.go`.
//!
//! Owner: Wave B task T12 (page-collections).

//! Go `page.TaxonomyList` (`map[string]Taxonomy`), `page.Taxonomy` (`map[string]WeightedPages`),
//! `OrderedTaxonomy`. Template values are named maps; `range` iterates keys in byte order.
//!
//! Go iterates the `Taxonomy` map in random order in `TaxonomyArray` (and so in `Alphabetical`
//! and `ByCount`, which sort that array with `sort.Stable`) and in `Page`. The port iterates in
//! key order: `ByCount` sorts by a total order, so only `TaxonomyArray` itself and the order of
//! names the collator ranks equal in `Alphabetical` differ from a given Go run (both are random in
//! Go).

use std::any::Any;
use std::borrow::Cow;
use std::collections::BTreeMap;
use std::sync::Arc;

use go_value::{GoString, HostCtx, Map, MapType, Object, SliceType, Value};
use nh_common::compare;
use nh_common::object::{GoResult, NamedMethods, args};

use crate::page::{PageRef, Pages};
use crate::weighted::{
    WEIGHTED_PAGES_TYPE, WeightedPages, page_opt_value, weighted_pages_call_method,
    weighted_pages_from_value, weighted_pages_has_method, weighted_pages_page,
    weighted_pages_pages, weighted_pages_to_value,
};

pub const TAXONOMY_LIST_TYPE: &str = "page.TaxonomyList";
pub const TAXONOMY_TYPE: &str = "page.Taxonomy";
/// Go type string of `page.OrderedTaxonomy`.
pub const ORDERED_TAXONOMY_TYPE: &str = "page.OrderedTaxonomy";
/// Go type string of `page.OrderedTaxonomyEntry` (a struct value).
pub const ORDERED_TAXONOMY_ENTRY_TYPE: &str = "page.OrderedTaxonomyEntry";

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

impl OrderedTaxonomyEntry {
    /// Go: `ie.Pages()`.
    // Go: resources/page/taxonomy.go:Pages
    pub fn pages(&self) -> Pages {
        weighted_pages_pages(&self.weighted_pages)
    }

    /// Go: `ie.Count()`.
    // Go: resources/page/taxonomy.go:Count
    pub fn count(&self) -> i64 {
        self.weighted_pages.len() as i64
    }

    /// Go: `ie.Term()`.
    // Go: resources/page/taxonomy.go:Term
    pub fn term(&self) -> String {
        self.name.clone()
    }
}

nh_common::go_methods!(OrderedTaxonomyEntry {
    "Pages" => |e, _c, a| {
        args::exactly(a, 0, "Pages")?;
        Ok(crate::page::pages_to_value(&e.pages()))
    },
    "Count" => |e, _c, a| {
        args::exactly(a, 0, "Count")?;
        Ok(Value::int(e.count()))
    },
    "Term" => |e, _c, a| {
        args::exactly(a, 0, "Term")?;
        Ok(Value::string(e.term()))
    },
});

impl Object for OrderedTaxonomyEntry {
    fn type_name(&self) -> Cow<'_, str> {
        Cow::Borrowed(ORDERED_TAXONOMY_ENTRY_TYPE)
    }
    fn kind(&self) -> go_value::Kind {
        go_value::Kind::Struct
    }
    /// Its own methods, then the promoted `WeightedPages` methods (`Pages` and `Count` are
    /// shadowed by the entry's own).
    fn has_method(&self, name: &str) -> bool {
        Self::go_has_method(name) || weighted_pages_has_method(name)
    }
    fn call_method(&self, ctx: HostCtx<'_>, name: &str, args: &[Value]) -> Option<GoResult<Value>> {
        if let Some(r) = self.go_call_method(ctx, name, args) {
            return Some(r);
        }
        weighted_pages_call_method(
            ctx,
            &weighted_pages_to_value(&self.weighted_pages),
            name,
            args,
        )
    }
    fn field(&self, name: &str) -> Option<Value> {
        match name {
            "Name" => Some(Value::string(self.name.clone())),
            "WeightedPages" => Some(weighted_pages_to_value(&self.weighted_pages)),
            _ => None,
        }
    }
    fn struct_fields(&self) -> Option<Vec<(Cow<'_, str>, Value)>> {
        Some(vec![
            (Cow::Borrowed("Name"), Value::string(self.name.clone())),
            (
                Cow::Borrowed("WeightedPages"),
                weighted_pages_to_value(&self.weighted_pages),
            ),
        ])
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// Go: `page.OrderedTaxonomy`.
pub type OrderedTaxonomy = Vec<OrderedTaxonomyEntry>;

/// Go: `TaxonomyList.String()`.
// Go: resources/page/taxonomy.go:String
pub fn taxonomy_list_string(tl: &TaxonomyList) -> String {
    format!("TaxonomyList({})", tl.len())
}

/// Go: `OrderedTaxonomy.getOneOPage()` — the first page of the first entry, `None` if there is
/// no entry (Go panics on an entry without pages; Hugo never creates one).
// Go: resources/page/taxonomy.go:getOneOPage
fn get_one_o_page(t: &OrderedTaxonomy) -> Option<PageRef> {
    t.first()?.pages().first().cloned()
}

/// Go: `Taxonomy.Get(key)` — lower-cases the key.
// Go: resources/page/taxonomy.go:Get
pub fn taxonomy_get(t: &Taxonomy, key: &str) -> Option<WeightedPages> {
    let k = go_unicode::strings::to_lower(key.as_bytes());
    t.get(&*String::from_utf8_lossy(&k)).cloned()
}

/// Go: `Taxonomy.Count(key)`.
// Go: resources/page/taxonomy.go:Count
pub fn taxonomy_count(t: &Taxonomy, key: &str) -> i64 {
    taxonomy_get(t, key).map(|w| w.len() as i64).unwrap_or(0)
}

/// Go: `Taxonomy.TaxonomyArray()` (Go: map order; here: key order).
// Go: resources/page/taxonomy.go:TaxonomyArray
pub fn taxonomy_array(t: &Taxonomy) -> OrderedTaxonomy {
    t.iter()
        .map(|(k, v)| OrderedTaxonomyEntry {
            name: k.clone(),
            weighted_pages: v.clone(),
        })
        .collect()
}

/// Go: `oiBy.Sort(taxonomy)` — `sort.Stable`.
// Go: resources/page/taxonomy.go:Sort
fn oi_by_sort(
    by: &mut dyn FnMut(&OrderedTaxonomyEntry, &OrderedTaxonomyEntry) -> bool,
    t: &mut OrderedTaxonomy,
) {
    go_sort::stable_by(t, by);
}

/// Go: `Taxonomy.Alphabetical()` — by name with the first collator of the current site's
/// language (locked for the sort).
// Go: resources/page/taxonomy.go:Alphabetical
pub fn alphabetical(t: &Taxonomy) -> OrderedTaxonomy {
    let mut ia = taxonomy_array(t);
    let Some(p) = get_one_o_page(&ia) else {
        return ia;
    };
    let lang = p.0.site().0.current().0.language();
    let coll = lang.collator1();
    let mut g = coll.lock();
    let mut name = |i1: &OrderedTaxonomyEntry, i2: &OrderedTaxonomyEntry| {
        g.compare_strings(i1.name.as_bytes(), i2.name.as_bytes()) < 0
    };
    oi_by_sort(&mut name, &mut ia);
    ia
}

/// Go: `Taxonomy.ByCount()` — count desc, then `compare.LessStrings(name)`.
// Go: resources/page/taxonomy.go:ByCount
pub fn by_count(t: &Taxonomy) -> OrderedTaxonomy {
    let mut count = |i1: &OrderedTaxonomyEntry, i2: &OrderedTaxonomyEntry| {
        let li1 = i1.weighted_pages.len();
        let li2 = i2.weighted_pages.len();

        if li1 == li2 {
            return compare::less_strings(i1.name.as_bytes(), i2.name.as_bytes());
        }
        li1 > li2
    };

    let mut ia = taxonomy_array(t);
    oi_by_sort(&mut count, &mut ia);
    ia
}

/// Go: `Taxonomy.Page()` — the parent of the first entry's owner (the taxonomy page), `None` if
/// the taxonomy has no terms.
// Go: resources/page/taxonomy.go:Page
pub fn taxonomy_page(t: &Taxonomy) -> nh_common::Result<Option<PageRef>> {
    if let Some(v) = t.values().next() {
        return match weighted_pages_page(v)? {
            Some(owner) => Ok(owner.0.parent()),
            None => Err(nh_common::herrors::Error::new(
                "runtime error: invalid memory address or nil pointer dereference",
            )),
        };
    }
    Ok(None)
}

/// Go: `OrderedTaxonomy.Reverse()` (in place in Go; returns the reversed list).
// Go: resources/page/taxonomy.go:Reverse
pub fn ordered_taxonomy_reverse(t: &OrderedTaxonomy) -> OrderedTaxonomy {
    let mut t = t.clone();
    t.reverse();
    t
}

/// Template values.
pub fn taxonomy_list_to_value(tl: &TaxonomyList) -> Value {
    let mut m = Map::new(MapType::Named(Arc::from(TAXONOMY_LIST_TYPE)));
    for (k, t) in tl.iter() {
        m.insert(GoString::from(k.as_str()), taxonomy_to_value(t));
    }
    Value::map(m)
}

/// `page.Taxonomy` as a template value.
pub fn taxonomy_to_value(t: &Taxonomy) -> Value {
    let mut m = Map::new(MapType::Named(Arc::from(TAXONOMY_TYPE)));
    for (k, wp) in t {
        m.insert(GoString::from(k.as_str()), weighted_pages_to_value(wp));
    }
    Value::map(m)
}

/// `page.Taxonomy` from a template value.
pub fn taxonomy_from_value(v: &Value) -> Option<Taxonomy> {
    match v {
        Value::Map(m) if matches!(&m.ty, MapType::Named(n) if &**n == TAXONOMY_TYPE) => m
            .entries
            .iter()
            .map(|(k, v)| Some((k.to_str_lossy().into_owned(), weighted_pages_from_value(v)?)))
            .collect(),
        Value::TypedNil(t) if &**t == TAXONOMY_TYPE => Some(BTreeMap::new()),
        _ => None,
    }
}

/// `page.OrderedTaxonomy` as a template value.
pub fn ordered_taxonomy_to_value(t: &OrderedTaxonomy) -> Value {
    Value::list(
        SliceType::Named(Arc::from(ORDERED_TAXONOMY_TYPE)),
        t.iter().map(|e| Value::object(e.clone())).collect(),
    )
}

/// `page.OrderedTaxonomy` from a template value.
pub fn ordered_taxonomy_from_value(v: &Value) -> Option<OrderedTaxonomy> {
    match v {
        Value::List(l) if matches!(&l.ty, SliceType::Named(n) if &**n == ORDERED_TAXONOMY_TYPE) => {
            l.items
                .iter()
                .map(|it| it.downcast::<OrderedTaxonomyEntry>().cloned())
                .collect()
        }
        Value::TypedNil(t) if &**t == ORDERED_TAXONOMY_TYPE => Some(Vec::new()),
        _ => None,
    }
}

pub fn taxonomy_has_method(name: &str) -> bool {
    matches!(
        name,
        "Get" | "Count" | "TaxonomyArray" | "Alphabetical" | "ByCount" | "Page"
    )
}

fn nh_err(e: nh_common::herrors::Error) -> go_value::Error {
    go_value::Error::new(e.to_string())
}

pub fn taxonomy_call_method(
    _ctx: HostCtx<'_>,
    recv: &Value,
    name: &str,
    a: &[Value],
) -> Option<GoResult<Value>> {
    if !taxonomy_has_method(name) {
        return None;
    }
    let t = taxonomy_from_value(recv)?;
    let r = (|| -> GoResult<Value> {
        match name {
            "Get" => {
                args::exactly(a, 1, name)?;
                let key = args::string(a, 0)?;
                Ok(match taxonomy_get(&t, &key.to_str_lossy()) {
                    Some(wp) => weighted_pages_to_value(&wp),
                    None => Value::TypedNil(Arc::from(WEIGHTED_PAGES_TYPE)),
                })
            }
            "Count" => {
                args::exactly(a, 1, name)?;
                let key = args::string(a, 0)?;
                Ok(Value::int(taxonomy_count(&t, &key.to_str_lossy())))
            }
            "TaxonomyArray" => {
                args::exactly(a, 0, name)?;
                Ok(ordered_taxonomy_to_value(&taxonomy_array(&t)))
            }
            "Alphabetical" => {
                args::exactly(a, 0, name)?;
                Ok(ordered_taxonomy_to_value(&alphabetical(&t)))
            }
            "ByCount" => {
                args::exactly(a, 0, name)?;
                Ok(ordered_taxonomy_to_value(&by_count(&t)))
            }
            _ => {
                args::exactly(a, 0, name)?;
                Ok(page_opt_value(taxonomy_page(&t).map_err(nh_err)?))
            }
        }
    })();
    Some(r)
}

pub const TAXONOMY_METHODS: NamedMethods = NamedMethods {
    has_method: taxonomy_has_method,
    call: taxonomy_call_method,
};

pub fn taxonomy_list_has_method(name: &str) -> bool {
    name == "String"
}

pub fn taxonomy_list_call_method(
    _ctx: HostCtx<'_>,
    recv: &Value,
    name: &str,
    a: &[Value],
) -> Option<GoResult<Value>> {
    if name != "String" {
        return None;
    }
    let n = match recv {
        Value::Map(m) => m.len(),
        _ => 0,
    };
    Some(args::exactly(a, 0, name).map(|_| Value::string(format!("TaxonomyList({n})"))))
}

/// Methods of `page.TaxonomyList` (`String`).
pub const TAXONOMY_LIST_METHODS: NamedMethods = NamedMethods {
    has_method: taxonomy_list_has_method,
    call: taxonomy_list_call_method,
};

pub fn ordered_taxonomy_has_method(name: &str) -> bool {
    name == "Reverse"
}

pub fn ordered_taxonomy_call_method(
    _ctx: HostCtx<'_>,
    recv: &Value,
    name: &str,
    a: &[Value],
) -> Option<GoResult<Value>> {
    if name != "Reverse" {
        return None;
    }
    let t = ordered_taxonomy_from_value(recv)?;
    Some(
        args::exactly(a, 0, name).map(|_| ordered_taxonomy_to_value(&ordered_taxonomy_reverse(&t))),
    )
}

/// Methods of `page.OrderedTaxonomy` (`Reverse`).
pub const ORDERED_TAXONOMY_METHODS: NamedMethods = NamedMethods {
    has_method: ordered_taxonomy_has_method,
    call: ordered_taxonomy_call_method,
};

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: resources/page/taxonomy.go (177 lines; 1/16 funcs executed)
//   types: TaxonomyList, Taxonomy, OrderedTaxonomy, OrderedTaxonomyEntry, orderedTaxonomySorter, oiBy
// OK L29-31: (tl TaxonomyList) String() string
// OK L46-51: (t OrderedTaxonomy) getOneOPage() Page
// OK L61-63: (i Taxonomy) Get(key string) WeightedPages
// OK L66-66: (i Taxonomy) Count(key string) int
// OK L69-77: (i Taxonomy) TaxonomyArray() OrderedTaxonomy
// OK L80-95: (i Taxonomy) Alphabetical() OrderedTaxonomy
// OK L99-113: (i Taxonomy) ByCount() OrderedTaxonomy
// OK L116-121: (i Taxonomy) Page() Page
// OK L124-126: (ie OrderedTaxonomyEntry) Pages() Pages
// OK L129-131: (ie OrderedTaxonomyEntry) Count() int
// OK L134-136: (ie OrderedTaxonomyEntry) Term() string
// OK L139-145: (t OrderedTaxonomy) Reverse() OrderedTaxonomy
// OK L156-162: (by oiBy) Sort(taxonomy OrderedTaxonomy)
// OK L165-167: (s *orderedTaxonomySorter) Len() int
// OK L170-172: (s *orderedTaxonomySorter) Swap(i, j int)
// OK L175-177: (s *orderedTaxonomySorter) Less(i, j int) bool
// ---------------------------------------------------------------------------
