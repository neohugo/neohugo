//! Port of `hugolib/site.go`.
//!
//! Owner: Wave B task T23 (hugolib-site).

//! Go `hugolib/site.go`: `Site` (one per language), its lazy inits (taxonomies, menus, prevNext),
//! `renderAndWritePage` (execute template -> publisher Descriptor: RSS always absURL, HTML absURL
//! when canonify; empty output writes NO file), GetPage, refLinker, language prefixes,
//! `SitemapAbsURL`, and the template-visible site methods.
//!
//! Parts of site.go live elsewhere (HUGO_LAYER.md §12, WAVE_B_PLAN.json):
//! * `NewHugoSites`/`newHugoSites`/`newSiteRefLinker` (construction) -> hugo_sites.rs (T20);
//! * `initRenderFormats`, `shouldBuild` (assembly) -> build_assemble.rs (T21);
//! * `hookRendererTemplate` -> page__per_output.rs (T22);
//! * `(s *Site) render` -> site_render.rs (T24).
//!
//! The Go `*Site` methods are functions over `(h, site_idx)` here (the site's handle is the
//! frozen `HugoSites` plus the index); `tplapi::site_methods` dispatches template calls to them.
//! The lazy site inits (`s.init.*` in Go) are `OnceLock`s: `Site.taxonomies`, `Site.menus` and
//! the per-page positions (page__position.rs).

use std::collections::{BTreeMap, HashMap};
use std::sync::atomic::Ordering;
use std::sync::{Arc, OnceLock};

use go_value::{Map, MapType, SliceType, Time, Value};
use nh_allconfig::allconfig::Config;
use nh_common::Result;
use nh_common::herrors::Error;
use nh_common::maps::scratch::Scratch;
use nh_common::text::Position;
use nh_deps::deps::Deps;
use nh_langs::language::Language;
use nh_media::output::output_format::Formats;
use nh_page::navigation::menu::{Menu, MenuEntry, Menus, PageMenus};
use nh_page::page::{PageRef, Pages};
use nh_page::pagemeta::page_frontmatter::FrontMatterHandler;
use nh_page::pages_related::RelatedDocsHandler;
use nh_page::taxonomy::TaxonomyList;
use nh_publisher::publisher::{DestinationPublisher, Publisher};
use nh_tpl::template::TplContext;
use nh_tplimpl::templatestore::{TemplInfo, TemplateStore};

use crate::content_map_page::{PageMap, PageMapQueryPagesInSection, page_predicates as pp};
use crate::hugo_sites::HugoSites;
use crate::page::{PageHandle, PageId, PageWrapper, handle_of};
use crate::page__content::HostState;
use crate::template_exec::{ExecCall, ExecKind};

/// Go: `hugolib.Site`.
pub struct Site {
    /// Index in `HugoSites.sites` (Go `languagei`).
    pub idx: usize,
    pub conf: Arc<Config>,
    pub language: Arc<Language>,
    pub deps: Arc<Deps>,
    pub page_map: crate::content_map_page::PageMap,
    pub store: Arc<Scratch>,
    /// Go `taxonomies` (lazy: `CreateSiteTaxonomies` on first use).
    pub taxonomies: OnceLock<TaxonomyList>,
    pub menus: OnceLock<Menus>,
    pub home: Option<PageId>,
    pub lastmod: Time,
    pub related_docs_handler: Arc<RelatedDocsHandler>,
    /// One publisher per site, each with its own HTML elements collector.
    pub publisher: Arc<DestinationPublisher>,
    pub frontmatter_handler: FrontMatterHandler,
    /// Sorted render formats of this site (Formats.Less).
    pub render_formats: Formats,
    /// This site's template store view (own func map).
    pub template_store: OnceLock<TemplateStore>,
}

impl Site {
    /// Go: `renderAndWritePage(statCounter, name, targetPath, p, d, templ)` — executes the
    /// template with the page in the context, then publishes the result (nothing when empty):
    /// RSS always gets absolute URLs, HTML when `relativeURLs` or `canonifyURLs` is set; the
    /// home page's HTML gets `AddHugoGeneratorTag = disableHugoGeneratorInject` (inverted, so no
    /// tag in a default build).
    // Go: hugolib/site.go:renderAndWritePage
    pub fn render_and_write_page(
        h: &Arc<HugoSites>,
        site_idx: usize,
        target_path: &str,
        p: PageId,
        data: &Value,
        templ: &Arc<nh_tplimpl::templatestore::TemplInfo>,
    ) -> Result<()> {
        Self::render_and_write_page_with(
            h,
            site_idx,
            None,
            target_path,
            p,
            data,
            templ,
            ExecKind::Page,
        )
    }

