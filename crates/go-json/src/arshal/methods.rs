//! Port of `encoding/json/v2/arshal_methods.go` (go1.27.1) for host
//! objects ([`Object::marshal_json`] is Go's `json.Marshaler`,
//! [`Object::marshal_text`] is `encoding.TextMarshaler`), plus
//! `json.Number` (`MarshalJSONTo`/`UnmarshalJSONFrom`, from
//! `json/v2_decode.go`) and `json.RawMessage` (= `jsontext.Value`).

use std::borrow::Cow;
use std::sync::Arc;

use go_value::{GoString, Kind, Object, UintKind, Value};

use super::default::{
    FieldRef, marshal_map_entries, marshal_slice, marshal_struct, plain_struct_fields,
};
use super::errors::new_marshal_error_before;
use super::{Dec, Enc, Static};
use crate::encode_struct::JsonStruct;
use crate::goerr::{Err, MarshalerErr};
use crate::jsonflags;
use crate::jsontext::token::Token;
use crate::jsonwire;

/// Go `reflect.TypeOf(va.Addr().Interface()).String()`: the pointer type
/// of an addressable value. A host type that is already a pointer
/// (`*hugolib.pageState`) reaches its methods through the pointer arshaler,
/// so its own name is the pointer type.
fn addr_type_name(type_name: &str) -> String {
    if type_name.starts_with('*') {
        type_name.to_string()
    } else {
        format!("*{}", type_name)
    }
}

// Go: internal.NewMarshalerError (injected by json/v2_inject.go)
fn new_marshaler_error(type_name: &str, err: Err, func_name: &'static str) -> Err {
    Err::Marshaler(Box::new(MarshalerErr {
        type_name: addr_type_name(type_name),
        err,
        source_func: func_name,
    }))
}

/// The arshaler of a host object: `makeMethodArshaler` (MarshalerTo,
/// Marshaler, TextMarshaler) over `makeDefaultArshaler` by kind.
pub(crate) fn marshal_object(enc: &mut Enc, o: &Arc<dyn Object>) -> Option<Err> {
    if let Some(n) = o.as_any().downcast_ref::<Number>() {
        return marshal_number(enc, n);
    }
    if let Some(s) = o.as_any().downcast_ref::<JsonStruct>() {
        return marshal_struct(
            enc,
            &s.type_name,
            s.fields.iter().map(|f| FieldRef {
                name: f.name.as_bytes(),
                value: &f.value,
                omit_empty: f.omit_empty,
                omit_zero: f.omit_zero,
                string: f.string,
                interface_typed: f.interface_typed,
            }),
        );
    }
    let type_name = o.type_name();
    if let Some(r) = o.marshal_json() {
        return marshal_json_method(enc, &type_name, r);
    }
    if let Some(r) = o.marshal_text() {
        return marshal_text_method(enc, &type_name, r);
    }
    match o.kind() {
        Kind::Map => {
            let mut keys = o.map_keys();
            keys.sort();
            let vals: Vec<Value> = keys
                .iter()
                .map(|k| o.map_get(k).unwrap_or(Value::Invalid))
                .collect();
            marshal_map_entries(
                enc,
                keys.iter()
                    .zip(vals.iter())
                    .map(|(k, v)| (Cow::Borrowed(k.as_bytes()), Cow::Borrowed(v))),
                keys.len(),
                Static::Any,
                &type_name,
            )
        }
        Kind::Slice => match o.list() {
            Some(items) => marshal_slice(enc, &items, Static::Any, &type_name),
            None => enc.write_token(&Token::Null),
        },
        // Go: makeInvalidArshaler (func, chan, complex): an UnsupportedTypeError.
        Kind::Func => Some(new_marshal_error_before(enc, &type_name, None)),
        Kind::Ptr | Kind::Struct | Kind::Interface => match o.struct_fields() {
            Some(fields) => marshal_struct(enc, &type_name, plain_struct_fields(&fields)),
            // Deviation: Go would reflect over the struct's fields; the port
            // cannot see them, so this is reported as an unsupported type.
            None => Some(new_marshal_error_before(enc, &type_name, None)),
        },
    }
}

