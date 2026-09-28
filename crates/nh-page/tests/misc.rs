//! Oracle test for the smaller parts of resources/page and pagemeta against
//! `tools/go-oracle/nh-page/misc` (fixtures/misc/misc.json.gz): DecodeCascadeConfig,
//! PageMatcher.Matches, NewOutputFormat, OutputFormats.Get, MarkupToMediaType,
//! PageConfig.Init/Compile and NamedPageMetaValue (on the nop page).

mod support;

use std::any::Any;
use std::borrow::Cow;
use std::sync::{Arc, Mutex};

use go_value::{GoString, HostCtx, Map, MapType, Time, Value};
use nh_common::Result;
use nh_common::loggers::{Level, Logger, Options};
use nh_common::object::GoResult;
use nh_common::paths::pathparser::Path;
use nh_config::common_config::SitemapConfig;
use nh_config::neohugo::neohugo::HugoInfoConfig;
use nh_helpers::source::file_info::File;
use nh_media::media::media_type::MediaType;
use nh_page::page::{Page, PageRef, Pages, named_page_meta_value};
use nh_page::page_matcher::{PageMatcher, decode_cascade_config_with_logger};
use nh_page::page_outputformat::{OutputFormat, output_formats_get};
use nh_page::pagemeta::page_frontmatter::{PageConfig, markup_to_media_type};
use nh_page::related::{IndexConfig, Keyword};
use nh_page::site::{SiteRef, new_dummy_hugo_site};
use nh_resource::resourcetypes::{Resource, Resources};
use serde_json::Value as J;
use support::*;

struct Env(String);

impl HugoInfoConfig for Env {
    fn environment(&self) -> String {
        self.0.clone()
    }
    fn running(&self) -> bool {
        false
    }
    fn working_dir(&self) -> String {
        String::new()
    }
    fn is_multihost(&self) -> bool {
        false
    }
    fn is_multilingual(&self) -> bool {
        false
    }
}

/// A page with what `PageMatcher.Matches` reads; its site is a `DummySite` with the
/// environment.
struct MatchPage {
    kind: String,
    lang: String,
    path: String,
    site: SiteRef,
}

