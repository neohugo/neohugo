//! Port of `resources/page/site.go`.
//!
//! Owner: Wave B task T11 (page-api-paths).
//!
//! Go `resources/page.Site` — the site interface seen by templates (`*page.siteWrapper`) and by
//! lower crates. Implemented by nh-hugolib. [`SiteRef`] is Go's `siteWrapper` (`WrapSite`): its
//! methods delegate to the wrapped site.

use std::any::Any;
use std::borrow::Cow;
use std::collections::BTreeMap;
use std::sync::Arc;

use go_value::{GoString, HostCtx, Map, MapType, Object, Value};
use nh_common::Result;
use nh_common::maps::scratch::Scratch;
use nh_common::object::GoResult;
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
    fn tpl_call_method(
        &self,
        ctx: HostCtx<'_>,
        name: &str,
        args: &[Value],
    ) -> Option<GoResult<Value>>;
}

/// Template value wrapper (`*page.siteWrapper`).
#[derive(Clone)]
pub struct SiteRef(pub Arc<dyn Site>);

impl SiteRef {
    /// Go: `WrapSite(s)`.
    // Go: resources/page/site.go:WrapSite
    pub fn wrap(s: Arc<dyn Site>) -> SiteRef {
        SiteRef(s)
    }

    pub fn to_value(&self) -> Value {
        Value::Object(Arc::new(self.clone()))
    }

    /// Go: `(*siteWrapper).Key()` — the language code.
    // Go: resources/page/site.go:Key
    pub fn key(&self) -> String {
        self.0.language().lang.clone()
    }
}

impl Object for SiteRef {
    fn type_name(&self) -> Cow<'_, str> {
        Cow::Borrowed("*page.siteWrapper")
    }
    fn has_method(&self, name: &str) -> bool {
        name == "Key" || self.0.tpl_has_method(name)
    }
    fn call_method(&self, ctx: HostCtx<'_>, name: &str, args: &[Value]) -> Option<GoResult<Value>> {
        if name == "Key" && !self.0.tpl_has_method(name) {
            return Some(
                nh_common::object::args::exactly(args, 0, name).map(|_| Value::string(self.key())),
            );
        }
        self.0.tpl_call_method(ctx, name, args)
    }
    fn hash_key(&self) -> Option<GoString> {
        Some(GoString::from(self.key()))
    }
    fn identity(&self) -> usize {
        Arc::as_ptr(&self.0) as *const () as usize
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// Go: `page.Sites`.
pub type Sites = Vec<SiteRef>;

/// Go: `Sites.Default()` — the site of the default content language (the first).
// Go: resources/page/site.go:Default
pub fn sites_default(s: &Sites) -> Option<SiteRef> {
    s.first().cloned()
}

/// Go: `Sites.First()` (deprecated alias of `Default`; Go logs a deprecation).
// Go: resources/page/site.go:First
pub fn sites_first(s: &Sites) -> Option<SiteRef> {
    sites_default(s)
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
        Value::object(self.clone())
    }
}

