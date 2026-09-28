//! Port of `tpl/encoding/encoding.go`.
//!
//! Owner: Wave B task T18 (tplfuncs-data).

use std::sync::Arc;

use go_value::{GoString, HostCtx, Object, Value};
use nh_common::cast::caste;
use nh_common::hreflect::ReflectKind;
use nh_common::object::{GoResult, args};
use nh_deps::deps::Deps;

use crate::collections::reflect_helpers::{
    as_slice, bool_of, float_of, int_of, kind, string_of, uint_of,
};

// Parity notes: `jsonify` = Go encoding/json Encoder with HTML escaping (go-json), trailing newline trimmed, returns template.HTML.

/// Go: `encoding.Namespace` (template value `*encoding.Namespace`).
pub struct Namespace {
    pub d: Arc<Deps>,
}

impl Namespace {
    // Go: tpl/encoding:New
    pub fn new(d: Arc<Deps>) -> Namespace {
        Namespace { d }
    }

    // Go: tpl/encoding:Base64Decode
    pub fn base64_decode(&self, _ctx: HostCtx<'_>, a: &[Value]) -> GoResult<Value> {
        args::exactly(a, 1, "Base64Decode")?;
        // Go: tpl/encoding/encoding.go:Base64Decode
        let conv = caste::to_string_e(&a[0])?;

        let (dec, err) = base64_std_decode(&conv);
        match err {
            Some(e) => Err(go_value::Error::new(e)),
            None => Ok(Value::String(GoString::from(dec))),
        }
    }

    // Go: tpl/encoding:Base64Encode
    pub fn base64_encode(&self, _ctx: HostCtx<'_>, a: &[Value]) -> GoResult<Value> {
        args::exactly(a, 1, "Base64Encode")?;
        // Go: tpl/encoding/encoding.go:Base64Encode
        let conv = caste::to_string_e(&a[0])?;

        Ok(Value::string(base64_std_encode(&conv)))
    }

    // Go: tpl/encoding:Jsonify
    pub fn jsonify(&self, _ctx: HostCtx<'_>, a: &[Value]) -> GoResult<Value> {
        Ok(Value::html(self.do_jsonify(a)?))
    }

    // Go: tpl/encoding/encoding.go:Jsonify
    /// Jsonify encodes a given object to JSON. To pretty print the JSON, pass a map or
    /// dictionary of options as the first value in args. Supported options are "prefix" and
    /// "indent". Each JSON element in the output will begin on a new line beginning with prefix
    /// followed by one or more copies of indent according to the indentation nesting.
    pub fn do_jsonify(&self, a: &[Value]) -> GoResult<GoString> {
        let obj;
        let mut opts = JsonifyOpts::default();

        match a.len() {
            0 => return Ok(GoString::empty()),
            1 => obj = &a[0],
            2 => {
                let m = nh_common::maps::maps::to_string_map_e(&a[0])?;
                opts = weak_decode_opts(&m)?;
                obj = &a[1];
            }
            _ => return Err(go_value::Error::new("too many arguments to jsonify")),
        }

        let mut buff: Vec<u8> = Vec::new();
        {
            let mut e = go_json::Encoder::new(&mut buff);
            e.set_escape_html(!opts.no_html_escape);
            e.set_indent(opts.prefix.as_bytes(), opts.indent.as_bytes());
            e.encode(obj)
                .map_err(|err| go_value::Error::new(err.to_string()))?;
        }
        // See https://github.com/golang/go/issues/37083
        // Hugo changed from MarshalIndent/Marshal. To make the output
        // the same, we need to trim the trailing newline.
        buff.pop();

        Ok(GoString::from(buff))
    }
}

/// Go: `jsonifyOpts`.
#[derive(Default)]
struct JsonifyOpts {
    prefix: GoString,
    indent: GoString,
    no_html_escape: bool,
}

