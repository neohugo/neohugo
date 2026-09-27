//! Port of `config/namespace.go`.
//!
//! Owner: Wave B task T04 (config-base-media).


use go_value::Value;
use nh_common::Result;

/// Go: `config.ConfigNamespace[S, C]`: the decoded config `C` plus the hash of the *source* map
/// (before defaults; includes `_merge` keys — this hash names every processed image).
#[derive(Clone, Debug)]
pub struct ConfigNamespace<S, C> {
    /// Source configuration with defaults applied (for `hugo config` output).
    pub source_structure: Value,
    /// `hashing.HashStringHex(configSource)`.
    pub source_hash: String,
    pub config: C,
    pub _signature: std::marker::PhantomData<S>,
}

/// Go: `config.DecodeNamespace[S, C](configSource, buildConfig)`.
// Go: config/namespace.go:DecodeNamespace
pub fn decode_namespace<S, C>(
    config_source: &Value,
    build_config: impl FnOnce(&Value) -> Result<(C, Option<Value>)>,
) -> Result<ConfigNamespace<S, C>> {
    todo!()
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: config/namespace.go (75 lines; 1/3 funcs executed)
//   types: ConfigNamespace[S
// EX L22-48: DecodeNamespace[S, C any](configSource any, buildConfig func(any) (C, any, error)) (*ConfigNamespace[S, C], error)
//    L66-68: (ns *ConfigNamespace[S, C]) MarshalJSON() ([]byte, error)
//    L72-75: (ns *ConfigNamespace[S, C]) Signature() S
// ---------------------------------------------------------------------------
