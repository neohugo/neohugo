//! Port of `resources/page/pages_sort.go`.
//!
//! Owner: Wave B task T12 (page-collections).

//! Go `resources/page/pages_sort.go`. Every page sort is Go's `sort.Stable` (insertion sort +
//! SymMerge), ported with `go_sort::stable_by` so that comparators that are not strict weak
//! orders (`ByParam`'s `<= 0` string test, `nil` params) give Go's order. Collated comparisons use
//! the collator of the language of `p.Site().Current()` — the site being rendered when a list is
//! first computed — locked like Go (`coll.Lock()` for the whole sort in `ByTitle`/`ByLinkTitle`,
//! per comparison in `DefaultPageSort`, the second collator in `ByParam`).

use std::sync::Arc;

use go_time::GoTimeExt;
use go_value::Value;
use nh_common::compare;
use nh_langs::language::Language;

use crate::page::{Page, PageRef, Pages};
use crate::pages_cache::spc;

/// Go `pageBy`: a less function over pages.
pub type PageBy<'a> = &'a dyn Fn(&dyn Page, &dyn Page) -> bool;

/// Go: `getOrdinals(p1, p2)` — the `collections.Order` ordinals, `-1, -1` unless both have one.
// Go: resources/page/pages_sort.go:getOrdinals
fn get_ordinals(p1: &dyn Page, p2: &dyn Page) -> (i64, i64) {
    match (p1.ordinal(), p2.ordinal()) {
        (Some(o1), Some(o2)) => (o1, o2),
        _ => (-1, -1),
    }
}

/// Go: `getWeight0s(p1, p2)` — the `resource.Weight0Provider` weights, `-1, -1` unless both
/// have one.
// Go: resources/page/pages_sort.go:getWeight0s
fn get_weight0s(p1: &dyn Page, p2: &dyn Page) -> (i64, i64) {
    match (p1.weight0(), p2.weight0()) {
        (Some(w1), Some(w2)) => (w1, w2),
        _ => (-1, -1),
    }
}

/// Go: `pageBy.Sort(pages)` — `sort.Stable` with the less function.
// Go: resources/page/pages_sort.go:Sort
pub fn page_by_sort(by: PageBy<'_>, pages: &mut Pages) {
    go_sort::stable_by(pages, |a, b| by(&*a.0, &*b.0));
}

/// The language of the site currently rendered (Go `p.Site().Current().Language()`).
fn current_language(p: &dyn Page) -> Arc<Language> {
    p.site().0.current().0.language()
}

/// Go: `page.DefaultPageSort` — ordinal, weight0, weight (0 last), `Date().Unix()` desc,
/// collated LinkTitle, `compare.LessStrings(PathInfo().Path())`.
// Go: resources/page/pages_sort.go:DefaultPageSort
pub fn default_page_sort(p1: &dyn Page, p2: &dyn Page) -> bool {
    let (o1, o2) = get_ordinals(p1, p2);
    if o1 != o2 && o1 != -1 && o2 != -1 {
        return o1 < o2;
    }
    // Weight0, as by the weight of the taxonomy entrie in the front matter.
    let (w01, w02) = get_weight0s(p1, p2);
    if w01 != w02 && w01 != -1 && w02 != -1 {
        return w01 < w02;
    }

    if p1.weight() == p2.weight() {
        if p1.date().go_unix() == p2.date().go_unix() {
            let c = collator_string_compare_fn(&|p: &dyn Page| p.link_title(), p1, p2);
            if c == 0 {
                // This is the full normalized path, which will contain extension and any language
                // code preserved, which is what we want for sorting.
                return compare::less_strings(
                    p1.path_info().path().as_bytes(),
                    p2.path_info().path().as_bytes(),
                );
            }
            return c < 0;
        }
        return p1.date().go_unix() > p2.date().go_unix();
    }

    if p2.weight() == 0 {
        return true;
    }

    if p1.weight() == 0 {
        return false;
    }

    p1.weight() < p2.weight()
}

/// The page's language (Go `p.Language()`).
fn page_language(p: &dyn Page) -> Arc<Language> {
    match nh_resource::resourcetypes::Resource::language(p) {
        Some(l) => l,
        None => p.site().0.language(),
    }
}

