//! Oracle test: `resource.Resources` (Get, GetMatch, Match, ByType, Mount, MergeByLanguage),
//! `NewCachedResourceGetter`, `Param`, `GetParam`/`GetParamToLower` against
//! `tools/go-oracle/nh-resource/resources` (fixtures/resources/resources.json.gz), plus the
//! hand-ported tests of resources/internal (Go's internal package rule keeps the oracle out).

use std::any::Any;
use std::borrow::Cow;
use std::io::Read;
use std::sync::{Arc, Once};

use go_time::GoTimeExt;
use go_value::{
    FloatKind, GoString, HostCtx, IntKind, List, Map, MapType, SafeKind, UintKind, Value,
};
use nh_common::hreflect::{map_type_from_name, slice_type_from_name};
use nh_common::object::GoResult;
use nh_media::media::media_type::MediaType;
use nh_resource::internal::key::ResourceTransformationKey;
use nh_resource::internal::resourcepaths::ResourcePaths;
use nh_resource::resourcetypes::{Resource, ResourceRef, Resources, resources_to_value};
use serde_json::Value as J;

fn fixture() -> J {
    let path = format!(
        "{}/tests/fixtures/resources/resources.json.gz",
        env!("CARGO_MANIFEST_DIR")
    );
    let f = std::fs::File::open(&path).unwrap();
    let mut s = String::new();
    flate2::read::GzDecoder::new(f)
        .read_to_string(&mut s)
        .unwrap();
    serde_json::from_str(&s).unwrap()
}

struct TestRes {
    name: String,
    normalized: Option<String>,
    typ: String,
    key: Option<String>,
    params: Arc<Map>,
}

