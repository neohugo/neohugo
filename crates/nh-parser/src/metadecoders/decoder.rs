//! Port of `parser/metadecoders/decoder.go`.
//!
//! Owner: Wave B task T03 (parser-langs).

//! Go `metadecoders.Decoder`: decodes YAML (gopkg.in/yaml.v2 YAML-1.1 resolution via go-yaml:
//! plain timestamps stay strings, ints are Go `int`, `map[any]any` stringified), TOML
//! (go-toml/v2 port: ints are `int64`, offset datetimes are `time.Time`, local dates are go-toml
//! Local* values), JSON (all numbers `float64`) into [`go_value::Value`].

use go_value::{IntKind, Map, MapType, SliceType, Value};

use super::format::{Format, format_from_string};
use crate::metadecoders::{csv, mxj, toml};

pub use nh_common::{Error, Result};

/// Go: `metadecoders.Decoder`.
#[derive(Clone, Debug)]
pub struct Decoder {
    /// Delimiter is the field delimiter. Used in the CSV decoder. Default is ','.
    pub delimiter: char,
    /// Comment, if not 0, is the comment character. Used in the CSV decoder.
    pub comment: Option<char>,
    /// If true, a quote may appear in an unquoted field and a non-doubled quote
    /// may appear in a quoted field. Used in the CSV decoder. Default is false.
    pub lazy_quotes: bool,
    /// The target data type, either slice or map. Used in the CSV decoder.
    /// Default is slice.
    pub target_type: String,
}

impl Default for Decoder {
    /// Go: `metadecoders.Default`.
    fn default() -> Self {
        Decoder {
            delimiter: ',',
            comment: None,
            lazy_quotes: false,
            target_type: "slice".into(),
        }
    }
}

/// `neohugo-rs: … is not supported` for the decoders seeksnack never reaches.
fn unsupported(what: &str) -> Error {
    Error::new(format!("neohugo-rs: {what} is not supported"))
}

// Go: parser/metadecoders/decoder.go:toFileError
fn to_file_error(f: Format, err: Error, pos: Option<(i64, i64)>) -> Error {
    let name = format!("_stream.{}", f.as_str());
    match pos {
        Some((line, column)) => err.at(nh_common::herrors::FilePos {
            filename: name,
            line,
            column,
        }),
        None => nh_common::herrors::new_file_error_from_name(err, &name),
    }
}

impl Decoder {
    /// OptionsKey is used in cache keys.
    // Go: parser/metadecoders/decoder.go:OptionsKey
    pub fn options_key(&self) -> String {
        let mut sb = String::new();
        sb.push(self.delimiter);
        sb.push(self.comment.unwrap_or('\0'));
        sb.push_str(if self.lazy_quotes { "true" } else { "false" });
        sb.push_str(&self.target_type);
        sb
    }

    /// Go: `UnmarshalToMap` — empty input -> empty map; result is a `map[string]any` value tree
    /// (keys NOT lower-cased here; `maps.PrepareParams` does that later). `data` is a non-nil
    /// Go slice; Go's nil map (a YAML or JSON `null`) is returned as an empty map (see
    /// [`Decoder::unmarshal_to_map_opt`] for the distinction).
    // Go: parser/metadecoders/decoder.go:UnmarshalToMap
    pub fn unmarshal_to_map(&self, data: &[u8], f: Format) -> Result<Map> {
        self.unmarshal_to_map_opt(Some(data), f)
    }

    /// Go: `UnmarshalToMap` with Go's nil `data` as `None` (nil data gives an empty map without
    /// decoding).
    // Go: parser/metadecoders/decoder.go:UnmarshalToMap
    pub fn unmarshal_to_map_opt(&self, data: Option<&[u8]>, f: Format) -> Result<Map> {
        Ok(self
            .unmarshal_to_map_nilable(data, f)?
            .unwrap_or_else(|| Map::new(MapType::StringAny)))
    }

    /// Go: `UnmarshalToMap`, with Go's nil result map (`null` documents) as `Ok(None)`.
    // Go: parser/metadecoders/decoder.go:UnmarshalToMap
    pub fn unmarshal_to_map_nilable(&self, data: Option<&[u8]>, f: Format) -> Result<Option<Map>> {
        let Some(data) = data else {
            return Ok(Some(Map::new(MapType::StringAny)));
        };

        match self.unmarshal_to(data, f, Target::Map)? {
            Value::Map(m) => Ok(Some(Map::clone(&m))),
            Value::TypedNil(_) => Ok(None),
            other => unreachable!("map target produced {other:?}"),
        }
    }

    /// UnmarshalFileToMap is the same as UnmarshalToMap, but reads the data from
    /// the given filename (`read_file` stands in for `afero.ReadFile(fs, filename)`).
    // Go: parser/metadecoders/decoder.go:UnmarshalFileToMap
    pub fn unmarshal_file_to_map(
        &self,
        filename: &str,
        read_file: &dyn Fn(&str) -> Result<Vec<u8>>,
    ) -> Result<Map> {
        let format = format_from_string(filename);
        if format == Format::Unknown {
            return Err(Error::new(format!(
                "{} is not a valid configuration format",
                go_strconv::quote(filename)
            )));
        }

        let data = read_file(filename)?;
        self.unmarshal_to_map(&data, format)
    }

