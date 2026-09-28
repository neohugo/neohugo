//! Shared helpers of the T12 collection tests (collections, pagination, related, menus): fake
//! pages and sites rebuilt from the page tables the Go oracles record
//! (`tools/go-oracle/nh-page/csupport`), and result encoders.
//!
//! Every recorded page VALUE is a [`FakePage`] (a `pageWithWeight0` or `*pageWithOrdinal` wrapper is
//! its own entry, with the id of the page it wraps). Page ids are unique in the test process, so
//! the global sorted-pages cache never mixes pages of two loaded builds. The sites share a
//! "current site" cell that the tests set like the Go oracle did.

#![allow(dead_code)]

use std::any::Any;
use std::borrow::Cow;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, OnceLock};

use go_value::{GoString, HostCtx, Map, MapType, Time, Value};
use nh_common::Result;
use nh_common::maps::scratch::Scratch;
use nh_common::object::GoResult;
use nh_common::paths::pathparser::Path;
use nh_config::common_config::SitemapConfig;
use nh_config::neohugo::neohugo::HugoInfo;
use nh_helpers::source::file_info::File;
use nh_langs::config::LanguageConfig;
use nh_langs::language::{Language, Languages};
use nh_media::media::media_type::MediaType;
use nh_page::navigation::menu::Menus;
use nh_page::page::{Page, PageRef, Pages, named_page_meta_value};
use nh_page::pages_related::RelatedDocsHandler;
use nh_page::related::{IndexConfig, Keyword};
use nh_page::site::{Site, SiteConfig, SiteRef};
use nh_page::taxonomy::TaxonomyList;
use nh_resource::resourcetypes::{Resource, Resources};
use serde_json::Value as J;

use crate::support::{decode, decode_time, gostring};

static NEXT_ID: AtomicU64 = AtomicU64::new(1);

/// A site of a recorded build.
pub struct FakeSite {
    pub idx: usize,
    pub lang: Arc<Language>,
    pub params: Arc<Map>,
    pub build: Arc<BuildShared>,
    /// The site's related docs handler (Go `s.relatedDocsHandler`).
    pub related: OnceLock<Arc<RelatedDocsHandler>>,
}

/// What the sites of one build share: the current site index and the sites.
pub struct BuildShared {
    pub current: AtomicUsize,
    pub sites: OnceLock<Vec<Arc<FakeSite>>>,
}

impl Site for FakeSite {
    fn site_index(&self) -> usize {
        self.idx
    }
    fn language(&self) -> Arc<Language> {
        self.lang.clone()
    }
    fn languages(&self) -> Languages {
        unimplemented!()
    }
    fn get_page(&self, _refs: &[String]) -> Result<Option<PageRef>> {
        unimplemented!()
    }
    fn all_pages(&self) -> Pages {
        unimplemented!()
    }
    fn regular_pages(&self) -> Pages {
        unimplemented!()
    }
    fn pages(&self) -> Pages {
        unimplemented!()
    }
    fn sections(&self) -> Pages {
        unimplemented!()
    }
    fn home(&self) -> Option<PageRef> {
        unimplemented!()
    }
    fn title(&self) -> String {
        unimplemented!()
    }
    fn language_code(&self) -> String {
        unimplemented!()
    }
    fn copyright(&self) -> String {
        unimplemented!()
    }
    fn sites(&self) -> Vec<SiteRef> {
        unimplemented!()
    }
    fn current(&self) -> SiteRef {
        let sites = self.build.sites.get().unwrap();
        SiteRef(sites[self.build.current.load(Ordering::SeqCst)].clone())
    }
    fn hugo(&self) -> HugoInfo {
        unimplemented!()
    }
    fn base_url(&self) -> String {
        unimplemented!()
    }
    fn taxonomies(&self) -> TaxonomyList {
        unimplemented!()
    }
    fn last_change(&self) -> Time {
        unimplemented!()
    }
    fn lastmod(&self) -> Time {
        unimplemented!()
    }
    fn menus(&self) -> Menus {
        unimplemented!()
    }
    fn main_sections(&self) -> Vec<String> {
        unimplemented!()
    }
    fn params(&self) -> Arc<Map> {
        self.params.clone()
    }
    fn param(&self, _key: &Value) -> Result<Value> {
        unimplemented!()
    }
    fn data(&self) -> Arc<Map> {
        unimplemented!()
    }
    fn config(&self) -> SiteConfig {
        unimplemented!()
    }
    fn build_drafts(&self) -> bool {
        unimplemented!()
    }
    fn is_multi_lingual(&self) -> bool {
        unimplemented!()
    }
    fn language_prefix(&self) -> String {
        unimplemented!()
    }
    fn store(&self) -> Arc<Scratch> {
        unimplemented!()
    }
    fn tpl_has_method(&self, _name: &str) -> bool {
        false
    }
    fn tpl_call_method(
        &self,
        _ctx: HostCtx<'_>,
        _name: &str,
        _args: &[Value],
    ) -> Option<GoResult<Value>> {
        None
    }
}