/// Go: `lessPageLanguage` (language weight (0 last), date desc, `compare.Strings(LinkTitle)`,
/// filename).
// Go: resources/page/pages_sort.go:lessPageLanguage
pub fn less_page_language(p1: &dyn Page, p2: &dyn Page) -> bool {
    let (l1, l2) = (page_language(p1), page_language(p2));
    if l1.config.weight == l2.config.weight {
        if p1.date().go_unix() == p2.date().go_unix() {
            let c = compare::strings(p1.link_title().as_bytes(), p2.link_title().as_bytes());
            if c == 0
                && let (Some(f1), Some(f2)) = (p1.file(), p2.file())
            {
                return compare::less_strings(f1.filename().as_bytes(), f2.filename().as_bytes());
            }
            return c < 0;
        }
        return p1.date().go_unix() > p2.date().go_unix();
    }

    if l2.config.weight == 0 {
        return true;
    }

    if l1.config.weight == 0 {
        return false;
    }

    l1.config.weight < l2.config.weight
}

// Go: resources/page/pages_sort.go:lessPageTitle
pub fn less_page_title(p1: &dyn Page, p2: &dyn Page) -> bool {
    collator_string_compare_fn(&|p: &dyn Page| Page::title(p), p1, p2) < 0
}

// Go: resources/page/pages_sort.go:lessPageLinkTitle
pub fn less_page_link_title(p1: &dyn Page, p2: &dyn Page) -> bool {
    collator_string_compare_fn(&|p: &dyn Page| p.link_title(), p1, p2) < 0
}

// Go: resources/page/pages_sort.go:lessPageDate
pub fn less_page_date(p1: &dyn Page, p2: &dyn Page) -> bool {
    p1.date().go_unix() < p2.date().go_unix()
}

// Go: resources/page/pages_sort.go:lessPagePubDate
pub fn less_page_pub_date(p1: &dyn Page, p2: &dyn Page) -> bool {
    p1.publish_date().go_unix() < p2.publish_date().go_unix()
}

/// Go: `Pages.Limit(n)` — the first `n` pages (all when `n` is larger; Go slices `p[0:n]`,
/// which panics for a negative `n`).
// Go: resources/page/pages_sort.go:Limit
pub fn limit(p: &Pages, n: usize) -> Pages {
    if p.len() > n {
        return p[0..n].to_vec();
    }
    p.clone()
}

/// Go: `collatorStringSort(getString)` — `sort.SliceStable` with the first collator of the
/// current site's language, locked for the whole sort.
// Go: resources/page/pages_sort.go:collatorStringSort
fn collator_string_sort(get_string: &dyn Fn(&dyn Page) -> String, p: &mut Pages) {
    if p.is_empty() {
        return;
    }
    // Pages may be a mix of multiple languages, so we need to use the language for the
    // currently rendered Site.
    let lang = current_language(&*p[0].0);
    let coll = lang.collator1();
    let mut g = coll.lock();
    go_sort::stable_by(p, |a, b| {
        g.compare_strings(get_string(&*a.0).as_bytes(), get_string(&*b.0).as_bytes()) < 0
    });
}

/// Go: `collatorStringCompare(getString, p1, p2)` — the first collator of
/// `p1.Site().Current().Language()`, locked for the comparison.
// Go: resources/page/pages_sort.go:collatorStringCompare
pub fn collator_string_compare_fn(
    get_string: &dyn Fn(&dyn Page) -> String,
    p1: &dyn Page,
    p2: &dyn Page,
) -> i32 {
    let (s1, s2) = (get_string(p1), get_string(p2));
    collator_string_compare(p1, &s1, &s2)
}

/// Go: `collatorStringCompare` with the two strings already read: the first collator of
/// `p1.Site().Current().Language()`, locked for the comparison.
pub fn collator_string_compare(p1: &dyn Page, a: &str, b: &str) -> i32 {
    let lang = current_language(p1);
    let mut g = lang.collator1().lock();
    g.compare_strings(a.as_bytes(), b.as_bytes())
}

/// Go: `collatorStringLess(p)` — the SECOND collator of the current site's language (issue
/// 11039), locked until the returned guard is dropped; `less(s1, s2)` is `CompareStrings < 1`
/// (i.e. `<=`).
// Go: resources/page/pages_sort.go:collatorStringLess
pub fn collator_string_less(p: &dyn Page) -> CollatorLess {
    CollatorLess {
        lang: current_language(p),
    }
}

/// The result of [`collator_string_less`]: holds the language whose second collator is locked
/// by [`CollatorLess::lock`].
pub struct CollatorLess {
    lang: Arc<Language>,
}

impl CollatorLess {
    /// Go `coll.Lock()`; `close()` is dropping the guard.
    pub fn lock(&self) -> CollatorLessGuard<'_> {
        CollatorLessGuard(self.lang.collator2().lock())
    }
}

/// A locked second collator.
pub struct CollatorLessGuard<'a>(nh_langs::language::CollatorGuard<'a>);