    /// [`Site::render_and_write_page`] with Go's `statCounter` and the exec kind (the pager
    /// number for `renderPaginator`, T24) of the `template_exec` call.
    // Go: hugolib/site.go:renderAndWritePage
    pub fn render_and_write_page_with(
        h: &Arc<HugoSites>,
        site_idx: usize,
        stat_counter: Option<&std::sync::atomic::AtomicU64>,
        target_path: &str,
        p: PageId,
        data: &Value,
        templ: &Arc<nh_tplimpl::templatestore::TemplInfo>,
        kind: ExecKind,
    ) -> Result<()> {
        // Go `s.h.pageRenderCounter.Add(1)` feeds the build counters log only.
        let s = &h.sites[site_idx];
        let ps = h.page(p);
        let po = ps.current_output();
        let of = po.f.clone();
        po.incr_render_state();

        let page_value = handle_of(h, p).page_ref().to_value();
        let ctx = TplContext {
            page: Some(page_value),
            ..Default::default()
        };

        let mut render_buffer: Vec<u8> = Vec::new();
        render_for_template(
            h,
            site_idx,
            &ctx,
            ps.meta.kind(),
            &of.name,
            data,
            &mut render_buffer,
            Some(templ),
            ExecCall {
                page: Some(p),
                output_format: of.name.clone(),
                kind,
                ordinal: 0,
            },
        )?;

        if render_buffer.is_empty() {
            return Ok(());
        }

        let is_html = of.is_html;
        let is_rss = of.name == "rss";

        let mut abs_url_path = String::new();
        let mut add_hugo_generator_tag = false;
        if is_rss {
            // Always canonify URLs in RSS
            abs_url_path = s.abs_url_path(target_path);
        } else if is_html {
            if s.conf.root.relative_urls || s.conf.root.canonify_urls {
                abs_url_path = s.abs_url_path(target_path);
            }

            // Live reload is never injected in a build (server mode is not ported).

            // For performance reasons we only inject the Hugo generator tag on the home page.
            if ps.meta.is_home() {
                add_hugo_generator_tag = s.conf.root.disable_hugo_generator_inject;
            }
        }

        let pd = nh_publisher::publisher::Descriptor {
            src: &render_buffer,
            output_format: ps.output_format().clone(),
            target_path: target_path.to_string(),
            stat_counter,
            live_reload_base_url: None,
            add_hugo_generator_tag,
            abs_url_path,
            minify: false,
        };

        s.publisher.publish(pd)
    }

    /// Go: `absURLPath(targetPath)` — baseURL with trailing `/` (or dotted relative path with relativeURLs).
    // Go: hugolib/site.go:absURLPath
    pub fn abs_url_path(&self, target_path: &str) -> String {
        if self.conf.root.relative_urls {
            nh_helpers::path::get_dotted_relative_path(target_path)
        } else {
            let mut url = self.deps.path_spec().cfg.base_url().string().to_string();
            if !url.ends_with('/') {
                url.push('/');
            }
            url
        }
    }

    /// Go: `getLanguageTargetPathLang` / `getLanguagePermalinkLang` (sitemaps: always in subdir).
    // Go: hugolib/site.go:getLanguageTargetPathLang
    pub fn get_language_target_path_lang(&self, always_in_sub_dir: bool) -> String {
        if self.deps.conf.is_multihost() {
            return self.language.lang.clone();
        }

        self.get_language_permalink_lang(always_in_sub_dir)
    }

    /// Go: `getLanguagePermalinkLang(alwaysInSubDir)` — any language code to prefix the relative
    /// permalink with. (Go reads `s.h.Conf`, the first site's config; the multihost and
    /// multilingual flags are the same in every language's config.)
    // Go: hugolib/site.go:getLanguagePermalinkLang
    pub fn get_language_permalink_lang(&self, always_in_sub_dir: bool) -> String {
        if self.deps.conf.is_multihost() {
            return String::new();
        }

        if self.deps.conf.is_multilingual() && always_in_sub_dir {
            return self.language.lang.clone();
        }

        self.deps.path_spec().paths.get_language_prefix()
    }

    /// Go: `SitemapAbsURL()` (sitemapindex).
    // Go: hugolib/site.go:SitemapAbsURL
    pub fn sitemap_abs_url(&self) -> String {
        let mut base = String::new();
        let num_languages = self
            .conf
            .languages
            .0
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .len();
        if num_languages > 1 || self.deps.conf.default_content_language_in_subdir() {
            base = self.language.lang.clone();
        }
        let mut p = self.deps.path_spec().abs_url(&base, false);
        if !p.ends_with('/') {
            p.push('/');
        }
        p.push_str(&self.conf.sitemap.filename);
        p
    }

    /// Go: `createNodeMenuEntryURL(in)` — site-relative menu URLs made to match the nodes.
    // Go: hugolib/site.go:createNodeMenuEntryURL
    pub fn create_node_menu_entry_url(&self, in_: &str) -> String {
        if !in_.starts_with('/') {
            return in_.to_string();
        }
        // make it match the nodes
        let mut menu_entry_url = self.deps.path_spec().urlize(in_);
        if !self.conf.root.canonify_urls {
            menu_entry_url = nh_common::paths::url::add_context_root(
                self.deps.path_spec().cfg.base_url().string(),
                &menu_entry_url,
            );
        }
        menu_entry_url
    }

    /// Go: `Title()`.
    // Go: hugolib/site.go:Title
    pub fn title(&self) -> String {
        self.conf.root.title.clone()
    }

    /// Go: `Copyright()`.
    // Go: hugolib/site.go:Copyright
    pub fn copyright(&self) -> String {
        self.conf.root.copyright.clone()
    }

    /// Go: `Config()` — `page.SiteConfig{Privacy, Services}`.
    // Go: hugolib/site.go:Config
    pub fn config(&self) -> nh_page::site::SiteConfig {
        nh_page::site::SiteConfig {
            privacy: self.conf.privacy.clone(),
            services: self.conf.services.clone(),
        }
    }

    /// Go: `LanguageCode()` — the language's `LanguageCode()` (not the top-level config key).
    // Go: hugolib/site.go:LanguageCode
    pub fn language_code(&self) -> String {
        self.language.language_code().to_string()
    }

    /// Go: `BaseURL()` — `conf.C.BaseURL.WithPath`.
    // Go: hugolib/site.go:BaseURL
    pub fn base_url(&self) -> String {
        self.conf.compiled().base_url.with_path.clone()
    }

