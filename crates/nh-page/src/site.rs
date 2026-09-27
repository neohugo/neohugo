//! Port of `resources/page/site.go`.
//!
//! Owner: Wave B task T11 (page-api-paths).


//! Go `resources/page.Site` — the site interface seen by templates (`*page.siteWrapper`) and by
//! lower crates. Implemented by nh-hugolib.

use std::any::Any;
use std::borrow::Cow;
use std::sync::Arc;

use go_value::{GoString, HostCtx, Map, Object, Value};
use nh_common::maps::scratch::Scratch;
use nh_common::object::GoResult;
use nh_common::Result;
use nh_config::neohugo::neohugo::HugoInfo;
use nh_langs::language::{Language, Languages};

use crate::navigation::menu::Menus;
use crate::page::{PageRef, Pages};
use crate::taxonomy::TaxonomyList;

/// Go: `page.Site`.
pub trait Site: Any + Send + Sync {
    /// Index of the site in `HugoSites.Sites` (language order).
    fn site_index(&self) -> usize;
    fn language(&self) -> Arc<Language>;
    fn languages(&self) -> Languages;
    /// Go `GetPage(ref ...string)`; `None` when not found (Go returns nil, nil).
    fn get_page(&self, refs: &[String]) -> Result<Option<PageRef>>;
    fn all_pages(&self) -> Pages;
    fn regular_pages(&self) -> Pages;
    fn pages(&self) -> Pages;
    fn sections(&self) -> Pages;
    fn home(&self) -> Option<PageRef>;
    fn title(&self) -> String;
    fn language_code(&self) -> String;
    fn copyright(&self) -> String;
    fn sites(&self) -> Vec<SiteRef>;
    /// Go `Current()` — the site currently being rendered (`h.currentSite`), used for the collator
    /// in `DefaultPageSort` tie-breaks.
    fn current(&self) -> SiteRef;
    fn hugo(&self) -> HugoInfo;
    fn base_url(&self) -> String;
    /// Go `Taxonomies()` (lazily built on first use).
    fn taxonomies(&self) -> TaxonomyList;
    fn last_change(&self) -> go_value::Time;
    fn lastmod(&self) -> go_value::Time;
    fn menus(&self) -> Menus;
    fn main_sections(&self) -> Vec<String>;
    fn params(&self) -> Arc<Map>;
    fn param(&self, key: &Value) -> Result<Value>;
    /// Go `Data()` (`map[string]any` from /data).
    fn data(&self) -> Arc<Map>;
    fn config(&self) -> SiteConfig;
    fn build_drafts(&self) -> bool;
    fn is_multi_lingual(&self) -> bool;
    fn language_prefix(&self) -> String;
    fn store(&self) -> Arc<Scratch>;

