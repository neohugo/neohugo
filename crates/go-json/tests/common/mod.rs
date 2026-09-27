//! Shared helpers for the oracle tests: the record reader, the vdump codec
//! (see tools/go-oracle/go-json/dump.go) and Go-style result formatting.

#![allow(dead_code)]

use std::borrow::Cow;
use std::io::Read;
use std::sync::Arc;

use go_json::{Error, JsonField, JsonStruct, Number};
use go_value::{
    FloatKind, GoString, IntKind, Kind, List, Map, MapType, Object, SafeKind, SliceType, Time,
    UintKind, Value,
};

/// Fixture location: `tests/fixtures`, or `$GO_JSON_FIXTURES` (used to
/// replay large corpora generated outside the repository).
pub fn fixture_path(name: &str) -> String {
    match std::env::var("GO_JSON_FIXTURES") {
        Ok(dir) if !dir.is_empty() => format!("{}/{}", dir, name),
        _ => format!("{}/tests/fixtures/{}", env!("CARGO_MANIFEST_DIR"), name),
    }
}

/// Reads a gzip-compressed record stream: "<name> <len>\n<data>\n"*.
pub fn read_records(name: &str) -> Vec<(String, Vec<u8>)> {
    let f = std::fs::File::open(fixture_path(name)).unwrap_or_else(|e| panic!("open {name}: {e}"));
    let mut data = Vec::new();
    flate2::read::GzDecoder::new(f)
        .read_to_end(&mut data)
        .unwrap();
    let mut out = Vec::new();
    let mut i = 0;
    while i < data.len() {
        let nl = data[i..].iter().position(|&c| c == b'\n').unwrap() + i;
        let head = std::str::from_utf8(&data[i..nl]).unwrap();
        let (name, len) = head.split_once(' ').unwrap();
        let len: usize = len.parse().unwrap();
        let body = data[nl + 1..nl + 1 + len].to_vec();
        assert_eq!(data[nl + 1 + len], b'\n');
        out.push((name.to_string(), body));
        i = nl + 2 + len;
    }
    out
}

/// Groups records into cases (each starting with a "case" record).
pub fn cases(records: Vec<(String, Vec<u8>)>) -> Vec<Vec<(String, Vec<u8>)>> {
    let mut out: Vec<Vec<(String, Vec<u8>)>> = Vec::new();
    for (n, d) in records {
        if n == "case" {
            out.push(Vec::new());
        } else {
            out.last_mut().unwrap().push((n, d));
        }
    }
    out
}

pub fn get<'a>(case: &'a [(String, Vec<u8>)], name: &str) -> &'a [u8] {
    &case
        .iter()
        .find(|(n, _)| n == name)
        .unwrap_or_else(|| panic!("no record {name}"))
        .1
}

pub fn get_all<'a>(case: &'a [(String, Vec<u8>)], name: &str) -> Vec<&'a [u8]> {
    case.iter()
        .filter(|(n, _)| n == name)
        .map(|(_, d)| d.as_slice())
        .collect()
}

/// Go oracle's errRec: "E<kind>:<offset>:<message>".
pub fn err_rec(err: &Error) -> Vec<u8> {
    // Same precedence as the oracle's errors.As chain.
    if let Error::Marshaler(_) = err {
        return format!("EM:-1:{}", err).into_bytes();
    }
    if let Some(se) = err.as_syntax_error() {
        return format!("ES:{}:{}", se.offset, err).into_bytes();
    }
    let (kind, off) = match err {
        Error::UnmarshalType(e) => ('T', e.offset),
        Error::UnsupportedValue { .. } => ('U', -1),
        Error::UnsupportedType { .. } => ('Y', -1),
        Error::Eof => ('F', -1),
        Error::UnexpectedEof => ('G', -1),
        _ => ('O', -1),
    };
    format!("E{}:{}:{}", kind, off, err).into_bytes()
}

pub fn result(r: Result<Vec<u8>, Error>) -> Vec<u8> {
    match r {
        Ok(b) => {
            let mut v = vec![b'O'];
            v.extend_from_slice(&b);
            v
        }
        Err(e) => err_rec(&e),
    }
}

