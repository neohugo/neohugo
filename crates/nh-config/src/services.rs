//! Port of `config/services/servicesConfig.go`.
//!
//! Owner: Wave B task T04 (config-base-media).

use go_value::Value;
use nh_common::{Error, Result};

use crate::config_provider::Provider;
use crate::decode::{FieldRef, weak_decode_into};
use crate::{decode_struct, struct_object};

const SERVICES_CONFIG_KEY: &str = "services";

const DISQUS_SHORTNAME_KEY: &str = "disqusshortname";
const GOOGLE_ANALYTICS_KEY: &str = "googleanalytics";
const RSS_LIMIT_KEY: &str = "rssLimit";

/// Go: `services.Config` (template: `.Site.Config.Services.RSS.Limit`).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Config {
    pub disqus: Disqus,
    pub google_analytics: GoogleAnalytics,
    pub instagram: Instagram,
    /// Deprecated in favor of X in v0.141.0.
    pub twitter: Twitter,
    pub x: X,
    pub rss: Rss,
}

decode_struct!(Config, "services.Config", |s| vec![
    FieldRef::new("Disqus", &mut s.disqus),
    FieldRef::new("GoogleAnalytics", &mut s.google_analytics),
    FieldRef::new("Instagram", &mut s.instagram),
    FieldRef::new("Twitter", &mut s.twitter),
    FieldRef::new("X", &mut s.x),
    FieldRef::new("RSS", &mut s.rss),
]);

struct_object!(Config, "services.Config", |s| [
    ("Disqus", Value::object(s.disqus.clone())),
    ("GoogleAnalytics", Value::object(s.google_analytics.clone())),
    ("Instagram", Value::object(s.instagram.clone())),
    ("Twitter", Value::object(s.twitter.clone())),
    ("X", Value::object(s.x.clone())),
    ("RSS", Value::object(s.rss.clone())),
]);

/// Go: `services.Disqus`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Disqus {
    /// A Shortname is the unique identifier assigned to a Disqus site.
    pub shortname: String,
}

decode_struct!(Disqus, "services.Disqus", |s| vec![FieldRef::new(
    "Shortname",
    &mut s.shortname
)]);

struct_object!(Disqus, "services.Disqus", |s| [(
    "Shortname",
    Value::string(s.shortname.as_str())
),]);

/// Go: `services.GoogleAnalytics`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct GoogleAnalytics {
    /// The GA tracking ID.
    pub id: String,
}

decode_struct!(GoogleAnalytics, "services.GoogleAnalytics", |s| vec![
    FieldRef::new("ID", &mut s.id)
]);

struct_object!(GoogleAnalytics, "services.GoogleAnalytics", |s| [(
    "ID",
    Value::string(s.id.as_str())
),]);

/// Go: `services.Instagram`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Instagram {
    pub disable_inline_css: bool,
    /// App or Client Access Token.
    pub access_token: String,
}

decode_struct!(Instagram, "services.Instagram", |s| vec![
    FieldRef::new("DisableInlineCSS", &mut s.disable_inline_css),
    FieldRef::new("AccessToken", &mut s.access_token),
]);

struct_object!(Instagram, "services.Instagram", |s| [
    ("DisableInlineCSS", Value::Bool(s.disable_inline_css)),
    ("AccessToken", Value::string(s.access_token.as_str())),
]);

/// Go: `services.Twitter` (deprecated in favor of X in v0.141.0).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Twitter {
    pub disable_inline_css: bool,
}

decode_struct!(Twitter, "services.Twitter", |s| vec![FieldRef::new(
    "DisableInlineCSS",
    &mut s.disable_inline_css
)]);

struct_object!(Twitter, "services.Twitter", |s| [(
    "DisableInlineCSS",
    Value::Bool(s.disable_inline_css)
),]);

/// Go: `services.X`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct X {
    pub disable_inline_css: bool,
}

decode_struct!(X, "services.X", |s| vec![FieldRef::new(
    "DisableInlineCSS",
    &mut s.disable_inline_css
)]);

struct_object!(X, "services.X", |s| [(
    "DisableInlineCSS",
    Value::Bool(s.disable_inline_css)
),]);

/// Go: `services.RSS`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Rss {
    /// Limit the number of pages (`services.rss.limit`, falling back to legacy `rssLimit`;
    /// 0 -> -1).
    pub limit: i64,
}

decode_struct!(Rss, "services.RSS", |s| vec![FieldRef::new(
    "Limit",
    &mut s.limit
)]);

struct_object!(Rss, "services.RSS", |s| [("Limit", Value::int(s.limit)),]);

/// Go: `services.DecodeConfig(cfg)`.
// Go: config/services/servicesConfig.go:DecodeConfig
pub fn decode_config(cfg: &dyn Provider) -> Result<Config> {
    let (c, err) = decode_config_partial(cfg);
    match err {
        Some(e) => Err(e),
        None => Ok(c),
    }
}

/// [`decode_config`] returning the (partially decoded) config along with the error, like Go.
pub fn decode_config_partial(cfg: &dyn Provider) -> (Config, Option<Error>) {
    let mut c = Config::default();
    let m = cfg.get_string_map(SERVICES_CONFIG_KEY);

    let err = weak_decode_into(&Value::map(m), &mut c).err();

    // Keep backwards compatibility.
    if c.google_analytics.id.is_empty() {
        // Try the global config
        c.google_analytics.id = cfg.get_string(GOOGLE_ANALYTICS_KEY);
    }
    if c.disqus.shortname.is_empty() {
        c.disqus.shortname = cfg.get_string(DISQUS_SHORTNAME_KEY);
    }

    if c.rss.limit == 0 {
        c.rss.limit = cfg.get_int(RSS_LIMIT_KEY);
        if c.rss.limit == 0 {
            c.rss.limit = -1;
        }
    }

    (c, err)
}

/// Template value of `services.Config` (struct with fields `Disqus`, `GoogleAnalytics`,
/// `Instagram`, `Twitter`, `X`, `RSS`; `RSS.Limit` is an `int`).
pub fn config_to_value(c: &Config) -> Value {
    Value::object(c.clone())
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: config/services/servicesConfig.go (110 lines; 1/1 funcs executed)
//   types: Config, Disqus, GoogleAnalytics, Instagram, Twitter, X, RSS
// OK L88-110: DecodeConfig(cfg config.Provider) (c Config, err error)
// ---------------------------------------------------------------------------
