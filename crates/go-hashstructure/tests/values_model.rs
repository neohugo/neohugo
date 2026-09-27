//! Differential test of the `go_value::Value` → reflect mapping
//! (`HashValue::Value`, `from_value`) against Go values of the types the
//! port maps each `Value` to (tools/go-oracle/go-hashstructure values.go):
//! `map[string]interface{}`, `maps.Params`, `map[string]string`, `[]any`,
//! `[]string`, `[]int`, `[]int64`, `[]float64`, `[]bool`, `[]uint8`,
//! `[]map[string]any`, typed nils, `template.HTML`, all number kinds and
//! `time.Time`.
//!
//! Columns: desc, hashstructure.Hash(v, xxhash), hashstructure.Hash(v, nil),
//! hashing.HashString(v), hashing.HashString("k", v, v),
//! hashing.HashStringHex([]any{v}).
//!
//! `GO_HASH_FIXTURE_DIR` (uncompressed files) overrides the checked-in
//! gzip fixture, for the large scratch corpora.

use std::collections::{BTreeMap, HashMap};
use std::io::Read;
use std::sync::Arc;

use go_hashstructure::hashing::{hash_string, hash_string_hex};
use go_hashstructure::{Error, HashOptions, HashValue as H, XxHash64, hash};
use go_value::{
    FloatKind, GoString, IntKind, List, Location, Map, MapType, SafeKind, SliceType, Time,
    UintKind, Value,
};

fn read_fixture(name: &str) -> String {
    if let Ok(dir) = std::env::var("GO_HASH_FIXTURE_DIR") {
        return std::fs::read_to_string(format!("{dir}/{name}")).unwrap();
    }
    let path = format!("{}/tests/fixtures/{name}.gz", env!("CARGO_MANIFEST_DIR"));
    let f = std::fs::File::open(&path).unwrap_or_else(|e| panic!("{path}: {e}"));
    let mut s = String::new();
    flate2::read::GzDecoder::new(f)
        .read_to_string(&mut s)
        .unwrap();
    s
}

fn unhex(s: &[u8]) -> Vec<u8> {
    let v = |c: u8| match c {
        b'0'..=b'9' => c - b'0',
        b'a'..=b'f' => c - b'a' + 10,
        _ => panic!("bad hex {c}"),
    };
    s.chunks(2).map(|p| v(p[0]) << 4 | v(p[1])).collect()
}

fn hex(b: &[u8]) -> String {
    b.iter().map(|c| format!("{c:02x}")).collect()
}

fn zones() -> HashMap<String, Arc<Location>> {
    let mut m = HashMap::new();
    for line in read_fixture("tzdata").lines() {
        let (name, data) = line.split_once('\t').unwrap();
        let loc = go_time::load_location_from_tz_data(name, &unhex(data.as_bytes())).unwrap();
        m.insert(name.to_string(), loc);
    }
    m
}

struct P<'a> {
    s: &'a [u8],
    i: usize,
    zones: &'a HashMap<String, Arc<Location>>,
}

