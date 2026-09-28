//! Port of `tpl/transform/unmarshal.go`.
//!
//! Owner: Wave B task T19 (tplfuncs-host).
//!
//! The decoding is nh-parser's `metadecoders.Decoder` (JSON, TOML, YAML; its CSV and XML
//! decoders are explicit `neohugo-rs` stubs).
//!
//! Deviation: Go's `decodeDecoder` deletes the `Delimiter`/`Comment` keys from the caller's
//! options map (a template `dict`); template maps are immutable values here, so the caller's
//! map keeps them (only a later use of the same dict in the template could see a difference).

use std::io::Read;

use go_value::{GoString, HostCtx, MapType, Value};
use nh_common::object::GoResult;
use nh_parser::metadecoders::decoder::Decoder;
use nh_parser::metadecoders::format::{Format, format_from_strings};

use super::transform::{Namespace, gerr};

/// The mapstructure view of `metadecoders.Decoder` (`Delimiter` and `Comment` are runes).
#[derive(Clone)]
struct DecoderFields {
    delimiter: i32,
    comment: i32,
    lazy_quotes: bool,
    target_type: String,
}

nh_config::decode_struct!(DecoderFields, "metadecoders.Decoder", |s| vec![
    nh_config::decode::FieldRef::new("Delimiter", &mut s.delimiter),
    nh_config::decode::FieldRef::new("Comment", &mut s.comment),
    nh_config::decode::FieldRef::new("LazyQuotes", &mut s.lazy_quotes),
    nh_config::decode::FieldRef::new("TargetType", &mut s.target_type),
]);

impl Default for DecoderFields {
    fn default() -> Self {
        let d = Decoder::default();
        DecoderFields {
            delimiter: d.delimiter as i32,
            comment: d.comment.map(|c| c as i32).unwrap_or(0),
            lazy_quotes: d.lazy_quotes,
            target_type: d.target_type,
        }
    }
}

fn rune_to_char(r: i32) -> char {
    char::from_u32(r as u32).unwrap_or('\u{FFFD}')
}

/// Go's `decoder != metadecoders.Default`.
fn is_default(d: &Decoder) -> bool {
    let def = Decoder::default();
    d.delimiter == def.delimiter
        && d.comment == def.comment
        && d.lazy_quotes == def.lazy_quotes
        && d.target_type == def.target_type
}

impl Namespace {
    /// Unmarshal unmarshals the data given, which can be either a string, json.RawMessage or a
    /// Resource. Supported formats are JSON, TOML, YAML, and CSV. You can optionally provide an
    /// options map as the first argument.
    // Go: tpl/transform/unmarshal.go:Unmarshal
    pub fn unmarshal(&self, _ctx: HostCtx<'_>, a: &[Value]) -> GoResult<Value> {
        if a.is_empty() || a.len() > 2 {
            return Err(gerr("unmarshal takes 1 or 2 arguments"));
        }

        let data: &Value;
        let mut decoder = Decoder::default();

        if a.len() == 1 {
            data = &a[0];
        } else {
            let m = match &a[0] {
                Value::Map(m) if m.ty == MapType::StringAny => Some(m.clone()),
                Value::TypedNil(t) if &**t == "map[string]interface {}" => None,
                _ => return Err(gerr("first argument must be a map")),
            };

            data = &a[1];
            decoder = decode_decoder(m.as_deref())
                .map_err(|e| gerr(format!("failed to decode options: {}", e.message())))?;
        }

        if let Some(r) = nh_resource::resourcetypes::resource_from_value_any(data)
            && let Some(reader) = r.read_seek_closer()
        {
            let mut key = r.key();

            if key.is_empty() {
                return Err(gerr("no Key set in Resource"));
            }

            if !is_default(&decoder) {
                key.push_str(&decoder.options_key());
            }

            let v = self.cache_unmarshal.get_or_create(key, |_| {
                let mt = r.media_type();
                let suffixes = mt.suffixes();
                let suffixes: Vec<&str> = suffixes.iter().map(|s| s.as_str()).collect();
                let f = format_from_strings(&suffixes);
                if f == Format::Unknown {
                    return Err(nh_common::herrors::Error::new(format!(
                        "MIME {} not supported",
                        go_strconv::quote(mt.typ.as_bytes())
                    )));
                }

                let mut reader = reader?;
                let mut b = Vec::new();
                reader
                    .read_to_end(&mut b)
                    .map_err(|e| nh_common::herrors::Error::new(e.to_string()))?;

                decoder.unmarshal(&b, f)
            })?;

            return Ok(v);
        }

        let Ok(data_str) = nh_common::types::convert::to_string_e(data) else {
            return Err(gerr(format!("type {} not supported", data.go_type_name())));
        };

        if go_unicode::strings::trim_space(data_str.as_bytes()).is_empty() {
            return Ok(Value::Invalid);
        }

        let key = nh_common::hashing::md5_from_string_hex_encoded(data_str.as_bytes());

        let v = self.cache_unmarshal.get_or_create(key, |_| {
            let f = decoder.format_from_content_string(data_str.as_bytes());
            if f == Format::Unknown {
                return Err(nh_common::herrors::Error::new("unknown format"));
            }

            decoder.unmarshal(data_str.as_bytes(), f)
        })?;

        Ok(v)
    }
}

