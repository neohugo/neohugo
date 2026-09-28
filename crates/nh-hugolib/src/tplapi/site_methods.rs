//! Module `tplapi::site_methods`.
//!
//! NEW: template-visible method set of page.Site (*page.siteWrapper) and *hugolib.Site
//!
//! Owner: Wave B task T23 (hugolib-site).

//! The template-visible method set of `*page.siteWrapper` (Go `page.Site`) and `*hugolib.Site`.
//! `nh_page::site::Site` is implemented for `crate::site::SiteHandle` here.
//!
//! Members seeksnack uses: AllPages, BaseURL, Config (.Services.RSS.Limit), Data, GetPage,
//! Home, Language, LanguageCode, Languages, Params (+ the `mainsections` special case lives in
//! the exec helper), RegularPages (+ `.Related`), Taxonomies, Title; `*hugolib.Site`: SitemapAbsURL,
//! Lastmod. Full Go interface: see resources/page/site.go.
//!
//! Method sets (from Go's reflect, checked by `tests/site.rs` against the `site` oracle):
//! * `*page.siteWrapper`: the 35 methods of `resources/page/site.go` (`Key` is answered by
//!   `nh_page::site::SiteRef` itself);
//! * `*hugolib.Site` (the elements of the sitemapindex data, and `.Site.Current`): its 80
//!   methods, including those promoted from the embedded `*deps.Deps` / `*helpers.PathSpec`.
//!   The path and URL helpers are ported; the build-internal ones (`Clone`, `Close`, `Init`,
//!   `LockBuild`, file systems, error collectors, ...) are explicit unsupported errors.

use std::sync::Arc;

use go_value::{HostCtx, Map, MapType, Object, Time, Value};
use nh_common::Result;
use nh_common::maps::scratch::Scratch;
use nh_common::object::{GoResult, args};
use nh_config::neohugo::neohugo::HugoInfo;
use nh_langs::language::{Language, LanguageObject, Languages};
use nh_page::navigation::menu::Menus;
use nh_page::page::{PageRef, Pages};
use nh_page::site::{Site, SiteConfig, SiteRef};
use nh_page::taxonomy::TaxonomyList;

use super::values::{
    arity, arity_variadic, bad_results, nil_of, page_or_nil, pages_nil_if_empty, strings_opt_value,
    unsupported,
};
use crate::site::{HugolibSiteObject, SiteHandle};

