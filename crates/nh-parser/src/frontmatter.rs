//! Port of `parser/frontmatter.go`, `parser/lowercase_camel_json.go`.
//!
//! Owner: Wave B task T03 (parser-langs).
//!
//! Not on the seeksnack build path (`hugo config`, `hugo new`). JSON output is ported over
//! go-json; the YAML, TOML and XML encoders (yaml.v2 `Marshal`, go-toml's `Encoder`, mxj) are not
//! ported and return an explicit unsupported error.

use std::sync::Arc;

use go_value::{GoString, Map, MapType, Value};

use crate::metadecoders::format::Format;
use nh_common::{Error, Result};

const YAML_DELIM_LF: &[u8] = b"---\n";
const TOML_DELIM_LF: &[u8] = b"+++\n";

fn unsupported(what: &str) -> Error {
    Error::new(format!("neohugo-rs: {what} is not supported"))
}

/// Go: `parser.InterfaceToConfig(in, format, w)` (config/front matter writer; used by `hugo config`).
// Go: parser/frontmatter.go:InterfaceToConfig
pub fn interface_to_config(v: &Value, format: Format, w: &mut Vec<u8>) -> Result<()> {
    if matches!(v, Value::Invalid) {
        return Err(Error::new("input was nil"));
    }

    match format {
        Format::Yaml => Err(unsupported("YAML encoding (yaml.v2 Marshal)")),
        Format::Toml => Err(unsupported("TOML encoding (go-toml Encoder)")),
        Format::Json => {
            let b = go_json::marshal_indent(v, "", "   ").map_err(|e| Error::new(e.to_string()))?;

            w.extend_from_slice(&b);
            w.push(b'\n');
            Ok(())
        }
        Format::Xml => Err(unsupported("XML encoding (clbanning/mxj)")),
        _ => Err(Error::new("unsupported Format provided")),
    }
}

// Go: parser/frontmatter.go:InterfaceToFrontMatter
pub fn interface_to_front_matter(v: &Value, format: Format, w: &mut Vec<u8>) -> Result<()> {
    if matches!(v, Value::Invalid) {
        return Err(Error::new("input was nil"));
    }

    match format {
        Format::Yaml => {
            w.extend_from_slice(YAML_DELIM_LF);
            interface_to_config(v, format, w)?;
            w.extend_from_slice(YAML_DELIM_LF);
            Ok(())
        }
        Format::Toml => {
            w.extend_from_slice(TOML_DELIM_LF);
            interface_to_config(v, format, w)?;
            w.extend_from_slice(TOML_DELIM_LF);
            Ok(())
        }
        _ => interface_to_config(v, format, w),
    }
}

/// Go: `keyMatchRegex = "(\w+)":` — every match replaced by `f(match)`.
fn replace_key_matches(b: &[u8], mut f: impl FnMut(&[u8]) -> Vec<u8>) -> Vec<u8> {
    let is_word = |c: u8| c.is_ascii_alphanumeric() || c == b'_';
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'"' {
            let mut j = i + 1;
            while j < b.len() && is_word(b[j]) {
                j += 1;
            }
            if j > i + 1 && j + 1 < b.len() && b[j] == b'"' && b[j + 1] == b':' {
                out.extend_from_slice(&f(&b[i..j + 2]));
                i = j + 2;
                continue;
            }
        }
        out.push(b[i]);
        i += 1;
    }
    out
}

/// Go: `parser.NullBoolJSONMarshaller` over the wrapped marshaler's output: replaces
/// `"enable…":null` with `"enable…": false`.
// Go: parser/lowercase_camel_json.go:(NullBoolJSONMarshaller).MarshalJSON
pub fn null_bool_json(b: &[u8]) -> Vec<u8> {
    // nullEnableBoolRegex = `\"(enable\w+)\":null`
    let is_word = |c: u8| c.is_ascii_alphanumeric() || c == b'_';
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i..].starts_with(b"\"enable") {
            let mut j = i + 7;
            while j < b.len() && is_word(b[j]) {
                j += 1;
            }
            if j > i + 7 && b[j..].starts_with(b"\":null") {
                out.push(b'"');
                out.extend_from_slice(&b[i + 1..j]);
                out.extend_from_slice(b"\": false");
                i = j + 6;
                continue;
            }
        }
        out.push(b[i]);
        i += 1;
    }
    out
}

// Go: parser/lowercase_camel_json.go:preserveUpperCaseKey
fn preserve_upper_case_key(m: &[u8]) -> bool {
    m.starts_with(b"\"HTTP")
}