    /// Go: `ServerPort()`.
    // Go: hugolib/site.go:ServerPort
    pub fn server_port(&self) -> i64 {
        self.conf.compiled().base_url.port()
    }

    /// Go: `Lastmod()`.
    // Go: hugolib/site.go:Lastmod
    pub fn lastmod(&self) -> Time {
        self.lastmod.clone()
    }

    /// Go: `LastChange()` (deprecated alias of `Lastmod`).
    // Go: hugolib/site.go:LastChange
    pub fn last_change(&self) -> Time {
        nh_config::neohugo::neohugo::deprecate(
            ".Site.LastChange",
            "Use .Site.Lastmod instead.",
            "v0.123.0",
        );
        self.lastmod.clone()
    }

    /// Go: `Params()` — the SAME `maps.Params` every time (the `mainsections` special case of
    /// the template exec helper compares pointers).
    // Go: hugolib/site.go:Params
    pub fn params(&self) -> Arc<Map> {
        self.conf.params.clone()
    }

    /// Go: `Author()` (deprecated; logged when the config has one).
    // Go: hugolib/site.go:Author
    pub fn author(&self) -> Option<Map> {
        if self.conf.author.as_ref().is_some_and(|m| !m.is_empty()) {
            nh_config::neohugo::neohugo::deprecate(
                ".Site.Author",
                "Implement taxonomy 'author' or use .Site.Params.Author instead.",
                "v0.124.0",
            );
        }
        self.conf.author.clone()
    }

    /// Go: `Authors()` (deprecated): an empty `page.AuthorList`.
    // Go: hugolib/site.go:Authors
    pub fn authors(&self) -> Value {
        nh_config::neohugo::neohugo::deprecate(
            ".Site.Authors",
            "Implement taxonomy 'authors' or use .Site.Params.Author instead.",
            "v0.124.0",
        );
        Value::map(Map::new(MapType::Named(Arc::from("page.AuthorList"))))
    }

    /// Go: `Social()` (deprecated).
    // Go: hugolib/site.go:Social
    pub fn social(&self) -> Option<Map> {
        nh_config::neohugo::neohugo::deprecate(
            ".Site.Social",
            "Implement taxonomy 'social' or use .Site.Params.Social instead.",
            "v0.124.0",
        );
        self.conf.social.clone()
    }

    /// Go: `Param(key)` — `resource.Param(s, nil, key)`.
    // Go: hugolib/site.go:Param
    pub fn param(&self, key: &Value) -> Result<Value> {
        nh_resource::params::param(&self.params(), None, key)
    }

    /// Go: `BuildDrafts()`.
    // Go: hugolib/site.go:BuildDrafts
    pub fn build_drafts(&self) -> bool {
        self.conf.root.build_drafts
    }

    /// Go: `LanguagePrefix()` — `"/" + GetLanguagePrefix()` (empty when there is none).
    // Go: hugolib/site.go:LanguagePrefix
    pub fn language_prefix(&self) -> String {
        let prefix = self.deps.path_spec().paths.get_language_prefix();
        if prefix.is_empty() {
            return String::new();
        }
        format!("/{prefix}")
    }

    /// Go: `Store()`.
    // Go: hugolib/site.go:Store
    pub fn store(&self) -> Arc<Scratch> {
        self.store.clone()
    }

    /// Go: `CheckReady()` — the port only hands out site handles after freezing, when every
    /// site is ready.
    // Go: hugolib/site.go:CheckReady
    pub fn check_ready(&self) {}

    /// Go: `MainSections()` (`None` is Go's nil `[]string`).
    // Go: hugolib/site.go:MainSections
    pub fn main_sections(&self) -> Option<Vec<String>> {
        self.check_ready();
        self.conf.compiled().main_sections()
    }

    /// Go: `GetInternalRelatedDocsHandler()`.
    // Go: hugolib/site.go:GetInternalRelatedDocsHandler
    pub fn get_internal_related_docs_handler(&self) -> Arc<RelatedDocsHandler> {
        self.related_docs_handler.clone()
    }

    /// Go: `Language()`.
    // Go: hugolib/site.go:Language
    pub fn language(&self) -> Arc<Language> {
        self.language.clone()
    }

    /// Go: `watching()` — never in a one-shot build.
    // Go: hugolib/site.go:watching
    pub fn watching(&self) -> bool {
        self.conf.internal.watch
    }

    /// Go: `RegisterMediaTypes()` (a no-op in Go too: it only ran for the removed mime
    /// registration).
    // Go: hugolib/site.go:RegisterMediaTypes
    pub fn register_media_types(&self) {}

    /// Go: `errorCollator(results, errs)` — collects the errors of a render and returns the one
    /// `pickOneAndLogTheRest` picks.
    // Go: hugolib/site.go:errorCollator
    pub fn error_collator(h: &HugoSites, results: Vec<Error>) -> Option<Error> {
        h.pick_one_and_log_the_rest(results)
    }
}

/// Go: `renderForTemplate(ctx, name, outputFormat, d, w, templ)` — executes `templ` (through
/// `template_exec`) and wraps an error as `render of "<page>" failed: ...`. A nil template
/// logs the missing layout in Go (`logMissingLayout`, site_render.go: T24) and renders nothing.
// Go: hugolib/site.go:renderForTemplate
#[allow(clippy::too_many_arguments)]
pub fn render_for_template(
    h: &Arc<HugoSites>,
    site_idx: usize,
    ctx: &TplContext,
    name: &str,
    _output_format: &str,
    d: &Value,
    w: &mut Vec<u8>,
    templ: Option<&Arc<TemplInfo>>,
    call: ExecCall,
) -> Result<()> {
    let Some(templ) = templ else {
        // Go: `s.logMissingLayout(name, "", "", outputFormat)` (T24's site_render.rs).
        return Ok(());
    };

    // Keep the hugolib context keys of the caller (T22's HostState).
    let ctx = HostState::of(ctx).set_on(ctx);

    if let Err(err) = crate::template_exec::execute(h, site_idx, &ctx, templ, w, d, &call) {
        let mut filename = name.to_string();
        if let Some(p) = nh_page::page::page_from_value(d)
            && let Some(ph) = p.0.as_any().downcast_ref::<PageHandle>()
            && ph.wrapper == PageWrapper::None
        {
            filename = ph.state().string();
        }
        return Err(err.wrap(format!("render of {} failed", go_strconv::quote(filename))));
    }
    Ok(())
}

