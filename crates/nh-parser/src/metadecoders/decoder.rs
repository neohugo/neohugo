//! Port of `parser/metadecoders/decoder.go`.
//!
//! Owner: Wave B task T03 (parser-langs).


//! Go `metadecoders.Decoder`: decodes YAML (gopkg.in/yaml.v2 YAML-1.1 resolution via go-yaml:
//! plain timestamps stay strings, ints are Go `int`, `map[any]any` stringified), TOML
//! (go-toml/v2: ints are `int64`, offset datetimes are `time.Time`, local dates are go-toml
//! Local* values), JSON (all numbers `float64`) into [`go_value::Value`].

use go_value::{Map, Value};

use super::format::Format;
use crate::metadecoders::toml;

pub use nh_common::{Error, Result};

/// Go: `metadecoders.Decoder` (CSV options are unused by seeksnack).
#[derive(Clone, Debug)]
pub struct Decoder {
    pub delimiter: char,
    pub comment: Option<char>,
    pub lazy_quotes: bool,
    pub target_type: String,
}

impl Default for Decoder {
    /// Go: `metadecoders.Default`.
    fn default() -> Self {
        Decoder { delimiter: ',', comment: None, lazy_quotes: false, target_type: "slice".into() }
    }
}

impl Decoder {
    /// Go: `UnmarshalToMap` — empty input -> empty map; result is a `map[string]any` value tree
    /// (keys NOT lower-cased here; `maps.PrepareParams` does that later).
    // Go: parser/metadecoders/decoder.go:UnmarshalToMap
    pub fn unmarshal_to_map(&self, data: &[u8], f: Format) -> Result<Map> {
        todo!()
    }

    /// Go: `Unmarshal` into `any`.
    // Go: parser/metadecoders/decoder.go:Unmarshal
    pub fn unmarshal(&self, data: &[u8], f: Format) -> Result<Value> {
        todo!()
    }

    /// Go: `UnmarshalStringTo(data, typ)` — HUGO_* env overrides typed like the existing value.
    // Go: parser/metadecoders/decoder.go:UnmarshalStringTo
    pub fn unmarshal_string_to(&self, data: &str, typ: &Value) -> Result<Value> {
        todo!()
    }
}

/// Go: `stringifyMapKeys` — converts `map[interface{}]interface{}` (yaml.v2) to `map[string]any`
/// recursively, including inside slices; keys via `cast.ToStringE`, fallback `fmt.Sprintf("%v")`.
// Go: parser/metadecoders/decoder.go:stringifyMapKeys
pub fn stringify_map_keys(v: Value) -> Value {
    todo!()
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: parser/metadecoders/decoder.go (375 lines; 5/11 funcs executed)
//   types: Decoder
//    L58-65: (d Decoder) OptionsKey() string
// EX L75-84: (d Decoder) UnmarshalToMap(data []byte, f Format) (map[string]any, error)
// EX L88-99: (d Decoder) UnmarshalFileToMap(fs afero.Fs, filename string) (map[string]any, error)
//    L102-125: (d Decoder) UnmarshalStringTo(data string, typ any) (any, error)
// EX L129-149: (d Decoder) Unmarshal(data []byte, f Format) (any, error)
// EX L152-235: (d Decoder) UnmarshalTo(data []byte, f Format, v any) error
//    L237-283: (d Decoder) unmarshalCSV(data []byte, v any) error
//    L285-291: parseORGDate(s string) string
//    L293-324: (d Decoder) unmarshalORG(data []byte, v any) error
//    L326-328: toFileError(f Format, data []byte, err error) error
// EX L336-375: stringifyMapKeys(in any) (any, bool)
// ---------------------------------------------------------------------------
