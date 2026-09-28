//! Port of `resources/page/page_nop.go`.
//!
//! Owner: Wave B task T11 (page-api-paths).
//!
//! Go `page.NopPage` (`new(nopPage)`) and `page.NilPage` (`(*nopPage)(nil)`, returned by
//! `.GetPage` misses: a non-nil `page.Page` holding a nil pointer; falsy, every method works and
//! returns its zero value). [`NopPage`] is both (`nil` tells them apart) and implements
//! `Resource` + `Page`; its template method set is Go's `*nopPage` method set.
//! `NopMarkup`/`NopContent`/`NopContentRenderer` are the other nop values of the file.

use std::any::Any;
use std::borrow::Cow;
use std::sync::Arc;

use go_value::{GoString, HostCtx, Map, MapType, Object, SafeKind, Time, Value};
use nh_common::Result;
use nh_common::object::{GoResult, args};
use nh_common::paths::pathparser::Path;
use nh_config::common_config::SitemapConfig;
use nh_helpers::source::file_info::File;
use nh_media::media::media_type::MediaType;
use nh_resource::resourcetypes::{Resource, Resources};

use crate::page::{Page, PageRef, Pages};
use crate::page_outputformat::OutputFormats;
use crate::related::{IndexConfig, Keyword};
use crate::site::SiteRef;

/// Go type string of the nop page.
pub const NOP_PAGE_TYPE: &str = "*page.nopPage";

/// Go: `*page.nopPage`. `nil == true` is `page.NilPage`.
#[derive(Clone, Copy, Debug, Default)]
pub struct NopPage {
    pub nil: bool,
}

/// Go: `page.NopPage` (a non-nil `*nopPage`).
pub fn nop_page() -> Arc<NopPage> {
    Arc::new(NopPage { nil: false })
}

/// Go: `page.NilPage` (a nil `*nopPage`).
pub fn nil_page() -> Arc<NopPage> {
    Arc::new(NopPage { nil: true })
}

/// The template value of `page.NilPage` (what `.GetPage` returns for a missing page).
pub fn nil_page_value() -> Value {
    PageRef(nil_page()).to_value()
}

fn nil_of(t: &str) -> Value {
    Value::TypedNil(Arc::from(t))
}

fn html_empty() -> Value {
    Value::Safe(SafeKind::Html, GoString::empty())
}

fn zero_time() -> Value {
    Value::Time(Time::zero())
}

/// `exactly(n)` then the value.
fn ret(a: &[Value], n: usize, name: &str, v: Value) -> GoResult<Value> {
    args::exactly(a, n, name)?;
    Ok(v)
}

