//! Differential tests against tools/go-oracle/go-hashstructure fixtures.
//!
//! - random.fixture: random typed Go values (reflect-built) described in a
//!   small language that is parsed back into `HashValue`s; 7 hash variants
//!   per value.
//! - named.fixture: hand-written Go types with methods, mirrored below.
//! - remote.fixture: GetRemote file-cache keys (all 51 present in the
//!   golden build's cache).
//!
//! `GO_HASH_FIXTURE_DIR` (uncompressed files) overrides the checked-in
//! gzip fixtures, for the large scratch corpora.

use std::collections::HashMap;
use std::io::Read;
use std::sync::Arc;

use go_hashstructure::hashing::{self, hash_string, hash_string_hex, hash_uint64};
use go_hashstructure::{
    Error, GoMap, GoStruct, HashOptions, HashValue as H, Hashable, Includable, IncludableMap,
    Receiver, UnsupportedKind, XxHash64, hash,
};
use go_value::{FloatKind, GoString, IntKind, Location, Map, MapType, Time, UintKind, Value};

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

// ---------------------------------------------------------------------------
// Description parser (see the oracle's desc function)

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

    /// Token up to one of the delimiters `:,;=()` or the end.
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

    fn list(&mut self) -> Vec<H> {
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

    fn value(&mut self) -> H {
        if self.eat("nil") {
            return H::Nil;
        }
        if self.eat("zeroiface") {
            return H::Unsupported {
                kind: UnsupportedKind::Interface,
                nil: true,
            };
        }
        if self.eat("bool:") {
            return H::Bool(self.tok() == b"1");
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
            return H::Int(self.num(), k);
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
            return H::Uint(self.num(), k);
        }
        if self.eat("f32:") {
            return H::Float(f32::from_bits(self.hexbits() as u32) as f64, FloatKind::F32);
        }
        if self.eat("f64:") {
            return H::Float(f64::from_bits(self.hexbits()), FloatKind::F64);
        }
        if self.eat("c64:") {
            let re = f32::from_bits(self.hexbits() as u32);
            self.expect(":");
            let im = f32::from_bits(self.hexbits() as u32);
            return H::Complex64(re, im);
        }
        if self.eat("c128:") {
            let re = f64::from_bits(self.hexbits());
            self.expect(":");
            let im = f64::from_bits(self.hexbits());
            return H::Complex128(re, im);
        }
        if self.eat("str:") {
            return H::String(GoString::from(unhex(self.tok())));
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
                // zone names contain '/', '_' — read up to a delimiter
                while self.i < self.s.len() && !b",;=()".contains(&self.s[self.i]) {
                    self.i += 1;
                }
                let name = std::str::from_utf8(&self.s[st..self.i]).unwrap();
                Some(self.zones[name].clone())
            };
            return H::Time(Time::from_unix(sec, nsec, loc));
        }
        if self.eat("iface(") {
            let v = self.value();
            self.expect(")");
            return H::Interface(Box::new(v));
        }
        if self.eat("slice:nil") {
            return H::Slice(None);
        }
        if self.s[self.i..].starts_with(b"slice(") {
            self.i += 5;
            return H::Slice(Some(self.list()));
        }
        if self.s[self.i..].starts_with(b"array(") {
            self.i += 5;
            return H::Array(self.list());
        }
        if self.eat("map:nil") {
            return H::Map(GoMap::nil());
        }
        if self.eat("map(") {
            let mut entries = Vec::new();
            if !self.eat(")") {
                loop {
                    let k = self.value();
                    self.expect("=");
                    let v = self.value();
                    entries.push((k, v));
                    if self.eat(")") {
                        break;
                    }
                    self.expect(",");
                }
            }
            return H::Map(GoMap::new(entries));
        }
        if self.eat("ptr:nil:") {
            let z = self.value();
            return H::Ptr(None, Some(Box::new(z)));
        }
        if self.eat("ptr(") {
            let v = self.value();
            self.expect(")");
            return H::Ptr(Some(Box::new(v)), None);
        }
        if self.eat("struct(") {
            let name = String::from_utf8(unhex(self.tok())).unwrap();
            self.expect(";");
            let mut s = GoStruct::new(name);
            if !self.eat(")") {
                loop {
                    let fname = String::from_utf8(unhex(self.tok())).unwrap();
                    self.expect(":");
                    let exported = self.tok() == b"1";
                    self.expect(":");
                    let tag = String::from_utf8(unhex(self.tok())).unwrap();
                    self.expect(":");
                    let v = self.value();
                    s.fields.push(go_hashstructure::GoField {
                        name: fname,
                        tag,
                        exported,
                        value: v,
                        stringer: None,
                    });
                    if self.eat(")") {
                        break;
                    }
                    self.expect(";");
                }
            }
            return H::Struct(s);
        }
        panic!(
            "unknown desc at {}: {:?}",
            self.i,
            String::from_utf8_lossy(&self.s[self.i..])
        );
    }
}

