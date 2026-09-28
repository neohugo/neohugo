//! Port of `resources/page/pagemeta/page_frontmatter.go`.
//!
//! Owner: Wave B task T11 (page-api-paths).
//!
//! Go `resources/page/pagemeta/page_frontmatter.go`: `PageConfig` (decoded front matter) and the
//! date handler chains (`date`, `lastmod`, `publishDate`, `expiryDate` from `[frontmatter]`; the
//! parsed `time.Time` replaces the param; `setParamIfNotSet` adds date params).

use std::collections::BTreeSet;
use std::sync::Arc;

use go_time::GoTimeExt;
use go_value::{GoString, Location, Map, MapType, Time, Value};
use nh_common::Result;
use nh_common::herrors::Error;
use nh_common::kinds;
use nh_common::loggers::Logger;
use nh_common::paths::pathparser::{Path, PathParser};
use nh_config::common_config::SitemapConfig;
use nh_media::media::media_type::{MediaType, Types};
use nh_media::output::output_format::Formats;

use super::pagemeta::BuildConfig;
use crate::page_matcher::Cascade;

/// Go: `pagemeta.DatesStrings` (the content adapter's string dates).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DatesStrings {
    pub date: String,
    pub lastmod: String,
    pub publish_date: String,
    pub expiry_date: String,
}

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
        Dates {
            date: Time::zero(),
            lastmod: Time::zero(),
            publish_date: Time::zero(),
            expiry_date: Time::zero(),
        }
    }
}

impl Dates {
    /// Go: `Dates.IsDateOrLastModAfter(in)`.
    // Go: resources/page/pagemeta/page_frontmatter.go:IsDateOrLastModAfter
    pub fn is_date_or_last_mod_after(&self, other: &Dates) -> bool {
        self.date.go_after(&other.date) || self.lastmod.go_after(&other.lastmod)
    }

    /// Go: `Dates.IsAllDatesZero()`.
    // Go: resources/page/pagemeta/page_frontmatter.go:IsAllDatesZero
    pub fn is_all_dates_zero(&self) -> bool {
        self.date.go_is_zero()
            && self.lastmod.go_is_zero()
            && self.publish_date.go_is_zero()
            && self.expiry_date.go_is_zero()
    }

    /// Go: `Dates.UpdateDateAndLastmodAndPublishDateIfAfter(in)` (node date aggregation; publish
    /// date only when before now).
    // Go: resources/page/pagemeta/page_frontmatter.go:UpdateDateAndLastmodAndPublishDateIfAfter
    pub fn update_date_and_lastmod_and_publish_date_if_after(&mut self, other: &Dates) {
        if other.date.go_after(&self.date) {
            self.date = other.date.clone();
        }
        if other.lastmod.go_after(&self.lastmod) {
            self.lastmod = other.lastmod.clone();
        }

        if other.publish_date.go_after(&self.publish_date)
            && other.publish_date.go_before(&nh_common::htime::now())
        {
            self.publish_date = other.publish_date.clone();
        }
    }
}

/// Go: `pagemeta.Source` — the content of a page or resource (content adapters, front matter
/// `content`).
#[derive(Clone, Debug, Default)]
pub struct Source {
    /// MediaType is the media type of the content.
    pub media_type: String,
    /// The markup used in Value. Only used in front matter.
    pub markup: String,
    /// The content (`None` = Go's nil).
    pub value: Option<Value>,
}

impl Source {
    // Go: resources/page/pagemeta/page_frontmatter.go:IsZero
    pub fn is_zero(&self) -> bool {
        match &self.value {
            None => true,
            Some(v) => !nh_common::hreflect::is_truthful(v),
        }
    }

    /// Go: `IsResourceValue()` — the value is a `resource.Resource`.
    // Go: resources/page/pagemeta/page_frontmatter.go:IsResourceValue
    pub fn is_resource_value(&self) -> bool {
        match &self.value {
            None => false,
            Some(v) => nh_resource::resourcetypes::resource_from_value_any(v).is_some(),
        }
    }

    /// Go: `ValueAsString()` — Go panics when cast fails; the port returns Go's panic text as an
    /// error.
    // Go: resources/page/pagemeta/page_frontmatter.go:ValueAsString
    pub fn value_as_string(&self) -> Result<GoString> {
        let Some(v) = &self.value else {
            return Ok(GoString::empty());
        };
        if v.is_invalid() {
            return Ok(GoString::empty());
        }
        nh_common::cast::caste::to_string_e(v).map_err(|err| {
            Error::new(format!(
                "content source: failed to convert {} to string: {}",
                v.go_type_name(),
                err.message()
            ))
        })
    }