nh_common::go_methods!(NopPage {
    "Aliases" => |_p, _c, a| ret(a, 0, "Aliases", nil_of("[]string")),
    "Sitemap" => |_p, _c, a| ret(a, 0, "Sitemap", Value::object(SitemapConfig::default())),
    "Layout" => |_p, _c, a| ret(a, 0, "Layout", Value::string("")),
    "AllTranslations" => |_p, _c, a| ret(a, 0, "AllTranslations", nil_of("page.Pages")),
    "LanguagePrefix" => |_p, _c, a| ret(a, 0, "LanguagePrefix", Value::string("")),
    "AlternativeOutputFormats" => |_p, _c, a| ret(a, 0, "AlternativeOutputFormats", nil_of("page.OutputFormats")),
    "BaseFileName" => |_p, _c, a| ret(a, 0, "BaseFileName", Value::string("")),
    "BundleType" => |_p, _c, a| ret(a, 0, "BundleType", Value::string("")),
    "Markup" => |_p, _c, _a| Ok(Value::object(NopMarkup)),
    "Content" => |_p, _c, a| ret(a, 0, "Content", Value::string("")),
    "ContentWithoutSummary" => |_p, _c, a| ret(a, 0, "ContentWithoutSummary", html_empty()),
    "ContentBaseName" => |_p, _c, a| ret(a, 0, "ContentBaseName", Value::string("")),
    "CurrentSection" => |_p, _c, a| ret(a, 0, "CurrentSection", nil_of("page.Page")),
    "Data" => |_p, _c, a| ret(a, 0, "Data", Value::Invalid),
    "Date" => |_p, _c, a| ret(a, 0, "Date", zero_time()),
    "Description" => |_p, _c, a| ret(a, 0, "Description", Value::string("")),
    "RefFrom" => |_p, _c, a| ret(a, 2, "RefFrom", Value::string("")),
    "RelRefFrom" => |_p, _c, a| ret(a, 2, "RelRefFrom", Value::string("")),
    "Dir" => |_p, _c, a| ret(a, 0, "Dir", Value::string("")),
    "Draft" => |_p, _c, a| ret(a, 0, "Draft", Value::Bool(false)),
    "Eq" => |p, _c, a| {
        args::exactly(a, 1, "Eq")?;
        // Go: `p == other` (pointer equality: the same nop page).
        let same = a[0].downcast::<PageRef>().is_some_and(|o| {
            o.0.as_any().downcast_ref::<NopPage>().is_some_and(|n| n.nil == p.nil)
        });
        Ok(Value::Bool(same))
    },
    "ExpiryDate" => |_p, _c, a| ret(a, 0, "ExpiryDate", zero_time()),
    "File" => |_p, _c, a| ret(a, 0, "File", nil_of("*source.File")),
    "FileInfo" => |_p, _c, a| ret(a, 0, "FileInfo", nil_of("hugofs.FileMetaInfo")),
    "Filename" => |_p, _c, a| ret(a, 0, "Filename", Value::string("")),
    "FirstSection" => |_p, _c, a| ret(a, 0, "FirstSection", nil_of("page.Page")),
    "FuzzyWordCount" => |_p, _c, a| ret(a, 0, "FuzzyWordCount", Value::int(0)),
    "GetPage" => |_p, _c, a| {
        args::exactly(a, 1, "GetPage")?;
        args::string(a, 0)?;
        Ok(nil_of("page.Page"))
    },
    "GetParam" => |_p, _c, a| {
        args::exactly(a, 1, "GetParam")?;
        args::string(a, 0)?;
        Ok(Value::Invalid)
    },
    "GetTerms" => |_p, _c, a| {
        args::exactly(a, 1, "GetTerms")?;
        args::string(a, 0)?;
        Ok(nil_of("page.Pages"))
    },
    "GitInfo" => |_p, _c, a| ret(a, 0, "GitInfo", nil_of("*source.GitInfo")),
    "CodeOwners" => |_p, _c, a| ret(a, 0, "CodeOwners", nil_of("[]string")),
    "HasMenuCurrent" => |_p, _c, a| ret(a, 2, "HasMenuCurrent", Value::Bool(false)),
    "HasShortcode" => |_p, _c, a| {
        args::exactly(a, 1, "HasShortcode")?;
        args::string(a, 0)?;
        Ok(Value::Bool(false))
    },
    "Hugo" => |_p, _c, a| {
        args::exactly(a, 0, "Hugo")?;
        Err(go_value::Error::new(
            "neohugo-rs: Hugo on a nop page (a zero neohugo.HugoInfo) is not supported",
        ))
    },
    "InSection" => |_p, _c, a| ret(a, 1, "InSection", Value::Bool(false)),
    "IsAncestor" => |_p, _c, a| ret(a, 1, "IsAncestor", Value::Bool(false)),
    "IsDescendant" => |_p, _c, a| ret(a, 1, "IsDescendant", Value::Bool(false)),
    "IsDraft" => |_p, _c, a| ret(a, 0, "IsDraft", Value::Bool(false)),
    "IsHome" => |_p, _c, a| ret(a, 0, "IsHome", Value::Bool(false)),
    "IsMenuCurrent" => |_p, _c, a| ret(a, 2, "IsMenuCurrent", Value::Bool(false)),
    "IsNode" => |_p, _c, a| ret(a, 0, "IsNode", Value::Bool(false)),
    "IsPage" => |_p, _c, a| ret(a, 0, "IsPage", Value::Bool(false)),
    "IsSection" => |_p, _c, a| ret(a, 0, "IsSection", Value::Bool(false)),
    "IsTranslated" => |_p, _c, a| ret(a, 0, "IsTranslated", Value::Bool(false)),
    "Keywords" => |_p, _c, a| ret(a, 0, "Keywords", nil_of("[]string")),
    "Kind" => |_p, _c, a| ret(a, 0, "Kind", Value::string("")),
    "Lang" => |_p, _c, a| ret(a, 0, "Lang", Value::string("")),
    "Language" => |_p, _c, a| ret(a, 0, "Language", nil_of("*langs.Language")),
    "Lastmod" => |_p, _c, a| ret(a, 0, "Lastmod", zero_time()),
    "Len" => |_p, _c, a| ret(a, 0, "Len", Value::int(0)),
    "LinkTitle" => |_p, _c, a| ret(a, 0, "LinkTitle", Value::string("")),
    "LogicalName" => |_p, _c, a| ret(a, 0, "LogicalName", Value::string("")),
    "MediaType" => |_p, _c, a| ret(a, 0, "MediaType", MediaType::default().to_value()),
    "Menus" => |_p, _c, a| ret(a, 0, "Menus", nil_of("navigation.PageMenus")),
    "Name" => |_p, _c, a| ret(a, 0, "Name", Value::string("")),
    "Next" => |_p, _c, a| ret(a, 0, "Next", nil_of("page.Page")),
    "OutputFormats" => |_p, _c, a| ret(a, 0, "OutputFormats", nil_of("page.OutputFormats")),
    "Pages" => |_p, _c, a| ret(a, 0, "Pages", nil_of("page.Pages")),
    "RegularPages" => |_p, _c, a| ret(a, 0, "RegularPages", nil_of("page.Pages")),
    "RegularPagesRecursive" => |_p, _c, a| ret(a, 0, "RegularPagesRecursive", nil_of("page.Pages")),
    "Paginate" => |_p, _c, a| {
        args::at_least(a, 1, "Paginate")?;
        Ok(nil_of("*page.Pager"))
    },
    "Paginator" => |_p, _c, _a| Ok(nil_of("*page.Pager")),
    "Param" => |_p, _c, a| ret(a, 1, "Param", Value::Invalid),
    "Params" => |_p, _c, a| ret(a, 0, "Params", nil_of("maps.Params")),
    "Page" => |p, _c, a| ret(a, 0, "Page", PageRef(Arc::new(*p)).to_value()),
    "Parent" => |_p, _c, a| ret(a, 0, "Parent", nil_of("page.Page")),
    "Ancestors" => |_p, _c, a| ret(a, 0, "Ancestors", nil_of("page.Pages")),
    "Path" => |_p, _c, a| ret(a, 0, "Path", Value::string("")),
    "PathInfo" => |_p, _c, a| ret(a, 0, "PathInfo", nil_of("*paths.Path")),
    "Permalink" => |_p, _c, a| ret(a, 0, "Permalink", Value::string("")),
    "Plain" => |_p, _c, a| ret(a, 0, "Plain", Value::string("")),
    "PlainWords" => |_p, _c, a| ret(a, 0, "PlainWords", nil_of("[]string")),
    "Prev" => |_p, _c, a| ret(a, 0, "Prev", nil_of("page.Page")),
    "PublishDate" => |_p, _c, a| ret(a, 0, "PublishDate", zero_time()),
    "PrevInSection" => |_p, _c, a| ret(a, 0, "PrevInSection", nil_of("page.Page")),
    "NextInSection" => |_p, _c, a| ret(a, 0, "NextInSection", nil_of("page.Page")),
    "PrevPage" => |_p, _c, a| ret(a, 0, "PrevPage", nil_of("page.Page")),
    "NextPage" => |_p, _c, a| ret(a, 0, "NextPage", nil_of("page.Page")),
    "RawContent" => |_p, _c, a| ret(a, 0, "RawContent", Value::string("")),
    "RenderShortcodes" => |_p, _c, a| ret(a, 0, "RenderShortcodes", html_empty()),
    "ReadingTime" => |_p, _c, a| ret(a, 0, "ReadingTime", Value::int(0)),
    "Ref" => |_p, _c, a| ret(a, 1, "Ref", Value::string("")),
    "RelPermalink" => |_p, _c, a| ret(a, 0, "RelPermalink", Value::string("")),
    "RelRef" => |_p, _c, a| ret(a, 1, "RelRef", Value::string("")),
    "Render" => |_p, _c, _a| Ok(html_empty()),
    "RenderString" => |_p, _c, _a| Ok(html_empty()),
    "ResourceType" => |_p, _c, a| ret(a, 0, "ResourceType", Value::string("")),
    "Resources" => |_p, _c, a| ret(a, 0, "Resources", nil_of("resource.Resources")),
    "Scratch" => |_p, _c, a| ret(a, 0, "Scratch", nil_of("*maps.Scratch")),
    "Store" => |_p, _c, a| ret(a, 0, "Store", nil_of("*maps.Scratch")),
    "RelatedKeywords" => |_p, _c, a| ret(a, 1, "RelatedKeywords", nil_of("[]related.Keyword")),
    "Section" => |_p, _c, a| ret(a, 0, "Section", Value::string("")),
    "Sections" => |_p, _c, a| ret(a, 0, "Sections", nil_of("page.Pages")),
    "SectionsEntries" => |_p, _c, a| ret(a, 0, "SectionsEntries", nil_of("[]string")),
    "SectionsPath" => |_p, _c, a| ret(a, 0, "SectionsPath", Value::string("")),
    "Site" => |_p, _c, a| ret(a, 0, "Site", nil_of("page.Site")),
    "Sites" => |_p, _c, a| ret(a, 0, "Sites", nil_of("page.Sites")),
    "Slug" => |_p, _c, a| ret(a, 0, "Slug", Value::string("")),
    "String" => |_p, _c, a| ret(a, 0, "String", Value::string("nopPage")),
    "Summary" => |_p, _c, a| ret(a, 0, "Summary", html_empty()),
    "TableOfContents" => |_p, _c, a| ret(a, 0, "TableOfContents", html_empty()),
    "Title" => |_p, _c, a| ret(a, 0, "Title", Value::string("")),
    "TranslationBaseName" => |_p, _c, a| ret(a, 0, "TranslationBaseName", Value::string("")),
    "TranslationKey" => |_p, _c, a| ret(a, 0, "TranslationKey", Value::string("")),
    "Translations" => |_p, _c, a| ret(a, 0, "Translations", nil_of("page.Pages")),
    "Truncated" => |_p, _c, a| ret(a, 0, "Truncated", Value::Bool(false)),
    "Type" => |_p, _c, a| ret(a, 0, "Type", Value::string("")),
    "URL" => |_p, _c, a| ret(a, 0, "URL", Value::string("")),
    "UniqueID" => |_p, _c, a| ret(a, 0, "UniqueID", Value::string("")),
    "Weight" => |_p, _c, a| ret(a, 0, "Weight", Value::int(0)),
    "WordCount" => |_p, _c, a| ret(a, 0, "WordCount", Value::int(0)),
    "Fragments" => |_p, _c, a| ret(a, 0, "Fragments", nil_of("*tableofcontents.Fragments")),
    "HeadingsFiltered" => |_p, _c, a| ret(a, 0, "HeadingsFiltered", nil_of("tableofcontents.Headings")),
});

