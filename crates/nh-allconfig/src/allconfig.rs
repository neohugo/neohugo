//! Port of `config/allconfig/allconfig.go`.
//!
//! Owner: Wave B task T09 (allconfig-modules).

//! Go `config/allconfig/allconfig.go`: the fully decoded site configuration, compiled per
//! language. Oracle: `tools/go-oracle/nh-allconfig/load` (the decoded sections of every
//! language config, `neohugo config --format json [--lang th]`, the compiled values).

use std::borrow::Cow;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Arc, Mutex, RwLock};

use go_value::{Map, MapType, Time, Value};
use nh_common::loggers::Logger;
use nh_common::urls::BaseURL;
use nh_common::{Error, Result};
use nh_config::common_config::{
    BuildConfig, CommonDirs, LoadConfigResult, PageConfig, Pagination, Server, SitemapConfig,
};
use nh_config::config_provider::{AllProvider, Provider};
use nh_config::decode::{DResult, Decode, DecodeError, Decoder, FieldRef, OutKind};
use nh_config::decode_struct;
use nh_config::namespace::ConfigNamespace;
use nh_config::security::security_config::Config as SecurityConfig;
use nh_helpers::cache::filecache::filecache_config::Configs as FileCacheConfigs;
use nh_helpers::cache::httpcache::httpcache::{
    Config as HttpCacheConfig, ConfigCompiled as HttpCacheConfigCompiled,
};
use nh_hugofs::afero::Fs;
use nh_hugofs::modules::client::Client as ModulesClient;
use nh_hugofs::modules::config::Config as ModuleConfig;
use nh_hugofs::modules::module::Modules;
use nh_images::config::{ImagingConfig, ImagingConfigInternal};
use nh_langs::config::LanguageConfig;
use nh_langs::language::{Language, Languages};
use nh_markup::goldmark::goldmark_config::{
    RENDER_HOOK_USE_EMBEDDED_ALWAYS, RENDER_HOOK_USE_EMBEDDED_AUTO,
    RENDER_HOOK_USE_EMBEDDED_FALLBACK, RENDER_HOOK_USE_EMBEDDED_NEVER,
};
use nh_markup::markup_config::Config as MarkupConfig;
use nh_media::media::config::ContentTypes;
use nh_media::media::media_type::Types as MediaTypes;
use nh_media::output::output_format::{Formats, OutputFormat};
use nh_page::navigation::menu::Menu;
use nh_page::page_matcher::{Cascade, PageMatcherParamsConfig};
use nh_page::pagemeta::page_frontmatter::FrontmatterConfig;
use nh_page::related::Config as RelatedConfig;
use nh_transform::minifiers::config::MinifyConfig;

use crate::alldecoders::{DecodeConfig, all_decoder_setups};
use crate::configlanguage::ConfigLanguage;
use crate::deployconfig::DeployConfig;
use crate::segments::{SegmentFilter, Segments};

// ---------------------------------------------------------------------------
// Go slices with nil (a nil and an empty slice differ in Hugo's config: `[]` vs `null` in the
// JSON dump, `renderSegments`, `mainSections`).

/// A Go `[]T` that keeps nil apart from empty (`None` = nil).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GoSlice<T>(pub Option<Vec<T>>);

impl<T> Default for GoSlice<T> {
    fn default() -> Self {
        GoSlice(None)
    }
}

impl<T> GoSlice<T> {
    /// A nil slice.
    pub fn nil() -> Self {
        GoSlice(None)
    }

    /// A non-nil slice.
    pub fn from_vec(v: Vec<T>) -> Self {
        GoSlice(Some(v))
    }

    pub fn is_nil(&self) -> bool {
        self.0.is_none()
    }

    pub fn as_slice(&self) -> &[T] {
        self.0.as_deref().unwrap_or(&[])
    }

    pub fn len(&self) -> usize {
        self.as_slice().len()
    }

    pub fn is_empty(&self) -> bool {
        self.as_slice().is_empty()
    }

    pub fn iter(&self) -> std::slice::Iter<'_, T> {
        self.as_slice().iter()
    }
}

impl<T: Decode + Default + 'static> Decode for GoSlice<T> {
    fn go_type(&self) -> Cow<'static, str> {
        Cow::Owned(format!("[]{}", T::default().go_type()))
    }
    fn out_kind(&self) -> OutKind {
        OutKind::Slice
    }
    fn decode_kind(&mut self, d: &Decoder<'_>, name: &str, data: &Value) -> DResult {
        // mapstructure only calls decodeSlice for non-nil data; the slice it sets is non-nil
        // (an empty map becomes an empty slice).
        let mut v = self.0.take().unwrap_or_default();
        let r = v.decode_kind(d, name, data);
        self.0 = Some(v);
        r
    }
    fn set_zero(&mut self) {
        self.0 = None;
    }
    fn is_zero_value(&self) -> bool {
        self.0.is_none()
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

// ---------------------------------------------------------------------------

/// Go: `allconfig.InternalConfig` (set from CLI flags; `clock` from `--clock`).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct InternalConfig {
    /// Server mode?
    pub running: bool,
    pub quiet: bool,
    pub verbose: bool,
    pub clock: String,
    pub watch: bool,
    pub fast_render_mode: bool,
    pub live_reload_port: i64,
}

decode_struct!(InternalConfig, "allconfig.InternalConfig", |s| vec![
    FieldRef::new("Running", &mut s.running),
    FieldRef::new("Quiet", &mut s.quiet),
    FieldRef::new("Verbose", &mut s.verbose),
    FieldRef::new("Clock", &mut s.clock),
    FieldRef::new("Watch", &mut s.watch),
    FieldRef::new("FastRenderMode", &mut s.fast_render_mode),
    FieldRef::new("LiveReloadPort", &mut s.live_reload_port),
]);

/// Go: `allconfig.RootConfig` (top-level keys, WeakDecoded).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct RootConfig {
    pub base_url: String,
    pub build_drafts: bool,
    pub build_expired: bool,
    pub build_future: bool,
    pub copyright: String,
    pub default_content_language: String,
    pub default_content_language_in_subdir: bool,
    pub default_output_format: String,
    pub disable_default_language_redirect: bool,
    pub disable_aliases: bool,
    pub disable_path_to_lower: bool,
    pub disable_kinds: GoSlice<String>,
    pub disable_languages: GoSlice<String>,
    pub render_segments: GoSlice<String>,
    /// NOTE neohugo inverted semantics: hugolib sets AddHugoGeneratorTag = this (false) -> never injected.
    pub disable_hugo_generator_inject: bool,
    pub disable_live_reload: bool,
    pub enable_emoji: bool,
    pub main_sections: GoSlice<String>,
    pub enable_robots_txt: bool,
    pub enable_git_info: bool,
    pub template_metrics: bool,
    pub template_metrics_hints: bool,
    pub no_build_lock: bool,
    pub ignore_logs: GoSlice<String>,
    pub ignore_files: GoSlice<String>,
    pub ignore_cache: bool,
    pub enable_missing_translation_placeholders: bool,
    pub panic_on_warning: bool,
    pub environment: String,
    pub language_code: String,
    pub has_cjk_language: bool,
    pub paginate: i64,
    pub paginate_path: String,
    pub pluralize_list_titles: bool,
    pub capitalize_list_titles: bool,
    pub canonify_urls: bool,
    pub relative_urls: bool,
    pub remove_path_accents: bool,
    pub print_unused_templates: bool,
    pub print_i18n_warnings: bool,
    pub print_path_warnings: bool,
    pub ref_links_not_found_url: String,
    pub ref_links_error_level: String,
    pub section_pages_menu: String,
    pub summary_length: i64,
    pub title: String,
    pub theme: GoSlice<String>,
    pub timeout: String,
    pub time_zone: String,
    pub title_case_style: String,
    pub new_content_editor: String,
    pub no_times: bool,
    pub no_chmod: bool,
    pub clean_destination_dir: bool,
    pub ignore_vendor_paths: String,
    pub common_dirs: CommonDirs,
    pub static_dir: GoSlice<String>,
    /// staticDir0..staticDir10.
    pub static_dir_n: [GoSlice<String>; 11],
}

