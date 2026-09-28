//! Port of `resources/page/pagination.go`.
//!
//! Owner: Wave B task T12 (page-collections).

//! Go `resources/page/pagination.go`: `Pager`/`Paginator`. The page-level "first call wins"
//! (`.Paginator` vs `.Paginate`, one `sync.Once` per page output) lives in nh-hugolib
//! `page__paginator.rs`.

use std::any::Any;
use std::borrow::Cow;
use std::sync::Arc;

use go_value::{HostCtx, Object, SliceType, Value};
use nh_common::Result;
use nh_config::config_provider::AllProvider;

use crate::page::Pages;
use crate::page_paths::TargetPathDescriptor;
use crate::pagegroup::PagesGroup;

/// Go: `paginatedElement` (a chunk of Pages or of PagesGroup).
#[derive(Clone)]
pub enum PaginatedElement {
    Pages(Pages),
    Groups(PagesGroup),
}

/// Go: `page.Paginator`.
pub struct Paginator {
    pub(crate) paginated_elements: Vec<PaginatedElement>,
    pub(crate) url_factory: Arc<dyn Fn(i64) -> String + Send + Sync>,
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
    // Go: resources/page/pagination.go:Pagers
    pub fn pagers(&self) -> Vec<Arc<Pager>> {
        todo!()
    }
    // Go: resources/page/pagination.go:PagerSize
    pub fn pager_size(&self) -> i64 {
        self.size
    }
    // Go: resources/page/pagination.go:TotalPages
    pub fn total_pages(&self) -> i64 {
        todo!()
    }
    // Go: resources/page/pagination.go:TotalNumberOfElements
    pub fn total_number_of_elements(&self) -> i64 {
        self.total
    }
}

impl Pager {
    // Go: resources/page/pagination.go:PageNumber
    pub fn page_number(&self) -> i64 {
        self.number
    }
    // Go: resources/page/pagination.go:URL
    pub fn url(&self) -> String {
        (self.paginator.url_factory)(self.number)
    }
    // Go: resources/page/pagination.go:Pages
    pub fn pages(&self) -> Pages {
        todo!()
    }
    // Go: resources/page/pagination.go:PageGroups
    pub fn page_groups(&self) -> PagesGroup {
        todo!()
    }
    // Go: resources/page/pagination.go:HasPrev
    pub fn has_prev(&self) -> bool {
        self.number > 1
    }
    // Go: resources/page/pagination.go:Prev
    pub fn prev(&self) -> Option<Arc<Pager>> {
        todo!()
    }
    // Go: resources/page/pagination.go:HasNext
    pub fn has_next(&self) -> bool {
        todo!()
    }
    // Go: resources/page/pagination.go:Next
    pub fn next(&self) -> Option<Arc<Pager>> {
        todo!()
    }
    // Go: resources/page/pagination.go:First
    pub fn first(&self) -> Arc<Pager> {
        todo!()
    }
    // Go: resources/page/pagination.go:Last
    pub fn last(&self) -> Arc<Pager> {
        todo!()
    }
    // Go: resources/page/pagination.go:NumberOfElements
    pub fn number_of_elements(&self) -> i64 {
        todo!()
    }
}

/// Template value of `*page.Pager` (pointer identity: `eq $pag $p` compares pagers).
#[derive(Clone)]
pub struct PagerRef(pub Arc<Pager>);

nh_common::go_methods!(PagerRef {
    "PageNumber" => |p, _c, _a| Ok(Value::int(p.0.page_number())),
    "URL" => |p, _c, _a| Ok(Value::string(p.0.url())),
    "Pages" => |p, _c, _a| Ok(crate::page::pages_to_value(&p.0.pages())),
    "PageGroups" => |p, _c, _a| todo!(),
    "NumberOfElements" => |p, _c, _a| Ok(Value::int(p.0.number_of_elements())),
    "HasPrev" => |p, _c, _a| Ok(Value::Bool(p.0.has_prev())),
    "Prev" => |p, _c, _a| Ok(pager_opt_value(p.0.prev())),
    "HasNext" => |p, _c, _a| Ok(Value::Bool(p.0.has_next())),
    "Next" => |p, _c, _a| Ok(pager_opt_value(p.0.next())),
    "First" => |p, _c, _a| Ok(Value::object(PagerRef(p.0.first()))),
    "Last" => |p, _c, _a| Ok(Value::object(PagerRef(p.0.last()))),
    "Pagers" => |p, _c, _a| Ok(pagers_to_value(&p.0.paginator.pagers())),
    "PagerSize" => |p, _c, _a| Ok(Value::int(p.0.paginator.pager_size())),
    "PageSize" => |p, _c, _a| Ok(Value::int(p.0.paginator.pager_size())),
    "TotalPages" => |p, _c, _a| Ok(Value::int(p.0.paginator.total_pages())),
    "TotalNumberOfElements" => |p, _c, _a| Ok(Value::int(p.0.paginator.total_number_of_elements())),
});

