//! **Throwaway (T38 walking skeleton).** A flat interim model on top of T23a's [`Model`]: the
//! auto nodes, URLs, lists and relations that T23b (`neohugo-site`) and T24 (`neohugo-nav`)
//! build properly. It exists only so that the skeleton can render a whole site; T33 replaces
//! it with views over the real model and deletes this module.
//!
//! Simplifications (none matter for the testsite):
//! - auto nodes: a missing home, missing *root* sections, taxonomy and term pages, the
//!   standalone pages (404 and sitemap per language, robots.txt and the sitemap index once);
//!   auto nodes get no cascade;
//! - no `[permalinks]`, no `translationKey`, no taxonomy weights, no nested-section lists;
//! - node dates are the maximum of the regular pages below the node;
//! - lists use byte order for link titles (no collation).

use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Arc;

use jiff::Zoned;
use neohugo_base::paths::{ContentKey, OutputPath, TermKey};
use neohugo_base::{ByteOrder, FormatId, IdVec, Idx, LangIdx, PageId, PageKind, Params, Value};
use neohugo_config::sections::SitemapConfig;
use neohugo_config::site::{RobotsPolicy, SiteConfig};
use neohugo_config::{Config, OutputFormat};
use neohugo_page::{
    Dates, LangPrefix, Links, Markup, PathShape, RenderMode, SortKey, SourcePath, TargetPaths,
    UrlInputs, default_order, default_title, links, target_paths,
};
use neohugo_site::{Model, PageRole};

/// Why the flat model could not be built.
#[derive(Debug, thiserror::Error)]
pub enum FlatError {
    #[error("{path} ({format}): {source}")]
    Target {
        path: String,
        format: String,
        #[source]
        source: neohugo_page::PageError,
    },
}

/// A page's content file, as the content phase needs it.
#[derive(Clone, Debug)]
pub struct FlatSource {
    /// The text after the front matter.
    pub body: Arc<str>,
    pub file: Arc<Path>,
    pub markup: Markup,
    /// Front matter `summary`.
    pub summary: Option<String>,
}

/// One rendered format of a page.
#[derive(Clone, Debug)]
pub struct FlatOutput {
    pub format: FormatId,
    pub target: TargetPaths,
    pub links: Links,
}

/// What a page's target paths are computed from.
#[derive(Clone, Debug)]
struct UrlSource {
    source: SourcePath,
    section: String,
    base_name: String,
    url: Option<String>,
    prefix: String,
    force_prefix: bool,
    ugly: bool,
}

/// A page of the flat model: a content page of the [`Model`] (same [`PageId`]) or an auto node.
#[derive(Clone, Debug)]
pub struct FlatPage {
    pub id: PageId,
    pub lang: LangIdx,
    pub kind: PageKind,
    pub key: ContentKey,
    pub title: String,
    pub link_title: String,
    pub description: String,
    /// The first key segment (`posts`).
    pub section: String,
    pub r#type: String,
    pub layout: Option<String>,
    pub dates: Dates,
    pub weight: i32,
    pub draft: bool,
    pub params: Params,
    pub keywords: Vec<String>,
    pub aliases: Vec<String>,
    pub sitemap: SitemapConfig,
    pub source: Option<FlatSource>,
    /// Rendered formats, in render order; the first is the primary one.
    pub outputs: Vec<FlatOutput>,
    /// In site and section lists.
    pub listed: bool,
    pub parent: Option<PageId>,
    /// `.Pages` (default order).
    pub pages: Vec<PageId>,
    /// `.RegularPages` (default order).
    pub regular_pages: Vec<PageId>,
    /// The same page in the other languages, in language order.
    pub translations: Vec<PageId>,
    pub prev_in_section: Option<PageId>,
    pub next_in_section: Option<PageId>,
    /// Term pages: `(taxonomy plural, term name as first written)`.
    pub term: Option<(String, String)>,
    /// The full source path used as the last sort key.
    sort_path: String,
    url: UrlSource,
}

impl FlatPage {
    fn sort_key(&self) -> SortKey<'_> {
        SortKey {
            weight: self.weight,
            date: self.dates.date.as_ref(),
            link_title: &self.link_title,
            path: &self.sort_path,
            ordinal: None,
            weight0: None,
        }
    }

    /// The primary output, if the page is rendered.
    #[must_use]
    pub fn primary(&self) -> Option<&FlatOutput> {
        self.outputs.first()
    }
}