impl Site for SiteHandle {
    fn site_index(&self) -> usize {
        self.idx
    }
    // Go: hugolib/site.go:Language
    fn language(&self) -> Arc<Language> {
        self.site().language()
    }
    // Go: hugolib/site.go:Languages
    fn languages(&self) -> Languages {
        self.h.configs.languages.clone()
    }
    // Go: hugolib/site.go:GetPage
    fn get_page(&self, refs: &[String]) -> Result<Option<PageRef>> {
        let (p, err) = crate::site::site_get_page(&self.h, self.idx, refs);
        match err {
            Some(e) => Err(e),
            None => Ok(Some(p)),
        }
    }
    // Go: hugolib/site.go:AllPages
    fn all_pages(&self) -> Pages {
        self.h.all_pages()
    }
    // Go: hugolib/site.go:RegularPages
    fn regular_pages(&self) -> Pages {
        crate::site::site_regular_pages(&self.h, self.idx)
    }
    // Go: hugolib/site.go:Pages
    fn pages(&self) -> Pages {
        crate::site::site_pages(&self.h, self.idx)
    }
    // Go: hugolib/site_sections.go:Sections
    fn sections(&self) -> Pages {
        crate::site_sections::sections(self)
    }
    // Go: hugolib/site_sections.go:Home
    fn home(&self) -> Option<PageRef> {
        crate::site_sections::home(self)
    }
    // Go: hugolib/site.go:Title
    fn title(&self) -> String {
        self.site().title()
    }
    // Go: hugolib/site.go:LanguageCode
    fn language_code(&self) -> String {
        self.site().language_code()
    }
    // Go: hugolib/site.go:Copyright
    fn copyright(&self) -> String {
        self.site().copyright()
    }
    // Go: hugolib/site.go:Sites
    fn sites(&self) -> Vec<SiteRef> {
        crate::site::site_sites(&self.h)
    }
    // Go: hugolib/site.go:Current
    fn current(&self) -> SiteRef {
        let idx = self
            .h
            .current_site
            .load(std::sync::atomic::Ordering::Relaxed);
        SiteHandle {
            h: self.h.clone(),
            idx,
        }
        .site_ref()
    }
    // Go: hugolib/site.go:Hugo
    fn hugo(&self) -> HugoInfo {
        self.h.hugo_info.clone()
    }
    // Go: hugolib/site.go:BaseURL
    fn base_url(&self) -> String {
        self.site().base_url()
    }
    // Go: hugolib/site.go:Taxonomies
    fn taxonomies(&self) -> TaxonomyList {
        crate::site::site_taxonomies(&self.h, self.idx)
    }
    // Go: hugolib/site.go:LastChange
    fn last_change(&self) -> Time {
        self.site().last_change()
    }
    // Go: hugolib/site.go:Lastmod
    fn lastmod(&self) -> Time {
        self.site().lastmod()
    }
    // Go: hugolib/site.go:Menus
    fn menus(&self) -> Menus {
        crate::site::site_menus(&self.h, self.idx)
    }
    // Go: hugolib/site.go:MainSections
    fn main_sections(&self) -> Vec<String> {
        self.site().main_sections().unwrap_or_default()
    }
    // Go: hugolib/site.go:Params
    fn params(&self) -> Arc<Map> {
        self.site().params()
    }
    // Go: hugolib/site.go:Param
    fn param(&self, key: &Value) -> Result<Value> {
        self.site().param(key)
    }
    // Go: hugolib/site.go:Data
    fn data(&self) -> Arc<Map> {
        self.h.data()
    }
    // Go: hugolib/site.go:Config
    fn config(&self) -> SiteConfig {
        self.site().config()
    }
    // Go: hugolib/site.go:BuildDrafts
    fn build_drafts(&self) -> bool {
        self.site().build_drafts()
    }
    // Go: hugolib/site.go:IsMultiLingual
    fn is_multi_lingual(&self) -> bool {
        crate::site::is_multi_lingual(&self.h)
    }
    // Go: hugolib/site.go:LanguagePrefix
    fn language_prefix(&self) -> String {
        self.site().language_prefix()
    }
    // Go: hugolib/site.go:Store
    fn store(&self) -> Arc<Scratch> {
        self.site().store()
    }
    fn tpl_has_method(&self, name: &str) -> bool {
        SITE_WRAPPER_METHODS.binary_search(&name).is_ok()
    }
    fn tpl_call_method(
        &self,
        ctx: HostCtx<'_>,
        name: &str,
        args: &[Value],
    ) -> Option<GoResult<Value>> {
        if !self.tpl_has_method(name) {
            return None;
        }
        Some(call_site_method(self, ctx, name, args, false))
    }
}

/// The exported method set of `*page.siteWrapper` (Go reflect, sorted), without `Key`, which
/// `SiteRef` answers.
pub const SITE_WRAPPER_METHODS: &[&str] = &[
    "AllPages",
    "Author",
    "Authors",
    "BaseURL",
    "BuildDrafts",
    "CheckReady",
    "Config",
    "Copyright",
    "Current",
    "Data",
    "ForEeachIdentityByName",
    "GetPage",
    "Home",
    "Hugo",
    "IsMultiLingual",
    "Language",
    "LanguageCode",
    "LanguagePrefix",
    "Languages",
    "LastChange",
    "Lastmod",
    "MainSections",
    "Menus",
    "Pages",
    "Param",
    "Params",
    "RegularPages",
    "Sections",
    "ServerPort",
    "Sites",
    "Social",
    "Store",
    "Taxonomies",
    "Title",
];

/// The exported method set of `*hugolib.Site` (Go reflect, sorted; promoted methods included).
pub const HUGOLIB_SITE_METHODS: &[&str] = &[
    "AbsPathify",
    "AbsProjectContentDir",
    "AbsURL",
    "AllModules",
    "AllPages",
    "AllRegularPages",
    "Author",
    "Authors",
    "BaseURL",
    "BuildDrafts",
    "CheckReady",
    "Clone",
    "Close",
    "Compile",
    "Config",
    "Copyright",
    "Current",
    "Data",
    "Debug",
    "ForEeachIdentityByName",
    "GetBasePath",
    "GetInternalRelatedDocsHandler",
    "GetLanguagePrefix",
    "GetPage",
    "GetTargetLanguageBasePath",
    "GetTemplateStore",
    "Home",
    "Hugo",
    "Init",
    "IsAbsURL",
    "IsContent",
    "IsMultiLingual",
    "IsStatic",
    "Lang",
    "Language",
    "LanguageCode",
    "LanguagePrefix",
    "Languages",
    "LastChange",
    "Lastmod",
    "LockBuild",
    "MainSections",
    "MakePath",
    "MakePathSanitized",
    "MakePathsSanitized",
    "MakeStaticPathRelative",
    "Menus",
    "MkdirTemp",
    "Pages",
    "Param",
    "Params",
    "PermalinkForBaseURL",
    "PrependBasePath",
    "RegisterMediaTypes",
    "RegularPages",
    "RelPathify",
    "RelURL",
    "ResolveJSConfigFile",
    "ResolveMarkup",
    "ResolvePaths",
    "SanitizeAnchorName",
    "Sections",
    "SendError",
    "ServerPort",
    "Site",
    "SitemapAbsURL",
    "Sites",
    "Social",
    "StartErrorCollector",
    "StatResource",
    "StaticFs",
    "StopErrorCollector",
    "Store",
    "Taxonomies",
    "Title",
    "TrimShortHTML",
    "URLEscape",
    "URLize",
    "URLizeFilename",
    "WatchFilenames",
];

