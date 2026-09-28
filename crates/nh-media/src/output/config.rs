//! Port of `output/config.go`.
//!
//! Owner: Wave B task T04 (config-base-media).

use std::borrow::Cow;
use std::sync::Arc;

use go_value::{HostCtx, Kind, Map, MapType, Object, Value};
use nh_common::hreflect::{ReflectKind, kind_of};
use nh_common::maps::maps::to_string_map_e;
use nh_common::maps::params::clean_config_string_map;
use nh_common::{Error, Result};
use nh_config::decode::{Decode, Decoder, DecoderConfig};
use nh_config::namespace::{ConfigNamespace, decode_namespace};

use super::output_format::{Formats, OutputFormat, default_formats};
use crate::media::media_type::{MediaType, Types};

/// Go: `output.OutputFormatConfig`: configures a single output format.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct OutputFormatConfig {
    /// The MediaType string. This must be a configured media type.
    pub media_type: String,
    pub format: OutputFormat,
}

/// `output.OutputFormatConfig` as a Go value (the source structure of the namespace). Its
/// JSON is the embedded `Format`'s (the promoted `MarshalJSON`).
impl Object for OutputFormatConfig {
    fn type_name(&self) -> Cow<'_, str> {
        Cow::Borrowed("output.OutputFormatConfig")
    }
    fn kind(&self) -> Kind {
        Kind::Struct
    }
    fn has_method(&self, name: &str) -> bool {
        name == "MarshalJSON" || self.format.has_method(name)
    }
    fn call_method(
        &self,
        ctx: HostCtx<'_>,
        name: &str,
        args: &[Value],
    ) -> Option<go_value::Result<Value>> {
        self.format.call_method(ctx, name, args)
    }
    fn field(&self, name: &str) -> Option<Value> {
        match name {
            "MediaType" => Some(Value::string(self.media_type.as_str())),
            "Format" => Some(Value::object(self.format.clone())),
            _ => self.format.field(name),
        }
    }
    fn struct_fields(&self) -> Option<Vec<(Cow<'_, str>, Value)>> {
        Some(vec![
            (
                Cow::Borrowed("MediaType"),
                Value::string(self.media_type.as_str()),
            ),
            (Cow::Borrowed("Format"), Value::object(self.format.clone())),
        ])
    }
    fn marshal_json(&self) -> Option<go_value::Result<Vec<u8>>> {
        Some(self.format.marshal_json_bytes().map_err(Into::into))
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

/// Go: `output.defaultOutputFormat`.
fn default_output_format() -> OutputFormat {
    OutputFormat {
        base_name: "index".to_string(),
        rel: "alternate".to_string(),
        ..Default::default()
    }
}

/// Go: `output.DecodeConfig(mediaTypes, in)` (`in` = `Value::Invalid` for Go's nil).
// Go: output/config.go:DecodeConfig
pub fn decode_config(media_types: &Types, input: &Value) -> Result<ConfigNamespace<Map, Formats>> {
    let build_config = |input: &Value| -> Result<(Formats, Option<Value>)> {
        let mut f = default_formats();
        if !input.is_invalid() {
            let m = to_string_map_e(input)
                .map_err(|e| Error::new(format!("failed convert config to map: {e}")))?;
            let m = clean_config_string_map(&m);

            // (Go iterates m in random map order; with unique names the sorted result is the
            // same, and the first error in byte order is reported.)
            for (k, v) in &m.entries {
                let k = String::from_utf8_lossy(k).into_owned();
                let mut found = false;
                for i in 0..f.0.len() {
                    // Both are lower case.
                    if k == f.0[i].name {
                        // Merge it with the existing
                        decode(media_types, v, &mut f.0[i])?;
                        found = true;
                    }
                }
                if found {
                    continue;
                }

                let mut new_out_format = default_output_format();
                decode(media_types, v, &mut new_out_format)?;
                new_out_format.name = k;

                f.0.push(new_out_format);
            }
        }

        // Also format is a map for documentation purposes.
        let mut docm = Map::new(MapType::Named(Arc::from(
            "map[string]output.OutputFormatConfig",
        )));
        for ff in &f.0 {
            docm.insert(
                ff.name.as_str(),
                Value::object(OutputFormatConfig {
                    media_type: ff.media_type.typ.clone(),
                    format: ff.clone(),
                }),
            );
        }

        f.sort();
        Ok((f, Some(Value::map(docm))))
    };

    decode_namespace(input, build_config)
}

/// The decode hook of `output.decode`: in a map, a `mediaType` key (any case) holding a string
/// is replaced by the configured media type.
fn media_type_hook(
    media_types: &Types,
    c: &Value,
    _target: &dyn Decode,
) -> std::result::Result<Value, String> {
    if kind_of(c) != ReflectKind::Map {
        return Ok(c.clone());
    }
    let Value::Map(m) = c else {
        return Ok(c.clone());
    };
    let mut out = (**m).clone();
    for (key, vv) in &m.entries {
        if !go_unicode::strings::equal_fold(key.as_bytes(), b"mediaType") {
            continue;
        }
        // If mediaType is a string, look it up and replace it in the map.
        match vv {
            Value::Object(o) if o.as_any().is::<MediaType>() => {
                // OK
            }
            Value::String(s) => {
                let s = String::from_utf8_lossy(s).into_owned();
                match media_types.get_by_type(&s) {
                    Some(media_type) => {
                        // Go: `dataVal.SetMapIndex(key, reflect.ValueOf(mediaType))` panics when
                        // the map's element type is not an interface (e.g. map[string]string).
                        let elem = match &m.ty {
                            MapType::StringString => Some("string".to_string()),
                            MapType::Named(n) => n
                                .split_once(']')
                                .map(|(_, e)| e.to_string())
                                .filter(|e| e != "interface {}" && e != "any"),
                            _ => None,
                        };
                        if let Some(elem) = elem {
                            panic!(
                                "reflect.Value.SetMapIndex: value of type media.Type is not assignable to type {elem}"
                            );
                        }
                        out.insert(key.clone(), Value::object(media_type));
                    }
                    None => {
                        return Err(format!("media type {} not found", go_strconv::quote(&s)));
                    }
                }
            }
            _ => {
                return Err(format!(
                    "invalid output format configuration; wrong type for media type, expected string (e.g. text/html), got {}",
                    vv.go_type_name()
                ));
            }
        }
    }
    Ok(Value::map(out))
}

// Go: output/config.go:decode
fn decode(media_types: &Types, input: &Value, output: &mut OutputFormat) -> Result<()> {
    let hook = |c: &Value, t: &dyn Decode| media_type_hook(media_types, c, t);
    let decoder = Decoder::new(DecoderConfig {
        weakly_typed_input: true,
        decode_hook: Some(&hook),
        ..Default::default()
    });

    decoder.decode_input(input, output).map_err(|e| {
        Error::new(format!(
            "failed to decode output format configuration: {}",
            e.message()
        ))
    })
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: output/config.go (144 lines; 1/2 funcs executed)
//   types: OutputFormatConfig
// OK L41-93: DecodeConfig(mediaTypes media.Types, in any) (*config.ConfigNamespace[map[string]OutputFormatConfig, Formats], error)
// OK L95-144: decode(mediaTypes media.Types, input any, output *Format) error
// ---------------------------------------------------------------------------
