//! Shared helpers of the nh-images oracle tests: the fixture reader and a decoder for the typed
//! JSON values of `tools/go-oracle/nh-common/goval` (the format the nh-images oracles use for
//! Go values).

#![allow(dead_code)]

use std::io::Read;
use std::sync::Arc;

use go_value::{FloatKind, GoString, IntKind, List, Map, MapType, SliceType, UintKind, Value};
use serde_json::Value as J;

/// Reads `tests/fixtures/<rel>` (gunzipped when it ends in `.gz`) as JSON.
pub fn fixture(rel: &str) -> J {
    let raw = fixture_bytes(rel);
    serde_json::from_slice(&raw).unwrap()
}

/// Reads `tests/fixtures/<rel>` (gunzipped when it ends in `.gz`).
pub fn fixture_bytes(rel: &str) -> Vec<u8> {
    let path = format!("{}/tests/fixtures/{rel}", env!("CARGO_MANIFEST_DIR"));
    let raw = std::fs::read(&path).unwrap_or_else(|e| panic!("{path}: {e}"));
    if rel.ends_with(".gz") {
        let mut out = Vec::new();
        flate2::read::GzDecoder::new(&raw[..])
            .read_to_end(&mut out)
            .unwrap();
        out
    } else {
        raw
    }
}

/// The repository root.
pub fn repo_root() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap()
}

/// Decodes `hex` digits.
pub fn hex(s: &str) -> Vec<u8> {
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap())
        .collect()
}

/// A goval string: the string, or `{"hex": ...}` for invalid UTF-8.
pub fn bytes(v: &J) -> Vec<u8> {
    match v {
        J::String(s) => s.as_bytes().to_vec(),
        J::Object(o) => hex(o["hex"].as_str().unwrap()),
        _ => panic!("not a goval string: {v}"),
    }
}

/// A float from its 16 hex digits of IEEE bits.
pub fn fbits(s: &str) -> f64 {
    f64::from_bits(u64::from_str_radix(s, 16).unwrap())
}

/// The 16 hex digits of the IEEE bits of `f`.
pub fn fbits_enc(f: f64) -> String {
    format!("{:016x}", f.to_bits())
}

fn int_kind(t: &str) -> Option<IntKind> {
    Some(match t {
        "int" => IntKind::Int,
        "int8" => IntKind::Int8,
        "int16" => IntKind::Int16,
        "int32" => IntKind::Int32,
        "int64" => IntKind::Int64,
        _ => return None,
    })
}

fn uint_kind(t: &str) -> Option<UintKind> {
    Some(match t {
        "uint" => UintKind::Uint,
        "uint8" => UintKind::Uint8,
        "uint16" => UintKind::Uint16,
        "uint32" => UintKind::Uint32,
        "uint64" => UintKind::Uint64,
        _ => return None,
    })
}

/// Decodes a goval typed value.
pub fn decode(v: &J) -> Value {
    let t = v["t"].as_str().unwrap();
    if t == "nil" {
        return Value::Invalid;
    }
    if let Some(k) = int_kind(t) {
        return Value::Int(v["v"].as_str().unwrap().parse().unwrap(), k);
    }
    if let Some(k) = uint_kind(t) {
        return Value::Uint(v["v"].as_str().unwrap().parse().unwrap(), k);
    }
    match t {
        "bool" => Value::Bool(v["v"].as_bool().unwrap()),
        "float64" => Value::Float(fbits(v["v"].as_str().unwrap()), FloatKind::F64),
        "float32" => Value::Float(fbits(v["v"].as_str().unwrap()), FloatKind::F32),
        "string" => Value::String(GoString::from(bytes(&v["s"]))),
        "maps.Params" | "map[string]interface {}" | "map[string]string" => {
            Value::Map(Arc::new(decode_map(v)))
        }
        "[]interface {}" => Value::List(Arc::new(List::new(
            SliceType::Any,
            v["items"].as_array().unwrap().iter().map(decode).collect(),
        ))),
        "[]string" => Value::List(Arc::new(List::new(
            SliceType::String,
            v["items"].as_array().unwrap().iter().map(decode).collect(),
        ))),
        _ => panic!("unsupported goval type {t}"),
    }
}

use go_image::color::{self, Color as GoColor};