    /// Go: `UnmarshalStringTo(data, typ)` — HUGO_* env overrides typed like the existing value.
    // Go: parser/metadecoders/decoder.go:UnmarshalStringTo
    pub fn unmarshal_string_to(&self, data: &str, typ: &Value) -> Result<Value> {
        let data = go_unicode::strings::trim_space(data.as_bytes());
        // We only check for the possible types in YAML, JSON and TOML.
        let is_map = match typ {
            Value::Map(m) => matches!(m.ty, MapType::StringAny | MapType::Params),
            Value::TypedNil(t) => &**t == "map[string]interface {}" || &**t == "maps.Params",
            _ => false,
        };
        let is_any_slice = match typ {
            Value::List(l) => l.ty == SliceType::Any,
            Value::TypedNil(t) => &**t == "[]interface {}",
            _ => false,
        };
        match typ {
            Value::String(_) => Ok(Value::string(data)),
            _ if is_map => {
                let format = self.format_from_content_string(data);
                Ok(Value::map(self.unmarshal_to_map(data, format)?))
            }
            // A standalone slice. Let YAML handle it.
            _ if is_any_slice => self.unmarshal(data, Format::Yaml),
            Value::Bool(_) => {
                nh_common::cast::caste::to_bool_e(&Value::string(data)).map(Value::Bool)
            }
            Value::Int(_, IntKind::Int) => {
                nh_common::cast::caste::to_int_e(&Value::string(data)).map(Value::int)
            }
            Value::Int(_, IntKind::Int64) => {
                nh_common::cast::caste::to_int64_e(&Value::string(data)).map(Value::int64)
            }
            Value::Float(_, go_value::FloatKind::F64) => {
                nh_common::cast::caste::to_float64_e(&Value::string(data)).map(Value::float64)
            }
            _ => Err(Error::new(format!(
                "unmarshal: {} not supported",
                typ.go_type_name()
            ))),
        }
    }

    /// Go: `Unmarshal` into `any`. This is what's needed for Hugo's /data handling.
    // Go: parser/metadecoders/decoder.go:Unmarshal
    pub fn unmarshal(&self, data: &[u8], f: Format) -> Result<Value> {
        if data.is_empty() {
            return match f {
                Format::Csv => match self.target_type.as_str() {
                    "map" => Ok(Value::map(Map::new(MapType::StringAny))),
                    "slice" => Ok(Value::list(
                        SliceType::Named(std::sync::Arc::from("[][]string")),
                        Vec::new(),
                    )),
                    _ => Err(Error::new(format!(
                        "invalid targetType: expected either slice or map, received {}",
                        self.target_type
                    ))),
                },
                _ => Ok(Value::map(Map::new(MapType::StringAny))),
            };
        }
        self.unmarshal_to(data, f, Target::Any)
    }

    // Go: parser/metadecoders/decoder.go:UnmarshalTo
    fn unmarshal_to(&self, data: &[u8], f: Format, target: Target) -> Result<Value> {
        let err: (Error, Option<(i64, i64)>) = match f {
            Format::Org => return Err(unsupported("ORG front matter decoding (go-org)")),
            Format::Json => {
                let res = match target {
                    Target::Map => go_json::unmarshal_map(data),
                    Target::Any => go_json::unmarshal(data),
                };
                match res {
                    Ok(v) => return Ok(v),
                    Err(e) => (Error::new(e.to_string()), None),
                }
            }
            Format::Xml => match mxj::new_map_xml_root(data) {
                Ok((root_name, root_value)) => {
                    // Type check before conversion
                    return match root_value {
                        Value::Map(_) => Ok(root_value),
                        other => Err(to_file_error(
                            f,
                            Error::new(format!(
                                "XML root element '{}' must be a map/object, got {}",
                                String::from_utf8_lossy(&root_name),
                                other.go_type_name()
                            )),
                            None,
                        )),
                    };
                }
                Err(e) => (
                    Error::new(String::from_utf8_lossy(&e.error_bytes()).into_owned()),
                    None,
                ),
            },
            Format::Toml => match toml::unmarshal_to_map(data) {
                Ok(m) => return Ok(Value::map(m)),
                Err(e) => (Error::new(e.message()), e.position()),
            },
            Format::Yaml => {
                let res = match target {
                    Target::Map => go_yaml::metadecoders::unmarshal_to_map(data).map(|m| match m {
                        Some(m) => Value::map(m),
                        None => Value::TypedNil(std::sync::Arc::from("map[string]interface {}")),
                    }),
                    Target::Any => go_yaml::metadecoders::unmarshal(data),
                };
                return match res {
                    Ok(v) => Ok(v),
                    Err(e) => Err(to_file_error(
                        f,
                        Error::new(String::from_utf8_lossy(&e.message_bytes()).into_owned()),
                        None,
                    )),
                };
            }
            Format::Csv => return self.unmarshal_csv(data, target),
            Format::Unknown => {
                return Err(Error::new(format!(
                    "unmarshal of format {} is not supported",
                    go_strconv::quote(f.as_str())
                )));
            }
        };

        let (err, pos) = err;
        Err(to_file_error(f, err.wrap("unmarshal failed"), pos))
    }

