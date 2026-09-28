//! Port of `config/privacy/privacyConfig.go`.
//!
//! Owner: Wave B task T04 (config-base-media).

use go_value::Value;
use nh_common::{Error, Result};

use crate::config_provider::Provider;
use crate::decode::{FieldRef, weak_decode_into};
use crate::{decode_struct, struct_object};

const PRIVACY_CONFIG_KEY: &str = "privacy";

/// Go: `privacy.Service`: the common values for a service in a policy definition.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Service {
    pub disable: bool,
}

decode_struct!(Service, "privacy.Service", |s| vec![FieldRef::new(
    "Disable",
    &mut s.disable
)]);

/// Go: `privacy.Config` (decoded; reachable from templates as `.Site.Config.Privacy`).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Config {
    pub disqus: Disqus,
    pub google_analytics: GoogleAnalytics,
    pub instagram: Instagram,
    /// Deprecated in favor of X in v0.141.0.
    pub twitter: Twitter,
    pub vimeo: Vimeo,
    pub youtube: YouTube,
    pub x: X,
}

decode_struct!(Config, "privacy.Config", |s| vec![
    FieldRef::new("Disqus", &mut s.disqus),
    FieldRef::new("GoogleAnalytics", &mut s.google_analytics),
    FieldRef::new("Instagram", &mut s.instagram),
    FieldRef::new("Twitter", &mut s.twitter),
    FieldRef::new("Vimeo", &mut s.vimeo),
    FieldRef::new("YouTube", &mut s.youtube),
    FieldRef::new("X", &mut s.x),
]);

struct_object!(Config, "privacy.Config", |s| [
    ("Disqus", Value::object(s.disqus.clone())),
    ("GoogleAnalytics", Value::object(s.google_analytics.clone())),
    ("Instagram", Value::object(s.instagram.clone())),
    ("Twitter", Value::object(s.twitter.clone())),
    ("Vimeo", Value::object(s.vimeo.clone())),
    ("YouTube", Value::object(s.youtube.clone())),
    ("X", Value::object(s.x.clone())),
]);

/// Go: `privacy.Disqus`: the privacy configuration settings related to the Disqus template.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Disqus {
    pub service: Service,
}

decode_struct!(Disqus, "privacy.Disqus", |s| vec![FieldRef::squash(
    "Service",
    &mut s.service
)]);

struct_object!(Disqus, "privacy.Disqus", |s| [
    ("Service", Value::object(s.service.clone())),
] promoted [("Disable", Value::Bool(s.service.disable))]);

/// Go: `privacy.GoogleAnalytics`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct GoogleAnalytics {
    pub service: Service,
    /// Enabling this will make the GA templates respect the "Do Not Track" HTTP header.
    pub respect_do_not_track: bool,
}

decode_struct!(GoogleAnalytics, "privacy.GoogleAnalytics", |s| vec![
    FieldRef::squash("Service", &mut s.service),
    FieldRef::new("RespectDoNotTrack", &mut s.respect_do_not_track),
]);

struct_object!(GoogleAnalytics, "privacy.GoogleAnalytics", |s| [
    ("Service", Value::object(s.service.clone())),
    ("RespectDoNotTrack", Value::Bool(s.respect_do_not_track)),
] promoted [("Disable", Value::Bool(s.service.disable))]);

/// Go: `privacy.Instagram`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Instagram {
    pub service: Service,
    /// If simple mode is enabled, a static and no-JS version of the Instagram image card will
    /// be built.
    pub simple: bool,
}

decode_struct!(Instagram, "privacy.Instagram", |s| vec![
    FieldRef::squash("Service", &mut s.service),
    FieldRef::new("Simple", &mut s.simple),
]);

struct_object!(Instagram, "privacy.Instagram", |s| [
    ("Service", Value::object(s.service.clone())),
    ("Simple", Value::Bool(s.simple)),
] promoted [("Disable", Value::Bool(s.service.disable))]);

/// Go: `privacy.Twitter` (deprecated in favor of X in v0.141.0).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Twitter {
    pub service: Service,
    pub enable_dnt: bool,
    pub simple: bool,
}