impl Object for PagerRef {
    nh_common::object_basics!("*page.Pager");

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

/// Go: `page.ResolvePagerSize(conf, options...)`.
// Go: resources/page/pagination.go:ResolvePagerSize
pub fn resolve_pager_size(conf: &dyn AllProvider, options: &[Value]) -> Result<i64> {
    todo!()
}

/// Go: `page.Paginate(td, seq, pagerSize)`.
// Go: resources/page/pagination.go:Paginate
pub fn paginate(td: &TargetPathDescriptor, seq: &Value, pager_size: i64) -> Result<Arc<Paginator>> {
    todo!()
}

/// Go: `newPaginationURLFactory(d)` — page 1 -> the node's RelPermalink; page N ->
/// `CreateTargetPaths(d + Addends "/page/N").RelPermalink`.
// Go: resources/page/pagination.go:newPaginationURLFactory
pub fn new_pagination_url_factory(
    d: TargetPathDescriptor,
) -> Arc<dyn Fn(i64) -> String + Send + Sync> {
    todo!()
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: resources/page/pagination.go (417 lines; 19/30 funcs executed)
//   types: PaginatorProvider, PaginatorNotSupportedFunc, Pager, paginatedElement, pagers, Paginator,
//          paginationURLFactory
//    L40-42: (f PaginatorNotSupportedFunc) Paginate(pages any, options ...any) (*Pager, error)
//    L44-46: (f PaginatorNotSupportedFunc) Paginator(options ...any) (*Pager, error)
//    L55-57: (p Pager) String() string
// EX L81-83: (p *Pager) PageNumber() int
// EX L86-88: (p *Pager) URL() string
// EX L92-102: (p *Pager) Pages() Pages
//    L106-116: (p *Pager) PageGroups() PagesGroup
// EX L118-123: (p *Pager) element() paginatedElement
//    L126-149: (p *Pager) page(index int) (Page, error)
//    L152-154: (p *Pager) NumberOfElements() int
// EX L157-159: (p *Pager) HasPrev() bool
// EX L162-167: (p *Pager) Prev() *Pager
// EX L170-172: (p *Pager) HasNext() bool
// EX L175-180: (p *Pager) Next() *Pager
// EX L183-185: (p *Pager) First() *Pager
// EX L188-190: (p *Pager) Last() *Pager
// EX L193-195: (p *Paginator) Pagers() pagers
//    L199-202: (p *Paginator) PageSize() int
// EX L205-207: (p *Paginator) PagerSize() int
// EX L210-212: (p *Paginator) TotalPages() int
//    L215-217: (p *Paginator) TotalNumberOfElements() int
// EX L219-227: splitPages(pages Pages, size int) []paginatedElement
//    L229-270: splitPageGroups(pageGroups PagesGroup, size int) []paginatedElement
// EX L272-288: ResolvePagerSize(conf config.AllProvider, options ...any) (int, error)
// EX L290-314: Paginate(td TargetPathDescriptor, seq any, pagerSize int) (*Paginator, error)
//    L320-364: probablyEqualPageLists(a1 any, a2 any) bool
// EX L366-374: newPaginatorFromPages(pages Pages, size int, urlFactory paginationURLFactory) (*Paginator, error)
//    L376-384: newPaginatorFromPageGroups(pageGroups PagesGroup, size int, urlFactory paginationURLFactory) (*Paginator, error)
// EX L386-404: newPaginator(elements []paginatedElement, total, size int, urlFactory paginationURLFactory) (*Paginator, error)
// EX L406-417: newPaginationURLFactory(d TargetPathDescriptor) paginationURLFactory
// ---------------------------------------------------------------------------