    /// Go: `ValueAsOpenReadSeekCloser()` — the string value, opened anew on every call.
    // Go: resources/page/pagemeta/page_frontmatter.go:ValueAsOpenReadSeekCloser
    pub fn value_as_open_read_seek_closer(&self) -> Result<Vec<u8>> {
        Ok(self.value_as_string()?.to_vec())
    }
}

/// Go: `pagemeta.PageConfig` — the normalised front matter of a page.
#[derive(Clone, Debug, Default)]
pub struct PageConfig {
    pub dates: Dates,
    pub dates_strings: DatesStrings,
    // PageConfigEarly
    pub kind: String,
    pub path: String,
    pub lang: String,
    /// Go `Cascade []map[string]any` (`None` = nil).
    pub cascade: Option<Vec<Map>>,
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
    /// Normalised params (lower-cased keys; `[]any` of strings -> `[]string`; date params as
    /// time.Time). `None` is Go's nil map.
    pub params: Option<Map>,
    /// The raw data from the content adapter (`None` = nil).
    pub content_adapter_data: Option<Map>,
    // Compiled values.
    pub cascade_compiled: Option<Cascade>,
    pub content_media_type: MediaType,
    pub configured_output_formats: Formats,
    pub is_from_content_adapter: bool,
}

impl PageConfig {
    /// Go: `pagemeta.DefaultPageConfig` (`PageConfig{Build: DefaultBuildConfig}`).
    pub fn default_page_config() -> PageConfig {
        PageConfig {
            build: BuildConfig::default_build_config(),
            ..Default::default()
        }
    }

    // Go: resources/page/pagemeta/page_frontmatter.go:Init
    pub fn init(&mut self, pages_from_data: bool) -> Result<()> {
        if pages_from_data {
            self.path = self
                .path
                .strip_prefix('/')
                .unwrap_or(&self.path)
                .to_string();

            if self.path.is_empty() && self.kind != kinds::KIND_HOME {
                return Err(Error::new("empty path is reserved for the home page"));
            }
            if !self.lang.is_empty() {
                return Err(Error::new("lang must not be set"));
            }

            if !self.content.markup.is_empty() {
                return Err(Error::new("markup must not be set, use mediaType"));
            }
        }

        if self.cascade.is_some() && !kinds::is_branch(&self.kind) {
            return Err(Error::new("cascade is only supported for branch nodes"));
        }

        Ok(())
    }

    // Go: resources/page/pagemeta/page_frontmatter.go:CompileForPagesFromDataPre
    pub fn compile_for_pages_from_data_pre(
        &mut self,
        base_path: &str,
        logger: Option<&Logger>,
        media_types: &Types,
    ) -> Result<()> {
        // In content adapters, we always get relative paths.
        if !base_path.is_empty() {
            self.path = go_path::path::join(&[base_path, self.path.as_str()]);
        }

        self.params = Some(match &self.params {
            None => Map::new(MapType::Params),
            Some(p) => nh_common::maps::params::prepare_params_clone(p),
        });

        if self.kind.is_empty() {
            self.kind = kinds::KIND_PAGE.to_string();
        }

        if let Some(cascade) = &self.cascade {
            let input = Value::list(
                go_value::SliceType::MapStringAny,
                cascade.iter().map(|m| Value::map(m.clone())).collect(),
            );
            let c = crate::page_matcher::decode_cascade(logger, false, &input).map_err(|err| {
                Error::new(format!("failed to decode cascade: {}", err.message()))
            })?;
            self.cascade_compiled = Some(c);
        }

        // Note that NormalizePathStringBasic will make sure that we don't preserve the
        // unnormalized path.
        self.path = nh_common::paths::pathparser::normalize_path_string_basic(&self.path);

        self.compile_pre_post("", media_types)
    }

    // Go: resources/page/pagemeta/page_frontmatter.go:compilePrePost
    fn compile_pre_post(&mut self, ext: &str, media_types: &Types) -> Result<()> {
        if self.content.markup.is_empty() && self.content.media_type.is_empty() {
            let ext = if ext.is_empty() { "md" } else { ext };
            self.content_media_type = markup_to_media_type(ext, media_types);
            if self.content_media_type.is_zero() {
                return Err(Error::new(format!(
                    "failed to resolve media type for suffix {}",
                    go_strconv::quote(ext.as_bytes())
                )));
            }
        }

        let mut s = String::new();
        if self.content_media_type.is_zero() {
            if !self.content.media_type.is_empty() {
                s = self.content.media_type.clone();
                self.content_media_type = media_types.get_by_type(&s).unwrap_or_default();
            }

            if self.content_media_type.is_zero() && !self.content.markup.is_empty() {
                s = self.content.markup.clone();
                self.content_media_type = markup_to_media_type(&s, media_types);
            }
        }

        if self.content_media_type.is_zero() {
            return Err(Error::new(format!(
                "failed to resolve media type for {}",
                go_strconv::quote(s.as_bytes())
            )));
        }

        if self.content.markup.is_empty() {
            self.content.markup = self.content_media_type.sub_type.clone();
        }
        Ok(())
    }