/// `*hugolib.Site` as a value of `h.sites[idx]`.
fn hugolib_site_value(sh: &SiteHandle) -> Value {
    Value::object(HugolibSiteObject(sh.clone()))
}

/// A map value of Go type `map[string]any` / `map[string]string` (`None` is Go's nil map).
fn map_value(m: Option<Map>, ty: MapType) -> Value {
    match m {
        Some(mut m) => {
            m.ty = ty;
            Value::map(m)
        }
        None => nil_of(&ty.go_name()),
    }
}

/// The site methods (Go `*Site`, reached through `*page.siteWrapper` or directly). `hugolib`
/// selects the `*hugolib.Site` receiver (whose `Site()`/promoted methods differ).
fn call_site_method(
    sh: &SiteHandle,
    _ctx: HostCtx<'_>,
    name: &str,
    a: &[Value],
    hugolib: bool,
) -> GoResult<Value> {
    let s = sh.site();
    let h = &sh.h;
    let recv = if hugolib {
        "*hugolib.Site"
    } else {
        "*page.siteWrapper"
    };
    let ps = || s.deps.path_spec();
    match name {
        "AllPages" => {
            arity(a, 0, false, name)?;
            Ok(pages_nil_if_empty(h.all_pages()))
        }
        "AllRegularPages" => {
            arity(a, 0, false, name)?;
            Ok(pages_nil_if_empty(h.all_regular_pages()))
        }
        "Author" => {
            arity(a, 0, false, name)?;
            Ok(map_value(s.author(), MapType::StringAny))
        }
        "Authors" => {
            arity(a, 0, false, name)?;
            Ok(s.authors())
        }
        "BaseURL" => {
            arity(a, 0, false, name)?;
            Ok(Value::string(s.base_url()))
        }
        "BuildDrafts" => {
            arity(a, 0, false, name)?;
            Ok(Value::Bool(s.build_drafts()))
        }
        "CheckReady" => bad_results(a, 0, false, false, name, 0),
        "Config" => {
            arity(a, 0, false, name)?;
            Ok(s.config().to_value())
        }
        "Copyright" => {
            arity(a, 0, false, name)?;
            Ok(Value::string(s.copyright()))
        }
        "Current" => {
            arity(a, 0, false, name)?;
            // Go `h.currentSite` is a `*Site`.
            let idx = h.current_site.load(std::sync::atomic::Ordering::Relaxed);
            Ok(hugolib_site_value(&SiteHandle { h: h.clone(), idx }))
        }
        "Data" => {
            arity(a, 0, false, name)?;
            Ok(Value::Map(h.data()))
        }
        "Debug" => bad_results(a, 0, false, false, name, 0),
        "ForEeachIdentityByName" => bad_results(a, 2, false, false, name, 0),
        "GetPage" => {
            arity_variadic(a, 1, false, name)?;
            let refs: Vec<String> = args::strings(a, 0)?
                .into_iter()
                .map(|s| s.to_str_lossy().into_owned())
                .collect();
            let (p, err) = crate::site::site_get_page(h, sh.idx, &refs);
            match err {
                Some(e) => Err(e.into()),
                None => Ok(p.to_value()),
            }
        }
        "Home" => {
            arity(a, 0, false, name)?;
            Ok(page_or_nil(crate::site_sections::home(sh)))
        }
        "Hugo" => {
            arity(a, 0, false, name)?;
            Ok(nh_config::neohugo::neohugo::hugo_info_value(&h.hugo_info))
        }
        "IsMultiLingual" => {
            arity(a, 0, false, name)?;
            Ok(Value::Bool(crate::site::is_multi_lingual(h)))
        }
        "Language" => {
            arity(a, 0, false, name)?;
            Ok(Value::object(LanguageObject(s.language())))
        }
        "LanguageCode" => {
            arity(a, 0, false, name)?;
            Ok(Value::string(s.language_code()))
        }
        "LanguagePrefix" => {
            arity(a, 0, false, name)?;
            Ok(Value::string(s.language_prefix()))
        }
        "Languages" => {
            arity(a, 0, false, name)?;
            Ok(nh_langs::language::languages_to_value(&h.configs.languages))
        }
        "LastChange" => {
            arity(a, 0, false, name)?;
            Ok(Value::Time(s.last_change()))
        }
        "Lastmod" => {
            arity(a, 0, false, name)?;
            Ok(Value::Time(s.lastmod()))
        }
        "MainSections" => {
            arity(a, 0, false, name)?;
            Ok(strings_opt_value(s.main_sections()))
        }
        "Menus" => {
            arity(a, 0, false, name)?;
            Ok(nh_page::navigation::menu::menus_to_value(
                &crate::site::site_menus(h, sh.idx),
            ))
        }
        "Pages" => {
            arity(a, 0, false, name)?;
            Ok(pages_nil_if_empty(crate::site::site_pages(h, sh.idx)))
        }
        "Param" => {
            arity(a, 1, false, name)?;
            s.param(&a[0]).map_err(Into::into)
        }
        "Params" => {
            arity(a, 0, false, name)?;
            Ok(Value::Map(s.params()))
        }
        "RegularPages" => {
            arity(a, 0, false, name)?;
            Ok(pages_nil_if_empty(crate::site::site_regular_pages(
                h, sh.idx,
            )))
        }
        "Sections" => {
            arity(a, 0, false, name)?;
            Ok(pages_nil_if_empty(crate::site_sections::sections(sh)))
        }
        "ServerPort" => {
            arity(a, 0, false, name)?;
            Ok(Value::int(s.server_port()))
        }
        "Sites" => {
            arity(a, 0, false, name)?;
            Ok(crate::site::sites_to_value(&crate::site::site_sites(h)))
        }
        "Social" => {
            arity(a, 0, false, name)?;
            Ok(map_value(s.social(), MapType::StringString))
        }
        "Store" => {
            arity(a, 0, false, name)?;
            Ok(Value::Object(s.store() as Arc<dyn Object>))
        }
        "Taxonomies" => {
            arity(a, 0, false, name)?;
            Ok(nh_page::taxonomy::taxonomy_list_to_value(
                &crate::site::site_taxonomies(h, sh.idx),
            ))
        }
        "Title" => {
            arity(a, 0, false, name)?;
            Ok(Value::string(s.title()))
        }

        // ---- `*hugolib.Site` only (own and promoted methods) ----
        "Site" => {
            arity(a, 0, false, name)?;
            Ok(sh.site_ref().to_value())
        }
        "SitemapAbsURL" => {
            arity(a, 0, false, name)?;
            Ok(Value::string(s.sitemap_abs_url()))
        }
        "AbsPathify" => {
            arity(a, 1, false, name)?;
            Ok(Value::string(
                ps().paths.abs_pathify(&args::string(a, 0)?.to_str_lossy()),
            ))
        }
        "AbsURL" => {
            arity(a, 2, false, name)?;
            let in_ = args::string(a, 0)?;
            let add_language = args::bool(a, 1)?;
            Ok(Value::string(
                ps().abs_url(&in_.to_str_lossy(), add_language),
            ))
        }
        "RelURL" => {
            arity(a, 2, false, name)?;
            let in_ = args::string(a, 0)?;
            let add_language = args::bool(a, 1)?;
            Ok(Value::string(
                ps().rel_url(&in_.to_str_lossy(), add_language),
            ))
        }
        "GetBasePath" => {
            arity(a, 1, false, name)?;
            Ok(Value::string(ps().paths.get_base_path(args::bool(a, 0)?)))
        }
        "GetLanguagePrefix" => {
            arity(a, 0, false, name)?;
            Ok(Value::string(ps().paths.get_language_prefix()))
        }
        "GetTargetLanguageBasePath" => {
            arity(a, 0, false, name)?;
            Ok(Value::string(ps().paths.get_target_language_base_path()))
        }
        "IsAbsURL" => {
            arity(a, 1, false, name)?;
            let in_ = args::string(a, 0)?;
            ps().try_is_abs_url(&in_.to_str_lossy())
                .map(Value::Bool)
                .map_err(Into::into)
        }
        "Lang" => {
            arity(a, 0, false, name)?;
            Ok(Value::string(ps().paths.lang()))
        }
        "MakePath" => {
            arity(a, 1, false, name)?;
            Ok(Value::string(
                ps().make_path(&args::string(a, 0)?.to_str_lossy()),
            ))
        }
        "MakePathSanitized" => {
            arity(a, 1, false, name)?;
            Ok(Value::string(
                ps().make_path_sanitized(&args::string(a, 0)?.to_str_lossy()),
            ))
        }
        "PermalinkForBaseURL" => {
            arity(a, 2, false, name)?;
            let link = args::string(a, 0)?;
            let base = args::string(a, 1)?;
            Ok(Value::string(ps().permalink_for_base_url(
                &link.to_str_lossy(),
                &base.to_str_lossy(),
            )))
        }
        "PrependBasePath" => {
            arity(a, 2, false, name)?;
            let rel = args::string(a, 0)?;
            let is_abs = args::bool(a, 1)?;
            Ok(Value::string(
                ps().prepend_base_path(&rel.to_str_lossy(), is_abs),
            ))
        }
        "RelPathify" => {
            arity(a, 1, false, name)?;
            Ok(Value::string(
                ps().paths.rel_pathify(&args::string(a, 0)?.to_str_lossy()),
            ))
        }
        "ResolveMarkup" => {
            arity(a, 1, false, name)?;
            Ok(Value::string(
                s.deps
                    .content_spec()
                    .resolve_markup(&args::string(a, 0)?.to_str_lossy()),
            ))
        }
        "SanitizeAnchorName" => {
            arity(a, 1, false, name)?;
            Ok(Value::string(
                s.deps
                    .content_spec()
                    .sanitize_anchor_name(&args::string(a, 0)?.to_str_lossy()),
            ))
        }
        "URLEscape" => {
            arity(a, 1, false, name)?;
            Ok(Value::string(
                ps().url_escape(&args::string(a, 0)?.to_str_lossy()),
            ))
        }
        "URLize" => {
            arity(a, 1, false, name)?;
            Ok(Value::string(
                ps().urlize(&args::string(a, 0)?.to_str_lossy()),
            ))
        }
        "URLizeFilename" => {
            arity(a, 1, false, name)?;
            Ok(Value::string(
                ps().urlize_filename(&args::string(a, 0)?.to_str_lossy()),
            ))
        }
        "AbsProjectContentDir" => bad_results(a, 1, false, false, name, 3),
        "MakePathsSanitized" => bad_results(a, 1, false, false, name, 0),
        "RegisterMediaTypes" => bad_results(a, 0, false, false, name, 0),
        "SendError" => bad_results(a, 1, false, false, name, 0),
        "StatResource" => bad_results(a, 2, false, false, name, 3),
        "StopErrorCollector" => bad_results(a, 0, false, false, name, 0),
        // Build internals (dependencies, file systems, locks, error collectors, modules).
        "AllModules"
        | "Close"
        | "GetInternalRelatedDocsHandler"
        | "GetTemplateStore"
        | "Init"
        | "LockBuild"
        | "StartErrorCollector"
        | "WatchFilenames" => {
            arity(a, 0, false, name)?;
            Err(unsupported(recv, name))
        }
        "Compile"
        | "IsContent"
        | "IsStatic"
        | "MakeStaticPathRelative"
        | "MkdirTemp"
        | "ResolveJSConfigFile"
        | "ResolvePaths"
        | "StaticFs" => {
            arity(a, 1, false, name)?;
            Err(unsupported(recv, name))
        }
        "Clone" | "TrimShortHTML" => {
            arity(a, 2, false, name)?;
            Err(unsupported(recv, name))
        }
        _ => Err(unsupported(recv, name)),
    }
}

impl Object for HugolibSiteObject {
    fn type_name(&self) -> std::borrow::Cow<'_, str> {
        std::borrow::Cow::Borrowed("*hugolib.Site")
    }
    fn has_method(&self, name: &str) -> bool {
        HUGOLIB_SITE_METHODS.binary_search(&name).is_ok()
    }
    fn call_method(&self, ctx: HostCtx<'_>, name: &str, args: &[Value]) -> Option<GoResult<Value>> {
        if !self.has_method(name) {
            return None;
        }
        Some(call_site_method(&self.0, ctx, name, args, true))
    }
    fn identity(&self) -> usize {
        // Go: the `*Site` pointer.
        &self.0.h.sites[self.0.idx] as *const crate::site::Site as usize
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn method_lists_are_sorted() {
        assert!(SITE_WRAPPER_METHODS.windows(2).all(|w| w[0] < w[1]));
        assert!(HUGOLIB_SITE_METHODS.windows(2).all(|w| w[0] < w[1]));
    }
}
