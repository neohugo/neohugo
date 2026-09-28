//! Port of `config/namespace.go`.
//!
//! Owner: Wave B task T04 (config-base-media).

use go_value::Value;
use nh_common::Result;

/// Go: `config.ConfigNamespace[S, C]`: the decoded config `C` plus the hash of the *source* map
/// (before defaults; includes `_merge` keys — this hash names every processed image).
#[derive(Clone, Debug)]
pub struct ConfigNamespace<S, C> {
    /// SourceStructure represents the source configuration with any defaults applied. This is
    /// used for documentation and printing of the configuration setup to the user.
    pub source_structure: Value,
    /// SourceHash is a hash of the source configuration before any defaults gets applied
    /// (`hashing.HashStringHex(configSource)`).
    pub source_hash: String,
    /// Config is the final configuration as used by Hugo.
    pub config: C,
    pub _signature: std::marker::PhantomData<S>,
}

/// Go: `config.DecodeNamespace[S, C](configSource, buildConfig)`. `build_config` returns the
/// config and the source structure (`None` = Go's nil: the config source is used).
// Go: config/namespace.go:DecodeNamespace
pub fn decode_namespace<S, C>(
    config_source: &Value,
    build_config: impl FnOnce(&Value) -> Result<(C, Option<Value>)>,
) -> Result<ConfigNamespace<S, C>> {
    // Calculate the hash of the input (not including any defaults applied later). This allows
    // us to introduce new config options without breaking the hash.
    let h = nh_common::hashing::try_hash_string_hex(std::slice::from_ref(config_source))?;

    // Build the config
    let (c, ext) = build_config(config_source)?;

    let ext = match ext {
        Some(e) if !e.is_invalid() => e,
        _ => config_source.clone(),
    };

    if ext.is_invalid() {
        panic!("ext is nil");
    }

    Ok(ConfigNamespace {
        source_structure: ext,
        source_hash: h,
        config: c,
        _signature: std::marker::PhantomData,
    })
}

impl<S, C> ConfigNamespace<S, C> {
    /// MarshalJSON marshals the source structure.
    // Go: config/namespace.go:MarshalJSON
    pub fn marshal_json(&self) -> Result<Vec<u8>> {
        go_json::marshal(&self.source_structure).map_err(|e| nh_common::Error::new(e.to_string()))
    }

    /// Signature returns the signature of the source structure (documentation only).
    // Go: config/namespace.go:Signature
    pub fn signature(&self) -> S
    where
        S: Default,
    {
        S::default()
    }
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: config/namespace.go (75 lines; 1/3 funcs executed)
//   types: ConfigNamespace[S
// OK L22-48: DecodeNamespace[S, C any](configSource any, buildConfig func(any) (C, any, error)) (*ConfigNamespace[S, C], error)
// OK L66-68: (ns *ConfigNamespace[S, C]) MarshalJSON() ([]byte, error)
// OK L72-75: (ns *ConfigNamespace[S, C]) Signature() S
// ---------------------------------------------------------------------------
