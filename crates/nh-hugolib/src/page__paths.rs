//! Port of `hugolib/page__paths.go`.
//!
//! Owner: Wave B task T21 (hugolib-assemble).

//! Go `hugolib/page__paths.go`: `createTargetPathDescriptor` (prefixes: "" for en, "th" for th;
//! sitemap always in a language subdir; BaseName = slug / standalone basename / BaseNameNoIdentifier)
//! and per-format target paths + `page.OutputFormats`. Owned by T21 because `initLazyProviders`
//! (page__init.rs, run during `assembleResources`) needs `newPagePaths`.
//!
//! Both functions run during assembly, before the `Arc<HugoSites>` exists, so they take the
//! page arena (`&HugoSites` + the page) instead of a `PageHandle`. The permalink expander reads
//! the page through [`PathsPageView`], a `page.Page` with the values Go's expander reads.

use std::any::Any;
use std::borrow::Cow;
use std::collections::BTreeMap;
use std::sync::Arc;

use go_value::{GoString, HostCtx, Map, Time, Value};
use nh_common::Result;
use nh_common::kinds;
use nh_common::object::GoResult;
use nh_common::paths::pathparser::Path;
use nh_config::common_config::SitemapConfig;
use nh_helpers::source::file_info::File;
use nh_media::media::media_type::MediaType;
use nh_media::output::output_format::Formats;
use nh_page::page::{Page, PageRef, Pages};
use nh_page::page_outputformat::{OutputFormat as PageOutputFormat, OutputFormats};
use nh_page::page_paths::{TargetPathDescriptor, TargetPaths, create_target_paths};
use nh_page::related::{IndexConfig, Keyword};
use nh_page::site::SiteRef;
use nh_resource::resourcetypes::{Resource, Resources};

use crate::hugo_sites::HugoSites;
use crate::page::PageState;
use crate::site::Site;

/// Go: `targetPathsHolder`.
#[derive(Clone, Debug)]
pub struct TargetPathsHolder {
    pub rel_url: String,
    pub paths: TargetPaths,
    pub output_format: PageOutputFormat,
}

/// Go: `pagePaths`.
#[derive(Clone)]
pub struct PagePaths {
    pub output_formats: OutputFormats,
    pub first_output_format: PageOutputFormat,
    /// Format name -> target paths.
    pub target_paths: BTreeMap<String, TargetPathsHolder>,
    pub target_path_descriptor: TargetPathDescriptor,
}

impl PagePaths {
    /// Go: `(l pagePaths) OutputFormats()`.
    // Go: hugolib/page__paths.go:OutputFormats
    pub fn output_formats(&self) -> &OutputFormats {
        &self.output_formats
    }
}

/// Go: `newPagePaths(ps)`.
///
/// Deviation: when the page has no output formats Go returns the zero `pagePaths{}` (a zero
/// target path descriptor too); the port keeps the computed descriptor (a zero one would need a
/// `PathSpec`). Such a page is never rendered and has no links either way.
// Go: hugolib/page__paths.go:newPagePaths
pub fn new_page_paths(h: &HugoSites, ps: &PageState) -> Result<PagePaths> {
    let s = &h.sites[ps.site_idx];
    let pm = &ps.meta;

    let target_path_descriptor = create_target_path_descriptor(h, ps)?;

    let output_formats: Formats = if pm.is_standalone() {
        Formats(vec![
            pm.standalone_output_format
                .clone()
                .expect("standalone page has its output format"),
        ])
    } else {
        let mut of = pm.output_formats(&s.conf);
        if of.0.is_empty() {
            return Ok(PagePaths {
                output_formats: Vec::new(),
                first_output_format: PageOutputFormat::new("", "", false, Default::default()),
                target_paths: BTreeMap::new(),
                target_path_descriptor,
            });
        }

        if pm.no_render() {
            of.0.truncate(1);
        }
        of
    };

    let path_spec = s.deps.path_spec().clone();
    let n = output_formats.0.len();
    let mut page_output_formats: Vec<PageOutputFormat> = Vec::with_capacity(n);
    let mut targets: BTreeMap<String, TargetPathsHolder> = BTreeMap::new();

    for (i, f) in output_formats.0.iter().enumerate() {
        let mut desc = target_path_descriptor.clone();
        desc.type_ = f.clone();
        let paths = create_target_paths(&desc);

        let mut rel_permalink = String::new();
        let mut permalink = String::new();

        // If a page is headless or bundled in another,
        // it will not get published on its own and it will have no links.
        // We also check the build options if it's set to not render or have
        // a link.
        if !pm.no_link() && !pm.bundled {
            rel_permalink = paths.rel_permalink(&path_spec);
            permalink = paths.permalink_for_output_format(&path_spec, f);
        }

        page_output_formats.push(PageOutputFormat::new(
            &rel_permalink,
            &permalink,
            n == 1,
            f.clone(),
        ));

        // Use the main format for permalinks, usually HTML.
        let mut permalinks_index = 0;
        if f.permalinkable {
            // Unless it's permalinkable.
            permalinks_index = i;
        }

        targets.insert(
            f.name.clone(),
            TargetPathsHolder {
                rel_url: rel_permalink,
                paths,
                output_format: page_output_formats[permalinks_index].clone(),
            },
        );
    }

    let out: OutputFormats = if !pm.no_link() {
        page_output_formats.clone()
    } else {
        Vec::new()
    };

    Ok(PagePaths {
        output_formats: out,
        first_output_format: page_output_formats[0].clone(),
        target_paths: targets,
        target_path_descriptor,
    })
}