/// Go's `%T` and `%v` of a `color.Color`, and its `RGBA()` (the oracle's `colorDesc`).
pub fn color_desc(c: Option<GoColor>) -> J {
    let Some(c) = c else {
        return J::Null;
    };
    let (t, v) = match c {
        GoColor::RGBA(x) => ("color.RGBA", format!("{{{} {} {} {}}}", x.r, x.g, x.b, x.a)),
        GoColor::RGBA64(x) => (
            "color.RGBA64",
            format!("{{{} {} {} {}}}", x.r, x.g, x.b, x.a),
        ),
        GoColor::NRGBA(x) => (
            "color.NRGBA",
            format!("{{{} {} {} {}}}", x.r, x.g, x.b, x.a),
        ),
        GoColor::NRGBA64(x) => (
            "color.NRGBA64",
            format!("{{{} {} {} {}}}", x.r, x.g, x.b, x.a),
        ),
        GoColor::Alpha(x) => ("color.Alpha", format!("{{{}}}", x.a)),
        GoColor::Alpha16(x) => ("color.Alpha16", format!("{{{}}}", x.a)),
        GoColor::Gray(x) => ("color.Gray", format!("{{{}}}", x.y)),
        GoColor::Gray16(x) => ("color.Gray16", format!("{{{}}}", x.y)),
        GoColor::YCbCr(x) => ("color.YCbCr", format!("{{{} {} {}}}", x.y, x.cb, x.cr)),
        GoColor::CMYK(x) => ("color.CMYK", format!("{{{} {} {} {}}}", x.c, x.m, x.y, x.k)),
        GoColor::NYCbCrA(_) => ("color.NYCbCrA", String::new()),
    };
    let (r, g, b, a) = c.rgba();
    serde_json::json!({"t": t, "v": v, "rgba": [r, g, b, a]})
}

/// The colour a `colorDesc` describes.
pub fn color_from_desc(d: &J) -> GoColor {
    let v = d["v"].as_str().unwrap();
    let n: Vec<u64> = v
        .trim_matches(|c| c == '{' || c == '}')
        .split(' ')
        .map(|x| x.parse().unwrap())
        .collect();
    match d["t"].as_str().unwrap() {
        "color.RGBA" => GoColor::RGBA(color::RGBA {
            r: n[0] as u8,
            g: n[1] as u8,
            b: n[2] as u8,
            a: n[3] as u8,
        }),
        "color.NRGBA" => GoColor::NRGBA(color::NRGBA {
            r: n[0] as u8,
            g: n[1] as u8,
            b: n[2] as u8,
            a: n[3] as u8,
        }),
        "color.RGBA64" => GoColor::RGBA64(color::RGBA64 {
            r: n[0] as u16,
            g: n[1] as u16,
            b: n[2] as u16,
            a: n[3] as u16,
        }),
        "color.NRGBA64" => GoColor::NRGBA64(color::NRGBA64 {
            r: n[0] as u16,
            g: n[1] as u16,
            b: n[2] as u16,
            a: n[3] as u16,
        }),
        "color.Alpha" => GoColor::Alpha(color::Alpha { a: n[0] as u8 }),
        "color.Alpha16" => GoColor::Alpha16(color::Alpha16 { a: n[0] as u16 }),
        "color.Gray" => GoColor::Gray(color::Gray { y: n[0] as u8 }),
        "color.Gray16" => GoColor::Gray16(color::Gray16 { y: n[0] as u16 }),
        "color.YCbCr" => GoColor::YCbCr(color::YCbCr {
            y: n[0] as u8,
            cb: n[1] as u8,
            cr: n[2] as u8,
        }),
        "color.CMYK" => GoColor::CMYK(color::CMYK {
            c: n[0] as u8,
            m: n[1] as u8,
            y: n[2] as u8,
            k: n[3] as u8,
        }),
        t => panic!("unsupported color type {t}"),
    }
}

/// The Go result of a case: `Ok(ok value)`, or `Err(message)` for `err` and `panic`.
pub fn go_result(res: &J) -> Result<&J, String> {
    if let Some(e) = res.get("err") {
        return Err(e.as_str().unwrap().to_string());
    }
    if let Some(e) = res.get("panic") {
        return Err(e.as_str().unwrap().to_string());
    }
    Ok(&res["ok"])
}

/// Decodes a goval map.
pub fn decode_map(v: &J) -> Map {
    let ty = match v["t"].as_str().unwrap() {
        "maps.Params" => MapType::Params,
        "map[string]string" => MapType::StringString,
        _ => MapType::StringAny,
    };
    let mut m = Map::new(ty);
    for e in v["entries"].as_array().unwrap() {
        m.insert(GoString::from(bytes(&e[0])), decode(&e[1]));
    }
    m
}
