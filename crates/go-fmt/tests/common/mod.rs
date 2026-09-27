//! Test support: the value spec language of tools/go-oracle/go-fmt/spec.go
//! (parsed into `go_value::Value`), the oracle's host types as `Object`s,
//! and fixture helpers.

#![allow(dead_code)]

use std::borrow::Cow;
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;

use go_value::{
    FloatKind, GoString, HostCtx, IntKind, Kind, List, Map, MapType, Object, Result, SafeKind,
    SliceType, Time, UintKind, Value,
};

pub fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(name)
}

pub fn read_fixture(name: &str) -> String {
    std::fs::read_to_string(fixture_path(name)).unwrap_or_else(|e| panic!("{name}: {e}"))
}

/// Go `strconv.Unquote` of a fixture field.
pub fn unquote(s: &str) -> Vec<u8> {
    go_strconv::unquote(s).unwrap_or_else(|e| panic!("unquote {s:?}: {e}"))
}

/// Go-style quoting for diagnostics.
pub fn q(b: &[u8]) -> String {
    go_strconv::quote(b)
}

// ---------------------------------------------------------------------------
// The oracle's host types.

fn fields_s(name: &'static str, s: &GoString) -> Option<Vec<(Cow<'static, str>, Value)>> {
    Some(vec![(Cow::Borrowed(name), Value::String(s.clone()))])
}

/// Go: `*main.Str` (pointer-receiver Stringer).
pub struct Str(pub GoString);
/// Go: `*main.Err` (pointer-receiver error).
pub struct Err(pub GoString);
/// Go: `*main.Both` (error and Stringer).
pub struct Both(pub GoString);
/// Go: `main.SV` (value-receiver Stringer struct).
pub struct Sv(pub GoString);
/// Go: `*main.GS` (pointer-receiver GoStringer and Stringer).
pub struct Gs(pub GoString);
/// Go: `main.Plain` / `*main.Plain`.
pub struct Plain {
    pub ptr: bool,
    pub a: Value,
    pub b: Value,
}
/// Go: `main.NM` as a Kind::Map object.
pub struct NmObj(pub BTreeMap<GoString, Value>);
/// Go: `main.NS` as a Kind::Slice object.
pub struct NsObj(pub Vec<Value>);

macro_rules! no_methods {
    () => {
        fn has_method(&self, _name: &str) -> bool {
            false
        }
        fn call_method(
            &self,
            _ctx: HostCtx<'_>,
            _name: &str,
            _args: &[Value],
        ) -> Option<Result<Value>> {
            None
        }
        fn as_any(&self) -> &dyn std::any::Any {
            self
        }
    };
}

impl Object for Str {
    fn type_name(&self) -> Cow<'_, str> {
        Cow::Borrowed("*main.Str")
    }
    fn go_string(&self) -> Option<GoString> {
        Some(self.0.clone())
    }
    fn struct_fields(&self) -> Option<Vec<(Cow<'_, str>, Value)>> {
        fields_s("S", &self.0)
    }
    no_methods!();
}

impl Object for Err {
    fn type_name(&self) -> Cow<'_, str> {
        Cow::Borrowed("*main.Err")
    }
    fn go_error(&self) -> Option<String> {
        // Error messages in the corpus are valid UTF-8.
        Some(self.0.to_str_lossy().into_owned())
    }
    fn struct_fields(&self) -> Option<Vec<(Cow<'_, str>, Value)>> {
        fields_s("Msg", &self.0)
    }
    no_methods!();
}

impl Object for Both {
    fn type_name(&self) -> Cow<'_, str> {
        Cow::Borrowed("*main.Both")
    }
    fn go_error(&self) -> Option<String> {
        Some(format!("E:{}", self.0.to_str_lossy()))
    }
    fn go_string(&self) -> Option<GoString> {
        let mut b = b"S:".to_vec();
        b.extend_from_slice(&self.0);
        Some(b.into())
    }
    fn struct_fields(&self) -> Option<Vec<(Cow<'_, str>, Value)>> {
        fields_s("S", &self.0)
    }
    no_methods!();
}