/// Go: `(s *Site) Pages()` — all pages of the site's language.
// Go: hugolib/site.go:Pages
pub fn site_pages(h: &Arc<HugoSites>, site_idx: usize) -> Pages {
    PageMap::get_pages_in_section(
        h,
        site_idx,
        &PageMapQueryPagesInSection {
            path: String::new(),
            key_part: "global".into(),
            include: Some(pp::should_list_global()),
            recursive: true,
            include_self: true,
        },
    )
}

/// Go: `(s *Site) RegularPages()` — all regular pages of the site's language.
// Go: hugolib/site.go:RegularPages
pub fn site_regular_pages(h: &Arc<HugoSites>, site_idx: usize) -> Pages {
    PageMap::get_pages_in_section(
        h,
        site_idx,
        &PageMapQueryPagesInSection {
            path: String::new(),
            key_part: "global".into(),
            include: Some(pp::and(pp::should_list_global(), vec![pp::kind_page()])),
            recursive: true,
            include_self: false,
        },
    )
}

/// Go: `(s *Site) Taxonomies()` — created once (`CreateSiteTaxonomies`, T21).
// Go: hugolib/site.go:Taxonomies
pub fn site_taxonomies(h: &Arc<HugoSites>, site_idx: usize) -> TaxonomyList {
    crate::content_map_page::site_taxonomies(h, site_idx)
}

/// Go: `(s *Site) Menus()` — assembled once (`s.init.menus`). Go ignores the init's error.
// Go: hugolib/site.go:Menus
pub fn site_menus(h: &Arc<HugoSites>, site_idx: usize) -> Menus {
    h.sites[site_idx]
        .menus
        .get_or_init(|| match assemble_menus(h, site_idx) {
            Ok(m) => m,
            Err(_) => Arc::new(BTreeMap::new()),
        })
        .clone()
}

/// Go: `(s *Site) GetPage(ref...)` — a miss is `page.NilPage` (with the lookup's error, if
/// any).
// Go: hugolib/site.go:GetPage
pub fn site_get_page(
    h: &Arc<HugoSites>,
    site_idx: usize,
    refs: &[String],
) -> (PageRef, Option<Error>) {
    let (p, err) =
        match crate::pagecollections::new_page_finder(h, site_idx).get_page_for_refs(refs) {
            Ok(p) => (p, None),
            Err(e) => (None, Some(e)),
        };

    // The nil struct has meaning in some situations, mostly to avoid breaking
    // existing sites doing $nilpage.IsDescendant($p), which will always return
    // false.
    let p = p.unwrap_or_else(|| PageRef(nh_page::page_nop::nil_page()));
    (p, err)
}

/// Go: `(s *Site) Sites()` — every site as a `page.Site` (`WrapSite`).
// Go: hugolib/site.go:Sites
pub fn site_sites(h: &Arc<HugoSites>) -> Vec<nh_page::site::SiteRef> {
    (0..h.sites.len())
        .map(|idx| SiteHandle { h: h.clone(), idx }.site_ref())
        .collect()
}

/// `page.Sites` as a template value.
pub fn sites_to_value(sites: &[nh_page::site::SiteRef]) -> Value {
    Value::list(
        SliceType::Named(Arc::from("page.Sites")),
        sites.iter().map(|s| s.to_value()).collect(),
    )
}

/// The `[]*hugolib.Site` template value (the data of the embedded `sitemapindex.xml`:
/// `range .Sites` reaches `.SitemapAbsURL` and `.Lastmod`).
pub fn hugolib_sites_value(h: &Arc<HugoSites>) -> Value {
    Value::list(
        SliceType::Named(Arc::from("[]*hugolib.Site")),
        (0..h.sites.len())
            .map(|idx| Value::object(HugolibSiteObject(SiteHandle { h: h.clone(), idx })))
            .collect(),
    )
}

/// Go: `(h *HugoSites) isMultilingual()` (`.Site.IsMultiLingual`, deprecated).
// Go: hugolib/site.go:IsMultiLingual
pub fn is_multi_lingual(h: &HugoSites) -> bool {
    nh_config::neohugo::neohugo::deprecate(
        ".Site.IsMultiLingual",
        "Use neohugo.IsMultilingual instead.",
        "v0.124.0",
    );
    h.sites.len() > 1
}

/// Go: `taxonomiesConfig.Values()` — the views sorted by plural (`sort.Slice`) and indexed by
/// tree key. The capture builds them (T20's `content_map::taxonomies_config_values`).
// Go: hugolib/site.go:Values
pub fn taxonomies_config_values(
    t: &BTreeMap<String, String>,
) -> crate::content_map::TaxonomiesConfigValues {
    crate::content_map::taxonomies_config_values(t)
}