decode_struct!(RootConfig, "allconfig.RootConfig", |s| {
    let [s0, s1, s2, s3, s4, s5, s6, s7, s8, s9, s10] = &mut s.static_dir_n;
    vec![
        FieldRef::new("BaseURL", &mut s.base_url),
        FieldRef::new("BuildDrafts", &mut s.build_drafts),
        FieldRef::new("BuildExpired", &mut s.build_expired),
        FieldRef::new("BuildFuture", &mut s.build_future),
        FieldRef::new("Copyright", &mut s.copyright),
        FieldRef::new("DefaultContentLanguage", &mut s.default_content_language),
        FieldRef::new(
            "DefaultContentLanguageInSubdir",
            &mut s.default_content_language_in_subdir,
        ),
        FieldRef::new("DefaultOutputFormat", &mut s.default_output_format),
        FieldRef::new(
            "DisableDefaultLanguageRedirect",
            &mut s.disable_default_language_redirect,
        ),
        FieldRef::new("DisableAliases", &mut s.disable_aliases),
        FieldRef::new("DisablePathToLower", &mut s.disable_path_to_lower),
        FieldRef::new("DisableKinds", &mut s.disable_kinds),
        FieldRef::new("DisableLanguages", &mut s.disable_languages),
        FieldRef::new("RenderSegments", &mut s.render_segments),
        FieldRef::new(
            "DisableHugoGeneratorInject",
            &mut s.disable_hugo_generator_inject,
        ),
        FieldRef::new("DisableLiveReload", &mut s.disable_live_reload),
        FieldRef::new("EnableEmoji", &mut s.enable_emoji),
        FieldRef::new("MainSections", &mut s.main_sections),
        FieldRef::new("EnableRobotsTXT", &mut s.enable_robots_txt),
        FieldRef::new("EnableGitInfo", &mut s.enable_git_info),
        FieldRef::new("TemplateMetrics", &mut s.template_metrics),
        FieldRef::new("TemplateMetricsHints", &mut s.template_metrics_hints),
        FieldRef::new("NoBuildLock", &mut s.no_build_lock),
        FieldRef::new("IgnoreLogs", &mut s.ignore_logs),
        FieldRef::new("IgnoreFiles", &mut s.ignore_files),
        FieldRef::new("IgnoreCache", &mut s.ignore_cache),
        FieldRef::new(
            "EnableMissingTranslationPlaceholders",
            &mut s.enable_missing_translation_placeholders,
        ),
        FieldRef::new("PanicOnWarning", &mut s.panic_on_warning),
        FieldRef::new("Environment", &mut s.environment),
        FieldRef::new("LanguageCode", &mut s.language_code),
        FieldRef::new("HasCJKLanguage", &mut s.has_cjk_language),
        FieldRef::new("Paginate", &mut s.paginate),
        FieldRef::new("PaginatePath", &mut s.paginate_path),
        FieldRef::new("PluralizeListTitles", &mut s.pluralize_list_titles),
        FieldRef::new("CapitalizeListTitles", &mut s.capitalize_list_titles),
        FieldRef::new("CanonifyURLs", &mut s.canonify_urls),
        FieldRef::new("RelativeURLs", &mut s.relative_urls),
        FieldRef::new("RemovePathAccents", &mut s.remove_path_accents),
        FieldRef::new("PrintUnusedTemplates", &mut s.print_unused_templates),
        FieldRef::new("PrintI18nWarnings", &mut s.print_i18n_warnings),
        FieldRef::new("PrintPathWarnings", &mut s.print_path_warnings),
        FieldRef::new("RefLinksNotFoundURL", &mut s.ref_links_not_found_url),
        FieldRef::new("RefLinksErrorLevel", &mut s.ref_links_error_level),
        FieldRef::new("SectionPagesMenu", &mut s.section_pages_menu),
        FieldRef::new("SummaryLength", &mut s.summary_length),
        FieldRef::new("Title", &mut s.title),
        FieldRef::new("Theme", &mut s.theme),
        FieldRef::new("Timeout", &mut s.timeout),
        FieldRef::new("TimeZone", &mut s.time_zone),
        FieldRef::new("TitleCaseStyle", &mut s.title_case_style),
        FieldRef::new("NewContentEditor", &mut s.new_content_editor),
        FieldRef::new("NoTimes", &mut s.no_times),
        FieldRef::new("NoChmod", &mut s.no_chmod),
        FieldRef::new("CleanDestinationDir", &mut s.clean_destination_dir),
        FieldRef::new("IgnoreVendorPaths", &mut s.ignore_vendor_paths),
        FieldRef::squash("CommonDirs", &mut s.common_dirs),
        FieldRef::new("StaticDir", &mut s.static_dir),
        FieldRef::new("StaticDir0", s0),
        FieldRef::new("StaticDir1", s1),
        FieldRef::new("StaticDir2", s2),
        FieldRef::new("StaticDir3", s3),
        FieldRef::new("StaticDir4", s4),
        FieldRef::new("StaticDir5", s5),
        FieldRef::new("StaticDir6", s6),
        FieldRef::new("StaticDir7", s7),
        FieldRef::new("StaticDir8", s8),
        FieldRef::new("StaticDir9", s9),
        FieldRef::new("StaticDir10", s10),
    ]
});

impl RootConfig {
    // Go: config/allconfig/allconfig.go:staticDirs
    pub fn static_dirs(&self) -> Vec<String> {
        let mut dirs: Vec<String> = Vec::new();
        dirs.extend(self.static_dir.iter().cloned());
        for d in &self.static_dir_n {
            dirs.extend(d.iter().cloned());
        }
        nh_helpers::general::unique_strings_reuse(dirs)
    }
}

/// Go: `Config.UglyURLs any` (a bool or a sections map; nil when not set).
#[derive(Clone, Debug, Default, PartialEq)]
pub enum UglyUrls {
    #[default]
    Nil,
    Bool(bool),
    Sections(BTreeMap<String, bool>),
}

/// The `languages` map of a `Config`. Go's `cloneForLang` copies the struct, so the map is
/// shared with the base config until a clone decodes its own; `CompileConfig` writes the
/// `Disabled` flags into it.
#[derive(Clone, Debug, Default)]
pub struct LanguagesMap(pub Arc<RwLock<BTreeMap<String, LanguageConfig>>>);

impl LanguagesMap {
    pub fn new(m: BTreeMap<String, LanguageConfig>) -> Self {
        LanguagesMap(Arc::new(RwLock::new(m)))
    }

    /// A snapshot of the map.
    pub fn get(&self) -> BTreeMap<String, LanguageConfig> {
        self.0.read().unwrap_or_else(|e| e.into_inner()).clone()
    }

    pub fn get_lang(&self, lang: &str) -> Option<LanguageConfig> {
        self.0
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .get(lang)
            .cloned()
    }

    pub fn contains_key(&self, lang: &str) -> bool {
        self.0
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .contains_key(lang)
    }

    fn set(&self, lang: &str, l: LanguageConfig) {
        self.0
            .write()
            .unwrap_or_else(|e| e.into_inner())
            .insert(lang.to_string(), l);
    }
}

/// Go: `allconfig.ConfigCompiled`.
pub struct ConfigCompiled {
    /// Go `time.Duration` (may be negative).
    pub timeout: go_time::Duration,
    pub base_url: BaseURL,
    pub base_url_live_reload: BaseURL,
    pub server_interface: String,
    /// kind -> output formats.
    pub kind_output_formats: BTreeMap<String, Formats>,
    pub default_output_format: OutputFormat,
    pub disabled_kinds: BTreeSet<String>,
    pub disabled_languages: BTreeSet<String>,
    pub ignored_logs: BTreeSet<String>,
    pub create_title: Arc<dyn Fn(&str) -> String + Send + Sync>,
    /// `create_title` over Go string bytes (invalid UTF-8 included).
    pub create_title_bytes: nh_helpers::general::TitleBytesFunc,
    pub is_ugly_url_section: Arc<dyn Fn(&str) -> bool + Send + Sync>,
    pub ignore_file: Arc<dyn Fn(&str) -> bool + Send + Sync>,
    pub segment_filter: SegmentFilter,
    /// Go `MainSections` (nil until set; `SetMainSections` may set it after compilation).
    pub main_sections: Mutex<Option<Vec<String>>>,
    /// From `--clock` (zero if unset).
    pub clock: Time,
    pub http_cache: HttpCacheConfigCompiled,
    /// The last transient error found during config compilation (an unknown output format:
    /// with themes/modules the config is computed in several passes).
    pub(crate) transient_err: Option<String>,
}

