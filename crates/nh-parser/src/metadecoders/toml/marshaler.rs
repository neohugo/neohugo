//! Port of `github.com/pelletier/go-toml/v2@v2.2.4/marshaler.go`: the `Encoder` as
//! `parser.InterfaceToConfig` uses it (`NewEncoder(w).SetIndentTables(true).Encode(v)`) over
//! the values neohugo's decoders and templates produce (maps, slices, strings, numbers,
//! booleans, `time.Time`, the go-toml local types and other `encoding.TextMarshaler`s).
//!
//! Owner: gaps follow-up of Wave B task T03 (parser-langs).
//!
//! Struct values (`encodeStruct`, `walkStruct`, struct tags, comments) are not ported: no
//! neohugo value that reaches the encoder is a Go struct without a `MarshalText` method, and
//! such an object gives a `neohugo-rs:` error. Map values carry no struct-tag options, so the
//! `valueOptions` are always zero (no `multiline`, `omitempty`, `commented`, `comment`).

use go_value::{FloatKind, Map, MapType, Value};

use super::characters::invalid_ascii;

/// An `Encode` error: Go's text, or an unsupported value.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EncodeError {
    Go(String),
    Unsupported(String),
}

/// Go: `toml.Encoder` with its settings.
pub struct Encoder {
    // global settings
    arrays_multiline: bool,
    indent_symbol: Vec<u8>,
    indent_tables: bool,
}

#[derive(Clone, Default)]
struct EncoderCtx {
    // Current top-level key.
    parent_key: Vec<Vec<u8>>,
    // Key that should be used for a KV.
    key: Vec<u8>,
    // Extra flag to account for the empty string
    has_key: bool,
    // Set to true to indicate that the encoder is inside a KV, so that all
    // tables need to be inlined.
    inside_kv: bool,
    // Set to true to skip the first table header in an array table.
    skip_table_header: bool,
    // Indentation level
    indent: usize,
    // (options.multiline; always false for map values)
    multiline: bool,
}

impl EncoderCtx {
    // Go: marshaler.go:(*encoderCtx).shiftKey
    fn shift_key(&mut self) {
        if self.has_key {
            self.parent_key.push(std::mem::take(&mut self.key));
            self.clear_key();
        }
    }

    // Go: marshaler.go:(*encoderCtx).setKey
    fn set_key(&mut self, k: &[u8]) {
        self.key = k.to_vec();
        self.has_key = true;
    }

    // Go: marshaler.go:(*encoderCtx).clearKey
    fn clear_key(&mut self) {
        self.key.clear();
        self.has_key = false;
    }

    // Go: marshaler.go:(*encoderCtx).isRoot
    fn is_root(&self) -> bool {
        self.parent_key.is_empty() && !self.has_key
    }
}

