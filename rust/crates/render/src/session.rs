//! The render [`Session`] (REWRITE_PLAN.md §2.6): templates, views and the named mutable build
//! state of phases C–E. `Session::{new, render_content, freeze_views, render_job}` are frozen by
//! T38; their bodies are the skeleton's.

use std::collections::BTreeMap;
use std::sync::{Arc, OnceLock};

use neohugo_base::diag::{Diagnostic, Diagnostics};
use neohugo_base::paths::{ContentKey, OutputPath};
use neohugo_base::{Clock, FormatId, IdVec, Idx, LangIdx, PageId, PageKind};
use neohugo_funcs::{Locales, PureEnv, register_placeholders, register_pure};
use neohugo_layouts::{
    LayoutQuery, LayoutStore, Selection, Selections, StandaloneKind, TemplateName, TemplateRole,
    Templates,
};
use neohugo_markup::Fragments;
use neohugo_resources::{ResourceStore, StoreConfig};
use neohugo_site::{Model, Page, PageRole, PageUrl};
use neohugo_vfs::Vfs;
use neohugo_view::views::HugoView;
use neohugo_view::{
    ContentError, ContentRenderer, Contents, ExpandedSource, HookVariant, NavSite,
    PaginationRecorder, Phase, RenderScope, RenderStringOptions, RenderedContent, SCOPE_KEY,
    ViewCache, ViewInputs, page_target,
};
use rayon::prelude::*;

use crate::job::{AliasPlan, Job, JobOrder, Output};
use crate::stubs::Stubs;
use crate::{RenderError, content};

/// The project inputs a session renders besides the model: the file system and the scanned
/// layouts.
#[derive(Clone, Debug)]
pub struct Project {
    pub vfs: Arc<Vfs>,
    pub layouts: Arc<LayoutStore>,
}

/// Build-wide render settings.
#[derive(Clone, Copy, Debug)]
pub struct RenderOptions {
    /// `now()` and the build's "now" (`--clock`).
    pub clock: Clock,
}

impl Default for RenderOptions {
    fn default() -> Self {
        Self {
            clock: Clock::system(),
        }
    }
}

/// The render state of one build. Created once; phases call it in order:
/// [`render_content`](Self::render_content) (C1), [`freeze_views`](Self::freeze_views) (D),
/// then [`render_job`](Self::render_job) for every job of waves 1 and 2 (E2, E3).
pub struct Session {
    model: Arc<Model>,
    project: Project,
    templates: Arc<Templates>,
    views: Arc<ViewCache>,
    pagination: Arc<PaginationRecorder>,
    selections: BTreeMap<(PageId, FormatId), Selection>,
    /// Front matter alias files per language (`neohugo_nav::page_aliases`).
    aliases: IdVec<LangIdx, Vec<AliasPlan>>,
    contents: OnceLock<BTreeMap<HookVariant, Contents>>,
    hugo: tera::Value,
    diagnostics: Arc<Diagnostics>,
}

impl std::fmt::Debug for Session {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Session")
            .field("pages", &self.model.pages.len())
            .field("templates", &self.templates)
            .finish_non_exhaustive()
    }
}

/// The pure-function environment of a model.
fn pure_env(model: &Model, o: &RenderOptions, diagnostics: &Arc<Diagnostics>) -> PureEnv {
    let cfg = &model.config;
    let site = cfg.default_site();
    let mut env = PureEnv::new(&site.language.key);
    env.clock = o.clock;
    env.time_zone = site.language.time_zone.clone();
    env.locales = Locales::new(
        &site.language.key,
        cfg.sites.iter().map(|s| s.language.key.as_str()),
    );
    env.title_style = site.titles.case_style;
    env.path_case = site.urls.path_case;
    env.accents = site.urls.accents;
    env.diagnostics = Arc::clone(diagnostics);
    env.project_dir = Some(cfg.project_dir.clone());
    env
}

/// The outputs a page is rendered in (none unless it is written), the primary first.
fn outputs(p: &Page) -> impl Iterator<Item = &PageUrl> {
    p.urls
        .iter()
        .filter(move |u| p.rendered() && u.links.is_some())
}

