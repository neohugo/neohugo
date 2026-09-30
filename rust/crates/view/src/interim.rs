//! **Throwaway (T38 walking skeleton).** A flat interim model on top of the site [`Model`]: the
//! page and site fields the skeleton's views read, copied out of the model (T23b's made pages,
//! titles, URLs, relations, lists, taxonomies and node dates), plus the few things the model
//! leaves to later tasks (prev/next in section and front matter alias files: T24's). It exists
//! only so that the skeleton can render a whole site; T33 replaces it with views over the real
//! model and deletes this module.

use std::path::Path;
use std::sync::Arc;

use neohugo_base::paths::ContentKey;
use neohugo_base::{FormatId, IdVec, Idx, LangIdx, OutputPath, PageId, PageKind, Params};
use neohugo_config::Config;
use neohugo_config::sections::SitemapConfig;
use neohugo_page::{Dates, Links, Markup, TargetPaths, links};
use neohugo_site::{Model, ModelError, PageRole};

/// Why the flat model could not be built, or a pager's URL computed.
#[derive(Debug, thiserror::Error)]
pub enum FlatError {
    #[error("{path} ({format}): {source}")]
    Target {
        path: String,
        format: String,
        #[source]
        source: neohugo_page::PageError,
    },
    #[error("{path} has no output in format {format}")]
    NoOutput { path: String, format: String },
    #[error(transparent)]
    Model(#[from] Box<ModelError>),
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

/// A page of the flat model: the [`Model`]'s page with the same [`PageId`].
#[derive(Clone, Debug)]
pub struct FlatPage {
    pub id: PageId,
    pub lang: LangIdx,
    pub kind: PageKind,
    pub key: ContentKey,
    /// `.Path` (`/posts/one`, `/`, `/404`).
    pub path: String,
    pub title: String,
    pub link_title: String,
    pub description: String,
    /// `.Section`: the first path segment (`posts`).
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
    /// `.RelPermalink`/`.Permalink` in the primary format (`None`: no link).
    pub links: Option<Links>,
    pub parent: Option<PageId>,
    /// `.Pages` (default order; standalone pages: the site's pages).
    pub pages: Vec<PageId>,
    /// `.RegularPages` (default order).
    pub regular_pages: Vec<PageId>,
    /// `.Sections`.
    pub sections: Vec<PageId>,
    /// The same page in the other languages, in language order.
    pub translations: Vec<PageId>,
    pub prev_in_section: Option<PageId>,
    pub next_in_section: Option<PageId>,
    /// `.GetTerms` per taxonomy of [`FlatLang::taxonomies`] (same order): the term pages.
    pub terms: Vec<Vec<PageId>>,
}

impl FlatPage {
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
    /// `.Data.Term`.
    pub name: String,
    pub page: PageId,
    /// The members (weight, then default order).
    pub pages: Vec<PageId>,
}

/// A taxonomy of a language.
#[derive(Clone, Debug)]
pub struct FlatTaxonomy {
    pub singular: String,
    pub plural: String,
    pub page: Option<PageId>,
    /// The terms of `.Site.Taxonomies`, by key.
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
    model: Arc<Model>,
}

impl FlatSite {
    /// Builds the flat model of `model`.
    ///
    /// # Errors
    /// None today (the model computed every URL); kept for the pager URLs of
    /// [`target`](Self::target).
    pub fn build(model: &Arc<Model>) -> Result<Self, FlatError> {
        let cfg = Arc::clone(&model.config);
        let mut langs: IdVec<LangIdx, FlatLang> = IdVec::new();
        for site in &model.sites {
            let taxonomies = site
                .taxonomies
                .iter()
                .map(|t| FlatTaxonomy {
                    singular: t.def.singular.clone(),
                    plural: t.def.plural.clone(),
                    page: t.page,
                    terms: t
                        .listed_terms(model)
                        .map(|(_, term)| FlatTerm {
                            key: term.key.segments().last().unwrap_or_default().to_owned(),
                            name: term.term.clone(),
                            page: term.page,
                            pages: term.members.iter().map(|w| w.page).collect(),
                        })
                        .collect(),
                })
                .collect();
            langs.push(FlatLang {
                home: Some(site.home),
                pages: site.pages.clone(),
                regular_pages: site.regular_pages.clone(),
                sections: model.pages[site.home].sections.clone(),
                taxonomies,
                standalone: Vec::new(),
            });
        }

        let mut root_standalone = Vec::new();
        let mut pages: IdVec<PageId, FlatPage> = IdVec::new();
        for p in &model.pages {
            match p.kind {
                PageKind::RobotsTxt | PageKind::SitemapIndex if p.lang.index() == 0 => {
                    root_standalone.push(p.id);
                }
                _ if p.standalone.is_some() => langs[p.lang].standalone.push(p.id),
                _ => {}
            }
            let id = pages.push(Self::page(model, p));
            debug_assert_eq!(id, p.id, "flat ids follow the model's");
        }

        // Prev/next in section (T24's): a section's regular pages are newest first; "prev" is
        // the older neighbour.
        for p in &model.pages {
            if !matches!(p.kind, PageKind::Section | PageKind::Home) {
                continue;
            }
            let list = &p.regular_pages;
            for (i, &q) in list.iter().enumerate() {
                let page = &mut pages[q];
                page.prev_in_section = list.get(i + 1).copied();
                page.next_in_section = i.checked_sub(1).map(|j| list[j]);
            }
        }

        Ok(Self {
            config: cfg,
            pages,
            langs,
            root_standalone,
            model: Arc::clone(model),
        })
    }

    /// The flat page of the model's page `p`.
    fn page(model: &Model, p: &neohugo_site::Page) -> FlatPage {
        let m = &p.meta;
        let rendered = p.rendered();
        let source = p
            .source
            .as_ref()
            .filter(|_| rendered && p.role == PageRole::Standalone);
        let outputs = if rendered {
            p.urls
                .iter()
                .filter_map(|u| {
                    u.links.as_ref().map(|l| FlatOutput {
                        format: u.format,
                        target: u.paths.clone(),
                        links: l.clone(),
                    })
                })
                .collect()
        } else {
            Vec::new()
        };
        let terms = model.sites[p.lang]
            .taxonomies
            .ids()
            .map(|t| {
                p.terms
                    .iter()
                    .filter(|&&(tx, _)| tx == t)
                    .map(|&(tx, term)| model.sites[p.lang].taxonomies[tx].terms[term].page)
                    .collect()
            })
            .collect();
        FlatPage {
            id: p.id,
            lang: p.lang,
            kind: p.kind,
            key: p.key.clone(),
            path: p.path(),
            title: p.title.clone(),
            link_title: p.link_title.clone(),
            description: m.description.clone(),
            section: p.section.clone(),
            r#type: p.r#type.clone(),
            layout: m.layout.clone(),
            dates: m.dates.clone(),
            weight: m.weight,
            draft: m.draft,
            params: m.params.clone(),
            keywords: m.keywords.clone(),
            aliases: m.aliases.clone(),
            sitemap: m.sitemap.clone(),
            source: source.map(|s| FlatSource {
                body: Arc::from(s.body()),
                file: Arc::from(s.file.abs.as_path()),
                markup: m.markup,
                summary: m.summary.clone(),
            }),
            outputs,
            links: p.links().cloned(),
            parent: p.parent,
            pages: p.pages.clone(),
            regular_pages: p.regular_pages.clone(),
            sections: p.sections.clone(),
            translations: p
                .translations
                .iter()
                .copied()
                .filter(|&q| q != p.id)
                .collect(),
            prev_in_section: None,
            next_in_section: None,
            terms,
        }
    }

    /// The target paths and links of page `id` in format `f`, for pager `pager` (≥ 2) or the
    /// page itself.
    ///
    /// # Errors
    /// The page has no output in `f`, or the pager's URL cannot be made.
    pub fn target(
        &self,
        id: PageId,
        f: FormatId,
        pager: Option<u32>,
    ) -> Result<(TargetPaths, Links), FlatError> {
        let p = &self.pages[id];
        let site = &self.config.sites[p.lang];
        let format = self.config.output_formats.get(f);
        let t = match pager {
            Some(n) => {
                let pager = format!("/{}/{n}", site.pagination.path);
                self.model.pager_paths(id, f, &pager).map_err(Box::new)?
            }
            None => match self.model.pages[id].url(f) {
                Some(u) => u.paths.clone(),
                None => {
                    return Err(FlatError::NoOutput {
                        path: p.path.clone(),
                        format: format.name.clone(),
                    });
                }
            },
        };
        let l = links(&t, &site.site_urls(), format).map_err(|source| FlatError::Target {
            path: p.path.clone(),
            format: format.name.clone(),
            source,
        })?;
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