impl P<'_> {
    fn eat(&mut self, lit: &str) -> bool {
        if self.s[self.i..].starts_with(lit.as_bytes()) {
            self.i += lit.len();
            true
        } else {
            false
        }
    }

    fn expect(&mut self, lit: &str) {
        assert!(
            self.eat(lit),
            "expected {lit:?} at {} in {:?}",
            self.i,
            String::from_utf8_lossy(self.s)
        );
    }

    fn tok(&mut self) -> &[u8] {
        let st = self.i;
        while self.i < self.s.len() && !b":,;=()".contains(&self.s[self.i]) {
            self.i += 1;
        }
        &self.s[st..self.i]
    }

    fn tok_str(&mut self) -> String {
        String::from_utf8(self.tok().to_vec()).unwrap()
    }

    fn num<T: std::str::FromStr>(&mut self) -> T
    where
        T::Err: std::fmt::Debug,
    {
        self.tok_str().parse().unwrap()
    }

    fn hexbits(&mut self) -> u64 {
        u64::from_str_radix(&self.tok_str(), 16).unwrap()
    }

    fn items(&mut self) -> Vec<Value> {
        let mut out = Vec::new();
        self.expect("(");
        if self.eat(")") {
            return out;
        }
        loop {
            out.push(self.value());
            if self.eat(")") {
                return out;
            }
            self.expect(",");
        }
    }

    fn value(&mut self) -> Value {
        if self.eat("nil") {
            return Value::Invalid;
        }
        if self.eat("tnil:") {
            let t = String::from_utf8(unhex(self.tok())).unwrap();
            return Value::TypedNil(t.into());
        }
        if self.eat("bool:") {
            return Value::Bool(self.tok() == b"1");
        }
        if self.eat("int:") {
            let k = match self.tok() {
                b"int" => IntKind::Int,
                b"int8" => IntKind::Int8,
                b"int16" => IntKind::Int16,
                b"int32" => IntKind::Int32,
                b"int64" => IntKind::Int64,
                k => panic!("int kind {k:?}"),
            };
            self.expect(":");
            return Value::Int(self.num(), k);
        }
        if self.eat("uint:") {
            let k = match self.tok() {
                b"uint" => UintKind::Uint,
                b"uint8" => UintKind::Uint8,
                b"uint16" => UintKind::Uint16,
                b"uint32" => UintKind::Uint32,
                b"uint64" => UintKind::Uint64,
                b"uintptr" => UintKind::Uintptr,
                k => panic!("uint kind {k:?}"),
            };
            self.expect(":");
            return Value::Uint(self.num(), k);
        }
        if self.eat("f32:") {
            return Value::Float(f32::from_bits(self.hexbits() as u32) as f64, FloatKind::F32);
        }
        if self.eat("f64:") {
            return Value::Float(f64::from_bits(self.hexbits()), FloatKind::F64);
        }
        if self.eat("str:") {
            return Value::String(GoString::from(unhex(self.tok())));
        }
        if self.eat("html:") {
            return Value::Safe(SafeKind::Html, GoString::from(unhex(self.tok())));
        }
        if self.eat("time:") {
            let sec: i64 = self.num();
            self.expect(":");
            let nsec: i64 = self.num();
            self.expect(":");
            let loc = if self.eat("utc") {
                None
            } else if self.eat("fixed:") {
                let name = String::from_utf8(unhex(self.tok())).unwrap();
                self.expect(":");
                let off: i64 = self.num();
                Some(go_time::fixed_zone(&name, off))
            } else {
                self.expect("tz:");
                let st = self.i;
                while self.i < self.s.len() && !b",;=()".contains(&self.s[self.i]) {
                    self.i += 1;
                }
                let name = std::str::from_utf8(&self.s[st..self.i]).unwrap();
                Some(self.zones[name].clone())
            };
            return Value::Time(Time::from_unix(sec, nsec, loc));
        }
        if self.eat("list:") {
            let ty = match self.tok() {
                b"any" => SliceType::Any,
                b"string" => SliceType::String,
                b"int" => SliceType::Int,
                b"int64" => SliceType::Int64,
                b"float64" => SliceType::Float64,
                b"bool" => SliceType::Bool,
                b"uint8" => SliceType::Uint8,
                b"mapany" => SliceType::MapStringAny,
                t => panic!("list type {t:?}"),
            };
            let items = self.items();
            return Value::List(Arc::new(List::new(ty, items)));
        }
        if self.eat("map:") {
            let ty = match self.tok() {
                b"any" => MapType::StringAny,
                b"params" => MapType::Params,
                b"strstr" => MapType::StringString,
                t => panic!("map type {t:?}"),
            };
            self.expect("(");
            let mut entries = BTreeMap::new();
            if !self.eat(")") {
                loop {
                    let k = GoString::from(unhex(self.tok()));
                    self.expect("=");
                    let v = self.value();
                    entries.insert(k, v);
                    if self.eat(")") {
                        break;
                    }
                    self.expect(",");
                }
            }
            return Value::Map(Arc::new(Map::with_entries(ty, entries)));
        }
        panic!(
            "unknown desc at {}: {:?}",
            self.i,
            String::from_utf8_lossy(&self.s[self.i..])
        );
    }
}

fn res(r: Result<u64, Error>) -> String {
    match r {
        Ok(h) => format!("{h:x}"),
        Err(e @ Error::GoPanic(_)) => format!("panic:{}", hex(e.to_string().as_bytes())),
        Err(e) => format!("err:{}", hex(e.to_string().as_bytes())),
    }
}

/// neohugo's HashString/HashStringHex panic with the hashstructure error.
fn panicky(r: Result<String, Error>) -> String {
    match r {
        Ok(s) => s,
        Err(e) => format!("panic:{}", hex(e.to_string().as_bytes())),
    }
}

fn variants(v: &Value) -> Vec<String> {
    let hv = H::Value(v.clone());
    let mut xo = HashOptions {
        hasher: Some(Box::new(XxHash64::new())),
        ..Default::default()
    };
    vec![
        res(hash(&hv, Some(&mut xo))),
        res(hash(&hv, None)),
        panicky(hash_string(std::slice::from_ref(&hv))),
        panicky(hash_string(&[H::string("k"), hv.clone(), hv.clone()])),
        panicky(hash_string_hex(&[H::any_slice(vec![hv.clone()])])),
    ]
}

/// Go visits map entries in random order: when several entries fail, the
/// reported error varies. Accept any error then.
fn map_order_error(desc: &str, rust: &str, go: &str) -> bool {
    let failed = |s: &str| s.starts_with("err:") || s.starts_with("panic:");
    desc.contains("map:") && failed(go) && failed(rust)
}

#[test]
fn values_model() {
    let zones = zones();
    let text = read_fixture("values.fixture");
    let mut n = 0;
    let mut bad = 0;
    for line in text.lines() {
        let cols: Vec<&str> = line.split('\t').collect();
        let mut p = P {
            s: cols[0].as_bytes(),
            i: 0,
            zones: &zones,
        };
        let v = p.value();
        assert_eq!(p.i, cols[0].len(), "trailing desc: {}", cols[0]);
        let got = variants(&v);
        n += 1;
        for (i, g) in got.iter().enumerate() {
            if g != cols[1 + i] && !map_order_error(cols[0], g, cols[1 + i]) {
                bad += 1;
                if bad < 15 {
                    eprintln!(
                        "MISMATCH variant {i}: {}\n  go:   {}\n  rust: {}",
                        cols[0],
                        cols[1 + i],
                        g
                    );
                }
                break;
            }
        }
    }
    eprintln!("values: {n} values x 5 variants, {bad} mismatching");
    assert!(n > 0);
    assert_eq!(bad, 0);
}
