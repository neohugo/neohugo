//! Port of `encoding/json/v2/arshal_default.go` (go1.27.1) for the Go
//! kinds a [`Value`] can hold: bool, string (and named string types), the
//! integer and float kinds, `[]byte`, slices, maps with string keys,
//! structs described by the host, pointers (as typed nils) and `any`.

use std::borrow::Cow;
use std::collections::BTreeMap;

use base64::Engine as _;
use go_value::{FloatKind, GoString, Kind, List, Map, MapType, SliceType, Value};

use super::errors::{
    is_fatal_error, new_marshal_error_before, new_unmarshal_error_after,
    new_unmarshal_error_after_with_skipping, new_unmarshal_error_after_with_value,
};
use super::{Dec, Enc, Static, map_string_any, marshal_as};
use crate::goerr::Err;
use crate::jsonflags;
use crate::jsontext::token::Token;
use crate::jsonwire::{self, ValueFlags};

// Go: arshal_default.go:makeBoolArshaler (marshal)
pub(crate) fn marshal_bool(enc: &mut Enc, v: bool) -> Option<Err> {
    let t = "bool";
    let mut stringify = false; // always false except `string` tag with StringifyWithLegacySemantics
    if enc.opts.flags.has(jsonflags::TAG_FLAGS) {
        stringify = enc.opts.flags.get(jsonflags::STRING_TAG)
            && enc
                .opts
                .flags
                .get(jsonflags::STRINGIFY_WITH_LEGACY_SEMANTICS);
        if enc.opts.flags.get(jsonflags::STRING_TAG)
            && !enc.opts.flags.get(
                jsonflags::STRINGIFY_WITH_LEGACY_SEMANTICS
                    | jsonflags::REPORT_ERRORS_WITH_LEGACY_SEMANTICS,
            )
        {
            return Some(new_marshal_error_before(
                enc,
                t,
                Some(Err::InvalidStringTag),
            ));
        }
    }

    // Optimize for marshaling without preceding whitespace.
    if !enc.opts.flags.get(jsonflags::ANY_WHITESPACE)
        && !stringify
        && !enc.state.tokens.last.need_object_name()
    {
        enc.state.tokens.may_append_delim(&mut enc.buf, b't');
        go_strconv::append_bool(&mut enc.buf, v);
        enc.state.tokens.last.increment();
        return None;
    }

    if stringify {
        if v {
            return enc.write_token(&Token::string(b"true"));
        } else {
            return enc.write_token(&Token::string(b"false"));
        }
    }
    enc.write_token(&Token::bool(v))
}

// Go: arshal_default.go:makeStringArshaler (marshal)
/// `t` is the Go type name (`string` or a named string type).
pub(crate) fn marshal_string(enc: &mut Enc, s: &[u8], t: &str) -> Option<Err> {
    let mut stringify = false; // always false except `string` tag with StringifyWithLegacySemantics
    if enc.opts.flags.has(jsonflags::TAG_FLAGS) {
        stringify = enc.opts.flags.get(jsonflags::STRING_TAG)
            && enc
                .opts
                .flags
                .get(jsonflags::STRINGIFY_WITH_LEGACY_SEMANTICS);
        if enc.opts.flags.get(jsonflags::STRING_TAG)
            && !enc.opts.flags.get(
                jsonflags::STRINGIFY_WITH_LEGACY_SEMANTICS
                    | jsonflags::REPORT_ERRORS_WITH_LEGACY_SEMANTICS,
            )
        {
            return Some(new_marshal_error_before(
                enc,
                t,
                Some(Err::InvalidStringTag),
            ));
        }
    }

    // Optimize for marshaling without preceding whitespace or string escaping.
    if !enc.opts.flags.get(jsonflags::ANY_WHITESPACE)
        && !stringify
        && !enc.state.tokens.last.need_object_name()
    {
        let mut b = std::mem::take(&mut enc.buf);
        let orig_len = b.len();
        enc.state.tokens.may_append_delim(&mut b, b'"');
        let err = jsonwire::append_quote(&mut b, s, &enc.opts.flags);
        if err.is_none() {
            enc.buf = b;
            enc.state.tokens.last.increment();
            return None;
        }
        b.truncate(orig_len);
        enc.buf = b;
    }

    if stringify {
        let mut b = Vec::new();
        if let Some(err) = jsonwire::append_quote(&mut b, s, &enc.opts.flags) {
            let serr = crate::jsontext::errors::SyntacticError {
                byte_offset: 0,
                json_pointer: Vec::new(),
                err: Some(err),
            };
            return Some(new_marshal_error_before(
                enc,
                t,
                Some(Err::Syntactic(Box::new(serr))),
            ));
        }
        // Go: jsontext.AppendQuote(nil, b) (with zero flags)
        let mut q = Vec::new();
        if let Some(err) = jsonwire::append_quote(&mut q, &b, &jsonflags::Flags::default()) {
            panic!("BUG: second AppendQuote should never fail: {}", err);
        }
        return enc.write_value(&q);
    }
    enc.write_token(&Token::string(s))
}

