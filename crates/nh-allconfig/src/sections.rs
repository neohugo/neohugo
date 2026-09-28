//! The config sections of `ConfigLanguage.GetConfigSection` (Go
//! `config/allconfig/configlanguage.go:GetConfigSection`), fetched by the crates that own the
//! section types with `nh_config::config_provider::config_section::<T>(cfg, name)`.
//!
//! | name | Rust type (`T`) | Go |
//! |---|---|---|
//! | `security` | `nh_config::security::security_config::Config` | `c.config.Security` |
//! | `build` | `nh_config::common_config::BuildConfig` | `c.config.Build` |
//! | `frontmatter` | `nh_page::pagemeta::page_frontmatter::FrontmatterConfig` | `c.config.Frontmatter` |
//! | `caches` | `nh_helpers::cache::filecache::filecache_config::Configs` | `c.config.Caches` |
//! | `markup` | `nh_markup::markup_config::Config` | `c.config.Markup` |
//! | `mediaTypes` | `nh_media::media::media_type::Types` | `c.config.MediaTypes.Config` |
//! | `outputFormats` | `nh_media::output::output_format::Formats` | `c.config.OutputFormats.Config` |
//! | `permalinks` | `BTreeMap<String, BTreeMap<String, String>>` | `c.config.Permalinks` |
//! | `minify` | `nh_transform::minifiers::config::MinifyConfig` | `c.config.Minify` |
//! | `allModules` | `nh_hugofs::modules::module::Modules` | `c.m.Modules` |
//! | `deployment` | `crate::deployconfig::DeployConfig` | `c.config.Deployment` |
//! | `httpCacheCompiled` | `nh_helpers::cache::httpcache::httpcache::ConfigCompiled` | `c.config.C.HTTPCache` |
//!
//! Sections that Go reads from the `*allconfig.Config` of `GetConfig()` directly, added so the
//! lower crates do not need to depend on nh-allconfig (Rust-only names):
//!
//! | name | Rust type | Go field |
//! |---|---|---|
//! | `contentTypes` | `nh_media::media::config::ContentTypes` | `ContentTypes.Config` |
//! | `imaging` | `ConfigNamespace<ImagingConfig, ImagingConfigInternal>` | `Imaging` (with `SourceHash`) |
//! | `httpCache` | `nh_helpers::cache::httpcache::httpcache::Config` | `HTTPCache` |
//! | `services` | `nh_config::services::Config` | `Services` |
//! | `privacy` | `nh_config::privacy::Config` | `Privacy` |
//! | `server` | `nh_config::common_config::Server` | `Server` |
//! | `sitemap` | `nh_config::common_config::SitemapConfig` | `Sitemap` |
//! | `pagination` | `nh_config::common_config::Pagination` | `Pagination` |
//! | `page` | `nh_config::common_config::PageConfig` | `Page` |
//! | `related` | `nh_page::related::Config` | `Related` |
//! | `taxonomies` | `BTreeMap<String, String>` | `Taxonomies` |
//! | `outputs` | `BTreeMap<String, Vec<String>>` | `Outputs` |
//! | `kindOutputFormats` | `BTreeMap<String, Formats>` | `C.KindOutputFormats` |
//! | `cascade` | `ConfigNamespace<Vec<PageMatcherParamsConfig>, Cascade>` | `Cascade` |
//! | `menus` | `ConfigNamespace<Map, BTreeMap<String, Menu>>` | `Menus` |
//! | `segmentFilter` | `crate::segments::SegmentFilter` | `C.SegmentFilter` |
//! | `params` | `go_value::Map` | `Params` |

use std::any::Any;
use std::sync::Arc;

use crate::configlanguage::ConfigLanguage;

/// The section `name` of the language config (`None` = Go's `panic("not implemented: ...")`).
pub fn config_section(c: &ConfigLanguage, name: &str) -> Option<Arc<dyn Any + Send + Sync>> {
    let conf = &c.config;
    let a: Arc<dyn Any + Send + Sync> = match name {
        "security" => Arc::new(conf.security.clone()),
        "build" => Arc::new(conf.build.clone()),
        "frontmatter" => Arc::new(conf.frontmatter.clone()),
        "caches" => Arc::new(conf.caches.clone()),
        "markup" => Arc::new(conf.markup.clone()),
        "mediaTypes" => Arc::new(conf.media_types.as_ref()?.config.clone()),
        "outputFormats" => Arc::new(conf.output_formats.as_ref()?.config.clone()),
        "permalinks" => Arc::new(conf.permalinks.clone()),
        "minify" => Arc::new(conf.minify.clone()),
        "allModules" => Arc::new((*c.modules).clone()),
        "deployment" => Arc::new(conf.deployment.clone()),
        "httpCacheCompiled" => Arc::new(conf.compiled().http_cache.clone()),
        // Rust-only names (see the module docs).
        "contentTypes" => Arc::new(conf.content_types.as_ref()?.config.clone()),
        "imaging" => Arc::new((**conf.imaging.as_ref()?).clone()),
        "httpCache" => Arc::new(conf.http_cache.clone()),
        "services" => Arc::new(conf.services.clone()),
        "privacy" => Arc::new(conf.privacy.clone()),
        "server" => Arc::new(conf.server.clone()),
        "sitemap" => Arc::new(conf.sitemap.clone()),
        "pagination" => Arc::new(conf.pagination.clone()),
        "page" => Arc::new(conf.page.clone()),
        "related" => Arc::new(conf.related.clone()),
        "taxonomies" => Arc::new(conf.taxonomies.clone()),
        "outputs" => Arc::new(conf.outputs.clone()),
        "kindOutputFormats" => Arc::new(conf.compiled().kind_output_formats.clone()),
        "cascade" => Arc::new((**conf.cascade.as_ref()?).clone()),
        "menus" => Arc::new((**conf.menus.as_ref()?).clone()),
        "segmentFilter" => Arc::new(conf.compiled().segment_filter.clone()),
        "params" => Arc::new((*conf.params).clone()),
        _ => return None,
    };
    Some(a)
}