impl CollatorLessGuard<'_> {
    /// Go: `coll.CompareStrings(s1, s2) < 1`.
    pub fn less(&mut self, s1: &[u8], s2: &[u8]) -> bool {
        self.0.compare_strings(s1, s2) < 1
    }
}

/// Go: `Pages.ByWeight()` (cached: `pageSort.ByWeight`).
// Go: resources/page/pages_sort.go:ByWeight
pub fn by_weight(p: &Pages) -> Pages {
    const KEY: &str = "pageSort.ByWeight";
    let (pages, _) = spc().get(KEY, &|ps| page_by_sort(&default_page_sort, ps), &[p]);
    pages
}

/// Go: `page.SortByDefault(pages)` (in place, stable).
// Go: resources/page/pages_sort.go:SortByDefault
pub fn sort_by_default(pages: &mut Pages) {
    page_by_sort(&default_page_sort, pages);
}

// Go: resources/page/pages_sort.go:ByTitle
pub fn by_title(p: &Pages) -> Pages {
    const KEY: &str = "pageSort.ByTitle";
    let (pages, _) = spc().get(
        KEY,
        &|ps| collator_string_sort(&|p: &dyn Page| Page::title(p), ps),
        &[p],
    );
    pages
}

// Go: resources/page/pages_sort.go:ByLinkTitle
pub fn by_link_title(p: &Pages) -> Pages {
    const KEY: &str = "pageSort.ByLinkTitle";
    let (pages, _) = spc().get(
        KEY,
        &|ps| collator_string_sort(&|p: &dyn Page| p.link_title(), ps),
        &[p],
    );
    pages
}

// Go: resources/page/pages_sort.go:ByDate
pub fn by_date(p: &Pages) -> Pages {
    const KEY: &str = "pageSort.ByDate";
    let (pages, _) = spc().get(KEY, &|ps| page_by_sort(&less_page_date, ps), &[p]);
    pages
}

// Go: resources/page/pages_sort.go:ByPublishDate
pub fn by_publish_date(p: &Pages) -> Pages {
    const KEY: &str = "pageSort.ByPublishDate";
    let (pages, _) = spc().get(KEY, &|ps| page_by_sort(&less_page_pub_date, ps), &[p]);
    pages
}

// Go: resources/page/pages_sort.go:ByExpiryDate
pub fn by_expiry_date(p: &Pages) -> Pages {
    const KEY: &str = "pageSort.ByExpiryDate";
    let exp_date =
        |p1: &dyn Page, p2: &dyn Page| p1.expiry_date().go_unix() < p2.expiry_date().go_unix();
    let (pages, _) = spc().get(KEY, &|ps| page_by_sort(&exp_date, ps), &[p]);
    pages
}

// Go: resources/page/pages_sort.go:ByLastmod
pub fn by_lastmod(p: &Pages) -> Pages {
    const KEY: &str = "pageSort.ByLastmod";
    let date = |p1: &dyn Page, p2: &dyn Page| p1.lastmod().go_unix() < p2.lastmod().go_unix();
    let (pages, _) = spc().get(KEY, &|ps| page_by_sort(&date, ps), &[p]);
    pages
}

/// Go: `Pages.ByLength(ctx)` — by `Len(ctx)` (the rendered content length). Every page
/// implements `resource.LengthProvider`, so the type tests of the Go comparator always pass.
// Go: resources/page/pages_sort.go:ByLength
pub fn by_length(ctx: go_value::HostCtx<'_>, p: &Pages) -> Pages {
    const KEY: &str = "pageSort.ByLength";
    // A content error makes Go's `Len` return 0.
    let length = |p1: &dyn Page, p2: &dyn Page| {
        p1.content_len(ctx).unwrap_or(0) < p2.content_len(ctx).unwrap_or(0)
    };
    let (pages, _) = spc().get(KEY, &|ps| page_by_sort(&length, ps), &[p]);
    pages
}

// Go: resources/page/pages_sort.go:ByLanguage
pub fn by_language(p: &Pages) -> Pages {
    const KEY: &str = "pageSort.ByLanguage";
    let (pages, _) = spc().get(KEY, &|ps| page_by_sort(&less_page_language, ps), &[p]);
    pages
}

/// Go: `page.SortByLanguage(pages)`.
// Go: resources/page/pages_sort.go:SortByLanguage
pub fn sort_by_language(pages: &mut Pages) {
    page_by_sort(&less_page_language, pages);
}

