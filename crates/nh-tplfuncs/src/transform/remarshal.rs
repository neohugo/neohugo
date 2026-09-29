//! Port of `tpl/transform/remarshal.go`.
//!
//! Owner: Wave B task T19 (tplfuncs-host).
//!
//! The encoders are nh-parser's `InterfaceToConfig` (JSON, YAML, TOML and XML).
//!
//! Deviation: Go's `applyMarshalTypes` rewrites the caller's map in place when `data` is a map;
//! the port rewrites a copy (template maps are immutable values).

use go_value::{HostCtx, IntKind, Map, MapType, Value};
use nh_common::cast::caste;
use nh_common::object::{GoResult, args};
use nh_parser::metadecoders::decoder::Decoder;
use nh_parser::metadecoders::format::{Format, format_from_string};

use super::transform::{Namespace, gerr};

impl Namespace {
    /// Remarshal is used in the Hugo documentation to convert configuration examples from YAML
    /// to JSON, TOML (and possibly the other way around). Format is one of json, yaml or toml.
    // Go: tpl/transform/remarshal.go:Remarshal
    pub fn remarshal(&self, _ctx: HostCtx<'_>, a: &[Value]) -> GoResult<Value> {
        args::exactly(a, 2, "Remarshal")?;
        let format = args::string(a, 0)?;
        let data = &a[1];

        let format =
            go_unicode::strings::to_lower(go_unicode::strings::trim_space(format.as_bytes()))
                .into_owned();

        let mark = to_format_mark(&String::from_utf8_lossy(&format))?;

        let mut meta: Map = match data {
            Value::Map(m) if m.ty == MapType::StringAny => Map::clone(m),
            Value::TypedNil(t) if &**t == "map[string]interface {}" => Map::new(MapType::StringAny),
            _ => {
                let from = caste::to_string_e(data)?;

                let from = go_unicode::strings::trim_space(from.as_bytes());
                if from.is_empty() {
                    return Ok(Value::string(""));
                }

                let from_format = Decoder::default().format_from_content_string(from);
                if from_format == Format::Unknown {
                    return Err(gerr("failed to detect format from content"));
                }

                Decoder::default().unmarshal_to_map(from, from_format)?
            }
        };

        // Make it so 1.0 float64 prints as 1 etc.
        apply_marshal_types(&mut meta);

        let mut result = Vec::new();
        nh_parser::frontmatter::interface_to_config(&Value::map(meta), mark, &mut result)?;

        Ok(Value::string(result))
    }
}

/// The unmarshal/marshal dance is extremely type lossy, and we need to make sure that integer
/// types prints as "43" and not "43.0" in all formats, hence this hack.
// Go: tpl/transform/remarshal.go:applyMarshalTypes
fn apply_marshal_types(m: &mut Map) {
    for v in m.entries.values_mut() {
        match v {
            Value::Map(mm) if mm.ty == MapType::StringAny => {
                let mut c = Map::clone(mm);
                apply_marshal_types(&mut c);
                *v = Value::map(c);
            }
            Value::Float(t, go_value::FloatKind::F64) => {
                // Go: `int64(t)` (arm64 saturates, NaN -> 0; Rust `as` does the same).
                let i = *t as i64;
                if *t == i as f64 {
                    *v = Value::Int(i, IntKind::Int64);
                }
            }
            _ => {}
        }
    }
}

// Go: tpl/transform/remarshal.go:toFormatMark
fn to_format_mark(format: &str) -> GoResult<Format> {
    let f = format_from_string(format);
    if f != Format::Unknown {
        return Ok(f);
    }

    Err(gerr("failed to detect target data serialization format"))
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: tpl/transform/remarshal.go (87 lines; 0/3 funcs executed)
// OK L19-62: (ns *Namespace) Remarshal(format string, data any) (string, error)
// OK L67-79: applyMarshalTypes(m map[string]any)
// OK L81-87: toFormatMark(format string) (metadecoders.Format, error)
// ---------------------------------------------------------------------------
