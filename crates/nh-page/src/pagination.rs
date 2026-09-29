//! Port of `resources/page/pagination.go`.
//!
//! Owner: Wave B task T12 (page-collections).

//! Go `resources/page/pagination.go`: `Pager`/`Paginator`. The page-level "first call wins"
//! (`.Paginator` vs `.Paginate`, one `sync.Once` per page output) lives in nh-hugolib
//! `page__paginator.rs`.
//!
//! As in Go, every `Pager` points back to its `Paginator` and the paginator holds its pagers
//! (`First`/`Last`/`Next`/`Prev` return the same pager objects, so `eq` compares them by
//! identity). The `Arc` cycle keeps a paginator alive until the process ends.

use std::sync::Arc;

use go_value::{GoString, HostCtx, Object, SliceType, Value};
use nh_common::Result;
use nh_common::herrors::Error;
use nh_common::object::args;
use nh_config::config_provider::AllProvider;

use crate::page::{PAGES_TYPE, Pages, pages_from_value};
use crate::page_paths::{TargetPathDescriptor, create_target_paths};
use crate::pagegroup::{
    PAGES_GROUP_TYPE, PageGroup, PagesGroup, group_key_eq, pages_group_len, pages_group_to_value,
    to_pages_group,
};

/// Go: `PaginatorNotSupportedFunc` — a paginator provider whose `Paginate` and `Paginator`
/// return the function's error (hugolib uses it for page kinds that cannot paginate).
#[derive(Clone)]
pub struct PaginatorNotSupportedFunc(pub Arc<dyn Fn() -> Error + Send + Sync>);

impl PaginatorNotSupportedFunc {
    // Go: resources/page/pagination.go:Paginate
    pub fn paginate(&self, _pages: &Value, _options: &[Value]) -> Result<Arc<Pager>> {
        Err((self.0)())
    }

    // Go: resources/page/pagination.go:Paginator
    pub fn paginator(&self, _options: &[Value]) -> Result<Arc<Pager>> {
        Err((self.0)())
    }
}

/// Go: `paginatedElement` (a chunk of Pages or of PagesGroup).
#[derive(Clone)]
pub enum PaginatedElement {
    Pages(Pages),
    Groups(PagesGroup),
}

