//! The `config` oracle (tools/go-oracle/nh-markup/config): `markup_config.Decode` of TOML site
//! configs (dumps and error texts), the converter registry of `markup.NewConverterProvider`
//! (`Get`, `IsGoldmark`, the defaultMarkdownHandler error) and `ResolveMarkup`.

mod common;

use std::any::Any;
use std::sync::Arc;
use std::time::Duration;

use common::*;
use nh_common::paths::pathparser::PathParser;
use nh_common::urls::BaseURL;
use nh_config::common_config::{BaseConfig, CommonDirs, Pagination};
use nh_config::config_provider::{AllProvider, ContentTypesProvider};
use nh_langs::language::{Language, Languages};
use nh_markup::converter::converter::ProviderConfig;
use nh_markup::markup::{new_converter_provider, resolve_markup};
use nh_markup::markup_config::Config as MarkupConfig;

#[test]
fn decode() {
    let fx = read_fixture("config/config.json.gz");
    let mut n = 0;
    for c in fx["decoded"].as_array().unwrap() {
        let toml = c["toml"].as_str().unwrap();
        match decode_markup(toml) {
            Ok(m) => {
                let want = c["dump"]
                    .as_str()
                    .unwrap_or_else(|| panic!("{toml:?}: want error {}", c["err"]));
                assert_eq!(dump_markup_config(&m), want, "{toml:?}");
            }
            Err(e) => assert_eq!(Some(e.to_string().as_str()), c["err"].as_str(), "{toml:?}"),
        }
        n += 1;
    }
    eprintln!("config: {n} decoded configs identical");
}

/// A `config.AllProvider` that only answers what the markup registry asks.
struct Conf(Arc<MarkupConfig>);

impl AllProvider for Conf {
    fn language(&self) -> Arc<Language> {
        unimplemented!()
    }
    fn languages(&self) -> Languages {
        unimplemented!()
    }
    fn languages_default_first(&self) -> Languages {
        unimplemented!()
    }
    fn language_prefix(&self) -> String {
        unimplemented!()
    }
    fn base_url(&self) -> BaseURL {
        unimplemented!()
    }
    fn base_url_live_reload(&self) -> BaseURL {
        unimplemented!()
    }
    fn path_parser(&self) -> Arc<PathParser> {
        unimplemented!()
    }
    fn environment(&self) -> String {
        unimplemented!()
    }
    fn is_multihost(&self) -> bool {
        unimplemented!()
    }
    fn is_multilingual(&self) -> bool {
        unimplemented!()
    }
    fn no_build_lock(&self) -> bool {
        unimplemented!()
    }
    fn base_config(&self) -> BaseConfig {
        unimplemented!()
    }
    fn dirs(&self) -> CommonDirs {
        unimplemented!()
    }
    fn quiet(&self) -> bool {
        unimplemented!()
    }
    fn dirs_base(&self) -> CommonDirs {
        unimplemented!()
    }
    fn content_types(&self) -> Arc<dyn ContentTypesProvider> {
        Arc::new(nh_media::media::config::default_content_types())
    }
    fn get_config_section(&self, name: &str) -> Arc<dyn Any + Send + Sync> {
        assert_eq!(name, "markup");
        self.0.clone()
    }
    fn get_config(&self) -> Arc<dyn Any + Send + Sync> {
        unimplemented!()
    }
    fn canonify_urls(&self) -> bool {
        unimplemented!()
    }
    fn disable_path_to_lower(&self) -> bool {
        unimplemented!()
    }
    fn remove_path_accents(&self) -> bool {
        unimplemented!()
    }
    fn is_ugly_urls(&self, _section: &str) -> bool {
        unimplemented!()
    }
    fn default_content_language(&self) -> String {
        unimplemented!()
    }
    fn default_content_language_in_subdir(&self) -> bool {
        unimplemented!()
    }
    fn is_lang_disabled(&self, _lang: &str) -> bool {
        unimplemented!()
    }
    fn summary_length(&self) -> i64 {
        unimplemented!()
    }
    fn pagination(&self) -> Pagination {
        unimplemented!()
    }
    fn build_expired(&self) -> bool {
        unimplemented!()
    }
    fn build_future(&self) -> bool {
        unimplemented!()
    }
    fn build_drafts(&self) -> bool {
        unimplemented!()
    }
    fn running(&self) -> bool {
        unimplemented!()
    }
    fn watching(&self) -> bool {
        unimplemented!()
    }
    fn fast_render_mode(&self) -> bool {
        unimplemented!()
    }
    fn print_unused_templates(&self) -> bool {
        unimplemented!()
    }
    fn enable_missing_translation_placeholders(&self) -> bool {
        unimplemented!()
    }
    fn template_metrics(&self) -> bool {
        unimplemented!()
    }
    fn template_metrics_hints(&self) -> bool {
        unimplemented!()
    }
    fn print_i18n_warnings(&self) -> bool {
        unimplemented!()
    }
    fn create_title(&self, _s: &str) -> String {
        unimplemented!()
    }
    fn ignore_file(&self, _s: &str) -> bool {
        unimplemented!()
    }
    fn new_content_editor(&self) -> String {
        unimplemented!()
    }
    fn timeout(&self) -> Duration {
        unimplemented!()
    }
    fn static_dirs(&self) -> Vec<String> {
        unimplemented!()
    }
    fn ignored_logs(&self) -> std::collections::BTreeSet<String> {
        unimplemented!()
    }
    fn working_dir(&self) -> String {
        unimplemented!()
    }
    fn enable_emoji(&self) -> bool {
        false
    }
}

#[test]
fn registry() {
    let fx = read_fixture("config/config.json.gz");
    for r in fx["registries"].as_array().unwrap() {
        let h = r["handler"].as_str().unwrap();
        let m = decode_markup(&format!("[markup]\ndefaultMarkdownHandler = \"{h}\"")).unwrap();
        let cfg = ProviderConfig {
            conf: Arc::new(Conf(Arc::new(m))),
            exec: nh_config::hexec::Exec::new_with_env(Default::default(), "", &[], None),
            highlighter: None,
            logger: None,
        };
        match new_converter_provider(cfg) {
            Err(e) => assert_eq!(Some(e.to_string().as_str()), r["err"].as_str(), "{h}"),
            Ok(cp) => {
                assert!(r["err"].is_null(), "{h}: want error {}", r["err"]);
                for (name, want) in r["is_goldmark"].as_object().unwrap() {
                    assert_eq!(
                        cp.is_goldmark(name),
                        want.as_bool().unwrap(),
                        "{h}: IsGoldmark({name})"
                    );
                    let got = cp.get(name).map(|p| p.name().to_string());
                    let want = r["get"]
                        .get(name)
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());
                    assert_eq!(got, want, "{h}: Get({name})");
                }
            }
        }
    }
    for (name, want) in fx["resolve"].as_object().unwrap() {
        assert_eq!(
            resolve_markup(name),
            want.as_str().unwrap(),
            "ResolveMarkup({name})"
        );
    }
}

#[test]
fn chroma_lexer_lookup() {
    let fx = read_fixture("config/config.json.gz");
    for (name, want) in fx["chroma"].as_object().unwrap() {
        assert_eq!(
            nh_markup::highlight::chromalexers::get(name.as_bytes()),
            want.as_bool().unwrap(),
            "chromalexers.Get({name:?})"
        );
    }
}