/// A term of a taxonomy.
#[derive(Clone, Debug)]
pub struct FlatTerm {
    /// The term key (`a`, `c-plus-plus`).
    pub key: String,
    pub name: String,
    pub page: PageId,
    pub pages: Vec<PageId>,
}

/// A taxonomy of a language.
#[derive(Clone, Debug)]
pub struct FlatTaxonomy {
    pub singular: String,
    pub plural: String,
    pub page: PageId,
    /// Sorted by key.
    pub terms: Vec<FlatTerm>,
}

/// One language of the flat model.
#[derive(Clone, Debug, Default)]
pub struct FlatLang {
    pub home: Option<PageId>,
    /// `.Site.Pages` (default order).
    pub pages: Vec<PageId>,
    /// `.Site.RegularPages` (default order).
    pub regular_pages: Vec<PageId>,
    /// `.Site.Sections`.
    pub sections: Vec<PageId>,
    pub taxonomies: Vec<FlatTaxonomy>,
    /// 404 and sitemap.
    pub standalone: Vec<PageId>,
}

/// An alias file of a page.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FlatAlias {
    pub from: OutputPath,
    pub to: PageId,
    pub format: FormatId,
}

/// The flat interim model (see the module docs).
#[derive(Clone, Debug)]
pub struct FlatSite {
    pub config: Arc<Config>,
    pub pages: IdVec<PageId, FlatPage>,
    pub langs: IdVec<LangIdx, FlatLang>,
    /// robots.txt and the sitemap index (multilingual sites), rendered once.
    pub root_standalone: Vec<PageId>,
}

fn formats_of<'a>(
    cfg: &'a Config,
    ids: &[FormatId],
) -> impl Iterator<Item = (FormatId, &'a OutputFormat)> {
    ids.iter().map(|&f| (f, cfg.output_formats.get(f)))
}

fn string_list(v: &Value) -> Vec<String> {
    match v {
        Value::String(s) => vec![s.to_string()],
        Value::Array(a) => a
            .iter()
            .filter_map(Value::as_str)
            .map(str::to_owned)
            .collect(),
        _ => Vec::new(),
    }
}

fn max_date<'a>(dates: impl Iterator<Item = Option<&'a Zoned>>) -> Option<Zoned> {
    dates.flatten().max_by_key(|d| d.timestamp()).cloned()
}

impl FlatSite {
    /// Builds the flat model of `model`.
    ///
    /// # Errors
    /// A target path that cannot be computed (a front matter `url` with a broken escape).
    pub fn build(model: &Model) -> Result<Self, FlatError> {
        let cfg = Arc::clone(&model.config);
        let mut b = Builder {
            cfg: &cfg,
            pages: IdVec::new(),
            keys: model.sites.iter().map(|_| BTreeMap::new()).collect(),
        };
        for p in &model.pages {
            let listed = p.meta.build.list != neohugo_page::ListMode::Never;
            let fp = b.content_page(p, listed);
            let id = b.pages.push(fp);
            debug_assert_eq!(id, p.id, "flat ids follow the model's");
            if p.role == PageRole::Standalone {
                b.keys[p.lang].insert(p.key.clone(), id);
            }
        }
        let mut langs: IdVec<LangIdx, FlatLang> = IdVec::new();
        for (lang, site) in cfg.sites.iter_enumerated() {
            langs.push(b.auto_nodes(lang, site));
        }
        let mut root_standalone = Vec::new();
        let default = LangIdx::from_index(0);
        if cfg.default_site().robots_txt == RobotsPolicy::Enabled {
            root_standalone.extend(b.standalone(default, PageKind::RobotsTxt));
        }
        if cfg.sites.len() > 1 {
            root_standalone.extend(b.standalone(default, PageKind::SitemapIndex));
        }
        let mut site = Self {
            config: Arc::clone(&cfg),
            pages: b.pages,
            langs,
            root_standalone,
        };
        site.relate(model);
        site.compute_outputs()?;
        site.apply_front_matter_outputs(model)?;
        Ok(site)
    }