    // Go: parser/metadecoders/decoder.go:unmarshalCSV
    fn unmarshal_csv(&self, data: &[u8], target: Target) -> Result<Value> {
        let mut r = csv::Reader::new(data);
        r.comma = self.delimiter as go_unicode::Rune;
        r.comment = self.comment.map_or(0, |c| c as go_unicode::Rune);
        r.lazy_quotes = self.lazy_quotes;

        let records = r.read_all().map_err(|e| Error::new(e.to_string()))?;

        let v_type = match target {
            Target::Any => "*interface {}",
            Target::Map => "*map[string]interface {}",
        };
        match target {
            Target::Any => match self.target_type.as_str() {
                "map" => {
                    let records = records.unwrap_or_default();
                    if records.len() < 2 {
                        return Err(Error::new(format!(
                            "cannot unmarshal CSV into {v_type}: expected at least a header row and one data row"
                        )));
                    }

                    let mut seen = std::collections::HashSet::new();
                    for field_name in &records[0] {
                        if !seen.insert(field_name.clone()) {
                            return Err(Error::new(format!(
                                "cannot unmarshal CSV into {v_type}: header row contains duplicate field names"
                            )));
                        }
                    }

                    let sm: Vec<Value> = records[1..]
                        .iter()
                        .map(|record| {
                            let mut m = Map::new(MapType::StringString);
                            for (j, col) in record.iter().enumerate() {
                                m.entries.insert(
                                    records[0][j].clone().into(),
                                    Value::string(col.clone()),
                                );
                            }
                            Value::map(m)
                        })
                        .collect();
                    Ok(Value::list(
                        SliceType::Named(std::sync::Arc::from("[]map[string]string")),
                        sm,
                    ))
                }
                "slice" => Ok(match records {
                    None => Value::TypedNil(std::sync::Arc::from("[][]string")),
                    Some(records) => Value::list(
                        SliceType::Named(std::sync::Arc::from("[][]string")),
                        records
                            .into_iter()
                            .map(|rec| {
                                Value::list(
                                    SliceType::String,
                                    rec.into_iter().map(Value::string).collect(),
                                )
                            })
                            .collect(),
                    ),
                }),
                _ => Err(Error::new(format!(
                    "cannot unmarshal CSV into {v_type}: invalid targetType: expected either slice or map, received {}",
                    self.target_type
                ))),
            },
            Target::Map => Err(Error::new(format!("cannot unmarshal CSV into {v_type}"))),
        }
    }
}

/// The two Go targets of `UnmarshalTo`: `*map[string]any` and `*any`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Target {
    Map,
    Any,
}

/// Go: `stringifyMapKeys` — converts `map[interface{}]interface{}` (yaml.v2) to `map[string]any`
/// recursively, including inside slices; keys via `cast.ToStringE`, fallback `fmt.Sprintf("%v")`.
///
/// `go_value` maps always have string keys: go-yaml's `metadecoders::to_value` applies this
/// function while converting yaml.v2's `map[interface{}]interface{}` values (with Go's
/// `cast.ToStringE`), so on a `Value` it is the identity.
// Go: parser/metadecoders/decoder.go:stringifyMapKeys
pub fn stringify_map_keys(v: Value) -> Value {
    v
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: parser/metadecoders/decoder.go (375 lines; 5/11 funcs executed)
//   types: Decoder
// OK L58-65: (d Decoder) OptionsKey() string
// OK L75-84: (d Decoder) UnmarshalToMap(data []byte, f Format) (map[string]any, error)
// OK L88-99: (d Decoder) UnmarshalFileToMap(fs afero.Fs, filename string) (map[string]any, error)
// OK L102-125: (d Decoder) UnmarshalStringTo(data string, typ any) (any, error)
// OK L129-149: (d Decoder) Unmarshal(data []byte, f Format) (any, error)
// OK L152-235: (d Decoder) UnmarshalTo(data []byte, f Format, v any) error   (ORG: STUB)
// OK L237-283: (d Decoder) unmarshalCSV(data []byte, v any) error
// STUB L285-291: parseORGDate(s string) string
// STUB L293-324: (d Decoder) unmarshalORG(data []byte, v any) error
// OK L326-328: toFileError(f Format, data []byte, err error) error   (no source excerpt)
// OK L336-375: stringifyMapKeys(in any) (any, bool)   (in go-yaml's to_value)
// ---------------------------------------------------------------------------
