//! Shared helpers of the nh-tplfuncs oracle tests: fixture loading, the typed JSON value
//! decoder/encoder of `tools/go-oracle/nh-tplfuncs/data` (goval's format plus pages, page
//! groups, `types.KeyValues`, `*maps.Scratch` and the `*main.tstObj` test struct), fake pages
//! rebuilt from the recorded page table, and a test `config.AllProvider`.

#![allow(dead_code)]

use std::any::Any;
use std::borrow::Cow;
use std::collections::HashMap;
use std::io::Read;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;

use go_time::GoTimeExt;
use go_value::{
    FloatKind, GoString, HostCtx, IntKind, Kind, List, Map, MapType, Object, SafeKind, Time,
    UintKind, Value,
};
use nh_common::hreflect::{map_type_from_name, slice_type_from_name};
use nh_common::object::{GoResult, args};
use nh_common::paths::pathparser::{Path, PathParser};
use nh_common::urls::BaseURL;
use nh_config::common_config::{BaseConfig, CommonDirs, Pagination, SitemapConfig};
use nh_config::config_provider::{AllProvider, ContentTypesProvider};
use nh_helpers::source::file_info::File;
use nh_langs::language::{Language, Languages};
use nh_media::media::media_type::MediaType;
use nh_page::page::{Page, PageRef, Pages};
use nh_page::related::{IndexConfig, Keyword};
use nh_page::site::SiteRef;
use nh_resource::resourcetypes::{Resource, Resources};
use serde_json::{Value as J, json};

// ---------------------------------------------------------------------------
// Fixtures

pub fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
}

/// Reads a gzip-compressed JSON fixture below `tests/fixtures`.
pub fn fixture(rel: &str) -> J {
    let path = fixtures_dir().join(rel);
    let f = std::fs::File::open(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    let mut s = String::new();
    flate2::read::GzDecoder::new(f)
        .read_to_string(&mut s)
        .unwrap();
    serde_json::from_str(&s).unwrap()
}

/// The fixture files of a topic directory, sorted.
pub fn fixture_files(dir: &str) -> Vec<String> {
    let mut v: Vec<String> = std::fs::read_dir(fixtures_dir().join(dir))
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .filter(|n| n.ends_with(".json.gz"))
        .collect();
    v.sort();
    v
}

/// A Go string from the fixture: a JSON string or `{"hex": ...}`.
pub fn gostr(v: &J) -> GoString {
    match v {
        J::String(s) => GoString::from(s.as_str()),
        J::Object(m) if m.contains_key("hex") => GoString::from(unhex(m["hex"].as_str().unwrap())),
        _ => panic!("not a Go string: {v}"),
    }
}

pub fn unhex(s: &str) -> Vec<u8> {
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap())
        .collect()
}

/// A string as the oracle encodes it.
pub fn str_enc(b: &[u8]) -> J {
    match std::str::from_utf8(b) {
        Ok(s) => J::String(s.to_string()),
        Err(_) => json!({"hex": b.iter().map(|c| format!("{c:02x}")).collect::<String>()}),
    }
}

fn fbits(s: &str) -> f64 {
    f64::from_bits(u64::from_str_radix(s, 16).unwrap())
}

fn fbits_enc(f: f64) -> String {
    format!("{:016x}", f.to_bits())
}

fn int_kind(t: &str) -> Option<IntKind> {
    Some(match t {
        "int" => IntKind::Int,
        "int8" => IntKind::Int8,
        "int16" => IntKind::Int16,
        "int32" => IntKind::Int32,
        "int64" => IntKind::Int64,
        _ => return None,
    })
}

fn uint_kind(t: &str) -> Option<UintKind> {
    Some(match t {
        "uint" => UintKind::Uint,
        "uint8" => UintKind::Uint8,
        "uint16" => UintKind::Uint16,
        "uint32" => UintKind::Uint32,
        "uint64" => UintKind::Uint64,
        "uintptr" => UintKind::Uintptr,
        _ => return None,
    })
}

fn safe_kind(t: &str) -> Option<SafeKind> {
    Some(match t {
        "template.HTML" => SafeKind::Html,
        "template.HTMLAttr" => SafeKind::HtmlAttr,
        "template.CSS" => SafeKind::Css,
        "template.JS" => SafeKind::Js,
        "template.JSStr" => SafeKind::JsStr,
        "template.URL" => SafeKind::Url,
        "template.Srcset" => SafeKind::Srcset,
        _ => return None,
    })
}