// Go: arshal_default.go:makeIntArshaler (marshal)
pub(crate) fn marshal_int(enc: &mut Enc, v: i64, t: &str) -> Option<Err> {
    let stringify = enc.state.tokens.last.need_object_name()
        || enc
            .opts
            .flags
            .get(jsonflags::STRINGIFY_NUMBERS | jsonflags::STRING_TAG);
    let _ = t; // (the `format` tag error needs t; format tags are not supported)
    if !enc.opts.flags.get(jsonflags::ANY_WHITESPACE) && !stringify {
        enc.state.tokens.may_append_delim(&mut enc.buf, b'0');
        go_strconv::append_int(&mut enc.buf, v, 10);
        enc.state.tokens.last.increment();
        return None;
    }
    let k = string_or_number_kind(stringify);
    enc.append_raw(k, true, &mut |b: &mut Vec<u8>| {
        go_strconv::append_int(b, v, 10);
        None
    })
}

// Go: arshal_default.go:makeUintArshaler (marshal)
pub(crate) fn marshal_uint(enc: &mut Enc, v: u64, t: &str) -> Option<Err> {
    let stringify = enc.state.tokens.last.need_object_name()
        || enc
            .opts
            .flags
            .get(jsonflags::STRINGIFY_NUMBERS | jsonflags::STRING_TAG);
    let _ = t;
    if !enc.opts.flags.get(jsonflags::ANY_WHITESPACE) && !stringify {
        enc.state.tokens.may_append_delim(&mut enc.buf, b'0');
        go_strconv::append_uint(&mut enc.buf, v, 10);
        enc.state.tokens.last.increment();
        return None;
    }
    let k = string_or_number_kind(stringify);
    enc.append_raw(k, true, &mut |b: &mut Vec<u8>| {
        go_strconv::append_uint(b, v, 10);
        None
    })
}

// Go: arshal_default.go:makeFloatArshaler (marshal)
pub(crate) fn marshal_float(enc: &mut Enc, fv: f64, kind: FloatKind) -> Option<Err> {
    let (t, bits) = match kind {
        FloatKind::F32 => ("float32", 32),
        FloatKind::F64 => ("float64", 64),
    };
    let stringify = enc.state.tokens.last.need_object_name()
        || enc
            .opts
            .flags
            .get(jsonflags::STRINGIFY_NUMBERS | jsonflags::STRING_TAG);
    // (the `format:nonfinite` tag option is not supported)
    if fv.is_nan() || fv.is_infinite() {
        // Go: fmt.Errorf("unsupported value: %v", fv)
        let s = if fv.is_nan() {
            "NaN"
        } else if fv > 0.0 {
            "+Inf"
        } else {
            "-Inf"
        };
        let err = Err::Msg(format!("unsupported value: {}", s));
        return Some(new_marshal_error_before(enc, t, Some(err)));
    }

    // Optimize for marshaling without preceding whitespace.
    if !enc.opts.flags.get(jsonflags::ANY_WHITESPACE) && !stringify {
        enc.state.tokens.may_append_delim(&mut enc.buf, b'0');
        jsonwire::append_float(&mut enc.buf, fv, bits);
        enc.state.tokens.last.increment();
        return None;
    }

    let k = string_or_number_kind(stringify);
    enc.append_raw(k, true, &mut |b: &mut Vec<u8>| {
        jsonwire::append_float(b, fv, bits);
        None
    })
}