/// The site's `prevNext` init (Go `s.init.prevNext`): the positions of every regular page in
/// the site's default order (`page.nextPrevSortOrder = "asc"` reverses it). Positions are only
/// set once (a page's position cell is an `OnceLock`).
// Go: hugolib/site.go:prepareInits
pub fn init_prev_next(h: &Arc<HugoSites>, site_idx: usize) {
    let s = &h.sites[site_idx];
    let mut regular_pages = site_regular_pages(h, site_idx);
    if s.conf.page.next_prev_sort_order == "asc" {
        regular_pages = nh_page::pages_sort::reverse(&regular_pages);
    }
    set_next_prev(&regular_pages, false);
}

/// The site's `prevNextInSection` init: positions within each section's (and the home page's)
/// regular pages.
// Go: hugolib/site.go:prepareInits
pub fn init_prev_next_in_section(h: &Arc<HugoSites>, site_idx: usize) {
    let s = &h.sites[site_idx];
    let sections = PageMap::get_pages_in_section(
        h,
        site_idx,
        &PageMapQueryPagesInSection {
            path: String::new(),
            key_part: "sectionorhome".into(),
            include: Some(pp::or(pp::kind_section(), vec![pp::kind_home()])),
            include_self: true,
            recursive: true,
        },
    );

    for section in &sections {
        let Some(sh) = section.0.as_any().downcast_ref::<PageHandle>() else {
            continue;
        };
        let mut ps = crate::page::regular_pages(sh);
        if s.conf.page.next_prev_in_section_sort_order == "asc" {
            ps = nh_page::pages_sort::reverse(&ps);
        }
        set_next_prev(&ps, true);
    }
}

/// Go's `setNextPrev(pas)` closure (and the `prevNext` loop): `next` is the previous element.
fn set_next_prev(pas: &Pages, in_section: bool) {
    for (i, p) in pas.iter().enumerate() {
        let Some(h) = p.0.as_any().downcast_ref::<PageHandle>() else {
            continue;
        };
        let ps = h.state();
        if !ps.meta.is_page() {
            // Go: `np.getNextPrev()` is nil for pages that are not regular pages.
            continue;
        }
        let next = if i > 0 {
            Some(pas[i - 1].clone())
        } else {
            None
        };
        let prev = if i + 1 < pas.len() {
            Some(pas[i + 1].clone())
        } else {
            None
        };
        let cell = if in_section {
            &ps.common.next_prev_in_section
        } else {
            &ps.common.next_prev
        };
        let _ = cell.set((next, prev));
    }
}