impl Object for Sv {
    fn type_name(&self) -> Cow<'_, str> {
        Cow::Borrowed("main.SV")
    }
    fn kind(&self) -> Kind {
        Kind::Struct
    }
    fn go_string(&self) -> Option<GoString> {
        let mut b = b"SV(".to_vec();
        b.extend_from_slice(&self.0);
        b.push(b')');
        Some(b.into())
    }
    fn struct_fields(&self) -> Option<Vec<(Cow<'_, str>, Value)>> {
        fields_s("S", &self.0)
    }
    no_methods!();
}

impl Object for Gs {
    fn type_name(&self) -> Cow<'_, str> {
        Cow::Borrowed("*main.GS")
    }
    fn go_string(&self) -> Option<GoString> {
        Some(self.0.clone())
    }
    fn go_go_string(&self) -> Option<GoString> {
        let mut b = b"GS(".to_vec();
        b.extend_from_slice(go_strconv::quote(&self.0).as_bytes());
        b.push(b')');
        Some(b.into())
    }
    fn struct_fields(&self) -> Option<Vec<(Cow<'_, str>, Value)>> {
        fields_s("S", &self.0)
    }
    no_methods!();
}

impl Object for Plain {
    fn type_name(&self) -> Cow<'_, str> {
        Cow::Borrowed(if self.ptr {
            "*main.Plain"
        } else {
            "main.Plain"
        })
    }
    fn kind(&self) -> Kind {
        if self.ptr { Kind::Ptr } else { Kind::Struct }
    }
    fn struct_fields(&self) -> Option<Vec<(Cow<'_, str>, Value)>> {
        // The fields are `any`: a nil of an interface type stored in them
        // is a nil `any`.
        Some(vec![
            (Cow::Borrowed("A"), any_field(&self.a)),
            (Cow::Borrowed("B"), any_field(&self.b)),
        ])
    }
    no_methods!();
}

/// A value stored in a Go `any` field.
fn any_field(v: &Value) -> Value {
    match v {
        Value::TypedNil(t) if go_fmt::typed_nil_kind(t) == go_fmt::NilKind::Interface => {
            Value::Invalid
        }
        v => v.clone(),
    }
}

impl Object for NmObj {
    fn type_name(&self) -> Cow<'_, str> {
        Cow::Borrowed("main.NM")
    }
    fn kind(&self) -> Kind {
        Kind::Map
    }
    fn map_get(&self, key: &[u8]) -> Option<Value> {
        self.0.get(key).cloned()
    }
    fn map_keys(&self) -> Vec<GoString> {
        self.0.keys().cloned().collect()
    }
    no_methods!();
}

impl Object for NsObj {
    fn type_name(&self) -> Cow<'_, str> {
        Cow::Borrowed("main.NS")
    }
    fn kind(&self) -> Kind {
        Kind::Slice
    }
    fn list(&self) -> Option<Vec<Value>> {
        Some(self.0.clone())
    }
    no_methods!();
}

// ---------------------------------------------------------------------------
// Spec parser.

pub struct Node {
    pub name: String,
    pub payload: String,
    pub children: Option<Vec<Node>>,
}

struct Parser<'a> {
    s: &'a [u8],
    pos: usize,
}

impl Parser<'_> {
    fn node(&mut self) -> Node {
        let start = self.pos;
        while self.pos < self.s.len() {
            let c = self.s[self.pos];
            if c.is_ascii_lowercase() || c.is_ascii_digit() || c == b'_' {
                self.pos += 1;
            } else {
                break;
            }
        }
        assert!(
            self.pos > start,
            "expected name at {} in {:?}",
            self.pos,
            String::from_utf8_lossy(self.s)
        );
        let name = String::from_utf8(self.s[start..self.pos].to_vec()).unwrap();
        let mut payload = String::new();
        if self.pos < self.s.len() && self.s[self.pos] == b':' {
            self.pos += 1;
            let start = self.pos;
            while self.pos < self.s.len() && !matches!(self.s[self.pos], b'(' | b')' | b',') {
                self.pos += 1;
            }
            payload = String::from_utf8(self.s[start..self.pos].to_vec()).unwrap();
        }
        let mut children = None;
        if self.pos < self.s.len() && self.s[self.pos] == b'(' {
            self.pos += 1;
            let mut cs = Vec::new();
            if self.s[self.pos] == b')' {
                self.pos += 1;
            } else {
                loop {
                    cs.push(self.node());
                    match self.s[self.pos] {
                        b',' => self.pos += 1,
                        b')' => {
                            self.pos += 1;
                            break;
                        }
                        c => panic!("unexpected {:?} at {}", c as char, self.pos),
                    }
                }
            }
            children = Some(cs);
        }
        Node {
            name,
            payload,
            children,
        }
    }
}