/// The marshaler of a slice value: `makeBytesArshaler` for `[]byte`,
/// `makeSliceArshaler` otherwise.
pub(crate) fn marshal_list(enc: &mut Enc, l: &List) -> Option<Err> {
    let t = l.ty.go_name();
    if l.ty == SliceType::Uint8 {
        return marshal_bytes(enc, &l.items, &t);
    }
    let elem = match l.ty {
        SliceType::String
        | SliceType::Int
        | SliceType::Int64
        | SliceType::Float64
        | SliceType::Bool
        | SliceType::MapStringAny => Static::Concrete,
        // []interface {} and named slice types (whose element types are
        // host interfaces such as page.Page).
        _ => Static::Any,
    };
    marshal_slice(enc, &l.items, elem, &t)
}

// Go: arshal_default.go:makeBytesArshaler (marshal)
fn marshal_bytes(enc: &mut Enc, items: &[Value], t: &str) -> Option<Err> {
    // (FormatBytesWithLegacySemantics is set under v1, so named byte types
    // are also base64-encoded; `format` tags are not supported.)
    if enc.opts.flags.has(
        jsonflags::TAG_FLAGS
            | jsonflags::FORMAT_BYTE_ARRAY_AS_ARRAY
            | jsonflags::FORMAT_BYTES_WITH_LEGACY_SEMANTICS
            | jsonflags::FORMAT_NIL_SLICE_AS_NULL,
    ) {
        if enc.opts.flags.get(jsonflags::STRING_TAG)
            && !enc
                .opts
                .flags
                .get(jsonflags::REPORT_ERRORS_WITH_LEGACY_SEMANTICS)
        {
            return Some(new_marshal_error_before(
                enc,
                t,
                Some(Err::InvalidStringTag),
            ));
        }
        // (A nil []byte is a Value::TypedNil, handled by the caller.)
    }
    let mut bytes = Vec::with_capacity(items.len());
    for it in items {
        match it {
            Value::Uint(u, _) => bytes.push(*u as u8),
            Value::Int(i, _) => bytes.push(*i as u8),
            other => {
                return Some(Err::Msg(format!(
                    "json: []uint8 element is not a byte: {}",
                    other.go_type_name()
                )));
            }
        }
    }
    enc.append_raw(b'"', true, &mut |b: &mut Vec<u8>| {
        b.extend_from_slice(
            base64::engine::general_purpose::STANDARD
                .encode(&bytes)
                .as_bytes(),
        );
        None
    })
}

// Go: arshal_default.go:makeSliceArshaler (marshal)
/// Marshals a non-nil slice whose elements have the static type `elem`.
pub(crate) fn marshal_slice(enc: &mut Enc, items: &[Value], elem: Static, t: &str) -> Option<Err> {
    // (Cycle detection is not needed: Value trees cannot be cyclic.)
    if enc.opts.flags.has(jsonflags::TAG_FLAGS)
        && enc.opts.flags.get(jsonflags::STRING_TAG)
        && !enc
            .opts
            .flags
            .get(jsonflags::REPORT_ERRORS_WITH_LEGACY_SEMANTICS)
    {
        return Some(new_marshal_error_before(
            enc,
            t,
            Some(Err::InvalidStringTag),
        ));
    }
    let n = items.len();
    if n == 0 {
        // (A nil slice is a Value::TypedNil; emitNull does not apply here.)
        if !enc.opts.flags.get(jsonflags::ANY_WHITESPACE)
            && !enc.state.tokens.last.need_object_name()
        {
            enc.state.tokens.may_append_delim(&mut enc.buf, b'[');
            enc.buf.extend_from_slice(b"[]");
            enc.state.tokens.last.increment();
            return None;
        }
    }

    if let Some(err) = enc.write_token(&Token::BeginArray) {
        return Some(err);
    }
    for v in items {
        if let Some(err) = marshal_as(enc, v, elem) {
            return Some(err);
        }
    }
    enc.write_token(&Token::EndArray)
}

