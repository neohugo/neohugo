//! Port of `config/allconfig/allconfig.go`.
//!
//! Owner: Wave B task T09 (allconfig-modules).


//! Go `config/allconfig/allconfig.go`: the fully decoded site configuration, compiled per
//! language. Oracle: `neohugo config --format json [--lang th]` (specs/architecture-core-data/
//! config-*.json) — nh-commands implements the same command so the JSON can be diffed.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;
use std::time::Duration;

use go_value::{Map, Time, Value};
use nh_common::urls::BaseURL;
use nh_common::Result;
use nh_config::common_config::{BuildConfig, CommonDirs, LoadConfigResult, PageConfig, Pagination, Server, SitemapConfig};
use nh_config::namespace::ConfigNamespace;
use nh_config::security::security_config::Config as SecurityConfig;
use nh_helpers::cache::filecache::filecache_config::Configs as FileCacheConfigs;
use nh_helpers::cache::httpcache::httpcache::{Config as HttpCacheConfig, ConfigCompiled as HttpCacheConfigCompiled};
use nh_hugofs::modules::config::Config as ModuleConfig;
use nh_hugofs::modules::module::Modules;
use nh_images::config::{ImagingConfig, ImagingConfigInternal};
use nh_langs::config::LanguageConfig;
use nh_langs::language::Languages;
use nh_markup::markup_config::Config as MarkupConfig;
use nh_media::media::config::ContentTypes;
use nh_media::media::media_type::Types as MediaTypes;
use nh_media::output::output_format::Formats;
use nh_page::navigation::menu::Menu;
use nh_page::page_matcher::{Cascade, PageMatcherParamsConfig};
use nh_page::pagemeta::page_frontmatter::FrontmatterConfig;
use nh_page::related::Config as RelatedConfig;
use nh_transform::minifiers::config::MinifyConfig;

use crate::segments::{SegmentFilter, Segments};

/// Go: `allconfig.InternalConfig` (set from CLI flags; `clock` from `--clock`).
#[derive(Clone, Debug, Default)]
pub struct InternalConfig {
    pub running: bool,
    pub quiet: bool,
    pub verbose: bool,
    pub clock: String,
    pub watch: bool,
    pub fast_render_mode: bool,
    pub live_reload_port: i64,
}

/// Go: `allconfig.RootConfig` (top-level keys, WeakDecoded).
#[derive(Clone, Debug, Default)]
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
    pub disable_kinds: Vec<String>,
    pub disable_languages: Vec<String>,
    pub render_segments: Vec<String>,
    /// NOTE neohugo inverted semantics: hugolib sets AddHugoGeneratorTag = this (false) -> never injected.
    pub disable_hugo_generator_inject: bool,
    pub disable_live_reload: bool,
    pub enable_emoji: bool,
    pub main_sections: Vec<String>,
    pub enable_robots_txt: bool,
    pub enable_git_info: bool,
    pub template_metrics: bool,
    pub template_metrics_hints: bool,
    pub no_build_lock: bool,
    pub ignore_logs: Vec<String>,
    pub ignore_files: Vec<String>,
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
    pub theme: Vec<String>,
    pub timeout: String,
    pub time_zone: String,
    pub title_case_style: String,
    pub new_content_editor: String,
    pub no_times: bool,
    pub no_chmod: bool,
    pub clean_destination_dir: bool,
    pub ignore_vendor_paths: String,
    pub common_dirs: CommonDirs,
    pub static_dir: Vec<String>,
    /// staticDir0..staticDir10.
    pub static_dir_n: [Vec<String>; 11],
}

/// Go: `allconfig.ConfigCompiled`.
#[derive(Clone)]
pub struct ConfigCompiled {
    pub timeout: Duration,
    pub base_url: BaseURL,
    pub base_url_live_reload: BaseURL,
    pub server_interface: String,
    /// kind -> output formats.
    pub kind_output_formats: BTreeMap<String, Formats>,
    pub default_output_format: nh_media::output::output_format::OutputFormat,
    pub disabled_kinds: BTreeSet<String>,
    pub disabled_languages: BTreeSet<String>,
    pub ignored_logs: BTreeSet<String>,
    pub create_title: Arc<dyn Fn(&str) -> String + Send + Sync>,
    pub is_ugly_url_section: Arc<dyn Fn(&str) -> bool + Send + Sync>,
    pub ignore_file: Arc<dyn Fn(&str) -> bool + Send + Sync>,
    pub segment_filter: SegmentFilter,
    pub main_sections: Vec<String>,
    /// From `--clock` (zero if unset).
    pub clock: Time,
    pub http_cache: HttpCacheConfigCompiled,
}

/// Go: `allconfig.Config`.
#[derive(Clone)]
pub struct Config {
    pub internal: InternalConfig,
    pub c: Option<Arc<ConfigCompiled>>,
    pub root: RootConfig,
    pub author: Map,
    pub social: Map,
    pub build: BuildConfig,
    pub caches: FileCacheConfigs,
    pub http_cache: HttpCacheConfig,
    pub markup: Arc<MarkupConfig>,
    pub content_types: Arc<ConfigNamespace<Map, ContentTypes>>,
    pub media_types: Arc<ConfigNamespace<Map, MediaTypes>>,
    pub imaging: Arc<ConfigNamespace<ImagingConfig, ImagingConfigInternal>>,
    pub output_formats: Arc<ConfigNamespace<Map, Formats>>,
    pub outputs: BTreeMap<String, Vec<String>>,
    pub cascade: Arc<ConfigNamespace<Vec<PageMatcherParamsConfig>, Cascade>>,
    pub segments: Arc<ConfigNamespace<Map, Segments>>,
    pub menus: Arc<ConfigNamespace<Map, BTreeMap<String, Menu>>>,
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
    pub languages: BTreeMap<String, LanguageConfig>,
    pub ugly_urls: Value,
}

