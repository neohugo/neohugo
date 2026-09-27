//! Port of `langs/config.go`.
//!
//! Owner: Wave B task T03 (parser-langs).


use nh_common::Result;

/// Go: `langs.LanguageConfig` (decoded with mapstructure WeakDecode from `[languages.<lang>]`).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct LanguageConfig {
    pub language_name: String,
    pub language_code: String,
    pub title: String,
    pub language_direction: String,
    pub weight: i64,
    pub disabled: bool,
}

/// Go: `langs.DecodeConfig(m map[string]any) (map[string]LanguageConfig, error)`.
// Go: langs/config.go:DecodeConfig
pub fn decode_config(m: &go_value::Map) -> Result<std::collections::BTreeMap<String, LanguageConfig>> {
    todo!()
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: langs/config.go (58 lines; 1/1 funcs executed)
//   types: LanguageConfig
// EX L47-58: DecodeConfig(m map[string]any) (map[string]LanguageConfig, error)
// ---------------------------------------------------------------------------