// Go: arshal_methods.go:makeMethodArshaler (the jsonMarshalerType case)
fn marshal_json_method(
    enc: &mut Enc,
    type_name: &str,
    r: go_value::Result<Vec<u8>>,
) -> Option<Err> {
    // (CallMethodsWithLegacySemantics only skips the method for map keys and
    // pointer-receiver methods on unaddressable values; hosts only report
    // methods Go would call.)
    let val = match r {
        Ok(val) => val,
        Err(err) => {
            let err = Err::Msg(err.message().to_string());
            if enc
                .opts
                .flags
                .get(jsonflags::REPORT_ERRORS_WITH_LEGACY_SEMANTICS)
            {
                return Some(new_marshaler_error(type_name, err, "MarshalJSON")); // unlike unmarshal, always wrapped
            }
            return Some(new_marshal_error_before(enc, type_name, Some(err)));
        }
    };
    if let Some(err) = enc.write_value(&val) {
        if enc
            .opts
            .flags
            .get(jsonflags::REPORT_ERRORS_WITH_LEGACY_SEMANTICS)
        {
            return Some(new_marshaler_error(type_name, err, "MarshalJSON")); // unlike unmarshal, always wrapped
        }
        return Some(err);
    }
    None
}

// Go: arshal_methods.go:makeMethodArshaler (the textMarshalerType case)
fn marshal_text_method(
    enc: &mut Enc,
    type_name: &str,
    r: go_value::Result<Vec<u8>>,
) -> Option<Err> {
    let mut r = Some(r);
    let res = enc.append_raw(b'"', false, &mut |b: &mut Vec<u8>| match r
        .take()
        .expect("called once")
    {
        Ok(b2) => {
            b.extend_from_slice(&b2);
            None
        }
        Err(err) => Some(Err::Msg(err.message().to_string())),
    });
    if let Some(err) = res {
        if enc
            .opts
            .flags
            .get(jsonflags::REPORT_ERRORS_WITH_LEGACY_SEMANTICS)
        {
            return Some(new_marshaler_error(type_name, err, "MarshalText")); // unlike unmarshal, always wrapped
        }
        return Some(err);
    }
    None
}

// Go: json/v2_decode.go:Number
/// A Number represents a JSON number literal (Go `json.Number`, the value
/// `UseNumber` decoders store instead of a float64).
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Number(pub GoString);

impl Number {
    // Go: v2_decode.go:Number.String
    /// String returns the literal text of the number.
    pub fn string(&self) -> &GoString {
        &self.0
    }

    // Go: v2_decode.go:Number.Float64
    /// Float64 returns the number as a float64.
    pub fn float64(&self) -> Result<f64, go_strconv::NumError> {
        go_strconv::parse_float(self.0.as_bytes(), 64)
    }

    // Go: v2_decode.go:Number.Int64
    /// Int64 returns the number as an int64.
    pub fn int64(&self) -> Result<i64, go_strconv::NumError> {
        go_strconv::parse_int(self.0.as_bytes(), 10, 64)
    }
}

