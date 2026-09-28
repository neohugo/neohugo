//! Module `tplapi::page_methods`.
//!
//! NEW: template-visible method set of page.Page (replaces reflection on *pageState)
//!
//! Owner: Wave B task T23 (hugolib-site).

//! The template-visible method set of a page (Go reflection on `*pageState` / `pageWithWeight0` /
//! `*pageState` via `page.Page`). Implement `nh_resource::resourcetypes::Resource` and
//! `nh_page::page::Page` for `crate::page::PageHandle` here, with ONE `go_methods!` table.
//!
//! Members seeksnack uses (templates-inventory §3.1 / A.2, exact case): AllTranslations,
//! AlternativeOutputFormats, Content (ctx), Data, Date, Description, File, IsHome, IsNode, IsPage,
//! IsSection, IsTranslated, Kind, Lang, Language, Lastmod, OutputFormats, Page, Pages, Paginate,
//! Paginator, Params, Parent, Permalink, Plain (ctx), PublishDate, RegularPages, RelPermalink,
//! Resources, Scratch, Section, Site, Sitemap, Title, Translations, Type (+ via render hooks:
//! GetPage, Resources on `PageInner`; via shortcodes: Ref/RelRef). Also implement the rest of the
//! Go `page.Page` interface (Name, LinkTitle, Weight, ExpiryDate, Summary, Truncated, WordCount,
//! ReadingTime, Len, TableOfContents, Fragments, RenderString, Render, RawContent, Aliases, Draft,
//! Keywords, Layout, BundleType, Slug, Path, Param, GetTerms, HasShortcode, CurrentSection,
//! FirstSection, InSection, IsAncestor, IsDescendant, Ancestors, Sections, Next, Prev,
//! NextInSection, PrevInSection, Store, Sites, Eq, String, GitInfo, CodeOwners, Menus,
//! HasMenuCurrent, IsMenuCurrent, TranslationKey, ResourceType, MediaType, Name, Key...).
//! Go type names for `%T`: `*hugolib.pageState`, `hugolib.pageWithWeight0`, `*hugolib.pageWithOrdinal`,
//! `*hugolib.pageForShortcode`, `*hugolib.pageForRenderHooks` (see `crate::page::PageWrapper`).
//!
//! Method-set rules for the wrappers (HUGO_LAYER.md §5.1):
//! * A Go wrapper struct exposes the methods of its EMBEDDED INTERFACES plus its own methods,
//!   not the full `*pageState` method set. `pageForShortcode` / `pageForRenderHooks` =
//!   `page.PageWithoutContent` (delegating to the page) + `TableOfContentsProvider` (the page;
//!   `pageForShortcode` overrides `TableOfContents` with the TOC placeholder) + `MarkupProvider`
//!   and `ContentProvider` from `page.NopPage` (`.Content`, `.Plain`, `.Summary`, ... are the
//!   nop results) + `Unwrapv`/`String`. `pageWithWeight0` / `pageWithOrdinal` embed `page.Page`
//!   (+ `Weight0()` / `Ordinal()`). Build the table per wrapper, not by filtering `*pageState`.
//! * A struct field shadows a promoted method of the same name at a deeper level (Go selector
//!   depth rule), e.g. `hugolib.aliasPage{Permalink string; page.Page}`: `.Permalink` is the field.
//!
//! Nil results (HUGO_LAYER.md §4.7): `Parent` (and other methods declared to return the
//! `page.Page` interface) return `TypedNil("page.Page")` when Go returns a nil interface — it
//! prints `<nil>` in text templates and becomes `Invalid` when passed to a func or stored
//! (Scratch/dict), so `jsonLd.html:389` (`.Parent` stored in a Scratch, then `.Parent` on the
//! `Get` result) yields nothing, as in Go. `GetPage` of a missing ref returns the NopPage OBJECT
//! (Go `page.NilPage` = `(*nopPage)(nil)`, a non-nil interface holding a nil pointer: methods
//! work, `IsZero` is true, prints `<nil>`), never `Invalid`. `Sitemap` returns
//! `Value::object(SitemapConfig)` (nh-config implements `Object` for it).

use std::any::Any;
use std::borrow::Cow;
use std::sync::Arc;

use go_value::{GoString, HostCtx, Map, Time, Value};
use nh_common::Result;
use nh_common::hugio::ReadSeekCloser;
use nh_common::object::GoResult;
use nh_common::paths::pathparser::Path;
use nh_config::common_config::SitemapConfig;
use nh_helpers::source::file_info::File;
use nh_langs::language::Language;
use nh_media::media::media_type::MediaType;
use nh_page::page::{Page, PageRef, Pages};
use nh_page::page_outputformat::OutputFormats;
use nh_page::related::{IndexConfig, Keyword};
use nh_page::site::SiteRef;
use nh_resource::resourcetypes::{Resource, Resources};

use crate::page::{PageHandle, PageWrapper};