    /// Compile sets up the page configuration after all fields have been set.
    // Go: resources/page/pagemeta/page_frontmatter.go:Compile
    pub fn compile(
        &mut self,
        ext: &str,
        _logger: Option<&Logger>,
        output_formats: &Formats,
        media_types: &Types,
    ) -> Result<()> {
        if self.is_from_content_adapter {
            // Go: mapstructure.WeakDecode(p.ContentAdapterData, p) (content adapters).
            return Err(Error::new(
                "neohugo-rs: content adapters (_content.gotmpl) are not supported",
            ));
        }

        match &mut self.params {
            None => self.params = Some(Map::new(MapType::Params)),
            Some(p) => nh_common::maps::params::prepare_params(p),
        }

        self.compile_pre_post(ext, media_types)?;

        if !self.outputs.is_empty() {
            let names: Vec<&str> = self.outputs.iter().map(|s| s.as_str()).collect();
            match output_formats.get_by_names(&names) {
                Ok(out_formats) => self.configured_output_formats = out_formats,
                Err(err) => {
                    return Err(Error::new(format!(
                        "failed to resolve output formats [{}]: {}",
                        self.outputs.join(" "),
                        err.message()
                    )));
                }
            }
        }

        Ok(())
    }
}

/// Go: `pagemeta.ClonePageConfigForRebuild(p, params)`.
// Go: resources/page/pagemeta/page_frontmatter.go:ClonePageConfigForRebuild
pub fn clone_page_config_for_rebuild(p: &PageConfig, params: Map) -> PageConfig {
    let mut pp = PageConfig {
        kind: p.kind.clone(),
        path: p.path.clone(),
        lang: p.lang.clone(),
        cascade: p.cascade.clone(),
        content: p.content.clone(),
        is_from_content_adapter: p.is_from_content_adapter,
        ..Default::default()
    };
    if pp.is_from_content_adapter {
        pp.content_adapter_data = Some(params);
    } else {
        pp.params = Some(params);
    }

    pp
}

/// MarkupToMediaType converts a markup string to a media type (zero if none).
// Go: resources/page/pagemeta/page_frontmatter.go:MarkupToMediaType
pub fn markup_to_media_type(s: &str, media_types: &Types) -> MediaType {
    let s = go_unicode::strings::to_lower_str(s);
    media_types
        .get_best_match(&nh_markup::markup::resolve_markup(&s))
        .unwrap_or_default()
}

/// Go: `pagemeta.ResourceConfig`.
#[derive(Clone, Debug, Default)]
pub struct ResourceConfig {
    pub path: String,
    pub name: String,
    pub title: String,
    pub params: Option<Map>,
    pub content: Source,
    // Compiled values.
    pub path_info: Option<Arc<Path>>,
    pub content_media_type: MediaType,
}

impl ResourceConfig {
    // Go: resources/page/pagemeta/page_frontmatter.go:Validate
    pub fn validate(&self) -> Result<()> {
        if !self.content.markup.is_empty() {
            return Err(Error::new("markup must not be set, use mediaType"));
        }
        Ok(())
    }

    // Go: resources/page/pagemeta/page_frontmatter.go:Compile
    pub fn compile(
        &mut self,
        base_path: &str,
        path_parser: &PathParser,
        media_types: &Types,
    ) -> Result<()> {
        if let Some(p) = &mut self.params {
            nh_common::maps::params::prepare_params(p);
        }

        // Note that NormalizePathStringBasic will make sure that we don't preserve the
        // unnormalized path.
        self.path =
            nh_common::paths::pathparser::normalize_path_string_basic(&go_path::path::join(&[
                base_path,
                self.path.as_str(),
            ]));
        self.path_info = Some(Arc::new(
            path_parser.parse(nh_common::files::COMPONENT_FOLDER_CONTENT, &self.path),
        ));
        if !self.content.media_type.is_empty() {
            match media_types.get_by_type(&self.content.media_type) {
                Some(mt) => self.content_media_type = mt,
                None => {
                    return Err(Error::new(format!(
                        "media type {} not found",
                        go_strconv::quote(self.content.media_type.as_bytes())
                    )));
                }
            }
        }
        Ok(())
    }
}

// These are all the date handler identifiers.
// All identifiers not starting with a ":" maps to a front matter parameter.
const FM_DATE: &str = "date";
const FM_PUB_DATE: &str = "publishdate";
const FM_LASTMOD: &str = "lastmod";
const FM_EXPIRY_DATE: &str = "expirydate";