/// Go: `Pages.Reverse()` (cached: `pageSort.Reverse`).
// Go: resources/page/pages_sort.go:Reverse
pub fn reverse(p: &Pages) -> Pages {
    const KEY: &str = "pageSort.Reverse";
    let reverse_func = |pages: &mut Pages| {
        let n = pages.len();
        let (mut i, mut j) = (0usize, n.wrapping_sub(1));
        while n > 0 && i < j {
            pages.swap(i, j);
            i += 1;
            j -= 1;
        }
    };
    let (pages, _) = spc().get(KEY, &reverse_func, &[p]);
    pages
}

/// Go's type switch in `ByParam`'s `isNumeric`.
fn is_numeric(v: &Value) -> bool {
    matches!(v, Value::Int(..) | Value::Uint(..) | Value::Float(..))
}

/// Go: `Pages.ByParam(paramsKey)` (cached per key: `pageSort.ByParam.<key>`). A nil param sorts
/// last (`v1 == nil` is never less); two numbers compare as floats, everything else by the
/// second collator with `CompareStrings < 1`.
// Go: resources/page/pages_sort.go:ByParam
pub fn by_param(p: &Pages, params_key: &Value) -> Pages {
    if p.len() < 2 {
        return p.clone();
    }
    let params_key_str = nh_common::cast::caste::to_string(params_key);
    let key = format!("pageSort.ByParam.{}", params_key_str.to_str_lossy());
    let key_value = Value::String(params_key_str);

    let string_less = collator_string_less(&*p[0].0);
    let guard = std::cell::RefCell::new(string_less.lock());

    let params_key_comparator = |p1: &dyn Page, p2: &dyn Page| -> bool {
        let v1 = p1.param(&key_value).unwrap_or(Value::Invalid);
        let v2 = p2.param(&key_value).unwrap_or(Value::Invalid);

        if v1.is_invalid() {
            return false;
        }

        if v2.is_invalid() {
            return true;
        }

        if is_numeric(&v1) && is_numeric(&v2) {
            return nh_common::cast::caste::to_float64(&v1)
                < nh_common::cast::caste::to_float64(&v2);
        }

        let s1 = nh_common::cast::caste::to_string(&v1);
        let s2 = nh_common::cast::caste::to_string(&v2);

        guard.borrow_mut().less(s1.as_bytes(), s2.as_bytes())
    };

    let (pages, _) = spc().get(&key, &|ps| page_by_sort(&params_key_comparator, ps), &[p]);
    pages
}

/// Go: `Pages.String()` — `Pages(<len>)`.
// Go: resources/page/pages.go:String
pub fn pages_string(p: &Pages) -> String {
    format!("Pages({})", p.len())
}

/// Identity of a page list element as Go compares interface values (`p1[i] != p2[i]`): the
/// same page behind the same wrapper.
pub fn page_ref_eq(a: &PageRef, b: &PageRef) -> bool {
    Arc::ptr_eq(&a.0, &b.0)
        || (a.0.page_id() == b.0.page_id()
            && a.0.weight0() == b.0.weight0()
            && a.0.ordinal() == b.0.ordinal()
            && a.0.tpl_type_name() == b.0.tpl_type_name())
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: resources/page/pages_sort.go (431 lines; 9/20 funcs executed)
//   types: pageSorter, pageBy
// OK L44-55: getOrdinals(p1, p2 Page) (int, int)
// OK L57-68: getWeight0s(p1, p2 Page) (int, int)
// OK L71-77: (by pageBy) Sort(pages Pages)
// OK L160-160: (ps *pageSorter) Len() int
// OK L161-161: (ps *pageSorter) Swap(i, j int)
// OK L164-164: (ps *pageSorter) Less(i, j int) bool
// OK L167-172: (p Pages) Limit(n int) Pages
// OK L220-224: (p Pages) ByWeight() Pages
// OK L227-229: SortByDefault(pages Pages)
// OK L236-242: (p Pages) ByTitle() Pages
// OK L249-255: (p Pages) ByLinkTitle() Pages
// OK L262-268: (p Pages) ByDate() Pages
// OK L275-281: (p Pages) ByPublishDate() Pages
// OK L288-298: (p Pages) ByExpiryDate() Pages
// OK L305-315: (p Pages) ByLastmod() Pages
// OK L322-343: (p Pages) ByLength(ctx context.Context) Pages
// OK L350-356: (p Pages) ByLanguage() Pages
// OK L359-361: SortByLanguage(pages Pages)
// OK L368-380: (p Pages) Reverse() Pages
// OK L387-431: (p Pages) ByParam(paramsKey any) Pages
// ---------------------------------------------------------------------------
