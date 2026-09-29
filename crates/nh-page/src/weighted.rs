//! Port of `resources/page/weighted.go`.
//!
//! Owner: Wave B task T12 (page-collections).

use std::any::Any;
use std::borrow::Cow;
use std::sync::Arc;

use go_value::{GoString, HostCtx, Object, SliceType, Value};
use nh_common::object::{GoResult, NamedMethods, args};

use crate::page::{PageRef, Pages, page_from_value};
use crate::pages_sort::default_page_sort;

pub const WEIGHTED_PAGES_TYPE: &str = "page.WeightedPages";
/// Go type string of `page.WeightedPage` (a struct value).
pub const WEIGHTED_PAGE_TYPE: &str = "page.WeightedPage";

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

    /// Go: `WeightedPage.String()` — `WeightedPage(%d,%q)` with the page title.
    // Go: resources/page/weighted.go:String
    pub fn string(&self) -> Vec<u8> {
        go_fmt::sprintf(
            "WeightedPage(%d,%q)",
            &[
                Value::int(self.weight),
                Value::string(crate::page::Page::title(&*self.page.0)),
            ],
        )
    }

    /// Go: `WeightedPage.Slice(in)` — for `collections.Slice` (`slice $wp1 $wp2`).
    // Go: resources/page/weighted.go:Slice
    pub fn slice(&self, input: &Value) -> GoResult<Value> {
        match input {
            Value::List(l) if matches!(&l.ty, SliceType::Named(n) if &**n == WEIGHTED_PAGES_TYPE) => {
                Ok(input.clone())
            }
            Value::TypedNil(t) if &**t == WEIGHTED_PAGES_TYPE => Ok(input.clone()),
            Value::List(l) if l.ty == SliceType::Any => {
                let mut weighted = Vec::with_capacity(l.items.len());
                for v in &l.items {
                    match v.downcast::<WeightedPage>() {
                        Some(g) => weighted.push(g.clone()),
                        None => {
                            return Err(go_value::Error::new(format!(
                                "type {} is not a WeightedPage",
                                type_of(v)
                            )));
                        }
                    }
                }
                Ok(weighted_pages_to_value(&weighted))
            }
            _ => Err(go_value::Error::new(format!(
                "invalid slice type {}",
                type_of(input)
            ))),
        }
    }
}

fn type_of(v: &Value) -> String {
    match v {
        Value::Invalid => "<nil>".to_string(),
        _ => v.go_type_name().into_owned(),
    }
}

/// The methods of the `page.Page` interface (`WeightedPage` embeds the interface, so only these
/// are promoted; `Slice` and `String` are `WeightedPage`'s own), sorted.
// Go: resources/page/page.go:Page (method set of page.WeightedPage)
const PAGE_INTERFACE_METHODS: &[&str] = &[
    "Aliases",
    "AllTranslations",
    "AlternativeOutputFormats",
    "Ancestors",
    "BundleType",
    "CodeOwners",
    "Content",
    "ContentWithoutSummary",
    "CurrentSection",
    "Data",
    "Date",
    "Description",
    "Draft",
    "Eq",
    "ExpiryDate",
    "File",
    "FirstSection",
    "Fragments",
    "FuzzyWordCount",
    "GetPage",
    "GetTerms",
    "GitInfo",
    "HasMenuCurrent",
    "HasShortcode",
    "HeadingsFiltered",
    "InSection",
    "IsAncestor",
    "IsDescendant",
    "IsHome",
    "IsMenuCurrent",
    "IsNode",
    "IsPage",
    "IsSection",
    "IsTranslated",
    "Keywords",
    "Kind",
    "Lang",
    "Language",
    "Lastmod",
    "Layout",
    "Len",
    "LinkTitle",
    "Markup",
    "MediaType",
    "Menus",
    "Name",
    "Next",
    "NextInSection",
    "NextPage",
    "OutputFormats",
    "Pages",
    "Paginate",
    "Paginator",
    "Param",
    "Params",
    "Parent",
    "Path",
    "PathInfo",
    "Permalink",
    "Plain",
    "PlainWords",
    "Prev",
    "PrevInSection",
    "PrevPage",
    "PublishDate",
    "RawContent",
    "ReadingTime",
    "Ref",
    "RefFrom",
    "RegularPages",
    "RegularPagesRecursive",
    "RelPermalink",
    "RelRef",
    "RelRefFrom",
    "RelatedKeywords",
    "Render",
    "RenderShortcodes",
    "RenderString",
    "ResourceType",
    "Resources",
    "Scratch",
    "Section",
    "Sections",
    "SectionsEntries",
    "SectionsPath",
    "Site",
    "Sitemap",
    "Sites",
    "Slug",
    "Store",
    "Summary",
    "TableOfContents",
    "Title",
    "TranslationKey",
    "Translations",
    "Truncated",
    "Type",
    "WordCount",
];