/// A recorded page value.
pub struct FakePage {
    pub entry: usize,
    pub id: u64,
    pub type_name: String,
    pub site: Arc<FakeSite>,
    pub weight0: Option<i64>,
    pub ordinal: Option<i64>,
    pub kind: String,
    pub title: String,
    pub link_title: String,
    pub description: String,
    pub weight: i64,
    pub date: Time,
    pub lastmod: Time,
    pub publish_date: Time,
    pub expiry_date: Time,
    pub is_home: bool,
    pub is_node: bool,
    pub is_page: bool,
    pub is_section: bool,
    pub section: String,
    pub page_type: String,
    pub layout: String,
    pub lang: String,
    pub path: String,
    pub path_info: Arc<Path>,
    pub slug: String,
    pub draft: bool,
    pub aliases: Vec<String>,
    pub keywords: Vec<String>,
    pub bundle_type: String,
    pub name: String,
    pub translation_key: String,
    pub rel_permalink: String,
    pub params: Arc<Map>,
    pub len: i64,
    pub file: Option<Arc<File>>,
    pub fragments: Option<Vec<String>>,
    pub headings: Vec<String>,
    /// Set for a `pageHeadingsFiltered` copy.
    pub headings_filtered: Option<Vec<String>>,
    pub parent: OnceLock<Option<PageRef>>,
    /// Pages this page is an ancestor of (menus oracle).
    pub ancestor_of: OnceLock<Vec<u64>>,
}