impl PaginatedElement {
    /// Go: `paginatedElement.Len()` (`Pages.Len` / `PagesGroup.Len`).
    pub fn len(&self) -> i64 {
        match self {
            PaginatedElement::Pages(p) => p.len() as i64,
            PaginatedElement::Groups(g) => pages_group_len(g),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// Go: `paginationURLFactory`.
pub type PaginationUrlFactory = Arc<dyn Fn(i64) -> String + Send + Sync>;

/// Go: `page.Paginator`.
pub struct Paginator {
    pub(crate) paginated_elements: Vec<PaginatedElement>,
    pub(crate) url_factory: PaginationUrlFactory,
    pub(crate) total: i64,
    pub(crate) size: i64,
    /// Filled right after construction (pagers point back to the paginator).
    pub(crate) pagers: std::sync::OnceLock<Vec<Arc<Pager>>>,
}

/// Go: `page.Pager` (`*page.Pager` in templates).
pub struct Pager {
    /// 1-based.
    pub number: i64,
    pub paginator: Arc<Paginator>,
}

impl Paginator {
    /// Go: `p.Pagers()`.
    // Go: resources/page/pagination.go:Pagers
    pub fn pagers(&self) -> Vec<Arc<Pager>> {
        self.pagers.get().cloned().unwrap_or_default()
    }
    /// Go: `p.PageSize()` (deprecated alias; Go logs a deprecation notice).
    // Go: resources/page/pagination.go:PageSize
    pub fn page_size(&self) -> i64 {
        self.size
    }
    // Go: resources/page/pagination.go:PagerSize
    pub fn pager_size(&self) -> i64 {
        self.size
    }
    // Go: resources/page/pagination.go:TotalPages
    pub fn total_pages(&self) -> i64 {
        self.paginated_elements.len() as i64
    }
    // Go: resources/page/pagination.go:TotalNumberOfElements
    pub fn total_number_of_elements(&self) -> i64 {
        self.total
    }
}

impl Pager {
    /// Go: `Pager.String()`.
    // Go: resources/page/pagination.go:String
    pub fn string(&self) -> String {
        format!("Pager {}", self.number)
    }
    // Go: resources/page/pagination.go:PageNumber
    pub fn page_number(&self) -> i64 {
        self.number
    }
    // Go: resources/page/pagination.go:URL
    pub fn url(&self) -> String {
        (self.paginator.url_factory)(self.page_number())
    }
    /// Go: `p.Pages()` — nil when the pager holds groups or there are no elements.
    // Go: resources/page/pagination.go:Pages
    pub fn pages(&self) -> Pages {
        self.pages_opt().unwrap_or_default()
    }
    /// [`Pager::pages`] with Go's nil as `None`.
    pub fn pages_opt(&self) -> Option<Pages> {
        if self.paginator.paginated_elements.is_empty() {
            return None;
        }

        if let Some(PaginatedElement::Pages(pages)) = self.element() {
            return Some(pages.clone());
        }

        None
    }
    /// Go: `p.PageGroups()` — nil when the pager holds pages or there are no elements.
    // Go: resources/page/pagination.go:PageGroups
    pub fn page_groups(&self) -> PagesGroup {
        self.page_groups_opt().unwrap_or_default()
    }
    /// [`Pager::page_groups`] with Go's nil as `None`.
    pub fn page_groups_opt(&self) -> Option<PagesGroup> {
        if self.paginator.paginated_elements.is_empty() {
            return None;
        }

        if let Some(PaginatedElement::Groups(groups)) = self.element() {
            return Some(groups.clone());
        }

        None
    }
    /// Go: `p.element()` — `None` is `paginatorEmptyPages`.
    // Go: resources/page/pagination.go:element
    fn element(&self) -> Option<&PaginatedElement> {
        if self.paginator.paginated_elements.is_empty() {
            return None;
        }
        self.paginator
            .paginated_elements
            .get((self.page_number() - 1) as usize)
    }
    /// Go: `p.page(index)` — the page at `index` of this pager (across groups).
    // Go: resources/page/pagination.go:page
    pub fn page(&self, index: i64) -> Option<crate::page::PageRef> {
        match self.element() {
            None => None,
            Some(PaginatedElement::Pages(pages)) => {
                if index >= 0 && (pages.len() as i64) > index {
                    return Some(pages[index as usize].clone());
                }
                None
            }
            Some(PaginatedElement::Groups(groups)) => {
                let mut i = 0;
                for v in groups {
                    for page in &v.pages {
                        if i == index {
                            return Some(page.clone());
                        }
                        i += 1;
                    }
                }
                None
            }
        }
    }
    // Go: resources/page/pagination.go:HasPrev
    pub fn has_prev(&self) -> bool {
        self.page_number() > 1
    }
    // Go: resources/page/pagination.go:Prev
    pub fn prev(&self) -> Option<Arc<Pager>> {
        if !self.has_prev() {
            return None;
        }
        self.paginator
            .pagers()
            .get((self.page_number() - 2) as usize)
            .cloned()
    }
    // Go: resources/page/pagination.go:HasNext
    pub fn has_next(&self) -> bool {
        self.page_number() < self.paginator.paginated_elements.len() as i64
    }
    // Go: resources/page/pagination.go:Next
    pub fn next(&self) -> Option<Arc<Pager>> {
        if !self.has_next() {
            return None;
        }
        self.paginator
            .pagers()
            .get(self.page_number() as usize)
            .cloned()
    }
    // Go: resources/page/pagination.go:First
    pub fn first(&self) -> Arc<Pager> {
        self.paginator.pagers()[0].clone()
    }
    // Go: resources/page/pagination.go:Last
    pub fn last(&self) -> Arc<Pager> {
        let ps = self.paginator.pagers();
        ps[ps.len() - 1].clone()
    }
    // Go: resources/page/pagination.go:NumberOfElements
    pub fn number_of_elements(&self) -> i64 {
        self.element().map(|e| e.len()).unwrap_or(0)
    }
}

/// Template value of `*page.Pager` (pointer identity: `eq $pag $p` compares pagers).
#[derive(Clone)]
pub struct PagerRef(pub Arc<Pager>);

fn pages_opt_value(p: Option<Pages>) -> Value {
    match p {
        Some(p) => crate::page::pages_to_value(&p),
        None => Value::TypedNil(Arc::from(PAGES_TYPE)),
    }
}

nh_common::go_methods!(PagerRef {
    "PageNumber" => |p, _c, a| {
        args::exactly(a, 0, "PageNumber")?;
        Ok(Value::int(p.0.page_number()))
    },
    "URL" => |p, _c, a| {
        args::exactly(a, 0, "URL")?;
        Ok(Value::string(p.0.url()))
    },
    "Pages" => |p, _c, a| {
        args::exactly(a, 0, "Pages")?;
        Ok(pages_opt_value(p.0.pages_opt()))
    },
    "PageGroups" => |p, _c, a| {
        args::exactly(a, 0, "PageGroups")?;
        Ok(match p.0.page_groups_opt() {
            Some(g) => pages_group_to_value(&g),
            None => Value::TypedNil(Arc::from(PAGES_GROUP_TYPE)),
        })
    },
    "NumberOfElements" => |p, _c, a| {
        args::exactly(a, 0, "NumberOfElements")?;
        Ok(Value::int(p.0.number_of_elements()))
    },
    "HasPrev" => |p, _c, a| {
        args::exactly(a, 0, "HasPrev")?;
        Ok(Value::Bool(p.0.has_prev()))
    },
    "Prev" => |p, _c, a| {
        args::exactly(a, 0, "Prev")?;
        Ok(pager_opt_value(p.0.prev()))
    },
    "HasNext" => |p, _c, a| {
        args::exactly(a, 0, "HasNext")?;
        Ok(Value::Bool(p.0.has_next()))
    },
    "Next" => |p, _c, a| {
        args::exactly(a, 0, "Next")?;
        Ok(pager_opt_value(p.0.next()))
    },
    "First" => |p, _c, a| {
        args::exactly(a, 0, "First")?;
        Ok(Value::object(PagerRef(p.0.first())))
    },
    "Last" => |p, _c, a| {
        args::exactly(a, 0, "Last")?;
        Ok(Value::object(PagerRef(p.0.last())))
    },
    "Pagers" => |p, _c, a| {
        args::exactly(a, 0, "Pagers")?;
        Ok(pagers_to_value(&p.0.paginator.pagers()))
    },
    "PagerSize" => |p, _c, a| {
        args::exactly(a, 0, "PagerSize")?;
        Ok(Value::int(p.0.paginator.pager_size()))
    },
    "PageSize" => |p, _c, a| {
        args::exactly(a, 0, "PageSize")?;
        Ok(Value::int(p.0.paginator.page_size()))
    },
    "TotalPages" => |p, _c, a| {
        args::exactly(a, 0, "TotalPages")?;
        Ok(Value::int(p.0.paginator.total_pages()))
    },
    "TotalNumberOfElements" => |p, _c, a| {
        args::exactly(a, 0, "TotalNumberOfElements")?;
        Ok(Value::int(p.0.paginator.total_number_of_elements()))
    },
    "String" => |p, _c, a| {
        args::exactly(a, 0, "String")?;
        Ok(Value::string(p.0.string()))
    },
});

impl Object for PagerRef {
    nh_common::object_basics!("*page.Pager");

    fn go_string(&self) -> Option<GoString> {
        Some(GoString::from(self.0.string()))
    }

    /// Go: the embedded `*Paginator` field.
    fn field(&self, name: &str) -> Option<Value> {
        match name {
            "Paginator" => Some(Value::object(PaginatorRef(self.0.paginator.clone()))),
            _ => None,
        }
    }

    fn identity(&self) -> usize {
        Arc::as_ptr(&self.0) as usize
    }
}

/// Go: a `*page.Paginator` as a template value (the `Paginator` field of a `*page.Pager`).
#[derive(Clone)]
pub struct PaginatorRef(pub Arc<Paginator>);

nh_common::go_methods!(PaginatorRef {
    // Go: resources/page/pagination.go:Pagers
    "Pagers" => |p, _c, a| {
        args::exactly(a, 0, "Pagers")?;
        Ok(pagers_to_value(&p.0.pagers()))
    },
    // Go: resources/page/pagination.go:PageSize
    "PageSize" => |p, _c, a| {
        args::exactly(a, 0, "PageSize")?;
        Ok(Value::int(p.0.page_size()))
    },
    // Go: resources/page/pagination.go:PagerSize
    "PagerSize" => |p, _c, a| {
        args::exactly(a, 0, "PagerSize")?;
        Ok(Value::int(p.0.pager_size()))
    },
    // Go: resources/page/pagination.go:TotalPages
    "TotalPages" => |p, _c, a| {
        args::exactly(a, 0, "TotalPages")?;
        Ok(Value::int(p.0.total_pages()))
    },
    // Go: resources/page/pagination.go:TotalNumberOfElements
    "TotalNumberOfElements" => |p, _c, a| {
        args::exactly(a, 0, "TotalNumberOfElements")?;
        Ok(Value::int(p.0.total_number_of_elements()))
    },
});

impl Object for PaginatorRef {
    nh_common::object_basics!("*page.Paginator");

    fn identity(&self) -> usize {
        Arc::as_ptr(&self.0) as usize
    }
}

fn pager_opt_value(p: Option<Arc<Pager>>) -> Value {
    match p {
        Some(p) => Value::object(PagerRef(p)),
        None => Value::TypedNil(Arc::from("*page.Pager")),
    }
}

/// `page.pagers` named slice value.
pub fn pagers_to_value(ps: &[Arc<Pager>]) -> Value {
    Value::list(
        SliceType::Named(Arc::from("page.pagers")),
        ps.iter()
            .map(|p| Value::object(PagerRef(p.clone())))
            .collect(),
    )
}

/// Go: `splitPages(pages, size)`.
// Go: resources/page/pagination.go:splitPages
pub fn split_pages(pages: &Pages, size: i64) -> Vec<PaginatedElement> {
    let mut split = Vec::new();
    let mut low = 0usize;
    let j = pages.len();
    while low < j {
        let high = std::cmp::min(low + size as usize, pages.len());
        split.push(PaginatedElement::Pages(pages[low..high].to_vec()));
        low += size as usize;
    }

    split
}

/// Go: `splitPageGroups(pageGroups, size)` — the groups flattened to (key, page) and chunked; a
/// chunk starts a new group whenever the key changes (a nil key starts one for every page).
// Go: resources/page/pagination.go:splitPageGroups
pub fn split_page_groups(page_groups: &PagesGroup, size: i64) -> Vec<PaginatedElement> {
    let mut split = Vec::new();
    let mut flattened: Vec<(Value, crate::page::PageRef)> = Vec::new();

    for g in page_groups {
        for p in &g.pages {
            flattened.push((g.key.clone(), p.clone()));
        }
    }

    let num_pages = flattened.len();

    let mut low = 0usize;
    while low < num_pages {
        let high = std::cmp::min(low + size as usize, num_pages);

        let mut pg: PagesGroup = Vec::new();
        let mut key = Value::Invalid;

        for kp in &flattened[low..high] {
            if key.is_invalid() || !group_key_eq(&key, &kp.0) {
                key = kp.0.clone();
                pg.push(PageGroup {
                    key: key.clone(),
                    pages: Vec::new(),
                });
            }
            let last = pg.len() - 1;
            pg[last].pages.push(kp.1.clone());
        }
        split.push(PaginatedElement::Groups(pg));
        low += size as usize;
    }

    split
}

/// Go: `page.ResolvePagerSize(conf, options...)`.
// Go: resources/page/pagination.go:ResolvePagerSize
pub fn resolve_pager_size(conf: &dyn AllProvider, options: &[Value]) -> Result<i64> {
    if options.is_empty() {
        return Ok(conf.pagination().pager_size);
    }

    resolve_pager_size_option(options)
}

/// `ResolvePagerSize` with options (the configured size is not needed).
pub fn resolve_pager_size_option(options: &[Value]) -> Result<i64> {
    if options.len() > 1 {
        return Err(Error::new(
            "too many arguments, 'pager size' is currently the only option",
        ));
    }

    match nh_common::cast::caste::to_int_e(&options[0]) {
        Ok(pas) if pas > 0 => Ok(pas),
        _ => Err(Error::new("'pager size' must be a positive integer")),
    }
}

/// Go: `page.Paginate(td, seq, pagerSize)`.
// Go: resources/page/pagination.go:Paginate
pub fn paginate(td: &TargetPathDescriptor, seq: &Value, pager_size: i64) -> Result<Arc<Paginator>> {
    if pager_size <= 0 {
        return Err(Error::new(
            "'paginate' configuration setting must be positive to paginate",
        ));
    }

    let url_factory = new_pagination_url_factory(td.clone());

    paginate_with(url_factory, seq, pager_size)
}

/// `Paginate` with a given URL factory (tests, and callers that build their own).
pub fn paginate_with(
    url_factory: PaginationUrlFactory,
    seq: &Value,
    pager_size: i64,
) -> Result<Arc<Paginator>> {
    if pager_size <= 0 {
        return Err(Error::new(
            "'paginate' configuration setting must be positive to paginate",
        ));
    }
    let groups = to_pages_group(seq)?;
    let paginator = match groups {
        Some(groups) => new_paginator_from_page_groups(&groups, pager_size, url_factory)?,
        None => {
            let pages = pages_from_value(seq)?;
            new_paginator_from_pages(&pages, pager_size, url_factory)?
        }
    };

    Ok(paginator)
}

/// Go: `probablyEqualPageLists(a1, a2)`.
// Go: resources/page/pagination.go:probablyEqualPageLists
pub fn probably_equal_page_lists(a1: &Value, a2: &Value) -> bool {
    if a1.is_invalid() || a2.is_invalid() {
        return a1.is_invalid() && a2.is_invalid();
    }

    if a1.go_type_name() != a2.go_type_name() {
        return false;
    }

    let is_group = |v: &Value| matches!(v, Value::List(l) if matches!(&l.ty, SliceType::Named(n) if &**n == PAGES_GROUP_TYPE));
    if is_group(a1) {
        let g1 = crate::pagegroup::pages_group_from_value(a1).unwrap_or_default();
        let g2 = crate::pagegroup::pages_group_from_value(a2).unwrap_or_default();
        if g1.len() != g2.len() {
            return false;
        }
        if g1.is_empty() {
            return true;
        }
        if pages_group_len(&g1) != pages_group_len(&g2) {
            return false;
        }

        // Go indexes `Pages[0]` of the first group (a panic when it is empty).
        return match (g1[0].pages.first(), g2[0].pages.first()) {
            (Some(a), Some(b)) => crate::pages_sort::page_ref_eq(a, b),
            _ => false,
        };
    }

    let (p1, p2) = (pages_from_value(a1), pages_from_value(a2));

    // probably the same wrong type
    if p1.is_err() && p2.is_err() {
        return true;
    }
    let (p1, p2) = (p1.unwrap_or_default(), p2.unwrap_or_default());

    if p1.len() != p2.len() {
        return false;
    }

    if p1.is_empty() {
        return true;
    }

    crate::pages_sort::page_ref_eq(&p1[0], &p2[0])
}

// Go: resources/page/pagination.go:newPaginatorFromPages
pub fn new_paginator_from_pages(
    pages: &Pages,
    size: i64,
    url_factory: PaginationUrlFactory,
) -> Result<Arc<Paginator>> {
    if size <= 0 {
        return Err(Error::new("Paginator size must be positive"));
    }

    let split = split_pages(pages, size);

    Ok(new_paginator(split, pages.len() as i64, size, url_factory))
}

// Go: resources/page/pagination.go:newPaginatorFromPageGroups
pub fn new_paginator_from_page_groups(
    page_groups: &PagesGroup,
    size: i64,
    url_factory: PaginationUrlFactory,
) -> Result<Arc<Paginator>> {
    if size <= 0 {
        return Err(Error::new("Paginator size must be positive"));
    }

    let split = split_page_groups(page_groups, size);

    Ok(new_paginator(
        split,
        pages_group_len(page_groups),
        size,
        url_factory,
    ))
}

/// Go: `newPaginator(elements, total, size, urlFactory)` — one pager per element, or a single
/// pager when there are none.
// Go: resources/page/pagination.go:newPaginator
fn new_paginator(
    elements: Vec<PaginatedElement>,
    total: i64,
    size: i64,
    url_factory: PaginationUrlFactory,
) -> Arc<Paginator> {
    let n = elements.len();
    let p = Arc::new(Paginator {
        paginated_elements: elements,
        url_factory,
        total,
        size,
        pagers: std::sync::OnceLock::new(),
    });

    let ps: Vec<Arc<Pager>> = if n > 0 {
        (0..n)
            .map(|i| {
                Arc::new(Pager {
                    number: (i + 1) as i64,
                    paginator: p.clone(),
                })
            })
            .collect()
    } else {
        vec![Arc::new(Pager {
            number: 1,
            paginator: p.clone(),
        })]
    };

    let _ = p.pagers.set(ps);

    p
}

/// Go: `newPaginationURLFactory(d)` — page 1 -> the node's RelPermalink; page N ->
/// `CreateTargetPaths(d + Addends "/<paginatePath>/N/").RelPermalink`.
// Go: resources/page/pagination.go:newPaginationURLFactory
pub fn new_pagination_url_factory(
    d: TargetPathDescriptor,
) -> Arc<dyn Fn(i64) -> String + Send + Sync> {
    Arc::new(move |page_number: i64| {
        let mut path_descriptor = d.clone();
        if page_number > 1 {
            let rel = format!("/{}/{}/", d.path_spec.cfg.pagination().path, page_number);
            path_descriptor.addends = rel;
        }

        create_target_paths(&path_descriptor).rel_permalink(&d.path_spec)
    })
}

/// `Pager` from a template value.
pub fn pager_from_value(v: &Value) -> Option<Arc<Pager>> {
    v.downcast::<PagerRef>().map(|p| p.0.clone())
}

impl std::fmt::Debug for Pager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.string())
    }
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: resources/page/pagination.go (417 lines; 19/30 funcs executed)
//   types: PaginatorProvider, PaginatorNotSupportedFunc, Pager, paginatedElement, pagers, Paginator,
//          paginationURLFactory
// OK L40-42: (f PaginatorNotSupportedFunc) Paginate(pages any, options ...any) (*Pager, error)
// OK L44-46: (f PaginatorNotSupportedFunc) Paginator(options ...any) (*Pager, error)
// OK L55-57: (p Pager) String() string
// OK L81-83: (p *Pager) PageNumber() int
// OK L86-88: (p *Pager) URL() string
// OK L92-102: (p *Pager) Pages() Pages
// OK L106-116: (p *Pager) PageGroups() PagesGroup
// OK L118-123: (p *Pager) element() paginatedElement
// OK L126-149: (p *Pager) page(index int) (Page, error)
// OK L152-154: (p *Pager) NumberOfElements() int
// OK L157-159: (p *Pager) HasPrev() bool
// OK L162-167: (p *Pager) Prev() *Pager
// OK L170-172: (p *Pager) HasNext() bool
// OK L175-180: (p *Pager) Next() *Pager
// OK L183-185: (p *Pager) First() *Pager
// OK L188-190: (p *Pager) Last() *Pager
// OK L193-195: (p *Paginator) Pagers() pagers
// OK L199-202: (p *Paginator) PageSize() int
// OK L205-207: (p *Paginator) PagerSize() int
// OK L210-212: (p *Paginator) TotalPages() int
// OK L215-217: (p *Paginator) TotalNumberOfElements() int
// OK L219-227: splitPages(pages Pages, size int) []paginatedElement
// OK L229-270: splitPageGroups(pageGroups PagesGroup, size int) []paginatedElement
// OK L272-288: ResolvePagerSize(conf config.AllProvider, options ...any) (int, error)
// OK L290-314: Paginate(td TargetPathDescriptor, seq any, pagerSize int) (*Paginator, error)
// OK L320-364: probablyEqualPageLists(a1 any, a2 any) bool
// OK L366-374: newPaginatorFromPages(pages Pages, size int, urlFactory paginationURLFactory) (*Paginator, error)
// OK L376-384: newPaginatorFromPageGroups(pageGroups PagesGroup, size int, urlFactory paginationURLFactory) (*Paginator, error)
// OK L386-404: newPaginator(elements []paginatedElement, total, size int, urlFactory paginationURLFactory) (*Paginator, error)
// OK L406-417: newPaginationURLFactory(d TargetPathDescriptor) paginationURLFactory
// ---------------------------------------------------------------------------