impl ConfigCompiled {
    /// This may be set after the config is compiled.
    // Go: config/allconfig/allconfig.go:SetMainSections
    pub fn set_main_sections(&self, sections: Vec<String>) {
        *self.main_sections.lock().unwrap_or_else(|e| e.into_inner()) = Some(sections);
    }

    /// IsMainSectionsSet returns whether the main sections have been set.
    // Go: config/allconfig/allconfig.go:IsMainSectionsSet
    pub fn is_main_sections_set(&self) -> bool {
        self.main_sections
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .is_some()
    }

    /// The main sections (Go reads the field; nil = `None`).
    pub fn main_sections(&self) -> Option<Vec<String>> {
        self.main_sections
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }

    /// This is set after the config is compiled by the server command (not supported: the
    /// port has no server; kept for the API).
    // Go: config/allconfig/allconfig.go:SetServerInfo
    pub fn set_server_info(
        &mut self,
        base_url: BaseURL,
        base_url_live_reload: BaseURL,
        server_interface: &str,
    ) {
        self.base_url = base_url;
        self.base_url_live_reload = base_url_live_reload;
        self.server_interface = server_interface.to_string();
    }
}

/// Go: `allconfig.Config`.
#[derive(Clone)]
pub struct Config {
    pub internal: InternalConfig,
    pub c: Option<Arc<ConfigCompiled>>,
    pub root: RootConfig,
    /// Deprecated: Use taxonomies instead. (`None` = Go's nil map.)
    pub author: Option<Map>,
    /// Deprecated: Use .Site.Params instead. (`map[string]string`; `None` = nil.)
    pub social: Option<Map>,
    pub build: BuildConfig,
    pub caches: FileCacheConfigs,
    pub http_cache: HttpCacheConfig,
    pub markup: MarkupConfig,
    pub content_types: Option<Arc<ConfigNamespace<Map, ContentTypes>>>,
    pub media_types: Option<Arc<ConfigNamespace<Map, MediaTypes>>>,
    pub imaging: Option<Arc<ConfigNamespace<ImagingConfig, ImagingConfigInternal>>>,
    pub output_formats: Option<Arc<ConfigNamespace<Map, Formats>>>,
    pub outputs: BTreeMap<String, Vec<String>>,
    pub cascade: Option<Arc<ConfigNamespace<Vec<PageMatcherParamsConfig>, Cascade>>>,
    pub segments: Option<Arc<ConfigNamespace<Map, Segments>>>,
    pub menus: Option<Arc<ConfigNamespace<Map, BTreeMap<String, Menu>>>>,
    pub deployment: DeployConfig,
    pub module: ModuleConfig,
    pub frontmatter: FrontmatterConfig,
    pub minify: MinifyConfig,
    pub permalinks: BTreeMap<String, BTreeMap<String, String>>,
    pub taxonomies: BTreeMap<String, String>,
    pub sitemap: SitemapConfig,
    pub related: RelatedConfig,
    pub server: Server,
    pub pagination: Pagination,
    pub page: PageConfig,
    pub privacy: nh_config::privacy::Config,
    pub security: SecurityConfig,
    pub services: nh_config::services::Config,
    /// `maps.Params` (lower-cased, `_merge` keys removed).
    pub params: Arc<Map>,
    pub languages: LanguagesMap,
    pub ugly_urls: UglyUrls,
    /// The imaging namespace's source structure is the config tree's own map in Go (the
    /// defaults merged into it, its `_merge` keys deleted when LoadConfig returns).
    pub(crate) imaging_live: bool,
    /// The segments namespace's source structure is the config tree's own map in Go (its
    /// `_merge` keys deleted when LoadConfig returns).
    pub(crate) segments_live: bool,
}

impl Config {
    /// The compiled config (Go `c.C`; set by `CompileConfig`).
    pub fn compiled(&self) -> &Arc<ConfigCompiled> {
        self.c.as_ref().expect("config is compiled")
    }

    /// Go: `CompileConfig(logger)` (derives `C`).
    // Go: config/allconfig/allconfig.go:CompileConfig
    pub fn compile_config(&mut self) -> Result<()> {
        self.compile_config_with_logger(&nh_common::loggers::log())
    }