// Go: tpl/transform/unmarshal.go:decodeDecoder
fn decode_decoder(m: Option<&go_value::Map>) -> nh_common::Result<Decoder> {
    let mut opts = DecoderFields::default();

    let Some(m) = m else {
        return Ok(to_decoder(opts));
    };

    // mapstructure does not support string to rune conversion, so do that manually.
    // See https://github.com/mitchellh/mapstructure/issues/151
    let mut rest = m.clone();
    for (k, v) in &m.entries {
        if go_unicode::strings::equal_fold(k.as_bytes(), b"Delimiter") {
            opts.delimiter = string_to_rune(v)?;
            rest.entries.remove(k);
        } else if go_unicode::strings::equal_fold(k.as_bytes(), b"Comment") {
            opts.comment = string_to_rune(v)?;
            rest.entries.remove(k);
        }
    }

    let res = nh_config::decode::weak_decode_into(&Value::map(rest), &mut opts);

    let d = to_decoder(opts);
    res.map(|_| d)
}

fn to_decoder(f: DecoderFields) -> Decoder {
    Decoder {
        delimiter: rune_to_char(f.delimiter),
        comment: (f.comment != 0).then(|| rune_to_char(f.comment)),
        lazy_quotes: f.lazy_quotes,
        target_type: f.target_type,
    }
}

// Go: tpl/transform/unmarshal.go:stringToRune
fn string_to_rune(v: &Value) -> nh_common::Result<i32> {
    let s: GoString = nh_common::cast::caste::to_string_e(v)?;

    if s.is_empty() {
        return Ok(0);
    }

    let mut r = 0;

    let b = s.as_bytes();
    let mut i = 0;
    while i < b.len() {
        let (rr, n) = go_unicode::utf8::decode_rune(&b[i..]);
        if i == 0 {
            r = rr;
        } else {
            return Err(nh_common::herrors::Error::new(format!(
                "invalid character: {}",
                String::from_utf8_lossy(&go_fmt::sprintf("%q", std::slice::from_ref(v)))
            )));
        }
        i += n;
    }

    Ok(r)
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: tpl/transform/unmarshal.go (201 lines; 1/3 funcs executed)
// OK L38-145: (ns *Namespace) Unmarshal(args ...any) (any, error)
// OK L147-178: decodeDecoder(m map[string]any) (metadecoders.Decoder, error)
// OK L180-201: stringToRune(v any) (rune, error)
// ---------------------------------------------------------------------------
