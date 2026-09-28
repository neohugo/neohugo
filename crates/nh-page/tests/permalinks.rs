//! Oracle test: `page.PermalinkExpander` (`Expand`, `ExpandPattern`) and
//! `page.DecodePermalinksConfig` against `tools/go-oracle/nh-page/permalinks`
//! (fixtures/permalinks/*.json.gz): the real pages of the oracle builds, with the site's own
//! permalinks config, a config using every token, and patterns using every token.

mod support;

use std::any::Any;
use std::borrow::Cow;
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use go_value::{GoString, HostCtx, Map, Time, Value};
use nh_common::Result;
use nh_common::object::GoResult;
use nh_common::paths::pathparser::Path;
use nh_config::common_config::SitemapConfig;
use nh_helpers::source::file_info::File;
use nh_media::media::media_type::MediaType;
use nh_page::page::{Page, PageRef, Pages};
use nh_page::page_outputformat::OutputFormats;
use nh_page::permalinks::{PermalinkExpander, decode_permalinks_config};
use nh_page::related::{IndexConfig, Keyword};
use nh_page::site::SiteRef;
use nh_resource::resourcetypes::{Resource, Resources};
use serde_json::Value as J;
use support::*;

/// A page with the attributes the expander reads (the rest is never called).
struct FakePage {
    kind: String,
    date: Time,
    title: String,
    slug: String,
    section: String,
    path_info: Arc<Path>,
    file: Option<Arc<File>>,
    sections_entries: Vec<String>,
    sections_path: String,
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
        unimplemented!()
    }
    fn data(&self) -> Value {
        unimplemented!()
    }
    fn name(&self) -> String {
        unimplemented!()
    }
    fn title(&self) -> String {
        self.title.clone()
    }
    fn params(&self) -> Arc<Map> {
        unimplemented!()
    }
    fn key(&self) -> String {
        unimplemented!()
    }
    fn tpl_type_name(&self) -> Cow<'_, str> {
        Cow::Borrowed("*main.fakePage")
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