// ---------------------------------------------------------------------------
// Test host objects mirroring the oracle's Go types.

pub struct Jm(pub Vec<u8>);
pub struct Jerr(pub String);
pub struct Tm(pub Vec<u8>);
pub struct Tmerr(pub String);
pub struct PlainStruct(pub Vec<(String, Value)>);

macro_rules! test_object {
    ($t:ty, $name:expr, $kind:expr, $($body:tt)*) => {
        impl Object for $t {
            fn type_name(&self) -> Cow<'_, str> {
                Cow::Borrowed($name)
            }
            fn kind(&self) -> Kind {
                $kind
            }
            fn has_method(&self, _name: &str) -> bool {
                false
            }
            fn call_method(&self, _ctx: go_value::HostCtx<'_>, _name: &str, _args: &[Value]) -> Option<go_value::Result<Value>> {
                None
            }
            fn as_any(&self) -> &dyn std::any::Any {
                self
            }
            $($body)*
        }
    };
}

test_object!(
    Jm,
    "main.jm",
    Kind::Struct,
    fn marshal_json(&self) -> Option<go_value::Result<Vec<u8>>> {
        Some(Ok(self.0.clone()))
    }
);
test_object!(
    Jerr,
    "main.jerr",
    Kind::Struct,
    fn marshal_json(&self) -> Option<go_value::Result<Vec<u8>>> {
        Some(Err(go_value::Error::new(self.0.clone())))
    }
);
test_object!(
    Tm,
    "main.tm",
    Kind::Struct,
    fn marshal_text(&self) -> Option<go_value::Result<Vec<u8>>> {
        Some(Ok(self.0.clone()))
    }
);
test_object!(
    Tmerr,
    "main.tmerr",
    Kind::Struct,
    fn marshal_text(&self) -> Option<go_value::Result<Vec<u8>>> {
        Some(Err(go_value::Error::new(self.0.clone())))
    }
);
test_object!(
    PlainStruct,
    "struct {...}",
    Kind::Struct,
    fn struct_fields(&self) -> Option<Vec<(Cow<'_, str>, Value)>> {
        Some(
            self.0
                .iter()
                .map(|(n, v)| (Cow::Borrowed(n.as_str()), v.clone()))
                .collect(),
        )
    }
);

// ---------------------------------------------------------------------------
// vdump parsing (Go value -> Value).

pub struct Parser<'a> {
    b: &'a [u8],
    i: usize,
}

impl<'a> Parser<'a> {
    pub fn new(b: &'a [u8]) -> Parser<'a> {
        Parser { b, i: 0 }
    }

    pub fn done(&self) -> bool {
        self.i == self.b.len()
    }

    fn byte(&mut self) -> u8 {
        let c = self.b[self.i];
        self.i += 1;
        c
    }

    fn expect(&mut self, c: u8) {
        assert_eq!(self.byte() as char, c as char, "at {}", self.i);
    }

    fn until(&mut self, end: u8) -> &'a [u8] {
        let start = self.i;
        while self.b[self.i] != end {
            self.i += 1;
        }
        let s = &self.b[start..self.i];
        self.i += 1;
        s
    }

    fn num<T: std::str::FromStr>(&mut self, end: u8) -> T
    where
        T::Err: std::fmt::Debug,
    {
        std::str::from_utf8(self.until(end))
            .unwrap()
            .parse()
            .unwrap()
    }

    fn count(&mut self, open: u8) -> usize {
        self.num(open)
    }