    // ---- template API ----
    fn tpl_has_method(&self, name: &str) -> bool;
    fn tpl_call_method(&self, ctx: HostCtx<'_>, name: &str, args: &[Value]) -> Option<GoResult<Value>>;
}

/// Template value wrapper (`*page.siteWrapper`).
#[derive(Clone)]
pub struct SiteRef(pub Arc<dyn Site>);

impl SiteRef {
    pub fn to_value(&self) -> Value {
        Value::Object(Arc::new(self.clone()))
    }
}

impl Object for SiteRef {
    fn type_name(&self) -> Cow<'_, str> {
        Cow::Borrowed("*page.siteWrapper")
    }
    fn has_method(&self, name: &str) -> bool {
        self.0.tpl_has_method(name)
    }
    fn call_method(&self, ctx: HostCtx<'_>, name: &str, args: &[Value]) -> Option<GoResult<Value>> {
        self.0.tpl_call_method(ctx, name, args)
    }
    fn identity(&self) -> usize {
        Arc::as_ptr(&self.0) as *const () as usize
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// Go: `page.SiteConfig` (`.Site.Config.Services.RSS.Limit`, `.Site.Config.Privacy`).
#[derive(Clone, Debug, Default)]
pub struct SiteConfig {
    pub privacy: nh_config::privacy::Config,
    pub services: nh_config::services::Config,
}

impl SiteConfig {
    /// Template value: struct with fields `Privacy`, `Services`.
    pub fn to_value(&self) -> Value {
        todo!()
    }
}

/// A site from a template value.
pub fn site_from_value(v: &Value) -> Option<SiteRef> {
    v.downcast::<SiteRef>().cloned()
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: resources/page/site.go (470 lines; 14/72 funcs executed)
//   types: Site, Sites, siteWrapper, testSite, SiteConfig
//    L136-139: (s Sites) First() Site
//    L143-148: (s Sites) Default() Site
// EX L157-162: WrapSite(s Site) Site
//    L164-166: (s *siteWrapper) Key() string
//    L169-171: (s *siteWrapper) Social() map[string]string
//    L174-176: (s *siteWrapper) Author() map[string]any
//    L179-181: (s *siteWrapper) Authors() AuthorList
// EX L183-185: (s *siteWrapper) GetPage(ref ...string) (Page, error)
// EX L187-189: (s *siteWrapper) Language() *langs.Language
// EX L191-193: (s *siteWrapper) Languages() langs.Languages
// EX L195-197: (s *siteWrapper) AllPages() Pages
// EX L199-201: (s *siteWrapper) RegularPages() Pages
//    L203-205: (s *siteWrapper) Pages() Pages
//    L207-209: (s *siteWrapper) Sections() Pages
//    L211-213: (s *siteWrapper) Home() Page
//    L215-217: (s *siteWrapper) ServerPort() int
// EX L219-221: (s *siteWrapper) Title() string
// EX L223-225: (s *siteWrapper) LanguageCode() string
//    L227-229: (s *siteWrapper) Copyright() string
//    L231-233: (s *siteWrapper) Sites() Sites
// EX L235-237: (s *siteWrapper) Current() Site
// EX L239-241: (s *siteWrapper) Config() SiteConfig
//    L243-245: (s *siteWrapper) Hugo() neohugo.HugoInfo
// EX L247-249: (s *siteWrapper) BaseURL() string
// EX L251-253: (s *siteWrapper) Taxonomies() TaxonomyList
//    L256-258: (s *siteWrapper) LastChange() time.Time
//    L260-262: (s *siteWrapper) Lastmod() time.Time
//    L264-266: (s *siteWrapper) Menus() navigation.Menus
//    L268-270: (s *siteWrapper) MainSections() []string
// EX L272-274: (s *siteWrapper) Params() maps.Params
//    L276-278: (s *siteWrapper) Param(key any) (any, error)
// EX L280-282: (s *siteWrapper) Data() map[string]any
//    L284-286: (s *siteWrapper) BuildDrafts() bool
//    L289-291: (s *siteWrapper) IsMultiLingual() bool
//    L293-295: (s *siteWrapper) LanguagePrefix() string
//    L297-299: (s *siteWrapper) Store() *maps.Scratch
//    L302-304: (s *siteWrapper) ForEeachIdentityByName(name string, f func(identity.Identity) bool)
//    L307-309: (s *siteWrapper) CheckReady()
//    L317-319: (s testSite) Author() map[string]any
//    L322-324: (s testSite) Authors() AuthorList
//    L327-329: (s testSite) Social() map[string]string
//    L331-333: (t testSite) Hugo() neohugo.HugoInfo
//    L335-337: (t testSite) ServerPort() int
//    L340-342: (testSite) LastChange() (t time.Time)
//    L344-346: (testSite) Lastmod() (t time.Time)
//    L348-350: (t testSite) Title() string
//    L352-354: (t testSite) LanguageCode() string
//    L356-358: (t testSite) Copyright() string
//    L360-362: (t testSite) Sites() Sites
//    L364-366: (t testSite) Sections() Pages
//    L368-370: (t testSite) GetPage(ref ...string) (Page, error)
//    L372-374: (t testSite) Current() Site
//    L376-378: (s testSite) LanguagePrefix() string
//    L380-382: (t testSite) Languages() langs.Languages
//    L384-386: (t testSite) MainSections() []string
//    L388-390: (t testSite) Language() *langs.Language
//    L392-394: (t testSite) Home() Page
//    L396-398: (t testSite) Pages() Pages
//    L400-402: (t testSite) AllPages() Pages
//    L404-406: (t testSite) RegularPages() Pages
//    L408-410: (t testSite) Menus() navigation.Menus
//    L412-414: (t testSite) Taxonomies() TaxonomyList
//    L416-418: (t testSite) BaseURL() string
//    L420-422: (t testSite) Params() maps.Params
//    L424-426: (t testSite) Data() map[string]any
//    L428-430: (s testSite) Config() SiteConfig
//    L432-434: (s testSite) BuildDrafts() bool
//    L437-439: (s testSite) IsMultiLingual() bool
//    L441-443: (s testSite) Param(key any) (any, error)
//    L445-447: (s testSite) Store() *maps.Scratch
//    L449-450: (s testSite) CheckReady()
//    L453-460: NewDummyHugoSite(conf config.AllProvider) Site
// ---------------------------------------------------------------------------
