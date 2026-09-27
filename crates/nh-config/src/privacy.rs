//! Port of `config/privacy/privacyConfig.go`.
//!
//! Owner: Wave B task T04 (config-base-media).


use nh_common::Result;

use crate::config_provider::Provider;

/// Go: `privacy.Config` (decoded; only printed via `.Site.Config.Privacy`, unused by seeksnack).
#[derive(Clone, Debug, Default)]
pub struct Config {
    pub disqus: Service,
    pub google_analytics: Service,
    pub instagram: Service,
    pub twitter: Service,
    pub vimeo: Service,
    pub youtube: Service,
    pub x: Service,
}

/// Go: `privacy.Service` (the per-service structs are flattened: Disable, RespectDoNotTrack,
/// Simple, PrivacyEnhanced, EnableDNT).
#[derive(Clone, Debug, Default)]
pub struct Service {
    pub disable: bool,
    pub respect_do_not_track: bool,
    pub simple: bool,
    pub privacy_enhanced: bool,
    pub enable_dnt: bool,
}

// Go: config/privacy/privacyConfig.go:DecodeConfig
pub fn decode_config(cfg: &dyn Provider) -> Result<Config> {
    todo!()
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: config/privacy/privacyConfig.go (124 lines; 1/1 funcs executed)
//   types: Service, Config, Disqus, GoogleAnalytics, Instagram, Twitter, Vimeo, YouTube, X
// EX L114-124: DecodeConfig(cfg config.Provider) (pc Config, err error)
// ---------------------------------------------------------------------------
