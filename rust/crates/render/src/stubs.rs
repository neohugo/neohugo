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
//! | `paginator()` | records the scope page's default list (`FlatSite::pagination_list`) with `pagination.pagerSize` on the first call per (page, format); returns pager `__nh.pager` (1 when unset) as a pager value (`page_number`, `url`, `pages`, `pager_size`, `total_pages`, `total_number_of_elements`, `has_prev`, `has_next`, `prev`, `next`, `first`, `last`) |
//! | `x \| rel_url`, `x \| abs_url` | `SiteUrls::rel_url` / `abs_url` of the scope's language (the default language without a scope) |
//! | `p \| deref` | the full value of the page with `p.id` in the scope's generation |
//! | `pages \| by_lastmod` | stable sort by `lastmod.unix`, pages without one first |

use std::sync::Arc;

use neohugo_base::{FormatId, Idx, LangIdx, PageId};
use neohugo_view::interim::FlatSite;
use neohugo_view::{PaginationRecorder, Phase, Recorded, RenderScope, SCOPE_KEY, ViewCache};
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
    fn flat(&self) -> &FlatSite {
        self.views.flat()
    }

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
        self.flat().config.sites[lang].site_urls()
    }

    fn deref(&self, v: &Value, st: &State) -> TeraResult<Value> {
        let id = page_id(v).ok_or_else(|| tera::Error::message("deref expects a page value"))?;
        if id.index() >= self.flat().pages.len() {
            return Err(tera::Error::message(format!("deref: no page {id}")));
        }
        let (phase, variant) = RenderScope::from_state(st)?
            .map_or((Phase::Layout, neohugo_view::HookVariant::Html), |s| {
                (s.phase, s.variant)
            });
        Ok(self.views.generation(phase, variant).full(id))
    }

    fn pager_url(&self, page: PageId, format: FormatId, number: u32) -> TeraResult<String> {
        let (_, links) = self
            .flat()
            .target(page, format, (number > 1).then_some(number))
            .map_err(|e| tera::Error::chain("paginator", e))?;
        Ok(links.rel_permalink.escaped())
    }

    fn paginator(&self, st: &State) -> TeraResult<Value> {
        let sc = scope(st, "paginator")?;
        let flat = self.flat();
        let size = flat.config.sites[sc.lang].pagination.pager_size;
        let rec = self.pagination.record(sc.page, sc.format, || Recorded {
            items: flat.pagination_list(sc.page).into(),
            size,
        });
        let number = sc.pager.unwrap_or(1);
        let total = rec.total_pages();
        let generation = self.views.generation(sc.phase, sc.variant);
        let link = |n: u32| -> TeraResult<Value> {
            let mut m = tera::Map::new();
            m.insert("page_number".into(), n.into());
            m.insert("url".into(), self.pager_url(sc.page, sc.format, n)?.into());
            Ok(Value::from(m))
        };
        let pages: Vec<Value> = rec
            .page(number)
            .iter()
            .map(|&p| generation.summaries[p].clone())
            .collect();
        let mut m = tera::Map::new();
        m.insert("page_number".into(), number.into());
        m.insert(
            "url".into(),
            self.pager_url(sc.page, sc.format, number)?.into(),
        );
        m.insert("pages".into(), Value::from(pages));
        m.insert("pager_size".into(), rec.size.into());
        m.insert("total_pages".into(), total.into());
        m.insert("total_number_of_elements".into(), rec.items.len().into());
        m.insert("has_prev".into(), (number > 1).into());
        m.insert("has_next".into(), (number < total).into());
        m.insert(
            "prev".into(),
            if number > 1 {
                link(number - 1)?
            } else {
                Value::none()
            },
        );
        m.insert(
            "next".into(),
            if number < total {
                link(number + 1)?
            } else {
                Value::none()
            },
        );
        m.insert("first".into(), link(1)?);
        m.insert("last".into(), link(total)?);
        Ok(Value::from(m))
    }
}