impl Config {
    /// Go: `CompileConfig(logger)` (derives `C`).
    // Go: config/allconfig/allconfig.go:CompileConfig
    pub fn compile_config(&mut self) -> Result<()> {
        todo!()
    }

    // Go: config/allconfig/allconfig.go:IsKindEnabled
    pub fn is_kind_enabled(&self, kind: &str) -> bool {
        todo!()
    }

    // Go: config/allconfig/allconfig.go:IsLangDisabled
    pub fn is_lang_disabled(&self, lang: &str) -> bool {
        todo!()
    }

    // Go: config/allconfig/allconfig.go:cloneForLang
    pub(crate) fn clone_for_lang(&self) -> Config {
        todo!()
    }
}

/// Go: `allconfig.Configs` — base + per-language configs + modules + languages.
pub struct Configs {
    pub base: Arc<Config>,
    pub loading_info: LoadConfigResult,
    pub language_config_map: BTreeMap<String, Arc<Config>>,
    pub language_config_slice: Vec<Arc<Config>>,
    pub is_multihost: bool,
    pub modules: Arc<Modules>,
    /// Enabled languages sorted by (weight, lang).
    pub languages: Languages,
    pub languages_default_first: Languages,
    pub content_path_parser: Arc<nh_common::paths::pathparser::PathParser>,
    pub(crate) config_langs: Vec<Arc<crate::configlanguage::ConfigLanguage>>,
}

impl Configs {
    /// Go: `Configs.Init()` — sort languages, filter disabled, PathParser, per-language providers,
    /// project config defaults (mounts).
    // Go: config/allconfig/allconfig.go:Init
    pub fn init(&mut self) -> Result<()> {
        todo!()
    }

    // Go: config/allconfig/allconfig.go:ConfigLangs
    pub fn config_langs(&self) -> Vec<Arc<dyn nh_config::config_provider::AllProvider>> {
        todo!()
    }

    // Go: config/allconfig/allconfig.go:GetFirstLanguageConfig
    pub fn get_first_language_config(&self) -> Arc<dyn nh_config::config_provider::AllProvider> {
        todo!()
    }

    // Go: config/allconfig/allconfig.go:GetByLang
    pub fn get_by_lang(&self, lang: &str) -> Option<Arc<dyn nh_config::config_provider::AllProvider>> {
        todo!()
    }
}

/// Go: `newDefaultConfig()`.
// Go: config/allconfig/allconfig.go:newDefaultConfig
pub fn new_default_config() -> Config {
    todo!()
}

/// Go: `createDefaultOutputFormats(allFormats)` (home/section/taxonomy/term: html+rss, page: html, rss: rss).
// Go: config/allconfig/allconfig.go:createDefaultOutputFormats
pub fn create_default_output_formats(all_formats: &Formats) -> BTreeMap<String, Vec<String>> {
    todo!()
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: config/allconfig/allconfig.go (1217 lines; 17/20 funcs executed)
//   types: InternalConfig, Config, configCompiler, ConfigCompiled, RootConfig, Configs
// EX L78-98: init()
// EX L216-252: (c Config) cloneForLang() *Config
// EX L254-505: (c *Config) CompileConfig(logger loggers.Logger) error
// EX L507-509: (c *Config) IsKindEnabled(kind string) bool
// EX L511-513: (c *Config) IsLangDisabled(lang string) bool
// EX L543-547: (c *ConfigCompiled) SetMainSections(sections []string)
// EX L550-554: (c *ConfigCompiled) IsMainSectionsSet() bool
//    L557-561: (c *ConfigCompiled) SetServerInfo(baseURL, baseURLLiveReload urls.BaseURL, serverInterface string)
// EX L780-795: (c RootConfig) staticDirs() []string
// EX L816-822: (c *Configs) Validate(logger loggers.Logger) error
// EX L825-832: (c *Configs) transientErr() error
//    L834-837: (c *Configs) IsZero() bool
// EX L839-977: (c *Configs) Init() error
// EX L979-981: (c Configs) ConfigLangs() []config.AllProvider
// EX L983-985: (c Configs) GetFirstLanguageConfig() config.AllProvider
//    L987-994: (c Configs) GetByLang(lang string) config.AllProvider
// EX L996-1022: newDefaultConfig() *Config
// EX L1025-1153: fromLoadConfigResult(fs afero.Fs, logger loggers.Logger, res config.LoadConfigResult) (*Configs, error)
// EX L1155-1189: decodeConfigFromParams(fs afero.Fs, logger loggers.Logger, bcfg config.BaseConfig, p config.Provider, target *Config, keys []string) error
// EX L1191-1217: createDefaultOutputFormats(allFormats output.Formats) map[string][]string
// ---------------------------------------------------------------------------
