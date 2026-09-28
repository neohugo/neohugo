//! Port of `langs/config.go`.
//!
//! Owner: Wave B task T03 (parser-langs).
//!
//! `mapstructure.WeakDecode(m, &langs)` (github.com/mitchellh/mapstructure
//! v1.5.1-0.20231216201459-8508981c8b6c) is ported for this one target type,
//! `map[string]langs.LanguageConfig`: `decodeMap` → `decodeMapFromMap` → `decodeStruct` →
//! `decodeStructFromMap` → `decodeString`/`decodeInt`/`decodeBool`, with weak typing and Go's
//! error texts (`mapstructure.Error`: `N error(s) decoding:` + sorted `* …` lines).

use std::collections::BTreeMap;

use go_value::{GoString, Map, Value};
use nh_common::hreflect::{ReflectKind, kind_of};
use nh_common::{Error, Result};

/// Go: `langs.LanguageConfig` (decoded with mapstructure WeakDecode from `[languages.<lang>]`).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct LanguageConfig {
    /// The language name, e.g. "English".
    pub language_name: String,
    /// The language code, e.g. "en-US".
    pub language_code: String,
    /// The language title. When set, this will override site.Title for this language.
    pub title: String,
    /// The language direction, e.g. "ltr" or "rtl".
    pub language_direction: String,
    /// The language weight. When set to a non-zero value, this will be the main sort criteria
    /// for the language.
    pub weight: i64,
    /// Set to true to disable this language.
    pub disabled: bool,
}

/// Go: `langs.DecodeConfig(m map[string]any) (map[string]LanguageConfig, error)`.
// Go: langs/config.go:DecodeConfig
pub fn decode_config(m: &go_value::Map) -> Result<BTreeMap<String, LanguageConfig>> {
    let m = nh_common::maps::params::clean_config_string_map(m);

    let langs = weak_decode_language_configs(&m).map_err(Error::new)?;
    if langs.is_empty() {
        return Err(Error::new("no languages configured"));
    }
    Ok(langs)
}

/// Go: mapstructure's `getKind` (all int sizes are `Int`, all floats `Float32`, ...).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum MsKind {
    Bool,
    Int,
    Uint,
    Float,
    String,
    Map,
    Slice,
    Other,
}

// Go: mapstructure.go:getKind
fn get_kind(v: &Value) -> MsKind {
    match kind_of(v) {
        ReflectKind::Bool => MsKind::Bool,
        ReflectKind::Int(_) => MsKind::Int,
        ReflectKind::Uint(_) => MsKind::Uint,
        ReflectKind::Float32 | ReflectKind::Float64 => MsKind::Float,
        ReflectKind::String => MsKind::String,
        ReflectKind::Map => MsKind::Map,
        ReflectKind::Slice => MsKind::Slice,
        _ => MsKind::Other,
    }
}

/// The underlying basic value (named basic types reflect as their kind).
fn basic(v: &Value) -> Value {
    match v {
        Value::Object(o) => o.underlying().unwrap_or_else(|| v.clone()),
        _ => v.clone(),
    }
}

/// Go's `reflect.Kind.String()` for mapstructure's messages.
fn reflect_kind_name(v: &Value) -> String {
    match kind_of(v) {
        ReflectKind::Invalid => "invalid".to_string(),
        ReflectKind::Bool => "bool".to_string(),
        ReflectKind::Int(k) => Value::Int(0, k).go_type_name().into_owned(),
        ReflectKind::Uint(k) => Value::Uint(0, k).go_type_name().into_owned(),
        ReflectKind::Float32 => "float32".to_string(),
        ReflectKind::Float64 => "float64".to_string(),
        ReflectKind::String => "string".to_string(),
        ReflectKind::Slice => "slice".to_string(),
        ReflectKind::Map => "map".to_string(),
        ReflectKind::Struct => "struct".to_string(),
        ReflectKind::Ptr => "ptr".to_string(),
        ReflectKind::Interface => "interface".to_string(),
        ReflectKind::Func => "func".to_string(),
        ReflectKind::Chan => "chan".to_string(),
    }
}

