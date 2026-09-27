//! Port of the marshal/unmarshal logic of `encoding/json/v2` (go1.27.1)
//! for [`go_value::Value`]: `arshal.go` here, `arshal_default.go`,
//! `arshal_any.go`, `arshal_methods.go`, `arshal_time.go` and `errors.go`
//! in submodules.
//!
//! Go selects an arshaler per `reflect.Type` (`lookupArshaler`). The port
//! selects one per `Value` variant: [`marshal_concrete`] is the arshaler of
//! the value's dynamic type, and [`Static`] records whether the Go static
//! type at a position is `interface{}` (map values of `map[string]any`,
//! elements of `[]any`, interface-typed struct fields) or the concrete type.

pub(crate) mod any;
pub(crate) mod default;
pub(crate) mod errors;
pub(crate) mod methods;
pub(crate) mod time;

use go_value::{GoString, Map, MapType, Value};

use crate::goerr::Err;
use crate::jsonflags;
use crate::jsonopts::Opt;
use crate::jsontext::decode::{DecoderState, NoReader};
use crate::jsontext::encode::EncoderState;
use crate::jsontext::token::Token;

pub(crate) type Enc = EncoderState;
pub(crate) type Dec = DecoderState<NoReader>;

/// Whether the Go static type of a position is an interface (`any`) or the
/// value's own concrete type.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Static {
    Any,
    Concrete,
}

// Go: arshal.go:Marshal
/// Marshal serializes a Go value as a []byte according to the provided
/// marshal and encode options (the v1 caller passes DefaultOptionsV1).
/// On error with ReportErrorsWithLegacySemantics, the error is the
/// (untransformed) internal error; the v1 layer transforms it.
pub(crate) fn marshal(input: &Value, opts: &[Opt]) -> Result<Vec<u8>, Err> {
    let mut enc = EncoderState::new_buffered(opts);
    enc.opts.flags.set(jsonflags::OMIT_TOP_LEVEL_NEWLINE | 1);
    if let Some(err) = marshal_encode(&mut enc, input) {
        return Err(err);
    }
    Ok(enc.buf)
}

// Go: arshal.go:marshalEncode
pub(crate) fn marshal_encode(out: &mut Enc, input: &Value) -> Option<Err> {
    // (Go: a nil interface or nil pointer encodes as null.)
    if matches!(input, Value::Invalid) {
        return out.write_token(&Token::Null);
    }
    marshal_concrete(out, input)
    // (Go: InvalidateDisabledNamespaces is only needed without AllowDuplicateNames.)
}

/// Marshals `v` with the arshaler of the static type `st`.
/// Every nested value is marshaled through here, so this is where deep
/// recursion is moved to a bigger stack (see `stack.rs`).
pub(crate) fn marshal_as(enc: &mut Enc, v: &Value, st: Static) -> Option<Err> {
    crate::stack::guard(|| match st {
        Static::Any => default::marshal_interface(enc, v),
        Static::Concrete => marshal_concrete(enc, v),
    })
}

// Go: arshal.go:lookupArshaler(t).marshal for the dynamic type of v
/// The marshaler of the dynamic Go type of `v`: `makeTimeArshaler`,
/// `makeMethodArshaler` and `makeDefaultArshaler` in Go's precedence.
pub(crate) fn marshal_concrete(enc: &mut Enc, v: &Value) -> Option<Err> {
    crate::stack::guard(|| match v {
        // Nil pointers, maps, slices and interfaces all marshal as null
        // (FormatNilMapAsNull and FormatNilSliceAsNull are set under v1).
        Value::Invalid | Value::TypedNil(_) => enc.write_token(&Token::Null),
        Value::Bool(b) => default::marshal_bool(enc, *b),
        Value::Int(i, k) => default::marshal_int(enc, *i, k.go_name()),
        Value::Uint(u, k) => default::marshal_uint(enc, *u, k.go_name()),
        Value::Float(f, k) => default::marshal_float(enc, *f, *k),
        Value::String(s) => default::marshal_string(enc, s, "string"),
        Value::Safe(k, s) => default::marshal_string(enc, s, k.go_name()),
        Value::Time(t) => time::marshal_time(enc, t),
        Value::List(l) => default::marshal_list(enc, l),
        Value::Map(m) => default::marshal_map(enc, m),
        Value::Object(o) => methods::marshal_object(enc, o),
    })
}

/// The Go value an unmarshal stores into.
pub(crate) enum Target {
    /// `*any` (holding nil)
    Any,
    /// `*map[string]any` holding this map (Hugo passes `make(map[string]any)`).
    Map(Map),
}

// Go: arshal.go:Unmarshal
/// Unmarshal decodes a []byte input into a Go value according to the
/// provided unmarshal and decode options. Returns what Go leaves in the
/// target and the (untransformed) error.
pub(crate) fn unmarshal(input: &[u8], target: Target, opts: &[Opt]) -> (Value, Option<Err>) {
    let mut dec = DecoderState::new_buffered(input, opts);
    unmarshal_decode(&mut dec, target, true)
}

// Go: arshal.go:unmarshalDecode
pub(crate) fn unmarshal_decode(dec: &mut Dec, target: Target, last: bool) -> (Value, Option<Err>) {
    let current = match &target {
        Target::Any => Value::Invalid,
        Target::Map(m) => Value::map(m.clone()),
    };
    if dec
        .opts
        .flags
        .get(jsonflags::REPORT_ERRORS_WITH_LEGACY_SEMANTICS)
    {
        if let Some(err) = dec.check_next_value(last) {
            if err == Err::Eof && last {
                let offset = dec.input_offset() + dec.unread_buffer().len() as i64;
                return (current, Some(syntactic_unexpected_eof(offset)));
            }
            return (current, Some(err));
        }
    }

    let (v, err) = match target {
        Target::Any => default::unmarshal_interface(dec),
        Target::Map(m) => default::unmarshal_map_string_any(dec, Some(m)),
    };
    if let Some(err) = err {
        // (Go: InvalidateDisabledNamespaces is only needed without AllowDuplicateNames.)
        if err == Err::Eof && last {
            let offset = dec.input_offset() + dec.unread_buffer().len() as i64;
            return (v, Some(syntactic_unexpected_eof(offset)));
        }
        return (v, Some(err));
    }
    if last {
        return (v, dec.check_eof());
    }
    (v, None)
}

/// `&jsontext.SyntacticError{ByteOffset: offset, Err: io.ErrUnexpectedEOF}`
pub(crate) fn syntactic_unexpected_eof(offset: i64) -> Err {
    Err::Syntactic(Box::new(crate::jsontext::errors::SyntacticError {
        byte_offset: offset,
        json_pointer: Vec::new(),
        err: Some(Err::UnexpectedEof),
    }))
}

/// A `map[string]interface {}` value with these entries.
pub(crate) fn map_string_any(entries: std::collections::BTreeMap<GoString, Value>) -> Value {
    Value::map(Map::with_entries(MapType::StringAny, entries))
}