    /// The default order of `ids`.
    fn sort(&self, ids: &mut [PageId]) {
        ids.sort_by(|&a, &b| self.order(a, b));
    }

    fn order(&self, a: PageId, b: PageId) -> Ordering {
        default_order(
            &self.pages[a].sort_key(),
            &self.pages[b].sort_key(),
            &ByteOrder,
        )
    }

    /// Parents, lists, node dates, prev/next and translations.
    fn relate(&mut self, model: &Model) {
        let is_listed_content = |p: &FlatPage| {
            p.listed
                && matches!(
                    p.kind,
                    PageKind::Home
                        | PageKind::Section
                        | PageKind::Page
                        | PageKind::Taxonomy
                        | PageKind::Term
                )
        };
        // Parents: the nearest ancestor key in the language (terms: their taxonomy).
        let mut by_key: IdVec<LangIdx, BTreeMap<ContentKey, PageId>> =
            self.langs.iter().map(|_| BTreeMap::new()).collect();
        for p in &self.pages {
            if is_listed_content(p)
                || matches!(
                    p.kind,
                    PageKind::Home | PageKind::Section | PageKind::Taxonomy
                )
            {
                by_key[p.lang].entry(p.key.clone()).or_insert(p.id);
            }
        }
        let ids: Vec<PageId> = self.pages.iter().map(|p| p.id).collect();
        for &id in &ids {
            let p = &self.pages[id];
            if !matches!(
                p.kind,
                PageKind::Section | PageKind::Page | PageKind::Taxonomy | PageKind::Term
            ) || model
                .pages
                .get(id)
                .is_some_and(|m| m.role != PageRole::Standalone)
            {
                continue;
            }
            let mut k = p.key.parent();
            let mut parent = None;
            while let Some(key) = k {
                if let Some(&pid) = by_key[p.lang].get(&key) {
                    parent = Some(pid);
                    break;
                }
                k = key.parent();
            }
            self.pages[id].parent = parent;
        }

        // Node dates: the newest regular page below the node.
        for &id in &ids {
            let p = &self.pages[id];
            if !p.kind.is_branch() || p.dates.date.is_some() {
                continue;
            }
            let below: Vec<PageId> = self
                .pages
                .iter()
                .filter(|q| q.lang == p.lang && q.kind == PageKind::Page && q.listed)
                .filter(|q| match p.kind {
                    PageKind::Term | PageKind::Taxonomy => false,
                    _ => q.key.starts_with_segments(&p.key),
                })
                .map(|q| q.id)
                .collect();
            let date = max_date(below.iter().map(|&q| self.pages[q].dates.date.as_ref()));
            let lastmod = max_date(below.iter().map(|&q| self.pages[q].dates.lastmod.as_ref()));
            let d = &mut self.pages[id].dates;
            if d.publish_date.is_none() {
                d.publish_date.clone_from(&date);
            }
            d.date = date;
            d.lastmod = lastmod.or_else(|| d.date.clone());
        }
        // Term and taxonomy dates from their members.
        for lang in 0..self.langs.len() {
            let lang = LangIdx::from_index(lang);
            for t in self.langs[lang].taxonomies.clone() {
                for term in &t.terms {
                    let date = max_date(
                        term.pages
                            .iter()
                            .map(|&q| self.pages[q].dates.date.as_ref()),
                    );
                    let lastmod = max_date(
                        term.pages
                            .iter()
                            .map(|&q| self.pages[q].dates.lastmod.as_ref()),
                    );
                    let d = &mut self.pages[term.page].dates;
                    d.publish_date.clone_from(&date);
                    d.date = date;
                    d.lastmod = lastmod;
                }
                let date = max_date(
                    t.terms
                        .iter()
                        .map(|x| self.pages[x.page].dates.date.as_ref()),
                );
                let lastmod = max_date(
                    t.terms
                        .iter()
                        .map(|x| self.pages[x.page].dates.lastmod.as_ref()),
                );
                let d = &mut self.pages[t.page].dates;
                d.publish_date.clone_from(&date);
                d.date = date;
                d.lastmod = lastmod;
            }
        }

        // Children lists.
        let mut children: BTreeMap<PageId, Vec<PageId>> = BTreeMap::new();
        for p in &self.pages {
            if let Some(parent) = p.parent
                && is_listed_content(p)
                && p.kind != PageKind::Term
            {
                children.entry(parent).or_default().push(p.id);
            }
        }
        for (parent, mut kids) in children {
            self.sort(&mut kids);
            let regular = kids
                .iter()
                .copied()
                .filter(|&k| self.pages[k].kind == PageKind::Page)
                .collect();
            let p = &mut self.pages[parent];
            p.pages = kids;
            p.regular_pages = regular;
        }
        for lang in 0..self.langs.len() {
            let lang = LangIdx::from_index(lang);
            let mut taxonomies = std::mem::take(&mut self.langs[lang].taxonomies);
            for t in &mut taxonomies {
                let mut terms: Vec<PageId> = t.terms.iter().map(|x| x.page).collect();
                self.sort(&mut terms);
                self.pages[t.page].pages = terms;
                for term in &mut t.terms {
                    self.sort(&mut term.pages);
                    let page = &mut self.pages[term.page];
                    page.pages.clone_from(&term.pages);
                    page.regular_pages.clone_from(&term.pages);
                }
            }
            self.langs[lang].taxonomies = taxonomies;

            let mut all: Vec<PageId> = self
                .pages
                .iter()
                .filter(|p| p.lang == lang && is_listed_content(p))
                .map(|p| p.id)
                .collect();
            self.sort(&mut all);
            let regular: Vec<PageId> = all
                .iter()
                .copied()
                .filter(|&p| self.pages[p].kind == PageKind::Page)
                .collect();
            let sections = all
                .iter()
                .copied()
                .filter(|&p| {
                    let q = &self.pages[p];
                    q.kind == PageKind::Section && q.parent == self.langs[lang].home
                })
                .collect();
            let l = &mut self.langs[lang];
            l.pages = all;
            l.regular_pages = regular;
            l.sections = sections;
        }

        // Prev/next in section: the list is newest first; "prev" is the older neighbour.
        for &id in &ids {
            let list = self.pages[id].regular_pages.clone();
            if self.pages[id].kind != PageKind::Section && self.pages[id].kind != PageKind::Home {
                continue;
            }
            for (i, &p) in list.iter().enumerate() {
                let page = &mut self.pages[p];
                page.prev_in_section = list.get(i + 1).copied();
                page.next_in_section = i.checked_sub(1).map(|j| list[j]);
            }
        }

        // Translations: the same (kind, key) in the other languages.
        let mut groups: BTreeMap<(PageKind, ContentKey), Vec<PageId>> = BTreeMap::new();
        for p in &self.pages {
            if matches!(
                p.kind,
                PageKind::Home
                    | PageKind::Section
                    | PageKind::Page
                    | PageKind::Taxonomy
                    | PageKind::Term
            ) && model
                .pages
                .get(p.id)
                .is_none_or(|m| m.role == PageRole::Standalone)
            {
                groups
                    .entry((p.kind, p.key.clone()))
                    .or_default()
                    .push(p.id);
            }
        }
        for ids in groups.into_values() {
            let mut ids = ids;
            ids.sort_by_key(|&p| self.pages[p].lang);
            for &p in &ids {
                self.pages[p].translations = ids.iter().copied().filter(|&q| q != p).collect();
            }
        }
    }

