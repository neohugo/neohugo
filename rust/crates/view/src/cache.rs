//! The view cache (REWRITE_PLAN.md §2.5): the Meta generation for the content phase and one
//! Full generation per hook variant, frozen before any layout renders.
//!
//! **Skeleton (T38).** Built from the interim [`FlatSite`]; T33 builds it from the real model
//! and keeps this API (`ViewCache::generation`, `ViewGeneration::{summaries, links, sites,
//! full}`).

use std::collections::BTreeMap;
use std::sync::{Arc, OnceLock};

use neohugo_base::{IdVec, LangIdx, PageId, PageKind};
use serde::Serialize;

use crate::content::RenderedContent;
use crate::interim::FlatSite;
use crate::scope::{HookVariant, Phase};
use crate::views::{ContentView, DateView, LanguageView, page_link, page_summary, params_value};

/// The rendered content of every page in one hook variant (`None`: no content).
pub type Contents = IdVec<PageId, Option<Arc<RenderedContent>>>;

/// One generation of page and site values.
#[derive(Debug)]
pub struct ViewGeneration {
    flat: Arc<FlatSite>,
    /// Relation-free page values; every list holds these (Arc-shared).
    pub summaries: IdVec<PageId, tera::Value>,
    /// Page link values.
    pub links: IdVec<PageId, tera::Value>,
    full: IdVec<PageId, OnceLock<tera::Value>>,
    /// `site`, per language.
    pub sites: IdVec<LangIdx, tera::Value>,
}

/// `site.taxonomies.<plural>.<key>`.
#[derive(Serialize)]
struct TermEntryView {
    name: String,
    key: String,
    count: usize,
    page: tera::Value,
    pages: tera::Value,
}

impl ViewGeneration {
    fn build(flat: &Arc<FlatSite>, contents: Option<&Contents>) -> Self {
        let summaries: IdVec<PageId, tera::Value> = flat
            .pages
            .iter()
            .map(|p| {
                let content = contents.map(|c| {
                    let raw = p.source.as_ref().map_or("", |s| &*s.body);
                    ContentView::new(c.get(p.id).and_then(Option::as_deref), raw)
                });
                tera::Value::from_serializable(&page_summary(flat, p, content))
            })
            .collect();
        let links = flat
            .pages
            .iter()
            .map(|p| tera::Value::from_serializable(&page_link(flat, p)))
            .collect();
        let full = flat.pages.iter().map(|_| OnceLock::new()).collect();
        let mut g = Self {
            flat: Arc::clone(flat),
            summaries,
            links,
            full,
            sites: IdVec::new(),
        };
        g.sites = flat.langs.ids().map(|l| g.site_value(l)).collect();
        g
    }

    fn list(&self, ids: &[PageId]) -> tera::Value {
        tera::Value::from(
            ids.iter()
                .map(|&p| self.summaries[p].clone())
                .collect::<Vec<_>>(),
        )
    }

    fn opt(&self, id: Option<PageId>) -> tera::Value {
        id.map_or_else(tera::Value::none, |p| self.summaries[p].clone())
    }