    fn str(&mut self) -> &'a [u8] {
        let n: usize = self.num(b':');
        let s = &self.b[self.i..self.i + n];
        self.i += n;
        s
    }

    fn hex(&mut self, n: usize) -> u64 {
        let s = std::str::from_utf8(&self.b[self.i..self.i + n]).unwrap();
        self.i += n;
        u64::from_str_radix(s, 16).unwrap()
    }

    pub fn value(&mut self) -> Value {
        match self.byte() {
            b'N' => Value::Invalid,
            b'Z' => Value::TypedNil(Arc::from(std::str::from_utf8(self.str()).unwrap())),
            b'T' => Value::Bool(true),
            b'F' => Value::Bool(false),
            b'i' => {
                let k = match self.byte() {
                    b'0' => IntKind::Int,
                    b'1' => IntKind::Int8,
                    b'2' => IntKind::Int16,
                    b'3' => IntKind::Int32,
                    _ => IntKind::Int64,
                };
                Value::Int(self.num(b';'), k)
            }
            b'u' => {
                let k = match self.byte() {
                    b'0' => UintKind::Uint,
                    b'1' => UintKind::Uint8,
                    b'2' => UintKind::Uint16,
                    b'3' => UintKind::Uint32,
                    b'4' => UintKind::Uint64,
                    _ => UintKind::Uintptr,
                };
                Value::Uint(self.num(b';'), k)
            }
            b'd' => Value::Float(f64::from_bits(self.hex(16)), FloatKind::F64),
            b'e' => Value::Float(f32::from_bits(self.hex(8) as u32) as f64, FloatKind::F32),
            b's' => Value::String(GoString::from(self.str())),
            b'h' => {
                let k = match self.byte() {
                    b'0' => SafeKind::Html,
                    b'1' => SafeKind::HtmlAttr,
                    b'2' => SafeKind::Css,
                    b'3' => SafeKind::Js,
                    b'4' => SafeKind::JsStr,
                    b'5' => SafeKind::Url,
                    _ => SafeKind::Srcset,
                };
                Value::Safe(k, GoString::from(self.str()))
            }
            b't' => {
                let sec: i64 = self.num(b';');
                let nsec: i64 = self.num(b';');
                let loc = match self.byte() {
                    b'U' => go_time::utc(),
                    _ => {
                        let name = String::from_utf8_lossy(self.str()).into_owned();
                        let off: i64 = self.num(b';');
                        go_time::fixed_zone(&name, off)
                    }
                };
                Value::Time(Time::from_unix(sec, nsec, Some(loc)))
            }
            b'a' => {
                let ty = match self.byte() {
                    b'A' => SliceType::Any,
                    b'S' => SliceType::String,
                    b'I' => SliceType::Int,
                    b'L' => SliceType::Int64,
                    b'D' => SliceType::Float64,
                    b'B' => SliceType::Bool,
                    b'Y' => SliceType::Uint8,
                    _ => SliceType::MapStringAny,
                };
                let n = self.count(b'[');
                let items = (0..n).map(|_| self.value()).collect();
                self.expect(b']');
                Value::List(Arc::new(List::new(ty, items)))
            }
            b'm' => {
                let ty = match self.byte() {
                    b'A' => MapType::StringAny,
                    _ => MapType::StringString,
                };
                let n = self.count(b'{');
                let mut m = Map::new(ty);
                for _ in 0..n {
                    let k = GoString::from(self.str());
                    let v = self.value();
                    m.entries.insert(k, v);
                }
                self.expect(b'}');
                Value::map(m)
            }
            b'n' => Value::object(Number(GoString::from(self.str()))),
            b'J' => Value::object(Jm(self.str().to_vec())),
            b'E' => Value::object(Jerr(String::from_utf8(self.str().to_vec()).unwrap())),
            b'X' => Value::object(Tm(self.str().to_vec())),
            b'W' => Value::object(Tmerr(String::from_utf8(self.str().to_vec()).unwrap())),
            b'R' => {
                let n = self.count(b'{');
                let mut fields = Vec::new();
                for _ in 0..n {
                    let name = String::from_utf8(self.str().to_vec()).unwrap();
                    fields.push((name, self.value()));
                }
                self.expect(b'}');
                Value::object(PlainStruct(fields))
            }
            b'P' => {
                let n = self.count(b'{');
                let mut fields = Vec::new();
                for _ in 0..n {
                    let flags = self.byte() - b'0';
                    let name = String::from_utf8(self.str().to_vec()).unwrap();
                    let v = self.value();
                    fields.push(JsonField {
                        name,
                        value: v,
                        omit_empty: flags & 1 != 0,
                        omit_zero: flags & 2 != 0,
                        string: flags & 4 != 0,
                        interface_typed: flags & 8 != 0,
                    });
                }
                self.expect(b'}');
                Value::object(JsonStruct::new("struct {...}", fields))
            }
            c => panic!("bad vdump byte {:?} at {}", c as char, self.i - 1),
        }
    }
}

