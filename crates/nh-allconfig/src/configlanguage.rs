//! Port of `config/allconfig/configlanguage.go`.
//!
//! Owner: Wave B task T09 (allconfig-modules).

//! Go `allconfig.ConfigLanguage` — implements `config.AllProvider` for one language.

use std::any::Any;
use std::collections::BTreeSet;
use std::sync::Arc;
use std::time::Duration;

use nh_common::paths::pathparser::PathParser;
use nh_common::urls::BaseURL;
use nh_config::common_config::{BaseConfig, CommonDirs, Pagination};
use nh_config::config_provider::{AllProvider, ContentTypesProvider};
use nh_langs::language::{Language, Languages};

use crate::allconfig::Config;

/// Go: `allconfig.ConfigLanguage`.
pub struct ConfigLanguage {
    pub config: Arc<Config>,
    pub base_config: BaseConfig,
    /// Shared (Go `m *Configs`): languages, modules, path parser.
    pub languages: Languages,
    pub languages_default_first: Languages,
    pub path_parser: Arc<PathParser>,
    pub is_multihost: bool,
    pub modules: Arc<nh_hugofs::modules::module::Modules>,
    pub language: Arc<Language>,
    /// Go `c.m.Base`.
    pub base: Arc<Config>,
}

impl ConfigLanguage {
    /// Go: `ConfigLanguage.NewIdentityManager(name)`: a no-op manager unless watching (never
    /// here; the port has no watch mode).
    // Go: config/allconfig/configlanguage.go:NewIdentityManager
    pub fn new_identity_manager(&self, _name: &str) -> Option<()> {
        if !self.watching() {
            return None;
        }
        Some(())
    }
}

/// `time.Duration` (nanoseconds, may be negative) as the `std` duration of the trait
/// (negative values saturate at zero).
fn std_duration(d: go_time::Duration) -> Duration {
    Duration::from_nanos(d.0.max(0) as u64)
}