    /// The output formats and target paths of every page.
    fn compute_outputs(&mut self) -> Result<(), FlatError> {
        let cfg = Arc::clone(&self.config);
        for i in 0..self.pages.len() {
            let id = PageId::from_index(i);
            let p = &self.pages[id];
            let site = &cfg.sites[p.lang];
            let formats: Vec<FormatId> = match p.kind {
                PageKind::Page if p.source.is_none() => Vec::new(),
                _ => site.outputs.get(p.kind).to_vec(),
            };
            let mut outputs = Vec::new();
            for (f, _) in formats_of(&cfg, &formats) {
                let (target, links) = self.target(id, f, None)?;
                outputs.push(FlatOutput {
                    format: f,
                    target,
                    links,
                });
            }
            self.pages[id].outputs = outputs;
        }
        Ok(())
    }

    /// Replaces the output formats of page `id` (front matter `outputs`).
    fn set_formats(&mut self, id: PageId, formats: &[FormatId]) -> Result<(), FlatError> {
        let mut outputs = Vec::new();
        for &f in formats {
            let (target, links) = self.target(id, f, None)?;
            outputs.push(FlatOutput {
                format: f,
                target,
                links,
            });
        }
        self.pages[id].outputs = outputs;
        Ok(())
    }

    /// The target paths and links of page `id` in format `f`, for pager `pager` (≥ 2) or the
    /// page itself.
    ///
    /// # Errors
    /// A front matter `url` with a broken escape.
    pub fn target(
        &self,
        id: PageId,
        f: FormatId,
        pager: Option<u32>,
    ) -> Result<(TargetPaths, Links), FlatError> {
        let p = &self.pages[id];
        let site = &self.config.sites[p.lang];
        let format = self.config.output_formats.get(f);
        let suffix = self.config.media_types.get(format.media_type).full_suffix();
        let urls = site.site_urls();
        let pager_path = pager.map(|n| format!("/{}/{n}", site.pagination.path));
        let u = &p.url;
        let err = |source| FlatError::Target {
            path: p.key.to_path(),
            format: format.name.clone(),
            source,
        };
        let t = target_paths(&UrlInputs {
            kind: p.kind,
            format,
            suffix: &suffix,
            source: &u.source,
            section: &u.section,
            base_name: &u.base_name,
            prefix: LangPrefix {
                file: &u.prefix,
                link: &u.prefix,
                force: u.force_prefix,
            },
            url: u.url.as_deref(),
            pager: pager_path.as_deref(),
            permalink: None,
            site_ugly: u.ugly,
            urls: &urls,
        })
        .map_err(err)?;
        let l = links(&t, &urls, format).map_err(err)?;
        Ok((t, l))
    }