    fn site_value(&self, lang: LangIdx) -> tera::Value {
        let flat = &self.flat;
        let cfg = &flat.config;
        let site = &cfg.sites[lang];
        let l = &flat.langs[lang];
        let mut taxonomies = tera::Map::new();
        for t in &l.taxonomies {
            let mut terms = tera::Map::new();
            for term in &t.terms {
                let v = TermEntryView {
                    name: term.name.clone(),
                    key: term.key.clone(),
                    count: term.pages.len(),
                    page: self.summaries[term.page].clone(),
                    pages: self.list(&term.pages),
                };
                terms.insert(term.key.clone().into(), tera::Value::from_serializable(&v));
            }
            taxonomies.insert(t.plural.clone().into(), tera::Value::from(terms));
        }
        let last_mod = l
            .regular_pages
            .iter()
            .filter_map(|&p| flat.pages[p].dates.lastmod.as_ref())
            .max_by_key(|d| d.timestamp())
            .map(DateView::new);
        let sitemap_abs_url = l
            .standalone
            .iter()
            .map(|&p| &flat.pages[p])
            .find(|p| p.kind == PageKind::Sitemap)
            .and_then(|p| p.primary())
            .map(|o| o.links.permalink.to_string());
        let mut m = tera::Map::new();
        let mut put = |k: &'static str, v: tera::Value| {
            m.insert(k.into(), v);
        };
        put("title", site.title.clone().into());
        put("base_url", site.base_url.as_str().into());
        put("lang", site.language.key.clone().into());
        put("language_code", site.language.code.clone().into());
        put(
            "language",
            tera::Value::from_serializable(&LanguageView::new(site)),
        );
        put(
            "languages",
            tera::Value::from(
                cfg.sites
                    .iter()
                    .map(|s| tera::Value::from_serializable(&LanguageView::new(s)))
                    .collect::<Vec<_>>(),
            ),
        );
        put("is_multilingual", (cfg.sites.len() > 1).into());
        put("copyright", site.copyright.clone().into());
        put("params", params_value(&site.params));
        put(
            "data",
            neohugo_base::Value::Map(Arc::new(neohugo_base::Map::new())).to_tera(),
        );
        put("home", self.opt(l.home));
        put("pages", self.list(&l.pages));
        put("regular_pages", self.list(&l.regular_pages));
        let all: Vec<PageId> = flat
            .langs
            .iter()
            .flat_map(|x| x.pages.iter().copied())
            .collect();
        put("all_pages", self.list(&all));
        put("sections", self.list(&l.sections));
        put("taxonomies", tera::Value::from(taxonomies));
        put("menus", tera::Value::from(tera::Map::new()));
        put(
            "last_mod",
            last_mod.map_or_else(tera::Value::none, |d| tera::Value::from_serializable(&d)),
        );
        let mut rss = tera::Map::new();
        rss.insert("limit".into(), site.services.rss.limit.into());
        let mut services = tera::Map::new();
        services.insert("rss".into(), tera::Value::from(rss));
        let mut config = tera::Map::new();
        config.insert("services".into(), tera::Value::from(services));
        put("config", tera::Value::from(config));
        put(
            "sitemap_abs_url",
            sitemap_abs_url.map_or_else(tera::Value::none, tera::Value::from),
        );
        tera::Value::from(m)
    }

    /// The full value of page `id`: its summary plus its relations (built on first use).
    #[must_use]
    pub fn full(&self, id: PageId) -> tera::Value {
        self.full[id].get_or_init(|| self.full_value(id)).clone()
    }

    fn full_value(&self, id: PageId) -> tera::Value {
        let p = &self.flat.pages[id];
        let mut m = self.summaries[id].as_map().cloned().unwrap_or_default();
        let mut put = |k: &'static str, v: tera::Value| {
            m.insert(k.into(), v);
        };
        let pages: &[PageId] = match p.kind {
            PageKind::Sitemap => &self.flat.langs[p.lang].pages,
            _ => &p.pages,
        };
        put("parent", self.opt(p.parent));
        put("pages", self.list(pages));
        put("regular_pages", self.list(&p.regular_pages));
        let sections: Vec<PageId> = p
            .pages
            .iter()
            .copied()
            .filter(|&q| self.flat.pages[q].kind == PageKind::Section)
            .collect();
        put("sections", self.list(&sections));
        put("prev_in_section", self.opt(p.prev_in_section));
        put("next_in_section", self.opt(p.next_in_section));
        put("translations", self.list(&p.translations));
        let mut all = p.translations.clone();
        all.push(id);
        all.sort_by_key(|&q| self.flat.pages[q].lang);
        put("all_translations", self.list(&all));
        tera::Value::from(m)
    }
}

/// The frozen page and site values of a build.
#[derive(Debug)]
pub struct ViewCache {
    flat: Arc<FlatSite>,
    meta: ViewGeneration,
    full: OnceLock<BTreeMap<HookVariant, ViewGeneration>>,
}

impl ViewCache {
    /// The cache with its Meta generation (no content fields).
    #[must_use]
    pub fn new(flat: Arc<FlatSite>) -> Self {
        Self {
            meta: ViewGeneration::build(&flat, None),
            flat,
            full: OnceLock::new(),
        }
    }

    /// Builds and freezes the Full generations from the rendered content of every variant
    /// (phase D). A second call is ignored.
    pub fn freeze(&self, contents: &BTreeMap<HookVariant, Contents>) {
        self.full.get_or_init(|| {
            contents
                .iter()
                .map(|(&v, c)| (v, ViewGeneration::build(&self.flat, Some(c))))
                .collect()
        });
    }

    /// Whether the Full generations are frozen.
    #[must_use]
    pub fn is_frozen(&self) -> bool {
        self.full.get().is_some()
    }

    /// The generation a render in `phase` with variant `v` sees: Meta in the content phase,
    /// else the Full generation of `v` (of `Html` when `v` has none; Meta before phase D).
    #[must_use]
    pub fn generation(&self, phase: Phase, v: HookVariant) -> &ViewGeneration {
        if phase == Phase::Content {
            return &self.meta;
        }
        let Some(full) = self.full.get() else {
            return &self.meta;
        };
        full.get(&v)
            .or_else(|| full.get(&HookVariant::Html))
            .unwrap_or(&self.meta)
    }

    /// **Interim (T38).** The flat model the views were built from.
    #[must_use]
    pub fn flat(&self) -> &Arc<FlatSite> {
        &self.flat
    }
}