impl Resource for NopPage {
    // Go: resources/page/page_nop.go:ResourceType
    fn resource_type(&self) -> String {
        String::new()
    }
    // Go: resources/page/page_nop.go:MediaType
    fn media_type(&self) -> MediaType {
        MediaType::default()
    }
    // Go: resources/page/page_nop.go:Permalink
    fn permalink(&self) -> String {
        String::new()
    }
    // Go: resources/page/page_nop.go:RelPermalink
    fn rel_permalink(&self) -> String {
        String::new()
    }
    // Go: resources/page/page_nop.go:Data
    fn data(&self) -> Value {
        Value::Invalid
    }
    // Go: resources/page/page_nop.go:Name
    fn name(&self) -> String {
        String::new()
    }
    // Go: resources/page/page_nop.go:Title
    fn title(&self) -> String {
        String::new()
    }
    // Go: resources/page/page_nop.go:Params (a nil maps.Params)
    fn params(&self) -> Arc<Map> {
        Arc::new(Map::new(MapType::Params))
    }
    fn key(&self) -> String {
        String::new()
    }
    // Go: resources/page/page_nop.go:TranslationKey
    fn translation_key(&self) -> Option<String> {
        Some(String::new())
    }
    fn tpl_type_name(&self) -> Cow<'_, str> {
        Cow::Borrowed(NOP_PAGE_TYPE)
    }
    fn tpl_has_method(&self, name: &str) -> bool {
        NopPage::go_has_method(name)
    }
    fn tpl_call_method(
        &self,
        ctx: HostCtx<'_>,
        name: &str,
        args: &[Value],
    ) -> Option<GoResult<Value>> {
        self.go_call_method(ctx, name, args)
    }
    // Go: resources/page/page_nop.go:String
    fn tpl_go_string(&self) -> Option<GoString> {
        if self.nil {
            None
        } else {
            Some(GoString::from("nopPage"))
        }
    }
    fn tpl_is_zero(&self) -> Option<bool> {
        Some(self.nil)
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

impl Page for NopPage {
    fn page_id(&self) -> u64 {
        // Distinct from every real page id (nh-hugolib numbers pages from 0).
        if self.nil { u64::MAX } else { u64::MAX - 1 }
    }
    fn unwrap_page(self: Arc<Self>) -> Arc<dyn Page> {
        self
    }
    fn kind(&self) -> String {
        String::new()
    }
    fn title(&self) -> String {
        String::new()
    }
    fn link_title(&self) -> String {
        String::new()
    }
    fn description(&self) -> String {
        String::new()
    }
    fn weight(&self) -> i64 {
        0
    }
    fn date(&self) -> Time {
        Time::zero()
    }
    fn lastmod(&self) -> Time {
        Time::zero()
    }
    fn publish_date(&self) -> Time {
        Time::zero()
    }
    fn expiry_date(&self) -> Time {
        Time::zero()
    }
    fn is_home(&self) -> bool {
        false
    }
    fn is_node(&self) -> bool {
        false
    }
    fn is_page(&self) -> bool {
        false
    }
    fn is_section(&self) -> bool {
        false
    }
    fn section(&self) -> String {
        String::new()
    }
    fn page_type(&self) -> String {
        String::new()
    }
    fn layout(&self) -> String {
        String::new()
    }
    fn lang(&self) -> String {
        String::new()
    }
    fn path(&self) -> String {
        String::new()
    }
    /// Go returns a nil `*paths.Path`; callers that dereference it panic in Go too.
    fn path_info(&self) -> Arc<Path> {
        panic!(
            "runtime error: invalid memory address or nil pointer dereference (nopPage.PathInfo)"
        )
    }
    fn slug(&self) -> String {
        String::new()
    }
    fn draft(&self) -> bool {
        false
    }
    fn aliases(&self) -> Vec<String> {
        Vec::new()
    }
    fn keywords(&self) -> Vec<String> {
        Vec::new()
    }
    fn bundle_type(&self) -> String {
        String::new()
    }
    fn sitemap(&self) -> SitemapConfig {
        SitemapConfig::default()
    }
    fn param(&self, _key: &Value) -> Result<Value> {
        Ok(Value::Invalid)
    }
    fn page_params(&self) -> Arc<Map> {
        Arc::new(Map::new(MapType::Params))
    }
    /// Go returns a nil `page.Site`; callers that use it panic in Go too.
    fn site(&self) -> SiteRef {
        panic!("runtime error: invalid memory address or nil pointer dereference (nopPage.Site)")
    }
    fn file(&self) -> Option<Arc<File>> {
        None
    }
    fn parent(&self) -> Option<PageRef> {
        None
    }
    fn pages(&self) -> Pages {
        Vec::new()
    }
    fn regular_pages(&self) -> Pages {
        Vec::new()
    }
    fn resources(&self) -> Resources {
        Vec::new()
    }
    fn output_formats(&self) -> OutputFormats {
        Vec::new()
    }
    fn all_translations(&self) -> Pages {
        Vec::new()
    }
    fn translations(&self) -> Pages {
        Vec::new()
    }
    fn current_section(&self) -> Option<PageRef> {
        None
    }
    fn sections_entries(&self) -> Vec<String> {
        Vec::new()
    }
    fn sections_path(&self) -> String {
        String::new()
    }
    fn plain(&self, _ctx: HostCtx<'_>) -> Result<GoString> {
        Ok(GoString::empty())
    }
    fn content_len(&self, _ctx: HostCtx<'_>) -> Result<i64> {
        Ok(0)
    }
    fn render_string(&self, _ctx: HostCtx<'_>, _args: &[Value]) -> Result<Value> {
        Ok(html_empty())
    }
    fn related_keywords(&self, _cfg: &IndexConfig) -> Result<Vec<Keyword>> {
        Ok(Vec::new())
    }
    fn ref_(&self, _args: &Map) -> Result<String> {
        Ok(String::new())
    }
    fn rel_ref(&self, _args: &Map) -> Result<String> {
        Ok(String::new())
    }
    fn page_string(&self) -> String {
        "nopPage".to_string()
    }
}

/// Go: `page.NopMarkup` (`*page.nopMarkup`).
#[derive(Clone, Copy, Debug, Default)]
pub struct NopMarkup;

nh_common::go_methods!(NopMarkup {
    // Go: resources/page/page_nop.go:Render
    "Render" => |_m, _c, a| ret(a, 0, "Render", Value::object(NopContent)),
    // Go: resources/page/page_nop.go:RenderString
    "RenderString" => |_m, _c, _a| Ok(html_empty()),
    // Go: resources/page/page_nop.go:RenderShortcodes
    "RenderShortcodes" => |_m, _c, a| ret(a, 0, "RenderShortcodes", html_empty()),
    // Go: resources/page/page_nop.go:Fragments
    "Fragments" => |_m, _c, a| ret(a, 0, "Fragments", nil_of("*tableofcontents.Fragments")),
    // Go: resources/page/page_nop.go:FragmentsHTML
    "FragmentsHTML" => |_m, _c, a| ret(a, 0, "FragmentsHTML", html_empty()),
});

impl Object for NopMarkup {
    nh_common::object_basics!("*page.nopMarkup");
}

/// Go: `page.NopContent` (`*page.nopContent`).
#[derive(Clone, Copy, Debug, Default)]
pub struct NopContent;

nh_common::go_methods!(NopContent {
    // Go: resources/page/page_nop.go:Plain
    "Plain" => |_m, _c, a| ret(a, 0, "Plain", Value::string("")),
    // Go: resources/page/page_nop.go:PlainWords
    "PlainWords" => |_m, _c, a| ret(a, 0, "PlainWords", nil_of("[]string")),
    // Go: resources/page/page_nop.go:WordCount
    "WordCount" => |_m, _c, a| ret(a, 0, "WordCount", Value::int(0)),
    // Go: resources/page/page_nop.go:FuzzyWordCount
    "FuzzyWordCount" => |_m, _c, a| ret(a, 0, "FuzzyWordCount", Value::int(0)),
    // Go: resources/page/page_nop.go:ReadingTime
    "ReadingTime" => |_m, _c, a| ret(a, 0, "ReadingTime", Value::int(0)),
    // Go: resources/page/page_nop.go:Len
    "Len" => |_m, _c, a| ret(a, 0, "Len", Value::int(0)),
    // Go: resources/page/page_nop.go:Content
    "Content" => |_m, _c, a| ret(a, 0, "Content", html_empty()),
    // Go: resources/page/page_nop.go:ContentWithoutSummary
    "ContentWithoutSummary" => |_m, _c, a| ret(a, 0, "ContentWithoutSummary", html_empty()),
    // Go: resources/page/page_nop.go:Summary
    "Summary" => |_m, _c, a| ret(a, 0, "Summary", Value::object(crate::page_markup::Summary::default())),
});

impl Object for NopContent {
    nh_common::object_basics!("*page.nopContent");
}

/// Go: `page.NopContentRenderer` (`*page.nopContentRenderer`): renders nothing.
#[derive(Clone, Copy, Debug, Default)]
pub struct NopContentRenderer;

impl NopContentRenderer {
    /// Go: `ParseAndRenderContent` — an empty result.
    // Go: resources/page/page_nop.go:ParseAndRenderContent
    pub fn parse_and_render_content(&self, _content: &[u8], _render_toc: bool) -> Result<Vec<u8>> {
        Ok(Vec::new())
    }

    /// Go: `ParseContent` — `(nil, false, nil)`.
    // Go: resources/page/page_nop.go:ParseContent
    pub fn parse_content(&self, _content: &[u8]) -> Result<bool> {
        Ok(false)
    }

    /// Go: `RenderContent` — `(nil, false, nil)`.
    // Go: resources/page/page_nop.go:RenderContent
    pub fn render_content(&self, _content: &[u8]) -> Result<bool> {
        Ok(false)
    }
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: resources/page/page_nop.go (592 lines; 0/131 funcs executed)
//   types: nopPage, nopContentRenderer, (group)
// Every *nopPage method below is an entry of the `go_methods!` table of `NopPage` (and, where
// the Page/Resource traits have it, a trait method); Hugo() is an explicit unsupported error.
// OK L57-59: (p *nopPage) Aliases() []string
// OK L61-63: (p *nopPage) Sitemap() config.SitemapConfig
// OK L65-67: (p *nopPage) Layout() string
// OK L69-71: (p *nopPage) AllTranslations() Pages
// OK L73-75: (p *nopPage) LanguagePrefix() string
// OK L77-79: (p *nopPage) AlternativeOutputFormats() OutputFormats
// OK L81-83: (p *nopPage) BaseFileName() string
// OK L85-87: (p *nopPage) BundleType() string
// OK L89-91: (p *nopPage) Markup(...any) Markup
// OK L93-95: (p *nopPage) Content(context.Context) (any, error)
// OK L97-99: (p *nopPage) ContentWithoutSummary(ctx context.Context) (template.HTML, error)
// OK L101-103: (p *nopPage) ContentBaseName() string
// OK L105-107: (p *nopPage) CurrentSection() Page
// OK L109-111: (p *nopPage) Data() any
// OK L113-115: (p *nopPage) Date() (t time.Time)
// OK L117-119: (p *nopPage) Description() string
// OK L121-123: (p *nopPage) RefFrom(argsm map[string]any, source any) (string, error)
// OK L125-127: (p *nopPage) RelRefFrom(argsm map[string]any, source any) (string, error)
// OK L129-131: (p *nopPage) Dir() string
// OK L133-135: (p *nopPage) Draft() bool
// OK L137-139: (p *nopPage) Eq(other any) bool
// OK L141-143: (p *nopPage) ExpiryDate() (t time.Time)
// OK L145-147: (p *nopPage) File() *source.File
// OK L149-151: (p *nopPage) FileInfo() hugofs.FileMetaInfo
// OK L153-155: (p *nopPage) Filename() string
// OK L157-159: (p *nopPage) FirstSection() Page
// OK L161-163: (p *nopPage) FuzzyWordCount(context.Context) int
// OK L165-167: (p *nopPage) GetPage(ref string) (Page, error)
// OK L169-171: (p *nopPage) GetParam(key string) any
// OK L173-175: (p *nopPage) GetTerms(taxonomy string) Pages
// OK L177-179: (p *nopPage) GitInfo() *source.GitInfo
// OK L181-183: (p *nopPage) CodeOwners() []string
// OK L185-187: (p *nopPage) HasMenuCurrent(menuID string, me *navigation.MenuEntry) bool
// OK L189-191: (p *nopPage) HasShortcode(name string) bool
// OK L193-195: (p *nopPage) Hugo() (h neohugo.HugoInfo)
// OK L197-199: (p *nopPage) InSection(other any) bool
// OK L201-203: (p *nopPage) IsAncestor(other any) bool
// OK L205-207: (p *nopPage) IsDescendant(other any) bool
// OK L209-211: (p *nopPage) IsDraft() bool
// OK L213-215: (p *nopPage) IsHome() bool
// OK L217-219: (p *nopPage) IsMenuCurrent(menuID string, inme *navigation.MenuEntry) bool
// OK L221-223: (p *nopPage) IsNode() bool
// OK L225-227: (p *nopPage) IsPage() bool
// OK L229-231: (p *nopPage) IsSection() bool
// OK L233-235: (p *nopPage) IsTranslated() bool
// OK L237-239: (p *nopPage) Keywords() []string
// OK L241-243: (p *nopPage) Kind() string
// OK L245-247: (p *nopPage) Lang() string
// OK L249-251: (p *nopPage) Language() *langs.Language
// OK L253-255: (p *nopPage) Lastmod() (t time.Time)
// OK L257-259: (p *nopPage) Len(context.Context) int
// OK L261-263: (p *nopPage) LinkTitle() string
// OK L265-267: (p *nopPage) LogicalName() string
// OK L269-271: (p *nopPage) MediaType() (m media.Type)
// OK L273-275: (p *nopPage) Menus() (m navigation.PageMenus)
// OK L277-279: (p *nopPage) Name() string
// OK L281-283: (p *nopPage) Next() Page
// OK L285-287: (p *nopPage) OutputFormats() OutputFormats
// OK L289-291: (p *nopPage) Pages() Pages
// OK L293-295: (p *nopPage) RegularPages() Pages
// OK L297-299: (p *nopPage) RegularPagesRecursive() Pages
// OK L301-303: (p *nopPage) Paginate(seq any, options ...any) (*Pager, error)
// OK L305-307: (p *nopPage) Paginator(options ...any) (*Pager, error)
// OK L309-311: (p *nopPage) Param(key any) (any, error)
// OK L313-315: (p *nopPage) Params() maps.Params
// OK L317-319: (p *nopPage) Page() Page
// OK L321-323: (p *nopPage) Parent() Page
// OK L325-327: (p *nopPage) Ancestors() Pages
// OK L329-331: (p *nopPage) Path() string
// OK L333-335: (p *nopPage) PathInfo() *paths.Path
// OK L337-339: (p *nopPage) Permalink() string
// OK L341-343: (p *nopPage) Plain(context.Context) string
// OK L345-347: (p *nopPage) PlainWords(context.Context) []string
// OK L349-351: (p *nopPage) Prev() Page
// OK L353-355: (p *nopPage) PublishDate() (t time.Time)
// OK L357-359: (p *nopPage) PrevInSection() Page
// OK L361-363: (p *nopPage) NextInSection() Page
// OK L365-367: (p *nopPage) PrevPage() Page
// OK L369-371: (p *nopPage) NextPage() Page
// OK L373-375: (p *nopPage) RawContent() string
// OK L377-379: (p *nopPage) RenderShortcodes(ctx context.Context) (template.HTML, error)
// OK L381-383: (p *nopPage) ReadingTime(context.Context) int
// OK L385-387: (p *nopPage) Ref(argsm map[string]any) (string, error)
// OK L389-391: (p *nopPage) RelPermalink() string
// OK L393-395: (p *nopPage) RelRef(argsm map[string]any) (string, error)
// OK L397-399: (p *nopPage) Render(ctx context.Context, layout ...string) (template.HTML, error)
// OK L401-403: (p *nopPage) RenderString(ctx context.Context, args ...any) (template.HTML, error)
// OK L405-407: (p *nopPage) ResourceType() string
// OK L409-411: (p *nopPage) Resources() resource.Resources
// OK L413-415: (p *nopPage) Scratch() *maps.Scratch
// OK L417-419: (p *nopPage) Store() *maps.Scratch
// OK L421-423: (p *nopPage) RelatedKeywords(cfg related.IndexConfig) ([]related.Keyword, error)
// OK L425-427: (p *nopPage) Section() string
// OK L429-431: (p *nopPage) Sections() Pages
// OK L433-435: (p *nopPage) SectionsEntries() []string
// OK L437-439: (p *nopPage) SectionsPath() string
// OK L441-443: (p *nopPage) Site() Site
// OK L445-447: (p *nopPage) Sites() Sites
// OK L449-451: (p *nopPage) Slug() string
// OK L453-455: (p *nopPage) String() string
// OK L457-459: (p *nopPage) Summary(context.Context) template.HTML
// OK L461-463: (p *nopPage) TableOfContents(context.Context) template.HTML
// OK L465-467: (p *nopPage) Title() string
// OK L469-471: (p *nopPage) TranslationBaseName() string
// OK L473-475: (p *nopPage) TranslationKey() string
// OK L477-479: (p *nopPage) Translations() Pages
// OK L481-483: (p *nopPage) Truncated(context.Context) bool
// OK L485-487: (p *nopPage) Type() string
// OK L489-491: (p *nopPage) URL() string
// OK L493-495: (p *nopPage) UniqueID() string
// OK L497-499: (p *nopPage) Weight() int
// OK L501-503: (p *nopPage) WordCount(context.Context) int
// OK L505-507: (p *nopPage) Fragments(context.Context) *tableofcontents.Fragments
// OK L509-511: (p *nopPage) HeadingsFiltered(context.Context) tableofcontents.Headings
// OK L515-518: (r *nopContentRenderer) ParseAndRenderContent(ctx context.Context, content []byte, renderTOC bool) (converter.ResultRender, error)
// OK L520-522: (r *nopContentRenderer) ParseContent(ctx context.Context, content []byte) (converter.ResultParse, bool, error)
// OK L524-526: (r *nopContentRenderer) RenderContent(ctx context.Context, content []byte, doc any) (converter.ResultRender, bool, error)
// OK L538-540: (c *nopMarkup) Render(context.Context) (Content, error)
// OK L542-544: (c *nopMarkup) RenderString(ctx context.Context, args ...any) (template.HTML, error)
// OK L546-548: (c *nopMarkup) RenderShortcodes(context.Context) (template.HTML, error)
// OK L550-552: (c *nopContent) Plain(context.Context) string
// OK L554-556: (c *nopContent) PlainWords(context.Context) []string
// OK L558-560: (c *nopContent) WordCount(context.Context) int
// OK L562-564: (c *nopContent) FuzzyWordCount(context.Context) int
// OK L566-568: (c *nopContent) ReadingTime(context.Context) int
// OK L570-572: (c *nopContent) Len(context.Context) int
// OK L574-576: (c *nopContent) Content(context.Context) (template.HTML, error)
// OK L578-580: (c *nopContent) ContentWithoutSummary(context.Context) (template.HTML, error)
// OK L582-584: (c *nopMarkup) Fragments(context.Context) *tableofcontents.Fragments
// OK L586-588: (c *nopMarkup) FragmentsHTML(context.Context) template.HTML
// OK L590-592: (c *nopContent) Summary(context.Context) (Summary, error)
// ---------------------------------------------------------------------------