    // Go: config/allconfig/allconfig.go:CompileConfig
    pub fn compile_config_with_logger(&mut self, logger: &Logger) -> Result<()> {
        let mut transient_err: Option<String> = None;
        let mut s = self.root.timeout.clone();
        if go_strconv::atoi(&s).is_ok() {
            // A number, assume seconds.
            s.push('s');
        }
        let timeout = match go_time::parse_duration(&s) {
            Ok(d) => d,
            Err(err) => return Err(Error::new(format!("failed to parse timeout: {err}"))),
        };
        let mut disabled_kinds: BTreeSet<String> = BTreeSet::new();
        for kind in self.root.disable_kinds.iter() {
            let mut kind = go_unicode::strings::to_lower_str(kind).into_owned();
            let new_kind = nh_common::kinds::is_deprecated_and_replaced_with(&kind);
            if !new_kind.is_empty() {
                logger.deprecatef(
                    false,
                    format!(
                        "Kind {} used in disableKinds is deprecated, use {} instead.",
                        quote(&kind),
                        quote(new_kind)
                    ),
                );
                // Legacy config.
                kind = new_kind.to_string();
            }
            if nh_common::kinds::get_kind_any(&kind).is_empty() {
                logger.warnf(format!(
                    "Unknown kind {} in disableKinds configuration.",
                    quote(&kind)
                ));
                continue;
            }
            disabled_kinds.insert(kind);
        }
        let mut kind_output_formats: BTreeMap<String, Formats> = BTreeMap::new();
        let is_rss_disabled = disabled_kinds.contains("rss");
        let output_formats = &self
            .output_formats
            .as_ref()
            .expect("output formats decoded")
            .config;
        // Go iterates the outputs map in random order; only which transient error is kept can
        // differ (the last one in iteration order).
        for (kind, formats) in &self.outputs {
            let mut kind = kind.clone();
            let new_kind = nh_common::kinds::is_deprecated_and_replaced_with(&kind);
            if !new_kind.is_empty() {
                logger.deprecatef(
                    false,
                    format!(
                        "Kind {} used in outputs configuration is deprecated, use {} instead.",
                        quote(&kind),
                        quote(new_kind)
                    ),
                );
                kind = new_kind.to_string();
            }
            if disabled_kinds.contains(&kind) {
                continue;
            }
            if nh_common::kinds::get_kind_any(&kind).is_empty() {
                logger.warnf(format!(
                    "Unknown kind {} in outputs configuration.",
                    quote(&kind)
                ));
                continue;
            }
            for format in formats {
                if is_rss_disabled && format == "rss" {
                    // Legacy config.
                    continue;
                }
                let Some(f) = output_formats.get_by_name(format) else {
                    transient_err = Some(format!(
                        "unknown output format {} for kind {}",
                        quote(format),
                        quote(&kind)
                    ));
                    continue;
                };
                kind_output_formats
                    .entry(kind.clone())
                    .or_default()
                    .0
                    .push(f);
            }
        }

        let mut default_output_format = output_formats.0[0].clone();
        self.root.default_output_format =
            go_unicode::strings::to_lower_str(&self.root.default_output_format).into_owned();
        if !self.root.default_output_format.is_empty() {
            match output_formats.get_by_name(&self.root.default_output_format) {
                Some(f) => default_output_format = f,
                None => {
                    return Err(Error::new(format!(
                        "unknown default output format {}",
                        quote(&self.root.default_output_format)
                    )));
                }
            }
        } else {
            self.root.default_output_format = default_output_format.name.clone();
        }

        let mut disabled_langs: BTreeSet<String> = BTreeSet::new();
        for lang in self.root.disable_languages.iter() {
            disabled_langs.insert(lang.clone());
        }
        // Go iterates the languages map in random order; the error below is the same for any.
        for (lang, mut language) in self.languages.get() {
            if !language.disabled && disabled_langs.contains(&lang) {
                language.disabled = true;
                self.languages.set(&lang, language.clone());
            }
            if language.disabled {
                disabled_langs.insert(lang.clone());
                if lang == self.root.default_content_language {
                    return Err(Error::new(format!(
                        "cannot disable default content language {}",
                        quote(&lang)
                    )));
                }
            }
        }

        if let Some(ignore_logs) = self.root.ignore_logs.0.as_mut() {
            for s in ignore_logs.iter_mut() {
                *s = go_unicode::strings::to_lower_str(s).into_owned();
            }
        }

        let mut ignored_log_ids: BTreeSet<String> = BTreeSet::new();
        for err in self.root.ignore_logs.iter() {
            ignored_log_ids.insert(err.clone());
        }

        let base_url = nh_common::urls::new_base_url_from_string(&self.root.base_url)?;

        let ugly = self.ugly_urls.clone();
        let is_ugly_url: Arc<dyn Fn(&str) -> bool + Send + Sync> =
            Arc::new(move |section: &str| match &ugly {
                UglyUrls::Bool(v) => *v,
                UglyUrls::Sections(v) => v.get(section).copied().unwrap_or(false),
                UglyUrls::Nil => false,
            });

        let mut ignore_file: Arc<dyn Fn(&str) -> bool + Send + Sync> = Arc::new(|_s: &str| false);
        if !self.root.ignore_files.is_empty() {
            let mut regexps = Vec::with_capacity(self.root.ignore_files.len());
            for pattern in self.root.ignore_files.iter() {
                match nh_config::goregexp::Regexp::compile(pattern) {
                    Ok(re) => regexps.push(re),
                    Err(err) => {
                        return Err(Error::new(format!(
                            "failed to compile ignoreFiles pattern {}: {}",
                            quote(pattern),
                            err
                        )));
                    }
                }
            }
            ignore_file = Arc::new(move |s: &str| regexps.iter().any(|r| r.match_string(s)));
        }

        let mut clock = Time::zero();
        if !self.internal.clock.is_empty() {
            match go_time::parse(go_time::RFC3339, &self.internal.clock) {
                Ok(t) => clock = t,
                Err(err) => return Err(Error::new(format!("failed to parse clock: {err}"))),
            }
        }

        let http_cache = self.http_cache.compile()?;

        // Legacy paginate values.
        if self.root.paginate != 0 {
            deprecate_with_logger(
                "site config key paginate",
                "Use pagination.pagerSize instead.",
                "v0.128.0",
                logger,
            );
            self.pagination.pager_size = self.root.paginate;
        }

        if !self.root.paginate_path.is_empty() {
            deprecate_with_logger(
                "site config key paginatePath",
                "Use pagination.path instead.",
                "v0.128.0",
                logger,
            );
            self.pagination.path = self.root.paginate_path.clone();
        }

        // Legacy privacy values.
        if self.privacy.twitter.service.disable {
            deprecate_with_logger(
                "site config key privacy.twitter.disable",
                "Use privacy.x.disable instead.",
                "v0.141.0",
                logger,
            );
            self.privacy.x.service.disable = self.privacy.twitter.service.disable;
        }
        if self.privacy.twitter.enable_dnt {
            deprecate_with_logger(
                "site config key privacy.twitter.enableDNT",
                "Use privacy.x.enableDNT instead.",
                "v0.141.0",
                logger,
            );
            self.privacy.x.enable_dnt = self.privacy.twitter.enable_dnt;
        }
        if self.privacy.twitter.simple {
            deprecate_with_logger(
                "site config key privacy.twitter.simple",
                "Use privacy.x.simple instead.",
                "v0.141.0",
                logger,
            );
            self.privacy.x.simple = self.privacy.twitter.simple;
        }

        // Legacy services values.
        if self.services.twitter.disable_inline_css {
            deprecate_with_logger(
                "site config key services.twitter.disableInlineCSS",
                "Use services.x.disableInlineCSS instead.",
                "v0.141.0",
                logger,
            );
            self.services.x.disable_inline_css = self.services.twitter.disable_inline_css;
        }

        // Legacy permalink tokens (Go: strings.Contains(fmt.Sprintf("%v", c.Permalinks), ...)).
        let permalinks_contain = |needle: &str| {
            self.permalinks.iter().any(|(k, m)| {
                k.contains(needle)
                    || m.iter()
                        .any(|(kk, v)| kk.contains(needle) || v.contains(needle))
            })
        };
        if permalinks_contain(":filename") {
            deprecate_with_logger(
                "the \":filename\" permalink token",
                "Use \":contentbasename\" instead.",
                "0.144.0",
                logger,
            );
        }
        if permalinks_contain(":slugorfilename") {
            deprecate_with_logger(
                "the \":slugorfilename\" permalink token",
                "Use \":slugorcontentbasename\" instead.",
                "0.144.0",
                logger,
            );
        }

        // Legacy render hook values.
        let alternative_details = format!(
            "Set to {} if previous value was false, or set to {} if previous value was true.",
            quote(RENDER_HOOK_USE_EMBEDDED_NEVER),
            quote(RENDER_HOOK_USE_EMBEDDED_FALLBACK),
        );
        let hooks = &mut self.markup.goldmark.render_hooks;
        if let Some(enable_default) = hooks.image.enable_default.as_deref().copied() {
            let alternative = format!(
                "Use markup.goldmark.renderHooks.image.useEmbedded instead. {alternative_details}"
            );
            deprecate_with_logger(
                "site config key markup.goldmark.renderHooks.image.enableDefault",
                &alternative,
                "0.148.0",
                logger,
            );
            hooks.image.use_embedded = if enable_default {
                RENDER_HOOK_USE_EMBEDDED_FALLBACK
            } else {
                RENDER_HOOK_USE_EMBEDDED_NEVER
            }
            .to_string();
        }
        if let Some(enable_default) = hooks.link.enable_default.as_deref().copied() {
            let alternative = format!(
                "Use markup.goldmark.renderHooks.link.useEmbedded instead. {alternative_details}"
            );
            deprecate_with_logger(
                "site config key markup.goldmark.renderHooks.link.enableDefault",
                &alternative,
                "0.148.0",
                logger,
            );
            hooks.link.use_embedded = if enable_default {
                RENDER_HOOK_USE_EMBEDDED_FALLBACK
            } else {
                RENDER_HOOK_USE_EMBEDDED_NEVER
            }
            .to_string();
        }

        // Validate render hook configuration.
        let render_hook_use_embedded_modes: Vec<String> = [
            RENDER_HOOK_USE_EMBEDDED_ALWAYS,
            RENDER_HOOK_USE_EMBEDDED_AUTO,
            RENDER_HOOK_USE_EMBEDDED_FALLBACK,
            RENDER_HOOK_USE_EMBEDDED_NEVER,
        ]
        .iter()
        .map(|s| s.to_string())
        .collect();
        if !render_hook_use_embedded_modes.contains(&hooks.image.use_embedded) {
            return Err(Error::new(format!(
                "site config markup.goldmark.renderHooks.image must be one of {}",
                nh_helpers::general::string_slice_to_list(&render_hook_use_embedded_modes, "or")
            )));
        }
        if !render_hook_use_embedded_modes.contains(&hooks.link.use_embedded) {
            return Err(Error::new(format!(
                "site config markup.goldmark.renderHooks.link must be one of {}",
                nh_helpers::general::string_slice_to_list(&render_hook_use_embedded_modes, "or")
            )));
        }

        let segment_filter = {
            let segments = &self.segments.as_ref().expect("segments decoded").config;
            let logger = logger.clone();
            let on_not_found = move |s: &str| {
                logger.warnf(format!(
                    "Render segment {} not found in configuration",
                    quote(s)
                ))
            };
            segments.get(Some(&on_not_found), self.root.render_segments.0.as_deref())
        };

        let create_title = nh_helpers::general::get_title_func(&self.root.title_case_style);
        let create_title_bytes =
            nh_helpers::general::get_title_bytes_func(&self.root.title_case_style);

        self.c = Some(Arc::new(ConfigCompiled {
            timeout,
            base_url: base_url.clone(),
            base_url_live_reload: base_url,
            server_interface: String::new(),
            disabled_kinds,
            disabled_languages: disabled_langs,
            ignored_logs: ignored_log_ids,
            kind_output_formats,
            default_output_format,
            create_title,
            create_title_bytes,
            is_ugly_url_section: is_ugly_url,
            ignore_file,
            segment_filter,
            main_sections: Mutex::new(self.root.main_sections.0.clone()),
            clock,
            http_cache,
            transient_err,
        }));

        for s in all_decoder_setups() {
            if let Some(get_compiler) = s.compile {
                get_compiler(self, logger)?;
            }
        }

        Ok(())
    }