/// Gets date from filename, e.g 218-02-22-mypage.md
const FM_FILENAME: &str = ":filename";

/// Gets date from file OS mod time.
const FM_MOD_TIME: &str = ":filemodtime";

/// Gets date from Git
const FM_GIT_AUTHOR_DATE: &str = ":git";

/// Go: `dateFieldAliases`.
fn date_field_aliases(v: &str) -> Option<&'static [&'static str]> {
    match v {
        FM_DATE => Some(&[]),
        FM_LASTMOD => Some(&["modified"]),
        FM_PUB_DATE => Some(&["pubdate", "published"]),
        FM_EXPIRY_DATE => Some(&["unpublishdate"]),
        _ => None,
    }
}

/// Go: `pagemeta.FrontmatterConfig` (lists of date keys / `:filename` / `:fileModTime` / `:git`).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct FrontmatterConfig {
    /// Controls how the Date is set from front matter.
    pub date: Vec<String>,
    /// Controls how the Lastmod is set from front matter.
    pub lastmod: Vec<String>,
    /// Controls how the PublishDate is set from front matter.
    pub publish_date: Vec<String>,
    /// Controls how the ExpiryDate is set from front matter.
    pub expiry_date: Vec<String>,
}

/// This is the config you get when doing nothing.
// Go: resources/page/pagemeta/page_frontmatter.go:newDefaultFrontmatterConfig
fn new_default_frontmatter_config() -> FrontmatterConfig {
    let v = |s: &[&str]| s.iter().map(|x| x.to_string()).collect::<Vec<_>>();
    FrontmatterConfig {
        date: v(&[FM_DATE, FM_PUB_DATE, FM_LASTMOD]),
        lastmod: v(&[FM_GIT_AUTHOR_DATE, FM_LASTMOD, FM_DATE, FM_PUB_DATE]),
        publish_date: v(&[FM_PUB_DATE, FM_DATE]),
        expiry_date: v(&[FM_EXPIRY_DATE]),
    }
}

/// Go: `pagemeta.DecodeFrontMatterConfig(cfg)`.
// Go: resources/page/pagemeta/page_frontmatter.go:DecodeFrontMatterConfig
pub fn decode_front_matter_config(
    cfg: &dyn nh_config::config_provider::Provider,
) -> Result<FrontmatterConfig> {
    let mut c = new_default_frontmatter_config();
    let default_config = c.clone();

    if cfg.is_set("frontmatter") {
        let fm = cfg.get_string_map("frontmatter");
        for (k, v) in fm.entries.iter() {
            let loki = go_unicode::strings::to_lower(k.as_bytes());
            match &loki[..] {
                b"date" => c.date = to_lower_slice(v),
                b"publishdate" => c.publish_date = to_lower_slice(v),
                b"lastmod" => c.lastmod = to_lower_slice(v),
                b"expirydate" => c.expiry_date = to_lower_slice(v),
                _ => {}
            }
        }
    }

    let expander = |c: &[String], d: &[String]| -> Vec<String> {
        let out = expand_default_values(c, d);
        add_date_field_aliases(&out)
    };

    c.date = expander(&c.date, &default_config.date);
    c.publish_date = expander(&c.publish_date, &default_config.publish_date);
    c.lastmod = expander(&c.lastmod, &default_config.lastmod);
    c.expiry_date = expander(&c.expiry_date, &default_config.expiry_date);

    Ok(c)
}

// Go: resources/page/pagemeta/page_frontmatter.go:addDateFieldAliases
fn add_date_field_aliases(values: &[String]) -> Vec<String> {
    let mut complete: Vec<String> = Vec::new();

    for v in values {
        complete.push(v.clone());
        if let Some(aliases) = date_field_aliases(v) {
            complete.extend(aliases.iter().map(|a| a.to_string()));
        }
    }
    nh_helpers::general::unique_strings_reuse(complete)
}

// Go: resources/page/pagemeta/page_frontmatter.go:expandDefaultValues
fn expand_default_values(values: &[String], defaults: &[String]) -> Vec<String> {
    let mut out = Vec::new();
    for v in values {
        if v == ":default" {
            out.extend(defaults.iter().cloned());
        } else {
            out.push(v.clone());
        }
    }
    out
}

// Go: resources/page/pagemeta/page_frontmatter.go:toLowerSlice
fn to_lower_slice(input: &Value) -> Vec<String> {
    nh_common::cast::caste::to_string_slice(input)
        .iter()
        .map(|s| String::from_utf8_lossy(&go_unicode::strings::to_lower(s.as_bytes())).into_owned())
        .collect()
}