impl Object for WeightedPage {
    fn type_name(&self) -> Cow<'_, str> {
        Cow::Borrowed(WEIGHTED_PAGE_TYPE)
    }
    fn kind(&self) -> go_value::Kind {
        go_value::Kind::Struct
    }
    /// Promoted methods of the embedded Page, plus `Slice`, `String`.
    fn has_method(&self, name: &str) -> bool {
        name == "Slice"
            || name == "String"
            || (PAGE_INTERFACE_METHODS.binary_search(&name).is_ok() && self.page.has_method(name))
    }
    fn call_method(&self, ctx: HostCtx<'_>, name: &str, args: &[Value]) -> Option<GoResult<Value>> {
        match name {
            "Slice" => {
                Some(args::exactly(args, 1, name).and_then(|_| self.slice(&args::get(args, 0)?)))
            }
            "String" => Some(
                args::exactly(args, 0, name).map(|_| Value::string(GoString::from(self.string()))),
            ),
            _ if PAGE_INTERFACE_METHODS.binary_search(&name).is_ok() => {
                self.page.call_method(ctx, name, args)
            }
            _ => None,
        }
    }
    fn field(&self, name: &str) -> Option<Value> {
        match name {
            "Weight" => Some(Value::int(self.weight)),
            "Page" => Some(self.page.to_value()),
            _ => None,
        }
    }
    fn go_string(&self) -> Option<GoString> {
        Some(GoString::from(self.string()))
    }
    fn struct_fields(&self) -> Option<Vec<(Cow<'_, str>, Value)>> {
        Some(vec![
            (Cow::Borrowed("Weight"), Value::int(self.weight)),
            (Cow::Borrowed("Page"), self.page.to_value()),
        ])
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// Go: `page.WeightedPages`.
pub type WeightedPages = Vec<WeightedPage>;

/// Go: `WeightedPages.Page()` — the owner of the first entry (the term page). Go panics with
/// `WeightedPages is empty` for an empty list; the port returns that message as the error.
// Go: resources/page/weighted.go:Page
pub fn weighted_pages_page(wp: &WeightedPages) -> nh_common::Result<Option<PageRef>> {
    let Some(first) = wp.first() else {
        return Err(nh_common::herrors::Error::new("WeightedPages is empty"));
    };

    // TODO(bep) fix tests
    Ok(first.owner.clone())
}

/// Go: `WeightedPages.Pages()`.
// Go: resources/page/weighted.go:Pages
pub fn weighted_pages_pages(wp: &WeightedPages) -> Pages {
    wp.iter().map(|w| w.page.clone()).collect()
}

/// Go: `WeightedPages.Next(cur)` — the entry BEFORE `cur`.
// Go: resources/page/weighted.go:Next
pub fn weighted_pages_next(wp: &WeightedPages, cur: &PageRef) -> Option<PageRef> {
    for (x, c) in wp.iter().enumerate() {
        if c.page.0.page_id() == cur.0.page_id() {
            if x == 0 {
                return None;
            }
            return Some(wp[x - 1].page.clone());
        }
    }
    None
}

/// Go: `WeightedPages.Prev(cur)` — the entry AFTER `cur`.
// Go: resources/page/weighted.go:Prev
pub fn weighted_pages_prev(wp: &WeightedPages, cur: &PageRef) -> Option<PageRef> {
    for (x, c) in wp.iter().enumerate() {
        if c.page.0.page_id() == cur.0.page_id() {
            if x < wp.len() - 1 {
                return Some(wp[x + 1].page.clone());
            }
            return None;
        }
    }
    None
}

/// Go: `WeightedPages.Less(i, j)` — weight, then `DefaultPageSort`.
// Go: resources/page/weighted.go:Less
pub fn weighted_less(a: &WeightedPage, b: &WeightedPage) -> bool {
    if a.weight == b.weight {
        return default_page_sort(&*a.page.0, &*b.page.0);
    }
    a.weight < b.weight
}

/// Go: `WeightedPages.Sort()` — `sort.Stable` by (weight, then DefaultPageSort).
// Go: resources/page/weighted.go:Sort
pub fn sort(wp: &mut WeightedPages) {
    go_sort::stable_by(wp, weighted_less);
}

pub fn weighted_pages_to_value(wp: &WeightedPages) -> Value {
    Value::list(
        SliceType::Named(Arc::from(WEIGHTED_PAGES_TYPE)),
        wp.iter().map(|w| Value::object(w.clone())).collect(),
    )
}

/// `page.WeightedPages` from a template value (`None` if it is not one).
pub fn weighted_pages_from_value(v: &Value) -> Option<WeightedPages> {
    match v {
        Value::List(l) if matches!(&l.ty, SliceType::Named(n) if &**n == WEIGHTED_PAGES_TYPE) => l
            .items
            .iter()
            .map(|it| it.downcast::<WeightedPage>().cloned())
            .collect(),
        Value::TypedNil(t) if &**t == WEIGHTED_PAGES_TYPE => Some(Vec::new()),
        _ => None,
    }
}

pub(crate) fn page_opt_value(p: Option<PageRef>) -> Value {
    match p {
        Some(p) => p.to_value(),
        None => Value::TypedNil(Arc::from(crate::page::PAGE_TYPE)),
    }
}

/// A `page.Page` parameter (Go `validateType`: a nil interface is allowed and is a nil page).
pub(crate) fn page_arg(args: &[Value], i: usize) -> GoResult<Option<PageRef>> {
    let v = args::get(args, i)?;
    if v.is_invalid() {
        return Ok(None);
    }
    match page_from_value(&v) {
        Some(p) => Ok(Some(p)),
        None => Err(args::wrong_type(crate::page::PAGE_TYPE, &v)),
    }
}

pub fn weighted_pages_has_method(name: &str) -> bool {
    matches!(
        name,
        "Page" | "Pages" | "Next" | "Prev" | "Len" | "Swap" | "Sort" | "Count" | "Less"
    )
}

pub fn weighted_pages_call_method(
    _ctx: HostCtx<'_>,
    recv: &Value,
    name: &str,
    a: &[Value],
) -> Option<GoResult<Value>> {
    if !weighted_pages_has_method(name) {
        return None;
    }
    let wp = weighted_pages_from_value(recv)?;
    Some(weighted_pages_call(&wp, name, a))
}

fn weighted_pages_call(wp: &WeightedPages, name: &str, a: &[Value]) -> GoResult<Value> {
    match name {
        "Page" => {
            args::exactly(a, 0, name)?;
            let p = weighted_pages_page(wp).map_err(|e| go_value::Error::new(e.to_string()))?;
            Ok(page_opt_value(p))
        }
        "Pages" => {
            args::exactly(a, 0, name)?;
            Ok(crate::page::pages_to_value(&weighted_pages_pages(wp)))
        }
        "Next" | "Prev" => {
            args::exactly(a, 1, name)?;
            let Some(cur) = page_arg(a, 0)? else {
                // Go: `c.Eq(nil)` is false for every entry.
                return Ok(page_opt_value(None));
            };
            let r = if name == "Next" {
                weighted_pages_next(wp, &cur)
            } else {
                weighted_pages_prev(wp, &cur)
            };
            Ok(page_opt_value(r))
        }
        "Len" | "Count" => {
            args::exactly(a, 0, name)?;
            Ok(Value::int(wp.len() as i64))
        }
        "Less" => {
            args::exactly(a, 2, name)?;
            let (i, j) = (args::int(a, 0)?, args::int(a, 1)?);
            let n = wp.len() as i64;
            if i < 0 || i >= n || j < 0 || j >= n {
                return Err(go_value::Error::new(format!(
                    "runtime error: index out of range [{}] with length {n}",
                    if i < 0 || i >= n { i } else { j }
                )));
            }
            Ok(Value::Bool(weighted_less(&wp[i as usize], &wp[j as usize])))
        }
        // `Swap(i, j int)` and `Sort()` have no results: templates cannot call them. Go's
        // text/template checks the argument count first (evalCall), then goodFunc.
        "Swap" => {
            args::exactly(a, 2, name)?;
            Err(nh_common::object::bad_results_error(name, 0))
        }
        _ => {
            args::exactly(a, 0, name)?;
            Err(nh_common::object::bad_results_error(name, 0))
        }
    }
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
// OK L31-44: (p WeightedPages) Page() Page
// OK L58-60: NewWeightedPage(weight int, p Page, owner Page) WeightedPage
// OK L62-64: (w WeightedPage) String() string
// OK L68-85: (p WeightedPage) Slice(in any) (any, error)
// OK L88-94: (wp WeightedPages) Pages() Pages
// OK L98-108: (wp WeightedPages) Next(cur Page) Page
// OK L112-122: (wp WeightedPages) Prev(cur Page) Page
// OK L124-124: (wp WeightedPages) Len() int
// OK L125-125: (wp WeightedPages) Swap(i, j int)
// OK L128-128: (wp WeightedPages) Sort()
// OK L131-131: (wp WeightedPages) Count() int
// OK L133-138: (wp WeightedPages) Less(i, j int) bool
// ---------------------------------------------------------------------------