/// The marshaler of a map value (all maps have string keys).
pub(crate) fn marshal_map(enc: &mut Enc, m: &Map) -> Option<Err> {
    let t = m.ty.go_name();
    let elem = match m.ty {
        MapType::StringString => Static::Concrete,
        _ => Static::Any,
    };
    marshal_map_entries(
        enc,
        m.entries
            .iter()
            .map(|(k, v)| (Cow::Borrowed(k.as_bytes()), Cow::Borrowed(v))),
        m.entries.len(),
        elem,
        &t,
    )
}

// Go: arshal_default.go:makeMapArshaler (marshal)
/// Marshals a non-nil map with string keys. `entries` must be sorted by key
/// bytes (Go sorts the names with slices.Sort under Deterministic).
pub(crate) fn marshal_map_entries<'a>(
    enc: &mut Enc,
    entries: impl Iterator<Item = (Cow<'a, [u8]>, Cow<'a, Value>)>,
    n: usize,
    elem: Static,
    t: &str,
) -> Option<Err> {
    // (Cycle detection is not needed: Value trees cannot be cyclic.)
    if enc.opts.flags.has(jsonflags::TAG_FLAGS)
        && enc.opts.flags.get(jsonflags::STRING_TAG)
        && !enc
            .opts
            .flags
            .get(jsonflags::REPORT_ERRORS_WITH_LEGACY_SEMANTICS)
    {
        return Some(new_marshal_error_before(
            enc,
            t,
            Some(Err::InvalidStringTag),
        ));
    }

    // Handle empty maps.
    if n == 0 {
        // (A nil map is a Value::TypedNil; emitNull does not apply here.)
        if !enc.opts.flags.get(jsonflags::ANY_WHITESPACE)
            && !enc.state.tokens.last.need_object_name()
        {
            enc.state.tokens.may_append_delim(&mut enc.buf, b'{');
            enc.buf.extend_from_slice(b"{}");
            enc.state.tokens.last.increment();
            return None;
        }
    }

    if let Some(err) = enc.write_token(&Token::BeginObject) {
        return Some(err);
    }
    if n > 0 {
        // (string keys with AllowInvalidUTF8 do not have a unique
        // representation, so the namespace is not disabled)
        //
        // Go: `!Deterministic || n <= 1` marshals the single key with the
        // string arshaler (NeedObjectName makes it WriteToken(String(k)));
        // otherwise the sorted names are written with WriteToken. Both
        // produce the same bytes.
        for (name, v) in entries {
            if let Some(err) = enc.write_token(&Token::string(&name)) {
                return Some(err);
            }
            if let Some(err) = marshal_as(enc, &v, elem) {
                return Some(err);
            }
        }
    }
    enc.write_token(&Token::EndObject)
}

/// One struct field for [`marshal_struct`]: Go's `structField` after
/// `makeStructFields`.
pub(crate) struct FieldRef<'a> {
    pub(crate) name: &'a [u8],
    pub(crate) value: &'a Value,
    pub(crate) omit_empty: bool,
    pub(crate) omit_zero: bool,
    pub(crate) string: bool,
    /// The field's static type is an interface.
    pub(crate) interface_typed: bool,
}