    // Go: config/allconfig/allconfig.go:IsKindEnabled
    pub fn is_kind_enabled(&self, kind: &str) -> bool {
        !self.compiled().disabled_kinds.contains(kind)
    }

    // Go: config/allconfig/allconfig.go:IsLangDisabled
    pub fn is_lang_disabled(&self, lang: &str) -> bool {
        self.compiled().disabled_languages.contains(lang)
    }

    // Go: config/allconfig/allconfig.go:cloneForLang
    pub(crate) fn clone_for_lang(&self) -> Config {
        let mut x = self.clone();
        x.c = None;

        // Collapse all static dirs to one.
        x.root.static_dir = GoSlice::from_vec(x.root.static_dirs());
        // Go: an empty result of staticDirs() is a nil slice.
        if x.root.static_dir.is_empty() {
            x.root.static_dir = GoSlice::nil();
        }
        // These will go away soon ...
        for d in x.root.static_dir_n.iter_mut() {
            *d = GoSlice::nil();
        }

        x
    }
}

/// Go: `neohugo.DeprecateWithLogger(item, alternative, version, logger)`.
// Go: common/neohugo/neohugo.go:DeprecateWithLogger
pub(crate) fn deprecate_with_logger(item: &str, alternative: &str, version: &str, log: &Logger) {
    use nh_config::neohugo::neohugo::{
        deprecate_level_with_logger, deprecation_log_level_from_version,
    };
    let level = deprecation_log_level_from_version(version);
    deprecate_level_with_logger(item, alternative, version, level, log);
}

/// Go `%q` of a string.
pub(crate) fn quote(s: &str) -> String {
    go_strconv::quote(s.as_bytes())
}

/// Go: `allconfig.Configs` — base + per-language configs + modules + languages.
pub struct Configs {
    pub base: Arc<Config>,
    pub loading_info: LoadConfigResult,
    pub language_config_map: BTreeMap<String, Arc<Config>>,
    pub language_config_slice: Vec<Arc<Config>>,
    pub is_multihost: bool,
    pub modules: Arc<Modules>,
    pub modules_client: Option<Arc<ModulesClient>>,
    /// Enabled languages sorted by (weight, lang).
    pub languages: Languages,
    pub languages_default_first: Languages,
    pub content_path_parser: Arc<nh_common::paths::pathparser::PathParser>,
    pub(crate) config_langs: Vec<Arc<ConfigLanguage>>,
}

/// A language config slot while building `Configs`: Go's `langConfigMap[k] = all` shares the
/// base config pointer.
#[derive(Clone)]
pub(crate) enum LangSlot {
    Base,
    Own(Box<Config>),
}

/// `Configs` before `Init` (the configs are still mutable).
pub(crate) struct ConfigsBuild {
    pub(crate) base: Config,
    pub(crate) language_config_map: BTreeMap<String, LangSlot>,
    pub(crate) loading_info: LoadConfigResult,
    pub(crate) is_multihost: bool,
    pub(crate) modules: Modules,
    pub(crate) modules_client: Option<Arc<ModulesClient>>,
}

impl ConfigsBuild {
    /// Go: `(*Configs).transientErr()`: the last transient error found during config
    /// compilation (Go returns the first found in map iteration order).
    // Go: config/allconfig/allconfig.go:transientErr
    pub(crate) fn transient_err(&self) -> Option<String> {
        for slot in self.language_config_map.values() {
            let c = match slot {
                LangSlot::Base => &self.base,
                LangSlot::Own(c) => c,
            };
            if let Some(e) = &c.compiled().transient_err {
                return Some(e.clone());
            }
        }
        None
    }

    fn lang_config(&self, k: &str) -> &Config {
        match &self.language_config_map[k] {
            LangSlot::Base => &self.base,
            LangSlot::Own(c) => c,
        }
    }

    fn lang_config_mut(&mut self, k: &str) -> &mut Config {
        match self
            .language_config_map
            .get_mut(k)
            .expect("language config")
        {
            LangSlot::Base => &mut self.base,
            LangSlot::Own(c) => c,
        }
    }