impl AllProvider for ConfigLanguage {
    // Go: config/allconfig/configlanguage.go:Language
    fn language(&self) -> Arc<Language> {
        self.language.clone()
    }
    // Go: config/allconfig/configlanguage.go:Languages
    fn languages(&self) -> Languages {
        self.languages.clone()
    }
    // Go: config/allconfig/configlanguage.go:LanguagesDefaultFirst
    fn languages_default_first(&self) -> Languages {
        self.languages_default_first.clone()
    }
    // Go: config/allconfig/configlanguage.go:LanguagePrefix
    fn language_prefix(&self) -> String {
        if self.default_content_language_in_subdir()
            && self.default_content_language() == self.language().lang
        {
            return self.language().lang.clone();
        }

        if !self.is_multilingual() || self.default_content_language() == self.language().lang {
            return String::new();
        }
        self.language().lang.clone()
    }
    // Go: config/allconfig/configlanguage.go:BaseURL
    fn base_url(&self) -> BaseURL {
        self.config.compiled().base_url.clone()
    }
    // Go: config/allconfig/configlanguage.go:BaseURLLiveReload
    fn base_url_live_reload(&self) -> BaseURL {
        self.config.compiled().base_url_live_reload.clone()
    }
    // Go: config/allconfig/configlanguage.go:PathParser
    fn path_parser(&self) -> Arc<PathParser> {
        self.path_parser.clone()
    }
    // Go: config/allconfig/configlanguage.go:Environment
    fn environment(&self) -> String {
        self.config.root.environment.clone()
    }
    // Go: config/allconfig/configlanguage.go:IsMultihost
    fn is_multihost(&self) -> bool {
        if self.languages.len() as i64 - self.config.compiled().disabled_languages.len() as i64 <= 1
        {
            return false;
        }
        self.is_multihost
    }
    // Go: config/allconfig/configlanguage.go:IsMultilingual
    fn is_multilingual(&self) -> bool {
        self.languages.len() > 1
    }
    // Go: config/allconfig/configlanguage.go:NoBuildLock
    fn no_build_lock(&self) -> bool {
        self.config.root.no_build_lock
    }
    // Go: config/allconfig/configlanguage.go:BaseConfig
    fn base_config(&self) -> BaseConfig {
        self.base_config.clone()
    }
    // Go: config/allconfig/configlanguage.go:Dirs
    fn dirs(&self) -> CommonDirs {
        self.config.root.common_dirs.clone()
    }
    // Go: config/allconfig/configlanguage.go:Quiet
    fn quiet(&self) -> bool {
        self.base.internal.quiet
    }
    // Go: config/allconfig/configlanguage.go:DirsBase
    fn dirs_base(&self) -> CommonDirs {
        self.base.root.common_dirs.clone()
    }
    // Go: config/allconfig/configlanguage.go:ContentTypes
    fn content_types(&self) -> Arc<dyn ContentTypesProvider> {
        Arc::new(
            self.config
                .content_types
                .as_ref()
                .expect("content types decoded")
                .config
                .clone(),
        )
    }
    /// GetConfigSection is mostly used in tests. The switch statement isn't complete, but
    /// what's in use.
    // Go: config/allconfig/configlanguage.go:GetConfigSection
    fn get_config_section(&self, name: &str) -> Arc<dyn Any + Send + Sync> {
        crate::sections::config_section(self, name)
            .unwrap_or_else(|| panic!("not implemented: {name}"))
    }
    // Go: config/allconfig/configlanguage.go:GetConfig
    fn get_config(&self) -> Arc<dyn Any + Send + Sync> {
        self.config.clone()
    }
    // Go: config/allconfig/configlanguage.go:CanonifyURLs
    fn canonify_urls(&self) -> bool {
        self.config.root.canonify_urls
    }
    // Go: config/allconfig/configlanguage.go:DisablePathToLower
    fn disable_path_to_lower(&self) -> bool {
        self.config.root.disable_path_to_lower
    }
    // Go: config/allconfig/configlanguage.go:RemovePathAccents
    fn remove_path_accents(&self) -> bool {
        self.config.root.remove_path_accents
    }
    // Go: config/allconfig/configlanguage.go:IsUglyURLs
    fn is_ugly_urls(&self, section: &str) -> bool {
        (self.config.compiled().is_ugly_url_section)(section)
    }
    // Go: config/allconfig/configlanguage.go:DefaultContentLanguage
    fn default_content_language(&self) -> String {
        self.config.root.default_content_language.clone()
    }
    // Go: config/allconfig/configlanguage.go:DefaultContentLanguageInSubdir
    fn default_content_language_in_subdir(&self) -> bool {
        self.config.root.default_content_language_in_subdir
    }
    // Go: config/allconfig/configlanguage.go:IsLangDisabled
    fn is_lang_disabled(&self, lang: &str) -> bool {
        self.config.compiled().disabled_languages.contains(lang)
    }
    // Go: config/allconfig/configlanguage.go:SummaryLength
    fn summary_length(&self) -> i64 {
        self.config.root.summary_length
    }
    // Go: config/allconfig/configlanguage.go:Pagination
    fn pagination(&self) -> Pagination {
        self.config.pagination.clone()
    }
    // Go: config/allconfig/configlanguage.go:BuildExpired
    fn build_expired(&self) -> bool {
        self.config.root.build_expired
    }
    // Go: config/allconfig/configlanguage.go:BuildFuture
    fn build_future(&self) -> bool {
        self.config.root.build_future
    }
    // Go: config/allconfig/configlanguage.go:BuildDrafts
    fn build_drafts(&self) -> bool {
        self.config.root.build_drafts
    }
    // Go: config/allconfig/configlanguage.go:Running
    fn running(&self) -> bool {
        self.config.internal.running
    }
    // Go: config/allconfig/configlanguage.go:Watching
    fn watching(&self) -> bool {
        self.base.internal.watch
    }
    // Go: config/allconfig/configlanguage.go:FastRenderMode
    fn fast_render_mode(&self) -> bool {
        self.config.internal.fast_render_mode
    }
    // Go: config/allconfig/configlanguage.go:PrintUnusedTemplates
    fn print_unused_templates(&self) -> bool {
        self.config.root.print_unused_templates
    }
    // Go: config/allconfig/configlanguage.go:EnableMissingTranslationPlaceholders
    fn enable_missing_translation_placeholders(&self) -> bool {
        self.config.root.enable_missing_translation_placeholders
    }
    // Go: config/allconfig/configlanguage.go:TemplateMetrics
    fn template_metrics(&self) -> bool {
        self.config.root.template_metrics
    }
    // Go: config/allconfig/configlanguage.go:TemplateMetricsHints
    fn template_metrics_hints(&self) -> bool {
        self.config.root.template_metrics_hints
    }
    // Go: config/allconfig/configlanguage.go:PrintI18nWarnings
    fn print_i18n_warnings(&self) -> bool {
        self.config.root.print_i18n_warnings
    }
    // Go: config/allconfig/configlanguage.go:CreateTitle
    fn create_title(&self, s: &str) -> String {
        (self.config.compiled().create_title)(s)
    }
    // Go: config/allconfig/configlanguage.go:IgnoreFile
    fn ignore_file(&self, s: &str) -> bool {
        (self.config.compiled().ignore_file)(s)
    }
    // Go: config/allconfig/configlanguage.go:NewContentEditor
    fn new_content_editor(&self) -> String {
        self.config.root.new_content_editor.clone()
    }
    // Go: config/allconfig/configlanguage.go:Timeout
    fn timeout(&self) -> Duration {
        std_duration(self.config.compiled().timeout)
    }
    // Go: config/allconfig/configlanguage.go:StaticDirs
    fn static_dirs(&self) -> Vec<String> {
        self.config.root.static_dirs()
    }
    // Go: config/allconfig/configlanguage.go:IgnoredLogs
    fn ignored_logs(&self) -> BTreeSet<String> {
        self.config.compiled().ignored_logs.clone()
    }
    // Go: config/allconfig/configlanguage.go:WorkingDir
    fn working_dir(&self) -> String {
        self.base.root.common_dirs.working_dir.clone()
    }
    // Go: config/allconfig/configlanguage.go:EnableEmoji
    fn enable_emoji(&self) -> bool {
        self.config.root.enable_emoji
    }
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: config/allconfig/configlanguage.go (261 lines; 33/47 funcs executed)
//   types: ConfigLanguage
// OK L34-36: (c ConfigLanguage) Language() *langs.Language
// OK L38-40: (c ConfigLanguage) Languages() langs.Languages
// OK L42-44: (c ConfigLanguage) LanguagesDefaultFirst() langs.Languages
// OK L46-48: (c ConfigLanguage) PathParser() *paths.PathParser
// OK L50-59: (c ConfigLanguage) LanguagePrefix() string
// OK L61-63: (c ConfigLanguage) BaseURL() urls.BaseURL
// OK L65-67: (c ConfigLanguage) BaseURLLiveReload() urls.BaseURL
// OK L69-71: (c ConfigLanguage) Environment() string
// OK L73-78: (c ConfigLanguage) IsMultihost() bool
// OK L80-82: (c ConfigLanguage) FastRenderMode() bool
// OK L84-86: (c ConfigLanguage) IsMultilingual() bool
// OK L88-90: (c ConfigLanguage) TemplateMetrics() bool
// OK L92-94: (c ConfigLanguage) TemplateMetricsHints() bool
// OK L96-98: (c ConfigLanguage) IsLangDisabled(lang string) bool
// OK L100-102: (c ConfigLanguage) IgnoredLogs() map[string]bool
// OK L104-106: (c ConfigLanguage) NoBuildLock() bool
// OK L108-110: (c ConfigLanguage) NewContentEditor() string
// OK L112-114: (c ConfigLanguage) Timeout() time.Duration
// OK L116-118: (c ConfigLanguage) BaseConfig() config.BaseConfig
// OK L120-122: (c ConfigLanguage) Dirs() config.CommonDirs
// OK L124-126: (c ConfigLanguage) DirsBase() config.CommonDirs
// OK L128-130: (c ConfigLanguage) WorkingDir() string
// OK L132-134: (c ConfigLanguage) Quiet() bool
// OK L136-138: (c ConfigLanguage) Watching() bool
// OK L140-145: (c ConfigLanguage) NewIdentityManager(name string, opts ...identity.ManagerOption) identity.Manager
// OK L147-149: (c ConfigLanguage) ContentTypes() config.ContentTypesProvider
// OK L152-181: (c ConfigLanguage) GetConfigSection(s string) any
// OK L183-185: (c ConfigLanguage) GetConfig() any
// OK L187-189: (c ConfigLanguage) CanonifyURLs() bool
// OK L191-193: (c ConfigLanguage) IsUglyURLs(section string) bool
// OK L195-197: (c ConfigLanguage) IgnoreFile(s string) bool
// OK L199-201: (c ConfigLanguage) DisablePathToLower() bool
// OK L203-205: (c ConfigLanguage) RemovePathAccents() bool
// OK L207-209: (c ConfigLanguage) DefaultContentLanguage() string
// OK L211-213: (c ConfigLanguage) DefaultContentLanguageInSubdir() bool
// OK L215-217: (c ConfigLanguage) SummaryLength() int
// OK L219-221: (c ConfigLanguage) BuildExpired() bool
// OK L223-225: (c ConfigLanguage) BuildFuture() bool
// OK L227-229: (c ConfigLanguage) BuildDrafts() bool
// OK L231-233: (c ConfigLanguage) Running() bool
// OK L235-237: (c ConfigLanguage) PrintUnusedTemplates() bool
// OK L239-241: (c ConfigLanguage) EnableMissingTranslationPlaceholders() bool
// OK L243-245: (c ConfigLanguage) PrintI18nWarnings() bool
// OK L247-249: (c ConfigLanguage) CreateTitle(s string) string
// OK L251-253: (c ConfigLanguage) Pagination() config.Pagination
// OK L255-257: (c ConfigLanguage) StaticDirs() []string
// OK L259-261: (c ConfigLanguage) EnableEmoji() bool
// ---------------------------------------------------------------------------