// Go: arshal_default.go:makeStructArshaler (marshal)
pub(crate) fn marshal_struct<'a>(
    enc: &mut Enc,
    t: &str,
    fields: impl Iterator<Item = FieldRef<'a>>,
) -> Option<Err> {
    if enc.opts.flags.has(jsonflags::TAG_FLAGS)
        && enc.opts.flags.get(jsonflags::STRING_TAG)
        && !enc
            .opts
            .flags
            .get(jsonflags::REPORT_ERRORS_WITH_LEGACY_SEMANTICS)
    {
        return Some(new_marshal_error_before(
            enc,
            t,
            Some(Err::InvalidStringTag),
        ));
    }
    if let Some(err) = enc.write_token(&Token::BeginObject) {
        return Some(err);
    }
    enc.state.tokens.last.disable_namespace(); // we manually ensure unique names below
    for f in fields {
        let v = f.value;
        let st = if f.interface_typed {
            Static::Any
        } else {
            Static::Concrete
        };

        // OmitZero skips the field if the Go value is zero,
        // which we can determine up front without calling the marshaler.
        if (f.omit_zero || enc.opts.flags.get(jsonflags::OMIT_ZERO_STRUCT_FIELDS)) && is_zero(v, st)
        {
            continue;
        }

        // Check for the legacy definition of omitempty.
        if f.omit_empty
            && enc
                .opts
                .flags
                .get(jsonflags::OMIT_EMPTY_WITH_LEGACY_SEMANTICS)
            && is_legacy_empty(v, st)
        {
            continue;
        }

        // (The non-legacy omitempty fast path is unreachable under v1.)

        // Write the object member name.
        //
        // The logic below is semantically equivalent to:
        //	enc.WriteToken(String(f.name))
        // but specialized and simplified because:
        //	1. The Encoder must be expecting an object name.
        //	2. The object namespace is guaranteed to be disabled.
        //	3. The object name is guaranteed to be valid and pre-escaped.
        //	4. There is no need to flush the buffer (for unwrite purposes).
        //	5. There is no possibility of an error occurring.
        let mut b = std::mem::take(&mut enc.buf);
        if enc.state.tokens.last.length() > 0 {
            b.push(b',');
            if enc.opts.flags.get(jsonflags::SPACE_AFTER_COMMA) {
                b.push(b' ');
            }
        }
        if enc.opts.flags.get(jsonflags::MULTILINE) {
            crate::jsontext::encode::append_indent(
                &enc.opts,
                &mut b,
                enc.state.tokens.need_indent(b'"'),
            );
        }
        if !jsonwire::need_escape(f.name) {
            // Go: f.quotedName, quoted with zero flags.
            let _ = jsonwire::append_quote(&mut b, f.name, &jsonflags::Flags::default());
        } else {
            let _ = jsonwire::append_quote(&mut b, f.name, &enc.opts.flags);
        }
        enc.buf = b;
        enc.state.tokens.last.increment();

        // Write the object member value.
        let flags_original = enc.opts.flags;
        if f.string {
            enc.opts.flags.set(jsonflags::STRING_TAG | 1);
        }
        let err = marshal_as(enc, v, st);
        enc.opts.flags = flags_original;
        enc.opts.format.clear();
        if let Some(err) = err {
            return Some(err);
        }
    }
    enc.write_token(&Token::EndObject)
}

/// The basic value of a named basic type (`Object::underlying`), if any.
fn basic_underlying(o: &dyn go_value::Object) -> Option<Value> {
    o.underlying().filter(|u| {
        matches!(
            u,
            Value::Bool(_) | Value::Int(..) | Value::Uint(..) | Value::Float(..) | Value::String(_)
        )
    })
}

// Go: arshal_default.go:isLegacyEmpty
/// isLegacyEmpty reports whether a value is empty according to the v1 definition.
pub(crate) fn is_legacy_empty(v: &Value, st: Static) -> bool {
    // Equivalent to encoding/json.isEmptyValue@v1.21.0.
    if st == Static::Any {
        return matches!(v, Value::Invalid); // reflect.Interface: v.IsNil()
    }
    match v {
        Value::Invalid | Value::TypedNil(_) => true,
        Value::Bool(b) => !*b,
        Value::Int(i, _) => *i == 0,
        Value::Uint(u, _) => *u == 0,
        Value::Float(f, _) => *f == 0.0,
        Value::String(s) | Value::Safe(_, s) => s.is_empty(),
        Value::List(l) => l.items.is_empty(),
        Value::Map(m) => m.entries.is_empty(),
        Value::Time(_) => false,
        Value::Object(o) => {
            if let Some(n) = o.as_any().downcast_ref::<super::methods::Number>() {
                return n.0.is_empty(); // json.Number is a string kind
            }
            // A named basic type is empty by its Kind.
            if let Some(u) = basic_underlying(o.as_ref()) {
                return is_legacy_empty(&u, Static::Concrete);
            }
            match o.kind() {
                Kind::Map => o.map_keys().is_empty(),
                Kind::Slice => o.list().is_none_or(|l| l.is_empty()),
                // A non-nil pointer or interface is not empty; structs never are.
                _ => false,
            }
        }
    }
}