decode_struct!(Twitter, "privacy.Twitter", |s| vec![
    FieldRef::squash("Service", &mut s.service),
    FieldRef::new("EnableDNT", &mut s.enable_dnt),
    FieldRef::new("Simple", &mut s.simple),
]);

struct_object!(Twitter, "privacy.Twitter", |s| [
    ("Service", Value::object(s.service.clone())),
    ("EnableDNT", Value::Bool(s.enable_dnt)),
    ("Simple", Value::Bool(s.simple)),
] promoted [("Disable", Value::Bool(s.service.disable))]);

/// Go: `privacy.Vimeo`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Vimeo {
    pub service: Service,
    pub enable_dnt: bool,
    pub simple: bool,
}

decode_struct!(Vimeo, "privacy.Vimeo", |s| vec![
    FieldRef::squash("Service", &mut s.service),
    FieldRef::new("EnableDNT", &mut s.enable_dnt),
    FieldRef::new("Simple", &mut s.simple),
]);

struct_object!(Vimeo, "privacy.Vimeo", |s| [
    ("Service", Value::object(s.service.clone())),
    ("EnableDNT", Value::Bool(s.enable_dnt)),
    ("Simple", Value::Bool(s.simple)),
] promoted [("Disable", Value::Bool(s.service.disable))]);

/// Go: `privacy.YouTube`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct YouTube {
    pub service: Service,
    /// When you turn on privacy-enhanced mode, YouTube won’t store information about visitors
    /// on your website unless the user plays the embedded video.
    pub privacy_enhanced: bool,
}

decode_struct!(YouTube, "privacy.YouTube", |s| vec![
    FieldRef::squash("Service", &mut s.service),
    FieldRef::new("PrivacyEnhanced", &mut s.privacy_enhanced),
]);

struct_object!(YouTube, "privacy.YouTube", |s| [
    ("Service", Value::object(s.service.clone())),
    ("PrivacyEnhanced", Value::Bool(s.privacy_enhanced)),
] promoted [("Disable", Value::Bool(s.service.disable))]);

/// Go: `privacy.X`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct X {
    pub service: Service,
    pub enable_dnt: bool,
    pub simple: bool,
}

decode_struct!(X, "privacy.X", |s| vec![
    FieldRef::squash("Service", &mut s.service),
    FieldRef::new("EnableDNT", &mut s.enable_dnt),
    FieldRef::new("Simple", &mut s.simple),
]);

struct_object!(X, "privacy.X", |s| [
    ("Service", Value::object(s.service.clone())),
    ("EnableDNT", Value::Bool(s.enable_dnt)),
    ("Simple", Value::Bool(s.simple)),
] promoted [("Disable", Value::Bool(s.service.disable))]);

struct_object!(Service, "privacy.Service", |s| [(
    "Disable",
    Value::Bool(s.disable)
),]);

/// DecodeConfig creates a privacy Config from a given Hugo configuration.
// Go: config/privacy/privacyConfig.go:DecodeConfig
pub fn decode_config(cfg: &dyn Provider) -> Result<Config> {
    let (pc, err) = decode_config_partial(cfg);
    match err {
        Some(e) => Err(e),
        None => Ok(pc),
    }
}

/// [`decode_config`] returning the (partially decoded) config along with the error, like Go.
pub fn decode_config_partial(cfg: &dyn Provider) -> (Config, Option<Error>) {
    let mut pc = Config::default();
    if !cfg.is_set(PRIVACY_CONFIG_KEY) {
        return (pc, None);
    }

    let m = cfg.get_string_map(PRIVACY_CONFIG_KEY);

    let err = weak_decode_into(&Value::map(m), &mut pc).err();

    (pc, err)
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: config/privacy/privacyConfig.go (124 lines; 1/1 funcs executed)
//   types: Service, Config, Disqus, GoogleAnalytics, Instagram, Twitter, Vimeo, YouTube, X
// OK L114-124: DecodeConfig(cfg config.Provider) (pc Config, err error)
// ---------------------------------------------------------------------------