/// The layout query of page `p` in `format`.
fn layout_query<'a>(p: &'a Page, path: &'a ContentKey, format: FormatId) -> LayoutQuery<'a> {
    let standalone = matches!(
        p.kind,
        PageKind::NotFound | PageKind::Sitemap | PageKind::SitemapIndex | PageKind::RobotsTxt
    );
    LayoutQuery {
        path,
        kind: Some(p.kind),
        layout: p.meta.layout.as_deref(),
        exact_layout: false,
        lang: (!standalone).then_some(p.lang),
        format,
    }
}

/// The lookup path of a page: its key with the first segment replaced by its type when that
/// differs from the section.
fn lookup_path(p: &Page) -> ContentKey {
    if p.section.is_empty() || p.r#type == p.section || p.r#type == "page" {
        return p.key.clone();
    }
    let rest: Vec<&str> = p.key.segments().skip(1).collect();
    let mut path = p.r#type.clone();
    for s in rest {
        path.push('/');
        path.push_str(s);
    }
    ContentKey::from_source(&path)
}

fn is_standalone(kind: PageKind) -> bool {
    matches!(
        kind,
        PageKind::NotFound | PageKind::Sitemap | PageKind::SitemapIndex | PageKind::RobotsTxt
    )
}

impl Session {
    /// Loads the templates and prepares the views of `model`: the site functions are
    /// registered, then the layouts are loaded (validating every template).
    ///
    /// The skeleton's stubs need no content, so it has no renderer slot yet. T35 adds it in
    /// the plan's order: slot created empty → `sitefuncs::register` → `layouts::load` →
    /// `Arc::new(Session)` → `slot.set(Arc::downgrade(&session))` (a `Weak<dyn
    /// ContentRenderer>`; `Session` implements the trait).
    ///
    /// # Errors
    /// [`RenderError::View`] (invalid `resources` front matter), [`RenderError::Nav`] (an
    /// alias that cannot be written), [`RenderError::Template`] (Tera load errors).
    pub fn new(
        project: Project,
        model: Arc<Model>,
        o: &RenderOptions,
    ) -> Result<Arc<Self>, RenderError> {
        let diagnostics = Arc::new(Diagnostics::new(model.config.ignore_logs.iter()));
        let store = Arc::new(ResourceStore::new(StoreConfig::from_config(
            &model.config,
            Some(Arc::clone(&project.vfs)),
            None,
        )));
        let (menus, menu_diagnostics) =
            neohugo_nav::build_menus(&NavSite::new(Arc::clone(&model)), &model.config);
        for d in menu_diagnostics {
            diagnostics.push(d);
        }
        let views = Arc::new(ViewCache::new(ViewInputs {
            model: Arc::clone(&model),
            store,
            menus: Arc::new(menus),
        })?);
        let pagination = Arc::new(PaginationRecorder::default());
        let aliases = model
            .config
            .sites
            .ids()
            .map(|l| neohugo_nav::page_aliases(views.nav(), &model.config, l))
            .collect::<Result<_, _>>()?;

        let mut selections = BTreeMap::new();
        for p in &model.pages {
            let path = lookup_path(p);
            for out in outputs(p) {
                match project.layouts.select(&layout_query(p, &path, out.format)) {
                    Some(s) => {
                        selections.insert((p.id, out.format), s);
                    }
                    None if is_standalone(p.kind) => {}
                    None => diagnostics.push(Diagnostic::warning(format!(
                        "no layout for {} page {} in format {}",
                        p.kind.as_str(),
                        p.key.to_path(),
                        model.config.output_formats.get(out.format).name
                    ))),
                }
            }
        }
        let sel: Selections = selections.values().cloned().collect();

        let pure = Arc::new(pure_env(&model, o, &diagnostics));
        let stubs = Stubs {
            views: Arc::clone(&views),
            pagination: Arc::clone(&pagination),
        };
        let templates = neohugo_layouts::load(Arc::clone(&project.layouts), &sel, &|t| {
            register_placeholders(t);
            register_pure(t, &pure);
            stubs.register(t);
        })?;
        let session = Arc::new(Self {
            hugo: tera::Value::from_serializable(&HugoView::new(&model.config)),
            model,
            project,
            templates: Arc::new(templates),
            views,
            pagination,
            selections,
            aliases,
            contents: OnceLock::new(),
            diagnostics,
        });
        Ok(session)
    }