/// Go: `(s *Site) assembleMenus()` — the site's menus from the config (`pageRef` resolved,
/// URLs made to match the nodes), the section pages menu (`sectionPagesMenu`) and the pages'
/// own entries (front matter), with children placed below their parents (a parent that does
/// not exist is created without a URL).
///
/// Go builds the tree by mutating shared `*MenuEntry` values and iterates Go maps (`flat`,
/// `children`); the port works on an arena of entries (the page entries by identity, so an
/// entry listed in two menus stays one entry), iterates in insertion order, and freezes the
/// arena into `Arc` trees at the end. Only full sort ties (weight, name, identifier) could
/// order differently (Go: random).
// Go: hugolib/site.go:assembleMenus
pub fn assemble_menus(h: &Arc<HugoSites>, site_idx: usize) -> Result<Menus> {
    let s = &h.sites[site_idx];
    let mut arena: Vec<MenuEntry> = Vec::new();
    let mut children_of: Vec<Vec<usize>> = Vec::new();
    // Page entries by identity (their Arc pointer).
    let mut page_entry_idx: HashMap<usize, usize> = HashMap::new();
    // Go `flat` (map[twoD]*MenuEntry), in insertion order.
    let mut flat_keys: Vec<(String, String)> = Vec::new();
    let mut flat: HashMap<(String, String), usize> = HashMap::new();
    let mut flat_insert = |flat_keys: &mut Vec<(String, String)>,
                           flat: &mut HashMap<(String, String), usize>,
                           key: (String, String),
                           idx: usize| {
        if flat.insert(key.clone(), idx).is_none() {
            flat_keys.push(key);
        }
    };
    let mut push =
        |arena: &mut Vec<MenuEntry>, children_of: &mut Vec<Vec<usize>>, me: MenuEntry| {
            arena.push(me);
            children_of.push(Vec::new());
            arena.len() - 1
        };

    // add menu entries from config to flat hash
    if let Some(ns) = &s.conf.menus {
        for (name, menu) in &ns.config {
            for me in menu {
                let mut e = (**me).clone();
                if e.page_ref().is_none() && !e.config.page_ref.is_empty() {
                    // Try to resolve the page.
                    e.page = crate::pagecollections::new_page_finder(h, site_idx)
                        .get_page(None, &e.config.page_ref)
                        .ok()
                        .flatten()
                        .map(|p| p.to_value());
                }

                // If page is still nill, we must make sure that we have a URL that considers baseURL etc.
                match e.page_ref() {
                    None => {
                        e.configured_url = s.create_node_menu_entry_url(&e.config.url);
                    }
                    Some(p) => nh_page::navigation::menu::set_page_values(&mut e, &p),
                }

                let key = (name.clone(), e.key_name());
                let idx = push(&mut arena, &mut children_of, e);
                flat_insert(&mut flat_keys, &mut flat, key, idx);
            }
        }
    }

    let section_pages_menu = s.conf.root.section_pages_menu.clone();

    if !section_pages_menu.is_empty() {
        let should_list_global = pp::should_list_global();
        PageMap::for_each_page(h, site_idx, Some(&should_list_global), &mut |p| {
            if p.meta.kind() != nh_common::kinds::KIND_SECTION
                || !p.meta.should_be_checked_for_menu_definitions()
            {
                return Ok(false);
            }

            // The section pages menus are attached to the top level section.
            let mut id = p.meta.section().to_string();
            if id.is_empty() {
                id = "/".to_string();
            }

            if flat.contains_key(&(section_pages_menu.clone(), id.clone())) {
                return Ok(false);
            }
            let pr = handle_of(h, p.id).page_ref();
            let mut me = MenuEntry::default();
            me.config.identifier = id;
            me.config.name = p.meta.link_title().to_string();
            me.config.weight = p.meta.weight();
            me.page = Some(pr.to_value());

            nh_page::navigation::menu::set_page_values(&mut me, &pr);
            let key = (section_pages_menu.clone(), me.key_name());
            let idx = push(&mut arena, &mut children_of, me);
            flat_insert(&mut flat_keys, &mut flat, key, idx);
            Ok(false)
        })?;
    }

    // Add menu entries provided by pages
    let mut visited: Vec<PageId> = Vec::new();
    let should_list_global = pp::should_list_global();
    PageMap::for_each_page(h, site_idx, Some(&should_list_global), &mut |p| {
        let handle = handle_of(h, p.id);
        visited.push(p.id);
        if let Some(pm) = crate::page__menus::init(&handle) {
            for (name, me) in pm {
                let key = (name.clone(), me.key_name());
                if flat.contains_key(&key) {
                    let err = p.wrap_error(Error::new(format!(
                        "duplicate menu entry with identifier {} in menu {}",
                        go_strconv::quote(me.key_name()),
                        go_strconv::quote(name)
                    )));
                    s.deps.log.warnf(err.to_string());
                    continue;
                }
                let ptr = Arc::as_ptr(me) as usize;
                let idx = match page_entry_idx.get(&ptr) {
                    Some(i) => *i,
                    None => {
                        let i = push(&mut arena, &mut children_of, (**me).clone());
                        page_entry_idx.insert(ptr, i);
                        i
                    }
                };
                flat_insert(&mut flat_keys, &mut flat, key, idx);
            }
        }
        Ok(false)
    })?;

    let sort_menu = |arena: &Vec<MenuEntry>, m: &mut Vec<usize>| {
        go_sort::stable_by(m, |a, b| {
            nh_page::navigation::menu::default_menu_entry_sort(&arena[*a], &arena[*b])
        });
    };

    // Create Children Menus First
    let mut children_keys: Vec<(String, String)> = Vec::new();
    let mut children: HashMap<(String, String), Vec<usize>> = HashMap::new();
    for key in &flat_keys {
        let e = flat[key];
        if !arena[e].config.parent.is_empty() {
            let ck = (arena[e].menu.clone(), arena[e].config.parent.clone());
            let m = children.entry(ck.clone()).or_insert_with(|| {
                children_keys.push(ck.clone());
                Vec::new()
            });
            // Go: `Menu.Add` appends and sorts.
            m.push(e);
            sort_menu(&arena, m);
        }
    }

    // Placing Children in Parents (in flat)
    for ck in &children_keys {
        let childmenu = children[ck].clone();
        if !flat.contains_key(ck) {
            // if parent does not exist, create one without a URL
            let mut me = MenuEntry::default();
            me.config.name = ck.1.clone();
            let idx = push(&mut arena, &mut children_of, me);
            flat_insert(&mut flat_keys, &mut flat, ck.clone(), idx);
        }
        children_of[flat[ck]] = childmenu;
    }

    // Assembling Top Level of Tree
    let mut menus_idx: BTreeMap<String, Vec<usize>> = BTreeMap::new();
    for key in &flat_keys {
        let e = flat[key];
        if arena[e].config.parent.is_empty() {
            let m = menus_idx.entry(key.0.clone()).or_default();
            m.push(e);
            sort_menu(&arena, m);
        }
    }

    // Freeze the arena (children first).
    let mut frozen: Vec<Option<Arc<MenuEntry>>> = vec![None; arena.len()];
    let mut visiting: Vec<bool> = vec![false; arena.len()];
    fn freeze(
        i: usize,
        arena: &[MenuEntry],
        children_of: &[Vec<usize>],
        frozen: &mut Vec<Option<Arc<MenuEntry>>>,
        visiting: &mut Vec<bool>,
    ) -> Arc<MenuEntry> {
        if let Some(f) = &frozen[i] {
            return f.clone();
        }
        visiting[i] = true;
        let mut e = arena[i].clone();
        let mut children: Menu = Vec::new();
        for &c in &children_of[i] {
            if visiting[c] {
                // A cycle (an entry below itself): Go builds a cyclic structure; the port drops
                // the back edge.
                continue;
            }
            children.push(freeze(c, arena, children_of, frozen, visiting));
        }
        if !children_of[i].is_empty() {
            e.children = children;
        }
        visiting[i] = false;
        let a = Arc::new(e);
        frozen[i] = Some(a.clone());
        a
    }

    let mut menus: BTreeMap<String, Menu> = BTreeMap::new();
    for (name, m) in &menus_idx {
        let menu: Menu = m
            .iter()
            .map(|&i| freeze(i, &arena, &children_of, &mut frozen, &mut visiting))
            .collect();
        menus.insert(name.clone(), menu);
    }

    // The pages' own entries, as the assembly left them.
    for id in visited {
        let handle = handle_of(h, id);
        let Some(pm) = crate::page__menus::init(&handle) else {
            continue;
        };
        let mut assembled: PageMenus = PageMenus::new();
        for (name, me) in pm {
            let ptr = Arc::as_ptr(me) as usize;
            let e = match page_entry_idx.get(&ptr) {
                Some(&i) => freeze(i, &arena, &children_of, &mut frozen, &mut visiting),
                None => me.clone(),
            };
            assembled.insert(name.clone(), e);
        }
        let _ = h.page(id).common.page_menus_assembled.set(assembled);
    }

    Ok(Arc::new(menus))
}