/// Go: `createTargetPathDescriptor(p)`.
// Go: hugolib/page__paths.go:createTargetPathDescriptor
pub fn create_target_path_descriptor(h: &HugoSites, p: &PageState) -> Result<TargetPathDescriptor> {
    let s = &h.sites[p.site_idx];
    let d = &s.deps;
    let pm = &p.meta;
    let always_in_sub_dir = pm.kind() == kinds::KIND_SITEMAP;

    let cs = crate::page__tree::current_section_id(h, p.id)
        // Go dereferences the nil home page.
        .expect("invalid memory address or nil pointer dereference");
    let mut page_info_page = pm.path_info.clone();
    let mut page_info_current_section = h.page(cs).meta.path_info.clone();
    if d.conf.disable_path_to_lower() {
        page_info_page = Arc::new(page_info_page.unnormalized().clone());
        page_info_current_section = Arc::new(page_info_current_section.unnormalized().clone());
    }

    let mut desc = TargetPathDescriptor {
        path_spec: d.path_spec().clone(),
        type_: Default::default(),
        kind: pm.kind().to_string(),
        path: page_info_page.clone(),
        section: Some(page_info_current_section),
        base_name: String::new(),
        prefix_file_path: String::new(),
        prefix_link: String::new(),
        // Go: `s.h.Conf` is the first site's config.
        ugly_urls: h.deps.conf.is_ugly_urls(pm.section()),
        force_prefix: h.deps.conf.is_multihost() || always_in_sub_dir,
        url: pm.page_config.url.clone(),
        addends: String::new(),
        expanded_permalink: String::new(),
    };

    if !pm.slug().is_empty() {
        desc.base_name = pm.slug().to_string();
    } else if let Some(sf) = &pm.standalone_output_format
        && !sf.base_name.is_empty()
    {
        desc.base_name = sf.base_name.clone();
    } else {
        desc.base_name = page_info_page.base_name_no_identifier().to_string();
    }

    desc.prefix_file_path = get_language_target_path_lang(h, s, always_in_sub_dir);
    desc.prefix_link = get_language_permalink_lang(h, s, always_in_sub_dir);

    let view = PathsPageView::new(h, p, cs);
    let permalinks = &d.resource_spec().permalinks;

    if !desc.url.is_empty() && desc.url.contains(':') {
        // Attempt to parse and expand an url
        let opath = permalinks.expand_pattern(&desc.url, &view)?;

        if !opath.is_empty() {
            desc.url = query_unescape(&opath);
        }
    }

    let opath = permalinks.expand(pm.section(), &view)?;

    if !opath.is_empty() {
        let mut opath = query_unescape(&opath);
        if opath.ends_with("//") {
            // When rewriting the _index of the section the permalink config is applied to,
            // we get double slashes at the end sometimes; clear them up here
            opath.pop();
        }

        desc.expanded_permalink = opath;
        // (Go logs the expanded path at debug level.)
    }

    Ok(desc)
}