pub fn parse_value(b: &[u8]) -> Value {
    let mut p = Parser::new(b);
    let v = p.value();
    assert!(p.done(), "trailing vdump bytes");
    v
}

// ---------------------------------------------------------------------------
// vdump writing (decoded Value -> bytes), for the types the decoder produces.

fn put_str(b: &mut Vec<u8>, s: &[u8]) {
    b.extend_from_slice(s.len().to_string().as_bytes());
    b.push(b':');
    b.extend_from_slice(s);
}

pub fn dump(b: &mut Vec<u8>, v: &Value) {
    match v {
        Value::Invalid => b.push(b'N'),
        Value::TypedNil(t) => {
            b.push(b'Z');
            put_str(b, t.as_bytes());
        }
        Value::Bool(true) => b.push(b'T'),
        Value::Bool(false) => b.push(b'F'),
        Value::Float(f, FloatKind::F64) => {
            b.extend_from_slice(format!("d{:016x}", f.to_bits()).as_bytes())
        }
        Value::String(s) => {
            b.push(b's');
            put_str(b, s.as_bytes());
        }
        Value::List(l) => {
            assert_eq!(l.ty, SliceType::Any);
            b.extend_from_slice(format!("aA{}[", l.items.len()).as_bytes());
            for it in &l.items {
                dump(b, it);
            }
            b.push(b']');
        }
        Value::Map(m) => {
            assert_eq!(m.ty, MapType::StringAny);
            b.extend_from_slice(format!("mA{}{{", m.entries.len()).as_bytes());
            for (k, v) in &m.entries {
                put_str(b, k.as_bytes());
                dump(b, v);
            }
            b.push(b'}');
        }
        Value::Object(o) => {
            let n = o.as_any().downcast_ref::<Number>().expect("json.Number");
            b.push(b'n');
            put_str(b, n.0.as_bytes());
        }
        other => panic!("dump: unexpected decoded value {other:?}"),
    }
}

pub fn dump_value(v: &Value) -> Vec<u8> {
    let mut b = Vec::new();
    dump(&mut b, v);
    b
}

/// Formats (value, error) like the oracle's unmarshal records.
pub fn unmarshal_rec(v: &Value, err: &Option<Error>) -> Vec<u8> {
    match err {
        None => {
            let mut b = vec![b'O'];
            dump(&mut b, v);
            b
        }
        Some(e) => {
            let mut b = err_rec(e);
            b.push(b'\n');
            dump(&mut b, v);
            b
        }
    }
}

/// Reader returning `chunk` bytes per call, then EOF (the oracle's chunkReader).
pub struct ChunkReader {
    pub data: Vec<u8>,
    pub pos: usize,
    pub chunk: usize,
}

impl Read for ChunkReader {
    fn read(&mut self, p: &mut [u8]) -> std::io::Result<usize> {
        let n = self.chunk.min(p.len()).min(self.data.len() - self.pos);
        p[..n].copy_from_slice(&self.data[self.pos..self.pos + n]);
        self.pos += n;
        Ok(n)
    }
}

/// Shows bytes for failure messages.
pub fn show(b: &[u8]) -> String {
    let s = String::from_utf8_lossy(b);
    if s.len() > 600 {
        format!(
            "{}...",
            &s[..s
                .char_indices()
                .take_while(|(i, _)| *i < 600)
                .last()
                .map_or(0, |(i, _)| i)]
        )
    } else {
        s.into_owned()
    }
}