/// Go: `siteRefLinker.logNotFound(ref, what, p, position)`.
// Go: hugolib/site.go:logNotFound
pub fn log_not_found(
    h: &HugoSites,
    site_idx: usize,
    r: &str,
    what: &str,
    p: Option<&PageRef>,
    position: &Position,
) {
    let s = &h.sites[site_idx];
    let linker = crate::hugo_sites::new_site_ref_linker(&s.conf, &s.deps.log);
    let lang = &s.language.lang;
    let msg = if position.is_valid() {
        format!(
            "[{lang}] REF_NOT_FOUND: Ref {}: {}: {what}",
            go_strconv::quote(r),
            position.string()
        )
    } else if let Some(p) = p {
        format!(
            "[{lang}] REF_NOT_FOUND: Ref {} from page {}: {what}",
            go_strconv::quote(r),
            go_strconv::quote(p.0.path())
        )
    } else {
        format!(
            "[{lang}] REF_NOT_FOUND: Ref {}: {what}",
            go_strconv::quote(r)
        )
    };
    if linker.log_warning {
        linker.logger.warnf(msg);
    } else {
        linker.logger.errorf(msg);
    }
}

/// Go: `siteRefLinker.notFoundURL` (`refLinksNotFoundURL`).
pub fn not_found_url(h: &HugoSites, site_idx: usize) -> String {
    h.sites[site_idx].conf.root.ref_links_not_found_url.clone()
}

/// Go: `(s *siteRefLinker) refLink(ref, source, relative, outputFormat)`.
// Go: hugolib/site.go:refLink
pub fn ref_link(
    h: &Arc<HugoSites>,
    site_idx: usize,
    r: &str,
    source: &Value,
    relative: bool,
    output_format: &str,
) -> Result<String> {
    let p = crate::page_unwrap::unwrap_page(source)?;
    let not_found = not_found_url(h, site_idx);

    // Go `filepath.ToSlash` (a no-op on unix).
    let ref_url = match go_url::parse(r) {
        Ok(u) => u,
        // Go returns `s.notFoundURL, err`; every caller returns the error.
        Err(err) => return Err(Error::new(err.to_string())),
    };

    let mut target: Option<PageRef> = None;
    let mut link = String::new();

    let ref_path = String::from_utf8_lossy(&ref_url.path).into_owned();
    let ref_fragment = String::from_utf8_lossy(&ref_url.fragment).into_owned();

    if !ref_path.is_empty() {
        let res = crate::pagecollections::new_page_finder(h, site_idx)
            .get_page_ref(p.as_ref(), &ref_path);
        let mut pos = Position::default();
        let failed = !matches!(&res, Ok(Some(_)));
        if failed {
            // Go: `source.(text.Positioner)`.
            if let Some(sc) = source.downcast::<crate::shortcode_page::ShortcodeWithPage>() {
                pos = sc.position();
            }
        }

        match res {
            Err(err) => {
                log_not_found(h, site_idx, &ref_path, &err.to_string(), p.as_ref(), &pos);
                return Ok(not_found);
            }
            Ok(None) => {
                log_not_found(h, site_idx, &ref_path, "page not found", p.as_ref(), &pos);
                return Ok(not_found);
            }
            Ok(Some(t)) => target = Some(t),
        }
        let t = target.as_ref().expect("set");

        if !output_format.is_empty() {
            let ofs = t.0.output_formats();
            let Some(o) = nh_page::page_outputformat::output_formats_get(&ofs, output_format)
            else {
                log_not_found(
                    h,
                    site_idx,
                    &ref_path,
                    &format!("output format {}", go_strconv::quote(output_format)),
                    p.as_ref(),
                    &pos,
                );
                return Ok(not_found);
            };
            link = if relative {
                o.rel_permalink().to_string()
            } else {
                o.permalink().to_string()
            };
        } else if relative {
            link = t.0.rel_permalink();
        } else {
            link = t.0.permalink();
        }
    }

    if !ref_fragment.is_empty() {
        link = format!("{link}#{ref_fragment}");
        // Go appends the converter's `DocumentInfo.AnchorSuffix()` of the target (or of the
        // source page) here; no converter implements `DocumentInfo`, so nothing is added.
        let _ = &target;
    }

    Ok(link)
}

/// The `page.Site` handle (Go `*page.siteWrapper` around `*Site`): implements `nh_page::site::Site`
/// (typed methods + template table in `tplapi::site_methods`).
#[derive(Clone)]
pub struct SiteHandle {
    pub h: Arc<HugoSites>,
    pub idx: usize,
}

impl SiteHandle {
    pub fn site(&self) -> &Site {
        &self.h.sites[self.idx]
    }

    pub fn site_ref(&self) -> nh_page::site::SiteRef {
        nh_page::site::SiteRef(Arc::new(self.clone()))
    }
}

