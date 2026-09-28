//! Module `tplapi::site_methods`.
//!
//! NEW: template-visible method set of page.Site (*page.siteWrapper) and *hugolib.Site
//!
//! Owner: Wave B task T23 (hugolib-site).

//! The template-visible method set of `*page.siteWrapper` (Go `page.Site`) and `*hugolib.Site`.
//! Implement `nh_page::site::Site` for `crate::site::SiteHandle` here.
//!
//! Members seeksnack uses: AllPages, BaseURL, Config (.Services.RSS.Limit), Data, GetPage,
//! Home, Language, LanguageCode, Languages, Params (+ the `mainsections` special case lives in
//! the exec helper), RegularPages (+ `.Related`), Taxonomies, Title; `*hugolib.Site`: SitemapAbsURL,
//! Lastmod. Full Go interface: see resources/page/site.go.

use std::sync::Arc;

use go_value::{HostCtx, Map, Time, Value};
use nh_common::Result;
use nh_common::maps::scratch::Scratch;
use nh_common::object::GoResult;
use nh_config::neohugo::neohugo::HugoInfo;
use nh_langs::language::{Language, Languages};
use nh_page::navigation::menu::Menus;
use nh_page::page::{PageRef, Pages};
use nh_page::site::{Site, SiteConfig, SiteRef};
use nh_page::taxonomy::TaxonomyList;

use crate::site::SiteHandle;

impl Site for SiteHandle {
    fn site_index(&self) -> usize {
        self.idx
    }
    fn language(&self) -> Arc<Language> {
        self.site().language.clone()
    }
    fn languages(&self) -> Languages {
        todo!()
    }
    fn get_page(&self, refs: &[String]) -> Result<Option<PageRef>> {
        todo!()
    }
    fn all_pages(&self) -> Pages {
        self.h.all_pages()
    }
    fn regular_pages(&self) -> Pages {
        todo!()
    }
    fn pages(&self) -> Pages {
        todo!()
    }
    fn sections(&self) -> Pages {
        todo!()
    }
    fn home(&self) -> Option<PageRef> {
        todo!()
    }
    fn title(&self) -> String {
        todo!()
    }
    fn language_code(&self) -> String {
        todo!()
    }
    fn copyright(&self) -> String {
        todo!()
    }
    fn sites(&self) -> Vec<SiteRef> {
        todo!()
    }
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
    fn hugo(&self) -> HugoInfo {
        self.h.hugo_info.clone()
    }
    fn base_url(&self) -> String {
        todo!()
    }
    fn taxonomies(&self) -> TaxonomyList {
        todo!()
    }
    fn last_change(&self) -> Time {
        todo!()
    }
    fn lastmod(&self) -> Time {
        todo!()
    }
    fn menus(&self) -> Menus {
        todo!()
    }
    fn main_sections(&self) -> Vec<String> {
        todo!()
    }
    fn params(&self) -> Arc<Map> {
        todo!()
    }
    fn param(&self, key: &Value) -> Result<Value> {
        todo!()
    }
    fn data(&self) -> Arc<Map> {
        self.h.data()
    }
    fn config(&self) -> SiteConfig {
        todo!()
    }
    fn build_drafts(&self) -> bool {
        todo!()
    }
    fn is_multi_lingual(&self) -> bool {
        todo!()
    }
    fn language_prefix(&self) -> String {
        todo!()
    }
    fn store(&self) -> Arc<Scratch> {
        self.site().store.clone()
    }
    fn tpl_has_method(&self, name: &str) -> bool {
        todo!()
    }
    fn tpl_call_method(
        &self,
        ctx: HostCtx<'_>,
        name: &str,
        args: &[Value],
    ) -> Option<GoResult<Value>> {
        todo!()
    }
}