impl Resource for MatchPage {
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
        unimplemented!()
    }
    fn data(&self) -> Value {
        unimplemented!()
    }
    fn name(&self) -> String {
        unimplemented!()
    }
    fn title(&self) -> String {
        unimplemented!()
    }
    fn params(&self) -> Arc<Map> {
        unimplemented!()
    }
    fn key(&self) -> String {
        unimplemented!()
    }
    fn tpl_type_name(&self) -> Cow<'_, str> {
        Cow::Borrowed("main.testPage")
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
        PageRef(self).to_value()
    }
    fn as_any_arc(self: Arc<Self>) -> Arc<dyn Any + Send + Sync> {
        self
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl Page for MatchPage {
    fn page_id(&self) -> u64 {
        1
    }
    fn unwrap_page(self: Arc<Self>) -> Arc<dyn Page> {
        self
    }
    fn kind(&self) -> String {
        self.kind.clone()
    }
    fn title(&self) -> String {
        unimplemented!()
    }
    fn link_title(&self) -> String {
        unimplemented!()
    }
    fn description(&self) -> String {
        unimplemented!()
    }
    fn weight(&self) -> i64 {
        unimplemented!()
    }
    fn date(&self) -> Time {
        unimplemented!()
    }
    fn lastmod(&self) -> Time {
        unimplemented!()
    }
    fn publish_date(&self) -> Time {
        unimplemented!()
    }
    fn expiry_date(&self) -> Time {
        unimplemented!()
    }
    fn is_home(&self) -> bool {
        unimplemented!()
    }
    fn is_node(&self) -> bool {
        unimplemented!()
    }
    fn is_page(&self) -> bool {
        unimplemented!()
    }
    fn is_section(&self) -> bool {
        unimplemented!()
    }
    fn section(&self) -> String {
        unimplemented!()
    }
    fn page_type(&self) -> String {
        unimplemented!()
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
    fn param(&self, _key: &Value) -> Result<Value> {
        unimplemented!()
    }
    fn page_params(&self) -> Arc<Map> {
        unimplemented!()
    }
    fn site(&self) -> SiteRef {
        self.site.clone()
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
    fn plain(&self, _ctx: HostCtx<'_>) -> Result<GoString> {
        unimplemented!()
    }
    fn content_len(&self, _ctx: HostCtx<'_>) -> Result<i64> {
        unimplemented!()
    }
    fn render_string(&self, _ctx: HostCtx<'_>, _args: &[Value]) -> Result<Value> {
        unimplemented!()
    }
    fn related_keywords(&self, _cfg: &IndexConfig) -> Result<Vec<Keyword>> {
        unimplemented!()
    }
    fn ref_(&self, _args: &Map) -> Result<String> {
        unimplemented!()
    }
    fn rel_ref(&self, _args: &Map) -> Result<String> {
        unimplemented!()
    }
}

fn matcher(v: &J) -> PageMatcher {
    let s = |k: &str| v[k].as_str().unwrap().to_string();
    PageMatcher {
        path: s("path"),
        kind: s("kind"),
        lang: s("lang"),
        environment: s("environment"),
    }
}

fn matcher_json(m: &PageMatcher) -> J {
    serde_json::json!({"path": m.path, "kind": m.kind, "lang": m.lang, "environment": m.environment})
}

fn store_logger() -> Logger {
    #[derive(Default)]
    struct Discard;
    impl std::io::Write for Discard {
        fn write(&mut self, b: &[u8]) -> std::io::Result<usize> {
            Ok(b.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let sink: nh_common::loggers::LogSink = Arc::new(Mutex::new(Discard));
    Logger::with_options(Options {
        level: Level::Warn,
        std_out: Some(sink.clone()),
        std_err: Some(sink),
        store_errors: true,
        ..Default::default()
    })
}

fn mt_json(m: &MediaType) -> J {
    serde_json::json!({
        "type": m.typ, "mainType": m.main_type, "subType": m.sub_type, "delimiter": m.delimiter,
        "suffix": m.first_suffix.suffix, "fullSuffix": m.first_suffix.full_suffix,
        "mimeSuffix": m.mime_suffix(), "suffixesCSV": m.suffixes_csv,
    })
}

fn check(fails: &mut Vec<String>, c: &J, got: J, want: &J) {
    if &got != want {
        fails.push(format!("{c}\n   got {got}"));
    }
}

#[test]
fn misc_matches_go() {
    nh_page::page::init();
    let fx = fixture("misc/misc.json.gz");
    let mut fails = Vec::new();
    let mut n = 0;
    for c in fx["cases"].as_array().unwrap() {
        n += 1;
        match c["fn"].as_str().unwrap() {
            "cascade" => {
                let logger = store_logger();
                let input = decode(&c["in"]);
                let got = match decode_cascade_config_with_logger(Some(&logger), true, &input) {
                    Ok(ns) => {
                        let entries: Vec<J> = ns
                            .config
                            .keys()
                            .iter()
                            .map(|k| {
                                let v = ns.config.get(k).unwrap();
                                serde_json::json!({
                                    "target": matcher_json(k),
                                    "params": encode(&Value::map(v.params.clone())),
                                    "fields": encode(&Value::map(v.fields.clone())),
                                })
                            })
                            .collect();
                        let entries = if entries.is_empty() {
                            J::Null
                        } else {
                            J::Array(entries)
                        };
                        serde_json::json!({"ok": {"entries": entries, "sourceHash": ns.source_hash}})
                    }
                    // Go panics on a kind glob that does not compile; the port returns Go's
                    // panic text as an error.
                    Err(e) if c["want"].get("panic").is_some() => {
                        serde_json::json!({"panic": e.message()})
                    }
                    Err(e) => serde_json::json!({"err": e.message()}),
                };
                check(&mut fails, c, got, &c["want"]);
                check(&mut fails, c, J::String(logger.errors()), &c["errors"]);
            }
            "matches" => {
                let m = matcher(&c["matcher"]);
                let p = &c["page"];
                // Go's NewInfo panics on an empty environment; the matcher only reads the field.
                let mut site = new_dummy_hugo_site(Arc::new(Env("x".to_string()))).unwrap();
                Arc::get_mut(&mut site).unwrap().h.environment =
                    p["env"].as_str().unwrap().to_string();
                let page = MatchPage {
                    kind: p["kind"].as_str().unwrap().to_string(),
                    lang: p["lang"].as_str().unwrap().to_string(),
                    path: p["path"].as_str().unwrap().to_string(),
                    site: SiteRef(site),
                };
                check(&mut fails, c, J::Bool(m.matches(&page)), &c["want"]);
            }
            "newOutputFormat" => {
                let f = output_format(&c["format"]);
                let o = OutputFormat::new(
                    "/rel/",
                    "https://x/abs/",
                    c["canonical"].as_bool().unwrap(),
                    f,
                );
                let got = serde_json::json!({"rel": o.rel, "name": o.name(), "permalink": o.permalink(), "relPermalink": o.rel_permalink()});
                check(&mut fails, c, got, &c["want"]);
            }
            "outputFormatsGet" => {
                let ofs: Vec<OutputFormat> = c["formats"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|f| {
                        let f = output_format(f);
                        let rel = format!("/r/{}", f.name);
                        OutputFormat::new(&rel, "p", false, f)
                    })
                    .collect();
                let got = match output_formats_get(&ofs, c["name"].as_str().unwrap()) {
                    Some(o) => J::String(o.rel_permalink().to_string()),
                    None => J::Null,
                };
                check(&mut fails, c, got, &c["want"]);
            }
            "markupToMediaType" => {
                let types = nh_media::media::config::default_types();
                let got = mt_json(&markup_to_media_type(c["in"].as_str().unwrap(), &types));
                check(&mut fails, c, got, &c["want"]);
            }
            "pageConfig" => {
                let i = &c["in"];
                let s = |k: &str| i[k].as_str().unwrap().to_string();
                let mut pc = PageConfig {
                    kind: s("kind"),
                    path: s("path"),
                    lang: s("lang"),
                    ..Default::default()
                };
                pc.content.markup = s("markup");
                pc.content.media_type = s("mediaType");
                if let Some(o) = i["outputs"].as_array() {
                    pc.outputs = o.iter().map(|x| x.as_str().unwrap().to_string()).collect();
                }
                pc.params = match decode(&i["params"]) {
                    Value::Map(m) => Some((*m).clone()),
                    _ => None,
                };
                if i["cascade"].as_bool().unwrap() {
                    pc.cascade = Some(vec![Map::new(MapType::StringAny)]);
                }
                let mut got = serde_json::Map::new();
                if let Err(e) = pc.init(i["pagesFromData"].as_bool().unwrap()) {
                    got.insert("initErr".into(), J::String(e.message().to_string()));
                }
                let formats = nh_media::output::output_format::default_formats();
                let types = nh_media::media::config::default_types();
                if let Err(e) = pc.compile(&s("ext"), None, &formats, &types) {
                    got.insert("compileErr".into(), J::String(e.message().to_string()));
                }
                let names: Vec<String> = pc
                    .configured_output_formats
                    .0
                    .iter()
                    .map(|f| f.name.clone())
                    .collect();
                got.insert("path".into(), J::String(pc.path.clone()));
                got.insert("markup".into(), J::String(pc.content.markup.clone()));
                got.insert("contentMediaType".into(), mt_json(&pc.content_media_type));
                got.insert(
                    "outputs".into(),
                    if names.is_empty() {
                        J::Null
                    } else {
                        serde_json::json!(names)
                    },
                );
                got.insert(
                    "params".into(),
                    encode(
                        &pc.params
                            .map(Value::map)
                            .unwrap_or(Value::TypedNil(Arc::from("maps.Params"))),
                    ),
                );
                check(&mut fails, c, J::Object(got), &c["want"]);
            }
            "namedPageMetaValue" => {
                let p = nh_page::page_nop::nop_page();
                let name = c["name"].as_str().unwrap();
                let got = match named_page_meta_value(p.as_ref(), name) {
                    Ok(Some(v)) => {
                        let value = match &v {
                            Value::Object(o) if o.type_name() == "media.Type" => {
                                mt_json(&MediaType::default())
                            }
                            // The Page trait returns Vec: Go's nil []string is an empty list here.
                            Value::List(l) if l.items.is_empty() => {
                                serde_json::json!({"t": "nil:[]string"})
                            }
                            _ => encode(&v),
                        };
                        serde_json::json!({"found": true, "value": value})
                    }
                    Ok(None) => serde_json::json!({"found": false, "value": {"t": "nil"}}),
                    Err(e) => serde_json::json!({"found": false, "err": e.message()}),
                };
                check(&mut fails, c, got, &c["want"]);
            }
            f => panic!("{f}"),
        }
    }
    eprintln!("misc: {n} cases, {} failures", fails.len());
    for f in fails.iter().take(30) {
        eprintln!("  {f}");
    }
    assert!(fails.is_empty());
}

/// Go: page_data_test.go and the page.Data methods.
#[test]
fn page_data() {
    use nh_page::page_data::*;
    let d = new_data(Map::new(MapType::StringAny));
    let m = d.as_map().unwrap();
    assert!(matches!(data_pages(m), Value::TypedNil(t) if &*t == "page.Pages"));
    let mut m2 = Map::new(MapType::StringAny);
    m2.insert("pages", nh_page::page::pages_to_value(&[]));
    assert_eq!(data_pages(&m2).as_list().unwrap().items.len(), 0);
    let mut m3 = Map::new(MapType::StringAny);
    m3.insert("pages", Value::object(LazyPages(Arc::new(Vec::new))));
    assert!(data_pages(&m3).as_list().is_some());
    let mut m4 = Map::new(MapType::StringAny);
    m4.insert("pages", Value::string("x"));
    assert_eq!(
        try_data_pages(&m4).unwrap_err().message(),
        "string is not Pages"
    );
    assert!(data_has_method("Pages"));
    assert!(data_call_method(&(), &d, "Pages", &[]).unwrap().is_ok());
}

/// The nop page's template table and flags (`*page.nopPage`, `page.NilPage`).
#[test]
fn nop_page() {
    use nh_page::page_nop::*;
    let nil = nil_page_value();
    let o = nil.as_object().unwrap();
    assert_eq!(o.type_name(), NOP_PAGE_TYPE);
    assert_eq!(o.is_zero(), Some(true));
    assert!(o.has_method("IsHome") && !o.has_method("IsZero"));
    assert_eq!(
        o.call_method(&(), "Title", &[]).unwrap().unwrap(),
        Value::string("")
    );
    assert!(
        matches!(o.call_method(&(), "Parent", &[]).unwrap().unwrap(), Value::TypedNil(t) if &*t == "page.Page")
    );
    assert!(
        o.call_method(&(), "Title", &[Value::int(1)])
            .unwrap()
            .is_err()
    );
    assert_eq!(
        o.call_method(&(), "Eq", std::slice::from_ref(&nil))
            .unwrap()
            .unwrap(),
        Value::Bool(true)
    );
    assert_eq!(
        o.call_method(&(), "Eq", &[PageRef(nop_page()).to_value()])
            .unwrap()
            .unwrap(),
        Value::Bool(false)
    );
    let nop = PageRef(nop_page()).to_value();
    assert_eq!(nop.as_object().unwrap().is_zero(), Some(false));
    assert_eq!(
        nop.as_object().unwrap().go_string().unwrap().as_bytes(),
        b"nopPage"
    );
    assert!(
        named_page_meta_value(nop_page().as_ref(), "custom")
            .unwrap()
            .is_none()
    );
}