/// Go: `pagemeta.FrontMatterDescriptor` — how to handle the front matter of a page; the
/// PageConfig (its Dates, Params and Slug) is updated.
pub struct FrontMatterDescriptor<'a> {
    /// This is the Page's base filename (BaseFilename), e.g. page.md., or
    /// if page is a leaf bundle, the bundle folder name (ContentBaseName).
    pub base_filename: String,
    /// The Page's path if the page is backed by a file, else its title.
    pub path_or_title: String,
    /// The content file's mod time.
    pub mod_time: Time,
    /// May be set from the author date in Git.
    pub git_author_date: Time,
    /// The below will be modified.
    pub page_config: &'a mut PageConfig,
    /// The Location to use to parse dates without time zone info.
    pub location: Arc<Location>,
}

/// Which of the four dates a handler chain sets.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum DateField {
    Date,
    Lastmod,
    PublishDate,
    ExpiryDate,
}

/// Go's `frontMatterFieldHandler` kinds (`newDate*Handler`).
#[derive(Clone, Debug, PartialEq, Eq)]
enum FieldHandler {
    Filename,
    ModTime,
    GitAuthorDate,
    Field(String),
}

/// Go: `pagemeta.FrontMatterHandler`.
#[derive(Clone)]
pub struct FrontMatterHandler {
    pub fm_config: FrontmatterConfig,
    pub(crate) all_date_keys: BTreeSet<String>,
    date_handler: Vec<FieldHandler>,
    last_mod_handler: Vec<FieldHandler>,
    publish_date_handler: Vec<FieldHandler>,
    expiry_date_handler: Vec<FieldHandler>,
    logger: Logger,
}

impl FrontMatterHandler {
    /// Go: `NewFrontmatterHandler(nil, cfg)` (a default logger).
    // Go: resources/page/pagemeta/page_frontmatter.go:NewFrontmatterHandler
    pub fn new(fm_config: FrontmatterConfig) -> Result<FrontMatterHandler> {
        Self::new_with_logger(None, fm_config)
    }

    /// NewFrontmatterHandler creates a new FrontMatterHandler with the given logger and
    /// configuration. If no logger is provided, one will be created.
    // Go: resources/page/pagemeta/page_frontmatter.go:NewFrontmatterHandler
    pub fn new_with_logger(
        logger: Option<Logger>,
        fm_config: FrontmatterConfig,
    ) -> Result<FrontMatterHandler> {
        let logger = logger.unwrap_or_else(Logger::new_default);

        let mut all_date_keys = BTreeSet::new();
        let mut add_keys = |vals: &[String]| {
            for k in vals {
                if !k.starts_with(':') {
                    all_date_keys.insert(k.clone());
                }
            }
        };

        add_keys(&fm_config.date);
        add_keys(&fm_config.expiry_date);
        add_keys(&fm_config.lastmod);
        add_keys(&fm_config.publish_date);

        let mut f = FrontMatterHandler {
            fm_config,
            all_date_keys,
            date_handler: Vec::new(),
            last_mod_handler: Vec::new(),
            publish_date_handler: Vec::new(),
            expiry_date_handler: Vec::new(),
            logger,
        };

        f.create_handlers()?;

        Ok(f)
    }

    // Go: resources/page/pagemeta/page_frontmatter.go:createHandlers
    fn create_handlers(&mut self) -> Result<()> {
        self.date_handler = Self::create_date_handler(&self.fm_config.date);
        self.last_mod_handler = Self::create_date_handler(&self.fm_config.lastmod);
        self.publish_date_handler = Self::create_date_handler(&self.fm_config.publish_date);
        self.expiry_date_handler = Self::create_date_handler(&self.fm_config.expiry_date);
        Ok(())
    }

    // Go: resources/page/pagemeta/page_frontmatter.go:createDateHandler
    fn create_date_handler(identifiers: &[String]) -> Vec<FieldHandler> {
        identifiers
            .iter()
            .map(|identifier| match identifier.as_str() {
                FM_FILENAME => FieldHandler::Filename,
                FM_MOD_TIME => FieldHandler::ModTime,
                FM_GIT_AUTHOR_DATE => FieldHandler::GitAuthorDate,
                _ => FieldHandler::Field(identifier.clone()),
            })
            .collect()
    }

    /// Go: `HandleDates(d)` — updates all the dates given the current configuration and the
    /// front matter params (all keys must be lower case).
    // Go: resources/page/pagemeta/page_frontmatter.go:HandleDates
    pub fn handle_dates(&self, d: &mut FrontMatterDescriptor<'_>) -> Result<()> {
        if d.page_config.is_from_content_adapter {
            self.content_adapter_dates_handler(d);
            return Ok(());
        }

        self.chained(&self.date_handler, DateField::Date, d)?;
        self.chained(&self.last_mod_handler, DateField::Lastmod, d)?;
        self.chained(&self.publish_date_handler, DateField::PublishDate, d)?;
        self.chained(&self.expiry_date_handler, DateField::ExpiryDate, d)?;

        Ok(())
    }