/// `'%s' expected type '%s', got unconvertible type '%s', value: '%v'`.
fn unconvertible(name: &str, typ: &str, data: &Value) -> String {
    let v = go_fmt::sprintf("%v", std::slice::from_ref(data));
    format!(
        "'{}' expected type '{}', got unconvertible type '{}', value: '{}'",
        name,
        typ,
        data.go_type_name(),
        String::from_utf8_lossy(&v)
    )
}

/// Go: `mapstructure.Error.Error()`.
// Go: error.go:(*Error).Error
fn mapstructure_error(errors: &[String]) -> String {
    let mut points: Vec<String> = errors.iter().map(|e| format!("* {e}")).collect();
    points.sort();
    format!(
        "{} error(s) decoding:\n\n{}",
        errors.len(),
        points.join("\n")
    )
}

/// Go: `mapstructure.WeakDecode(m, &langs)` with `var langs map[string]LanguageConfig`.
// Go: mapstructure.go:decodeMap, decodeMapFromMap
fn weak_decode_language_configs(
    m: &Map,
) -> std::result::Result<BTreeMap<String, LanguageConfig>, String> {
    let mut errors: Vec<String> = Vec::new();
    let mut val_map = BTreeMap::new();

    // If the input data is empty, then we just match what the input data is.
    if m.entries.is_empty() {
        return Ok(val_map);
    }

    for (k, v) in &m.entries {
        let key = String::from_utf8_lossy(k.as_bytes()).into_owned();
        let field_name = format!("[{key}]");

        let mut current_val = LanguageConfig::default();
        if let Err(e) = decode_language_config(&field_name, v, &mut current_val) {
            errors.extend(e);
            continue;
        }

        val_map.insert(key, current_val);
    }

    if !errors.is_empty() {
        return Err(mapstructure_error(&errors));
    }

    Ok(val_map)
}

/// Go: `decode(name, input, LanguageConfig)` → `decodeStruct` → `decodeStructFromMap`.
/// Errors are the accumulated `mapstructure.Error` lines.
// Go: mapstructure.go:decodeStruct
fn decode_language_config(
    name: &str,
    data: &Value,
    val: &mut LanguageConfig,
) -> std::result::Result<(), Vec<String>> {
    // Go: decode(): a nil input (or a typed nil pointer) leaves the zero value.
    if is_nil_input(data) {
        return Ok(());
    }

    let entries: Vec<(GoString, Value)> = match data {
        Value::Map(m) => m
            .entries
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect(),
        Value::TypedNil(_) if get_kind(data) == MsKind::Map => Vec::new(),
        _ => {
            return Err(vec![format!(
                "'{}' expected a map, got '{}'",
                name,
                reflect_kind_name(data)
            )]);
        }
    };

    let mut errors: Vec<String> = Vec::new();

    // Go: the struct fields in declaration order.
    const FIELDS: [&str; 6] = [
        "LanguageName",
        "LanguageCode",
        "Title",
        "LanguageDirection",
        "Weight",
        "Disabled",
    ];

    for field in FIELDS {
        // Exact key, then a case-insensitive search over the keys (Go iterates the map in
        // random order; the first fold-equal key in byte order is used here).
        let raw = entries
            .iter()
            .find(|(k, _)| k.as_bytes() == field.as_bytes())
            .or_else(|| {
                entries
                    .iter()
                    .find(|(k, _)| go_unicode::strings::equal_fold(k.as_bytes(), field.as_bytes()))
            });
        let Some((_, raw_val)) = raw else {
            continue;
        };

        let field_name = if name.is_empty() {
            field.to_string()
        } else {
            format!("{name}.{field}")
        };

        let r = match field {
            "LanguageName" => decode_string(&field_name, raw_val, &mut val.language_name),
            "LanguageCode" => decode_string(&field_name, raw_val, &mut val.language_code),
            "Title" => decode_string(&field_name, raw_val, &mut val.title),
            "LanguageDirection" => decode_string(&field_name, raw_val, &mut val.language_direction),
            "Weight" => decode_int(&field_name, raw_val, &mut val.weight),
            "Disabled" => decode_bool(&field_name, raw_val, &mut val.disabled),
            _ => unreachable!(),
        };
        if let Err(e) = r {
            errors.push(e);
        }
    }

    if !errors.is_empty() {
        return Err(errors);
    }
    Ok(())
}