// Go: fields.go (structField.isZero) and reflect.Value.IsZero
/// Whether a field value is zero for `omitzero`: the type's `IsZero`
/// method if it has one (`time.Time`, [`Object::is_zero`]), else
/// `reflect.Value.IsZero`.
pub(crate) fn is_zero(v: &Value, st: Static) -> bool {
    if st == Static::Any {
        return matches!(v, Value::Invalid); // an `any` field: v.IsZero() of the interface
    }
    match v {
        Value::Invalid | Value::TypedNil(_) => true,
        Value::Bool(b) => !*b,
        Value::Int(i, _) => *i == 0,
        Value::Uint(u, _) => *u == 0,
        Value::Float(f, _) => *f == 0.0,
        Value::String(s) | Value::Safe(_, s) => s.is_empty(),
        // Hugo's maps.Params has an IsZero method, which omitzero calls
        // (fields.go: t.Implements(isZeroerType)).
        Value::Map(m) if m.ty == MapType::Params => params_is_zero(m),
        // A non-nil slice or map is never zero.
        Value::List(_) | Value::Map(_) => false,
        Value::Time(t) => t.is_zero(),
        Value::Object(o) => {
            if let Some(n) = o.as_any().downcast_ref::<super::methods::Number>() {
                return n.0.is_empty();
            }
            if let Some(z) = o.is_zero() {
                return z;
            }
            // A named basic type: reflect.Value.IsZero of its Kind.
            if let Some(u) = basic_underlying(o.as_ref()) {
                return is_zero(&u, Static::Concrete);
            }
            if o.kind() == Kind::Struct {
                if let Some(fields) = o.struct_fields() {
                    return fields.iter().all(|(_, v)| is_zero(v, Static::Concrete));
                }
            }
            false
        }
    }
}

// Go: neohugo common/maps/params.go:(Params).IsZero
/// Params is zero when empty or when its only key is the merge strategy key.
fn params_is_zero(m: &Map) -> bool {
    if m.entries.is_empty() {
        return true;
    }
    if m.entries.len() > 1 {
        return false;
    }
    m.entries
        .keys()
        .next()
        .is_some_and(|k| k.as_bytes() == b"_merge")
}

// Go: arshal_default.go:stringOrNumberKind
fn string_or_number_kind(is_string: bool) -> u8 {
    if is_string { b'"' } else { b'0' }
}

// Go: arshal_default.go:makeInterfaceArshaler (marshal, t == any)
pub(crate) fn marshal_interface(enc: &mut Enc, v: &Value) -> Option<Err> {
    let t = "interface {}";
    if enc.opts.flags.has(jsonflags::TAG_FLAGS) && enc.opts.flags.get(jsonflags::STRING_TAG) {
        if !enc
            .opts
            .flags
            .get(jsonflags::REPORT_ERRORS_WITH_LEGACY_SEMANTICS)
        {
            return Some(new_marshal_error_before(
                enc,
                t,
                Some(Err::InvalidStringTag),
            ));
        }
        if enc
            .opts
            .flags
            .get(jsonflags::STRINGIFY_WITH_LEGACY_SEMANTICS)
        {
            enc.opts.flags.clear(jsonflags::STRING_TAG); // the `string` tag option does not apply to interface types
        }
    }
    if matches!(v, Value::Invalid) {
        return enc.write_token(&Token::Null);
    }
    // (`any` has no methods, so CallMethodsWithLegacySemantics does not apply.)

    // Optimize for the any type if there are no special options.
    if !enc
        .opts
        .flags
        .get(jsonflags::STRINGIFY_NUMBERS | jsonflags::TAG_FLAGS)
    {
        return super::any::marshal_value_any(enc, v);
    }
    super::marshal_concrete(enc, v)
}

// ---------------------------------------------------------------------------
// Unmarshal