    /// IsDateKey returns whether the given front matter key is considered a date by the current
    /// configuration.
    // Go: resources/page/pagemeta/page_frontmatter.go:IsDateKey
    pub fn is_date_key(&self, key: &str) -> bool {
        self.all_date_keys.contains(key)
    }

    /// The first successful handler wins; handler errors are logged.
    // Go: resources/page/pagemeta/page_frontmatter.go:newChainedFrontMatterFieldHandler
    fn chained(
        &self,
        handlers: &[FieldHandler],
        field: DateField,
        d: &mut FrontMatterDescriptor<'_>,
    ) -> Result<bool> {
        for h in handlers {
            // First successful handler wins.
            match self.run_handler(h, field, d) {
                Err(err) => self.logger.errorf(err.message()),
                Ok(true) => return Ok(true),
                Ok(false) => {}
            }
        }
        Ok(false)
    }

    fn run_handler(
        &self,
        h: &FieldHandler,
        field: DateField,
        d: &mut FrontMatterDescriptor<'_>,
    ) -> Result<bool> {
        match h {
            FieldHandler::Filename => new_date_filename_handler(field, d),
            FieldHandler::ModTime => new_date_mod_time_handler(field, d),
            FieldHandler::GitAuthorDate => new_date_git_author_date_handler(field, d),
            FieldHandler::Field(key) => new_date_field_handler(key, field, d),
        }
    }

    // Go: resources/page/pagemeta/page_frontmatter.go:createContentAdapterDatesHandler
    fn content_adapter_dates_handler(&self, d: &mut FrontMatterDescriptor<'_>) {
        fn get_time(key: DateField, p: &PageConfig) -> Time {
            match key {
                DateField::Date => p.dates.date.clone(),
                DateField::Lastmod => p.dates.lastmod.clone(),
                DateField::PublishDate => p.dates.publish_date.clone(),
                DateField::ExpiryDate => p.dates.expiry_date.clone(),
            }
        }
        fn set_time(key: DateField, value: Time, p: &mut PageConfig) {
            match key {
                DateField::Date => p.dates.date = value,
                DateField::Lastmod => p.dates.lastmod = value,
                DateField::PublishDate => p.dates.publish_date = value,
                DateField::ExpiryDate => p.dates.expiry_date = value,
            }
        }
        let create_setter = |identifiers: &[String], date: DateField| {
            let get_times: Vec<DateField> = identifiers
                .iter()
                .filter(|i| !i.starts_with(':'))
                .filter_map(|i| match i.as_str() {
                    FM_DATE => Some(DateField::Date),
                    FM_LASTMOD => Some(DateField::Lastmod),
                    FM_PUB_DATE => Some(DateField::PublishDate),
                    FM_EXPIRY_DATE => Some(DateField::ExpiryDate),
                    _ => None,
                })
                .collect();
            move |pcfg: &mut PageConfig| {
                for get in &get_times {
                    let t = get_time(*get, pcfg);
                    if !t.go_is_zero() {
                        set_time(date, t, pcfg);
                        return;
                    }
                }
            }
        };

        let set_date = create_setter(&self.fm_config.date, DateField::Date);
        let set_lastmod = create_setter(&self.fm_config.lastmod, DateField::Lastmod);
        let set_publish_date = create_setter(&self.fm_config.publish_date, DateField::PublishDate);
        let set_expiry_date = create_setter(&self.fm_config.expiry_date, DateField::ExpiryDate);

        let pcfg = &mut *d.page_config;
        set_date(pcfg);
        set_lastmod(pcfg);
        set_publish_date(pcfg);
        set_expiry_date(pcfg);
    }
}

fn params_mut<'a>(d: &'a mut FrontMatterDescriptor<'_>) -> &'a mut Map {
    // Go writes into the page's (non-nil) params map; a nil map would panic there. The port
    // creates it.
    d.page_config
        .params
        .get_or_insert_with(|| Map::new(MapType::Params))
}

// Go: resources/page/pagemeta/page_frontmatter.go:setParamIfNotSet
fn set_param_if_not_set(key: &str, value: Value, d: &mut FrontMatterDescriptor<'_>) {
    let params = params_mut(d);
    if params.get(key.as_bytes()).is_some() {
        return;
    }
    params.insert(key, value);
}