/// Go non-nil `*T` pointing at a nil interface keeps T (an interface type)
/// as the ZeroNil type; descriptions encode it as `ptr(nil)`.
fn fix_ptr_to_iface(v: H) -> H {
    match v {
        H::Ptr(Some(inner), None) if matches!(*inner, H::Nil) => H::Ptr(
            Some(inner),
            Some(Box::new(H::Unsupported {
                kind: UnsupportedKind::Interface,
                nil: true,
            })),
        ),
        H::Ptr(Some(inner), z) => H::Ptr(Some(Box::new(fix_ptr_to_iface(*inner))), z),
        H::Interface(inner) => H::Interface(Box::new(fix_ptr_to_iface(*inner))),
        H::Slice(Some(items)) => H::Slice(Some(items.into_iter().map(fix_ptr_to_iface).collect())),
        H::Array(items) => H::Array(items.into_iter().map(fix_ptr_to_iface).collect()),
        H::Map(m) => H::Map(GoMap {
            entries: m.entries.map(|e| {
                e.into_iter()
                    .map(|(k, v)| (fix_ptr_to_iface(k), fix_ptr_to_iface(v)))
                    .collect()
            }),
            include_map: m.include_map,
        }),
        H::Struct(mut s) => {
            for f in &mut s.fields {
                f.value = fix_ptr_to_iface(std::mem::replace(&mut f.value, H::Nil));
            }
            H::Struct(s)
        }
        v => v,
    }
}

fn res(r: Result<u64, Error>) -> String {
    match r {
        Ok(h) => format!("{h:x}"),
        // Go panics in these cases; the oracle records "panic:<msg>".
        Err(e @ Error::GoPanic(_)) => format!("panic:{}", hex(e.to_string().as_bytes())),
        Err(e) => format!("err:{}", hex(e.to_string().as_bytes())),
    }
}

fn xx(v: &H, f: impl FnOnce(&mut HashOptions)) -> String {
    let mut o = HashOptions {
        hasher: Some(Box::new(XxHash64::new())),
        ..Default::default()
    };
    f(&mut o);
    res(hash(v, Some(&mut o)))
}

fn all_variants(v: &H) -> Vec<String> {
    vec![
        xx(v, |_| {}),
        res(hash(v, None)),
        xx(v, |o| o.slices_as_sets = true),
        xx(v, |o| o.zero_nil = true),
        xx(v, |o| o.ignore_zero_value = true),
        xx(v, |o| o.use_stringer = true),
        match hash_uint64(&[v.clone(), v.clone()]) {
            Ok(h) => format!("{h:x}"),
            // Go's HashUint64 panics with the error.
            Err(e) => format!("panic:{}", hex(e.to_string().as_bytes())),
        },
    ]
}

/// Go visits map entries in random order, so when several entries fail
/// the reported error is not deterministic: accept any error then.
fn map_order_error(desc: &str, rust: &str, go: &str) -> bool {
    let failed = |s: &str| s.starts_with("err:") || s.starts_with("panic:");
    desc.contains("map(") && failed(go) && failed(rust)
}

