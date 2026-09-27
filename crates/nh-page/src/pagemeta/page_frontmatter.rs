//! Port of `resources/page/pagemeta/page_frontmatter.go`.
//!
//! Owner: Wave B task T11 (page-api-paths).


//! Go `resources/page/pagemeta/page_frontmatter.go`: `PageConfig` (decoded front matter) and the
//! date handler chains (`date`, `lastmod`, `publishDate`, `expiryDate` from `[frontmatter]`; the
//! parsed `time.Time` replaces the param; `setParamIfNotSet` adds date params).

use std::sync::Arc;

use go_value::{Location, Map, Time, Value};
use nh_common::Result;
use nh_config::common_config::SitemapConfig;
use nh_media::media::media_type::MediaType;
use nh_media::output::output_format::Formats;

use super::pagemeta::BuildConfig;

/// Go: `pagemeta.Dates`.
#[derive(Clone, Debug)]
pub struct Dates {
    pub date: Time,
    pub lastmod: Time,
    pub publish_date: Time,
    pub expiry_date: Time,
}

impl Default for Dates {
    fn default() -> Self {
        Dates { date: Time::zero(), lastmod: Time::zero(), publish_date: Time::zero(), expiry_date: Time::zero() }
    }
}

impl Dates {
    /// Go: `Dates.IsAllDatesZero()`.
    pub fn is_all_dates_zero(&self) -> bool {
        self.date.is_zero() && self.lastmod.is_zero() && self.publish_date.is_zero() && self.expiry_date.is_zero()
    }

    /// Go: `Dates.UpdateDateAndLastmodAndPublishDateIfAfter(in)` (node date aggregation; publish
    /// date only when before now).
    // Go: resources/page/pagemeta/page_frontmatter.go:UpdateDateAndLastmodAndPublishDateIfAfter
    pub fn update_date_and_lastmod_and_publish_date_if_after(&mut self, other: &Dates) {
        todo!()
    }
}

/// Go: `pagemeta.Source` (content adapters; unused).
#[derive(Clone, Debug, Default)]
pub struct Source {
    pub media_type: String,
    pub markup: String,
    pub value: Option<Value>,
}

/// Go: `pagemeta.PageConfig` — the normalised front matter of a page.
#[derive(Clone, Debug, Default)]
pub struct PageConfig {
    pub dates: Dates,
    // PageConfigEarly
    pub kind: String,
    pub path: String,
    pub lang: String,
    pub cascade: Vec<Map>,
    pub content: Source,
    // the rest
    pub title: String,
    pub link_title: String,
    pub type_: String,
    pub layout: String,
    pub weight: i64,
    pub url: String,
    pub slug: String,
    pub description: String,
    pub summary: String,
    pub draft: bool,
    pub headless: bool,
    pub is_cjk_language: bool,
    pub translation_key: String,
    pub keywords: Vec<String>,
    pub aliases: Vec<String>,
    pub outputs: Vec<String>,
    /// Front matter `resources` (resource metadata).
    pub resources_meta: Vec<Map>,
    pub sitemap: SitemapConfig,
    pub build: BuildConfig,
    pub menus: Option<Value>,
    /// Normalised params (lower-cased keys; `[]any` of strings -> `[]string`; date params as time.Time).
    pub params: Option<Map>,
    pub content_media_type: MediaType,
    pub configured_output_formats: Formats,
}

/// Go: `pagemeta.ResourceConfig`.
#[derive(Clone, Debug, Default)]
pub struct ResourceConfig {
    pub path: String,
    pub name: String,
    pub title: String,
    pub params: Option<Map>,
    pub content: Source,
    pub content_media_type: MediaType,
}

/// Go: `pagemeta.FrontmatterConfig` (lists of date keys / `:filename` / `:fileModTime` / `:git`).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct FrontmatterConfig {
    pub date: Vec<String>,
    pub lastmod: Vec<String>,
    pub publish_date: Vec<String>,
    pub expiry_date: Vec<String>,
}

/// Go: `pagemeta.DecodeFrontMatterConfig(cfg)`.
// Go: resources/page/pagemeta/page_frontmatter.go:DecodeFrontMatterConfig
pub fn decode_front_matter_config(cfg: &dyn nh_config::config_provider::Provider) -> Result<FrontmatterConfig> {
    todo!()
}

/// Go: `pagemeta.FrontMatterDescriptor`.
pub struct FrontMatterDescriptor<'a> {
    /// The Params that will be used for templates (updated with parsed dates).
    pub params: &'a mut Map,
    pub base_filename: String,
    pub path_or_title: String,
    pub mod_time: Time,
    pub git_author_date: Time,
    pub page_config: &'a mut PageConfig,
    pub location: Arc<Location>,
}

/// Go: `pagemeta.FrontMatterHandler`.
#[derive(Clone)]
pub struct FrontMatterHandler {
    pub fm_config: FrontmatterConfig,
    pub(crate) all_date_keys: std::collections::BTreeSet<String>,
}