fn location(name: &str, abbr: &str, off: i64) -> Arc<go_value::Location> {
    if name == "UTC" && abbr == "UTC" && off == 0 {
        go_time::utc()
    } else {
        assert_eq!(name, abbr, "only fixed zones are decoded");
        go_time::fixed_zone(abbr, off)
    }
}

pub fn decode_time(v: &J) -> Time {
    let loc = location(
        v["loc"].as_str().unwrap(),
        v["abbr"].as_str().unwrap(),
        v["off"].as_i64().unwrap(),
    );
    go_time::unix(v["unix"].as_i64().unwrap(), v["nsec"].as_i64().unwrap()).in_loc(&loc)
}

// ---------------------------------------------------------------------------
// Test objects

/// A named basic Go type without a Rust counterpart (encoded "named").
pub struct NamedBasic {
    pub name: String,
    pub under: Value,
}

nh_common::go_methods!(NamedBasic {});

impl Object for NamedBasic {
    fn type_name(&self) -> Cow<'_, str> {
        Cow::Borrowed(&self.name)
    }
    fn has_method(&self, name: &str) -> bool {
        Self::go_has_method(name)
    }
    fn call_method(&self, ctx: HostCtx<'_>, name: &str, a: &[Value]) -> Option<GoResult<Value>> {
        self.go_call_method(ctx, name, a)
    }
    fn kind(&self) -> Kind {
        Kind::Struct
    }
    fn underlying(&self) -> Option<Value> {
        Some(self.under.clone())
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// The oracle's `*main.tstObj`: fields ID, A, B, P; methods GetA, Double, Fail, WithArg.
pub struct TstObj {
    pub id: String,
    pub a: String,
    pub b: i64,
    pub p: Value,
}

nh_common::go_methods!(TstObj {
    "Double" => |t, _c, a| { args::exactly(a, 0, "Double")?; Ok(Value::int(2 * t.b)) },
    "Fail" => |_t, _c, a| { args::exactly(a, 0, "Fail")?; Err(go_value::Error::new("boom")) },
    "GetA" => |t, _c, a| { args::exactly(a, 0, "GetA")?; Ok(Value::string(t.a.as_str())) },
    "WithArg" => |t, _c, a| {
        args::exactly(a, 1, "WithArg")?;
        let s = args::string(a, 0)?;
        Ok(Value::string(format!("{}{}", s.to_str_lossy(), t.a)))
    },
});

impl Object for TstObj {
    nh_common::object_basics!("*main.tstObj");

    fn field(&self, name: &str) -> Option<Value> {
        match name {
            "ID" => Some(Value::string(self.id.as_str())),
            "A" => Some(Value::string(self.a.as_str())),
            "B" => Some(Value::int(self.b)),
            "P" => Some(self.p.clone()),
            _ => None,
        }
    }

    fn struct_fields(&self) -> Option<Vec<(Cow<'_, str>, Value)>> {
        Some(vec![
            (Cow::Borrowed("ID"), Value::string(self.id.as_str())),
            (Cow::Borrowed("A"), Value::string(self.a.as_str())),
            (Cow::Borrowed("B"), Value::int(self.b)),
            (Cow::Borrowed("P"), self.p.clone()),
        ])
    }
}

fn tst_obj(id: &str) -> Value {
    static REG: OnceLock<Mutex<HashMap<String, Value>>> = OnceLock::new();
    let reg = REG.get_or_init(|| Mutex::new(HashMap::new()));
    let mut reg = reg.lock().unwrap();
    reg.entry(id.to_string())
        .or_insert_with(|| {
            let params = |k: &str, v: &str| {
                let mut m = Map::new(MapType::Params);
                m.insert(k, Value::string(v));
                Value::map(m)
            };
            let (a, b, p) = match id {
                "o1" => ("x", 1, params("k", "v1")),
                "o2" => ("y", 2, params("k", "v2")),
                "o3" => ("x", 3, Value::TypedNil(Arc::from("maps.Params"))),
                _ => panic!("unknown test object {id}"),
            };
            Value::object(TstObj {
                id: id.to_string(),
                a: a.to_string(),
                b,
                p,
            })
        })
        .clone()
}

// ---------------------------------------------------------------------------
// Fake pages

/// A recorded page value (a `*hugolib.pageState` or one of its wrappers).
pub struct FakePage {
    pub entry: usize,
    pub id: u64,
    pub type_name: String,
    pub weight0: Option<i64>,
    pub ordinal: Option<i64>,
    pub kind: String,
    pub title: String,
    pub link_title: String,
    pub weight: i64,
    pub date: Time,
    pub lastmod: Time,
    pub publish_date: Time,
    pub section: String,
    pub page_type: String,
    pub lang: String,
    pub path: String,
    pub is_page: bool,
    pub is_node: bool,
    pub is_home: bool,
    pub is_section: bool,
    pub name: String,
    pub rel_permalink: String,
    pub params: Arc<Map>,
    /// The unwrapped `*hugolib.pageState` (for wrappers).
    pub unwrapped: OnceLock<PageRef>,
}

impl FakePage {
    fn is_wrapper(&self) -> bool {
        self.type_name != "*hugolib.pageState"
    }
}

impl Resource for FakePage {
    fn resource_type(&self) -> String {
        "page".into()
    }
    fn media_type(&self) -> MediaType {
        unimplemented!()
    }
    fn permalink(&self) -> String {
        unimplemented!()
    }
    fn rel_permalink(&self) -> String {
        self.rel_permalink.clone()
    }
    fn data(&self) -> Value {
        unimplemented!()
    }
    fn name(&self) -> String {
        self.name.clone()
    }
    fn title(&self) -> String {
        self.title.clone()
    }
    fn params(&self) -> Arc<Map> {
        self.params.clone()
    }
    fn key(&self) -> String {
        unimplemented!()
    }
    fn tpl_type_name(&self) -> Cow<'_, str> {
        Cow::Borrowed(&self.type_name)
    }
    fn tpl_has_method(&self, name: &str) -> bool {
        match name {
            "Weight0" => self.weight0.is_some(),
            "Ordinal" => self.ordinal.is_some(),
            "Unwrapv" => self.type_name == "hugolib.pageWithWeight0",
            _ => matches!(
                name,
                "Section"
                    | "Kind"
                    | "Type"
                    | "Weight"
                    | "Title"
                    | "LinkTitle"
                    | "Lang"
                    | "Path"
                    | "Name"
                    | "RelPermalink"
                    | "IsPage"
                    | "IsNode"
                    | "IsHome"
                    | "IsSection"
                    | "Date"
                    | "Lastmod"
                    | "PublishDate"
                    | "Params"
                    | "Eq"
                    | "Slice"
                    | "Group"
            ),
        }
    }
    fn tpl_call_method(
        &self,
        _ctx: HostCtx<'_>,
        name: &str,
        a: &[Value],
    ) -> Option<GoResult<Value>> {
        if !self.tpl_has_method(name) {
            return None;
        }
        let s = |v: &str| Value::string(v);
        let zero_args = |a: &[Value]| args::exactly(a, 0, name);
        Some((|| {
            Ok(match name {
                "Section" => {
                    zero_args(a)?;
                    s(&self.section)
                }
                "Kind" => {
                    zero_args(a)?;
                    s(&self.kind)
                }
                "Type" => {
                    zero_args(a)?;
                    s(&self.page_type)
                }
                "Weight" => {
                    zero_args(a)?;
                    Value::int(self.weight)
                }
                "Weight0" => {
                    zero_args(a)?;
                    Value::int(self.weight0.unwrap())
                }
                "Ordinal" => {
                    zero_args(a)?;
                    Value::int(self.ordinal.unwrap())
                }
                "Title" => {
                    zero_args(a)?;
                    s(&self.title)
                }
                "LinkTitle" => {
                    zero_args(a)?;
                    s(&self.link_title)
                }
                "Lang" => {
                    zero_args(a)?;
                    s(&self.lang)
                }
                "Path" => {
                    zero_args(a)?;
                    s(&self.path)
                }
                "Name" => {
                    zero_args(a)?;
                    s(&self.name)
                }
                "RelPermalink" => {
                    zero_args(a)?;
                    s(&self.rel_permalink)
                }
                "IsPage" => {
                    zero_args(a)?;
                    Value::Bool(self.is_page)
                }
                "IsNode" => {
                    zero_args(a)?;
                    Value::Bool(self.is_node)
                }
                "IsHome" => {
                    zero_args(a)?;
                    Value::Bool(self.is_home)
                }
                "IsSection" => {
                    zero_args(a)?;
                    Value::Bool(self.is_section)
                }
                "Date" => {
                    zero_args(a)?;
                    Value::Time(self.date.clone())
                }
                "Lastmod" => {
                    zero_args(a)?;
                    Value::Time(self.lastmod.clone())
                }
                "PublishDate" => {
                    zero_args(a)?;
                    Value::Time(self.publish_date.clone())
                }
                "Params" => {
                    zero_args(a)?;
                    Value::Map(self.params.clone())
                }
                "Unwrapv" => {
                    zero_args(a)?;
                    self.unwrapped.get().unwrap().to_value()
                }
                // Go: hugolib/page.go:Eq (unwrapPage + pointer equality).
                "Eq" => {
                    args::exactly(a, 1, name)?;
                    Value::Bool(match nh_page::page::page_from_value(&a[0]) {
                        Some(p) => p.0.page_id() == self.id,
                        None => false,
                    })
                }
                // Go: hugolib/collections.go:Slice (page.ToPages).
                "Slice" => {
                    args::exactly(a, 1, name)?;
                    let ps = nh_page::page::pages_from_value(&a[0])?;
                    nh_page::page::pages_to_value(&ps)
                }
                // Go: hugolib/collections.go:Group
                "Group" => {
                    args::exactly(a, 2, name)?;
                    nh_page::pages::group(&a[0], &a[1])?.to_value()
                }
                _ => unreachable!(),
            })
        })())
    }
    fn to_value(self: Arc<Self>) -> Value {
        PageRef(self).to_value()
    }
    fn as_any_arc(self: Arc<Self>) -> Arc<dyn Any + Send + Sync> {
        self
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl Page for FakePage {
    fn page_id(&self) -> u64 {
        self.id
    }
    fn unwrap_page(self: Arc<Self>) -> Arc<dyn Page> {
        if self.is_wrapper() {
            return self.unwrapped.get().unwrap().0.clone();
        }
        self
    }
    fn weight0(&self) -> Option<i64> {
        self.weight0
    }
    fn ordinal(&self) -> Option<i64> {
        self.ordinal
    }
    fn kind(&self) -> String {
        self.kind.clone()
    }
    fn title(&self) -> String {
        self.title.clone()
    }
    fn link_title(&self) -> String {
        self.link_title.clone()
    }
    fn description(&self) -> String {
        unimplemented!()
    }
    fn weight(&self) -> i64 {
        self.weight
    }
    fn date(&self) -> Time {
        self.date.clone()
    }
    fn lastmod(&self) -> Time {
        self.lastmod.clone()
    }
    fn publish_date(&self) -> Time {
        self.publish_date.clone()
    }
    fn expiry_date(&self) -> Time {
        unimplemented!()
    }
    fn is_home(&self) -> bool {
        self.is_home
    }
    fn is_node(&self) -> bool {
        self.is_node
    }
    fn is_page(&self) -> bool {
        self.is_page
    }
    fn is_section(&self) -> bool {
        self.is_section
    }
    fn section(&self) -> String {
        self.section.clone()
    }
    fn page_type(&self) -> String {
        self.page_type.clone()
    }
    fn layout(&self) -> String {
        unimplemented!()
    }
    fn lang(&self) -> String {
        self.lang.clone()
    }
    fn path(&self) -> String {
        self.path.clone()
    }
    fn path_info(&self) -> Arc<Path> {
        unimplemented!()
    }
    fn slug(&self) -> String {
        unimplemented!()
    }
    fn draft(&self) -> bool {
        unimplemented!()
    }
    fn aliases(&self) -> Vec<String> {
        unimplemented!()
    }
    fn keywords(&self) -> Vec<String> {
        unimplemented!()
    }
    fn bundle_type(&self) -> String {
        unimplemented!()
    }
    fn sitemap(&self) -> SitemapConfig {
        unimplemented!()
    }
    fn param(&self, _key: &Value) -> nh_common::Result<Value> {
        unimplemented!()
    }
    fn page_params(&self) -> Arc<Map> {
        self.params.clone()
    }
    fn site(&self) -> SiteRef {
        unimplemented!()
    }
    fn file(&self) -> Option<Arc<File>> {
        unimplemented!()
    }
    fn parent(&self) -> Option<PageRef> {
        unimplemented!()
    }
    fn pages(&self) -> Pages {
        unimplemented!()
    }
    fn regular_pages(&self) -> Pages {
        unimplemented!()
    }
    fn resources(&self) -> Resources {
        unimplemented!()
    }
    fn output_formats(&self) -> nh_page::page_outputformat::OutputFormats {
        unimplemented!()
    }
    fn all_translations(&self) -> Pages {
        unimplemented!()
    }
    fn translations(&self) -> Pages {
        unimplemented!()
    }
    fn plain(&self, _ctx: HostCtx<'_>) -> nh_common::Result<GoString> {
        unimplemented!()
    }
    fn content_len(&self, _ctx: HostCtx<'_>) -> nh_common::Result<i64> {
        unimplemented!()
    }
    fn render_string(&self, _ctx: HostCtx<'_>, _args: &[Value]) -> nh_common::Result<Value> {
        unimplemented!()
    }
    fn related_keywords(&self, _cfg: &IndexConfig) -> nh_common::Result<Vec<Keyword>> {
        unimplemented!()
    }
    fn ref_(&self, _args: &Map) -> nh_common::Result<String> {
        unimplemented!()
    }
    fn rel_ref(&self, _args: &Map) -> nh_common::Result<String> {
        unimplemented!()
    }
}

fn s(v: &J) -> String {
    gostr(v).to_str_lossy().into_owned()
}

/// Rebuilds the recorded page table (wrappers point at the entry of their pageState).
pub fn load_pages(entries: &[J]) -> Vec<Arc<FakePage>> {
    nh_page::page::init();
    let fakes: Vec<Arc<FakePage>> = entries
        .iter()
        .enumerate()
        .map(|(i, e)| {
            let params = match decode_value(&e["params"], &[]) {
                Value::Map(m) => m,
                other => panic!("params: {other:?}"),
            };
            Arc::new(FakePage {
                entry: i,
                id: 1 + e["pid"].as_u64().unwrap(),
                type_name: e["type"].as_str().unwrap().to_string(),
                weight0: e.get("weight0").map(|v| v.as_i64().unwrap()),
                ordinal: e.get("ordinal").map(|v| v.as_i64().unwrap()),
                kind: s(&e["kind"]),
                title: s(&e["title"]),
                link_title: s(&e["linkTitle"]),
                weight: e["weight"].as_i64().unwrap(),
                date: decode_time(&e["date"]),
                lastmod: decode_time(&e["lastmod"]),
                publish_date: decode_time(&e["publishDate"]),
                section: s(&e["section"]),
                page_type: s(&e["pageType"]),
                lang: e["lang"].as_str().unwrap().to_string(),
                path: s(&e["path"]),
                is_page: e["isPage"].as_bool().unwrap(),
                is_node: e["isNode"].as_bool().unwrap(),
                is_home: e["isHome"].as_bool().unwrap(),
                is_section: e["isSection"].as_bool().unwrap(),
                name: s(&e["name"]),
                rel_permalink: s(&e["relPermalink"]),
                params,
                unwrapped: OnceLock::new(),
            })
        })
        .collect();
    for f in &fakes {
        if f.is_wrapper() {
            let base = fakes
                .iter()
                .find(|g| g.id == f.id && !g.is_wrapper())
                .expect("the wrapped page is recorded");
            let _ = f.unwrapped.set(PageRef(base.clone()));
        }
    }
    fakes
}

// ---------------------------------------------------------------------------
// Values

/// Decodes a value of the oracle's typed JSON.
pub fn decode_value(v: &J, pages: &[Arc<FakePage>]) -> Value {
    let t = v["t"].as_str().unwrap();
    if let Some(ty) = t.strip_prefix("nil:") {
        return Value::TypedNil(Arc::from(ty));
    }
    if let Some(i) = v.get("page") {
        return PageRef(pages[i.as_u64().unwrap() as usize].clone()).to_value();
    }
    if let Some(k) = int_kind(t) {
        return Value::Int(v["v"].as_str().unwrap().parse().unwrap(), k);
    }
    if let Some(k) = uint_kind(t) {
        return Value::Uint(v["v"].as_str().unwrap().parse().unwrap(), k);
    }
    if let Some(k) = safe_kind(t) {
        return Value::Safe(k, gostr(&v["s"]));
    }
    match t {
        "nil" => Value::Invalid,
        "bool" => Value::Bool(v["v"].as_bool().unwrap()),
        "float64" => Value::Float(fbits(v["v"].as_str().unwrap()), FloatKind::F64),
        "float32" => Value::Float(fbits(v["v"].as_str().unwrap()), FloatKind::F32),
        "string" => Value::String(gostr(&v["s"])),
        "time.Time" => Value::Time(decode_time(v)),
        "*main.tstObj" => tst_obj(v["id"].as_str().unwrap()),
        "named" => {
            let name = v["name"].as_str().unwrap();
            let under = decode_value(&v["under"], pages);
            match name {
                "hstring.HTML" => Value::object(nh_common::types::hstring::Html(
                    under.as_go_string().unwrap().clone(),
                )),
                "json.Number" => {
                    Value::object(go_json::Number(under.as_go_string().unwrap().clone()))
                }
                "neohugo.VersionString" => {
                    Value::object(nh_config::neohugo::version::VersionString(
                        under.as_go_string().unwrap().to_str_lossy().into_owned(),
                    ))
                }
                _ => Value::object(NamedBasic {
                    name: name.to_string(),
                    under,
                }),
            }
        }
        _ if v.get("items").is_some() => Value::List(Arc::new(List::new(
            slice_type_from_name(t),
            v["items"]
                .as_array()
                .unwrap()
                .iter()
                .map(|x| decode_value(x, pages))
                .collect(),
        ))),
        _ if v.get("entries").is_some() => {
            let mut m = Map::new(map_type_from_name(t));
            for e in v["entries"].as_array().unwrap() {
                m.insert(gostr(&e[0]), decode_value(&e[1], pages));
            }
            Value::map(m)
        }
        _ => panic!("cannot decode {v}"),
    }
}

/// Encodes a value like the oracle's encoder.
pub fn encode_value(v: &Value) -> J {
    match v {
        Value::Invalid => json!({"t": "nil"}),
        Value::TypedNil(t) => json!({"t": format!("nil:{t}")}),
        Value::Bool(b) => json!({"t": "bool", "v": b}),
        Value::Int(i, k) => json!({"t": k.go_name(), "v": i.to_string()}),
        Value::Uint(u, k) => json!({"t": k.go_name(), "v": u.to_string()}),
        Value::Float(f, FloatKind::F64) => json!({"t": "float64", "v": fbits_enc(*f)}),
        Value::Float(f, FloatKind::F32) => json!({"t": "float32", "v": fbits_enc(*f)}),
        Value::String(s) => json!({"t": "string", "s": str_enc(s)}),
        Value::Safe(k, s) => json!({"t": k.go_name(), "s": str_enc(s)}),
        Value::Time(t) => {
            let (abbr, off) = t.zone();
            json!({
                "t": "time.Time", "unix": t.go_unix(), "nsec": t.nanosecond(),
                "loc": go_time::location_string(t.loc.as_ref()), "abbr": abbr, "off": off,
            })
        }
        Value::List(l) => {
            json!({"t": l.ty.go_name(), "items": l.items.iter().map(encode_value).collect::<Vec<_>>()})
        }
        Value::Map(m) => json!({
            "t": m.ty.go_name(),
            "entries": m.entries.iter().map(|(k, v)| json!([str_enc(k), encode_value(v)])).collect::<Vec<_>>(),
        }),
        Value::Object(o) => {
            if let Some(p) = v.downcast::<PageRef>() {
                let f =
                    p.0.as_any()
                        .downcast_ref::<FakePage>()
                        .expect("a fake page");
                return json!({"t": f.type_name, "page": f.entry});
            }
            if let Some(g) = v.downcast::<nh_page::pagegroup::PageGroup>() {
                return json!({"t": "page.PageGroup", "key": encode_value(&g.key),
                    "pages": encode_value(&nh_page::page::pages_to_value(&g.pages))});
            }
            if let Some(t) = v.downcast::<TstObj>() {
                return json!({"t": "*main.tstObj", "id": t.id});
            }
            match &*o.type_name() {
                "types.KeyValues" => {
                    return json!({"t": "types.KeyValues", "key": encode_value(&o.field("Key").unwrap()),
                        "values": encode_value(&o.field("Values").unwrap())});
                }
                "*maps.Scratch" => return json!({"t": "*maps.Scratch"}),
                _ => {}
            }
            if let Some(u) = o.underlying() {
                return json!({"t": "named", "name": o.type_name(), "under": encode_value(&u)});
            }
            panic!("cannot encode {v:?}")
        }
    }
}

// ---------------------------------------------------------------------------
// Config

/// A `config.AllProvider` with only a language (the namespaces read `Language()`).
pub struct TestCfg {
    pub language: Arc<Language>,
}

impl TestCfg {
    pub fn new(lang: &str) -> Arc<TestCfg> {
        Arc::new(TestCfg {
            language: Language::new(lang, lang, "", Default::default()).unwrap(),
        })
    }
}

impl AllProvider for TestCfg {
    fn language(&self) -> Arc<Language> {
        self.language.clone()
    }
    fn languages(&self) -> Languages {
        vec![self.language.clone()]
    }
    fn languages_default_first(&self) -> Languages {
        vec![self.language.clone()]
    }
    fn language_prefix(&self) -> String {
        String::new()
    }
    fn base_url(&self) -> BaseURL {
        unimplemented!()
    }
    fn base_url_live_reload(&self) -> BaseURL {
        unimplemented!()
    }
    fn path_parser(&self) -> Arc<PathParser> {
        unimplemented!()
    }
    fn environment(&self) -> String {
        "production".to_string()
    }
    fn is_multihost(&self) -> bool {
        false
    }
    fn is_multilingual(&self) -> bool {
        false
    }
    fn no_build_lock(&self) -> bool {
        true
    }
    fn base_config(&self) -> BaseConfig {
        unimplemented!()
    }
    fn dirs(&self) -> CommonDirs {
        unimplemented!()
    }
    fn quiet(&self) -> bool {
        true
    }
    fn dirs_base(&self) -> CommonDirs {
        unimplemented!()
    }
    fn content_types(&self) -> Arc<dyn ContentTypesProvider> {
        unimplemented!()
    }
    fn get_config_section(&self, name: &str) -> Arc<dyn Any + Send + Sync> {
        unimplemented!("config section {name}")
    }
    fn get_config(&self) -> Arc<dyn Any + Send + Sync> {
        unimplemented!()
    }
    fn canonify_urls(&self) -> bool {
        false
    }
    fn disable_path_to_lower(&self) -> bool {
        false
    }
    fn remove_path_accents(&self) -> bool {
        false
    }
    fn is_ugly_urls(&self, _section: &str) -> bool {
        false
    }
    fn default_content_language(&self) -> String {
        self.language.lang.clone()
    }
    fn default_content_language_in_subdir(&self) -> bool {
        false
    }
    fn is_lang_disabled(&self, _lang: &str) -> bool {
        false
    }
    fn summary_length(&self) -> i64 {
        70
    }
    fn pagination(&self) -> Pagination {
        unimplemented!()
    }
    fn build_expired(&self) -> bool {
        false
    }
    fn build_future(&self) -> bool {
        false
    }
    fn build_drafts(&self) -> bool {
        false
    }
    fn running(&self) -> bool {
        false
    }
    fn watching(&self) -> bool {
        false
    }
    fn fast_render_mode(&self) -> bool {
        false
    }
    fn print_unused_templates(&self) -> bool {
        false
    }
    fn enable_missing_translation_placeholders(&self) -> bool {
        false
    }
    fn template_metrics(&self) -> bool {
        false
    }
    fn template_metrics_hints(&self) -> bool {
        false
    }
    fn print_i18n_warnings(&self) -> bool {
        false
    }
    fn create_title(&self, s: &str) -> String {
        s.to_string()
    }
    fn ignore_file(&self, _s: &str) -> bool {
        false
    }
    fn new_content_editor(&self) -> String {
        String::new()
    }
    fn timeout(&self) -> Duration {
        Duration::from_secs(30)
    }
    fn static_dirs(&self) -> Vec<String> {
        Vec::new()
    }
    fn ignored_logs(&self) -> std::collections::BTreeSet<String> {
        Default::default()
    }
    fn working_dir(&self) -> String {
        String::new()
    }
    fn enable_emoji(&self) -> bool {
        false
    }
}