/// Go `mapstructure.WeakDecode(m, &opts)` for `jsonifyOpts` (mitchellh/mapstructure
/// v1.5.1-0.20231216201459-8508981c8b6c): fields matched by exact name, else by
/// `strings.EqualFold` over the keys (the first in key order; Go ranges the map), weakly typed
/// string and bool conversions, nil values skipped, errors collected and sorted.
fn weak_decode_opts(m: &go_value::Map) -> GoResult<JsonifyOpts> {
    let mut opts = JsonifyOpts::default();
    let mut errors: Vec<String> = Vec::new();

    let lookup = |field: &str| -> Option<&Value> {
        if let Some(v) = m.get(field.as_bytes()) {
            return Some(v);
        }
        m.entries
            .iter()
            .find(|(k, _)| go_unicode::strings::equal_fold(k, field.as_bytes()))
            .map(|(_, v)| v)
    };

    for field in ["Prefix", "Indent", "NoHTMLEscape"] {
        let Some(v) = lookup(field) else {
            continue;
        };
        if v.is_invalid() {
            continue;
        }
        if field == "NoHTMLEscape" {
            match weak_bool(field, v) {
                Ok(b) => opts.no_html_escape = b,
                Err(e) => errors.push(e),
            }
        } else {
            match weak_string(field, v) {
                Ok(s) => {
                    if field == "Prefix" {
                        opts.prefix = s;
                    } else {
                        opts.indent = s;
                    }
                }
                Err(e) => errors.push(e),
            }
        }
    }

    if !errors.is_empty() {
        let mut points: Vec<String> = errors.iter().map(|e| format!("* {e}")).collect();
        points.sort();
        return Err(go_value::Error::new(format!(
            "{} error(s) decoding:\n\n{}",
            errors.len(),
            points.join("\n")
        )));
    }
    Ok(opts)
}

fn unconvertible(name: &str, expected: &str, v: &Value) -> String {
    format!(
        "'{name}' expected type '{expected}', got unconvertible type '{}', value: '{}'",
        v.go_type_name(),
        String::from_utf8_lossy(&go_fmt::sprintf("%v", std::slice::from_ref(v)))
    )
}

// Go: github.com/mitchellh/mapstructure/mapstructure.go:decodeString
fn weak_string(name: &str, v: &Value) -> Result<GoString, String> {
    match kind(v) {
        ReflectKind::String => Ok(string_of(v)),
        ReflectKind::Bool => Ok(GoString::from(if bool_of(v) { "1" } else { "0" })),
        ReflectKind::Int(_) => Ok(GoString::from(int_of(v).to_string())),
        ReflectKind::Uint(_) => Ok(GoString::from(uint_of(v).to_string())),
        ReflectKind::Float32 | ReflectKind::Float64 => Ok(GoString::from(
            go_strconv::format_float(float_of(v), b'f', -1, 64),
        )),
        ReflectKind::Slice => {
            let s = as_slice(v).unwrap();
            if s.elem_type() == "uint8" {
                Ok(GoString::from(
                    s.items
                        .iter()
                        .map(|x| uint_of(x) as u8)
                        .collect::<Vec<u8>>(),
                ))
            } else {
                Err(unconvertible(name, "string", v))
            }
        }
        _ => Err(unconvertible(name, "string", v)),
    }
}

// Go: github.com/mitchellh/mapstructure/mapstructure.go:decodeBool
fn weak_bool(name: &str, v: &Value) -> Result<bool, String> {
    match kind(v) {
        ReflectKind::Bool => Ok(bool_of(v)),
        ReflectKind::Int(_) => Ok(int_of(v) != 0),
        ReflectKind::Uint(_) => Ok(uint_of(v) != 0),
        ReflectKind::Float32 | ReflectKind::Float64 => Ok(float_of(v) != 0.0),
        ReflectKind::String => {
            let s = string_of(v);
            match go_strconv::parse_bool(&s) {
                Ok(b) => Ok(b),
                Err(_) if s.is_empty() => Ok(false),
                Err(e) => Err(format!("cannot parse '{name}' as bool: {e}")),
            }
        }
        _ => Err(unconvertible(name, "bool", v)),
    }
}

const ENCODE_STD: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

// Go: encoding/base64/base64.go:EncodeToString (StdEncoding)
fn base64_std_encode(src: &[u8]) -> String {
    let mut out = String::with_capacity(src.len().div_ceil(3) * 4);
    for chunk in src.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = chunk.get(1).copied().unwrap_or(0) as u32;
        let b2 = chunk.get(2).copied().unwrap_or(0) as u32;
        let val = (b0 << 16) | (b1 << 8) | b2;
        out.push(ENCODE_STD[(val >> 18 & 0x3F) as usize] as char);
        out.push(ENCODE_STD[(val >> 12 & 0x3F) as usize] as char);
        if chunk.len() > 1 {
            out.push(ENCODE_STD[(val >> 6 & 0x3F) as usize] as char);
        } else {
            out.push('=');
        }
        if chunk.len() > 2 {
            out.push(ENCODE_STD[(val & 0x3F) as usize] as char);
        } else {
            out.push('=');
        }
    }
    out
}