#[test]
fn random_values() {
    let zones = zones();
    let text = read_fixture("random.fixture");
    let mut n = 0;
    let mut bad = 0;
    for line in text.lines() {
        let cols: Vec<&str> = line.split('\t').collect();
        let mut p = P {
            s: cols[0].as_bytes(),
            i: 0,
            zones: &zones,
        };
        let v = fix_ptr_to_iface(p.value());
        assert_eq!(p.i, cols[0].len(), "trailing desc: {}", cols[0]);
        let got = all_variants(&v);
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
    eprintln!("random: {n} values x 7 variants, {bad} mismatching");
    assert!(n > 0);
    assert_eq!(bad, 0);
}

#[test]
fn remote_keys() {
    // The checked-in fixture uses a placeholder API key; the scratch
    // fixture (GO_HASH_FIXTURE_DIR) the site's real key, whose 51 keys are
    // exactly the file names of the golden build's getresource cache.
    let name = if std::env::var("GO_HASH_FIXTURE_DIR").is_ok() {
        "remote.fixture"
    } else {
        "remote-public.fixture"
    };
    let text = read_fixture(name);
    let mut n = 0;
    for line in text.lines() {
        let (uri, key) = line.split_once('\t').unwrap();
        // hashing.HashString(uri, map[string]any(nil))
        let got = hash_string(&[
            H::string(uri),
            H::Value(Value::TypedNil("map[string]interface {}".into())),
        ])
        .unwrap();
        assert_eq!(got, key, "{uri}");
        let got2 = hash_string(&[H::string(uri), H::Map(GoMap::nil())]).unwrap();
        assert_eq!(got2, key, "{uri}");
        n += 1;
    }
    assert_eq!(n, 51);
}

// ---------------------------------------------------------------------------
// Named cases (mirror of the oracle's namedCases)

fn foo(name: &str, age: i64) -> GoStruct {
    GoStruct::new("Foo")
        .field("Name", H::string(name))
        .field("Age", H::int(age))
}

fn int_slice(v: &[i64]) -> H {
    H::Slice(Some(v.iter().map(|&i| H::int(i)).collect()))
}

fn stringer_t(v: &str) -> GoStruct {
    GoStruct::new("stringerT").unexported("v", H::string(v))
}

struct HashableX {
    base: u64,
    x: u64,
}
impl Hashable for HashableX {
    fn hash(&self) -> Result<u64, Error> {
        Ok(self.base + self.x)
    }
}
struct HashableErr;
impl Hashable for HashableErr {
    fn hash(&self) -> Result<u64, Error> {
        Err(Error::Custom("boom".into()))
    }
}
struct IgnoreField;
impl Includable for IgnoreField {
    fn hash_include(&self, field: &str, _v: &H) -> Result<bool, Error> {
        Ok(field != "Ignore")
    }
}
struct IgnoreMapKey {
    only_field: Option<&'static str>,
}
impl IncludableMap for IgnoreMapKey {
    fn hash_include_map(&self, field: &str, k: &H, _v: &H) -> Result<bool, Error> {
        if let Some(f) = self.only_field
            && field != f
        {
            return Ok(true);
        }
        Ok(!matches!(k, H::String(s) if s.as_bytes() == b"ignore"))
    }
}

fn hashable_v(x: u64) -> GoStruct {
    GoStruct::new("hashableV")
        .field("X", H::int(x as i64))
        .with_hashable(Receiver::Value, Arc::new(HashableX { base: 500, x }))
}
fn hashable_p(x: u64) -> GoStruct {
    GoStruct::new("hashableP")
        .field("X", H::int(x as i64))
        .with_hashable(Receiver::Pointer, Arc::new(HashableX { base: 700, x }))
}

fn filter(opts_vals: Vec<H>, inner: GoStruct) -> H {
    let f = GoStruct::new("filter")
        .field(
            "Options",
            GoStruct::new("filterOpts")
                .field("Version", H::int(0))
                .field("Vals", H::iface(H::any_slice(opts_vals))),
        )
        .field("Filter", H::iface(H::Struct(inner)));
    H::iface(H::Struct(f))
}

fn named_values() -> Vec<(&'static str, H)> {
    let zones = zones();
    let nested = |ptr: bool| {
        let s = if ptr {
            GoStruct::new("nested")
                .field("Foo", foo("", 0))
                .field("PFoo", H::ptr(H::Struct(foo("b", 2))))
                .field("Any", H::iface(H::ptr(H::Struct(foo("c", 3)))))
                .field("M", H::Map(GoMap::nil()))
        } else {
            GoStruct::new("nested")
                .field("Foo", foo("a", 1))
                .field("PFoo", H::ptr(H::Struct(foo("b", 2))))
                .field("Any", H::iface(H::Struct(foo("c", 3))))
                .field(
                    "M",
                    H::string_any_map([
                        ("x", H::Struct(foo("d", 4))),
                        ("y", H::any_slice(vec![H::int(1), H::string("z")])),
                    ]),
                )
        };
        if ptr {
            H::ptr(H::Struct(s))
        } else {
            H::Struct(s)
        }
    };
    let tags = |ptr: bool| {
        let s = if ptr {
            GoStruct::new("withTags")
                .tagged("A", r#"hash:"ignore""#, H::string("a"))
                .tagged("B", r#"hash:"-""#, H::string(""))
                .tagged("C", r#"hash:"set""#, int_slice(&[3, 2, 1]))
                .field("D", int_slice(&[3, 2, 1]))
                .unexported("e", H::int(0))
                .tagged("F", r#"json:"f" hash:"set""#, H::string(""))
        } else {
            GoStruct::new("withTags")
                .tagged("A", r#"hash:"ignore""#, H::string("a"))
                .tagged("B", r#"hash:"-""#, H::string("b"))
                .tagged("C", r#"hash:"set""#, int_slice(&[1, 2, 3]))
                .field("D", int_slice(&[1, 2, 3]))
                .unexported("e", H::int(5))
                .tagged("F", r#"json:"f" hash:"set""#, H::string("f"))
        };
        if ptr {
            H::ptr(H::Struct(s))
        } else {
            H::Struct(s)
        }
    };
    let blanks = GoStruct::new("blanks")
        .blank(H::int(0))
        .field("A", H::int(1))
        .blank(H::string(""));
    let stringer = GoStruct::new("withStringer")
        .stringer_field("A", r#"hash:"string""#, stringer_t("a"), "S:a")
        .stringer_field("B", "", stringer_t("b"), "S:b")
        .field("C", H::int(1));
    let includable = |recv| {
        GoStruct::new(if recv == Receiver::Value {
            "includable"
        } else {
            "includableP"
        })
        .field("Value", H::string("v"))
        .field("Ignore", H::string("i"))
        .with_includable(recv, Arc::new(IgnoreField))
    };
    // time.Date(y, m, d, 3, 4, 5, 6, loc)
    let date = |y: i64, m: i64, d: i64, loc: Arc<Location>| {
        H::Time(go_time::date(y, go_time::Month(m), d, 3, 4, 5, 6, &loc))
    };
    let params = {
        let mut exif = Map::new(MapType::Params);
        exif.insert("_merge", Value::string("none"));
        exif.insert("disabledate", Value::Bool(false));
        exif.insert("disablelatlong", Value::Bool(false));
        exif.insert("excludefields", Value::string(".*"));
        exif.insert("includefields", Value::string(""));
        let mut m = Map::new(MapType::Params);
        m.insert("_merge", Value::string("none"));
        m.insert("exif", Value::map(exif));
        H::Value(Value::map(m))
    };
    let wm = "/images/watermark_hu_3bf49ff914f6e68c.png_6519743917224815147";
    let overlay = filter(
        vec![H::string(wm), H::int(0), H::int(0)],
        GoStruct::new("overlayFilter")
            .unexported("src", H::Nil)
            .unexported("x", H::int(0))
            .unexported("y", H::int(0)),
    );
    let mut out: Vec<(&'static str, H)> = vec![
        ("foo", H::Struct(foo("x", 3))),
        ("foo-ptr", H::ptr(H::Struct(foo("x", 3)))),
        ("foo-zero", H::Struct(foo("", 0))),
        (
            "foo-nilptr",
            H::Ptr(None, Some(Box::new(H::Struct(foo("", 0))))),
        ),
        ("tags", tags(false)),
        ("tags-ptr", tags(true)),
        ("blanks", H::Struct(blanks.clone())),
        ("blanks-ptr", H::ptr(H::Struct(blanks))),
        ("stringer", H::Struct(stringer.clone())),
        ("stringer-ptr", H::ptr(H::Struct(stringer))),
        (
            "badstringer",
            H::Struct(GoStruct::new("badStringer").tagged("C", r#"hash:"string""#, H::int(1))),
        ),
        ("hashable-v", H::Struct(hashable_v(1))),
        ("hashable-v-ptr", H::ptr(H::Struct(hashable_v(1)))),
        ("hashable-p", H::Struct(hashable_p(1))),
        ("hashable-p-ptr", H::ptr(H::Struct(hashable_p(1)))),
        (
            "hashable-p-slice",
            H::Slice(Some(vec![H::Struct(hashable_p(2))])),
        ),
        (
            "hashable-p-iface-slice",
            H::any_slice(vec![H::Struct(hashable_p(2))]),
        ),
        (
            "hashable-p-map",
            H::Map(GoMap::new(vec![(H::string("a"), H::Struct(hashable_p(3)))])),
        ),
        (
            "hashable-err",
            H::Struct(
                GoStruct::new("hashableErr").with_hashable(Receiver::Value, Arc::new(HashableErr)),
            ),
        ),
        ("includable", H::Struct(includable(Receiver::Value))),
        ("includable-p", H::Struct(includable(Receiver::Pointer))),
        (
            "includable-p-ptr",
            H::ptr(H::Struct(includable(Receiver::Pointer))),
        ),
        (
            "includable-map",
            H::Struct(
                GoStruct::new("includableMap")
                    .field(
                        "Map",
                        H::Map(GoMap::new(vec![
                            (H::string("foo"), H::string("bar")),
                            (H::string("ignore"), H::string("x")),
                        ])),
                    )
                    .with_include_map(Arc::new(IgnoreMapKey {
                        only_field: Some("Map"),
                    })),
            ),
        ),
        (
            "map-incl",
            H::Map(
                GoMap::new(vec![
                    (H::string("foo"), H::string("bar")),
                    (H::string("ignore"), H::string("x")),
                ])
                .with_include_map(Arc::new(IgnoreMapKey { only_field: None })),
            ),
        ),
        (
            "keyer",
            H::Struct(
                GoStruct::new("keyer")
                    .unexported("key", H::string("k"))
                    .with_key("k"),
            ),
        ),
        (
            "func",
            H::Struct(GoStruct::new("withFunc").field(
                "F",
                H::Unsupported {
                    kind: UnsupportedKind::Func,
                    nil: false,
                },
            )),
        ),
        (
            "func-nil",
            H::Struct(GoStruct::new("withFunc").field(
                "F",
                H::Unsupported {
                    kind: UnsupportedKind::Func,
                    nil: true,
                },
            )),
        ),
        (
            "chan",
            H::Struct(GoStruct::new("withChan").field(
                "C",
                H::Unsupported {
                    kind: UnsupportedKind::Chan,
                    nil: false,
                },
            )),
        ),
        (
            "uintptr",
            H::Struct(GoStruct::new("withUintptr").field("P", H::Uint(5, UintKind::Uintptr))),
        ),
        ("nested", nested(false)),
        ("nested-ptr", nested(true)),
        (
            "pptr-nil",
            H::ptr(H::Ptr(None, Some(Box::new(H::Struct(foo("", 0)))))),
        ),
        (
            "ptr-iface-nil",
            H::Ptr(
                Some(Box::new(H::Nil)),
                Some(Box::new(H::Unsupported {
                    kind: UnsupportedKind::Interface,
                    nil: true,
                })),
            ),
        ),
        (
            "anon",
            H::Struct(GoStruct::new("").field("A", H::int(1)).tagged(
                "B",
                r#"hash:"set""#,
                H::string("x"),
            )),
        ),
        ("time-zero", H::Time(Time::zero())),
        ("time-utc", date(2021, 1, 2, go_time::utc())),
        (
            "time-fixed",
            date(2021, 1, 2, go_time::fixed_zone("ICT", 7 * 3600)),
        ),
        (
            "time-fixed-sec",
            date(2021, 1, 2, go_time::fixed_zone("LMT", 3661)),
        ),
        (
            "time-fixed-bad",
            date(2021, 1, 2, go_time::fixed_zone("X", -60)),
        ),
        (
            "time-bangkok",
            date(1900, 1, 2, zones["Asia/Bangkok"].clone()),
        ),
        (
            "time-newyork-2030",
            date(2030, 7, 2, zones["America/New_York"].clone()),
        ),
        ("params", params),
        ("overlay", H::Slice(Some(vec![overlay]))),
    ];
    // Addressability contexts (independent verifier additions).
    let blank_at = |a: i64| {
        GoStruct::new("blanks")
            .blank(H::int(0))
            .field("A", H::int(a))
            .blank(H::string(""))
    };
    let anon = |name: &str, v: H| H::Struct(GoStruct::new("").field(name, v));
    let tagged = |tag: &str, v: H| H::Struct(GoStruct::new("").tagged("T", tag, v));
    let tm = || {
        H::Time(go_time::date(
            2021,
            go_time::Month(1),
            2,
            3,
            4,
            5,
            6,
            &go_time::utc(),
        ))
    };
    let time_zero = H::Time(Time::zero());
    let mut more: Vec<(&'static str, H)> = vec![
        ("hashable-p-array", H::Array(vec![H::Struct(hashable_p(4))])),
        (
            "hashable-p-array-ptr",
            H::ptr(H::Array(vec![H::Struct(hashable_p(4))])),
        ),
        ("hashable-p-field", anon("H", H::Struct(hashable_p(5)))),
        (
            "hashable-p-field-ptr",
            H::ptr(anon("H", H::Struct(hashable_p(5)))),
        ),
        (
            "hashable-p-array-in-slice",
            H::Slice(Some(vec![H::Array(vec![H::Struct(hashable_p(6))])])),
        ),
        (
            "blanks-array",
            H::Array(vec![H::Struct(blank_at(1)), H::Struct(blank_at(2))]),
        ),
        (
            "blanks-array-ptr",
            H::ptr(H::Array(vec![
                H::Struct(blank_at(1)),
                H::Struct(blank_at(2)),
            ])),
        ),
        ("blanks-slice", H::Slice(Some(vec![H::Struct(blank_at(1))]))),
        (
            "blanks-iface-slice",
            H::any_slice(vec![H::Struct(blank_at(1))]),
        ),
        (
            "blanks-map",
            H::Map(GoMap::new(vec![(H::string("k"), H::Struct(blank_at(1)))])),
        ),
        ("blanks-field", anon("B", H::Struct(blank_at(3)))),
        (
            "blanks-field-ptr",
            H::ptr(anon("B", H::Struct(blank_at(3)))),
        ),
        (
            "blanks-iface-field-ptr",
            H::ptr(anon("B", H::iface(H::Struct(blank_at(3))))),
        ),
        (
            "includable-p-slice",
            H::Slice(Some(vec![H::Struct(includable(Receiver::Pointer))])),
        ),
        (
            "includable-p-field",
            anon("I", H::Struct(includable(Receiver::Pointer))),
        ),
        (
            "includable-p-field-ptr",
            H::ptr(anon("I", H::Struct(includable(Receiver::Pointer)))),
        ),
        ("stringer-time-field", tagged(r#"hash:"string""#, tm())),
        (
            "stringer-ptrtime-field",
            tagged(r#"hash:"string""#, H::ptr(tm())),
        ),
        (
            "stringer-ptrtime-nil",
            tagged(
                r#"hash:"string""#,
                H::Ptr(None, Some(Box::new(time_zero.clone()))),
            ),
        ),
        (
            "stringer-any-time",
            tagged(r#"hash:"string""#, H::iface(tm())),
        ),
        (
            "stringer-any-int",
            tagged(r#"hash:"string""#, H::iface(H::int(1))),
        ),
        (
            "ignorezero-mixed",
            H::Struct(
                GoStruct::new("")
                    .field("A", H::int(0))
                    .field("B", H::Ptr(None, Some(Box::new(H::int(0)))))
                    .field("C", H::Slice(Some(vec![])))
                    .field("D", H::Map(GoMap::new(vec![])))
                    .field("E", H::Nil)
                    .field("F", H::Struct(blank_at(0)))
                    .field("G", H::Array(vec![H::float64(-0.0), H::float64(0.0)]))
                    .field("H", time_zero)
                    .field("I", H::string(""))
                    .field("J", H::Array(vec![])),
            ),
        ),
    ];
    out.append(&mut more);
    out
}

#[test]
fn named() {
    let text = read_fixture("named.fixture");
    let want: HashMap<&str, &str> = text.lines().filter_map(|l| l.split_once('\t')).collect();
    let mut n = 0;
    for (name, v) in named_values() {
        let got = all_variants(&v);
        for (i, g) in got.iter().enumerate() {
            let key = format!("{name}#{i}");
            let w = want
                .get(key.as_str())
                .unwrap_or_else(|| panic!("missing {key}"));
            assert_eq!(g, w, "{key}: {v:?}");
            n += 1;
        }
    }
    let wm = "/images/watermark_hu_3bf49ff914f6e68c.png_6519743917224815147";
    let overlay = |x: H, y: H| {
        filter(
            vec![H::string(wm), x, y],
            GoStruct::new("overlayFilter")
                .unexported("src", H::Nil)
                .unexported("x", H::int(0))
                .unexported("y", H::int(0)),
        )
    };
    let plain = [
        (
            "overlay-key",
            hash_string(&[H::Slice(Some(vec![overlay(H::int(0), H::int(0))]))]).unwrap(),
        ),
        (
            "overlay-key-float",
            hash_string(&[H::Slice(Some(vec![overlay(H::float64(0.0), H::int(1))]))]).unwrap(),
        ),
        (
            "opacity-key",
            hash_string(&[H::Slice(Some(vec![filter(
                vec![H::float64(0.5)],
                GoStruct::new("opacityFilter").unexported("opacity", H::float32(0.5)),
            )]))])
            .unwrap(),
        ),
        (
            "process-key",
            hash_string(&[H::Slice(Some(vec![filter(
                vec![H::string("resize 100x100")],
                GoStruct::new("processFilter").unexported("spec", H::string("resize 100x100")),
            )]))])
            .unwrap(),
        ),
        (
            "resize-key",
            hash_string_hex(&[H::string_slice(["resize", "600x480", "webp"])]).unwrap(),
        ),
        (
            "hashstring-ab",
            hash_string(&[H::string("a"), H::string("b")]).unwrap(),
        ),
        (
            "hashstring-keyer",
            hash_string(&[
                H::string("a"),
                H::string("b"),
                H::Struct(GoStruct::new("keyer").with_key("c")),
            ])
            .unwrap(),
        ),
        ("hashstring-none", hash_string(&[]).unwrap()),
        ("hashstring-nil", hash_string(&[H::Nil]).unwrap()),
        (
            "hashstring-nilmap",
            hash_string(&[H::string("u"), H::Map(GoMap::nil())]).unwrap(),
        ),
        ("xx-hex", hashing::xxhash_from_string_hex_encoded("hello")),
        ("md5", hashing::md5_from_string_hex_encoded("hello")),
        ("reader-strings", {
            let (h, n) = hashing::xxhash_from_reader(
                &mut &b"hello world"[..],
                hashing::ReaderKind::WriterTo,
            )
            .unwrap();
            format!("{h}:{n}")
        }),
        ("reader-file", {
            let (h, n) = hashing::xxhash_from_reader(
                &mut &b"hello world"[..],
                hashing::ReaderKind::ReadFrom,
            )
            .unwrap();
            format!("{h}:{n}")
        }),
    ];
    for (name, got) in plain {
        assert_eq!(Some(&got.as_str()), want.get(name), "{name}");
        n += 1;
    }
    eprintln!("named: {n} results checked");
}