pub fn parse_spec(s: &str) -> Node {
    let mut p = Parser {
        s: s.as_bytes(),
        pos: 0,
    };
    let n = p.node();
    assert_eq!(p.pos, s.len(), "trailing input in {s:?}");
    n
}

fn unhex(s: &str) -> Vec<u8> {
    assert!(s.len().is_multiple_of(2), "odd hex {s:?}");
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap())
        .collect()
}

fn unhex_str(s: &str) -> String {
    String::from_utf8(unhex(s)).unwrap()
}

pub fn slice_type(name: &str) -> SliceType {
    match name {
        "[]interface {}" => SliceType::Any,
        "[]string" => SliceType::String,
        "[]int" => SliceType::Int,
        "[]int64" => SliceType::Int64,
        "[]float64" => SliceType::Float64,
        "[]bool" => SliceType::Bool,
        "[]uint8" => SliceType::Uint8,
        "[]map[string]interface {}" => SliceType::MapStringAny,
        n => SliceType::Named(Arc::from(n)),
    }
}

pub fn map_type(name: &str) -> MapType {
    match name {
        "map[string]interface {}" => MapType::StringAny,
        "map[string]string" => MapType::StringString,
        "maps.Params" => MapType::Params,
        n => MapType::Named(Arc::from(n)),
    }
}

fn kv_entries(children: &[Node]) -> BTreeMap<GoString, Value> {
    let mut m = BTreeMap::new();
    for c in children {
        assert_eq!(c.name, "kv");
        let cs = c.children.as_ref().unwrap();
        assert_eq!(cs.len(), 1);
        m.insert(GoString::from(unhex(&c.payload)), node_value(&cs[0]));
    }
    m
}