    /// Go: `Configs.Init()` — sort languages, filter disabled, PathParser, per-language
    /// providers, project config defaults (mounts).
    // Go: config/allconfig/allconfig.go:Init
    pub(crate) fn init(mut self) -> Result<Configs> {
        const EN: &str = "en";

        let mut lang_keys: Vec<String> = self.language_config_map.keys().cloned().collect();
        let has_en = self.language_config_map.contains_key(EN);

        // Sort the LanguageConfigSlice by language weight (if set) or lang.
        let weights: BTreeMap<String, i64> = lang_keys
            .iter()
            .map(|k| {
                let w = self
                    .lang_config(k)
                    .languages
                    .get_lang(k)
                    .map(|l| l.weight)
                    .unwrap_or(0);
                (k.clone(), w)
            })
            .collect();
        // Go: sort.Slice (the keys are unique, so there are no ties).
        go_sort::sort_by(&mut lang_keys, |ki, kj| {
            let (wi, wj) = (weights[ki], weights[kj]);
            if wi != wj {
                return wi < wj;
            }
            ki < kj
        });

        // See issue #13646.
        let mut default_config_language_fallback = EN.to_string();
        if !has_en {
            // Pick the first one.
            default_config_language_fallback = lang_keys[0].clone();
        }

        if self.base.root.default_content_language.is_empty() {
            self.base.root.default_content_language = default_config_language_fallback.clone();
        }

        let mut languages: Languages = Vec::new();
        let mut slice_keys: Vec<String> = Vec::new();
        for k in &lang_keys {
            let base_dcl = self.base.root.default_content_language.clone();
            let v = self.lang_config_mut(k);
            if v.root.default_content_language.is_empty() {
                v.root.default_content_language = default_config_language_fallback.clone();
            }
            slice_keys.push(k.clone());
            let language_conf = v.languages.get_lang(k).unwrap_or_default();
            let language = Language::new(k, &base_dcl, &v.root.time_zone, language_conf)?;
            languages.push(language);
        }

        // Filter out disabled languages.
        languages.retain(|l| !l.config.disabled);

        let dcl = self.base.root.default_content_language.clone();
        let mut languages_default_first: Languages = Vec::new();
        for l in &languages {
            if l.lang == dcl {
                languages_default_first.push(l.clone());
            }
        }
        for l in &languages {
            if l.lang != dcl {
                languages_default_first.push(l.clone());
            }
        }

        if self.modules.is_empty() {
            return Err(Error::new(
                "no modules loaded (need at least the main module)",
            ));
        }

        // Freeze the configs (shared base pointer for the languages that use it).
        let base = Arc::new(self.base.clone());
        let mut language_config_map: BTreeMap<String, Arc<Config>> = BTreeMap::new();
        for (k, slot) in &self.language_config_map {
            let c = match slot {
                LangSlot::Base => base.clone(),
                LangSlot::Own(c) => Arc::new((**c).clone()),
            };
            language_config_map.insert(k.clone(), c);
        }

        let content_path_parser =
            Arc::new(new_content_path_parser(&languages_default_first, &base));

        let languages_arc = languages.clone();
        let make_config_langs = |base: &Arc<Config>,
                                 lcm: &BTreeMap<String, Arc<Config>>,
                                 modules: &Arc<Modules>|
         -> Vec<Arc<ConfigLanguage>> {
            languages_default_first
                .iter()
                .map(|l| {
                    Arc::new(ConfigLanguage {
                        config: lcm[&l.lang].clone(),
                        base_config: self.loading_info.base_config.clone(),
                        languages: languages_arc.clone(),
                        languages_default_first: languages_default_first.clone(),
                        path_parser: content_path_parser.clone(),
                        is_multihost: self.is_multihost,
                        modules: modules.clone(),
                        language: l.clone(),
                        base: base.clone(),
                    })
                })
                .collect()
        };

        // Apply default project mounts.
        let mut project = (*self.modules[0]).clone();
        {
            let tmp_modules = Arc::new(self.modules.clone());
            let config_langs = make_config_langs(&base, &language_config_map, &tmp_modules);
            let cfgs: Vec<&dyn AllProvider> = config_langs
                .iter()
                .map(|c| c.as_ref() as &dyn AllProvider)
                .collect();
            nh_hugofs::modules::config::apply_project_config_defaults(&mut project, &cfgs)?;
        }
        self.modules[0] = Arc::new(project);

        // We should consolidate this, but to get a full view of the mounts in e.g. "hugo
        // config" we need to transfer any default mounts added above to the config used to
        // print the config.
        let mut base_mounts = self.base.module.mounts.clone();
        let mut base_raw = self.base.module.mounts_raw_files.clone();
        for m in self.modules[0].mounts() {
            let found = base_mounts
                .iter()
                .any(|cm| cm.source == m.source && cm.target == m.target && cm.lang == m.lang);
            if !found {
                // Keep the raw include/exclude values aligned with the mounts.
                while base_raw.len() < base_mounts.len() {
                    let i = base_raw.len();
                    base_raw.push(self.base.module.mount_raw_files(i));
                }
                base_mounts.push(m.clone());
                base_raw.push((
                    nh_hugofs::modules::config::raw_files(&m.include_files),
                    nh_hugofs::modules::config::raw_files(&m.exclude_files),
                ));
            }
        }

        // Transfer the changed mounts to the language versions (all share the same mount set,
        // but can be displayed in different languages).
        let set_mounts = |c: &mut Config| {
            c.module.mounts = base_mounts.clone();
            c.module.mounts_raw_files = base_raw.clone();
        };
        set_mounts(&mut self.base);
        let keys: Vec<String> = self.language_config_map.keys().cloned().collect();
        for k in &keys {
            if let LangSlot::Own(c) = self.language_config_map.get_mut(k).expect("key") {
                set_mounts(c);
            }
        }

        // The final (frozen) configs.
        let base = Arc::new(self.base.clone());
        let mut language_config_map: BTreeMap<String, Arc<Config>> = BTreeMap::new();
        for (k, slot) in &self.language_config_map {
            let c = match slot {
                LangSlot::Base => base.clone(),
                LangSlot::Own(c) => Arc::new((**c).clone()),
            };
            language_config_map.insert(k.clone(), c);
        }
        let language_config_slice: Vec<Arc<Config>> = slice_keys
            .iter()
            .map(|k| language_config_map[k].clone())
            .collect();
        let modules = Arc::new(self.modules.clone());
        let config_langs = make_config_langs(&base, &language_config_map, &modules);

        Ok(Configs {
            base,
            loading_info: clone_result(&self.loading_info),
            language_config_map,
            language_config_slice,
            is_multihost: self.is_multihost,
            modules,
            modules_client: self.modules_client.clone(),
            languages: languages_arc,
            languages_default_first: languages_default_first.clone(),
            content_path_parser,
            config_langs,
        })
    }
}

/// Go: the `ContentPathParser` of `Configs.Init`.
fn new_content_path_parser(
    languages_default_first: &Languages,
    base: &Arc<Config>,
) -> nh_common::paths::pathparser::PathParser {
    let b1 = base.clone();
    let b2 = base.clone();
    let b3 = base.clone();
    nh_common::paths::pathparser::PathParser {
        language_index: Some(nh_langs::language::as_index_set(languages_default_first)),
        is_lang_disabled: Some(Arc::new(move |lang: &str| b1.is_lang_disabled(lang))),
        is_content_ext: Some(Arc::new(move |ext: &str| {
            b2.content_types
                .as_ref()
                .expect("content types decoded")
                .config
                .is_content_suffix(ext)
        })),
        is_output_format: Some(Arc::new(move |name: &str, ext: &str| {
            if name.is_empty() {
                return false;
            }

            if let Some(of) = b3
                .output_formats
                .as_ref()
                .expect("output formats decoded")
                .config
                .get_by_name(name)
            {
                if !ext.is_empty() && !of.media_type.has_suffix(ext) {
                    return false;
                }
                return true;
            }
            false
        })),
    }
}

impl Configs {
    /// Go: `Configs.Init()` — sort languages, filter disabled, PathParser, per-language providers,
    /// project config defaults (mounts). `load_config` runs it; calling it again rebuilds the
    /// languages and providers from the current configs.
    // Go: config/allconfig/allconfig.go:Init
    pub fn init(&mut self) -> Result<()> {
        let mut map = BTreeMap::new();
        for (k, c) in &self.language_config_map {
            if Arc::ptr_eq(c, &self.base) {
                map.insert(k.clone(), LangSlot::Base);
            } else {
                map.insert(k.clone(), LangSlot::Own(Box::new((**c).clone())));
            }
        }
        let b = ConfigsBuild {
            base: (*self.base).clone(),
            language_config_map: map,
            loading_info: clone_result(&self.loading_info),
            is_multihost: self.is_multihost,
            modules: (*self.modules).clone(),
            modules_client: self.modules_client.clone(),
        };
        *self = b.init()?;
        Ok(())
    }

    /// Go: `(*Configs).Validate(logger)`: warns about cascade target paths with extensions.
    // Go: config/allconfig/allconfig.go:Validate
    pub fn validate(&self, logger: &Logger) -> Result<()> {
        if let Some(cascade) = &self.base.cascade {
            cascade.config.range(|p, _cfg| {
                nh_page::page_matcher::check_cascade_pattern(Some(logger), p);
                true
            });
        }
        Ok(())
    }

    /// A config always has at least one language.
    // Go: config/allconfig/allconfig.go:IsZero
    pub fn is_zero(&self) -> bool {
        self.languages.is_empty()
    }

    // Go: config/allconfig/allconfig.go:ConfigLangs
    pub fn config_langs(&self) -> Vec<Arc<dyn AllProvider>> {
        self.config_langs
            .iter()
            .map(|c| c.clone() as Arc<dyn AllProvider>)
            .collect()
    }

    // Go: config/allconfig/allconfig.go:GetFirstLanguageConfig
    pub fn get_first_language_config(&self) -> Arc<dyn AllProvider> {
        self.config_langs[0].clone()
    }

    // Go: config/allconfig/allconfig.go:GetByLang
    pub fn get_by_lang(&self, lang: &str) -> Option<Arc<dyn AllProvider>> {
        self.config_langs
            .iter()
            .find(|l| l.language.lang == lang)
            .map(|l| l.clone() as Arc<dyn AllProvider>)
    }
}