/// Go: `table` (`entry.Value` is the map value).
#[derive(Default)]
struct Table<'a> {
    kvs: Vec<(Vec<u8>, &'a Value)>,
    tables: Vec<(Vec<u8>, &'a Value)>,
}

/// The reflect kind a value has for the encoder.
enum Kind<'a> {
    /// A nil interface (`Value::Invalid`).
    NilInterface,
    Map(&'a Map),
    EmptyMap,
    Slice(&'a [Value]),
    /// `time.Time`.
    Time(&'a go_value::Time),
    /// A value whose `String()` go-toml writes (`LocalDate`, `LocalTime`, `LocalDateTime`).
    Local(String),
    /// An `encoding.TextMarshaler` (type name, text).
    TextMarshaler(String, Result<Vec<u8>, String>),
    String(std::borrow::Cow<'a, [u8]>),
    Float(f64, bool),
    Bool(bool),
    Uint(u64),
    Int(i64),
    Unsupported,
}

fn kind_of(v: &Value) -> Kind<'_> {
    match v {
        Value::Invalid => Kind::NilInterface,
        Value::Map(m) => Kind::Map(m),
        Value::List(l) => Kind::Slice(&l.items),
        Value::Time(t) => Kind::Time(t),
        Value::String(s) | Value::Safe(_, s) => Kind::String(s.as_bytes().into()),
        Value::Float(f, k) => Kind::Float(*f, *k == FloatKind::F32),
        Value::Bool(b) => Kind::Bool(*b),
        Value::Uint(u, _) => Kind::Uint(*u),
        Value::Int(i, _) => Kind::Int(*i),
        Value::TypedNil(t) => match go_value::typed_nil_kind(t) {
            go_value::NilKind::Map => Kind::EmptyMap,
            go_value::NilKind::Slice => Kind::Slice(&[]),
            _ => Kind::Unsupported,
        },
        Value::Object(o) => {
            if let Some(l) = o.as_any().downcast_ref::<super::TomlLocal>() {
                return Kind::Local(l.string());
            }
            if let Some(text) = o.marshal_text() {
                return Kind::TextMarshaler(
                    o.type_name().into_owned(),
                    text.map_err(|e| e.message().to_string()),
                );
            }
            match o.underlying() {
                Some(Value::String(s)) => Kind::String(s.as_bytes().to_vec().into()),
                Some(Value::Bool(b)) => Kind::Bool(b),
                Some(Value::Int(i, _)) => Kind::Int(i),
                Some(Value::Uint(u, _)) => Kind::Uint(u),
                Some(Value::Float(f, k)) => Kind::Float(f, k == FloatKind::F32),
                _ => Kind::Unsupported,
            }
        }
    }
}

fn unsupported(v: &Value) -> EncodeError {
    EncodeError::Unsupported(format!(
        "neohugo-rs: TOML encoding of {} is not supported",
        v.go_type_name()
    ))
}

const LITERAL_QUOTE: u8 = b'\'';

impl Encoder {
    /// Go: `toml.NewEncoder(w)`.
    // Go: marshaler.go:NewEncoder
    pub fn new() -> Encoder {
        Encoder {
            arrays_multiline: false,
            indent_symbol: b"  ".to_vec(),
            indent_tables: false,
        }
    }

    /// SetIndentTables forces the encoder to intent tables and array tables.
    // Go: marshaler.go:(*Encoder).SetIndentTables
    pub fn set_indent_tables(&mut self, indent: bool) -> &mut Encoder {
        self.indent_tables = indent;
        self
    }

    /// Encode writes a TOML representation of v to the stream.
    // Go: marshaler.go:(*Encoder).Encode
    pub fn encode(&self, w: &mut Vec<u8>, v: &Value) -> Result<(), EncodeError> {
        let b: Vec<u8> = Vec::new();
        let ctx = EncoderCtx::default();

        if matches!(v, Value::Invalid) {
            return Err(EncodeError::Go(
                "toml: cannot encode a nil interface".to_string(),
            ));
        }

        let b = self.encode_value(b, &ctx, v)?;
        w.extend_from_slice(&b);
        Ok(())
    }

    // Go: marshaler.go:(*Encoder).encode
    fn encode_value(
        &self,
        mut b: Vec<u8>,
        ctx: &EncoderCtx,
        v: &Value,
    ) -> Result<Vec<u8>, EncodeError> {
        match kind_of(v) {
            Kind::Time(t) => {
                use go_time::GoTimeExt;
                if t.nsec > 0 {
                    t.append_format(&mut b, go_time::RFC3339_NANO.as_bytes());
                } else {
                    t.append_format(&mut b, go_time::RFC3339.as_bytes());
                }
                Ok(b)
            }
            Kind::Local(s) => {
                b.extend_from_slice(s.as_bytes());
                Ok(b)
            }
            Kind::TextMarshaler(type_name, text) => {
                if ctx.is_root() {
                    return Err(EncodeError::Go(format!(
                        "toml: type {type_name} implementing the TextMarshaler interface cannot be a root element"
                    )));
                }
                let text = text.map_err(EncodeError::Go)?;
                Ok(self.encode_string(b, &text, ctx.multiline))
            }
            // containers
            Kind::Map(m) => self.encode_map(b, ctx, m),
            Kind::EmptyMap => self.encode_map(b, ctx, &Map::new(MapType::StringAny)),
            Kind::Slice(items) => self.encode_slice(b, ctx, items),
            Kind::NilInterface => Err(EncodeError::Go(
                "toml: encoding a nil interface is not supported".to_string(),
            )),
            // values
            Kind::String(s) => Ok(self.encode_string(b, &s, ctx.multiline)),
            Kind::Float(f, true) => {
                if f.is_nan() {
                    b.extend_from_slice(b"nan");
                } else if f > f32::MAX as f64 {
                    b.extend_from_slice(b"inf");
                } else if f < -(f32::MAX as f64) {
                    b.extend_from_slice(b"-inf");
                } else if f.trunc() == f {
                    go_strconv::append_float(&mut b, f, b'f', 1, 32);
                } else {
                    go_strconv::append_float(&mut b, f, b'f', -1, 32);
                }
                Ok(b)
            }
            Kind::Float(f, false) => {
                if f.is_nan() {
                    b.extend_from_slice(b"nan");
                } else if f > f64::MAX {
                    b.extend_from_slice(b"inf");
                } else if f < -f64::MAX {
                    b.extend_from_slice(b"-inf");
                } else if f.trunc() == f {
                    go_strconv::append_float(&mut b, f, b'f', 1, 64);
                } else {
                    go_strconv::append_float(&mut b, f, b'f', -1, 64);
                }
                Ok(b)
            }
            Kind::Bool(x) => {
                b.extend_from_slice(if x { b"true" } else { b"false" });
                Ok(b)
            }
            Kind::Uint(x) => {
                if x > i64::MAX as u64 {
                    return Err(EncodeError::Go(format!(
                        "toml: not encoding uint ({x}) greater than max int64 ({})",
                        i64::MAX
                    )));
                }
                b.extend_from_slice(x.to_string().as_bytes());
                Ok(b)
            }
            Kind::Int(x) => {
                b.extend_from_slice(x.to_string().as_bytes());
                Ok(b)
            }
            Kind::Unsupported => Err(unsupported(v)),
        }
    }

    // Go: marshaler.go:(*Encoder).encodeKv
    fn encode_kv(
        &self,
        mut b: Vec<u8>,
        ctx: &EncoderCtx,
        v: &Value,
    ) -> Result<Vec<u8>, EncodeError> {
        // Go: `if !ctx.inline` (the Encoder's tablesInline, always false here; also inside an
        // inline table): no comment, not commented, indented.
        b = self.indent(ctx.indent, b);

        b = self.encode_key(b, &ctx.key);
        b.extend_from_slice(b" = ");

        // create a copy of the context because the value of a KV shouldn't
        // modify the global context.
        let mut subctx = ctx.clone();
        subctx.inside_kv = true;
        subctx.shift_key();
        subctx.multiline = false;

        self.encode_value(b, &subctx, v)
    }

    // Go: marshaler.go:(*Encoder).encodeString
    fn encode_string(&self, b: Vec<u8>, v: &[u8], multiline: bool) -> Vec<u8> {
        if needs_quoting(v) {
            return self.encode_quoted_string(multiline, b, v);
        }

        self.encode_literal_string(b, v)
    }

    // caller should have checked that the string does not contain new lines or ' .
    // Go: marshaler.go:(*Encoder).encodeLiteralString
    fn encode_literal_string(&self, mut b: Vec<u8>, v: &[u8]) -> Vec<u8> {
        b.push(LITERAL_QUOTE);
        b.extend_from_slice(v);
        b.push(LITERAL_QUOTE);
        b
    }

    // Go: marshaler.go:(*Encoder).encodeQuotedString
    fn encode_quoted_string(&self, multiline: bool, mut b: Vec<u8>, v: &[u8]) -> Vec<u8> {
        let string_quote: &[u8] = if multiline { b"\"\"\"" } else { b"\"" };

        b.extend_from_slice(string_quote);
        if multiline {
            b.push(b'\n');
        }

        const HEXTABLE: &[u8] = b"0123456789ABCDEF";
        // U+0000 to U+0008, U+000A to U+001F, U+007F
        const NUL: u8 = 0x0;
        const BS: u8 = 0x8;
        const LF: u8 = 0xa;
        const US: u8 = 0x1f;
        const DEL: u8 = 0x7f;

        for &r in v {
            match r {
                b'\\' => b.extend_from_slice(b"\\\\"),
                b'"' => b.extend_from_slice(b"\\\""),
                0x08 => b.extend_from_slice(b"\\b"),
                0x0c => b.extend_from_slice(b"\\f"),
                b'\n' => {
                    if multiline {
                        b.push(r);
                    } else {
                        b.extend_from_slice(b"\\n");
                    }
                }
                b'\r' => b.extend_from_slice(b"\\r"),
                b'\t' => b.extend_from_slice(b"\\t"),
                _ => {
                    if (NUL..=BS).contains(&r) || (LF..=US).contains(&r) || r == DEL {
                        b.extend_from_slice(b"\\u00");
                        b.push(HEXTABLE[(r >> 4) as usize]);
                        b.push(HEXTABLE[(r & 0x0f) as usize]);
                    } else {
                        b.push(r);
                    }
                }
            }
        }

        b.extend_from_slice(string_quote);
        b
    }

    // Go: marshaler.go:(*Encoder).encodeTableHeader
    fn encode_table_header(&self, ctx: &EncoderCtx, mut b: Vec<u8>) -> Vec<u8> {
        if ctx.parent_key.is_empty() {
            return b;
        }

        b = self.indent(ctx.indent, b);

        b.push(b'[');

        b = self.encode_key(b, &ctx.parent_key[0]);

        for k in &ctx.parent_key[1..] {
            b.push(b'.');
            b = self.encode_key(b, k);
        }

        b.extend_from_slice(b"]\n");
        b
    }

    // Go: marshaler.go:(*Encoder).encodeKey
    fn encode_key(&self, mut b: Vec<u8>, k: &[u8]) -> Vec<u8> {
        let mut needs_quotation = false;
        let mut cannot_use_literal = false;

        if k.is_empty() {
            b.extend_from_slice(b"''");
            return b;
        }

        for c in go_unicode::utf8::runes(k) {
            let c = c.1;
            if (('A' as i32)..=('Z' as i32)).contains(&c)
                || (('a' as i32)..=('z' as i32)).contains(&c)
                || (('0' as i32)..=('9' as i32)).contains(&c)
                || c == '-' as i32
                || c == '_' as i32
            {
                continue;
            }

            if c == LITERAL_QUOTE as i32 {
                cannot_use_literal = true;
            }

            needs_quotation = true;
        }

        if needs_quotation && needs_quoting(k) {
            cannot_use_literal = true;
        }

        if cannot_use_literal {
            self.encode_quoted_string(false, b, k)
        } else if needs_quotation {
            self.encode_literal_string(b, k)
        } else {
            b.extend_from_slice(k);
            b
        }
    }

    // Go: marshaler.go:(*Encoder).encodeMap
    fn encode_map(&self, b: Vec<u8>, ctx: &EncoderCtx, m: &Map) -> Result<Vec<u8>, EncodeError> {
        let mut t = Table::default();

        for (k, v) in &m.entries {
            if is_nil(v) {
                continue;
            }

            // (string keys: keyToString is the key itself)
            if will_convert_to_table_or_array_table(ctx, v) {
                t.tables.push((k.as_bytes().to_vec(), v));
            } else {
                t.kvs.push((k.as_bytes().to_vec(), v));
            }
        }

        // sortEntriesByKey: the keys are distinct (and already in byte order).
        t.kvs.sort_by(|a, b| a.0.cmp(&b.0));
        t.tables.sort_by(|a, b| a.0.cmp(&b.0));

        self.encode_table(b, ctx.clone(), &t)
    }

    // Go: marshaler.go:(*Encoder).encodeTable
    fn encode_table(
        &self,
        mut b: Vec<u8>,
        mut ctx: EncoderCtx,
        t: &Table<'_>,
    ) -> Result<Vec<u8>, EncodeError> {
        ctx.shift_key();

        // (ctx.inline is false: SetTablesInline is not called)
        if ctx.inside_kv {
            return self.encode_table_inline(b, ctx, t);
        }

        if !ctx.skip_table_header {
            b = self.encode_table_header(&ctx, b);

            if self.indent_tables && !ctx.parent_key.is_empty() {
                ctx.indent += 1;
            }
        }
        ctx.skip_table_header = false;

        let mut has_non_empty_kv = false;
        for (k, v) in &t.kvs {
            has_non_empty_kv = true;

            ctx.set_key(k);
            let ctx2 = ctx.clone();

            b = self.encode_kv(b, &ctx2, v)?;

            b.push(b'\n');
        }

        let mut first = true;
        for (k, v) in &t.tables {
            if first {
                first = false;
                if has_non_empty_kv {
                    b.push(b'\n');
                }
            } else {
                b.push(b'\n');
            }

            ctx.set_key(k);

            ctx.multiline = false;
            let ctx2 = ctx.clone();

            b = self.encode_value(b, &ctx2, v)?;
        }

        Ok(b)
    }

    // Go: marshaler.go:(*Encoder).encodeTableInline
    fn encode_table_inline(
        &self,
        mut b: Vec<u8>,
        mut ctx: EncoderCtx,
        t: &Table<'_>,
    ) -> Result<Vec<u8>, EncodeError> {
        b.push(b'{');

        let mut first = true;
        for (k, v) in &t.kvs {
            if first {
                first = false;
            } else {
                b.extend_from_slice(b", ");
            }

            ctx.set_key(k);

            b = self.encode_kv(b, &ctx, v)?;
        }

        if !t.tables.is_empty() {
            panic!("inline table cannot contain nested tables, only key-values");
        }

        b.push(b'}');

        Ok(b)
    }

    // Go: marshaler.go:(*Encoder).encodeSlice
    fn encode_slice(
        &self,
        mut b: Vec<u8>,
        ctx: &EncoderCtx,
        items: &[Value],
    ) -> Result<Vec<u8>, EncodeError> {
        if items.is_empty() {
            b.extend_from_slice(b"[]");

            return Ok(b);
        }

        if will_convert_slice_to_array_table(ctx, items) {
            return self.encode_slice_as_array_table(b, ctx.clone(), items);
        }

        self.encode_slice_as_array(b, ctx, items)
    }

    // caller should have checked that v is a slice that only contains values that
    // encode into tables.
    // Go: marshaler.go:(*Encoder).encodeSliceAsArrayTable
    fn encode_slice_as_array_table(
        &self,
        mut b: Vec<u8>,
        mut ctx: EncoderCtx,
        items: &[Value],
    ) -> Result<Vec<u8>, EncodeError> {
        ctx.shift_key();

        let mut scratch: Vec<u8> = Vec::with_capacity(64);

        if self.indent_tables {
            scratch = self.indent(ctx.indent, scratch);
        }

        scratch.extend_from_slice(b"[[");

        for (i, k) in ctx.parent_key.iter().enumerate() {
            if i > 0 {
                scratch.push(b'.');
            }

            scratch = self.encode_key(scratch, k);
        }

        scratch.extend_from_slice(b"]]\n");
        ctx.skip_table_header = true;

        if self.indent_tables {
            ctx.indent += 1;
        }

        for (i, v) in items.iter().enumerate() {
            if i != 0 {
                b.push(b'\n');
            }

            b.extend_from_slice(&scratch);

            b = self.encode_value(b, &ctx, v)?;
        }

        Ok(b)
    }

    // Go: marshaler.go:(*Encoder).encodeSliceAsArray
    fn encode_slice_as_array(
        &self,
        mut b: Vec<u8>,
        ctx: &EncoderCtx,
        items: &[Value],
    ) -> Result<Vec<u8>, EncodeError> {
        let multiline = ctx.multiline || self.arrays_multiline;
        let mut separator: &[u8] = b", ";

        b.push(b'[');

        let mut sub_ctx = ctx.clone();
        sub_ctx.multiline = false;

        if multiline {
            separator = b",\n";

            b.push(b'\n');

            sub_ctx.indent += 1;
        }

        let mut first = true;

        for v in items {
            if first {
                first = false;
            } else {
                b.extend_from_slice(separator);
            }

            if multiline {
                b = self.indent(sub_ctx.indent, b);
            }

            b = self.encode_value(b, &sub_ctx, v)?;
        }

        if multiline {
            b.push(b'\n');
            b = self.indent(ctx.indent, b);
        }

        b.push(b']');

        Ok(b)
    }

    // Go: marshaler.go:(*Encoder).indent
    fn indent(&self, level: usize, mut b: Vec<u8>) -> Vec<u8> {
        for _ in 0..level {
            b.extend_from_slice(&self.indent_symbol);
        }

        b
    }
}

impl Default for Encoder {
    fn default() -> Self {
        Encoder::new()
    }
}

/// Go: `isNil(v)` of a map value: the map's values are `interface{}`s (or of a concrete
/// type for typed maps), so only a nil interface is nil.
// Go: marshaler.go:isNil
fn is_nil(v: &Value) -> bool {
    matches!(v, Value::Invalid)
}

// Go: marshaler.go:needsQuoting
fn needs_quoting(v: &[u8]) -> bool {
    // TODO: vectorize
    v.iter()
        .any(|&b| b == b'\'' || b == b'\r' || b == b'\n' || invalid_ascii(b))
}

// Go: marshaler.go:willConvertToTable
fn will_convert_to_table(v: &Value) -> bool {
    match kind_of(v) {
        Kind::NilInterface => false,
        Kind::Time(_) | Kind::Local(_) | Kind::TextMarshaler(..) => false,
        // (ctx.inline is false)
        Kind::Map(_) | Kind::EmptyMap => true,
        // A struct (reflect.Struct) other than the above.
        Kind::Unsupported => matches!(v, Value::Object(o) if o.kind() == go_value::Kind::Struct),
        _ => false,
    }
}

// Go: marshaler.go:willConvertToTableOrArrayTable
fn will_convert_to_table_or_array_table(ctx: &EncoderCtx, v: &Value) -> bool {
    if ctx.inside_kv {
        return false;
    }

    if let Kind::Slice(items) = kind_of(v) {
        return will_convert_slice_to_array_table(ctx, items);
    }

    will_convert_to_table(v)
}

/// The slice case of `willConvertToTableOrArrayTable`.
fn will_convert_slice_to_array_table(ctx: &EncoderCtx, items: &[Value]) -> bool {
    if ctx.inside_kv {
        return false;
    }
    if items.is_empty() {
        // An empty slice should be a kv = [].
        return false;
    }

    items.iter().all(will_convert_to_table)
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (github.com/pelletier/go-toml/v2@v2.2.4 marshaler.go)
// OK NewEncoder, SetIndentTables, Encode, encode, encodeKv, isNil, needsQuoting, encodeString,
//    encodeLiteralString, encodeQuotedString, encodeTableHeader, encodeKey, keyToString (string
//    keys), encodeMap, sortEntriesByKey, pushKV/pushTable (distinct map keys), encodeTable,
//    encodeTableInline, willConvertToTable, willConvertToTableOrArrayTable, encodeSlice,
//    encodeSliceAsArrayTable, encodeSliceAsArray, indent
// NOT PORTED (no neohugo input): Marshal, SetTablesInline, SetArraysMultiline,
//    SetIndentSymbol, SetMarshalJsonNumbers, walkStruct, encodeStruct, encodeComment,
//    isEmptyValue, isEmptyStruct, isValidName, parseTag, shouldOmitEmpty
// ---------------------------------------------------------------------------