/// Go: `parser.LowerCaseCamelJSONMarshaller{Value: v}.MarshalJSON()`.
// Go: parser/lowercase_camel_json.go:(LowerCaseCamelJSONMarshaller).MarshalJSON
pub fn lower_case_camel_json(v: &Value) -> Result<Vec<u8>> {
    let marshalled = go_json::marshal(v);
    let (marshalled, err) = match marshalled {
        Ok(b) => (b, None),
        Err(e) => (Vec::new(), Some(Error::new(e.to_string()))),
    };

    let converted = replace_key_matches(&marshalled, |m| {
        // Attributes on the form XML, JSON etc.
        if m == go_unicode::bytes::to_upper(m).as_slice() {
            return go_unicode::bytes::to_lower(m);
        }

        let mut m = m.to_vec();
        // Empty keys are valid JSON, only lowercase if we do not have an
        // empty key.
        if m.len() > 2 && !preserve_upper_case_key(&m) {
            // Decode first rune after the double quotes
            let (r, width) = go_unicode::utf8::decode_rune(&m[1..]);
            let r = go_unicode::to_lower(r);
            go_unicode::utf8::encode_rune(&mut m[1..width + 1], r);
        }
        m
    });

    match err {
        Some(e) => Err(e),
        None => Ok(converted),
    }
}

/// Go: `parser.ReplacingJSONMarshaller` (used by `hugo config --format json`, the nh-allconfig oracle).
pub struct ReplacingJsonMarshaller {
    pub value: Value,
    pub keys_to_lower: bool,
    pub omit_empty: bool,
}

impl ReplacingJsonMarshaller {
    // Go: parser/lowercase_camel_json.go:(ReplacingJSONMarshaller).MarshalJSON
    pub fn marshal_json(&self) -> Result<Vec<u8>> {
        let (mut converted, mut err) = match go_json::marshal(&self.value) {
            Ok(b) => (b, None),
            Err(e) => (Vec::new(), Some(Error::new(e.to_string()))),
        };

        if self.keys_to_lower {
            converted = replace_key_matches(&converted, go_unicode::bytes::to_lower);
        }

        if self.omit_empty {
            // It's tricky to do this with a regexp, so convert it to a map, remove zero values and convert back.
            let m = match go_json::unmarshal_map(&converted) {
                Ok(Value::Map(m)) => Some(Map::clone(&m)),
                Ok(_) => None,
                Err(e) => return Err(Error::new(e.to_string())),
            };
            let v = match m {
                Some(mut m) => {
                    remove_zero_values(&mut m);
                    Value::map(m)
                }
                None => Value::TypedNil(Arc::from("map[string]interface {}")),
            };
            match go_json::marshal(&v) {
                Ok(b) => {
                    converted = b;
                    err = None;
                }
                Err(e) => {
                    converted = Vec::new();
                    err = Some(Error::new(e.to_string()));
                }
            }
        }

        match err {
            Some(e) => Err(e),
            None => Ok(converted),
        }
    }
}

fn remove_zero_values(m: &mut Map) {
    let keys: Vec<GoString> = m.entries.keys().cloned().collect();
    for k in keys {
        let v = m.entries.get(&k).cloned().unwrap_or(Value::Invalid);
        if !nh_common::hreflect::is_map(&v) && !nh_common::hreflect::is_truthful(&v) {
            m.entries.remove(&k);
        } else {
            match v {
                Value::Map(vv) if vv.ty == MapType::StringAny => {
                    let mut vv = Map::clone(&vv);
                    remove_zero_values(&mut vv);
                    m.entries.insert(k, Value::map(vv));
                }
                Value::List(l) if l.ty == go_value::SliceType::Any => {
                    let items = l
                        .items
                        .iter()
                        .map(|vvv| match vvv {
                            Value::Map(mm) if mm.ty == MapType::StringAny => {
                                let mut mm = Map::clone(mm);
                                remove_zero_values(&mut mm);
                                Value::map(mm)
                            }
                            other => other.clone(),
                        })
                        .collect();
                    m.entries
                        .insert(k, Value::list(go_value::SliceType::Any, items));
                }
                _ => {}
            }
        }
    }
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: parser/frontmatter.go (117 lines; 0/2 funcs executed)
// OK L35-78: InterfaceToConfig(in any, format metadecoders.Format, w io.Writer) error   (JSON; YAML/TOML/XML STUB)
// OK L80-117: InterfaceToFrontMatter(in any, format metadecoders.Format, w io.Writer) error
// Source: parser/lowercase_camel_json.go (132 lines; 0/4 funcs executed)
//   types: NullBoolJSONMarshaller, LowerCaseCamelJSONMarshaller, ReplacingJSONMarshaller
// OK L36-42: (c NullBoolJSONMarshaller) MarshalJSON() ([]byte, error)
// OK L51-53: preserveUpperCaseKey(match []byte) bool
// OK L55-79: (c LowerCaseCamelJSONMarshaller) MarshalJSON() ([]byte, error)
// OK L88-132: (c ReplacingJSONMarshaller) MarshalJSON() ([]byte, error)
// ---------------------------------------------------------------------------