/// `*hugolib.Site` as a template value (sitemapindex data is `[]*hugolib.Site`: `.SitemapAbsURL`,
/// `.Lastmod`, plus the siteWrapper methods).
pub struct HugolibSiteObject(pub SiteHandle);

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: hugolib/site.go (1613 lines; 42/78 funcs executed) — construction lines -> hugo_sites.rs,
//          initRenderFormats/shouldBuild -> build_assemble.rs, hookRendererTemplate -> page__per_output.rs,
//          render -> site_render.rs
//   types: siteState, Site, (group), siteInit, siteRefLinker, WhatChanged, fileEventInfo, hookRendererTemplate
//    L137-140: (s *Site) Debug()  [bad results in templates; debug printing not ported]
// OK L457-459: (s *Site) ServerPort() int
// OK L462-464: (s *Site) Title() string
// OK L466-468: (s *Site) Copyright() string
// OK L470-475: (s *Site) Config() page.SiteConfig
// OK L477-479: (s *Site) LanguageCode() string
// OK L482-488: (s *Site) Sites() page.Sites
// OK L491-493: (s *Site) Current() page.Site  [tplapi/site_methods.rs]
// OK L496-499: (s *Site) MainSections() []string
// OK L502-510: (s *Site) Hugo() neohugo.HugoInfo  [tplapi/site_methods.rs]
// OK L513-515: (s *Site) BaseURL() string
// OK L518-522: (s *Site) LastChange() time.Time
// OK L525-527: (s *Site) Lastmod() time.Time
// OK L530-532: (s *Site) Params() maps.Params
// OK L535-540: (s *Site) Author() map[string]any
// OK L543-546: (s *Site) Authors() page.AuthorList
// OK L549-552: (s *Site) Social() map[string]string
// OK L554-556: (s *Site) Param(key any) (any, error)
// OK L559-561: (s *Site) Data() map[string]any  [hugo_sites_data.rs]
// OK L563-565: (s *Site) BuildDrafts() bool
// OK L568-571: (s *Site) IsMultiLingual() bool
// OK L573-579: (s *Site) LanguagePrefix() string
// OK L581-583: (s *Site) Site() page.Site  [SiteHandle::site_ref]
//    L585-591: (s *Site) ForEeachIdentityByName(name string, f func(identity.Identity) bool)  [no identity tracking]
// OK L595-608: (s *Site) Pages() page.Pages
// OK L612-624: (s *Site) RegularPages() page.Pages
// OK L627-630: (s *Site) AllPages() page.Pages  [HugoSites::all_pages]
// OK L633-636: (s *Site) AllRegularPages() page.Pages  [HugoSites::all_regular_pages]
// OK L638-640: (s *Site) Store() *maps.Scratch
// OK L642-646: (s *Site) CheckReady()
// OK L648-652: (s *Site) Taxonomies() page.TaxonomyList
// OK L662-680: (t taxonomiesConfig) Values() taxonomiesConfigValues  [T20: content_map.rs]
//    L690-695: (init *siteInit) Reset()  [rebuilds only]
// OK L697-792: (s *Site) prepareInits()  [init_prev_next, init_prev_next_in_section, site_menus, site_taxonomies]
// OK L794-798: (s *Site) Menus() navigation.Menus
// OK L839-841: (s *Site) GetInternalRelatedDocsHandler() *page.RelatedDocsHandler
// OK L843-845: (s *Site) Language() *langs.Language
// OK L847-849: (s *Site) Languages() langs.Languages  [tplapi/site_methods.rs]
// OK L869-877: (s siteRefLinker) logNotFound(ref, what string, p page.Page, position text.Position)
// OK L879-955: (s *siteRefLinker) refLink(ref string, source any, relative bool, outputFormat string) (string, error)
// OK L957-959: (s *Site) watching() bool
//    L969-973: (w *WhatChanged) init()  [rebuilds only]
//    L975-984: (w *WhatChanged) Add(ids ...identity.Identity)  [rebuilds only]
//    L986-990: (w *WhatChanged) Clear()  [rebuilds only]
//    L992-994: (w *WhatChanged) clear()  [rebuilds only: no identities are tracked]
//    L996-1001: (w *WhatChanged) Changes() []identity.Identity  [rebuilds only]
//    L1003-1009: (w *WhatChanged) Drain() []identity.Identity  [rebuilds only]
// OK L1013-1019: (s *Site) RegisterMediaTypes()
//    L1021-1072: (h *HugoSites) fileEventsFilter(events []fsnotify.Event) []fsnotify.Event  [server mode]
//    L1082-1139: (h *HugoSites) fileEventsApplyInfo(events []fsnotify.Event) []fileEventInfo  [server mode]
//    L1141-1153: (h *HugoSites) fileEventsTrim(events []fsnotify.Event) []fsnotify.Event  [server mode]
//    L1155-1219: (h *HugoSites) fileEventsContentPaths(p []pathChange) []pathChange  [server mode]
// OK L1222-1233: (s *Site) SitemapAbsURL() string
// OK L1235-1246: (s *Site) createNodeMenuEntryURL(in string) string
// OK L1248-1357: (s *Site) assembleMenus() error
// OK L1360-1366: (s *Site) getLanguageTargetPathLang(alwaysInSubDir bool) string
// OK L1369-1379: (s *Site) getLanguagePermalinkLang(alwaysInSubDir bool) string
//    L1382-1386: (s *Site) resetBuildState(sourceChanged bool)  [rebuilds only]
// OK L1388-1397: (s *Site) errorCollator(results <-chan error, errs chan<- error)
// OK L1406-1418: (s *Site) GetPage(ref ...string) (page.Page, error)
// OK L1420-1433: (s *Site) absURLPath(targetPath string) string
// OK L1440-1489: (s *Site) renderAndWritePage(statCounter *uint64, name string, targetPath string, p *pageState, d any, templ *tplimpl.TemplInfo) error
// OK L1536-1554: (s *Site) renderForTemplate(ctx context.Context, name, outputFormat string, d any, w io.Writer, templ *tplimpl.TemplInfo) (err error)
// ---------------------------------------------------------------------------
