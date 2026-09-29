//! Port of `config/configProvider.go`.
//!
//! Owner: Wave B task T04 (config-base-media).

//! The two config interfaces of Go `config/configProvider.go`:
//! * [`Provider`]: the raw, case-insensitive key/value config tree (`maps.Params`), used while
//!   loading and by the section decoders.
//! * [`AllProvider`]: the compiled per-language config every build component reads. Implemented by
//!   `nh_allconfig::configlanguage::ConfigLanguage`. Config *sections* whose types live in higher
//!   crates are fetched with [`AllProvider::get_config_section`] and downcast by their owner crate
//!   (Go does `cfg.GetConfigSection("markup").(markup_config.Config)`).

use std::any::Any;
use std::sync::Arc;
use std::time::Duration;

use go_value::{Map, Value};
use nh_common::maps::params::KeyParams;
use nh_common::paths::pathparser::PathParser;
use nh_common::urls::BaseURL;
use nh_langs::language::{Language, Languages};

use crate::common_config::{BaseConfig, CommonDirs, Pagination};

/// Go: `config.Provider`. Keys are case-insensitive and dot-separated (`"markup.goldmark"`).
pub trait Provider: Send + Sync {
    fn get_string(&self, key: &str) -> String;
    fn get_int(&self, key: &str) -> i64;
    fn get_bool(&self, key: &str) -> bool;
    /// `maps.Params` at key (nil -> None).
    fn get_params(&self, key: &str) -> Option<Map>;
    fn get_string_map(&self, key: &str) -> Map;
    fn get_string_map_string(&self, key: &str) -> Map;
    fn get_string_slice(&self, key: &str) -> Vec<String>;
    /// `Value::Invalid` when not set.
    fn get(&self, key: &str) -> Value;
    fn set(&self, key: &str, value: Value);
    fn keys(&self) -> Vec<String>;
    fn merge(&self, key: &str, value: Value);
    fn set_defaults(&self, params: &Map);
    fn set_default_merge_strategy(&self);
    fn walk_params(&self, walk_fn: &mut dyn FnMut(&[KeyParams]) -> bool);
    fn is_set(&self, key: &str) -> bool;
}

/// Go: `config.GetStringSlicePreserveString(cfg, key)`: a string value is not split into
/// fields.
// Go: config/configProvider.go:GetStringSlicePreserveString
pub fn get_string_slice_preserve_string(cfg: &dyn Provider, key: &str) -> Vec<String> {
    let sd = cfg.get(key);
    nh_common::types::convert::to_string_slice_preserve_string(&sd)
        .iter()
        .map(|s| String::from_utf8_lossy(s).into_owned())
        .collect()
}

/// Go: `config.ContentTypesProvider` (implemented by `media.ContentTypes`).
pub trait ContentTypesProvider: Send + Sync {
    fn is_content_suffix(&self, suffix: &str) -> bool;
    fn is_content_file(&self, filename: &str) -> bool;
    fn is_index_content_file(&self, filename: &str) -> bool;
    fn is_html_suffix(&self, suffix: &str) -> bool;
}

/// Go: `config.AllProvider` — the per-language compiled configuration.
pub trait AllProvider: Send + Sync {
    fn language(&self) -> Arc<Language>;
    fn languages(&self) -> Languages;
    fn languages_default_first(&self) -> Languages;
    /// `""` for the default language unless `defaultContentLanguageInSubdir`; `"th"` for th.
    fn language_prefix(&self) -> String;
    fn base_url(&self) -> BaseURL;
    fn base_url_live_reload(&self) -> BaseURL;
    fn path_parser(&self) -> Arc<PathParser>;
    fn environment(&self) -> String;
    fn is_multihost(&self) -> bool;
    fn is_multilingual(&self) -> bool;
    fn no_build_lock(&self) -> bool;
    fn base_config(&self) -> BaseConfig;
    fn dirs(&self) -> CommonDirs;
    fn quiet(&self) -> bool;
    fn dirs_base(&self) -> CommonDirs;
    fn content_types(&self) -> Arc<dyn ContentTypesProvider>;
    /// Go: `GetConfigSection(name) any`. Known names: security, build, frontmatter, caches, markup,
    /// mediaTypes, outputFormats, permalinks, minify, allModules, deployment, httpCacheCompiled.
    fn get_config_section(&self, name: &str) -> Arc<dyn Any + Send + Sync>;
    /// Go: `GetConfig() any` — the whole `*allconfig.Config`.
    fn get_config(&self) -> Arc<dyn Any + Send + Sync>;
    fn canonify_urls(&self) -> bool;
    fn disable_path_to_lower(&self) -> bool;
    fn remove_path_accents(&self) -> bool;
    fn is_ugly_urls(&self, section: &str) -> bool;
    fn default_content_language(&self) -> String;
    fn default_content_language_in_subdir(&self) -> bool;
    fn is_lang_disabled(&self, lang: &str) -> bool;
    fn summary_length(&self) -> i64;
    fn pagination(&self) -> Pagination;
    fn build_expired(&self) -> bool;
    fn build_future(&self) -> bool;
    fn build_drafts(&self) -> bool;
    fn running(&self) -> bool;
    fn watching(&self) -> bool;
    fn fast_render_mode(&self) -> bool;
    fn print_unused_templates(&self) -> bool;
    fn enable_missing_translation_placeholders(&self) -> bool;
    fn template_metrics(&self) -> bool;
    fn template_metrics_hints(&self) -> bool;
    fn print_i18n_warnings(&self) -> bool;
    /// Go: `CreateTitle(s)` = `helpers.GetTitleFunc(titleCaseStyle)`.
    fn create_title(&self, s: &str) -> String;
    /// Go: `CreateTitle(s)` over Go string bytes (invalid UTF-8 included). The default converts
    /// invalid UTF-8 lossily; `nh-allconfig`'s provider is exact.
    fn create_title_bytes(&self, s: &[u8]) -> Vec<u8> {
        self.create_title(&String::from_utf8_lossy(s)).into_bytes()
    }
    fn ignore_file(&self, s: &str) -> bool;
    fn new_content_editor(&self) -> String;
    fn timeout(&self) -> Duration;
    fn static_dirs(&self) -> Vec<String>;
    fn ignored_logs(&self) -> std::collections::BTreeSet<String>;
    fn working_dir(&self) -> String;
    fn enable_emoji(&self) -> bool;
}

/// Downcast helper for [`AllProvider::get_config_section`].
pub fn config_section<T: Any + Send + Sync>(cfg: &dyn AllProvider, name: &str) -> Arc<T> {
    cfg.get_config_section(name)
        .downcast::<T>()
        .unwrap_or_else(|_| panic!("config section {name}: unexpected type"))
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: config/configProvider.go (112 lines; 0/1 funcs executed)
//   types: AllProvider, ContentTypesProvider, Provider
// OK L109-112: GetStringSlicePreserveString(cfg Provider, key string) []string
// ---------------------------------------------------------------------------
