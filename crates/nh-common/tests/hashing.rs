//! `common/hashing`: the Go test table (common/hashing/hashing_test.go), the worked vectors of the
//! seeksnack build (specs/images.md §4.5, architecture-core.md §2.3) and the differential test
//! against `tools/go-oracle/nh-common/hashing` (from an arm64 oracle build).

mod support;

use std::any::Any;
use std::borrow::Cow;
use std::io::Read;

use go_hashstructure::{GoStruct, HashValue as H};
use go_value::{GoString, HostCtx, Kind, Map, MapType, Object, Value};
use nh_common::hashing::{self, ReaderKind};
use nh_common::maps::params::{self, ParamsMergeStrategy};
use serde_json::{Value as J, json};
use support::*;

/// The oracle's `main.keyed`: a `Key() string` provider.
struct Keyed(String);

impl Object for Keyed {
    fn type_name(&self) -> Cow<'_, str> {
        Cow::Borrowed("main.keyed")
    }
    fn kind(&self) -> Kind {
        Kind::Struct
    }
    fn has_method(&self, name: &str) -> bool {
        name == "Key"
    }
    fn call_method(
        &self,
        _: HostCtx<'_>,
        name: &str,
        _: &[Value],
    ) -> Option<go_value::Result<Value>> {
        (name == "Key").then(|| Ok(Value::string(self.0.as_str())))
    }
    fn hash_key(&self) -> Option<GoString> {
        Some(GoString::from(self.0.as_str()))
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

fn dec(v: &J) -> Value {
    if v["t"] == "main.keyed" {
        return Value::object(Keyed(v["id"].as_str().unwrap().to_string()));
    }
    match v.get("items") {
        Some(items) if v["t"] == "[]interface {}" => {
            Value::any_list(items.as_array().unwrap().iter().map(dec).collect())
        }
        _ => decode(v),
    }
}

// Go: common/hashing/hashing_test.go:TestXxHashFromReader
#[test]
fn go_test_xxhash_from_reader() {
    let s = "Hello World";
    let (got, size) = hashing::xxhash_from_reader(&mut s.as_bytes(), ReaderKind::WriterTo).unwrap();
    assert_eq!(size, s.len() as i64);
    assert_eq!(got, 7148569436472236994);
}

// Go: common/hashing/hashing_test.go:TestXxHashFromReaderPara
#[test]
fn go_test_xxhash_from_reader_para() {
    let handles: Vec<_> = (0..10)
        .map(|i| {
            std::thread::spawn(move || {
                for j in 0..100 {
                    let s = "Hello ".repeat(i + j + 42);
                    let (got, size) =
                        hashing::xxhash_from_reader(&mut s.as_bytes(), ReaderKind::WriterTo)
                            .unwrap();
                    assert_eq!(size, s.len() as i64);
                    assert_eq!(got, hashing::xxhash_from_string(s.as_bytes()));
                }
            })
        })
        .collect();
    for h in handles {
        h.join().unwrap();
    }
}

// Go: common/hashing/hashing_test.go:TestXxHashFromString
#[test]
fn go_test_xxhash_from_string() {
    assert_eq!(
        hashing::xxhash_from_string(b"Hello World"),
        7148569436472236994
    );
}

// Go: common/hashing/hashing_test.go:TestXxHashFromStringHexEncoded
#[test]
fn go_test_xxhash_from_string_hex_encoded() {
    assert_eq!(
        hashing::xxhash_from_string_hex_encoded(b"The quick brown fox jumps over the lazy dog"),
        "0b242d361fda71bc"
    );
}

// Go: common/hashing/hashing_test.go:TestHashString
#[test]
fn go_test_hash_string() {
    assert_eq!(
        hashing::hash_string(&[Value::string("a"), Value::string("b")]),
        "3176555414984061461"
    );
    assert_eq!(
        hashing::hash_string(&[Value::string("ab")]),
        "7347350983217793633"
    );
    let vals = [
        Value::string("a"),
        Value::string("b"),
        Value::object(Keyed("c".into())),
    ];
    assert_eq!(hashing::hash_string(&vals), "4438730547989914315");
}

/// The imaging config as neohugo's config loader prepares it: `_merge` keys hold
/// `ParamsMergeStrategy` values (specs/images.md §4.3).
fn imaging_config() -> Map {
    let mut exif = Map::new(MapType::StringAny);
    exif.insert("_merge", Value::string("none"));
    exif.insert("disableDate", Value::Bool(false));
    exif.insert("disableLatLong", Value::Bool(false));
    exif.insert("excludeFields", Value::string(".*"));
    exif.insert("includeFields", Value::string(""));
    let mut imaging = Map::new(MapType::StringAny);
    imaging.insert("_merge", Value::string("none"));
    imaging.insert("exif", Value::map(exif));
    let v = Value::map(imaging);
    params::to_params_and_prepare(&v).unwrap()
}

// specs/images.md §4.5 and architecture-core.md §2.3 (verified against the golden build).
#[test]
fn seeksnack_vectors() {
    let cfg_params = imaging_config();
    assert_eq!(
        params::get_merge_strategy(&cfg_params),
        (ParamsMergeStrategy::None, true)
    );
    let cfg = hashing::hash_string_hex(&[Value::map(cfg_params)]);
    assert_eq!(cfg, "4bf645f71319dd1d", "imagingCfgSourceHash");

    let key = |opts: &[&str]| hashing::hash_string_hex(&[Value::string_list(opts.iter().copied())]);
    assert_eq!(key(&["resize", "600x480"]), "7bfd4638d4eb3be2");
    assert_eq!(key(&["resize", "300x240"]), "a08a22b9e9d91e29");
    assert_eq!(key(&["resize", "600x480", "webp"]), "590f9512b18cac1e");
    assert_eq!(key(&["resize", "640x480", "webp"]), "73816495dd661eee");

    // resources/image.go: the target name hash of a processed image.
    let target = |incoming: &str, src: u64, conf_key: &str| {
        hashing::hash_string_hex(&[
            Value::string(incoming),
            Value::Uint(src, go_value::UintKind::Uint64),
            Value::string(conf_key),
            Value::string(cfg.as_str()),
        ])
    };
    // xxhash64(watermark.png): the input file is a seeksnack asset (private); the value appears
    // in the watermark's Key() and so in the filter key below.
    let wm_hash: u64 = 6519743917224815147;
    let wm_hu = target("", wm_hash, &key(&["resize", "600x480"]));
    assert_eq!(wm_hu, "3bf49ff914f6e68c");
    let wm_key = format!("/images/watermark_hu_{wm_hu}.png_{wm_hash}");
    assert_eq!(
        wm_key,
        "/images/watermark_hu_3bf49ff914f6e68c.png_6519743917224815147"
    );

    // images.Filter(images.Overlay(wm, 0, 0)): HashString([]gift.Filter{images.filter{...}}).
    let f = GoStruct::new("filter")
        .field(
            "Options",
            GoStruct::new("filterOpts")
                .field("Version", H::int(0))
                .field(
                    "Vals",
                    H::iface(H::any_slice(vec![
                        H::string(wm_key.as_str()),
                        H::int(0),
                        H::int(0),
                    ])),
                ),
        )
        .field(
            "Filter",
            H::iface(H::Struct(
                GoStruct::new("overlayFilter")
                    .unexported("src", H::Nil)
                    .unexported("x", H::int(0))
                    .unexported("y", H::int(0)),
            )),
        );
    let fk = hashing::hash_uint64_values(&[H::Slice(Some(vec![H::iface(H::Struct(f))]))])
        .unwrap()
        .to_string();
    assert_eq!(
        fk, "1682858112077426900",
        "filterKey(Overlay wm600x480, 0, 0)"
    );

    let src: u64 = 0x41b9c8214be9d5d7;
    let a = target("", src, &key(&["resize", "600x480"]));
    let b = target(&a, src, &fk);
    let c = target(&b, src, &key(&["resize", "600x480", "webp"]));
    assert_eq!(
        (a.as_str(), b.as_str(), c.as_str()),
        ("143d7e1f185771c7", "c8f2bc05dec496d8", "1e78ebf3348c6618")
    );

    // GetRemote cache key: HashString(uri, map[string]any(nil)). The seeksnack key
    // (iIsZs0m-BVU -> 5844198154546968338) needs the site's private API key; the same code path
    // is checked on the 51 public-key URIs of the go-hashstructure fixture below.
    let uri = "https://www.googleapis.com/youtube/v3/videos?key=API_KEY&part=snippet,contentDetails,statistics&id=-1sYhonMIt8";
    assert_eq!(
        hashing::hash_string(&[
            Value::string(uri),
            Value::TypedNil("map[string]interface {}".into())
        ]),
        "1293619673859574247"
    );
}

/// The 51 GetRemote cache keys with a placeholder API key (go-hashstructure's public fixture).
#[test]
fn remote_keys() {
    let path = format!(
        "{}/../go-hashstructure/tests/fixtures/remote-public.fixture.gz",
        env!("CARGO_MANIFEST_DIR")
    );
    let raw = std::fs::read(&path).unwrap();
    let mut text = String::new();
    flate2::read::GzDecoder::new(&raw[..])
        .read_to_string(&mut text)
        .unwrap();
    let mut n = 0;
    for line in text.lines() {
        let (uri, key) = line.split_once('\t').unwrap();
        let got = hashing::hash_string(&[
            Value::string(uri),
            Value::TypedNil("map[string]interface {}".into()),
        ]);
        assert_eq!(got, key, "{uri}");
        n += 1;
    }
    assert_eq!(n, 51);
}

#[test]
fn oracle() {
    init();
    let fx = fixture("hashing/hashing.json.gz");
    assert_eq!(
        fx["goarch"], "arm64",
        "the fixture must come from an arm64 oracle build"
    );
    let mut n = 0;
    let mut bad = Vec::new();
    let mut chk = |what: String, want: &J, got: J| {
        n += 1;
        if !same_result(want, &got) {
            bad.push(format!("{what}: want {want} got {got}"));
        }
    };
    let s = |x: String| json!({"ok": {"t": "string", "s": x}});
    let r = |res: nh_common::Result<String>| match res {
        Ok(x) => s(x),
        Err(e) => json!({"err": e.message()}),
    };
    for c in fx["cases"].as_array().unwrap() {
        match c["op"].as_str().unwrap() {
            "hash" => {
                let vs: Vec<Value> = c["vs"]
                    .as_array()
                    .map(|a| a.iter().map(dec).collect())
                    .unwrap_or_default();
                let what = format!("{vs:?}");
                chk(
                    format!("{what} HashString"),
                    &c["string"],
                    r(hashing::try_hash_string(&vs)),
                );
                chk(
                    format!("{what} HashStringHex"),
                    &c["stringHex"],
                    r(hashing::try_hash_string_hex(&vs)),
                );
                let u = hashing::try_hash_uint64(&vs)
                    .map(|h| json!({"ok": encode(&Value::Uint(h, go_value::UintKind::Uint64))}));
                chk(
                    format!("{what} HashUint64"),
                    &c["uint64"],
                    u.unwrap_or_else(|e| json!({"err": e.message()})),
                );
                let h = hashing::hash(&vs)
                    .map(|h| json!({"ok": encode(&Value::Uint(h, go_value::UintKind::Uint64))}));
                chk(
                    format!("{what} Hash"),
                    &c["hash"],
                    h.unwrap_or_else(|e| json!({"err": e.message()})),
                );
            }
            "string" => {
                let b = bytes(&c["s"]);
                let what = format!("{:?}", String::from_utf8_lossy(&b));
                let xx = hashing::xxhash_from_string(&b);
                chk(
                    format!("{what} xxhash"),
                    &c["xxhash"],
                    json!({"ok": encode(&Value::Uint(xx, go_value::UintKind::Uint64))}),
                );
                chk(
                    format!("{what} xxhashHex"),
                    &json!({"ok": c["xxhashHex"]}),
                    json!({"ok": hashing::xxhash_from_string_hex_encoded(&b)}),
                );
                chk(
                    format!("{what} md5"),
                    &json!({"ok": c["md5"]}),
                    json!({"ok": hashing::md5_from_string_hex_encoded(&b)}),
                );
                if let Some(hs) = c.get("hashString") {
                    chk(
                        format!("{what} HashString"),
                        &json!({"ok": hs}),
                        json!({"ok": hashing::hash_string(&[Value::String(GoString::from(b.clone()))])}),
                    );
                    for (field, kind) in [
                        ("reader", ReaderKind::WriterTo),
                        ("onlyReader", ReaderKind::ReadFrom),
                    ] {
                        let (h, size) = hashing::xxhash_from_reader(&mut &b[..], kind).unwrap();
                        chk(format!("{what} {field}"), &c[field], ok_pair(h, size));
                    }
                    chk(
                        format!("{what} readerHex"),
                        &c["readerHex"],
                        s(hashing::xxhash_from_reader_hex_encoded(&mut &b[..]).unwrap()),
                    );
                }
            }
            "long" => {
                let n_bytes = c["n"].as_u64().unwrap() as usize;
                let b: Vec<u8> = "Hello ".repeat(n_bytes / 6 + 1).into_bytes()[..n_bytes].to_vec();
                let xx = hashing::xxhash_from_string(&b);
                chk(
                    format!("long {n_bytes} xxhash"),
                    &c["xxhash"],
                    json!({"ok": encode(&Value::Uint(xx, go_value::UintKind::Uint64))}),
                );
                for (field, kind) in [
                    ("reader", ReaderKind::WriterTo),
                    ("onlyReader", ReaderKind::ReadFrom),
                ] {
                    let (h, size) = hashing::xxhash_from_reader(&mut &b[..], kind).unwrap();
                    chk(
                        format!("long {n_bytes} {field}"),
                        &c[field],
                        ok_pair(h, size),
                    );
                }
            }
            other => panic!("unknown op {other}"),
        }
    }
    eprintln!("hashing: {n} checks, {} mismatches", bad.len());
    for b in bad.iter().take(20) {
        eprintln!("{b}");
    }
    assert!(n > 10000, "too few checks: {n}");
    assert!(bad.is_empty());
}

/// Go's `[]any{uint64, int64}` result encoding.
fn ok_pair(h: u64, size: i64) -> J {
    json!({"ok": encode(&Value::any_list(vec![
        Value::Uint(h, go_value::UintKind::Uint64),
        Value::Int(size, go_value::IntKind::Int64),
    ]))})
}