/// The setter closures of Go's `createHandlers`.
fn set_date(field: DateField, d: &mut FrontMatterDescriptor<'_>, t: Time) {
    match field {
        DateField::Date => {
            d.page_config.dates.date = t.clone();
            set_param_if_not_set(FM_DATE, Value::Time(t), d);
        }
        DateField::Lastmod => {
            set_param_if_not_set(FM_LASTMOD, Value::Time(t.clone()), d);
            d.page_config.dates.lastmod = t;
        }
        DateField::PublishDate => {
            set_param_if_not_set(FM_PUB_DATE, Value::Time(t.clone()), d);
            d.page_config.dates.publish_date = t;
        }
        DateField::ExpiryDate => {
            set_param_if_not_set(FM_EXPIRY_DATE, Value::Time(t.clone()), d);
            d.page_config.dates.expiry_date = t;
        }
    }
}

/// dateAndSlugFromBaseFilename returns a time.Time value (resolved to the default system
/// location) and a slug, extracted by parsing the provided path. Parsing supports
/// YYYY-MM-DD-HH-MM-SS and YYYY-MM-DD date/time formats. Within the YYYY-MM-DD-HH-MM-SS format,
/// the date and time values may be separated by any character including a space (e.g.,
/// YYYY-MM-DD HH-MM-SS).
// Go: resources/page/pagemeta/page_frontmatter.go:dateAndSlugFromBaseFilename
fn date_and_slug_from_base_filename(location: &Arc<Location>, path: &str) -> (Time, String) {
    let (base, _) = nh_common::paths::path::file_and_ext(path);
    let base = base.as_bytes();

    if base.len() < 10 {
        // Not long enough to start with a YYYY-MM-DD date.
        return (Time::zero(), String::new());
    }

    // Delimiters allowed between the date and the slug.
    let delimiters = b" -_";

    let trim = |s: &[u8]| -> String {
        String::from_utf8_lossy(go_unicode::strings::trim(s, delimiters)).into_owned()
    };

    if base.len() >= 19 {
        // Attempt to parse a YYYY-MM-DD-HH-MM-SS date-time prefix.
        let ds = &base[..10];
        let ts = go_unicode::strings::replace_all(&base[11..19], b"-", b":");

        let mut s = ds.to_vec();
        s.push(b'T');
        s.extend_from_slice(&ts);
        if let Ok(d) = nh_common::htime::to_time_in_default_location_e(
            &Value::String(GoString::from(s)),
            location,
        ) {
            return (d, trim(&base[19..]));
        }
    }

    // Attempt to parse a YYYY-MM-DD date prefix.
    let ds = &base[..10];

    if let Ok(d) = nh_common::htime::to_time_in_default_location_e(
        &Value::String(GoString::from(ds)),
        location,
    ) {
        return (d, trim(&base[10..]));
    }

    // If no date is defined, return the zero time instant.
    (Time::zero(), String::new())
}

// Go: resources/page/pagemeta/page_frontmatter.go:newDateFieldHandler
fn new_date_field_handler(
    key: &str,
    field: DateField,
    d: &mut FrontMatterDescriptor<'_>,
) -> Result<bool> {
    let v = match d
        .page_config
        .params
        .as_ref()
        .and_then(|p| p.get(key.as_bytes()))
    {
        Some(v) => v.clone(),
        None => return Ok(false),
    };

    // Go: `v == "" || v == nil` (an empty string, or a nil interface).
    match &v {
        Value::String(s) if s.is_empty() => return Ok(false),
        Value::Invalid => return Ok(false),
        _ => {}
    }

    let date = match &v {
        Value::Time(vt) if Arc::ptr_eq(&vt.go_location(), &d.location) => vt.clone(),
        _ => {
            let date = match nh_common::htime::to_time_in_default_location_e(&v, &d.location) {
                Ok(t) => t,
                Err(_) => {
                    return Err(Error::new(format!(
                        "the {} front matter field is not a parsable date: see {}",
                        go_strconv::quote(key.as_bytes()),
                        d.path_or_title
                    )));
                }
            };
            params_mut(d).insert(key, Value::Time(date.clone()));
            date
        }
    };

    // We map several date keys to one, so, for example,
    // "expirydate", "unpublishdate" will all set .ExpiryDate (first found).
    set_date(field, d, date);

    Ok(true)
}

// Go: resources/page/pagemeta/page_frontmatter.go:newDateFilenameHandler
fn new_date_filename_handler(field: DateField, d: &mut FrontMatterDescriptor<'_>) -> Result<bool> {
    let (date, slug) = date_and_slug_from_base_filename(&d.location, &d.base_filename);
    if date.go_is_zero() {
        return Ok(false);
    }

    set_date(field, d, date);

    let has_slug = d
        .page_config
        .params
        .as_ref()
        .is_some_and(|p| p.get(b"slug").is_some());
    if !has_slug {
        // Use slug from filename
        d.page_config.slug = slug;
    }

    Ok(true)
}

// Go: resources/page/pagemeta/page_frontmatter.go:newDateModTimeHandler
fn new_date_mod_time_handler(field: DateField, d: &mut FrontMatterDescriptor<'_>) -> Result<bool> {
    if d.mod_time.go_is_zero() {
        return Ok(false);
    }
    let t = d.mod_time.clone();
    set_date(field, d, t);
    Ok(true)
}