    /// Phase C1: renders the content of every page (all hook variants; the skeleton has only
    /// `Html`). A second call does nothing.
    ///
    /// # Errors
    /// The first page whose content fails.
    pub fn render_content(&self) -> Result<(), RenderError> {
        if self.contents.get().is_some() {
            return Ok(());
        }
        let model = &self.model;
        let rendered: Vec<Option<Arc<RenderedContent>>> = model
            .pages
            .as_slice()
            .par_iter()
            .map(|p| {
                p.source
                    .as_ref()
                    .filter(|_| p.rendered() && p.role == PageRole::Standalone)
                    .map(|src| {
                        let src = content::PageSource {
                            body: src.body(),
                            file: Arc::from(src.file.abs.as_path()),
                            markup: p.meta.markup,
                            summary: p.meta.summary.as_deref(),
                        };
                        content::render_page(p.id, &src, &model.config.sites[p.lang]).map(Arc::new)
                    })
                    .transpose()
            })
            .collect::<Result<_, _>>()?;
        let mut all = BTreeMap::new();
        all.insert(HookVariant::Html, IdVec::from(rendered));
        let _ = self.contents.set(all);
        Ok(())
    }

    /// Phase D: freezes the Full view generations (after [`render_content`](Self::render_content)).
    ///
    /// # Errors
    /// [`RenderError::Phase`] when the content was not rendered.
    pub fn freeze_views(&self) -> Result<(), RenderError> {
        let contents = self
            .contents
            .get()
            .ok_or(RenderError::Phase("freeze_views before render_content"))?;
        self.views.freeze(contents);
        Ok(())
    }

    /// Phase E: renders one job. Pure: no I/O; the outputs go to the publisher.
    ///
    /// # Errors
    /// A template error ([`RenderError::Render`]) or an unknown page or format.
    pub fn render_job(&self, job: &Job) -> Result<Vec<Output>, RenderError> {
        if !self.views.is_frozen() {
            return Err(RenderError::Phase("render_job before freeze_views"));
        }
        let model = &self.model;
        match *job {
            Job::Page { page, format } | Job::Standalone { page, format } => {
                let p = &model.pages[page];
                let Some(out) = outputs(p).find(|o| o.format == format) else {
                    return Err(RenderError::NoOutput { page, format });
                };
                self.render_layout(job, page, format, None, out.paths.target.clone())
            }
            Job::Pager {
                page,
                format,
                number,
            } => {
                let (target, _) = page_target(model, page, format, Some(number))?;
                self.render_layout(job, page, format, Some(number), target.target)
            }
            Job::Alias(ref a) => {
                let to = self.permalink(a.to, a.format)?;
                self.render_alias(job, &a.from, &to, Some(a.to), a.format)
            }
            Job::PagerAlias { page, format } => {
                let (target, _) = page_target(model, page, format, Some(1))?;
                let to = self.permalink(page, format)?;
                self.render_alias(job, &target.target, &to, Some(page), format)
            }
            Job::LanguageRedirect => {
                let cfg = &model.config;
                let lang = LangIdx::from_index(0);
                let home = model.sites[lang].home;
                let Some(out) = outputs(&model.pages[home]).next() else {
                    return Ok(Vec::new());
                };
                let Some(links) = &out.links else {
                    return Ok(Vec::new());
                };
                let from =
                    OutputPath::new(&format!("/{}/index.html", cfg.sites[lang].language.key));
                let to = links.permalink.to_string();
                self.render_alias(job, &from, &to, None, out.format)
            }
        }
    }

    fn permalink(&self, page: PageId, format: FormatId) -> Result<String, RenderError> {
        outputs(&self.model.pages[page])
            .find(|o| o.format == format)
            .and_then(|o| o.links.as_ref())
            .map(|l| l.permalink.to_string())
            .ok_or(RenderError::NoOutput { page, format })
    }

    /// The order of a job.
    #[must_use]
    pub fn order(&self, job: &Job) -> JobOrder {
        let rank = |f: FormatId| u8::try_from(f.index()).unwrap_or(u8::MAX);
        let of = |page: PageId, format: FormatId, sub: u8| {
            JobOrder::new(self.model.pages[page].lang, rank(format), page.raw(), sub)
        };
        match *job {
            Job::Alias(ref a) => of(a.to, a.format, 0),
            Job::Page { page, format } => of(page, format, 1),
            Job::PagerAlias { page, format } => of(page, format, 2),
            Job::Pager { page, format, .. } => of(page, format, 3),
            Job::Standalone { page, format } => of(page, format, 4),
            Job::LanguageRedirect => JobOrder::new(LangIdx::from_index(0), 0, 0, 5),
        }
    }