impl Resource for FakePage {
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
        self.rel_permalink.clone()
    }
    fn data(&self) -> Value {
        unimplemented!()
    }
    fn name(&self) -> String {
        self.name.clone()
    }
    fn title(&self) -> String {
        self.title.clone()
    }
    fn params(&self) -> Arc<Map> {
        self.params.clone()
    }
    fn key(&self) -> String {
        unimplemented!()
    }
    fn language(&self) -> Option<Arc<Language>> {
        Some(self.site.lang.clone())
    }
    fn translation_key(&self) -> Option<String> {
        Some(self.translation_key.clone())
    }
    fn tpl_type_name(&self) -> Cow<'_, str> {
        Cow::Borrowed(&self.type_name)
    }
    fn tpl_has_method(&self, name: &str) -> bool {
        self.tpl_call_method(&(), name, &[]).is_some()
    }
    fn tpl_call_method(
        &self,
        _ctx: HostCtx<'_>,
        name: &str,
        args: &[Value],
    ) -> Option<GoResult<Value>> {
        let s = |v: &str| Value::string(v);
        Some(Ok(match name {
            "Section" => s(&self.section),
            "Kind" => s(&self.kind),
            "Type" => s(&self.page_type),
            "Weight" => Value::int(self.weight),
            "Title" => s(&self.title),
            "LinkTitle" => s(&self.link_title),
            "Lang" => s(&self.lang),
            "Layout" => s(&self.layout),
            "BundleType" => s(&self.bundle_type),
            "Slug" => s(&self.slug),
            "Draft" => Value::Bool(self.draft),
            "IsPage" => Value::Bool(self.is_page),
            "IsNode" => Value::Bool(self.is_node),
            "Date" => Value::Time(self.date.clone()),
            "Parent" => match self.parent() {
                Some(p) => p.to_value(),
                None => Value::TypedNil(Arc::from("page.Page")),
            },
            "IsAncestor" => {
                let other = nh_page::page::page_from_value(&args[0]);
                Value::Bool(other.is_some_and(|o| {
                    self.ancestor_of
                        .get()
                        .is_some_and(|a| a.contains(&o.0.page_id()))
                }))
            }
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

impl Page for FakePage {
    fn page_id(&self) -> u64 {
        self.id
    }
    fn unwrap_page(self: Arc<Self>) -> Arc<dyn Page> {
        self
    }
    fn weight0(&self) -> Option<i64> {
        self.weight0
    }
    fn ordinal(&self) -> Option<i64> {
        self.ordinal
    }
    fn kind(&self) -> String {
        self.kind.clone()
    }
    fn title(&self) -> String {
        self.title.clone()
    }
    fn link_title(&self) -> String {
        self.link_title.clone()
    }
    fn description(&self) -> String {
        self.description.clone()
    }
    fn weight(&self) -> i64 {
        self.weight
    }
    fn date(&self) -> Time {
        self.date.clone()
    }
    fn lastmod(&self) -> Time {
        self.lastmod.clone()
    }
    fn publish_date(&self) -> Time {
        self.publish_date.clone()
    }
    fn expiry_date(&self) -> Time {
        self.expiry_date.clone()
    }
    fn is_home(&self) -> bool {
        self.is_home
    }
    fn is_node(&self) -> bool {
        self.is_node
    }
    fn is_page(&self) -> bool {
        self.is_page
    }
    fn is_section(&self) -> bool {
        self.is_section
    }
    fn section(&self) -> String {
        self.section.clone()
    }
    fn page_type(&self) -> String {
        self.page_type.clone()
    }
    fn layout(&self) -> String {
        self.layout.clone()
    }
    fn lang(&self) -> String {
        self.lang.clone()
    }
    fn path(&self) -> String {
        self.path.clone()
    }
    fn path_info(&self) -> Arc<Path> {
        self.path_info.clone()
    }
    fn slug(&self) -> String {
        self.slug.clone()
    }
    fn draft(&self) -> bool {
        self.draft
    }
    fn aliases(&self) -> Vec<String> {
        self.aliases.clone()
    }
    fn keywords(&self) -> Vec<String> {
        self.keywords.clone()
    }
    fn bundle_type(&self) -> String {
        self.bundle_type.clone()
    }
    fn sitemap(&self) -> SitemapConfig {
        unimplemented!()
    }
    fn param(&self, key: &Value) -> Result<Value> {
        nh_resource::params::param(&self.params, Some(&self.site.params), key)
    }
    fn page_params(&self) -> Arc<Map> {
        self.params.clone()
    }
    fn site(&self) -> SiteRef {
        SiteRef(self.site.clone())
    }
    fn file(&self) -> Option<Arc<File>> {
        self.file.clone()
    }
    fn parent(&self) -> Option<PageRef> {
        self.parent.get().cloned().flatten()
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
    fn content_len(&self, _ctx: HostCtx<'_>) -> Result<i64> {
        Ok(self.len)
    }
    fn render_string(&self, _ctx: HostCtx<'_>, _args: &[Value]) -> Result<Value> {
        unimplemented!()
    }
    // Go: hugolib/page.go:RelatedKeywords
    fn related_keywords(&self, cfg: &IndexConfig) -> Result<Vec<Keyword>> {
        let v = named_page_meta_value(self, &cfg.name)?;
        match v {
            None => Ok(Vec::new()),
            Some(v) => cfg.to_keywords(&v),
        }
    }
    fn ref_(&self, _args: &Map) -> Result<String> {
        unimplemented!()
    }
    fn rel_ref(&self, _args: &Map) -> Result<String> {
        unimplemented!()
    }
    fn related_docs_handler(&self) -> Option<Arc<RelatedDocsHandler>> {
        self.site.related.get().cloned()
    }
    fn fragments_identifiers(&self, _ctx: HostCtx<'_>) -> Option<Vec<String>> {
        Some(self.fragments.clone().unwrap_or_default())
    }
    // Go: hugolib/page.go:ApplyFilterToHeadings
    fn apply_filter_to_headings(
        &self,
        _ctx: HostCtx<'_>,
        filter: &dyn Fn(&str) -> bool,
    ) -> Option<PageRef> {
        let headings: Vec<String> = self
            .headings
            .iter()
            .filter(|h| filter(h))
            .cloned()
            .collect();
        Some(PageRef(Arc::new(self.filtered_copy(headings))))
    }
}

impl FakePage {
    fn filtered_copy(&self, headings: Vec<String>) -> FakePage {
        let parent = OnceLock::new();
        let _ = parent.set(self.parent());
        FakePage {
            entry: self.entry,
            id: self.id,
            type_name: "*hugolib.pageHeadingsFiltered".to_string(),
            site: self.site.clone(),
            weight0: self.weight0,
            ordinal: self.ordinal,
            kind: self.kind.clone(),
            title: self.title.clone(),
            link_title: self.link_title.clone(),
            description: self.description.clone(),
            weight: self.weight,
            date: self.date.clone(),
            lastmod: self.lastmod.clone(),
            publish_date: self.publish_date.clone(),
            expiry_date: self.expiry_date.clone(),
            is_home: self.is_home,
            is_node: self.is_node,
            is_page: self.is_page,
            is_section: self.is_section,
            section: self.section.clone(),
            page_type: self.page_type.clone(),
            layout: self.layout.clone(),
            lang: self.lang.clone(),
            path: self.path.clone(),
            path_info: self.path_info.clone(),
            slug: self.slug.clone(),
            draft: self.draft,
            aliases: self.aliases.clone(),
            keywords: self.keywords.clone(),
            bundle_type: self.bundle_type.clone(),
            name: self.name.clone(),
            translation_key: self.translation_key.clone(),
            rel_permalink: self.rel_permalink.clone(),
            params: self.params.clone(),
            len: self.len,
            file: self.file.clone(),
            fragments: self.fragments.clone(),
            headings: self.headings.clone(),
            headings_filtered: Some(headings),
            parent,
            ancestor_of: OnceLock::new(),
        }
    }
}

/// A loaded build: its sites and page entries.
pub struct Build {
    pub shared: Arc<BuildShared>,
    pub sites: Vec<Arc<FakeSite>>,
    pub pages: Vec<PageRef>,
    pub fakes: Vec<Arc<FakePage>>,
}

impl Build {
    pub fn set_current(&self, i: usize) {
        self.shared.current.store(i, Ordering::SeqCst);
    }

    /// The pages of entry indices.
    pub fn list(&self, v: &J) -> Pages {
        v.as_array()
            .map(|a| a.as_slice())
            .unwrap_or(&[])
            .iter()
            .map(|i| self.pages[i.as_u64().unwrap() as usize].clone())
            .collect()
    }

    /// The entry index of a page (by the fake behind it).
    pub fn index(&self, p: &PageRef) -> i64 {
        match p.0.as_any().downcast_ref::<FakePage>() {
            Some(f) => f.entry as i64,
            None => panic!("not a fake page"),
        }
    }

    pub fn opt_index(&self, p: Option<&PageRef>) -> i64 {
        p.map(|p| self.index(p)).unwrap_or(-1)
    }

    pub fn indices(&self, ps: &Pages) -> J {
        J::Array(ps.iter().map(|p| J::from(self.index(p))).collect())
    }
}

fn s(v: &J) -> String {
    gostring(v)
}

fn strings(v: &J) -> Vec<String> {
    match decode(v) {
        Value::List(l) => l
            .items
            .iter()
            .map(|x| match x {
                Value::String(s) => s.to_str_lossy().into_owned(),
                _ => panic!("not a string"),
            })
            .collect(),
        _ => Vec::new(),
    }
}

fn params_map(v: &J) -> Arc<Map> {
    match decode(v) {
        Value::Map(m) => {
            let mut m = (*m).clone();
            m.ty = MapType::Params;
            Arc::new(m)
        }
        Value::TypedNil(_) | Value::Invalid => Arc::new(Map::new(MapType::Params)),
        other => panic!("params: {}", other.go_type_name()),
    }
}

/// Rebuilds a recorded build from a fixture header (`sites`, `pages`).
pub fn load(h: &J) -> Build {
    nh_page::page::init();
    let shared = Arc::new(BuildShared {
        current: AtomicUsize::new(0),
        sites: OnceLock::new(),
    });
    let sites: Vec<Arc<FakeSite>> = h["sites"]
        .as_array()
        .unwrap()
        .iter()
        .enumerate()
        .map(|(i, s)| {
            let lang = s["lang"].as_str().unwrap();
            let cfg = LanguageConfig {
                weight: s["weight"].as_i64().unwrap(),
                ..Default::default()
            };
            Arc::new(FakeSite {
                idx: i,
                lang: Language::new(lang, "en", "", cfg).unwrap(),
                params: params_map(&s["params"]),
                build: shared.clone(),
                related: OnceLock::new(),
            })
        })
        .collect();
    let _ = shared.sites.set(sites.clone());

    let base = NEXT_ID.fetch_add(1_000_000, Ordering::SeqCst);
    let pp = nh_common::paths::pathparser::PathParser {
        is_content_ext: Some(Arc::new(|_: &str| true)),
        ..Default::default()
    };
    let entries = h["pages"].as_array().unwrap();
    let mut fakes = Vec::with_capacity(entries.len());
    for (i, e) in entries.iter().enumerate() {
        let path_info_s = s(&e["pathInfo"]);
        let path_info = pp.parse("content", &path_info_s);
        assert_eq!(path_info.path(), path_info_s, "path info of entry {i}");
        let file = e.get("filename").map(|f| {
            nh_helpers::source::file_info::File::new(nh_hugofs::fileinfo::new_file_meta_info(
                "",
                false,
                nh_hugofs::fileinfo::FileMeta {
                    filename: s(f),
                    ..Default::default()
                },
            ))
        });
        let opt_strings = |k: &str| {
            e.get(k)
                .map(|v| v.as_array().unwrap().iter().map(s).collect::<Vec<_>>())
        };
        fakes.push(Arc::new(FakePage {
            entry: i,
            id: base + e["pid"].as_u64().unwrap(),
            type_name: e["type"].as_str().unwrap().to_string(),
            site: sites[e["site"].as_u64().unwrap() as usize].clone(),
            weight0: e.get("weight0").map(|v| v.as_i64().unwrap()),
            ordinal: e.get("ordinal").map(|v| v.as_i64().unwrap()),
            kind: s(&e["kind"]),
            title: s(&e["title"]),
            link_title: s(&e["linkTitle"]),
            description: s(&e["description"]),
            weight: e["weight"].as_i64().unwrap(),
            date: decode_time(&e["date"]),
            lastmod: decode_time(&e["lastmod"]),
            publish_date: decode_time(&e["publishDate"]),
            expiry_date: decode_time(&e["expiryDate"]),
            is_home: e["isHome"].as_bool().unwrap(),
            is_node: e["isNode"].as_bool().unwrap(),
            is_page: e["isPage"].as_bool().unwrap(),
            is_section: e["isSection"].as_bool().unwrap(),
            section: s(&e["section"]),
            page_type: s(&e["pageType"]),
            layout: s(&e["layout"]),
            lang: e["lang"].as_str().unwrap().to_string(),
            path: s(&e["path"]),
            path_info: Arc::new(path_info),
            slug: s(&e["slug"]),
            draft: e["draft"].as_bool().unwrap(),
            aliases: strings(&e["aliases"]),
            keywords: strings(&e["keywords"]),
            bundle_type: s(&e["bundleType"]),
            name: s(&e["name"]),
            translation_key: s(&e["translationKey"]),
            rel_permalink: s(&e["relPermalink"]),
            params: params_map(&e["params"]),
            len: e["len"].as_i64().unwrap(),
            file,
            fragments: opt_strings("fragments"),
            headings: opt_strings("headings").unwrap_or_default(),
            headings_filtered: None,
            parent: OnceLock::new(),
            ancestor_of: OnceLock::new(),
        }));
    }
    let pages: Vec<PageRef> = fakes.iter().map(|f| PageRef(f.clone())).collect();
    for (i, e) in entries.iter().enumerate() {
        let parent = e["parent"].as_i64().unwrap();
        let _ = fakes[i].parent.set(if parent < 0 {
            None
        } else {
            Some(pages[parent as usize].clone())
        });
        if let Some(a) = e.get("ancestorOf") {
            let ids = a
                .as_array()
                .map(|a| a.as_slice())
                .unwrap_or(&[])
                .iter()
                .map(|j| fakes[j.as_u64().unwrap() as usize].id)
                .collect();
            let _ = fakes[i].ancestor_of.set(ids);
        }
    }
    Build {
        shared,
        sites,
        pages,
        fakes,
    }
}

/// Encodes a template value like the Go oracle's group keys (`keyEnc`).
pub fn key_json(b: &Build, v: &Value) -> J {
    if let Some(p) = nh_page::page::page_from_value(v) {
        return serde_json::json!({"t": "page", "i": b.index(&p)});
    }
    match v {
        Value::TypedNil(t) if &**t == "page.Page" => serde_json::json!({"t": "nil"}),
        Value::Safe(..) => {
            serde_json::json!({"t": v.go_type_name(), "s": crate::support::enc(v.as_go_string().unwrap().as_bytes())})
        }
        _ => crate::support::encode(v),
    }
}