pub fn node_value(n: &Node) -> Value {
    let p = n.payload.as_str();
    let children = || n.children.as_deref().unwrap_or(&[]);
    match n.name.as_str() {
        "nil" => Value::Invalid,
        "bool" => Value::Bool(p == "1"),
        "int" => Value::Int(p.parse().unwrap(), IntKind::Int),
        "int8" => Value::Int(p.parse().unwrap(), IntKind::Int8),
        "int16" => Value::Int(p.parse().unwrap(), IntKind::Int16),
        "int32" => Value::Int(p.parse().unwrap(), IntKind::Int32),
        "int64" => Value::Int(p.parse().unwrap(), IntKind::Int64),
        "uint" => Value::Uint(p.parse().unwrap(), UintKind::Uint),
        "uint8" => Value::Uint(p.parse().unwrap(), UintKind::Uint8),
        "uint16" => Value::Uint(p.parse().unwrap(), UintKind::Uint16),
        "uint32" => Value::Uint(p.parse().unwrap(), UintKind::Uint32),
        "uint64" => Value::Uint(p.parse().unwrap(), UintKind::Uint64),
        "uintptr" => Value::Uint(p.parse().unwrap(), UintKind::Uintptr),
        "f64" => Value::Float(
            f64::from_bits(u64::from_str_radix(p, 16).unwrap()),
            FloatKind::F64,
        ),
        "f32" => Value::Float(
            go_strconv::internal::f32_to_f64(f32::from_bits(u32::from_str_radix(p, 16).unwrap())),
            FloatKind::F32,
        ),
        "str" => Value::String(unhex(p).into()),
        "html" => Value::Safe(SafeKind::Html, unhex(p).into()),
        "htmlattr" => Value::Safe(SafeKind::HtmlAttr, unhex(p).into()),
        "css" => Value::Safe(SafeKind::Css, unhex(p).into()),
        "js" => Value::Safe(SafeKind::Js, unhex(p).into()),
        "jsstr" => Value::Safe(SafeKind::JsStr, unhex(p).into()),
        "url" => Value::Safe(SafeKind::Url, unhex(p).into()),
        "srcset" => Value::Safe(SafeKind::Srcset, unhex(p).into()),
        "bytes" => Value::List(Arc::new(List::new(
            SliceType::Uint8,
            unhex(p)
                .into_iter()
                .map(|b| Value::Uint(b as u64, UintKind::Uint8))
                .collect(),
        ))),
        "tnil" => Value::TypedNil(Arc::from(unhex_str(p).as_str())),
        "list" => Value::List(Arc::new(List::new(
            slice_type(&unhex_str(p)),
            children().iter().map(node_value).collect(),
        ))),
        "map" => Value::Map(Arc::new(Map::with_entries(
            map_type(&unhex_str(p)),
            kv_entries(children()),
        ))),
        "time" => {
            let parts: Vec<&str> = p.splitn(3, ';').collect();
            let sec: i64 = parts[0].parse().unwrap();
            let nsec: i64 = parts[1].parse().unwrap();
            let loc = parts[2];
            let loc = if loc == "nil" {
                None
            } else if loc == "utc" {
                Some(go_time::utc())
            } else if let Some(rest) = loc.strip_prefix("fixed=") {
                let (name, off) = rest.split_once('=').unwrap();
                Some(go_time::fixed_zone(name, off.parse().unwrap()))
            } else if let Some(z) = loc.strip_prefix("zone=") {
                Some(go_time::load_location(z).unwrap())
            } else {
                panic!("bad loc {loc}")
            };
            Value::Time(Time::from_unix(sec, nsec, loc))
        }
        "obj_str" => Value::object(Str(unhex(p).into())),
        "obj_err" => Value::object(Err(unhex(p).into())),
        "obj_both" => Value::object(Both(unhex(p).into())),
        "obj_sv" => Value::object(Sv(unhex(p).into())),
        "obj_gs" => Value::object(Gs(unhex(p).into())),
        "obj_plain" | "obj_pplain" => {
            let cs = children();
            Value::object(Plain {
                ptr: n.name == "obj_pplain",
                a: node_value(&cs[0]),
                b: node_value(&cs[1]),
            })
        }
        "pages" => Value::List(Arc::new(List::new(
            SliceType::Named(Arc::from("page.Pages")),
            vec![Value::Invalid; p.parse().unwrap()],
        ))),
        "taxlist" => {
            let mut m = Map::new(MapType::Named(Arc::from("page.TaxonomyList")));
            for i in 0..p.parse::<usize>().unwrap() {
                m.insert(
                    format!("t{i}"),
                    Value::map(Map::new(MapType::Named(Arc::from("page.Taxonomy")))),
                );
            }
            Value::map(m)
        }
        "obj_nm" => Value::object(NmObj(kv_entries(children()))),
        "obj_ns" => Value::object(NsObj(children().iter().map(node_value).collect())),
        other => panic!("unknown spec node {other:?}"),
    }
}

pub fn spec_value(s: &str) -> Value {
    node_value(&parse_spec(s))
}

/// An `args(...)` node.
pub fn spec_args(s: &str) -> Vec<Value> {
    let n = parse_spec(s);
    assert_eq!(n.name, "args");
    n.children
        .as_deref()
        .unwrap_or(&[])
        .iter()
        .map(node_value)
        .collect()
}

/// FNV-1a 64 (Go: hash/fnv New64a).
pub struct Fnv64(pub u64);

impl Fnv64 {
    pub fn new() -> Self {
        Fnv64(0xcbf29ce484222325)
    }
    pub fn write(&mut self, b: &[u8]) {
        for &c in b {
            self.0 ^= c as u64;
            self.0 = self.0.wrapping_mul(0x100000001b3);
        }
    }
}