/// Go: `opath, _ = url.QueryUnescape(opath)` (`""` on error).
fn query_unescape(s: &str) -> String {
    match go_url::query_unescape(s) {
        Ok(b) => String::from_utf8_lossy(&b).into_owned(),
        Err(_) => String::new(),
    }
}

/// Go: `(s *Site) getLanguageTargetPathLang(alwaysInSubDir)` — any language code to prefix the
/// target file path with (T23 owns the `Site` method; the descriptor needs it during
/// assembly).
// Go: hugolib/site.go:getLanguageTargetPathLang
pub(crate) fn get_language_target_path_lang(
    h: &HugoSites,
    s: &Site,
    always_in_sub_dir: bool,
) -> String {
    if h.deps.conf.is_multihost() {
        return s.language.lang.clone();
    }

    get_language_permalink_lang(h, s, always_in_sub_dir)
}

/// Go: `(s *Site) getLanguagePermalinkLang(alwaysInSubDir)` — any language code to prefix the
/// relative permalink with.
// Go: hugolib/site.go:getLanguagePermalinkLang
pub(crate) fn get_language_permalink_lang(
    h: &HugoSites,
    s: &Site,
    always_in_sub_dir: bool,
) -> String {
    if h.deps.conf.is_multihost() {
        return String::new();
    }

    if h.deps.conf.is_multilingual() && always_in_sub_dir {
        return s.language.lang.clone();
    }

    // Go: `s.GetLanguagePrefix()` (the site's PathSpec: `Cfg.LanguagePrefix()`).
    s.deps.conf.language_prefix()
}

/// The page as the permalink expander sees it while the target path descriptor is created
/// (Go passes the `*pageState`): kind, date, title, slug, section, file, path info and the
/// current section's `SectionsEntries`/`SectionsPath`. `current_section()` is `None`, so the
/// expander asks the page itself for those (nh-page's deviation 11). Nothing else is read.
pub struct PathsPageView {
    kind: String,
    date: Time,
    title: String,
    slug: String,
    section: String,
    path_info: Arc<Path>,
    file: Option<Arc<File>>,
    sections_entries: Vec<String>,
    sections_path: String,
}

impl PathsPageView {
    fn new(h: &HugoSites, p: &PageState, cs: crate::page::PageId) -> PathsPageView {
        let sections_path = h.page(cs).meta.path();
        let sections_entries = if sections_path == "/" {
            Vec::new()
        } else {
            sections_path[1..]
                .split('/')
                .map(|s| s.to_string())
                .collect()
        };
        PathsPageView {
            kind: p.meta.kind().to_string(),
            date: p.meta.date(),
            title: p.meta.title().to_string(),
            slug: p.meta.slug().to_string(),
            section: p.meta.section().to_string(),
            path_info: p.meta.path_info.clone(),
            file: p.meta.f.clone(),
            sections_entries,
            sections_path,
        }
    }
}

fn not_read(what: &str) -> ! {
    unreachable!("{what} is not read by the permalink expander")
}