// Go: mapstructure.go:decodeString (WeaklyTypedInput)
fn decode_string(name: &str, data: &Value, val: &mut String) -> std::result::Result<(), String> {
    if is_nil_input(data) {
        return Ok(());
    }
    let d = basic(data);
    match (get_kind(data), &d) {
        (MsKind::String, Value::String(s) | Value::Safe(_, s)) => {
            *val = String::from_utf8_lossy(s.as_bytes()).into_owned();
        }
        (MsKind::Bool, Value::Bool(b)) => *val = if *b { "1" } else { "0" }.to_string(),
        (MsKind::Int, Value::Int(i, _)) => *val = i.to_string(),
        (MsKind::Uint, Value::Uint(u, _)) => *val = u.to_string(),
        (MsKind::Float, Value::Float(f, _)) => {
            *val = go_strconv::format_float(*f, b'f', -1, 64);
        }
        (MsKind::Slice, Value::List(l)) if l.ty == go_value::SliceType::Uint8 => {
            let bytes: Vec<u8> = l
                .items
                .iter()
                .map(|x| match x {
                    Value::Uint(u, _) => *u as u8,
                    _ => 0,
                })
                .collect();
            *val = String::from_utf8_lossy(&bytes).into_owned();
        }
        _ => return Err(unconvertible(name, "string", data)),
    }
    Ok(())
}

// Go: mapstructure.go:decodeInt (WeaklyTypedInput)
fn decode_int(name: &str, data: &Value, val: &mut i64) -> std::result::Result<(), String> {
    if is_nil_input(data) {
        return Ok(());
    }
    let d = basic(data);
    match (get_kind(data), &d) {
        (MsKind::Int, Value::Int(i, _)) => *val = *i,
        (MsKind::Uint, Value::Uint(u, _)) => *val = *u as i64,
        // Go: int64(f) — truncation, saturating on arm64 (Rust `as`).
        (MsKind::Float, Value::Float(f, _)) => *val = *f as i64,
        (MsKind::Bool, Value::Bool(b)) => *val = i64::from(*b),
        (MsKind::String, Value::String(s) | Value::Safe(_, s)) => {
            let mut str: &[u8] = s.as_bytes();
            if str.is_empty() {
                str = b"0";
            }

            match go_strconv::parse_int(str, 0, 64) {
                Ok(i) => *val = i,
                Err(err) => return Err(format!("cannot parse '{name}' as int: {err}")),
            }
        }
        _ => return Err(unconvertible(name, "int", data)),
    }
    Ok(())
}

// Go: mapstructure.go:decodeBool (WeaklyTypedInput)
fn decode_bool(name: &str, data: &Value, val: &mut bool) -> std::result::Result<(), String> {
    if is_nil_input(data) {
        return Ok(());
    }
    let d = basic(data);
    match (get_kind(data), &d) {
        (MsKind::Bool, Value::Bool(b)) => *val = *b,
        (MsKind::Int, Value::Int(i, _)) => *val = *i != 0,
        (MsKind::Uint, Value::Uint(u, _)) => *val = *u != 0,
        (MsKind::Float, Value::Float(f, _)) => *val = *f != 0.0,
        (MsKind::String, Value::String(s) | Value::Safe(_, s)) => {
            match go_strconv::parse_bool(s.as_bytes()) {
                Ok(b) => *val = b,
                Err(_) if s.as_bytes().is_empty() => *val = false,
                Err(err) => return Err(format!("cannot parse '{name}' as bool: {err}")),
            }
        }
        _ => return Err(unconvertible(name, "bool", data)),
    }
    Ok(())
}

/// Go: `decode()`'s early return for a nil input or a typed nil pointer.
fn is_nil_input(data: &Value) -> bool {
    matches!(kind_of(data), ReflectKind::Invalid | ReflectKind::Ptr)
        && !matches!(data, Value::Object(_))
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: langs/config.go (58 lines; 1/1 funcs executed)
//   types: LanguageConfig
// OK L47-58: DecodeConfig(m map[string]any) (map[string]LanguageConfig, error)
// ---------------------------------------------------------------------------