/// Go: `newDefaultConfig()`.
// Go: config/allconfig/allconfig.go:newDefaultConfig
pub fn new_default_config() -> Config {
    let mut taxonomies = BTreeMap::new();
    taxonomies.insert("tag".to_string(), "tags".to_string());
    taxonomies.insert("category".to_string(), "categories".to_string());
    Config {
        internal: InternalConfig::default(),
        c: None,
        root: RootConfig {
            environment: nh_config::neohugo::neohugo::ENVIRONMENT_PRODUCTION.to_string(),
            title_case_style: "AP".to_string(),
            pluralize_list_titles: true,
            capitalize_list_titles: true,
            static_dir: GoSlice::from_vec(vec!["static".to_string()]),
            summary_length: 70,
            timeout: "60s".to_string(),

            common_dirs: CommonDirs {
                arche_type_dir: "archetypes".to_string(),
                content_dir: "content".to_string(),
                resource_dir: "resources".to_string(),
                publish_dir: "public".to_string(),
                themes_dir: "themes".to_string(),
                asset_dir: "assets".to_string(),
                layout_dir: "layouts".to_string(),
                i18n_dir: "i18n".to_string(),
                data_dir: "data".to_string(),
                ..Default::default()
            },
            ..Default::default()
        },
        author: None,
        social: None,
        build: BuildConfig::default(),
        caches: FileCacheConfigs::default(),
        http_cache: HttpCacheConfig::default(),
        markup: MarkupConfig::default(),
        content_types: None,
        media_types: None,
        imaging: None,
        output_formats: None,
        outputs: BTreeMap::new(),
        cascade: None,
        segments: None,
        menus: None,
        deployment: DeployConfig::default(),
        module: ModuleConfig::default(),
        frontmatter: FrontmatterConfig::default(),
        minify: MinifyConfig::default(),
        permalinks: BTreeMap::new(),
        taxonomies,
        sitemap: SitemapConfig {
            priority: -1.0,
            filename: "sitemap.xml".to_string(),
            ..Default::default()
        },
        related: RelatedConfig::default(),
        server: Server::default(),
        pagination: Pagination::default(),
        page: PageConfig::default(),
        privacy: nh_config::privacy::Config::default(),
        security: SecurityConfig::default(),
        services: nh_config::services::Config::default(),
        params: Arc::new(Map::new(MapType::Params)),
        languages: LanguagesMap::default(),
        ugly_urls: UglyUrls::Nil,
        imaging_live: false,
        segments_live: false,
    }
}

/// Go's `images.DecodeConfig` merges the imaging defaults into the map it is given, which is
/// the config tree's own `imaging` map (the root one, or a language's): replay that write.
fn write_back_imaging(cfg: &dyn Provider, path: &str, c: &mut Config) {
    let ns = c.imaging.as_ref().expect("imaging decoded");
    if let Value::Map(m) = &ns.source_structure {
        let mut m = (**m).clone();
        m.ty = MapType::Params;
        cfg.set(path, Value::map(m));
    }
    c.imaging_live = true;
}

/// `LoadConfigResult` is not `Clone` (its provider is shared like Go's interface value).
pub(crate) fn clone_result(r: &LoadConfigResult) -> LoadConfigResult {
    LoadConfigResult {
        cfg: r.cfg.clone(),
        config_files: r.config_files.clone(),
        base_config: r.base_config.clone(),
    }
}

/// `Provider.GetStringMap(key)` with Go's nil: `None` when the value is nil or not convertible
/// to a map (Go's `cast.ToStringMapE` error).
pub(crate) fn get_string_map_opt(p: &dyn Provider, key: &str) -> Option<Map> {
    let v = p.get(key);
    if v.is_nil() {
        return None;
    }
    nh_common::maps::maps::to_string_map_e(&v).ok()
}

/// `Provider.GetStringMapString(key)` with Go's nil.
pub(crate) fn get_string_map_string_opt(p: &dyn Provider, key: &str) -> Option<Map> {
    let v = p.get(key);
    if v.is_nil() {
        return None;
    }
    nh_common::maps::maps::to_string_map_string_e(&v).ok()
}

/// Go: `reflect.DeepEqual(a, b)` of config values.
fn deep_equal(a: &Value, b: &Value) -> bool {
    a == b
}

/// Go: `fromLoadConfigResult(fs, logger, res)` — creates the configs from res.
// Go: config/allconfig/allconfig.go:fromLoadConfigResult
pub(crate) fn from_load_config_result(
    fs: &Arc<dyn Fs>,
    logger: &Logger,
    res: &mut LoadConfigResult,
) -> Result<ConfigsBuild> {
    if !res.cfg.is_set("languages") {
        // We need at least one
        let mut lang = res.cfg.get_string("defaultContentLanguage");
        if lang.is_empty() {
            lang = "en".to_string();
        }
        let mut m = Map::new(MapType::Params);
        m.insert(lang.as_str(), Value::map(Map::new(MapType::Params)));
        res.cfg.set("languages", Value::map(m));
    }
    let mut bcfg = res.base_config.clone();
    let cfg = res.cfg.clone();

    let mut all = new_default_config();

    decode_config_from_params(fs, logger, &bcfg, cfg.as_ref(), &mut all, None)?;
    if let Value::Map(m) = cfg.get("imaging")
        && m.ty == MapType::Params
    {
        write_back_imaging(cfg.as_ref(), "imaging", &mut all);
    }
    all.segments_live = matches!(cfg.get("segments"), Value::Map(m) if m.ty == MapType::Params);

    let mut lang_config_map: BTreeMap<String, LangSlot> = BTreeMap::new();

    let languages_config = get_string_map_opt(cfg.as_ref(), "languages")
        .unwrap_or_else(|| Map::new(MapType::StringAny));
    let mut is_multihost = false;

    all.compile_config_with_logger(logger)?;

    // Go iterates the languages map in random order (IsMultihost is set during the loop and
    // read by the goldmark defaults below); the port uses the key order.
    for (k, v) in languages_config.entries.iter() {
        let k = k.to_str_lossy().into_owned();
        let merged_config = nh_config::default_config_provider::DefaultConfigProvider::new();
        let mut different_root_keys: Vec<String> = Vec::new();
        match v {
            Value::Map(x) if x.ty == MapType::Params => {
                let mut x = (**x).clone();
                if x.get(b"params").is_none() {
                    let mut p = Map::new(MapType::Params);
                    p.insert(
                        nh_common::maps::params::MERGE_STRATEGY_KEY,
                        nh_common::maps::params::ParamsMergeStrategy::Deep.value(),
                    );
                    x.insert("params", Value::map(p.clone()));
                    // Go writes into the shared map of the config tree.
                    cfg.set(&format!("languages.{k}.params"), Value::map(p));
                }

                for (kk, vv) in x.entries.iter() {
                    let kk = kk.to_str_lossy().into_owned();
                    if kk == "_merge" {
                        continue;
                    }
                    if kk == "baseurl" {
                        // baseURL configure don the language level is a multihost setup.
                        is_multihost = true;
                    }
                    merged_config.set(&kk, vv.clone());
                    let rootv = cfg.get(&kk);
                    if !rootv.is_invalid() && cfg.is_set(&kk) {
                        // This overrides a root key and potentially needs a merge.
                        if !deep_equal(&rootv, vv) {
                            match vv {
                                Value::Map(vvv) if vvv.ty == MapType::Params => {
                                    different_root_keys.push(kk.clone());

                                    // Use the language value as base.
                                    let mut merged_config_entry = (**vvv).clone();
                                    // Merge in the root value.
                                    match &rootv {
                                        Value::Map(rm) if rm.ty == MapType::Params => {
                                            nh_common::maps::params::merge_params(
                                                &mut merged_config_entry,
                                                rm,
                                            );
                                        }
                                        _ => {
                                            return Err(Error::new(format!(
                                                "interface conversion: interface {{}} is {}, not maps.Params",
                                                rootv.go_type_name()
                                            )));
                                        }
                                    }

                                    // Go: Set of a Params over a Params is SetParams into the
                                    // language's own map of the config tree.
                                    cfg.set(
                                        &format!("languages.{k}.{kk}"),
                                        Value::map(merged_config_entry.clone()),
                                    );
                                    merged_config.set(&kk, Value::map(merged_config_entry));
                                }
                                _ => {
                                    // Apply new values to the root.
                                    different_root_keys.push(String::new());
                                }
                            }
                        }
                    } else {
                        match vv {
                            Value::Map(vvv) if vvv.ty == MapType::Params => {
                                different_root_keys.push(kk.clone());
                            }
                            _ => {
                                // Apply new values to the root.
                                different_root_keys.push(String::new());
                            }
                        }
                    }
                }
                let different_root_keys =
                    nh_helpers::general::unique_strings_sorted(different_root_keys)
                        .unwrap_or_default();

                if different_root_keys.is_empty() {
                    lang_config_map.insert(k, LangSlot::Base);
                    continue;
                }

                // Create a copy of the complete config and replace the root keys with the language specific ones.
                let mut clone = all.clone_for_lang();

                if let Err(err) = decode_config_from_params(
                    fs,
                    logger,
                    &bcfg,
                    &merged_config,
                    &mut clone,
                    Some(&different_root_keys),
                ) {
                    return Err(Error::new(format!(
                        "failed to decode config for language {}: {}",
                        quote(&k),
                        err
                    )));
                }
                if different_root_keys.iter().any(|k| k == "imaging")
                    && let Value::Map(m) = merged_config.get("imaging")
                    && m.ty == MapType::Params
                {
                    write_back_imaging(cfg.as_ref(), &format!("languages.{k}.imaging"), &mut clone);
                }
                if different_root_keys.iter().any(|k| k == "segments") {
                    clone.segments_live = matches!(merged_config.get("segments"), Value::Map(m) if m.ty == MapType::Params);
                }
                clone.compile_config_with_logger(logger)?;

                // Adjust Goldmark config defaults for multilingual, single-host sites.
                if languages_config.len() > 1
                    && !is_multihost
                    && !clone.markup.goldmark.duplicate_resource_files
                {
                    let hooks = &mut clone.markup.goldmark.render_hooks;
                    if hooks.image.use_embedded == RENDER_HOOK_USE_EMBEDDED_AUTO {
                        hooks.image.use_embedded = RENDER_HOOK_USE_EMBEDDED_FALLBACK.to_string();
                    }
                    if hooks.link.use_embedded == RENDER_HOOK_USE_EMBEDDED_AUTO {
                        hooks.link.use_embedded = RENDER_HOOK_USE_EMBEDDED_FALLBACK.to_string();
                    }
                }

                lang_config_map.insert(k, LangSlot::Own(Box::new(clone)));
            }
            v if is_merge_strategy(v) => {}
            _ => {
                // Go panics here.
                return Err(Error::new(format!(
                    "unknown type in languages config: {}",
                    v.go_type_name()
                )));
            }
        }
    }

    bcfg.publish_dir = all.root.common_dirs.publish_dir.clone();
    res.base_config = bcfg.clone();
    all.root.common_dirs.cache_dir = bcfg.cache_dir.clone();
    for l in lang_config_map.values_mut() {
        if let LangSlot::Own(l) = l {
            l.root.common_dirs.cache_dir = bcfg.cache_dir.clone();
        }
    }

    Ok(ConfigsBuild {
        base: all,
        language_config_map: lang_config_map,
        loading_info: clone_result(res),
        is_multihost,
        modules: Vec::new(),
        modules_client: None,
    })
}