impl Page for FakePage {
    fn page_id(&self) -> u64 {
        0
    }
    fn unwrap_page(self: Arc<Self>) -> Arc<dyn Page> {
        self
    }
    fn kind(&self) -> String {
        self.kind.clone()
    }
    fn title(&self) -> String {
        self.title.clone()
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
        self.date.clone()
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
        self.section.clone()
    }
    fn page_type(&self) -> String {
        unimplemented!()
    }
    fn layout(&self) -> String {
        unimplemented!()
    }
    fn lang(&self) -> String {
        unimplemented!()
    }
    fn path(&self) -> String {
        unimplemented!()
    }
    fn path_info(&self) -> Arc<Path> {
        self.path_info.clone()
    }
    fn slug(&self) -> String {
        self.slug.clone()
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
        unimplemented!()
    }
    fn file(&self) -> Option<Arc<File>> {
        self.file.clone()
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
    fn output_formats(&self) -> OutputFormats {
        unimplemented!()
    }
    fn all_translations(&self) -> Pages {
        unimplemented!()
    }
    fn translations(&self) -> Pages {
        unimplemented!()
    }
    /// The recorded `CurrentSection()` values are the page's own here.
    fn current_section(&self) -> Option<PageRef> {
        None
    }
    fn sections_entries(&self) -> Vec<String> {
        self.sections_entries.clone()
    }
    fn sections_path(&self) -> String {
        self.sections_path.clone()
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

fn patterns_map(v: &J) -> BTreeMap<String, BTreeMap<String, String>> {
    v.as_object()
        .unwrap()
        .iter()
        .map(|(k, m)| {
            (
                k.clone(),
                m.as_object()
                    .unwrap()
                    .iter()
                    .map(|(k2, p)| (k2.clone(), p.as_str().unwrap().to_string()))
                    .collect(),
            )
        })
        .collect()
}

fn res(r: Result<String>) -> J {
    match r {
        Ok(s) => serde_json::json!({ "ok": enc(s.as_bytes()) }),
        Err(e) => serde_json::json!({ "err": e.message() }),
    }
}

#[test]
fn permalink_expander_matches_go() {
    nh_page::page::init();
    let mut n = 0usize;
    let mut fails = Vec::new();
    for file in fixture_files("permalinks") {
        if file == "decode.json.gz" {
            continue;
        }
        let fx = fixture(&format!("permalinks/{file}"));
        let misses: Misses = Arc::new(Mutex::new(Vec::new()));
        let pp = build_parser(&fx["parser"], &misses);
        let paths = build_paths(&fx["paths"], &pp);
        let specs: Vec<_> = fx["pathspecs"]
            .as_array()
            .unwrap()
            .iter()
            .map(build_path_spec)
            .collect();
        let patterns: Vec<String> = fx["patterns"]
            .as_array()
            .unwrap()
            .iter()
            .map(|p| p.as_str().unwrap().to_string())
            .collect();
        let site_cfg = patterns_map(&fx["siteConfig"]);
        let all_cfg = patterns_map(&fx["allConfig"]);

        let mut expanders = Vec::new();
        for ps in &specs {
            let ps2 = ps.clone();
            let urlize: Arc<dyn Fn(&str) -> String + Send + Sync> =
                Arc::new(move |s| ps2.urlize(s));
            expanders.push((
                PermalinkExpander::new(urlize.clone(), &site_cfg).unwrap(),
                PermalinkExpander::new(urlize, &all_cfg).unwrap(),
            ));
        }

        for c in fx["cases"].as_array().unwrap() {
            let file_obj = c.get("file").map(|f| {
                let pi = paths[f["pathInfo"].as_u64().unwrap() as usize].clone();
                let meta = nh_hugofs::fileinfo::FileMeta {
                    path_info: Some(pi),
                    ..Default::default()
                };
                let fo = File::new(nh_hugofs::fileinfo::new_file_meta_info("", false, meta));
                assert_eq!(fo.dir(), gostring(&f["dir"]));
                assert_eq!(
                    fo.translation_base_name(),
                    gostring(&f["translationBaseName"])
                );
                fo
            });
            let entries = match decode(&c["sectionsEntries"]) {
                Value::List(l) => l
                    .items
                    .iter()
                    .map(|v| v.as_go_string().unwrap().to_str_lossy().into_owned())
                    .collect(),
                _ => Vec::new(),
            };
            let p = FakePage {
                kind: c["kind"].as_str().unwrap().to_string(),
                date: decode_time(&c["date"]),
                title: gostring(&c["title"]),
                slug: gostring(&c["slug"]),
                section: gostring(&c["section"]),
                path_info: paths[c["pathInfo"].as_u64().unwrap() as usize].clone(),
                file: file_obj,
                sections_entries: entries,
                sections_path: gostring(&c["sectionsPath"]),
            };
            let (site_exp, all_exp) = &expanders[c["ps"].as_u64().unwrap() as usize];
            let mut check = |name: &str, got: J, want: &J| {
                n += 1;
                if &got != want {
                    fails.push(format!(
                        "{file} {} {name}: got {got}, want {want}",
                        c["pathInfo"]
                    ));
                }
            };
            check(
                "expandSite",
                res(site_exp.expand(&p.section, &p)),
                &c["expandSite"],
            );
            check(
                "expandAll",
                res(all_exp.expand(&p.section, &p)),
                &c["expandAll"],
            );
            for (i, pat) in patterns.iter().enumerate() {
                let got = match catch(|| all_exp.expand_pattern(pat, &p)) {
                    Ok(r) => res(r),
                    Err(e) => serde_json::json!({ "panic": e }),
                };
                check(pat, got, &c["patterns"][i]);
            }
        }
        assert!(
            misses.lock().unwrap().is_empty(),
            "{file}: {:?}",
            misses.lock().unwrap()
        );
    }
    eprintln!("permalinks: {n} checks, {} failures", fails.len());
    for f in fails.iter().take(40) {
        eprintln!("  {f}");
    }
    assert!(fails.is_empty());
}

#[test]
fn decode_permalinks_config_matches_go() {
    let fx = fixture("permalinks/decode.json.gz");
    let mut fails = Vec::new();
    for c in fx["cases"].as_array().unwrap() {
        let input = match decode(&c["in"]) {
            Value::Map(m) => (*m).clone(),
            // A nil maps.Params (no [permalinks] section).
            Value::TypedNil(_) => Map::new(go_value::MapType::Params),
            other => panic!("{other:?}"),
        };
        let got = match decode_permalinks_config(&input) {
            Ok(m) => serde_json::json!({ "ok": m }),
            Err(e) => serde_json::json!({ "err": e.message() }),
        };
        if got != c["want"] {
            fails.push(format!("{}: got {got}, want {}", c["name"], c["want"]));
        }
    }
    for f in &fails {
        eprintln!("{f}");
    }
    assert!(fails.is_empty());
}