impl Resource for PageHandle {
    fn resource_type(&self) -> String {
        "page".to_string()
    }
    fn media_type(&self) -> MediaType {
        todo!()
    }
    fn permalink(&self) -> String {
        todo!()
    }
    fn rel_permalink(&self) -> String {
        todo!()
    }
    fn data(&self) -> Value {
        todo!()
    }
    fn name(&self) -> String {
        todo!()
    }
    fn title(&self) -> String {
        todo!()
    }
    fn params(&self) -> Arc<Map> {
        todo!()
    }
    fn key(&self) -> String {
        todo!()
    }
    fn content(&self, ctx: HostCtx<'_>) -> Option<Result<Value>> {
        todo!()
    }
    fn language(&self) -> Option<Arc<Language>> {
        todo!()
    }
    fn translation_key(&self) -> Option<String> {
        todo!()
    }

    fn tpl_type_name(&self) -> Cow<'_, str> {
        Cow::Borrowed(match self.wrapper {
            PageWrapper::None => "*hugolib.pageState",
            PageWrapper::Weight0(_) => "hugolib.pageWithWeight0",
            PageWrapper::Ordinal(_) => "*hugolib.pageWithOrdinal",
            PageWrapper::ForShortcode => "*hugolib.pageForShortcode",
            PageWrapper::ForRenderHooks => "*hugolib.pageForRenderHooks",
        })
    }
    fn tpl_has_method(&self, name: &str) -> bool {
        todo!("PageHandle::go_has_method(name) once the table exists")
    }
    fn tpl_call_method(
        &self,
        ctx: HostCtx<'_>,
        name: &str,
        args: &[Value],
    ) -> Option<GoResult<Value>> {
        todo!()
    }
    fn tpl_is_zero(&self) -> Option<bool> {
        None
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

impl Page for PageHandle {
    fn page_id(&self) -> u64 {
        self.state().pid
    }
    fn unwrap_page(self: Arc<Self>) -> Arc<dyn Page> {
        Arc::new(PageHandle {
            h: self.h.clone(),
            id: self.id,
            wrapper: PageWrapper::None,
        })
    }
    fn weight0(&self) -> Option<i64> {
        match self.wrapper {
            PageWrapper::Weight0(w) => Some(w),
            _ => None,
        }
    }
    fn ordinal(&self) -> Option<i64> {
        match self.wrapper {
            PageWrapper::Ordinal(o) => Some(o),
            _ => None,
        }
    }
    fn kind(&self) -> String {
        self.state().meta.kind().to_string()
    }
    fn title(&self) -> String {
        self.state().meta.title().to_string()
    }
    fn link_title(&self) -> String {
        self.state().meta.link_title().to_string()
    }
    fn description(&self) -> String {
        todo!()
    }
    fn weight(&self) -> i64 {
        todo!()
    }
    fn date(&self) -> Time {
        self.state().meta.date()
    }
    fn lastmod(&self) -> Time {
        todo!()
    }
    fn publish_date(&self) -> Time {
        todo!()
    }
    fn expiry_date(&self) -> Time {
        todo!()
    }
    fn is_home(&self) -> bool {
        todo!()
    }
    fn is_node(&self) -> bool {
        self.state().meta.is_node()
    }
    fn is_page(&self) -> bool {
        todo!()
    }
    fn is_section(&self) -> bool {
        todo!()
    }
    fn section(&self) -> String {
        todo!()
    }
    fn page_type(&self) -> String {
        self.state().meta.page_type()
    }
    fn layout(&self) -> String {
        todo!()
    }
    fn lang(&self) -> String {
        todo!()
    }
    fn path(&self) -> String {
        todo!()
    }
    fn path_info(&self) -> Arc<Path> {
        self.state().meta.path_info.clone()
    }
    fn slug(&self) -> String {
        todo!()
    }
    fn draft(&self) -> bool {
        todo!()
    }
    fn aliases(&self) -> Vec<String> {
        todo!()
    }
    fn keywords(&self) -> Vec<String> {
        todo!()
    }
    fn bundle_type(&self) -> String {
        todo!()
    }
    fn sitemap(&self) -> SitemapConfig {
        todo!()
    }
    fn param(&self, key: &Value) -> Result<Value> {
        todo!()
    }
    fn page_params(&self) -> Arc<Map> {
        todo!()
    }
    fn site(&self) -> SiteRef {
        todo!()
    }
    fn file(&self) -> Option<Arc<File>> {
        todo!()
    }
    fn parent(&self) -> Option<PageRef> {
        crate::page__tree::parent(self)
    }
    fn pages(&self) -> Pages {
        todo!()
    }
    fn regular_pages(&self) -> Pages {
        todo!()
    }
    fn resources(&self) -> Resources {
        todo!()
    }
    fn output_formats(&self) -> OutputFormats {
        todo!()
    }
    fn all_translations(&self) -> Pages {
        todo!()
    }
    fn translations(&self) -> Pages {
        todo!()
    }
    fn plain(&self, ctx: HostCtx<'_>) -> Result<GoString> {
        todo!()
    }
    fn content_len(&self, ctx: HostCtx<'_>) -> Result<i64> {
        todo!()
    }
    fn render_string(&self, ctx: HostCtx<'_>, args: &[Value]) -> Result<Value> {
        todo!()
    }
    fn related_keywords(&self, cfg: &IndexConfig) -> Result<Vec<Keyword>> {
        todo!()
    }
    fn ref_(&self, args: &Map) -> Result<String> {
        todo!()
    }
    fn rel_ref(&self, args: &Map) -> Result<String> {
        todo!()
    }
}
