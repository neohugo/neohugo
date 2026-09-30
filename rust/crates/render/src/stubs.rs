//! **Throwaway (T38 walking skeleton).** Stub site functions, just enough for the testsite
//! layouts and the embedded `rss.xml` / `sitemap.xml`. `neohugo-sitefuncs` (T35) replaces
//! this module.
//!
//! Every site-bound `spec::FUNCS` entry is first registered as a kwargs-checking placeholder
//! (`neohugo_funcs::register_placeholders`: a filter returns its input, a function none, a test
//! false); these five are then replaced by working stubs:
//!
//! | name | stub |
//! |---|---|
//! | `paginator()` | records the scope page's default list (`neohugo_nav::default_pagination_list`) with `pagination.pagerSize` on the first call per (page, format); returns pager `__nh.pager` (1 when unset) as a `PagerView` (`neohugo_view::pager_view`) |
//! | `x \| rel_url`, `x \| abs_url` | `SiteUrls::rel_url` / `abs_url` of the scope's language (the default language without a scope) |
//! | `p \| deref` | the full value of the page with `p.id` in the scope's generation |
//! | `pages \| by_lastmod` | stable sort by `lastmod.unix`, pages without one first |

use std::sync::Arc;

use neohugo_base::{Idx, LangIdx, PageId};
use neohugo_nav::{Pagination, PaginationItems, default_pagination_list};
use neohugo_view::{
    PaginationRecorder, Phase, RenderScope, SCOPE_KEY, ViewCache, pager_url, pager_view,
};
use tera::{Kwargs, State, TeraResult, Value};

/// What the stubs read.
#[derive(Clone)]
pub(crate) struct Stubs {
    pub views: Arc<ViewCache>,
    pub pagination: Arc<PaginationRecorder>,
}

fn scope(st: &State, name: &str) -> TeraResult<RenderScope> {
    RenderScope::from_state(st)?.ok_or_else(|| {
        tera::Error::message(format!(
            "`{name}` needs the render scope `{SCOPE_KEY}` (declare `@{SCOPE_KEY}` in a component)"
        ))
    })
}

fn page_id(v: &Value) -> Option<PageId> {
    let id = v.as_map()?.get(&tera::value::Key::Str("id"))?.as_u64()?;
    u32::try_from(id).ok().map(PageId::from_raw)
}

fn field<'a>(v: &'a Value, k: &'static str) -> Option<&'a Value> {
    v.as_map()?.get(&tera::value::Key::Str(k))
}

impl Stubs {
    pub(crate) fn register(&self, tera: &mut tera::Tera) {
        let s = self.clone();
        tera.register_function("paginator", move |_: Kwargs, st: &State| s.paginator(st));
        let s = self.clone();
        tera.register_filter("rel_url", move |v: String, _: Kwargs, st: &State| {
            Ok::<_, tera::Error>(Value::from(s.urls(st).rel_url(&v)))
        });
        let s = self.clone();
        tera.register_filter("abs_url", move |v: String, _: Kwargs, st: &State| {
            Ok::<_, tera::Error>(Value::from(s.urls(st).abs_url(&v)))
        });
        let s = self.clone();
        tera.register_filter("deref", move |v: Value, _: Kwargs, st: &State| {
            s.deref(&v, st)
        });
        tera.register_filter("by_lastmod", |v: Value, _: Kwargs, _: &State| {
            let mut items = v
                .as_array()
                .ok_or_else(|| tera::Error::message("by_lastmod expects a list of pages"))?
                .to_vec();
            items.sort_by_key(|p| {
                field(p, "lastmod")
                    .and_then(|d| field(d, "unix"))
                    .and_then(Value::as_i64)
            });
            Ok::<_, tera::Error>(Value::from(items))
        });
    }

    fn urls(&self, st: &State) -> neohugo_base::url::SiteUrls {
        let lang = RenderScope::from_state(st)
            .ok()
            .flatten()
            .map_or(LangIdx::from_index(0), |s| s.lang);
        self.views.model().config.sites[lang].site_urls()
    }

    fn deref(&self, v: &Value, st: &State) -> TeraResult<Value> {
        let id = page_id(v).ok_or_else(|| tera::Error::message("deref expects a page value"))?;
        if !self.views.contains(id) {
            return Err(tera::Error::message(format!("deref: no page {id}")));
        }
        let (phase, variant) = RenderScope::from_state(st)?
            .map_or((Phase::Layout, neohugo_view::HookVariant::Html), |s| {
                (s.phase, s.variant)
            });
        Ok(self.views.generation(phase, variant).full(id))
    }

    fn paginator(&self, st: &State) -> TeraResult<Value> {
        let sc = scope(st, "paginator")?;
        let model = self.views.model();
        let size = model.config.sites[sc.lang].pagination.pager_size.max(1);
        let rec = match self.pagination.get(sc.page, sc.format) {
            Some(r) => r,
            None => {
                let items =
                    PaginationItems::Pages(default_pagination_list(self.views.nav(), sc.page));
                let p =
                    Pagination::new(items, size).map_err(|e| tera::Error::chain("paginator", e))?;
                self.pagination.paginator(sc.page, sc.format, None, || p)
            }
        };
        let generation = self.views.generation(sc.phase, sc.variant);
        let pager = pager_view(generation, &rec, sc.pager.unwrap_or(1), |n| {
            pager_url(model, sc.page, sc.format, n).map_err(|e| tera::Error::chain("paginator", e))
        })?;
        Ok(Value::from_serializable(&pager))
    }
}