impl Resource for PathsPageView {
    fn resource_type(&self) -> String {
        "page".to_string()
    }
    fn media_type(&self) -> MediaType {
        not_read("MediaType")
    }
    fn permalink(&self) -> String {
        not_read("Permalink")
    }
    fn rel_permalink(&self) -> String {
        not_read("RelPermalink")
    }
    fn data(&self) -> Value {
        not_read("Data")
    }
    fn name(&self) -> String {
        not_read("Name")
    }
    fn title(&self) -> String {
        self.title.clone()
    }
    fn params(&self) -> Arc<Map> {
        not_read("Params")
    }
    fn key(&self) -> String {
        not_read("Key")
    }
    fn tpl_type_name(&self) -> Cow<'_, str> {
        Cow::Borrowed("*hugolib.pageState")
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

impl Page for PathsPageView {
    fn page_id(&self) -> u64 {
        0
    }
    fn unwrap_page(self: Arc<Self>) -> Arc<dyn Page> {
        self
    }
    fn kind(&self) -> String {
        self.kind.clone()
    }
    fn title(&self) -> String {
        self.title.clone()
    }
    fn link_title(&self) -> String {
        not_read("LinkTitle")
    }
    fn description(&self) -> String {
        not_read("Description")
    }
    fn weight(&self) -> i64 {
        not_read("Weight")
    }
    fn date(&self) -> Time {
        self.date.clone()
    }
    fn lastmod(&self) -> Time {
        not_read("Lastmod")
    }
    fn publish_date(&self) -> Time {
        not_read("PublishDate")
    }
    fn expiry_date(&self) -> Time {
        not_read("ExpiryDate")
    }
    fn is_home(&self) -> bool {
        not_read("IsHome")
    }
    fn is_node(&self) -> bool {
        not_read("IsNode")
    }
    fn is_page(&self) -> bool {
        not_read("IsPage")
    }
    fn is_section(&self) -> bool {
        not_read("IsSection")
    }
    fn section(&self) -> String {
        self.section.clone()
    }
    fn page_type(&self) -> String {
        not_read("Type")
    }
    fn layout(&self) -> String {
        not_read("Layout")
    }
    fn lang(&self) -> String {
        not_read("Lang")
    }
    fn path(&self) -> String {
        not_read("Path")
    }
    fn path_info(&self) -> Arc<Path> {
        self.path_info.clone()
    }
    fn slug(&self) -> String {
        self.slug.clone()
    }
    fn draft(&self) -> bool {
        not_read("Draft")
    }
    fn aliases(&self) -> Vec<String> {
        not_read("Aliases")
    }
    fn keywords(&self) -> Vec<String> {
        not_read("Keywords")
    }
    fn bundle_type(&self) -> String {
        not_read("BundleType")
    }
    fn sitemap(&self) -> SitemapConfig {
        not_read("Sitemap")
    }
    fn param(&self, _key: &Value) -> Result<Value> {
        not_read("Param")
    }
    fn page_params(&self) -> Arc<Map> {
        not_read("Params")
    }
    fn site(&self) -> SiteRef {
        not_read("Site")
    }
    fn file(&self) -> Option<Arc<File>> {
        self.file.clone()
    }
    fn parent(&self) -> Option<PageRef> {
        not_read("Parent")
    }
    fn pages(&self) -> Pages {
        not_read("Pages")
    }
    fn regular_pages(&self) -> Pages {
        not_read("RegularPages")
    }
    fn resources(&self) -> Resources {
        not_read("Resources")
    }
    fn output_formats(&self) -> OutputFormats {
        not_read("OutputFormats")
    }
    fn all_translations(&self) -> Pages {
        not_read("AllTranslations")
    }
    fn translations(&self) -> Pages {
        not_read("Translations")
    }
    fn current_section(&self) -> Option<PageRef> {
        None
    }
    fn sections_entries(&self) -> Vec<String> {
        self.sections_entries.clone()
    }
    fn sections_path(&self) -> String {
        self.sections_path.clone()
    }
    fn plain(&self, _ctx: HostCtx<'_>) -> Result<GoString> {
        not_read("Plain")
    }
    fn content_len(&self, _ctx: HostCtx<'_>) -> Result<i64> {
        not_read("Len")
    }
    fn render_string(&self, _ctx: HostCtx<'_>, _args: &[Value]) -> Result<Value> {
        not_read("RenderString")
    }
    fn related_keywords(&self, _cfg: &IndexConfig) -> Result<Vec<Keyword>> {
        not_read("RelatedKeywords")
    }
    fn ref_(&self, _args: &Map) -> Result<String> {
        not_read("Ref")
    }
    fn rel_ref(&self, _args: &Map) -> Result<String> {
        not_read("RelRef")
    }
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: hugolib/page__paths.go (181 lines; 3/3 funcs executed)
//   types: pagePaths
// OK L26-97: newPagePaths(ps *pageState) (pagePaths, error)
// OK L107-109: (l pagePaths) OutputFormats() page.OutputFormats
// OK L111-181: createTargetPathDescriptor(p *pageState) (page.TargetPathDescriptor, error)
// ---------------------------------------------------------------------------