    /// The list `paginator()` pages through by default (semantics §15.2): the site's regular
    /// pages for the home page and standalone pages, the section's regular pages, the
    /// taxonomy's or term's pages.
    #[must_use]
    pub fn pagination_list(&self, id: PageId) -> &[PageId] {
        let p = &self.pages[id];
        match p.kind {
            PageKind::Section => &p.regular_pages,
            PageKind::Taxonomy | PageKind::Term => &p.pages,
            _ => &self.langs[p.lang].regular_pages,
        }
    }

    /// The alias files of every rendered page (front matter `aliases`), in page order.
    #[must_use]
    pub fn aliases(&self) -> Vec<FlatAlias> {
        let mut out = Vec::new();
        for p in &self.pages {
            let Some(primary) = p.primary() else { continue };
            let site = &self.config.sites[p.lang];
            if site.aliases != neohugo_config::site::AliasPolicy::Write {
                continue;
            }
            let prefix = &site.language.url_prefix;
            for a in &p.aliases {
                let mut path = if a.starts_with('/') {
                    a.clone()
                } else {
                    format!("/{a}")
                };
                if !prefix.is_empty() && !path.starts_with(&format!("/{prefix}/")) {
                    path = format!("/{prefix}{path}");
                }
                if path.ends_with('/')
                    || Path::new(&path)
                        .extension()
                        .is_none_or(std::ffi::OsStr::is_empty)
                {
                    if !path.ends_with('/') {
                        path.push('/');
                    }
                    path.push_str("index.html");
                }
                out.push(FlatAlias {
                    from: OutputPath::new(&path),
                    to: p.id,
                    format: primary.format,
                });
            }
        }
        out
    }
}

/// The build state of [`FlatSite::build`].
struct Builder<'a> {
    cfg: &'a Arc<Config>,
    pages: IdVec<PageId, FlatPage>,
    keys: IdVec<LangIdx, BTreeMap<ContentKey, PageId>>,
}

