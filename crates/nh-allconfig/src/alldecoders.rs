//! Port of `config/allconfig/alldecoders.go`.
//!
//! Owner: Wave B task T09 (allconfig-modules).


//! Go `alldecoders.go`: one decoder per top-level key, run in (weight, key) order.

use nh_common::Result;
use nh_config::config_provider::Provider;

use crate::allconfig::Config;

/// Go: `decodeWeight`.
pub struct DecodeWeight {
    pub key: &'static str,
    pub weight: i64,
    pub internal_or_deprecated: bool,
    pub decode: fn(&DecodeWeight, &mut DecodeConfig<'_>) -> Result<()>,
}

/// Go: `decodeConfig`.
pub struct DecodeConfig<'a> {
    pub p: &'a dyn Provider,
    pub c: &'a mut Config,
    pub bcfg: nh_config::common_config::BaseConfig,
}

/// Go: `allDecoderSetups` (sorted by (weight, key) at use).
pub fn all_decoder_setups() -> Vec<DecodeWeight> {
    todo!()
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: config/allconfig/alldecoders.go (469 lines; 1/1 funcs executed)
//   types: decodeConfig, decodeWeight
// EX L455-469: init()
// ---------------------------------------------------------------------------