// Go: arshal_default.go:makeInterfaceArshaler (unmarshal, t == any, nil)
/// Unmarshals the next value into a nil `interface{}`.
pub(crate) fn unmarshal_interface(dec: &mut Dec) -> (Value, Option<Err>) {
    crate::stack::guard(|| {
        // (Tag flags are never set while decoding into interface values, and
        // the target is always nil, so MergeWithLegacySemantics has no effect.)
        if dec.peek_kind() == b'n' {
            if let Err(err) = dec.read_token() {
                return (Value::Invalid, Some(err));
            }
            return (Value::Invalid, None);
        }

        // Go: the unmarshalValueAny fast path is disabled by AllowDuplicateNames.
        let k = dec.peek_kind();
        match k {
            b'f' | b't' => {
                let (b, err) = unmarshal_bool(dec);
                (Value::Bool(b), err)
            }
            b'"' => {
                let (s, err) = unmarshal_string(dec, "string");
                (Value::String(s), err)
            }
            b'0' => {
                if dec.opts.flags.get(jsonflags::UNMARSHAL_ANY_WITH_RAW_NUMBER) {
                    super::methods::unmarshal_number(dec)
                } else {
                    let (f, err) = unmarshal_float64(dec);
                    (Value::float64(f), err)
                }
            }
            b'{' => unmarshal_map_string_any(dec, None),
            b'[' => unmarshal_slice_any(dec),
            _ => {
                let mut flags = ValueFlags::default();
                match dec.read_value(&mut flags) {
                    Err(err) => (Value::Invalid, Some(err)),
                    Ok(_) => (Value::Invalid, None),
                }
            }
        }
    })
}

// Go: arshal_default.go:makeBoolArshaler (unmarshal)
fn unmarshal_bool(dec: &mut Dec) -> (bool, Option<Err>) {
    let tok = match dec.read_token() {
        Ok(t) => t,
        Err(err) => return (false, Some(err)),
    };
    let k = tok.kind();
    match k {
        b'n' => return (false, None),
        b't' | b'f' => return (tok.as_bool(), None),
        _ => {}
    }
    (
        false,
        Some(new_unmarshal_error_after_with_skipping(dec, "bool", None)),
    )
}

// Go: arshal_default.go:makeStringArshaler (unmarshal)
/// Unmarshals a string (the target starts as "").
fn unmarshal_string(dec: &mut Dec, t: &str) -> (GoString, Option<Err>) {
    let mut flags = ValueFlags::default();
    let r = match dec.read_value(&mut flags) {
        Ok(r) => r,
        Err(err) => return (GoString::empty(), Some(err)),
    };
    let val = &dec.buf[r];
    let k = crate::jsontext::encode::value_kind(val);
    match k {
        b'n' => return (GoString::empty(), None),
        b'"' => {
            let val = jsonwire::unquote_may_copy(val, flags.is_verbatim());
            return (GoString::from(val), None);
        }
        _ => {}
    }
    (
        GoString::empty(),
        Some(Err::Semantic(Box::new(new_unmarshal_error_after(
            dec, t, None,
        )))),
    )
}

// Go: arshal_default.go:makeFloatArshaler (unmarshal, float64)
fn unmarshal_float64(dec: &mut Dec) -> (f64, Option<Err>) {
    let t = "float64";
    let stringify = dec.state.tokens.last.need_object_name()
        || dec
            .opts
            .flags
            .get(jsonflags::STRINGIFY_NUMBERS | jsonflags::STRING_TAG);
    let mut flags = ValueFlags::default();
    let r = match dec.read_value(&mut flags) {
        Ok(r) => r,
        Err(err) => return (0.0, Some(err)),
    };
    let val = dec.buf[r].to_vec();
    let k = crate::jsontext::encode::value_kind(&val);
    match k {
        b'n' => return (0.0, None),
        b'0' if !stringify => {
            let (fv, err) = go_strconv::internal::parse_float(&val, 64);
            if let Some(err) = err {
                let e = match err {
                    go_strconv::internal::Error::Range => Err::StrconvRange,
                    _ => Err::StrconvSyntax,
                };
                return (
                    fv,
                    Some(new_unmarshal_error_after_with_value(dec, t, Some(e))),
                );
            }
            return (fv, None);
        }
        _ => {}
    }
    (
        0.0,
        Some(Err::Semantic(Box::new(new_unmarshal_error_after(
            dec, t, None,
        )))),
    )
}

/// Go type name of `map[string]any`.
pub(crate) const MAP_STRING_ANY: &str = "map[string]interface {}";