impl Builder<'_> {
    fn url_source(
        site: &SiteConfig,
        kind: PageKind,
        key: &ContentKey,
        source: SourcePath,
        base_name: String,
    ) -> UrlSource {
        let section = if kind.is_branch() {
            key.to_path()
        } else if key.segments().nth(1).is_some() {
            format!("/{}", key.first_segment())
        } else {
            "/".to_owned()
        };
        let sitemap_in_lang_dir = kind == PageKind::Sitemap && site.language.url_prefix.is_empty();
        UrlSource {
            ugly: site.urls.ugly.in_section(key.first_segment()),
            section,
            source,
            base_name,
            url: None,
            prefix: if sitemap_in_lang_dir {
                site.language.key.clone()
            } else {
                site.language.url_prefix.clone()
            },
            force_prefix: kind == PageKind::Sitemap,
        }
    }

    fn content_page(&self, p: &neohugo_site::Page, listed: bool) -> FlatPage {
        let site = &self.cfg.sites[p.lang];
        let m = &p.meta;
        let (source, sort_path, src_path) = match &p.source {
            Some(s) => (
                Some(FlatSource {
                    body: Arc::from(s.body()),
                    file: Arc::from(s.file.abs.as_path()),
                    markup: m.markup,
                    summary: m.summary.clone(),
                }),
                s.info.path.clone(),
                SourcePath::from_path_info(&s.info, site.urls.path_case),
            ),
            None => (None, p.key.to_path(), SourcePath::from_key(&p.key)),
        };
        let base_name = m.slug.clone().unwrap_or_else(|| src_path.name.clone());
        let mut url = Self::url_source(site, p.kind, &p.key, src_path, base_name);
        url.url.clone_from(&m.url);
        let title = m.title.clone().unwrap_or_else(|| match p.kind {
            PageKind::Home => site.title.clone(),
            _ => String::new(),
        });
        let section = p.key.first_segment().to_owned();
        let render = m.build.render != RenderMode::Never;
        FlatPage {
            id: p.id,
            lang: p.lang,
            kind: p.kind,
            key: p.key.clone(),
            link_title: m.link_title.clone().unwrap_or_else(|| title.clone()),
            title,
            description: m.description.clone(),
            r#type: m.r#type.clone().unwrap_or_else(|| {
                if section.is_empty() {
                    "page".to_owned()
                } else {
                    section.clone()
                }
            }),
            section,
            layout: m.layout.clone(),
            dates: m.dates.clone(),
            weight: m.weight,
            draft: m.draft,
            params: m.params.clone(),
            keywords: m.keywords.clone(),
            aliases: m.aliases.clone(),
            sitemap: m.sitemap.clone(),
            source: if render && p.role == PageRole::Standalone {
                source
            } else {
                None
            },
            outputs: Vec::new(),
            listed: listed && p.role == PageRole::Standalone,
            parent: None,
            pages: Vec::new(),
            regular_pages: Vec::new(),
            translations: Vec::new(),
            prev_in_section: None,
            next_in_section: None,
            term: None,
            sort_path,
            url,
        }
    }

    fn auto_page(
        &mut self,
        lang: LangIdx,
        kind: PageKind,
        key: ContentKey,
        title: String,
    ) -> PageId {
        let site = &self.cfg.sites[lang];
        let (source, base_name) = match kind {
            PageKind::NotFound
            | PageKind::Sitemap
            | PageKind::SitemapIndex
            | PageKind::RobotsTxt => {
                // The format's base name; the 404 format has none and is named after its kind.
                let f = site
                    .outputs
                    .get(kind)
                    .first()
                    .map(|&f| self.cfg.output_formats.get(f).base_name.clone())
                    .filter(|b| !b.is_empty())
                    .unwrap_or_else(|| kind.as_str().to_owned());
                (
                    SourcePath {
                        dir: "/".to_owned(),
                        name: f.clone(),
                        shape: PathShape::File,
                    },
                    f,
                )
            }
            _ => {
                let s = SourcePath::from_key(&key);
                let name = s.name.clone();
                (s, name)
            }
        };
        let url = Self::url_source(site, kind, &key, source, base_name);
        let section = key.first_segment().to_owned();
        let id = self.pages.next_id();
        self.pages.push(FlatPage {
            id,
            lang,
            kind,
            sort_path: key.to_path(),
            key: key.clone(),
            link_title: title.clone(),
            title,
            description: String::new(),
            r#type: if section.is_empty() {
                kind.as_str().to_owned()
            } else {
                section.clone()
            },
            section,
            layout: None,
            dates: Dates::default(),
            weight: 0,
            draft: false,
            params: Params::default(),
            keywords: Vec::new(),
            aliases: Vec::new(),
            sitemap: site.sitemap.clone(),
            source: None,
            outputs: Vec::new(),
            listed: !matches!(
                kind,
                PageKind::NotFound
                    | PageKind::Sitemap
                    | PageKind::SitemapIndex
                    | PageKind::RobotsTxt
            ),
            parent: None,
            pages: Vec::new(),
            regular_pages: Vec::new(),
            translations: Vec::new(),
            prev_in_section: None,
            next_in_section: None,
            term: None,
            url,
        });
        if !key.is_home() || kind == PageKind::Home {
            self.keys[lang].entry(key).or_insert(id);
        }
        id
    }

    fn standalone(&mut self, lang: LangIdx, kind: PageKind) -> Option<PageId> {
        let site = &self.cfg.sites[lang];
        if site.outputs.get(kind).is_empty() {
            return None;
        }
        let title = default_title(kind, "", &site.titles);
        Some(self.auto_page(lang, kind, ContentKey::home(), title))
    }

    /// Adds the auto nodes of `lang`.
    fn auto_nodes(&mut self, lang: LangIdx, site: &SiteConfig) -> FlatLang {
        let mut l = FlatLang::default();
        let home = match self.keys[lang].get(&ContentKey::home()).copied() {
            Some(h) if self.pages[h].kind == PageKind::Home => h,
            _ => self.auto_page(lang, PageKind::Home, ContentKey::home(), site.title.clone()),
        };
        l.home = Some(home);

        let regular: Vec<PageId> = self
            .pages
            .iter()
            .filter(|p| p.lang == lang && p.kind == PageKind::Page && p.listed)
            .map(|p| p.id)
            .collect();
        for &p in &regular {
            let key = &self.pages[p].key;
            if key.segments().nth(1).is_none() {
                continue;
            }
            let section = ContentKey::from_source(key.first_segment());
            if !self.keys[lang].contains_key(&section) {
                let title = default_title(PageKind::Section, section.as_ref(), &site.titles);
                self.auto_page(lang, PageKind::Section, section, title);
            }
        }

        for t in &site.taxonomies {
            let tkey = ContentKey::from_source(&t.plural);
            let page = match self.keys[lang].get(&tkey).copied() {
                Some(p) => p,
                None => {
                    let title = default_title(PageKind::Taxonomy, &t.plural, &site.titles);
                    self.auto_page(lang, PageKind::Taxonomy, tkey, title)
                }
            };
            let mut terms: BTreeMap<String, FlatTerm> = BTreeMap::new();
            for &p in &regular {
                let Some(v) = self.pages[p].params.get(&t.plural) else {
                    continue;
                };
                for name in string_list(v) {
                    let tk = TermKey::new(&t.plural, &name);
                    let key = ContentKey::from_source(tk.as_ref());
                    let term_key = key.segments().last().unwrap_or_default().to_owned();
                    let entry = match terms.entry(term_key.clone()) {
                        std::collections::btree_map::Entry::Occupied(e) => e.into_mut(),
                        std::collections::btree_map::Entry::Vacant(e) => {
                            let page = match self.keys[lang].get(&key).copied() {
                                Some(tp) => tp,
                                None => {
                                    let title = default_title(PageKind::Term, &name, &site.titles);
                                    self.auto_page(lang, PageKind::Term, key, title)
                                }
                            };
                            self.pages[page].term = Some((t.plural.clone(), name.clone()));
                            e.insert(FlatTerm {
                                key: term_key,
                                name,
                                page,
                                pages: Vec::new(),
                            })
                        }
                    };
                    if !entry.pages.contains(&p) {
                        entry.pages.push(p);
                    }
                }
            }
            l.taxonomies.push(FlatTaxonomy {
                singular: t.singular.clone(),
                plural: t.plural.clone(),
                page,
                terms: terms.into_values().collect(),
            });
        }

        l.standalone
            .extend(self.standalone(lang, PageKind::NotFound));
        l.standalone
            .extend(self.standalone(lang, PageKind::Sitemap));
        l
    }
}

impl FlatSite {
    /// Applies front matter `outputs` (auto nodes keep the site's).
    fn apply_front_matter_outputs(&mut self, model: &Model) -> Result<(), FlatError> {
        for p in &model.pages {
            if let Some(formats) = &p.meta.outputs
                && self.pages[p.id].source.is_some()
            {
                self.set_formats(p.id, formats)?;
            }
        }
        Ok(())
    }
}