// Go: resources/page/pagemeta/page_frontmatter.go:newDateGitAuthorDateHandler
fn new_date_git_author_date_handler(
    field: DateField,
    d: &mut FrontMatterDescriptor<'_>,
) -> Result<bool> {
    if d.git_author_date.go_is_zero() {
        return Ok(false);
    }
    let t = d.git_author_date.clone();
    set_date(field, d, t);
    Ok(true)
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: resources/page/pagemeta/page_frontmatter.go (864 lines; 20/33 funcs executed)
//   types: DatesStrings, Dates, PageConfigEarly, PageConfig, ResourceConfig, Source, FrontMatterOnlyValues,
//          FrontMatterHandler, FrontMatterDescriptor, frontMatterFieldHandler, FrontmatterConfig,
//          frontmatterFieldHandlers
// OK L58-60: (d Dates) IsDateOrLastModAfter(in Dates) bool
// OK L62-73: (d *Dates) UpdateDateAndLastmodAndPublishDateIfAfter(in Dates)
// OK L75-77: (d Dates) IsAllDatesZero() bool
// OK L134-146: ClonePageConfigForRebuild(p *PageConfig, params map[string]any) *PageConfig
// OK L152-175: (p *PageConfig) Init(pagesFromData bool) error
// OK L177-208: (p *PageConfig) CompileForPagesFromDataPre(basePath string, logger loggers.Logger, mediaTypes media.Types) error
// OK L210-242: (p *PageConfig) compilePrePost(ext string, mediaTypes media.Types) error
// OK L245-275: (p *PageConfig) Compile(ext string, logger loggers.Logger, outputFormats output.Formats, mediaTypes media.Types) error
// OK L278-282: MarkupToMediaType(s string, mediaTypes media.Types) media.Type
// OK L296-301: (rc *ResourceConfig) Validate() error
// OK L303-322: (rc *ResourceConfig) Compile(basePath string, pathParser *paths.PathParser, mediaTypes media.Types) error
// OK L335-337: (s Source) IsZero() bool
// OK L339-342: (s Source) IsResourceValue() bool
// OK L344-353: (s Source) ValueAsString() string
// OK L355-357: (s Source) ValueAsOpenReadSeekCloser() hugio.OpenReadSeekCloser
// OK L415-448: (f FrontMatterHandler) HandleDates(d *FrontMatterDescriptor) error
// OK L452-454: (f FrontMatterHandler) IsDateKey(key string) bool
// OK L461-493: dateAndSlugFromBaseFilename(location *time.Location, path string) (time.Time, string)
// OK L497-510: (f FrontMatterHandler) newChainedFrontMatterFieldHandler(handlers ...frontMatterFieldHandler) frontMatterFieldHandler
// OK L542-549: newDefaultFrontmatterConfig() FrontmatterConfig
// OK L551-584: DecodeFrontMatterConfig(cfg config.Provider) (FrontmatterConfig, error)
// OK L586-596: addDateFieldAliases(values []string) []string
// OK L598-608: expandDefaultValues(values []string, defaults []string) []string
// OK L610-617: toLowerSlice(in any) []string
// OK L621-647: NewFrontmatterHandler(logger loggers.Logger, frontMatterConfig FrontmatterConfig) (FrontMatterHandler, error)
// OK L649-689: (f *FrontMatterHandler) createHandlers() error
// OK L691-696: setParamIfNotSet(key string, value any, d *FrontMatterDescriptor)
// OK L698-776: (f FrontMatterHandler) createContentAdapterDatesHandler(fmcfg FrontmatterConfig) (func(d *FrontMatterDescriptor) error, error)
// OK L778-796: (f FrontMatterHandler) createDateHandler(identifiers []string, setter func(d *FrontMatterDescriptor, t time.Time)) (frontMatterFieldHandler, error)
// OK L800-826: (f *frontmatterFieldHandlers) newDateFieldHandler(key string, setter func(d *FrontMatterDescriptor, t time.Time)) frontMatterFieldHandler
// OK L828-844: (f *frontmatterFieldHandlers) newDateFilenameHandler(setter func(d *FrontMatterDescriptor, t time.Time)) frontMatterFieldHandler
// OK L846-854: (f *frontmatterFieldHandlers) newDateModTimeHandler(setter func(d *FrontMatterDescriptor, t time.Time)) frontMatterFieldHandler
// OK L856-864: (f *frontmatterFieldHandlers) newDateGitAuthorDateHandler(setter func(d *FrontMatterDescriptor, t time.Time)) frontMatterFieldHandler
// ---------------------------------------------------------------------------