// Go: arshal_default.go:makeMapArshaler (unmarshal, map[string]any)
/// Unmarshals into a `map[string]any` holding `existing` (None: nil map).
pub(crate) fn unmarshal_map_string_any(
    dec: &mut Dec,
    existing: Option<Map>,
) -> (Value, Option<Err>) {
    let t = MAP_STRING_ANY;
    let current = |m: &Option<Map>| match m {
        Some(m) => Value::map(m.clone()),
        None => Value::TypedNil(t.into()),
    };
    let tok = match dec.read_token() {
        Ok(tok) => tok,
        Err(err) => return (current(&existing), Some(err)),
    };
    let k = tok.kind();
    match k {
        b'n' => (Value::TypedNil(t.into()), None),
        b'{' => {
            let mut entries: BTreeMap<GoString, Value> = match existing {
                Some(m) => m.entries,
                None => BTreeMap::new(), // va.Set(reflect.MakeMap(t))
            };
            // (string keys with AllowInvalidUTF8 keep the namespace enabled;
            // duplicate names are allowed under v1, so `seen` is not needed)
            let mut err_unmarshal: Option<Err> = None;
            while dec.peek_kind() != b'}' {
                // Unmarshal the map entry key.
                let (key, err) = unmarshal_string(dec, "string");
                if let Some(err) = err {
                    if is_fatal_error(&err, &dec.opts.flags) {
                        return (map_string_any(entries), Some(err));
                    }
                    if let Some(err) = dec.skip_value() {
                        return (map_string_any(entries), Some(err));
                    }
                    err_unmarshal = err_unmarshal.or(Some(err));
                    continue;
                }

                // Unmarshal the map entry value. A duplicate key starts from
                // the zero value under MergeWithLegacySemantics.
                let (val, err) = unmarshal_interface(dec);
                entries.insert(key, val);
                if let Some(err) = err {
                    if is_fatal_error(&err, &dec.opts.flags) {
                        return (map_string_any(entries), Some(err));
                    }
                    err_unmarshal = err_unmarshal.or(Some(err));
                }
            }
            if let Err(err) = dec.read_token() {
                return (map_string_any(entries), Some(err));
            }
            (map_string_any(entries), err_unmarshal)
        }
        _ => {
            let err = new_unmarshal_error_after_with_skipping(dec, t, None);
            (current(&existing), Some(err))
        }
    }
}

// Go: arshal_default.go:makeSliceArshaler (unmarshal, []any)
/// Unmarshals into a nil `[]any`.
fn unmarshal_slice_any(dec: &mut Dec) -> (Value, Option<Err>) {
    let t = "[]interface {}";
    let tok = match dec.read_token() {
        Ok(tok) => tok,
        Err(err) => return (Value::TypedNil(t.into()), Some(err)),
    };
    let k = tok.kind();
    match k {
        b'n' => (Value::TypedNil(t.into()), None),
        b'[' => {
            let mut items: Vec<Value> = Vec::new();
            let mut err_unmarshal: Option<Err> = None;
            while dec.peek_kind() != b']' {
                let (v, err) = unmarshal_interface(dec);
                items.push(v);
                if let Some(err) = err {
                    if is_fatal_error(&err, &dec.opts.flags) {
                        return (Value::any_list(items), Some(err));
                    }
                    err_unmarshal = err_unmarshal.or(Some(err));
                }
            }
            // (i == 0: va.Set(emptySlice), an empty non-nil slice)
            if let Err(err) = dec.read_token() {
                return (Value::any_list(items), Some(err));
            }
            (Value::any_list(items), err_unmarshal)
        }
        _ => {
            let err = new_unmarshal_error_after_with_skipping(dec, t, None);
            (Value::TypedNil(t.into()), Some(err))
        }
    }
}

/// `struct_fields` of a host struct as [`FieldRef`]s (exported fields
/// without tags, of their concrete types).
pub(crate) fn plain_struct_fields<'a>(
    fields: &'a [(Cow<'a, str>, Value)],
) -> impl Iterator<Item = FieldRef<'a>> {
    fields.iter().map(|(name, value)| FieldRef {
        name: name.as_bytes(),
        value,
        omit_empty: false,
        omit_zero: false,
        string: false,
        interface_typed: false,
    })
}