impl FrontMatterHandler {
    // Go: resources/page/pagemeta/page_frontmatter.go:NewFrontmatterHandler
    pub fn new(fm_config: FrontmatterConfig) -> Result<FrontMatterHandler> {
        todo!()
    }

    /// Go: `HandleDates(d)`.
    // Go: resources/page/pagemeta/page_frontmatter.go:HandleDates
    pub fn handle_dates(&self, d: &mut FrontMatterDescriptor<'_>) -> Result<()> {
        todo!()
    }

    /// Go: `IsDateKey(key)`.
    pub fn is_date_key(&self, key: &str) -> bool {
        self.all_date_keys.contains(key)
    }
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: resources/page/pagemeta/page_frontmatter.go (864 lines; 20/33 funcs executed)
//   types: DatesStrings, Dates, PageConfigEarly, PageConfig, ResourceConfig, Source, FrontMatterOnlyValues,
//          FrontMatterHandler, FrontMatterDescriptor, frontMatterFieldHandler, FrontmatterConfig,
//          frontmatterFieldHandlers
//    L58-60: (d Dates) IsDateOrLastModAfter(in Dates) bool
// EX L62-73: (d *Dates) UpdateDateAndLastmodAndPublishDateIfAfter(in Dates)
// EX L75-77: (d Dates) IsAllDatesZero() bool
//    L134-146: ClonePageConfigForRebuild(p *PageConfig, params map[string]any) *PageConfig
// EX L152-175: (p *PageConfig) Init(pagesFromData bool) error
//    L177-208: (p *PageConfig) CompileForPagesFromDataPre(basePath string, logger loggers.Logger, mediaTypes media.Types) error
// EX L210-242: (p *PageConfig) compilePrePost(ext string, mediaTypes media.Types) error
// EX L245-275: (p *PageConfig) Compile(ext string, logger loggers.Logger, outputFormats output.Formats, mediaTypes media.Types) error
// EX L278-282: MarkupToMediaType(s string, mediaTypes media.Types) media.Type
//    L296-301: (rc *ResourceConfig) Validate() error
//    L303-322: (rc *ResourceConfig) Compile(basePath string, pathParser *paths.PathParser, mediaTypes media.Types) error
//    L335-337: (s Source) IsZero() bool
//    L339-342: (s Source) IsResourceValue() bool
//    L344-353: (s Source) ValueAsString() string
//    L355-357: (s Source) ValueAsOpenReadSeekCloser() hugio.OpenReadSeekCloser
// EX L415-448: (f FrontMatterHandler) HandleDates(d *FrontMatterDescriptor) error
// EX L452-454: (f FrontMatterHandler) IsDateKey(key string) bool
//    L461-493: dateAndSlugFromBaseFilename(location *time.Location, path string) (time.Time, string)
// EX L497-510: (f FrontMatterHandler) newChainedFrontMatterFieldHandler(handlers ...frontMatterFieldHandler) frontMatterFieldHandler
// EX L542-549: newDefaultFrontmatterConfig() FrontmatterConfig
// EX L551-584: DecodeFrontMatterConfig(cfg config.Provider) (FrontmatterConfig, error)
// EX L586-596: addDateFieldAliases(values []string) []string
// EX L598-608: expandDefaultValues(values []string, defaults []string) []string
//    L610-617: toLowerSlice(in any) []string
// EX L621-647: NewFrontmatterHandler(logger loggers.Logger, frontMatterConfig FrontmatterConfig) (FrontMatterHandler, error)
// EX L649-689: (f *FrontMatterHandler) createHandlers() error
// EX L691-696: setParamIfNotSet(key string, value any, d *FrontMatterDescriptor)
// EX L698-776: (f FrontMatterHandler) createContentAdapterDatesHandler(fmcfg FrontmatterConfig) (func(d *FrontMatterDescriptor) error, error)
// EX L778-796: (f FrontMatterHandler) createDateHandler(identifiers []string, setter func(d *FrontMatterDescriptor, t time.Time)) (frontMatterFieldHandler, error)
// EX L800-826: (f *frontmatterFieldHandlers) newDateFieldHandler(key string, setter func(d *FrontMatterDescriptor, t time.Time)) frontMatterFieldHandler
//    L828-844: (f *frontmatterFieldHandlers) newDateFilenameHandler(setter func(d *FrontMatterDescriptor, t time.Time)) frontMatterFieldHandler
//    L846-854: (f *frontmatterFieldHandlers) newDateModTimeHandler(setter func(d *FrontMatterDescriptor, t time.Time)) frontMatterFieldHandler
// EX L856-864: (f *frontmatterFieldHandlers) newDateGitAuthorDateHandler(setter func(d *FrontMatterDescriptor, t time.Time)) frontMatterFieldHandler
// ---------------------------------------------------------------------------
