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

use std::sync::{Arc, OnceLock};

use go_value::{Map, Time, Value};
use nh_allconfig::allconfig::Config;
use nh_common::Result;
use nh_common::maps::scratch::Scratch;
use nh_deps::deps::Deps;
use nh_langs::language::Language;
use nh_media::output::output_format::Formats;
use nh_page::navigation::menu::Menus;
use nh_page::pagemeta::page_frontmatter::FrontMatterHandler;
use nh_page::pages_related::RelatedDocsHandler;
use nh_page::taxonomy::TaxonomyList;
use nh_publisher::publisher::DestinationPublisher;
use nh_tplimpl::templatestore::TemplateStore;

use crate::content_map_page::PageMap;
use crate::hugo_sites::HugoSites;
use crate::page::PageId;

/// Go: `hugolib.Site`.
pub struct Site {
    /// Index in `HugoSites.sites` (Go `languagei`).
    pub idx: usize,
    pub conf: Arc<Config>,
    pub language: Arc<Language>,
    pub deps: Arc<Deps>,
    pub page_map: PageMap,
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
    /// Go: `renderAndWritePage(statCounter, name, targetPath, p, d, templ)`.
    // Go: hugolib/site.go:renderAndWritePage
    pub fn render_and_write_page(
        h: &Arc<HugoSites>,
        site_idx: usize,
        target_path: &str,
        p: PageId,
        data: &Value,
        templ: &Arc<nh_tplimpl::templatestore::TemplInfo>,
    ) -> Result<()> {
        todo!()
    }

    /// Go: `absURLPath(targetPath)` — baseURL with trailing `/` (or dotted relative path with relativeURLs).
    // Go: hugolib/site.go:absURLPath
    pub fn abs_url_path(&self, target_path: &str) -> String {
        todo!()
    }

    /// Go: `getLanguageTargetPathLang` / `getLanguagePermalinkLang` (sitemaps: always in subdir).
    // Go: hugolib/site.go:getLanguageTargetPathLang
    pub fn get_language_target_path_lang(&self, always_in_sub_dir: bool) -> String {
        todo!()
    }

    /// Go: `SitemapAbsURL()` (sitemapindex).
    // Go: hugolib/site.go:SitemapAbsURL
    pub fn sitemap_abs_url(&self) -> String {
        todo!()
    }
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
//    L137-140: (s *Site) Debug()
//    L457-459: (s *Site) ServerPort() int
// EX L462-464: (s *Site) Title() string
//    L466-468: (s *Site) Copyright() string
// EX L470-475: (s *Site) Config() page.SiteConfig
// EX L477-479: (s *Site) LanguageCode() string
//    L482-488: (s *Site) Sites() page.Sites
// EX L491-493: (s *Site) Current() page.Site
//    L496-499: (s *Site) MainSections() []string
// EX L502-510: (s *Site) Hugo() neohugo.HugoInfo
// EX L513-515: (s *Site) BaseURL() string
//    L518-522: (s *Site) LastChange() time.Time
// EX L525-527: (s *Site) Lastmod() time.Time
// EX L530-532: (s *Site) Params() maps.Params
//    L535-540: (s *Site) Author() map[string]any
//    L543-546: (s *Site) Authors() page.AuthorList
//    L549-552: (s *Site) Social() map[string]string
//    L554-556: (s *Site) Param(key any) (any, error)
// EX L559-561: (s *Site) Data() map[string]any
//    L563-565: (s *Site) BuildDrafts() bool
//    L568-571: (s *Site) IsMultiLingual() bool
//    L573-579: (s *Site) LanguagePrefix() string
//    L581-583: (s *Site) Site() page.Site
//    L585-591: (s *Site) ForEeachIdentityByName(name string, f func(identity.Identity) bool)
// EX L595-608: (s *Site) Pages() page.Pages
// EX L612-624: (s *Site) RegularPages() page.Pages
// EX L627-630: (s *Site) AllPages() page.Pages
//    L633-636: (s *Site) AllRegularPages() page.Pages
//    L638-640: (s *Site) Store() *maps.Scratch
// EX L642-646: (s *Site) CheckReady()
// EX L648-652: (s *Site) Taxonomies() page.TaxonomyList
// EX L662-680: (t taxonomiesConfig) Values() taxonomiesConfigValues
//    L690-695: (init *siteInit) Reset()
// EX L697-792: (s *Site) prepareInits()
//    L794-798: (s *Site) Menus() navigation.Menus
// EX L839-841: (s *Site) GetInternalRelatedDocsHandler() *page.RelatedDocsHandler
// EX L843-845: (s *Site) Language() *langs.Language
// EX L847-849: (s *Site) Languages() langs.Languages
//    L869-877: (s siteRefLinker) logNotFound(ref, what string, p page.Page, position text.Position)
// EX L879-955: (s *siteRefLinker) refLink(ref string, source any, relative bool, outputFormat string) (string, error)
// EX L957-959: (s *Site) watching() bool
//    L969-973: (w *WhatChanged) init()
//    L975-984: (w *WhatChanged) Add(ids ...identity.Identity)
//    L986-990: (w *WhatChanged) Clear()
// EX L992-994: (w *WhatChanged) clear()
// EX L996-1001: (w *WhatChanged) Changes() []identity.Identity
// EX L1003-1009: (w *WhatChanged) Drain() []identity.Identity
//    L1013-1019: (s *Site) RegisterMediaTypes()
//    L1021-1072: (h *HugoSites) fileEventsFilter(events []fsnotify.Event) []fsnotify.Event
//    L1082-1139: (h *HugoSites) fileEventsApplyInfo(events []fsnotify.Event) []fileEventInfo
//    L1141-1153: (h *HugoSites) fileEventsTrim(events []fsnotify.Event) []fsnotify.Event
//    L1155-1219: (h *HugoSites) fileEventsContentPaths(p []pathChange) []pathChange
// EX L1222-1233: (s *Site) SitemapAbsURL() string
//    L1235-1246: (s *Site) createNodeMenuEntryURL(in string) string
//    L1248-1357: (s *Site) assembleMenus() error
// EX L1360-1366: (s *Site) getLanguageTargetPathLang(alwaysInSubDir bool) string
// EX L1369-1379: (s *Site) getLanguagePermalinkLang(alwaysInSubDir bool) string
//    L1382-1386: (s *Site) resetBuildState(sourceChanged bool)
// EX L1388-1397: (s *Site) errorCollator(results <-chan error, errs chan<- error)
// EX L1406-1418: (s *Site) GetPage(ref ...string) (page.Page, error)
// EX L1420-1433: (s *Site) absURLPath(targetPath string) string
// EX L1440-1489: (s *Site) renderAndWritePage(statCounter *uint64, name string, targetPath string, p *pageState, d any, templ *tplimpl.TemplInfo) error
// EX L1536-1554: (s *Site) renderForTemplate(ctx context.Context, name, outputFormat string, d any, w io.Writer, templ *tplimpl.TemplInfo) (err error)
// ---------------------------------------------------------------------------
