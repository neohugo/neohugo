//! Ports of the Go unit tests of the T12 modules: resources/page/pages_sort_test.go,
//! pagination_test.go, pagegroup_test.go, pages_prev_next_test.go, pages_sort_search_test.go,
//! pages_cache_test.go, pages_test.go, pages_related_test.go and navigation/menu_cache_test.go,
//! over a port of their `testPage` (resources/page/testhelpers_test.go). (The
//! related/inverted_index_test.go tables are unit tests in `src/related.rs`, which needs the
//! index internals; hugolib/menu_test.go's sites are built by the menus oracle.)

use std::any::Any;
use std::borrow::Cow;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use go_time::GoTimeExt;
use go_value::{GoString, HostCtx, Map, MapType, Object, Time, Value};
use nh_common::Result;
use nh_common::object::GoResult;
use nh_common::paths::pathparser::{Path, PathParser};
use nh_config::common_config::SitemapConfig;
use nh_config::neohugo::neohugo::HugoInfoConfig;
use nh_helpers::source::file_info::File;
use nh_media::media::media_type::MediaType;
use nh_page::page::{Page, PageRef, Pages, pages_from_value, pages_to_value};
use nh_page::pagegroup::{PageGroup, PagesGroup};
use nh_page::pages_related::RelatedDocsHandler;
use nh_page::related::{Config as RelatedConfig, IndexConfig, Keyword};
use nh_page::site::{SiteRef, new_dummy_hugo_site};
use nh_resource::resourcetypes::{Resource, Resources};

static IDS: AtomicU64 = AtomicU64::new(1);

struct Env;

impl HugoInfoConfig for Env {
    fn environment(&self) -> String {
        "production".into()
    }
    fn running(&self) -> bool {
        false
    }
    fn working_dir(&self) -> String {
        String::new()
    }
    fn is_multihost(&self) -> bool {
        false
    }
    fn is_multilingual(&self) -> bool {
        false
    }
}

fn site() -> SiteRef {
    static SITE: std::sync::OnceLock<SiteRef> = std::sync::OnceLock::new();
    SITE.get_or_init(|| SiteRef(new_dummy_hugo_site(Arc::new(Env)).unwrap()))
        .clone()
}

fn related_handler() -> Arc<RelatedDocsHandler> {
    static H: std::sync::OnceLock<Arc<RelatedDocsHandler>> = std::sync::OnceLock::new();
    H.get_or_init(|| RelatedDocsHandler::new(RelatedConfig::default_config()))
        .clone()
}

#[derive(Clone)]
struct Data {
    description: String,
    title: String,
    link_title: String,
    section: String,
    content: String,
    fuzzy_word_count: i64,
    path: String,
    date: Time,
    last_mod: Time,
    expiry_date: Time,
    pub_date: Time,
    weight: i64,
    params: Map,
}

/// Go `testPage` (testhelpers_test.go).
struct TestPage {
    id: u64,
    path_info: Arc<Path>,
    d: Mutex<Data>,
}

fn path_info(p: &str) -> Arc<Path> {
    let pp = PathParser {
        is_content_ext: Some(Arc::new(|_: &str| true)),
        ..Default::default()
    };
    Arc::new(pp.parse("content", p))
}

/// Go `newTestPage()` (file `/a/b/c.md`).
fn new_test_page() -> Arc<TestPage> {
    Arc::new(TestPage {
        id: IDS.fetch_add(1, Ordering::SeqCst),
        path_info: path_info("/a/b/c.md"),
        d: Mutex::new(Data {
            description: String::new(),
            title: String::new(),
            link_title: String::new(),
            section: String::new(),
            content: String::new(),
            fuzzy_word_count: 0,
            path: String::new(),
            date: Time::zero(),
            last_mod: Time::zero(),
            expiry_date: Time::zero(),
            pub_date: Time::zero(),
            weight: 0,
            params: Map::new(MapType::Params),
        }),
    })
}

fn tp(p: &PageRef) -> &TestPage {
    p.0.as_any().downcast_ref::<TestPage>().unwrap()
}

fn with<R>(p: &PageRef, f: impl FnOnce(&mut Data) -> R) -> R {
    f(&mut tp(p).d.lock().unwrap())
}

fn r(p: &Arc<TestPage>) -> PageRef {
    PageRef(p.clone())
}