    fn render_layout(
        &self,
        job: &Job,
        page: PageId,
        format: FormatId,
        pager: Option<u32>,
        target: OutputPath,
    ) -> Result<Vec<Output>, RenderError> {
        let Some(sel) = self.selections.get(&(page, format)) else {
            return Ok(Vec::new());
        };
        let cfg = &self.model.config;
        let p = &self.model.pages[page];
        let scope = RenderScope::layout(page, p.lang, format, pager);
        let generation = self.views.generation(Phase::Layout, scope.variant);
        let full = generation.page_value(page);
        let format_name = &cfg.output_formats.get(format).name;
        let output_format = full
            .as_map()
            .and_then(|m| m.get(&tera::value::Key::Str("output_formats")))
            .and_then(tera::Value::as_map)
            .and_then(|m| m.get(&tera::value::Key::String(format_name.clone().into())))
            .cloned()
            .unwrap_or_else(tera::Value::none);
        let mut ctx = tera::Context::new();
        ctx.insert_value("page", full);
        ctx.insert_value("site", generation.sites[p.lang].clone());
        ctx.insert_value("hugo", self.hugo.clone());
        ctx.insert("lang", &cfg.sites[p.lang].language.key);
        ctx.insert_value("output_format", output_format);
        if p.kind == PageKind::SitemapIndex {
            ctx.insert_value("sites", tera::Value::from(generation.sites.as_slice()));
        }
        ctx.insert_value(SCOPE_KEY, scope.to_value());
        let text = self
            .templates
            .tera()
            .render(sel.render_as.as_str(), &ctx)
            .map_err(|source| RenderError::Render {
                template: sel.render_as.to_string(),
                page: p.key.to_path(),
                source: Box::new(source),
            })?;
        Ok(vec![Output {
            path: target,
            text,
            format,
            lang: p.lang,
            is_html: cfg.output_formats.get(format).is_html,
            order: self.order(job),
        }])
    }

    /// The alias template (user, theme or embedded `alias.html`).
    fn alias_template(&self) -> Option<TemplateName> {
        self.project
            .layouts
            .templates()
            .filter(|t| t.role == TemplateRole::Standalone(StandaloneKind::Alias))
            .min_by(|a, b| a.origin.cmp(&b.origin))
            .map(|t| t.render_name().clone())
    }

    fn render_alias(
        &self,
        job: &Job,
        from: &OutputPath,
        to: &str,
        page: Option<PageId>,
        format: FormatId,
    ) -> Result<Vec<Output>, RenderError> {
        let Some(name) = self.alias_template() else {
            return Ok(Vec::new());
        };
        let lang = page.map_or(LangIdx::from_index(0), |p| self.model.pages[p].lang);
        let generation = self.views.generation(Phase::Layout, HookVariant::Html);
        let mut ctx = tera::Context::new();
        ctx.insert("permalink", to);
        ctx.insert_value(
            "page",
            page.map_or_else(tera::Value::none, |p| generation.links[p].clone()),
        );
        ctx.insert_value("site", generation.sites[lang].clone());
        ctx.insert_value("hugo", self.hugo.clone());
        let text = self
            .templates
            .tera()
            .render(name.as_str(), &ctx)
            .map_err(|source| RenderError::Render {
                template: name.to_string(),
                page: from.to_string(),
                source: Box::new(source),
            })?;
        Ok(vec![Output {
            path: from.clone(),
            text,
            format,
            lang,
            is_html: true,
            order: self.order(job),
        }])
    }