fn is_merge_strategy(v: &Value) -> bool {
    match v {
        Value::Object(o) => o
            .as_any()
            .is::<nh_common::maps::params::ParamsMergeStrategy>(),
        _ => false,
    }
}

/// Go: `decodeConfigFromParams(fs, logger, bcfg, p, target, keys)`.
// Go: config/allconfig/allconfig.go:decodeConfigFromParams
pub(crate) fn decode_config_from_params(
    fs: &Arc<dyn Fs>,
    logger: &Logger,
    bcfg: &nh_config::common_config::BaseConfig,
    p: &dyn Provider,
    target: &mut Config,
    keys: Option<&[String]>,
) -> Result<()> {
    let all = all_decoder_setups();
    let mut decoder_setups = Vec::new();

    match keys {
        None => decoder_setups.extend(all.iter()),
        Some([]) => decoder_setups.extend(all.iter()),
        Some(keys) => {
            for key in keys {
                match all.iter().find(|d| d.key == key) {
                    Some(v) => decoder_setups.push(v),
                    None => logger.warnf(format!("Skip unknown config key {}", quote(key))),
                }
            }
        }
    }

    // Sort them to get the dependency order right.
    decoder_setups.sort_by(|ki, kj| {
        if ki.weight == kj.weight {
            return ki.key.cmp(kj.key);
        }
        ki.weight.cmp(&kj.weight)
    });

    for v in decoder_setups {
        let mut dc = DecodeConfig {
            p,
            c: target,
            fs: fs.as_ref(),
            bcfg: bcfg.clone(),
            logger,
        };
        if let Err(err) = (v.decode)(v, &mut dc) {
            return Err(Error::with_kind(
                err.kind(),
                format!("failed to decode {}: {}", quote(v.key), err),
            ));
        }
    }

    Ok(())
}

/// Go: `createDefaultOutputFormats(allFormats)` (home/section/taxonomy/term: html+rss, page: html, rss: rss).
// Go: config/allconfig/allconfig.go:createDefaultOutputFormats
pub fn create_default_output_formats(all_formats: &Formats) -> BTreeMap<String, Vec<String>> {
    if all_formats.0.is_empty() {
        panic!("no output formats");
    }
    let builtin = nh_media::output::output_format::builtin_formats();
    let rss = all_formats.get_by_name(&builtin.rss.name);
    let html_out = all_formats
        .get_by_name(&builtin.html.name)
        .unwrap_or_default();

    let mut default_list_types = vec![html_out.name.clone()];
    if let Some(rss_out) = &rss {
        default_list_types.push(rss_out.name.clone());
    }

    let mut m = BTreeMap::new();
    m.insert(
        nh_common::kinds::KIND_PAGE.to_string(),
        vec![html_out.name.clone()],
    );
    m.insert(
        nh_common::kinds::KIND_HOME.to_string(),
        default_list_types.clone(),
    );
    m.insert(
        nh_common::kinds::KIND_SECTION.to_string(),
        default_list_types.clone(),
    );
    m.insert(
        nh_common::kinds::KIND_TERM.to_string(),
        default_list_types.clone(),
    );
    m.insert(
        nh_common::kinds::KIND_TAXONOMY.to_string(),
        default_list_types,
    );

    // May be disabled
    if let Some(rss_out) = rss {
        m.insert("rss".to_string(), vec![rss_out.name]);
    }

    m
}

/// `DecodeError` of a mapstructure decode (used by the `page`/`pagination` decoders).
pub(crate) fn decode_err(e: DecodeError) -> Error {
    Error::new(e.message())
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: config/allconfig/allconfig.go (1217 lines; 17/20 funcs executed)
//   types: InternalConfig, Config, configCompiler, ConfigCompiled, RootConfig, Configs
// OK L78-98: init()   (configLanguageKeys: unused by the port)
// OK L216-252: (c Config) cloneForLang() *Config
// OK L254-505: (c *Config) CompileConfig(logger loggers.Logger) error
// OK L507-509: (c *Config) IsKindEnabled(kind string) bool
// OK L511-513: (c *Config) IsLangDisabled(lang string) bool
// OK L543-547: (c *ConfigCompiled) SetMainSections(sections []string)
// OK L550-554: (c *ConfigCompiled) IsMainSectionsSet() bool
// OK L557-561: (c *ConfigCompiled) SetServerInfo(baseURL, baseURLLiveReload urls.BaseURL, serverInterface string)
// OK L780-795: (c RootConfig) staticDirs() []string
// OK L816-822: (c *Configs) Validate(logger loggers.Logger) error
// OK L825-832: (c *Configs) transientErr() error
// OK L834-837: (c *Configs) IsZero() bool
// OK L839-977: (c *Configs) Init() error
// OK L979-981: (c Configs) ConfigLangs() []config.AllProvider
// OK L983-985: (c Configs) GetFirstLanguageConfig() config.AllProvider
// OK L987-994: (c Configs) GetByLang(lang string) config.AllProvider
// OK L996-1022: newDefaultConfig() *Config
// OK L1025-1153: fromLoadConfigResult(fs afero.Fs, logger loggers.Logger, res config.LoadConfigResult) (*Configs, error)
// OK L1155-1189: decodeConfigFromParams(fs afero.Fs, logger loggers.Logger, bcfg config.BaseConfig, p config.Provider, target *Config, keys []string) error
// OK L1191-1217: createDefaultOutputFormats(allFormats output.Formats) map[string][]string
// ---------------------------------------------------------------------------