impl Object for Number {
    fn type_name(&self) -> Cow<'_, str> {
        Cow::Borrowed("json.Number")
    }

    /// json.Number is a named string type; `go-value` has no string kind
    /// for objects, so it reports `Struct` and exposes its text through
    /// `String()` (Go's fmt prints a Stringer with its String method).
    fn kind(&self) -> Kind {
        Kind::Struct
    }

    /// Go: `type Number string`. fmt formats the underlying string where no
    /// method applies (`%#v` prints `"5"`, including inside collections, and
    /// bad verbs print `%!d(json.Number=5)`).
    fn underlying(&self) -> Option<Value> {
        Some(Value::String(self.0.clone()))
    }

    fn has_method(&self, name: &str) -> bool {
        matches!(name, "String" | "Float64" | "Int64")
    }

    fn call_method(
        &self,
        _ctx: go_value::HostCtx<'_>,
        name: &str,
        args: &[Value],
    ) -> Option<go_value::Result<Value>> {
        if !self.has_method(name) {
            return None;
        }
        if !args.is_empty() {
            return Some(Err(go_value::Error::new(format!(
                "wrong number of args for {}: want 0 got {}",
                name,
                args.len()
            ))));
        }
        Some(match name {
            "String" => Ok(Value::String(self.0.clone())),
            "Float64" => self
                .float64()
                .map(Value::float64)
                .map_err(|e| go_value::Error::new(e.to_string())),
            _ => self
                .int64()
                .map(Value::int64)
                .map_err(|e| go_value::Error::new(e.to_string())),
        })
    }

    fn go_string(&self) -> Option<GoString> {
        Some(self.0.clone())
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

// Go: v2_decode.go:Number.MarshalJSONTo, through arshal_methods.go's
// jsonMarshalerToType case.
fn marshal_number(enc: &mut Enc, n: &Number) -> Option<Err> {
    let type_name = "json.Number";
    let (prev_depth, prev_length) = enc.state.tokens.depth_length();
    enc.opts.flags.set(jsonflags::WITHIN_ARSHAL_CALL | 1);
    let mut err = number_marshal_json_to(enc, n);
    enc.opts.flags.set(jsonflags::WITHIN_ARSHAL_CALL);
    let (curr_depth, curr_length) = enc.state.tokens.depth_length();
    if (prev_depth != curr_depth || prev_length + 1 != curr_length) && err.is_none() {
        err = Some(Err::Msg("must read or write exactly one value".to_string()));
    }
    if let Some(err) = err {
        if enc
            .opts
            .flags
            .get(jsonflags::REPORT_ERRORS_WITH_LEGACY_SEMANTICS)
        {
            return Some(new_marshaler_error(type_name, err, "MarshalJSONTo")); // unlike unmarshal, always wrapped
        }
        return Some(err);
    }
    None
}

// Go: v2_decode.go:Number.MarshalJSONTo
fn number_marshal_json_to(enc: &mut Enc, n: &Number) -> Option<Err> {
    let (mut stringify, _) = enc.opts.get_option(jsonflags::STRINGIFY_NUMBERS);
    let depth = enc.stack_depth();
    let (k, len) = enc.stack_index(depth);
    if k == b'{' && len % 2 == 0 {
        stringify = true; // expecting a JSON object name
    }
    let n: &[u8] = if n.0.is_empty() { b"0" } else { n.0.as_bytes() };
    let mut val = Vec::new();
    let num: &[u8];
    if stringify {
        val.push(b'"');
        val.extend_from_slice(n);
        val.push(b'"');
        num = &val[1..val.len() - 1];
    } else {
        val.extend_from_slice(n);
        num = &val;
    }
    let (m, err) = jsonwire::consume_number(num);
    if m != num.len() || err.is_some() {
        return Some(Err::Msg(format!(
            "cannot parse {} as JSON number: invalid syntax",
            go_strconv::quote(&val)
        )));
    }
    enc.write_value(&val)
}

// Go: v2_decode.go:(*Number).UnmarshalJSONFrom, through
// arshal_methods.go's jsonUnmarshalerFromType case.
/// Unmarshals a JSON number into a `json.Number` held by an interface.
pub(crate) fn unmarshal_number(dec: &mut Dec) -> (Value, Option<Err>) {
    let (prev_depth, prev_length) = dec.state.tokens.depth_length();
    if prev_depth == 1 && dec.at_eof() {
        return (Value::object(Number(GoString::empty())), Some(Err::Eof)); // check EOF early to avoid fn reporting an EOF
    }
    dec.opts.flags.set(jsonflags::WITHIN_ARSHAL_CALL | 1);
    let (n, mut err) = number_unmarshal_json_from(dec);
    dec.opts.flags.set(jsonflags::WITHIN_ARSHAL_CALL);
    let (curr_depth, curr_length) = dec.state.tokens.depth_length();
    if (prev_depth != curr_depth || prev_length + 1 != curr_length) && err.is_none() {
        err = Some(Err::Msg("must read or write exactly one value".to_string()));
    }
    if let Some(err) = err {
        if dec
            .opts
            .flags
            .get(jsonflags::REPORT_ERRORS_WITH_LEGACY_SEMANTICS)
        {
            if let Some(err2) = dec.skip_until(prev_depth, prev_length + 1) {
                return (Value::object(n), Some(err2));
            }
            return (Value::object(n), Some(err)); // unlike marshal, never wrapped
        }
        return (Value::object(n), Some(err));
    }
    (Value::object(n), None)
}

// Go: v2_decode.go:(*Number).UnmarshalJSONFrom
fn number_unmarshal_json_from(dec: &mut Dec) -> (Number, Option<Err>) {
    let (mut stringify, _) = dec.opts.get_option(jsonflags::STRINGIFY_NUMBERS);
    let depth = dec.stack_depth();
    let (k, n) = dec.stack_index(depth);
    if k == b'{' && n % 2 == 0 {
        stringify = true; // expecting a JSON object name
    }
    let mut flags = jsonwire::ValueFlags::default();
    let r = match dec.read_value(&mut flags) {
        Ok(r) => r,
        Err(err) => return (Number(GoString::empty()), Some(err)),
    };
    let val = dec.buf[r].to_vec();
    let k = crate::jsontext::encode::value_kind(&val);
    match k {
        b'n' => {
            // (MergeWithLegacySemantics: *n is left unchanged)
            return (Number(GoString::empty()), None);
        }
        b'"' => {
            let verbatim = jsonwire::consume_simple_string(&val) == val.len();
            let v = jsonwire::unquote_may_copy(&val, verbatim);
            let (m, err) = jsonwire::consume_number(&v);
            if m != v.len() || err.is_some() {
                let serr = super::errors::SemanticError {
                    action: "",
                    byte_offset: 0,
                    json_pointer: Vec::new(),
                    json_kind: k,
                    json_value: val.clone(),
                    go_type: Some("json.Number".to_string()),
                    err: Some(Err::StrconvSyntax),
                };
                return (
                    Number(GoString::empty()),
                    Some(Err::Semantic(Box::new(serr))),
                );
            }
            return (Number(GoString::from(v)), None);
        }
        b'0' if !stringify => return (Number(GoString::from(val)), None),
        _ => {}
    }
    let serr = super::errors::SemanticError {
        action: "",
        byte_offset: 0,
        json_pointer: Vec::new(),
        json_kind: k,
        json_value: Vec::new(),
        go_type: Some("json.Number".to_string()),
        err: None,
    };
    (
        Number(GoString::empty()),
        Some(Err::Semantic(Box::new(serr))),
    )
}

// Go: json/v2_stream.go:RawMessage (= jsontext.Value)
/// RawMessage is a raw encoded JSON value (Go `json.RawMessage`, an alias
/// of `jsontext.Value`). It implements `Marshaler`: `None` (a nil
/// RawMessage) marshals as `null`, otherwise the bytes are validated and
/// reformatted like any `MarshalJSON` output.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RawMessage(pub Option<Vec<u8>>);

impl RawMessage {
    // Go: jsontext/value.go:Value.MarshalJSON
    /// MarshalJSON returns v as the JSON encoding of v.
    /// It returns the stored value as the raw JSON output without any validation.
    /// If v is nil, then this returns a JSON null.
    pub fn marshal_json(&self) -> Vec<u8> {
        match &self.0 {
            None => b"null".to_vec(),
            Some(v) => v.clone(),
        }
    }
}

impl Object for RawMessage {
    fn type_name(&self) -> Cow<'_, str> {
        Cow::Borrowed("jsontext.Value")
    }

    fn kind(&self) -> Kind {
        Kind::Slice
    }

    fn has_method(&self, name: &str) -> bool {
        name == "MarshalJSON"
    }

    fn call_method(
        &self,
        _ctx: go_value::HostCtx<'_>,
        _name: &str,
        _args: &[Value],
    ) -> Option<go_value::Result<Value>> {
        None
    }

    fn list(&self) -> Option<Vec<Value>> {
        self.0.as_ref().map(|b| {
            b.iter()
                .map(|&c| Value::Uint(c as u64, UintKind::Uint8))
                .collect()
        })
    }

    fn marshal_json(&self) -> Option<go_value::Result<Vec<u8>>> {
        Some(Ok(RawMessage::marshal_json(self)))
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}