fn decode_map(c: u8) -> u8 {
    match c {
        b'A'..=b'Z' => c - b'A',
        b'a'..=b'z' => c - b'a' + 26,
        b'0'..=b'9' => c - b'0' + 52,
        b'+' => 62,
        b'/' => 63,
        _ => 0xff,
    }
}

fn corrupt(offset: usize) -> String {
    format!("illegal base64 data at input byte {offset}")
}

// Go: encoding/base64/base64.go:decodeQuantum (StdEncoding: padding '=', not strict)
fn decode_quantum(dst: &mut Vec<u8>, src: &[u8], mut si: usize) -> (usize, Option<String>) {
    let mut dbuf = [0u8; 4];
    let mut dlen = 4;
    let mut err = None;

    let mut j = 0usize;
    while j < 4 {
        if src.len() == si {
            if j == 0 {
                return (si, None);
            }
            // j == 1, or padding is required.
            return (si, Some(corrupt(si - j)));
        }
        let inb = src[si];
        si += 1;

        let out = decode_map(inb);
        if out != 0xff {
            dbuf[j] = out;
            j += 1;
            continue;
        }

        if inb == b'\n' || inb == b'\r' {
            continue;
        }

        if inb != b'=' {
            return (si, Some(corrupt(si - 1)));
        }

        // We've reached the end and there's padding
        match j {
            0 | 1 => {
                // incorrect padding
                return (si, Some(corrupt(si - 1)));
            }
            2 => {
                // "==" is expected, the first "=" is already consumed.
                // skip over newlines
                while si < src.len() && (src[si] == b'\n' || src[si] == b'\r') {
                    si += 1;
                }
                if si == src.len() {
                    // not enough padding
                    return (si, Some(corrupt(src.len())));
                }
                if src[si] != b'=' {
                    // incorrect padding
                    return (si, Some(corrupt(si - 1)));
                }

                si += 1;
            }
            _ => {}
        }

        // skip over newlines
        while si < src.len() && (src[si] == b'\n' || src[si] == b'\r') {
            si += 1;
        }
        if si < src.len() {
            // trailing garbage
            err = Some(corrupt(si));
        }
        dlen = j;
        break;
    }

    // Convert 4x 6bit source bytes into 3 bytes
    let val =
        (dbuf[0] as u32) << 18 | (dbuf[1] as u32) << 12 | (dbuf[2] as u32) << 6 | dbuf[3] as u32;
    let b = [(val >> 16) as u8, (val >> 8) as u8, val as u8];
    if dlen >= 2 {
        dst.extend_from_slice(&b[..dlen - 1]);
    }
    (si, err)
}

// Go: encoding/base64/base64.go:DecodeString (StdEncoding)
/// The decoded bytes and Go's `CorruptInputError` text, if any.
fn base64_std_decode(src: &[u8]) -> (Vec<u8>, Option<String>) {
    let mut dst = Vec::with_capacity(src.len() / 4 * 3);
    let mut si = 0;
    while si < src.len() {
        let (nsi, err) = decode_quantum(&mut dst, src, si);
        si = nsi;
        if err.is_some() {
            return (dst, err);
        }
    }
    (dst, None)
}

nh_common::go_methods!(Namespace {
    "Base64Decode" => |n, ctx, a| n.base64_decode(ctx, a),
    "Base64Encode" => |n, ctx, a| n.base64_encode(ctx, a),
    "Jsonify" => |n, ctx, a| n.jsonify(ctx, a),
});

impl Object for Namespace {
    nh_common::object_basics!("*encoding.Namespace");
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: tpl/encoding/encoding.go (116 lines; 2/4 funcs executed)
//   types: Namespace, jsonifyOpts
// OK L31-33: New() *Namespace
// OK L39-47: (ns *Namespace) Base64Decode(content any) (string, error)
// OK L50-57: (ns *Namespace) Base64Encode(content any) (string, error)
// OK L64-110: (ns *Namespace) Jsonify(args ...any) (template.HTML, error)
// ---------------------------------------------------------------------------
