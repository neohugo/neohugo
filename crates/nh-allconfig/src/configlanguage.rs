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

use crate::allconfig::{Config, Configs};

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
}

impl AllProvider for ConfigLanguage {
    fn language(&self) -> Arc<Language> { self.language.clone() }
    fn languages(&self) -> Languages { self.languages.clone() }
    fn languages_default_first(&self) -> Languages { self.languages_default_first.clone() }
    // Go: config/allconfig/configlanguage.go:LanguagePrefix
    fn language_prefix(&self) -> String { todo!() }
    fn base_url(&self) -> BaseURL { todo!() }
    fn base_url_live_reload(&self) -> BaseURL { todo!() }
    fn path_parser(&self) -> Arc<PathParser> { self.path_parser.clone() }
    fn environment(&self) -> String { self.config.root.environment.clone() }
    fn is_multihost(&self) -> bool { todo!() }
    fn is_multilingual(&self) -> bool { self.languages.len() > 1 }
    fn no_build_lock(&self) -> bool { self.config.root.no_build_lock }
    fn base_config(&self) -> BaseConfig { self.base_config.clone() }
    fn dirs(&self) -> CommonDirs { self.config.root.common_dirs.clone() }
    fn quiet(&self) -> bool { self.config.internal.quiet }
    fn dirs_base(&self) -> CommonDirs { todo!() }
    fn content_types(&self) -> Arc<dyn ContentTypesProvider> { todo!() }
    // Go: config/allconfig/configlanguage.go:GetConfigSection
    fn get_config_section(&self, name: &str) -> Arc<dyn Any + Send + Sync> { todo!() }
    fn get_config(&self) -> Arc<dyn Any + Send + Sync> { self.config.clone() }
    fn canonify_urls(&self) -> bool { self.config.root.canonify_urls }
    fn disable_path_to_lower(&self) -> bool { self.config.root.disable_path_to_lower }
    fn remove_path_accents(&self) -> bool { self.config.root.remove_path_accents }
    fn is_ugly_urls(&self, section: &str) -> bool { todo!() }
    fn default_content_language(&self) -> String { self.config.root.default_content_language.clone() }
    fn default_content_language_in_subdir(&self) -> bool { self.config.root.default_content_language_in_subdir }
    fn is_lang_disabled(&self, lang: &str) -> bool { todo!() }
    fn summary_length(&self) -> i64 { self.config.root.summary_length }
    fn pagination(&self) -> Pagination { self.config.pagination.clone() }
    fn build_expired(&self) -> bool { self.config.root.build_expired }
    fn build_future(&self) -> bool { self.config.root.build_future }
    fn build_drafts(&self) -> bool { self.config.root.build_drafts }
    fn running(&self) -> bool { self.config.internal.running }
    fn watching(&self) -> bool { self.config.internal.watch }
    fn fast_render_mode(&self) -> bool { self.config.internal.fast_render_mode }
    fn print_unused_templates(&self) -> bool { self.config.root.print_unused_templates }
    fn enable_missing_translation_placeholders(&self) -> bool { self.config.root.enable_missing_translation_placeholders }
    fn template_metrics(&self) -> bool { self.config.root.template_metrics }
    fn template_metrics_hints(&self) -> bool { self.config.root.template_metrics_hints }
    fn print_i18n_warnings(&self) -> bool { self.config.root.print_i18n_warnings }
    fn create_title(&self, s: &str) -> String { todo!() }
    fn ignore_file(&self, s: &str) -> bool { todo!() }
    fn new_content_editor(&self) -> String { self.config.root.new_content_editor.clone() }
    fn timeout(&self) -> Duration { todo!() }
    fn static_dirs(&self) -> Vec<String> { todo!() }
    fn ignored_logs(&self) -> BTreeSet<String> { todo!() }
    fn working_dir(&self) -> String { self.base_config.working_dir.clone() }
    fn enable_emoji(&self) -> bool { self.config.root.enable_emoji }
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: config/allconfig/configlanguage.go (261 lines; 33/47 funcs executed)
//   types: ConfigLanguage
// EX L34-36: (c ConfigLanguage) Language() *langs.Language
//    L38-40: (c ConfigLanguage) Languages() langs.Languages
//    L42-44: (c ConfigLanguage) LanguagesDefaultFirst() langs.Languages
// EX L46-48: (c ConfigLanguage) PathParser() *paths.PathParser
// EX L50-59: (c ConfigLanguage) LanguagePrefix() string
// EX L61-63: (c ConfigLanguage) BaseURL() urls.BaseURL
//    L65-67: (c ConfigLanguage) BaseURLLiveReload() urls.BaseURL
// EX L69-71: (c ConfigLanguage) Environment() string
// EX L73-78: (c ConfigLanguage) IsMultihost() bool
//    L80-82: (c ConfigLanguage) FastRenderMode() bool
// EX L84-86: (c ConfigLanguage) IsMultilingual() bool
// EX L88-90: (c ConfigLanguage) TemplateMetrics() bool
//    L92-94: (c ConfigLanguage) TemplateMetricsHints() bool
//    L96-98: (c ConfigLanguage) IsLangDisabled(lang string) bool
// EX L100-102: (c ConfigLanguage) IgnoredLogs() map[string]bool
// EX L104-106: (c ConfigLanguage) NoBuildLock() bool
//    L108-110: (c ConfigLanguage) NewContentEditor() string
// EX L112-114: (c ConfigLanguage) Timeout() time.Duration
// EX L116-118: (c ConfigLanguage) BaseConfig() config.BaseConfig
// EX L120-122: (c ConfigLanguage) Dirs() config.CommonDirs
//    L124-126: (c ConfigLanguage) DirsBase() config.CommonDirs
// EX L128-130: (c ConfigLanguage) WorkingDir() string
//    L132-134: (c ConfigLanguage) Quiet() bool
// EX L136-138: (c ConfigLanguage) Watching() bool
// EX L140-145: (c ConfigLanguage) NewIdentityManager(name string, opts ...identity.ManagerOption) identity.Manager
// EX L147-149: (c ConfigLanguage) ContentTypes() config.ContentTypesProvider
// EX L152-181: (c ConfigLanguage) GetConfigSection(s string) any
// EX L183-185: (c ConfigLanguage) GetConfig() any
// EX L187-189: (c ConfigLanguage) CanonifyURLs() bool
// EX L191-193: (c ConfigLanguage) IsUglyURLs(section string) bool
// EX L195-197: (c ConfigLanguage) IgnoreFile(s string) bool
// EX L199-201: (c ConfigLanguage) DisablePathToLower() bool
// EX L203-205: (c ConfigLanguage) RemovePathAccents() bool
// EX L207-209: (c ConfigLanguage) DefaultContentLanguage() string
// EX L211-213: (c ConfigLanguage) DefaultContentLanguageInSubdir() bool
//    L215-217: (c ConfigLanguage) SummaryLength() int
// EX L219-221: (c ConfigLanguage) BuildExpired() bool
// EX L223-225: (c ConfigLanguage) BuildFuture() bool
// EX L227-229: (c ConfigLanguage) BuildDrafts() bool
// EX L231-233: (c ConfigLanguage) Running() bool
//    L235-237: (c ConfigLanguage) PrintUnusedTemplates() bool
// EX L239-241: (c ConfigLanguage) EnableMissingTranslationPlaceholders() bool
//    L243-245: (c ConfigLanguage) PrintI18nWarnings() bool
//    L247-249: (c ConfigLanguage) CreateTitle(s string) string
// EX L251-253: (c ConfigLanguage) Pagination() config.Pagination
//    L255-257: (c ConfigLanguage) StaticDirs() []string
// EX L259-261: (c ConfigLanguage) EnableEmoji() bool
// ---------------------------------------------------------------------------