impl Resource for TestPage {
    fn resource_type(&self) -> String {
        "page".into()
    }
    fn media_type(&self) -> MediaType {
        unimplemented!()
    }
    fn permalink(&self) -> String {
        unimplemented!()
    }
    fn rel_permalink(&self) -> String {
        unimplemented!()
    }
    fn data(&self) -> Value {
        unimplemented!()
    }
    fn name(&self) -> String {
        panic!("testpage: not implemented")
    }
    fn title(&self) -> String {
        self.d.lock().unwrap().title.clone()
    }
    fn params(&self) -> Arc<Map> {
        Arc::new(self.d.lock().unwrap().params.clone())
    }
    fn key(&self) -> String {
        unimplemented!()
    }
    fn tpl_type_name(&self) -> Cow<'_, str> {
        Cow::Borrowed("*page.testPage")
    }
    fn tpl_has_method(&self, name: &str) -> bool {
        matches!(name, "Weight" | "FuzzyWordCount" | "Type" | "Section")
    }
    fn tpl_call_method(
        &self,
        _ctx: HostCtx<'_>,
        name: &str,
        _args: &[Value],
    ) -> Option<GoResult<Value>> {
        let d = self.d.lock().unwrap();
        Some(Ok(match name {
            "Weight" => Value::int(d.weight),
            "FuzzyWordCount" => Value::int(d.fuzzy_word_count),
            // Go testPage.Type() is the section.
            "Type" | "Section" => Value::string(d.section.clone()),
            _ => return None,
        }))
    }
    fn to_value(self: Arc<Self>) -> Value {
        PageRef(self).to_value()
    }
    fn as_any_arc(self: Arc<Self>) -> Arc<dyn Any + Send + Sync> {
        self
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl Page for TestPage {
    fn page_id(&self) -> u64 {
        self.id
    }
    fn unwrap_page(self: Arc<Self>) -> Arc<dyn Page> {
        self
    }
    fn kind(&self) -> String {
        String::new()
    }
    fn title(&self) -> String {
        self.d.lock().unwrap().title.clone()
    }
    // Go: resources/page/testhelpers_test.go:LinkTitle
    fn link_title(&self) -> String {
        let d = self.d.lock().unwrap();
        if d.link_title.is_empty() {
            if d.title.is_empty() {
                return d.path.clone();
            }
            return d.title.clone();
        }
        d.link_title.clone()
    }
    fn description(&self) -> String {
        self.d.lock().unwrap().description.clone()
    }
    fn weight(&self) -> i64 {
        self.d.lock().unwrap().weight
    }
    fn date(&self) -> Time {
        self.d.lock().unwrap().date.clone()
    }
    fn lastmod(&self) -> Time {
        self.d.lock().unwrap().last_mod.clone()
    }
    fn publish_date(&self) -> Time {
        self.d.lock().unwrap().pub_date.clone()
    }
    fn expiry_date(&self) -> Time {
        self.d.lock().unwrap().expiry_date.clone()
    }
    fn is_home(&self) -> bool {
        false
    }
    fn is_node(&self) -> bool {
        false
    }
    fn is_page(&self) -> bool {
        true
    }
    fn is_section(&self) -> bool {
        false
    }
    fn section(&self) -> String {
        self.d.lock().unwrap().section.clone()
    }
    fn page_type(&self) -> String {
        self.section()
    }
    fn layout(&self) -> String {
        String::new()
    }
    fn lang(&self) -> String {
        String::new()
    }
    fn path(&self) -> String {
        self.d.lock().unwrap().path.clone()
    }
    fn path_info(&self) -> Arc<Path> {
        self.path_info.clone()
    }
    fn slug(&self) -> String {
        String::new()
    }
    fn draft(&self) -> bool {
        false
    }
    fn aliases(&self) -> Vec<String> {
        unimplemented!()
    }
    fn keywords(&self) -> Vec<String> {
        unimplemented!()
    }
    fn bundle_type(&self) -> String {
        unimplemented!()
    }
    fn sitemap(&self) -> SitemapConfig {
        SitemapConfig::default()
    }
    // Go: resources/page/testhelpers_test.go:Param
    fn param(&self, key: &Value) -> Result<Value> {
        let params = self.d.lock().unwrap().params.clone();
        nh_resource::params::param(&params, None, key)
    }
    fn page_params(&self) -> Arc<Map> {
        Resource::params(self)
    }
    fn site(&self) -> SiteRef {
        site()
    }
    fn file(&self) -> Option<Arc<File>> {
        None
    }
    fn parent(&self) -> Option<PageRef> {
        None
    }
    fn pages(&self) -> Pages {
        unimplemented!()
    }
    fn regular_pages(&self) -> Pages {
        unimplemented!()
    }
    fn resources(&self) -> Resources {
        unimplemented!()
    }
    fn output_formats(&self) -> nh_page::page_outputformat::OutputFormats {
        unimplemented!()
    }
    fn all_translations(&self) -> Pages {
        unimplemented!()
    }
    fn translations(&self) -> Pages {
        unimplemented!()
    }
    fn plain(&self, _ctx: HostCtx<'_>) -> Result<GoString> {
        unimplemented!()
    }
    // Go: resources/page/testhelpers_test.go:Len
    fn content_len(&self, _ctx: HostCtx<'_>) -> Result<i64> {
        Ok(self.d.lock().unwrap().content.len() as i64)
    }
    fn render_string(&self, _ctx: HostCtx<'_>, _args: &[Value]) -> Result<Value> {
        unimplemented!()
    }
    // Go: resources/page/testhelpers_test.go:RelatedKeywords
    fn related_keywords(&self, cfg: &IndexConfig) -> Result<Vec<Keyword>> {
        let v = self.param(&Value::string(cfg.name.clone()))?;
        cfg.to_keywords(&v)
    }
    fn ref_(&self, _args: &Map) -> Result<String> {
        unimplemented!()
    }
    fn rel_ref(&self, _args: &Map) -> Result<String> {
        unimplemented!()
    }
    fn related_docs_handler(&self) -> Option<Arc<RelatedDocsHandler>> {
        Some(related_handler())
    }
}

fn now() -> Time {
    go_time::unix(1_750_000_000, 0)
}

fn hours(t: &Time, h: i64) -> Time {
    t.add(go_time::Duration(h * 3_600_000_000_000))
}

/// Go `createSortTestPages(num)`.
fn create_sort_test_pages(num: usize) -> Pages {
    (0..num)
        .map(|i| {
            let p = new_test_page();
            {
                let mut d = p.d.lock().unwrap();
                d.path = format!("/x/y/p{i}.md");
                d.title = format!("Title {}", i % num.div_ceil(2).max(1));
                let mut nested = Map::new(MapType::StringAny);
                nested.insert("nested", Value::string(format!("xyz{}", 100 - i as i64)));
                d.params.insert("arbitrarily", Value::map(nested));
                d.fuzzy_word_count = i as i64;
                d.weight = if i % 2 == 0 { 10 } else { 5 };
                d.description = "initial".into();
            }
            r(&p)
        })
        .collect()
}

/// Go `createTestPages(num)`.
fn create_test_pages(num: usize) -> Pages {
    (0..num)
        .map(|i| {
            let p = new_test_page();
            {
                let mut d = p.d.lock().unwrap();
                d.path = format!("/x/y/z/p{i}.md");
                d.weight = if i % 2 == 0 { 10 } else { 5 };
                d.fuzzy_word_count = i as i64 + 2;
            }
            r(&p)
        })
        .collect()
}

/// Go `setSortVals(dates, titles, weights, pages)`.
fn set_sort_vals(dates: [Time; 4], titles: [&str; 4], weights: [i64; 4], pages: &Pages) {
    for i in 0..dates.len() {
        let title = titles[i].to_string();
        with(&pages[i], |d| {
            d.date = dates[i].clone();
            d.last_mod = dates[i].clone();
            d.weight = weights[i];
            d.title = title.clone();
        });
        with(&pages[dates.len() - 1 - i], |d| {
            // make sure we compare apples and ... apples ...
            d.link_title = format!("{title}l");
            d.pub_date = dates[i].clone();
            d.expiry_date = dates[i].clone();
            d.content = format!("{}_content", titles[i]);
        });
    }
    let last_last_mod = pages[2].0.lastmod();
    let lm1 = pages[1].0.lastmod();
    with(&pages[2], |d| d.last_mod = lm1);
    with(&pages[1], |d| d.last_mod = last_last_mod);

    for p in pages {
        with(p, |d| d.content = String::new());
    }
}

fn same(a: &PageRef, b: &PageRef) -> bool {
    a.0.page_id() == b.0.page_id()
}

fn same_list(a: &Pages, b: &Pages) -> bool {
    a.len() == b.len() && a.iter().zip(b).all(|(x, y)| same(x, y))
}

// ---- pages_sort_test.go ----

// Go: resources/page/pages_sort_test.go:TestDefaultSort
#[test]
fn default_sort() {
    let d1 = now();
    let d2 = hours(&d1, -1);
    let d3 = hours(&d1, -2);
    let d4 = hours(&d1, -3);

    let mut p = create_sort_test_pages(4);

    // first by weight
    set_sort_vals(
        [d1.clone(), d2.clone(), d3.clone(), d4.clone()],
        ["b", "a", "c", "d"],
        [4, 3, 2, 1],
        &p,
    );
    nh_page::pages_sort::sort_by_default(&mut p);
    assert_eq!(p[0].0.weight(), 1);

    // Consider zero weight, issue #2673
    set_sort_vals(
        [d1.clone(), d2.clone(), d3.clone(), d4.clone()],
        ["b", "a", "d", "c"],
        [0, 0, 0, 1],
        &p,
    );
    nh_page::pages_sort::sort_by_default(&mut p);
    assert_eq!(p[0].0.weight(), 1);

    // next by date
    set_sort_vals(
        [d3.clone(), d4.clone(), d1.clone(), d2.clone()],
        ["a", "b", "c", "d"],
        [1, 1, 1, 1],
        &p,
    );
    nh_page::pages_sort::sort_by_default(&mut p);
    assert!(p[0].0.date().go_equal(&d1));

    // finally by link title
    set_sort_vals(
        [d3.clone(), d3.clone(), d3.clone(), d3.clone()],
        ["b", "c", "a", "d"],
        [1, 1, 1, 1],
        &p,
    );
    nh_page::pages_sort::sort_by_default(&mut p);
    assert_eq!(p[0].0.link_title(), "al");
    assert_eq!(p[1].0.link_title(), "bl");
    assert_eq!(p[2].0.link_title(), "cl");
}

// Go: resources/page/pages_sort_test.go:TestSortByLinkTitle
#[test]
fn sort_by_link_title() {
    let pages = create_sort_test_pages(6);
    for (i, p) in pages.iter().enumerate() {
        with(p, |d| {
            if i < 5 {
                d.title = format!("title{i}");
            }
            if i > 2 {
                d.link_title = format!("linkTitle{i}");
            }
        });
    }
    // Go shuffles randomly; a fixed permutation here.
    let pages: Pages = [3, 0, 5, 1, 4, 2]
        .iter()
        .map(|&i| pages[i].clone())
        .collect();
    let bylt = nh_page::pages_sort::by_link_title(&pages);
    for (i, p) in bylt.iter().enumerate() {
        if i < 3 {
            assert_eq!(p.0.link_title(), format!("linkTitle{}", i + 3));
        } else {
            assert_eq!(p.0.link_title(), format!("title{}", i - 3));
        }
    }
}

// Go: resources/page/pages_sort_test.go:TestSortByN
#[test]
fn sort_by_n() {
    use nh_page::pages_sort as ps;
    let d1 = now();
    let d2 = hours(&d1, -2);
    let d3 = hours(&d1, -10);
    let d4 = hours(&d1, -20);

    let p = create_sort_test_pages(4);
    type Case = (fn(&Pages) -> Pages, Box<dyn Fn(&Pages) -> bool>);
    let d3c = d3.clone();
    let d4a = d4.clone();
    let d4b = d4.clone();
    let d4c = d4.clone();
    let cases: Vec<Case> = vec![
        (ps::by_weight, Box::new(|p: &Pages| p[0].0.weight() == 1)),
        (
            ps::by_title,
            Box::new(|p: &Pages| Page::title(&*p[0].0) == "ab"),
        ),
        (
            ps::by_link_title,
            Box::new(|p: &Pages| p[0].0.link_title() == "abl"),
        ),
        (
            ps::by_date,
            Box::new(move |p: &Pages| p[0].0.date().go_equal(&d4a)),
        ),
        (
            ps::by_publish_date,
            Box::new(move |p: &Pages| p[0].0.publish_date().go_equal(&d4b)),
        ),
        (
            ps::by_expiry_date,
            Box::new(move |p: &Pages| p[0].0.expiry_date().go_equal(&d4c)),
        ),
        (
            ps::by_lastmod,
            Box::new(move |p: &Pages| p[1].0.lastmod().go_equal(&d3c)),
        ),
        (
            |p: &Pages| ps::by_length(&(), p),
            Box::new(|p: &Pages| {
                p[0].0.content_len(&()).unwrap() == tp(&p[0]).d.lock().unwrap().content.len() as i64
            }),
        ),
    ];
    for (i, (sort_func, assert_func)) in cases.into_iter().enumerate() {
        set_sort_vals(
            [d1.clone(), d2.clone(), d3.clone(), d4.clone()],
            ["b", "ab", "cde", "fg"],
            [0, 3, 2, 1],
            &p,
        );
        nh_page::page::clear();
        let sorted = sort_func(&p);
        assert!(assert_func(&sorted), "[{i}] sort error");
    }
}

// Go: resources/page/pages_sort_test.go:TestLimit
#[test]
fn limit() {
    let p = create_sort_test_pages(10);
    let first_five = nh_page::pages_sort::limit(&p, 5);
    assert_eq!(first_five.len(), 5);
    for i in 0..5 {
        assert!(same(&first_five[i], &p[i]));
    }
    assert!(same_list(&nh_page::pages_sort::limit(&p, 10), &p));
    assert!(same_list(&nh_page::pages_sort::limit(&p, 11), &p));
}

// Go: resources/page/pages_sort_test.go:TestPageSortReverse
#[test]
fn page_sort_reverse() {
    let p1 = create_sort_test_pages(10);
    assert_eq!(tp(&p1[0]).d.lock().unwrap().fuzzy_word_count, 0);
    assert_eq!(tp(&p1[9]).d.lock().unwrap().fuzzy_word_count, 9);
    let p2 = nh_page::pages_sort::reverse(&p1);
    assert_eq!(tp(&p2[0]).d.lock().unwrap().fuzzy_word_count, 9);
    assert_eq!(tp(&p2[9]).d.lock().unwrap().fuzzy_word_count, 0);
    // cached
    assert!(nh_page::pages_cache::pages_equal(
        &p2,
        &nh_page::pages_sort::reverse(&p1)
    ));
}

fn param_of(p: &PageRef, k: &str) -> Value {
    p.0.param(&Value::string(k)).unwrap()
}

// Go: resources/page/pages_sort_test.go:TestPageSortByParam
#[test]
fn page_sort_by_param() {
    let k = "arbitrarily.nested";
    let unsorted = create_sort_test_pages(10);
    with(&unsorted[9], |d| {
        d.params.entries.remove(b"arbitrarily".as_slice());
    });

    let first_set_value = param_of(&unsorted[0], k);
    let second_set_value = param_of(&unsorted[1], k);
    let last_set_value = param_of(&unsorted[8], k);
    let unset_value = param_of(&unsorted[9], k);

    assert_eq!(
        first_set_value.as_go_string().unwrap().as_bytes(),
        b"xyz100"
    );
    assert_eq!(
        second_set_value.as_go_string().unwrap().as_bytes(),
        b"xyz99"
    );
    assert_eq!(last_set_value.as_go_string().unwrap().as_bytes(), b"xyz92");
    assert!(unset_value.is_invalid());

    let sorted = nh_page::pages_sort::by_param(&unsorted, &Value::string(k));
    let s = |i: usize| param_of(&sorted[i], k);
    let gs = |v: &Value| v.as_go_string().map(|s| s.to_vec());
    assert_eq!(gs(&s(0)), gs(&first_set_value));
    assert_eq!(gs(&s(8)), gs(&second_set_value));
    assert_eq!(gs(&s(1)), gs(&last_set_value));
    assert!(s(9).is_invalid());
}

// Go: resources/page/pages_sort_test.go:TestPageSortByParamNumeric
#[test]
fn page_sort_by_param_numeric() {
    let k = "arbitrarily.nested";
    let n = 10;
    let unsorted = create_sort_test_pages(n);
    for (i, p) in unsorted.iter().enumerate() {
        let v = if i % 2 == 0 {
            Value::float64(100.0 - i as f64)
        } else {
            Value::int(100 - i as i64)
        };
        with(p, |d| {
            let mut nested = Map::new(MapType::StringAny);
            nested.insert("nested", v);
            d.params = Map::new(MapType::Params);
            d.params.insert("arbitrarily", Value::map(nested));
        });
    }
    with(&unsorted[9], |d| {
        d.params.entries.remove(b"arbitrarily".as_slice());
    });
    let num = |v: &Value| nh_common::cast::caste::to_float64(v);

    assert_eq!(num(&param_of(&unsorted[0], k)), 100.0);
    assert_eq!(num(&param_of(&unsorted[1], k)), 99.0);
    assert_eq!(num(&param_of(&unsorted[8], k)), 92.0);
    assert!(param_of(&unsorted[9], k).is_invalid());

    let sorted = nh_page::pages_sort::by_param(&unsorted, &Value::string(k));
    assert_eq!(num(&param_of(&sorted[0], k)), 92.0);
    assert_eq!(num(&param_of(&sorted[1], k)), 93.0);
    assert_eq!(num(&param_of(&sorted[8], k)), 100.0);
    assert!(param_of(&sorted[9], k).is_invalid());
}

// ---- pagination_test.go ----

fn url_factory() -> nh_page::pagination::PaginationUrlFactory {
    Arc::new(|page: i64| format!("page/{page}/"))
}

fn group_by(p: &Pages, key: &str, order: &str) -> PagesGroup {
    nh_page::pagegroup::group_by(p, key, &[order.to_string()]).unwrap()
}

// Go: resources/page/pagination_test.go:TestSplitPages
#[test]
fn split_pages() {
    let pages = create_test_pages(21);
    let chunks = nh_page::pagination::split_pages(&pages, 5);
    assert_eq!(chunks.len(), 5);
    for c in chunks.iter().take(4) {
        assert_eq!(c.len(), 5);
    }
    assert_eq!(chunks[4].len(), 1);
}

// Go: resources/page/pagination_test.go:TestSplitPageGroups
#[test]
fn split_page_groups() {
    use nh_page::pagination::PaginatedElement;
    let pages = create_test_pages(21);
    let groups = group_by(&pages, "Weight", "desc");
    let chunks = nh_page::pagination::split_page_groups(&groups, 5);
    assert_eq!(chunks.len(), 5);

    let PaginatedElement::Groups(first) = &chunks[0] else {
        panic!("Excepted PageGroup")
    };
    assert_eq!(nh_page::pagegroup::pages_group_len(first), 5);
    for pg in first {
        // first group 10 in weight
        assert!(matches!(pg.key, Value::Int(10, _)));
        for p in &pg.pages {
            assert_eq!(tp(p).d.lock().unwrap().fuzzy_word_count % 2, 0); // magic test
        }
    }

    let PaginatedElement::Groups(last) = &chunks[4] else {
        panic!("Excepted PageGroup")
    };
    assert_eq!(nh_page::pagegroup::pages_group_len(last), 1);
    for pg in last {
        // last should have 5 in weight
        assert!(matches!(pg.key, Value::Int(5, _)));
        for p in &pg.pages {
            assert_ne!(tp(p).d.lock().unwrap().fuzzy_word_count % 2, 0); // magic test
        }
    }
}

fn do_test_pages(paginator: &Arc<nh_page::pagination::Paginator>) {
    let paginator_pages = paginator.pagers();
    assert_eq!(paginator_pages.len(), 5);
    assert_eq!(paginator.total_number_of_elements(), 21);
    assert_eq!(paginator.pager_size(), 5);
    assert_eq!(paginator.total_pages(), 5);

    let first = &paginator_pages[0];
    assert_eq!(first.url(), "page/1/");
    assert!(Arc::ptr_eq(&first.first(), first));
    assert!(first.has_next());
    assert!(Arc::ptr_eq(&first.next().unwrap(), &paginator_pages[1]));
    assert!(!first.has_prev());
    assert!(first.prev().is_none());
    assert_eq!(first.number_of_elements(), 5);
    assert_eq!(first.page_number(), 1);

    let third = &paginator_pages[2];
    assert!(third.has_next());
    assert!(third.has_prev());
    assert!(Arc::ptr_eq(&third.prev().unwrap(), &paginator_pages[1]));

    let last = &paginator_pages[4];
    assert_eq!(last.url(), "page/5/");
    assert!(Arc::ptr_eq(&last.last(), last));
    assert!(!last.has_next());
    assert!(last.next().is_none());
    assert!(last.has_prev());
    assert_eq!(last.number_of_elements(), 1);
    assert_eq!(last.page_number(), 5);
}

// Go: resources/page/pagination_test.go:TestPager
#[test]
fn pager() {
    use nh_page::pagination::{new_paginator_from_page_groups, new_paginator_from_pages};
    let pages = create_test_pages(21);
    let groups = group_by(&pages, "Weight", "desc");

    assert!(new_paginator_from_pages(&pages, -1, url_factory()).is_err());
    assert!(new_paginator_from_page_groups(&groups, -1, url_factory()).is_err());

    let pag = new_paginator_from_pages(&pages, 5, url_factory()).unwrap();
    do_test_pages(&pag);
    let first = pag.pagers()[0].first();
    assert_eq!(first.string(), "Pager 1");
    assert!(!first.pages().is_empty());
    assert!(first.page_groups().is_empty());

    let pag = new_paginator_from_page_groups(&groups, 5, url_factory()).unwrap();
    do_test_pages(&pag);
    let first = pag.pagers()[0].first();
    assert!(!first.page_groups().is_empty());
    assert!(first.pages().is_empty());
}

fn do_test_pager_no_pages(paginator: &Arc<nh_page::pagination::Paginator>) {
    let paginator_pages = paginator.pagers();
    assert_eq!(paginator_pages.len(), 1);
    assert_eq!(paginator.total_number_of_elements(), 0);
    assert_eq!(paginator.pager_size(), 5);
    assert_eq!(paginator.total_pages(), 0);

    // pageOne should be nothing but the first
    let page_one = &paginator_pages[0];
    let _ = page_one.first();
    assert!(!page_one.has_next());
    assert!(!page_one.has_prev());
    assert!(page_one.next().is_none());
    assert_eq!(page_one.paginator.pagers().len(), 1);
    assert_eq!(page_one.pages().len(), 0);
    assert_eq!(page_one.number_of_elements(), 0);
    assert_eq!(page_one.paginator.total_number_of_elements(), 0);
    assert_eq!(page_one.paginator.total_pages(), 0);
    assert_eq!(page_one.page_number(), 1);
    assert_eq!(page_one.paginator.pager_size(), 5);
}

// Go: resources/page/pagination_test.go:TestPagerNoPages
#[test]
fn pager_no_pages() {
    use nh_page::pagination::{new_paginator_from_page_groups, new_paginator_from_pages};
    let pages = create_test_pages(0);
    let groups = group_by(&pages, "Weight", "desc");

    let paginator = new_paginator_from_pages(&pages, 5, url_factory()).unwrap();
    do_test_pager_no_pages(&paginator);
    let first = paginator.pagers()[0].first();
    assert!(first.page_groups().is_empty());
    assert!(first.pages().is_empty());

    let paginator = new_paginator_from_page_groups(&groups, 5, url_factory()).unwrap();
    do_test_pager_no_pages(&paginator);
    let first = paginator.pagers()[0].first();
    assert!(first.page_groups().is_empty());
    assert!(first.pages().is_empty());
}

fn groups_value(g: &PagesGroup) -> Value {
    nh_page::pagegroup::pages_group_to_value(g)
}

// Go: resources/page/pagination_test.go:TestProbablyEqualPageLists
#[test]
fn probably_equal_page_lists() {
    use nh_page::pagination::probably_equal_page_lists;
    let five_pages = pages_to_value(&create_test_pages(5));
    let zero_pages = pages_to_value(&create_test_pages(0));
    let zero_by_weight = groups_value(&group_by(&create_test_pages(0), "Weight", "asc"));
    let five_by_weight = groups_value(&group_by(&create_test_pages(5), "Weight", "asc"));
    let nine_by_weight = groups_value(&group_by(&create_test_pages(9), "Weight", "asc"));

    let cases: Vec<(Value, Value, bool)> = vec![
        (Value::Invalid, Value::Invalid, true),
        (Value::string("a"), Value::string("b"), true),
        (Value::string("a"), five_pages.clone(), false),
        (five_pages.clone(), Value::string("a"), false),
        (
            five_pages.clone(),
            pages_to_value(&create_test_pages(2)),
            false,
        ),
        (five_pages.clone(), five_pages.clone(), true),
        (zero_pages.clone(), zero_pages.clone(), true),
        (five_by_weight.clone(), five_by_weight.clone(), true),
        (zero_by_weight.clone(), five_by_weight.clone(), false),
        (zero_by_weight.clone(), zero_by_weight.clone(), true),
        (five_by_weight.clone(), five_pages.clone(), false),
        (five_by_weight.clone(), nine_by_weight.clone(), false),
    ];
    for (i, (v1, v2, expect)) in cases.into_iter().enumerate() {
        assert_eq!(probably_equal_page_lists(&v1, &v2), expect, "[{i}]");
    }
}

// Go: resources/page/pagination_test.go:TestPaginationPage
#[test]
fn pagination_page() {
    use nh_page::pagination::{new_paginator_from_page_groups, new_paginator_from_pages};
    let five_pages = create_test_pages(7);
    let five_pages_fuzzy_word_count = group_by(&create_test_pages(7), "FuzzyWordCount", "asc");

    let p1 = new_paginator_from_pages(&five_pages, 2, url_factory()).unwrap();
    let p2 =
        new_paginator_from_page_groups(&five_pages_fuzzy_word_count, 2, url_factory()).unwrap();

    let f1 = p1.pagers()[0].first();
    let f2 = p2.pagers()[0].first();

    let page11 = f1.page(1).unwrap();
    assert!(f1.page(3).is_none());
    let page21 = f2.page(1).unwrap();
    assert!(f2.page(3).is_none());

    assert_eq!(tp(&page11).d.lock().unwrap().fuzzy_word_count, 3);
    assert_eq!(tp(&page21).d.lock().unwrap().fuzzy_word_count, 3);
}

// ---- pagegroup_test.go ----

const PAGE_GROUP_TEST_SOURCES: [(&str, i64, &str, &str); 5] = [
    ("/section1/testpage1.md", 3, "2012-04-06", "foo"),
    ("/section1/testpage2.md", 3, "2012-01-01", "bar"),
    ("/section1/testpage3.md", 2, "2012-04-06", "foo"),
    ("/section2/testpage4.md", 1, "2012-03-02", "bar"),
    // date might also be a full datetime:
    ("/section2/testpage5.md", 1, "2012-04-06T00:00:00Z", "baz"),
];

/// Go `preparePageGroupTestPages`.
fn prepare_page_group_test_pages() -> Pages {
    PAGE_GROUP_TEST_SOURCES
        .iter()
        .map(|(path, weight, date, param)| {
            let p = new_test_page();
            let t = nh_common::cast::time::to_time(&Value::string(*date));
            {
                let mut d = p.d.lock().unwrap();
                d.path = path.to_string();
                d.section = path
                    .trim_start_matches('/')
                    .split('/')
                    .next()
                    .unwrap()
                    .to_string();
                d.weight = *weight;
                d.date = t.clone();
                d.pub_date = t.clone();
                d.expiry_date = t.clone();
                d.last_mod = t.add_date(3, 0, 0);
                d.params.insert("custom_param", Value::string(*param));
                d.params.insert("custom_date", Value::Time(t.clone()));
                d.params.insert("custom_string_date", Value::string(*date));
                let mut obj = Map::new(MapType::StringAny);
                obj.insert("param", Value::string(*param));
                obj.insert("date", Value::Time(t.clone()));
                obj.insert("string_date", Value::string(*date));
                d.params.insert("custom_object", Value::map(obj));
            }
            r(&p)
        })
        .collect()
}

fn check_groups(got: &PagesGroup, expect: &[(Value, Vec<usize>)], pages: &Pages) {
    assert_eq!(got.len(), expect.len());
    for (g, (k, idx)) in got.iter().zip(expect) {
        assert!(
            nh_page::pagegroup::group_key_eq(&g.key, k),
            "key {:?}",
            g.key.go_type_name()
        );
        let want: Pages = idx.iter().map(|&i| pages[i].clone()).collect();
        assert!(same_list(&g.pages, &want));
    }
}

fn k(s: &str) -> Value {
    Value::string(s)
}

// Go: resources/page/pagegroup_test.go:TestGroupByWithFieldNameArg etc.
#[test]
fn group_by_tests() {
    let pages = prepare_page_group_test_pages();
    check_groups(
        &nh_page::pagegroup::group_by(&pages, "Weight", &[]).unwrap(),
        &[
            (Value::int(1), vec![3, 4]),
            (Value::int(2), vec![2]),
            (Value::int(3), vec![0, 1]),
        ],
        &pages,
    );
    check_groups(
        &nh_page::pagegroup::group_by(&pages, "Type", &[]).unwrap(),
        &[(k("section1"), vec![0, 1, 2]), (k("section2"), vec![3, 4])],
        &pages,
    );
    check_groups(
        &nh_page::pagegroup::group_by(&pages, "Section", &[]).unwrap(),
        &[(k("section1"), vec![0, 1, 2]), (k("section2"), vec![3, 4])],
        &pages,
    );
    let desc = group_by(&pages, "Weight", "desc");
    check_groups(
        &desc,
        &[
            (Value::int(3), vec![0, 1]),
            (Value::int(2), vec![2]),
            (Value::int(1), vec![3, 4]),
        ],
        &pages,
    );
    // TestGroupByCalledWithEmptyPages
    assert!(
        nh_page::pagegroup::group_by_ctx(&(), &Vec::new(), "Weight", &[])
            .unwrap()
            .is_none()
    );
    // TestReverse
    let asc = nh_page::pagegroup::group_by(&pages, "Weight", &[]).unwrap();
    let rev = nh_page::pagegroup::pages_group_reverse(&asc);
    for (a, b) in rev.iter().zip(&desc) {
        assert!(nh_page::pagegroup::group_key_eq(&a.key, &b.key));
        assert!(same_list(&a.pages, &b.pages));
    }
}

// Go: resources/page/pagegroup_test.go:TestGroupByParam*
#[test]
fn group_by_param_tests() {
    use nh_page::pagegroup::group_by_param;
    let pages = prepare_page_group_test_pages();
    let expect = [
        (k("bar"), vec![1, 3]),
        (k("baz"), vec![4]),
        (k("foo"), vec![0, 2]),
    ];
    check_groups(
        &group_by_param(&pages, "custom_param", &[])
            .unwrap()
            .unwrap(),
        &expect,
        &pages,
    );
    check_groups(
        &group_by_param(&pages, "custom_param", &["desc".into()])
            .unwrap()
            .unwrap(),
        &[
            (k("foo"), vec![0, 2]),
            (k("baz"), vec![4]),
            (k("bar"), vec![1, 3]),
        ],
        &pages,
    );
    check_groups(
        &group_by_param(&pages, "custom_object.param", &[])
            .unwrap()
            .unwrap(),
        &expect,
        &pages,
    );

    // TestGroupByParamCalledWithCapitalLetterString
    let p = new_test_page();
    p.d.lock()
        .unwrap()
        .params
        .insert("custom_param", k("TestString"));
    let g = group_by_param(&vec![r(&p)], "custom_param", &[])
        .unwrap()
        .unwrap();
    assert_eq!(g[0].key.as_go_string().unwrap().as_bytes(), b"TestString");

    // TestGroupByParamCalledWithSomeUnavailableParams
    let pages2 = prepare_page_group_test_pages();
    for i in [1, 3, 4] {
        with(&pages2[i], |d| {
            d.params.entries.remove(b"custom_param".as_slice());
        });
    }
    check_groups(
        &group_by_param(&pages2, "custom_param", &[])
            .unwrap()
            .unwrap(),
        &[(k("foo"), vec![0, 2])],
        &pages2,
    );
    // TestGroupByParamCalledWithEmptyPages
    assert!(
        group_by_param(&Vec::new(), "custom_param", &[])
            .unwrap()
            .is_none()
    );
    // TestGroupByParamCalledWithUnavailableParam
    assert!(group_by_param(&pages, "unavailable_param", &[]).is_ok());
}

// Go: resources/page/pagegroup_test.go:TestGroupByDate etc.
#[test]
fn group_by_date_tests() {
    use nh_page::pagegroup as pg;
    let pages = prepare_page_group_test_pages();
    let desc = [
        (k("2012-04"), vec![4, 2, 0]),
        (k("2012-03"), vec![3]),
        (k("2012-01"), vec![1]),
    ];
    let asc = [
        (k("2012-01"), vec![1]),
        (k("2012-03"), vec![3]),
        (k("2012-04"), vec![0, 2, 4]),
    ];
    let asc_order = ["asc".to_string()];
    nh_page::page::clear();
    check_groups(
        &pg::group_by_date(&pages, "2006-01", &[]).unwrap().unwrap(),
        &desc,
        &pages,
    );
    check_groups(
        &pg::group_by_date(&pages, "2006-01", &asc_order)
            .unwrap()
            .unwrap(),
        &asc,
        &pages,
    );
    check_groups(
        &pg::group_by_publish_date(&pages, "2006-01", &[])
            .unwrap()
            .unwrap(),
        &desc,
        &pages,
    );
    check_groups(
        &pg::group_by_expiry_date(&pages, "2006-01", &[])
            .unwrap()
            .unwrap(),
        &desc,
        &pages,
    );
    for key in [
        "custom_date",
        "custom_object.date",
        "custom_string_date",
        "custom_object.string_date",
    ] {
        check_groups(
            &pg::group_by_param_date(&pages, key, "2006-01", &[])
                .unwrap()
                .unwrap(),
            &desc,
            &pages,
        );
    }
    check_groups(
        &pg::group_by_param_date(&pages, "custom_date", "2006-01", &asc_order)
            .unwrap()
            .unwrap(),
        &asc,
        &pages,
    );
    check_groups(
        &pg::group_by_lastmod(&pages, "2006-01", &[])
            .unwrap()
            .unwrap(),
        &[
            (k("2015-04"), vec![4, 2, 0]),
            (k("2015-03"), vec![3]),
            (k("2015-01"), vec![1]),
        ],
        &pages,
    );
    check_groups(
        &pg::group_by_lastmod(&pages, "2006-01", &asc_order)
            .unwrap()
            .unwrap(),
        &[
            (k("2015-01"), vec![1]),
            (k("2015-03"), vec![3]),
            (k("2015-04"), vec![0, 2, 4]),
        ],
        &pages,
    );
    assert!(
        pg::group_by_publish_date(&Vec::new(), "2006-01", &[])
            .unwrap()
            .is_none()
    );
    assert!(
        pg::group_by_param_date(&Vec::new(), "custom_date", "2006-01", &[])
            .unwrap()
            .is_none()
    );
}

// ---- pages_prev_next_test.go ----

// Go: resources/page/pages_prev_next_test.go:TestPrev / TestNext
#[test]
fn prev_next() {
    use nh_page::pages_prev_next::{next, prev};
    let pages = prepare_page_group_test_pages();
    assert!(same(&prev(&pages, &*pages[3].0).unwrap(), &pages[4]));
    assert!(same(&prev(&pages, &*pages[1].0).unwrap(), &pages[2]));
    assert!(prev(&pages, &*pages[4].0).is_none());

    assert!(next(&pages, &*pages[0].0).is_none());
    assert!(same(&next(&pages, &*pages[1].0).unwrap(), &pages[0]));
    assert!(same(&next(&pages, &*pages[4].0).unwrap(), &pages[3]));
}

// Go: resources/page/pages_prev_next_test.go:TestWeightedPagesPrev / TestWeightedPagesNext
#[test]
fn weighted_pages_prev_next() {
    use nh_page::weighted::{WeightedPage, sort, weighted_pages_next, weighted_pages_prev};
    let sources = [
        ("/section1/testpage1.md", 5, "2012-04-06"),
        ("/section1/testpage2.md", 4, "2012-01-01"),
        ("/section1/testpage3.md", 3, "2012-04-06"),
        ("/section2/testpage4.md", 2, "2012-03-02"),
        ("/section2/testpage5.md", 1, "2012-04-06"),
    ];
    let mut w: Vec<WeightedPage> = sources
        .iter()
        .map(|(path, weight, date)| {
            let p = new_test_page();
            let t = nh_common::cast::time::to_time(&Value::string(*date));
            {
                let mut d = p.d.lock().unwrap();
                d.path = path.to_string();
                d.weight = *weight;
                d.date = t.clone();
                d.pub_date = t;
            }
            WeightedPage::new(*weight, r(&p), None)
        })
        .collect();
    sort(&mut w);

    assert!(same(
        &weighted_pages_prev(&w, &w[0].page).unwrap(),
        &w[1].page
    ));
    assert!(same(
        &weighted_pages_prev(&w, &w[1].page).unwrap(),
        &w[2].page
    ));
    assert!(weighted_pages_prev(&w, &w[4].page).is_none());

    assert!(weighted_pages_next(&w, &w[0].page).is_none());
    assert!(same(
        &weighted_pages_next(&w, &w[1].page).unwrap(),
        &w[0].page
    ));
    assert!(same(
        &weighted_pages_next(&w, &w[4].page).unwrap(),
        &w[3].page
    ));
}

// ---- pages_sort_search_test.go ----

// Go: resources/page/pages_sort_search_test.go:TestSearchPage
#[test]
fn search_page_binary() {
    use nh_page::pages_sort_search::{is_pages_probably_sorted, search_page_binary};
    let pages = create_sort_test_pages(10);
    for (i, p) in pages.iter().enumerate() {
        with(p, |d| d.title = format!("Title {}", i % 2));
    }
    let by_title = nh_page::pages_sort::by_title(&pages);
    for pages in [by_title.clone(), nh_page::pages_sort::reverse(&by_title)] {
        let less =
            is_pages_probably_sorted(&pages, &[nh_page::pages_sort::less_page_title]).unwrap();
        for (i, p) in pages.iter().enumerate() {
            let idx = search_page_binary(&*p.0, &pages, &*less);
            assert_eq!(idx, i as i64);
        }
    }
}

// Go: resources/page/pages_sort_search_test.go:TestIsPagesProbablySorted
#[test]
fn is_pages_probably_sorted() {
    use nh_page::pages_sort::{by_title, by_weight, default_page_sort};
    use nh_page::pages_sort_search::is_pages_probably_sorted as f;
    let all: [nh_page::pages_sort_search::LessFn; 5] = [
        default_page_sort,
        nh_page::pages_sort::less_page_date,
        nh_page::pages_sort::less_page_pub_date,
        nh_page::pages_sort::less_page_title,
        nh_page::pages_sort::less_page_link_title,
    ];
    assert!(f(&by_weight(&create_sort_test_pages(6)), &[default_page_sort]).is_some());
    assert!(
        f(
            &by_weight(&create_sort_test_pages(300)),
            &[default_page_sort]
        )
        .is_some()
    );
    assert!(f(&create_sort_test_pages(6), &[default_page_sort]).is_none());
    assert!(f(&by_title(&create_sort_test_pages(300)), &all).is_some());
}

/// `searchPage` on lists of 1000+ pages takes the binary search path (Go's BenchmarkSearchPage
/// variants as a test): every page is found at its index.
#[test]
fn search_page_large_lists() {
    use nh_page::pages_sort as ps;
    let pages = create_sort_test_pages(1500);
    let mut seed = 7u64;
    for p in &pages {
        seed = seed
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        let n = (seed >> 33) as i64;
        with(p, |d| {
            d.weight = n % 1500;
            d.title = format!("Title {}", n % 1500);
            d.pub_date = hours(&now(), n % 300);
            d.date = hours(&now(), (n >> 7) % 300);
        });
    }
    for list in [
        ps::by_weight(&pages),
        ps::reverse(&ps::by_weight(&pages)),
        ps::by_date(&pages),
        ps::by_publish_date(&pages),
        ps::by_title(&pages),
        pages.clone(),
    ] {
        for j in (0..list.len()).step_by(7) {
            assert_eq!(
                nh_page::pages_sort_search::search_page(&*list[j].0, &list),
                j as i64
            );
        }
    }
}

// ---- pages_cache_test.go ----

// Go: resources/page/pages_cache_test.go:TestPageCache (sequential)
#[test]
fn page_cache() {
    let c1 = nh_page::pages_cache::PageCache::default();
    let change_first = |p: &mut Pages| with(&p[0], |d| d.description = "changed".into());
    let sets: Vec<Pages> = (0..50).map(|i| create_sort_test_pages(i + 1)).collect();
    for round in 0..3 {
        for pages in &sets {
            let (p, ca) = c1.get("k1", &|_| {}, &[pages]);
            assert_eq!(ca, round > 0);
            let (p2, c2) = c1.get("k1", &|_| {}, &[&p]);
            assert!(c2);
            assert!(nh_page::pages_cache::pages_equal(&p, &p2));
            assert!(nh_page::pages_cache::pages_equal(&p, pages));

            let (p3, c3) = c1.get("k2", &change_first, &[pages]);
            assert_eq!(c3, round > 0);
            assert_eq!(p3[0].0.description(), "changed");
        }
    }
    c1.clear();
    assert!(!c1.get("k1", &|_| {}, &[&sets[0]]).1);
}

// ---- pages_test.go ----

fn titled(t: &str) -> PageRef {
    let p = new_test_page();
    p.d.lock().unwrap().title = t.into();
    r(&p)
}

// Go: resources/page/pages_test.go:TestProbablyEq
#[test]
fn probably_eq() {
    use nh_page::pagegroup::pages_group_probably_eq;
    use nh_page::pages::probably_eq;
    let (p1, p2, p3) = (titled("p1"), titled("p2"), titled("p3"));
    let pages12 = vec![p1.clone(), p2.clone()];
    let pages21 = vec![p2.clone(), p1.clone()];
    let pages123 = vec![p1, p2, p3];

    assert!(probably_eq(&pages12, &pages12));
    assert!(!probably_eq(&pages123, &pages12));
    assert!(!probably_eq(&pages12, &pages21));

    let g = |k: &str, p: &Pages| PageGroup {
        key: Value::string(k),
        pages: p.clone(),
    };
    assert!(g("a", &pages12).probably_eq(&g("a", &pages12).to_value()));
    assert!(!g("a", &pages12).probably_eq(&g("b", &pages12).to_value()));

    let (pg1, pg2) = (g("a", &pages12), g("b", &pages123));
    let v = nh_page::pagegroup::pages_group_to_value(&vec![pg1.clone(), pg2.clone()]);
    assert!(pages_group_probably_eq(&vec![pg1.clone(), pg2.clone()], &v));
    let v = nh_page::pagegroup::pages_group_to_value(&vec![pg2.clone(), pg1.clone()]);
    assert!(!pages_group_probably_eq(&vec![pg1, pg2], &v));
}

// Go: resources/page/pages_test.go:TestToPages
#[test]
fn to_pages() {
    let (p1, p2) = (titled("p1"), titled("p2"));
    let pages12 = vec![p1.clone(), p2.clone()];
    assert!(pages_from_value(&Value::Invalid).unwrap().is_empty());
    assert!(same_list(
        &pages_from_value(&pages_to_value(&pages12)).unwrap(),
        &pages12
    ));
    let typed = Value::list(
        go_value::SliceType::Named(Arc::from("[]page.Page")),
        vec![p1.to_value(), p2.to_value()],
    );
    assert!(same_list(&pages_from_value(&typed).unwrap(), &pages12));
    let any = Value::any_list(vec![p1.to_value(), p2.to_value()]);
    assert!(same_list(&pages_from_value(&any).unwrap(), &pages12));
    assert!(pages_from_value(&Value::string("not a page")).is_err());
}

// ---- pages_related_test.go ----

/// `types.KeyValues` as the `keyVals` template function returns it.
struct KeyValuesObj {
    key: Value,
    values: Vec<Value>,
}

impl Object for KeyValuesObj {
    fn type_name(&self) -> Cow<'_, str> {
        Cow::Borrowed("types.KeyValues")
    }
    fn kind(&self) -> go_value::Kind {
        go_value::Kind::Struct
    }
    fn has_method(&self, _name: &str) -> bool {
        false
    }
    fn call_method(
        &self,
        _ctx: HostCtx<'_>,
        _name: &str,
        _args: &[Value],
    ) -> Option<GoResult<Value>> {
        None
    }
    fn field(&self, name: &str) -> Option<Value> {
        match name {
            "Key" => Some(self.key.clone()),
            "Values" => Some(Value::any_list(self.values.clone())),
            _ => None,
        }
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

fn kv(key: &str, values: &[&str]) -> Value {
    Value::object(KeyValuesObj {
        key: Value::string(key),
        values: values.iter().map(|v| Value::string(*v)).collect(),
    })
}

// Go: resources/page/pages_related_test.go:TestRelated
#[test]
fn related() {
    use nh_page::pages_related::related;
    let mk = |title: &str, date: &str, kws: &[&str]| {
        let p = new_test_page();
        {
            let mut d = p.d.lock().unwrap();
            d.title = title.into();
            d.pub_date = go_time::parse("2006-01-02", date).unwrap();
            d.params
                .insert("keywords", Value::string_list(kws.iter().copied()));
        }
        r(&p)
    };
    let pages = vec![
        mk("Page 1", "2017-01-03", &["hugo", "says"]),
        mk("Page 2", "2017-01-02", &["hugo", "rocks"]),
        mk("Page 3", "2017-01-01", &["bep", "says"]),
    ];
    let titles = |r: &Pages| -> Vec<String> { r.iter().map(|p| Page::title(&*p.0)).collect() };

    let mut opts = Map::new(MapType::StringAny);
    opts.insert("namedSlices", kv("keywords", &["hugo", "rocks"]));
    let result = related(&(), &pages, &Value::map(opts)).unwrap();
    assert_eq!(titles(&result), ["Page 2", "Page 1"]);

    let result = related(&(), &pages, &pages[0].to_value()).unwrap();
    assert_eq!(titles(&result), ["Page 2", "Page 3"]);

    let mut opts = Map::new(MapType::StringAny);
    opts.insert("document", pages[0].to_value());
    opts.insert("indices", Value::string_list(["keywords"]));
    let result = related(&(), &pages, &Value::map(opts)).unwrap();
    assert_eq!(titles(&result), ["Page 2", "Page 3"]);

    let mut opts = Map::new(MapType::StringAny);
    opts.insert(
        "namedSlices",
        Value::any_list(vec![kv("keywords", &["bep", "rocks"])]),
    );
    let result = related(&(), &pages, &Value::map(opts)).unwrap();
    assert_eq!(titles(&result), ["Page 2", "Page 3"]);
}

// ---- navigation/menu_cache_test.go ----

// Go: navigation/menu_cache_test.go:TestMenuCache (sequential)
#[test]
fn menu_cache() {
    use nh_page::navigation::menu::{Menu, MenuEntry};
    use nh_page::navigation::menu_cache::{MenuCache, menu_equal};
    let c1 = MenuCache::default();
    let sets: Vec<Menu> = (0..50)
        .map(|i| (0..=i).map(|_| Arc::new(MenuEntry::default())).collect())
        .collect();
    for round in 0..3 {
        for menu in &sets {
            let (m, ca) = c1.get("k1", &|_| {}, &[menu]);
            assert_eq!(ca, round > 0);
            let (m2, c2) = c1.get("k1", &|_| {}, &[&m]);
            assert!(c2);
            assert!(menu_equal(&m, &m2));
            assert!(menu_equal(&m, menu));

            let (m3, c3) = c1.get(
                "k2",
                &|m: &mut Menu| {
                    let mut e = (*m[0]).clone();
                    e.config.title = "changed".into();
                    m[0] = Arc::new(e);
                },
                &[menu],
            );
            assert_eq!(c3, round > 0);
            assert_eq!(m3[0].config.title, "changed");
        }
    }
}