    /// **Skeleton (T38) job planning; T36 owns the real one.** Wave 1 of language `lang`:
    /// aliases, then every page × format, then the standalone pages (robots.txt and the
    /// sitemap index with the first language), in [`JobOrder`].
    #[must_use]
    pub fn wave1(&self, lang: LangIdx) -> Vec<Job> {
        let model = &self.model;
        let mut jobs: Vec<Job> = self.aliases[lang].iter().cloned().map(Job::Alias).collect();
        // robots.txt and the sitemap index are rendered once, with the first language.
        let root = |p: &Page| {
            matches!(p.kind, PageKind::RobotsTxt | PageKind::SitemapIndex) && p.lang.index() == 0
        };
        let mut standalone: Vec<PageId> = model
            .pages
            .iter()
            .filter(|p| p.lang == lang && p.standalone.is_some() && !root(p))
            .map(|p| p.id)
            .collect();
        if lang.index() == 0 {
            standalone.extend(model.pages.iter().filter(|p| root(p)).map(|p| p.id));
        }
        for p in &model.pages {
            if p.lang != lang || is_standalone(p.kind) {
                continue;
            }
            for o in outputs(p) {
                jobs.push(Job::Page {
                    page: p.id,
                    format: o.format,
                });
            }
        }
        for s in standalone {
            for o in outputs(&model.pages[s]) {
                jobs.push(Job::Standalone {
                    page: s,
                    format: o.format,
                });
            }
        }
        jobs.sort_by_key(|j| self.order(j));
        jobs
    }

    /// **Skeleton (T38).** Wave 2: from the recorded paginations, the `page/1/` aliases (HTML
    /// formats, unless `pagination.disableAliases`) and pagers 2..N; then the language
    /// redirect.
    #[must_use]
    pub fn wave2(&self) -> Vec<Job> {
        let cfg = &self.model.config;
        let mut jobs = Vec::new();
        for ((page, format), rec) in self.pagination.recorded() {
            let site = &cfg.sites[self.model.pages[page].lang];
            if cfg.output_formats.get(format).is_html && !site.pagination.disable_aliases {
                jobs.push(Job::PagerAlias { page, format });
            }
            for number in 2..=rec.total_pages() {
                jobs.push(Job::Pager {
                    page,
                    format,
                    number,
                });
            }
        }
        jobs.sort_by_key(|j| self.order(j));
        if cfg.sites.len() > 1 && cfg.default_site().language.url_prefix.is_empty() {
            jobs.push(Job::LanguageRedirect);
        }
        jobs
    }

    /// The model the session renders.
    #[must_use]
    pub fn model(&self) -> &Arc<Model> {
        &self.model
    }

    /// The frozen views.
    #[must_use]
    pub fn views(&self) -> &Arc<ViewCache> {
        &self.views
    }

    /// Warnings and errors of the render.
    #[must_use]
    pub fn diagnostics(&self) -> &Arc<Diagnostics> {
        &self.diagnostics
    }
}

impl ContentRenderer for Session {
    fn content(
        &self,
        p: PageId,
        v: HookVariant,
        _: &RenderScope,
    ) -> Result<Arc<RenderedContent>, ContentError> {
        let all = self
            .contents
            .get()
            .ok_or_else(|| ContentError::Render("content not rendered yet".to_owned()))?;
        all.get(&v)
            .or_else(|| all.get(&HookVariant::Html))
            .and_then(|c| c.get(p))
            .and_then(Option::clone)
            .ok_or(ContentError::NoContent(p))
    }

    fn fragments(&self, p: PageId, s: &RenderScope) -> Result<Arc<Fragments>, ContentError> {
        self.content(p, HookVariant::Html, s)
            .map(|c| Arc::clone(&c.fragments))
    }

    fn render_shortcodes(
        &self,
        _: PageId,
        _: &RenderScope,
    ) -> Result<Arc<ExpandedSource>, ContentError> {
        Err(ContentError::Render(
            "render_shortcodes is not in the walking skeleton".to_owned(),
        ))
    }

    fn render_markdown(
        &self,
        md: &str,
        _: RenderStringOptions,
        s: &RenderScope,
    ) -> Result<String, ContentError> {
        let site = &self.model.config.sites[s.lang];
        let file: Arc<std::path::Path> = Arc::from(std::path::Path::new(""));
        content::markdown(md, s.page, &file, &content::markdown_options(site))
            .map(|r| r.html)
            .map_err(|e| ContentError::Render(e.to_string()))
    }

    fn render_template(
        &self,
        t: &TemplateName,
        mut ctx: tera::Context,
        s: &RenderScope,
    ) -> Result<String, ContentError> {
        let child = s.child();
        if child.too_deep() {
            return Err(ContentError::TooDeep {
                limit: neohugo_view::MAX_DEPTH,
            });
        }
        ctx.insert_value(SCOPE_KEY, child.to_value());
        self.templates
            .tera()
            .render(t.as_str(), &ctx)
            .map_err(|e| ContentError::Render(e.to_string()))
    }
}