impl Resource for TestRes {
    fn resource_type(&self) -> String {
        self.typ.clone()
    }
    fn media_type(&self) -> MediaType {
        MediaType::default()
    }
    fn permalink(&self) -> String {
        String::new()
    }
    fn rel_permalink(&self) -> String {
        String::new()
    }
    fn data(&self) -> Value {
        Value::Invalid
    }
    fn name(&self) -> String {
        self.name.clone()
    }
    fn title(&self) -> String {
        String::new()
    }
    fn params(&self) -> Arc<Map> {
        self.params.clone()
    }
    fn key(&self) -> String {
        self.name.clone()
    }
    fn name_normalized(&self) -> Option<String> {
        self.normalized.clone()
    }
    fn translation_key(&self) -> Option<String> {
        self.key.clone()
    }
    fn tpl_type_name(&self) -> Cow<'_, str> {
        Cow::Borrowed("main.testResource")
    }
    fn tpl_has_method(&self, _name: &str) -> bool {
        false
    }
    fn tpl_call_method(
        &self,
        _ctx: HostCtx<'_>,
        _name: &str,
        _args: &[Value],
    ) -> Option<GoResult<Value>> {
        None
    }
    fn to_value(self: Arc<Self>) -> Value {
        Value::object(ResourceRef(self))
    }
    fn as_any_arc(self: Arc<Self>) -> Arc<dyn Any + Send + Sync> {
        self
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

fn build(set: &J) -> Resources {
    set.as_array()
        .unwrap()
        .iter()
        .map(|r| {
            let s = |k: &str| r.get(k).and_then(|v| v.as_str()).map(|v| v.to_string());
            let plain = r.get("plain").and_then(|v| v.as_bool()).unwrap_or(false);
            let res: Arc<dyn Resource> = Arc::new(TestRes {
                name: s("name").unwrap(),
                normalized: if plain {
                    None
                } else {
                    Some(s("normalized").unwrap_or_default())
                },
                typ: s("type").unwrap(),
                key: s("key"),
                params: Arc::new(Map::new(MapType::Params)),
            });
            res
        })
        .collect()
}

fn gostr(v: &J) -> Vec<u8> {
    match v {
        J::String(s) => s.as_bytes().to_vec(),
        J::Object(m) if m.contains_key("hex") => {
            let h = m["hex"].as_str().unwrap();
            (0..h.len())
                .step_by(2)
                .map(|i| u8::from_str_radix(&h[i..i + 2], 16).unwrap())
                .collect()
        }
        _ => panic!("{v}"),
    }
}

/// Decodes `goval.Encode` values.
fn decode(v: &J) -> Value {
    let t = v["t"].as_str().unwrap();
    if let Some(ty) = t.strip_prefix("nil:") {
        return Value::TypedNil(Arc::from(ty));
    }
    let int = |k: IntKind| Value::Int(v["v"].as_str().unwrap().parse().unwrap(), k);
    let fl = |k: FloatKind| {
        Value::Float(
            f64::from_bits(u64::from_str_radix(v["v"].as_str().unwrap(), 16).unwrap()),
            k,
        )
    };
    match t {
        "nil" => Value::Invalid,
        "bool" => Value::Bool(v["v"].as_bool().unwrap()),
        "int" => int(IntKind::Int),
        "int8" => int(IntKind::Int8),
        "int32" => int(IntKind::Int32),
        "int64" => int(IntKind::Int64),
        "uint" => Value::Uint(v["v"].as_str().unwrap().parse().unwrap(), UintKind::Uint),
        "uint64" => Value::Uint(v["v"].as_str().unwrap().parse().unwrap(), UintKind::Uint64),
        "float64" => fl(FloatKind::F64),
        "float32" => fl(FloatKind::F32),
        "string" => Value::String(GoString::from(gostr(&v["s"]))),
        "template.HTML" => Value::Safe(SafeKind::Html, GoString::from(gostr(&v["s"]))),
        "time.Time" => Value::Time(
            go_time::unix(v["unix"].as_i64().unwrap(), v["nsec"].as_i64().unwrap())
                .in_loc(&go_time::utc()),
        ),
        _ if v.get("items").is_some() => Value::List(Arc::new(List::new(
            slice_type_from_name(t),
            v["items"].as_array().unwrap().iter().map(decode).collect(),
        ))),
        _ if v.get("entries").is_some() => {
            let mut m = Map::new(map_type_from_name(t));
            for e in v["entries"].as_array().unwrap() {
                m.insert(GoString::from(gostr(&e[0])), decode(&e[1]));
            }
            Value::map(m)
        }
        _ => panic!("cannot decode {v}"),
    }
}

fn enc_str(b: &[u8]) -> J {
    match std::str::from_utf8(b) {
        Ok(s) => J::String(s.to_string()),
        Err(_) => {
            serde_json::json!({ "hex": b.iter().map(|c| format!("{c:02x}")).collect::<String>() })
        }
    }
}

/// Encodes like `goval.Encode`.
fn encode(v: &Value) -> J {
    match v {
        Value::Invalid => serde_json::json!({"t": "nil"}),
        Value::TypedNil(t) => serde_json::json!({"t": format!("nil:{t}")}),
        Value::Bool(b) => serde_json::json!({"t": "bool", "v": b}),
        Value::Int(i, k) => serde_json::json!({"t": k.go_name(), "v": i.to_string()}),
        Value::Uint(i, k) => serde_json::json!({"t": k.go_name(), "v": i.to_string()}),
        Value::Float(f, k) => {
            serde_json::json!({"t": match k { FloatKind::F64 => "float64", FloatKind::F32 => "float32" }, "v": format!("{:016x}", f.to_bits())})
        }
        Value::String(s) => serde_json::json!({"t": "string", "s": enc_str(s.as_bytes())}),
        Value::Safe(_, s) => serde_json::json!({"t": v.go_type_name(), "s": enc_str(s.as_bytes())}),
        Value::Time(t) => {
            let (abbr, off) = t.zone();
            serde_json::json!({"t": "time.Time", "unix": t.go_unix(), "nsec": t.nanosecond(), "loc": t.go_location().name, "abbr": abbr, "off": off})
        }
        Value::List(l) => {
            serde_json::json!({"t": v.go_type_name(), "items": l.items.iter().map(encode).collect::<Vec<_>>()})
        }
        Value::Map(m) => {
            serde_json::json!({"t": v.go_type_name(), "entries": m.entries.iter().map(|(k, v)| serde_json::json!([enc_str(k.as_bytes()), encode(v)])).collect::<Vec<_>>()})
        }
        Value::Object(o) => serde_json::json!({"t": o.type_name()}),
    }
}

fn name_of(r: Option<Arc<dyn Resource>>) -> J {
    match r {
        Some(r) => J::String(r.name()),
        None => J::Null,
    }
}

fn names(r: Option<Resources>) -> J {
    match r {
        Some(r) => J::Array(r.iter().map(|x| J::String(x.name())).collect()),
        None => J::Null,
    }
}

fn ok(v: nh_common::Result<J>) -> J {
    match v {
        Ok(v) => serde_json::json!({ "ok": v }),
        Err(e) => serde_json::json!({ "panic": e.message() }),
    }
}

fn init() {
    static ONCE: Once = Once::new();
    ONCE.call_once(nh_resource::resourcetypes::register);
}

#[test]
fn resources_match_go() {
    init();
    use nh_resource::resources as rs;
    let fx = fixture();
    let sets: Vec<Resources> = fx["sets"].as_array().unwrap().iter().map(build).collect();
    let mut fails = Vec::new();
    let mut n = 0;
    for c in fx["cases"].as_array().unwrap() {
        let set = || &sets[c["set"].as_u64().unwrap() as usize];
        let arg = || decode(&c["arg"]);
        let got = match c["fn"].as_str().unwrap() {
            "Get" => ok(rs::try_get(Some(set()), &arg()).map(name_of)),
            "GetNil" => ok(rs::try_get(None, &arg()).map(name_of)),
            "GetMatch" => ok(rs::try_get_match(set(), &arg()).map(name_of)),
            "Match" => ok(rs::try_match(set(), &arg()).map(names)),
            "ByType" => ok(rs::try_by_type(set(), &arg()).map(names)),
            "Mount" => {
                use nh_resource::resources::ResourceGetter;
                let m = rs::mount(
                    set(),
                    c["base"].as_str().unwrap(),
                    c["target"].as_str().unwrap(),
                );
                ok(m.try_get(&arg()).map(name_of))
            }
            "MergeByLanguage" => {
                let r2 = &sets[c["set2"].as_u64().unwrap() as usize];
                serde_json::json!({ "ok": names(Some(rs::merge_by_language(set(), r2))) })
            }
            "Cached" => {
                use nh_resource::resources::ResourceGetter;
                let g = rs::new_cached_resource_getter(&[
                    resources_to_value(&sets[3]),
                    resources_to_value(&sets[2]),
                ]);
                ok(g.try_get(&arg()).map(name_of))
            }
            "Param" | "ParamFallback" => {
                let params = match decode(&c["params"]) {
                    Value::Map(m) => m,
                    _ => panic!(),
                };
                let fb = c.get("fallback").map(|f| match decode(f) {
                    Value::Map(m) => m,
                    _ => panic!(),
                });
                match nh_resource::params::param(&params, fb.as_deref(), &decode(&c["key"])) {
                    Ok(v) => serde_json::json!({ "ok": encode(&v) }),
                    Err(e) => serde_json::json!({ "err": e.message() }),
                }
            }
            f @ ("GetParam" | "GetParamToLower") => {
                let params = match decode(&c["params"]) {
                    Value::Map(m) => m,
                    _ => panic!(),
                };
                let r: Arc<dyn Resource> = Arc::new(TestRes {
                    name: String::new(),
                    normalized: None,
                    typ: String::new(),
                    key: None,
                    params,
                });
                let key = c["key"]["s"].as_str().unwrap();
                let v = if f == "GetParam" {
                    nh_resource::resource_helpers::get_param_of(r.as_ref(), key)
                } else {
                    nh_resource::resource_helpers::get_param_to_lower(r.as_ref(), key)
                };
                serde_json::json!({ "ok": encode(&v) })
            }
            f => panic!("{f}"),
        };
        n += 1;
        if got != c["want"] {
            fails.push(format!("{}: got {got}, want {}", c, c["want"]));
        }
    }
    eprintln!("resources: {n} cases, {} failures", fails.len());
    for f in fails.iter().take(30) {
        eprintln!("  {f}");
    }
    assert!(fails.is_empty());
}

/// The template call path of `resource.Resources` (the named-type methods).
#[test]
fn resources_template_methods() {
    init();
    let fx = fixture();
    let set = build(&fx["sets"][3]);
    let v = resources_to_value(&set);
    let call = |name: &str, args: &[Value]| {
        nh_resource::resources::resources_call_method(&(), &v, name, args).expect("method")
    };
    let r = call("Get", &[Value::string("logo")]).unwrap();
    assert_eq!(r.downcast::<ResourceRef>().unwrap().0.name(), "Logo");
    let r = call("Get", &[Value::string("nope")]).unwrap();
    assert!(matches!(r, Value::TypedNil(t) if &*t == "resource.Resource"));
    let r = call("Match", &[Value::string("*.nothing")]).unwrap();
    assert!(matches!(r, Value::TypedNil(t) if &*t == "resource.Resources"));
    let r = call("Match", &[Value::string("images/*")]).unwrap();
    assert_eq!(r.as_list().unwrap().items.len(), 1); // "Logo" matches only through its normalized name
    let r = call("ByType", &[Value::string("image")]).unwrap();
    assert_eq!(r.as_list().unwrap().items.len(), 3);
    let e = call("Get", &[Value::string_list(["x"])]).unwrap_err();
    assert_eq!(
        e.message(),
        "unable to cast []string{\"x\"} of type []string to string"
    );
    let e = call("GetMatch", &[Value::string("[")]).unwrap_err();
    assert!(!e.message().is_empty());
    assert!(call("Get", &[]).is_err());
    assert!(nh_resource::resources::resources_call_method(&(), &v, "Nope", &[]).is_none());
    let m = call("Mount", &[Value::string("images"), Value::string("img")]).unwrap();
    let got = m
        .as_object()
        .unwrap()
        .call_method(&(), "Get", &[Value::string("img/Sunset.JPG")])
        .unwrap()
        .unwrap();
    assert_eq!(
        got.downcast::<ResourceRef>().unwrap().0.name(),
        "images/Sunset.JPG"
    );
}

/// Go: resources/internal/key_test.go (the struct elements hash through go-hashstructure; the
/// Go test's struct cannot be built here, so the elements are the basic values it holds).
#[test]
fn resource_transformation_key() {
    let k = ResourceTransformationKey::new("testing", Vec::new());
    assert_eq!(k.value(), "testing");
    let k = ResourceTransformationKey::new("testing", vec![Value::string("a")]);
    assert_eq!(
        k.value(),
        format!(
            "testing_{}",
            nh_common::hashing::hash_string(&[Value::string("a")])
        )
    );
}

/// Go: resources/internal/resourcepaths.go, by hand (the oracle cannot import it).
#[test]
fn resource_paths() {
    let d = ResourcePaths {
        dir: "/a".into(),
        base_dir_target: "/th".into(),
        base_dir_link: "sub/".into(),
        target_base_paths: Vec::new(),
        file: "x.png".into(),
    };
    assert_eq!(d.target_link(), "/sub//a/x.png");
    assert_eq!(d.target_path(), "/th/a/x.png");
    assert_eq!(d.path(), "/a/x.png");
    assert_eq!(d.target_paths(), vec!["/th/a/x.png"]);
    let d2 = ResourcePaths {
        target_base_paths: vec!["/en".into(), "/th".into()],
        ..d.clone()
    };
    assert_eq!(d2.target_paths(), vec!["/en/th/a/x.png", "/th/th/a/x.png"]);
    assert_eq!(d2.target_filenames(), d2.target_paths());
    // join: empty parts skipped, "/" added between parts and at the start.
    let e = ResourcePaths {
        dir: "a".into(),
        file: "/y.css".into(),
        ..Default::default()
    };
    assert_eq!(e.path(), "/a/y.css");
    assert_eq!(e.target_link(), "/a/y.css");
    let z = ResourcePaths::default();
    assert_eq!(z.path(), "/");
    let f = d2.from_target_path("/a/b/c.png");
    assert_eq!(
        (
            f.dir.as_str(),
            f.file.as_str(),
            f.base_dir_link.as_str(),
            f.base_dir_target.as_str()
        ),
        ("/a/b", "c.png", "", "")
    );
    assert_eq!(f.target_base_paths, d2.target_base_paths);
    let f = d.from_target_path("a.png");
    assert_eq!((f.dir.as_str(), f.file.as_str()), ("", "a.png"));
    let f = d.from_target_path("/a.png");
    assert_eq!((f.dir.as_str(), f.file.as_str()), ("", "a.png"));
    let f = d.from_target_path("a/b/");
    assert_eq!((f.dir.as_str(), f.file.as_str()), ("/a/b", ""));
    let f = d.from_target_path("");
    assert_eq!((f.dir.as_str(), f.file.as_str()), ("", ""));
    let n = nh_resource::internal::resourcepaths::new_resource_paths("/a/b.css", "/bt", "/bl", &[]);
    assert_eq!(n.target_path(), "/bt/a/b.css");
    assert_eq!(n.target_link(), "/bl/a/b.css");
}

#[test]
fn dates_and_stale() {
    use nh_resource::dates::{Dates, is_expired, is_future, is_zero_dates};
    let d = Dates::default();
    assert!(is_zero_dates(&d));
    assert!(!is_future(&d));
    assert!(!is_expired(&d));
    let past = go_time::unix(1_000_000_000, 0);
    let future = go_time::unix(40_000_000_000, 0);
    let d = Dates {
        publish_date: future.clone(),
        expiry_date: past.clone(),
        ..Dates::default()
    };
    assert!(is_future(&d));
    assert!(is_expired(&d));
    assert!(!is_zero_dates(&d));
    let d = Dates {
        publish_date: past,
        expiry_date: future,
        ..Dates::default()
    };
    assert!(!is_future(&d));
    assert!(!is_expired(&d));

    let r: Arc<dyn Resource> = Arc::new(TestRes {
        name: "n".into(),
        normalized: Some("norm".into()),
        typ: "t".into(),
        key: None,
        params: Arc::new(Map::new(MapType::Params)),
    });
    assert_eq!(nh_resource::resourcetypes::stale_version(r.as_ref()), 0);
    assert_eq!(
        nh_resource::resourcetypes::stale_version_sum(&[r.as_ref()]),
        0
    );
    assert_eq!(
        nh_resource::resourcetypes::name_normalized_or_name(r.as_ref()),
        "norm"
    );
    let e = nh_resource::resourcetypes::ResourceError::new(
        &nh_common::herrors::Error::new("boom"),
        None,
    );
    assert_eq!(e.to_string(), "boom");
    assert!(matches!(e.data(), Value::Map(m) if m.is_empty() && m.ty == MapType::StringAny));
}