impl Object for SiteConfig {
    fn type_name(&self) -> Cow<'_, str> {
        Cow::Borrowed("page.SiteConfig")
    }
    fn kind(&self) -> go_value::Kind {
        go_value::Kind::Struct
    }
    fn has_method(&self, _name: &str) -> bool {
        false
    }
    fn call_method(
        &self,
        _ctx: HostCtx<'_>,
        _name: &str,
        _args: &[Value],
    ) -> Option<GoResult<Value>> {
        None
    }
    fn field(&self, name: &str) -> Option<Value> {
        match name {
            "Privacy" => Some(Value::object(self.privacy.clone())),
            "Services" => Some(Value::object(self.services.clone())),
            _ => None,
        }
    }
    fn struct_fields(&self) -> Option<Vec<(Cow<'_, str>, Value)>> {
        Some(vec![
            (
                Cow::Borrowed("Privacy"),
                Value::object(self.privacy.clone()),
            ),
            (
                Cow::Borrowed("Services"),
                Value::object(self.services.clone()),
            ),
        ])
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// A site from a template value.
pub fn site_from_value(v: &Value) -> Option<SiteRef> {
    v.downcast::<SiteRef>().cloned()
}

/// Go: `testSite` (`NewDummyHugoSite`): a minimal site for tests. Its collections are empty and
/// its language is `en`.
#[derive(Clone)]
pub struct DummySite {
    pub h: HugoInfo,
    pub l: Arc<Language>,
}

/// Go: `page.NewDummyHugoSite(conf)` — `conf` provides the `HugoInfo` (Go `neohugo.NewInfo`).
// Go: resources/page/site.go:NewDummyHugoSite
pub fn new_dummy_hugo_site(
    conf: Arc<dyn nh_config::neohugo::neohugo::HugoInfoConfig>,
) -> Result<Arc<DummySite>> {
    Ok(Arc::new(DummySite {
        h: HugoInfo::new(conf),
        l: Language::new("en", "en", "", Default::default())?,
    }))
}

impl Site for DummySite {
    fn site_index(&self) -> usize {
        0
    }
    // Go: resources/page/site.go:Language
    fn language(&self) -> Arc<Language> {
        self.l.clone()
    }
    // Go: resources/page/site.go:Languages
    fn languages(&self) -> Languages {
        Vec::new()
    }
    // Go: resources/page/site.go:GetPage
    fn get_page(&self, _refs: &[String]) -> Result<Option<PageRef>> {
        Ok(None)
    }
    // Go: resources/page/site.go:AllPages
    fn all_pages(&self) -> Pages {
        Vec::new()
    }
    // Go: resources/page/site.go:RegularPages
    fn regular_pages(&self) -> Pages {
        Vec::new()
    }
    // Go: resources/page/site.go:Pages
    fn pages(&self) -> Pages {
        Vec::new()
    }
    // Go: resources/page/site.go:Sections
    fn sections(&self) -> Pages {
        Vec::new()
    }
    // Go: resources/page/site.go:Home
    fn home(&self) -> Option<PageRef> {
        None
    }
    // Go: resources/page/site.go:Title
    fn title(&self) -> String {
        "foo".to_string()
    }
    // Go: resources/page/site.go:LanguageCode
    fn language_code(&self) -> String {
        "en".to_string()
    }
    // Go: resources/page/site.go:Copyright
    fn copyright(&self) -> String {
        String::new()
    }
    // Go: resources/page/site.go:Sites
    fn sites(&self) -> Vec<SiteRef> {
        Vec::new()
    }
    // Go: resources/page/site.go:Current
    fn current(&self) -> SiteRef {
        SiteRef(Arc::new(self.clone()))
    }
    // Go: resources/page/site.go:Hugo
    fn hugo(&self) -> HugoInfo {
        self.h.clone()
    }
    // Go: resources/page/site.go:BaseURL
    fn base_url(&self) -> String {
        String::new()
    }
    // Go: resources/page/site.go:Taxonomies
    fn taxonomies(&self) -> TaxonomyList {
        Arc::new(BTreeMap::new())
    }
    // Go: resources/page/site.go:LastChange
    fn last_change(&self) -> go_value::Time {
        go_value::Time::zero()
    }
    // Go: resources/page/site.go:Lastmod
    fn lastmod(&self) -> go_value::Time {
        go_value::Time::zero()
    }
    // Go: resources/page/site.go:Menus
    fn menus(&self) -> Menus {
        Arc::new(BTreeMap::new())
    }
    // Go: resources/page/site.go:MainSections
    fn main_sections(&self) -> Vec<String> {
        Vec::new()
    }
    // Go: resources/page/site.go:Params
    fn params(&self) -> Arc<Map> {
        Arc::new(Map::new(MapType::Params))
    }
    // Go: resources/page/site.go:Param
    fn param(&self, _key: &Value) -> Result<Value> {
        Ok(Value::Invalid)
    }
    // Go: resources/page/site.go:Data
    fn data(&self) -> Arc<Map> {
        Arc::new(Map::new(MapType::StringAny))
    }
    // Go: resources/page/site.go:Config
    fn config(&self) -> SiteConfig {
        SiteConfig::default()
    }
    // Go: resources/page/site.go:BuildDrafts
    fn build_drafts(&self) -> bool {
        false
    }
    // Go: resources/page/site.go:IsMultiLingual
    fn is_multi_lingual(&self) -> bool {
        false
    }
    // Go: resources/page/site.go:LanguagePrefix
    fn language_prefix(&self) -> String {
        String::new()
    }
    // Go: resources/page/site.go:Store
    fn store(&self) -> Arc<Scratch> {
        Arc::new(Scratch::new())
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

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: resources/page/site.go (470 lines; 14/72 funcs executed)
//   types: Site, Sites, siteWrapper, testSite, SiteConfig
// The siteWrapper methods are `SiteRef`'s: it delegates every call to the wrapped site's
// template table (`Site::tpl_call_method`, nh-hugolib) and answers `Key` itself.
// The testSite methods are `DummySite`'s.
// OK L136-139: (s Sites) First() Site
// OK L143-148: (s Sites) Default() Site
// OK L157-162: WrapSite(s Site) Site
// OK L164-166: (s *siteWrapper) Key() string
// OK L169-171: (s *siteWrapper) Social() map[string]string
// OK L174-176: (s *siteWrapper) Author() map[string]any
// OK L179-181: (s *siteWrapper) Authors() AuthorList
// OK L183-185: (s *siteWrapper) GetPage(ref ...string) (Page, error)
// OK L187-189: (s *siteWrapper) Language() *langs.Language
// OK L191-193: (s *siteWrapper) Languages() langs.Languages
// OK L195-197: (s *siteWrapper) AllPages() Pages
// OK L199-201: (s *siteWrapper) RegularPages() Pages
// OK L203-205: (s *siteWrapper) Pages() Pages
// OK L207-209: (s *siteWrapper) Sections() Pages
// OK L211-213: (s *siteWrapper) Home() Page
// OK L215-217: (s *siteWrapper) ServerPort() int
// OK L219-221: (s *siteWrapper) Title() string
// OK L223-225: (s *siteWrapper) LanguageCode() string
// OK L227-229: (s *siteWrapper) Copyright() string
// OK L231-233: (s *siteWrapper) Sites() Sites
// OK L235-237: (s *siteWrapper) Current() Site
// OK L239-241: (s *siteWrapper) Config() SiteConfig
// OK L243-245: (s *siteWrapper) Hugo() neohugo.HugoInfo
// OK L247-249: (s *siteWrapper) BaseURL() string
// OK L251-253: (s *siteWrapper) Taxonomies() TaxonomyList
// OK L256-258: (s *siteWrapper) LastChange() time.Time
// OK L260-262: (s *siteWrapper) Lastmod() time.Time
// OK L264-266: (s *siteWrapper) Menus() navigation.Menus
// OK L268-270: (s *siteWrapper) MainSections() []string
// OK L272-274: (s *siteWrapper) Params() maps.Params
// OK L276-278: (s *siteWrapper) Param(key any) (any, error)
// OK L280-282: (s *siteWrapper) Data() map[string]any
// OK L284-286: (s *siteWrapper) BuildDrafts() bool
// OK L289-291: (s *siteWrapper) IsMultiLingual() bool
// OK L293-295: (s *siteWrapper) LanguagePrefix() string
// OK L297-299: (s *siteWrapper) Store() *maps.Scratch
// OK L302-304: (s *siteWrapper) ForEeachIdentityByName(name string, f func(identity.Identity) bool)
// OK L307-309: (s *siteWrapper) CheckReady()
// OK L317-319: (s testSite) Author() map[string]any
// OK L322-324: (s testSite) Authors() AuthorList
// OK L327-329: (s testSite) Social() map[string]string
// OK L331-333: (t testSite) Hugo() neohugo.HugoInfo
// OK L335-337: (t testSite) ServerPort() int
// OK L340-342: (testSite) LastChange() (t time.Time)
// OK L344-346: (testSite) Lastmod() (t time.Time)
// OK L348-350: (t testSite) Title() string
// OK L352-354: (t testSite) LanguageCode() string
// OK L356-358: (t testSite) Copyright() string
// OK L360-362: (t testSite) Sites() Sites
// OK L364-366: (t testSite) Sections() Pages
// OK L368-370: (t testSite) GetPage(ref ...string) (Page, error)
// OK L372-374: (t testSite) Current() Site
// OK L376-378: (s testSite) LanguagePrefix() string
// OK L380-382: (t testSite) Languages() langs.Languages
// OK L384-386: (t testSite) MainSections() []string
// OK L388-390: (t testSite) Language() *langs.Language
// OK L392-394: (t testSite) Home() Page
// OK L396-398: (t testSite) Pages() Pages
// OK L400-402: (t testSite) AllPages() Pages
// OK L404-406: (t testSite) RegularPages() Pages
// OK L408-410: (t testSite) Menus() navigation.Menus
// OK L412-414: (t testSite) Taxonomies() TaxonomyList
// OK L416-418: (t testSite) BaseURL() string
// OK L420-422: (t testSite) Params() maps.Params
// OK L424-426: (t testSite) Data() map[string]any
// OK L428-430: (s testSite) Config() SiteConfig
// OK L432-434: (s testSite) BuildDrafts() bool
// OK L437-439: (s testSite) IsMultiLingual() bool
// OK L441-443: (s testSite) Param(key any) (any, error)
// OK L445-447: (s testSite) Store() *maps.Scratch
// OK L449-450: (s testSite) CheckReady()
// OK L453-460: NewDummyHugoSite(conf config.AllProvider) Site
// ---------------------------------------------------------------------------
