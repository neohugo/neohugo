//! Port of `config/services/servicesConfig.go`.
//!
//! Owner: Wave B task T04 (config-base-media).


use std::any::Any;
use std::borrow::Cow;

use go_value::{HostCtx, Object, Value};
use nh_common::Result;

use crate::config_provider::Provider;

/// Go: `services.Config` (template: `.Site.Config.Services.RSS.Limit`).
#[derive(Clone, Debug, Default)]
pub struct Config {
    pub disqus_shortname: String,
    pub google_analytics_id: String,
    pub instagram_disable_inline_css: bool,
    pub instagram_access_token: String,
    pub twitter_disable_inline_css: bool,
    pub x_disable_inline_css: bool,
    /// `services.rss.limit`, falling back to legacy `rssLimit`; 0 -> -1.
    pub rss_limit: i64,
}

/// Go: `services.DecodeConfig(cfg)`.
// Go: config/services/servicesConfig.go:DecodeConfig
pub fn decode_config(cfg: &dyn Provider) -> Result<Config> {
    todo!()
}

/// Template value of `services.Config` (struct with fields `Disqus`, `GoogleAnalytics`,
/// `Instagram`, `Twitter`, `X`, `RSS`; `RSS.Limit` is an `int`).
pub fn config_to_value(c: &Config) -> Value {
    todo!("objects for services.Config / services.RSS with field() access")
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: config/services/servicesConfig.go (110 lines; 1/1 funcs executed)
//   types: Config, Disqus, GoogleAnalytics, Instagram, Twitter, X, RSS
// EX L88-110: DecodeConfig(cfg config.Provider) (c Config, err error)
// ---------------------------------------------------------------------------
