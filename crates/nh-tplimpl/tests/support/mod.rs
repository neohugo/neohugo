//! Shared helpers of the nh-tplimpl oracle tests: fixture loading, rebuilding a site's layouts
//! filesystem and template store from the store oracle's fixture (the same files in a MemMapFs
//! at the same paths, the modules' layouts mounts, the path parser and output formats from the
//! recorded configuration), template ids as the Go oracle computes them, and the store dump.

#![allow(dead_code)]

use std::collections::{BTreeMap, BTreeSet};
use std::io::Read;
use std::path::PathBuf;
use std::sync::{Arc, OnceLock};

use nh_common::object::NamedTypeRegistry;
use nh_common::paths::pathparser::PathParser;
use nh_hugofs::afero::{self, Fs};
use nh_hugofs::component_fs::{ComponentFs, ComponentFsOptions};
use nh_hugofs::fileinfo::{FileMeta, FileMetaInfo};
use nh_hugofs::overlayfs::{Options as OverlayOptions, OverlayFs};
use nh_hugofs::rootmapping_fs::{RootMapping, RootMappingFs};
use nh_markup::goldmark::goldmark_config::{RenderHook, RenderHooks};
use nh_media::media::media_type::{MediaType, SuffixInfo, Types as MediaTypes};
use nh_media::output::output_format::{Formats, OutputFormat};
use nh_tplimpl::category::{Category, opt_category_string};
use nh_tplimpl::engine::{FuncMap, TplFunc};
use nh_tplimpl::templatedescriptor::TemplateDescriptor;
use nh_tplimpl::templatestore::{SiteOptions, StoreOptions, TemplInfo, TemplateStore};
use serde_json::{Value as J, json};

/// Reads a gzip-compressed JSON fixture.
pub fn load_fixture(topic: &str, name: &str) -> J {
    let path = fixture_dir(topic).join(name);
    let f = std::fs::File::open(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    let mut s = String::new();
    flate2::read::GzDecoder::new(f)
        .read_to_string(&mut s)
        .unwrap();
    serde_json::from_str(&s).unwrap()
}

pub fn fixture_dir(topic: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(topic)
}

/// The named sites of the oracles (tsupport.AllSites without the integration test archives).
pub const SITES: &[&str] = &["docs", "testsite", "legacy", "modern", "themes"];

/// Replaces the pool references (`{"$p": i}`) of an integration fixture by the strings.
fn unpool(v: &mut J, pool: &[J]) {
    match v {
        J::Object(m) => {
            if m.len() == 1
                && let Some(i) = m.get("$p").and_then(|i| i.as_u64())
            {
                *v = pool[i as usize].clone();
                return;
            }
            for (_, x) in m.iter_mut() {
                unpool(x, pool);
            }
        }
        J::Array(a) => {
            for x in a.iter_mut() {
                unpool(x, pool);
            }
        }
        _ => {}
    }
}

/// The integration test archives' fixtures of a topic (`integration.json.gz`: `{"sites":
/// {name: fixture}, "pool": [...]}`), by site name.
pub fn integration_fixtures(topic: &str) -> BTreeMap<String, J> {
    let mut fx = load_fixture(topic, "integration.json.gz");
    let pool = fx["pool"].as_array().cloned().unwrap_or_default();
    let mut sites = fx["sites"].take();
    unpool(&mut sites, &pool);
    match sites {
        J::Object(m) => m.into_iter().collect(),
        _ => BTreeMap::new(),
    }
}

/// Runs `f` on a thread with a large stack (template parsing, escaping and execution recurse
/// like Go, whose goroutine stacks grow).
pub fn big_stack<T: Send + 'static>(f: impl FnOnce() -> T + Send + 'static) -> T {
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(f)
        .unwrap()
        .join()
        .unwrap()
}

fn s(d: &J, k: &str) -> String {
    d[k].as_str()
        .unwrap_or_else(|| panic!("{k} in {d}"))
        .to_string()
}

/// Rebuilds a `media.Type` from the oracle's dump.
pub fn media_type(d: &J) -> MediaType {
    let mut m = MediaType::from_string(&s(d, "type")).unwrap_or_default();
    m.typ = s(d, "type");
    m.main_type = s(d, "mainType");
    m.sub_type = s(d, "subType");
    m.delimiter = s(d, "delimiter");
    m.first_suffix = SuffixInfo {
        suffix: s(d, "suffix"),
        full_suffix: s(d, "fullSuffix"),
    };
    m.suffixes_csv = s(d, "suffixesCSV");
    assert_eq!(m.mime_suffix(), s(d, "mimeSuffix"), "mimeSuffix of {d}");
    m
}

/// Rebuilds an `output.Format` from the oracle's dump.
pub fn output_format(d: &J) -> OutputFormat {
    let b = |k: &str| d[k].as_bool().unwrap();
    OutputFormat {
        name: s(d, "name"),
        media_type: media_type(&d["mediaType"]),
        path: s(d, "path"),
        base_name: s(d, "baseName"),
        rel: s(d, "rel"),
        protocol: s(d, "protocol"),
        is_plain_text: b("isPlainText"),
        is_html: b("isHTML"),
        no_ugly: b("noUgly"),
        ugly: b("ugly"),
        not_alternative: b("notAlternative"),
        root: b("root"),
        permalinkable: b("permalinkable"),
        weight: d["weight"].as_i64().unwrap(),
    }
}

fn strings(d: &J) -> Vec<String> {
    match d.as_array() {
        Some(a) => a.iter().map(|v| v.as_str().unwrap().to_string()).collect(),
        None => Vec::new(),
    }
}

/// The path parser allconfig builds (config/allconfig/allconfig.go:918-935).
pub fn path_parser(cfg: &J, formats: &Formats) -> PathParser {
    let language_index = cfg["languageIndex"].as_object().map(|m| {
        m.iter()
            .map(|(k, v)| (k.clone(), v.as_u64().unwrap() as usize))
            .collect::<BTreeMap<_, _>>()
    });
    let disabled: BTreeSet<String> = strings(&cfg["disabledLanguages"]).into_iter().collect();
    let content: BTreeSet<String> = strings(&cfg["contentSuffixes"]).into_iter().collect();
    let formats = formats.clone();
    PathParser {
        language_index,
        is_lang_disabled: Some(Arc::new(move |l: &str| disabled.contains(l))),
        is_content_ext: Some(Arc::new(move |e: &str| content.contains(e))),
        is_output_format: Some(Arc::new(move |name: &str, ext: &str| {
            if name.is_empty() {
                return false;
            }
            if let Some(of) = formats.get_by_name(name) {
                if !ext.is_empty() && !of.media_type.has_suffix(ext) {
                    return false;
                }
                return true;
            }
            false
        })),
    }
}

/// A site rebuilt from the store fixture.
pub struct Site {
    pub fx: J,
    pub fs: Arc<dyn Fs>,
    pub opts: StoreOptions,
    pub func_names: Vec<String>,
}

/// The layouts component filesystem as basefs builds it: the site files in a MemMapFs at the Go
/// paths, one RootMappingFs per module (its non-content, non-static mounts, weighted like
/// createOverlayFs), overlaid in module order, viewed as the "layouts" component.
pub fn layouts_fs(fx: &J, pp: &Arc<PathParser>, default_content_language: &str) -> Arc<dyn Fs> {
    let mem = afero::new_mem_map_fs();
    let site_dir = s(fx, "siteDir");
    for (k, v) in fx["files"].as_object().unwrap() {
        let name = format!("{site_dir}/{k}");
        afero::write_file(&*mem, &name, v.as_str().unwrap().as_bytes(), 0o666).unwrap();
    }
    let source = nh_hugofs::decorators::new_base_file_decorator(mem, Vec::new());

    let mut overlay = OverlayFs::new(OverlayOptions::default());
    for m in fx["modules"].as_array().unwrap() {
        let ordinal = m["ordinal"].as_i64().unwrap();
        let dir = s(m, "dir");
        let is_project = m["isProject"].as_bool().unwrap();
        let mounts = m["mounts"].as_array().unwrap();
        let mut from_to = Vec::new();
        for (i, mnt) in mounts.iter().enumerate() {
            let target = s(mnt, "target");
            if target.starts_with("content") || target.starts_with("static") {
                continue;
            }
            let weight = (10 + ordinal) * (mounts.len() as i64 - i as i64);
            let source = s(mnt, "source");
            let (base, filename) = if source.starts_with('/') {
                (String::new(), source.clone())
            } else {
                (
                    dir.clone(),
                    go_path::filepath::join(&[dir.as_str(), source.as_str()]),
                )
            };
            let mut meta = FileMeta {
                watch: m["watch"].as_bool().unwrap(),
                weight,
                ..Default::default()
            };
            meta.lang = s(mnt, "lang");
            let mut rm = RootMapping::new(&target, &filename, Arc::new(meta));
            rm.to_base = base;
            rm.module = s(m, "path");
            rm.module_ordinal = ordinal;
            rm.is_project = is_project;
            from_to.push(rm);
        }
        let rmfs = RootMappingFs::new(source.clone(), from_to).unwrap();
        overlay = overlay.append(vec![rmfs]);
    }
    ComponentFs::new(ComponentFsOptions {
        fs: Arc::new(overlay),
        component: "layouts".to_string(),
        default_content_language: default_content_language.to_string(),
        path_parser: pp.clone(),
    })
}

/// Loads the store fixture of a site and rebuilds its store options.
pub fn site(name: &str) -> Site {
    site_from_fixture(load_fixture("store", &format!("{name}.json.gz")))
}

/// Rebuilds the store options of a site from a fixture in the store oracle's format.
pub fn site_from_fixture(fx: J) -> Site {
    let cfg = &fx["config"];
    let formats = Formats(
        cfg["outputFormats"]
            .as_array()
            .unwrap()
            .iter()
            .map(output_format)
            .collect(),
    );
    let media_types = MediaTypes(
        cfg["mediaTypes"]
            .as_array()
            .unwrap()
            .iter()
            .map(media_type)
            .collect(),
    );
    let pp = Arc::new(path_parser(cfg, &formats));
    let dcl = s(cfg, "defaultContentLanguage");
    let fs = layouts_fs(&fx, &pp, &dcl);
    let taxonomies = cfg["taxonomies"]
        .as_object()
        .map(|m| {
            m.iter()
                .map(|(k, v)| (k.clone(), v.as_str().unwrap().to_string()))
                .collect()
        })
        .unwrap_or_default();
    let opts = StoreOptions {
        fs: fs.clone(),
        log: None,
        path_parser: pp,
        output_formats: formats,
        media_types,
        default_content_language: dcl,
        default_output_format: s(cfg, "defaultOutputFormat"),
        taxonomy_singular_plural: taxonomies,
        watching: false,
        render_hooks: RenderHooks {
            image: RenderHook {
                enable_default: None,
                use_embedded: s(cfg, "renderHookImage"),
            },
            link: RenderHook {
                enable_default: None,
                use_embedded: s(cfg, "renderHookLink"),
            },
        },
        named_types: Arc::new(NamedTypeRegistry::new()),
    };
    let func_names = strings(&fx["store"]["funcNames"]);
    Site {
        fx,
        fs,
        opts,
        func_names,
    }
}

/// A function map with a stub for every name (the store only needs the names to parse).
pub fn stub_funcs(names: &[String]) -> FuncMap {
    let mut m = FuncMap::new();
    for n in names {
        let name = n.clone();
        let f: TplFunc = Arc::new(move |_ctx, _args| {
            Err(go_value::Error::new(format!("stub function {name} called")))
        });
        m.insert(n.clone(), f);
    }
    m
}

impl Site {
    /// The store, built with stub functions.
    pub fn store(&self) -> TemplateStore {
        self.store_with(stub_funcs(&self.func_names))
    }

    pub fn store_with(&self, funcs: FuncMap) -> TemplateStore {
        TemplateStore::new(
            self.opts.clone(),
            SiteOptions {
                site: Arc::new(OnceLock::new()),
                template_funcs: Arc::new(funcs),
            },
        )
        .unwrap_or_else(|e| panic!("{}: NewStore: {e}", s(&self.fx, "site")))
    }

    /// The walk of the layouts filesystem (Go helpers.Walk), as the oracle records it.
    pub fn layouts_walk(&self) -> J {
        let mut out = Vec::new();
        let mut walker = |pth: &str, fi: &FileMetaInfo| -> nh_common::Result<()> {
            let m = fi.meta();
            let pi = m
                .path_info
                .as_ref()
                .map(|p| p.path().to_string())
                .unwrap_or_default();
            out.push(json!([pth, fi.is_dir(), m.filename, m.module_ordinal, pi]));
            Ok(())
        };
        nh_helpers::path::walk(self.fs.clone(), "", &mut walker).unwrap();
        opt_array(out)
    }
}

/// Go's `OracleNormName`: a doParseTemplate name counter ("x.html-2", "x.html-2$htmltemplate_…")
/// becomes "-N" (regexp `^(.*\.[A-Za-z0-9]+)-[0-9]+($|\$.*)`, greedy).
pub fn norm_name(name: &str) -> String {
    let b = name.as_bytes();
    let mut i = b.len();
    while i > 0 {
        i -= 1;
        if b[i] != b'-' {
            continue;
        }
        let mut j = i + 1;
        while j < b.len() && b[j].is_ascii_digit() {
            j += 1;
        }
        if j == i + 1 || !(j == b.len() || b[j] == b'$') {
            continue;
        }
        let mut k = i;
        while k > 0 && b[k - 1].is_ascii_alphanumeric() {
            k -= 1;
        }
        if k == i || k == 0 || b[k - 1] != b'.' {
            continue;
        }
        return format!("{}-N{}", &name[..i], &name[j..]);
    }
    name.to_string()
}

/// Go `%+v` of a descriptor.
pub fn desc_string(d: &TemplateDescriptor) -> String {
    d.go_string_plus_v()
}

/// Template ids as the Go oracle computes them (`OracleTemplID`).
pub struct Ids {
    ids: Vec<(Arc<TemplInfo>, String)>,
}

impl Ids {
    pub fn new(store: &TemplateStore) -> Ids {
        let mut ids = Vec::new();
        for (k, nk, vv) in store.main_tree_entries() {
            let path = path_of(&vv);
            ids.push((
                vv.clone(),
                format!(
                    "main|{}|{}|{}|{}",
                    k,
                    nk.c.string(),
                    path,
                    desc_string(&nk.d)
                ),
            ));
            let overlay_d = vv.d();
            for (bk, bd, tb) in variants_of(&vv) {
                ids.push((
                    tb,
                    format!(
                        "variant|{}|{}|{}|base={}|{}",
                        k,
                        path,
                        desc_string(&overlay_d),
                        bk,
                        desc_string(&bd)
                    ),
                ));
            }
        }
        for (k, name, d, vv) in store.shortcode_tree_entries() {
            let path = path_of(&vv);
            ids.push((
                vv.clone(),
                format!("shortcode|{}|{}|{}|{}", k, name, path, desc_string(&d)),
            ));
        }
        Ids { ids }
    }

    pub fn id(&self, ti: Option<&Arc<TemplInfo>>) -> String {
        let Some(ti) = ti else {
            return String::new();
        };
        for (t, id) in &self.ids {
            if Arc::ptr_eq(t, ti) {
                return id.clone();
            }
        }
        format!("other|{}|{}", path_of(ti), desc_string(&ti.d()))
    }

    /// The id of a template given by reference (a Consider func's candidate).
    pub fn id_of_ref(&self, ti: &TemplInfo) -> String {
        for (t, id) in &self.ids {
            if std::ptr::eq(Arc::as_ptr(t), ti) {
                return id.clone();
            }
        }
        format!("other|{}|{}", path_of(ti), desc_string(&ti.d()))
    }

    /// The template with the given id.
    pub fn get(&self, id: &str) -> Option<Arc<TemplInfo>> {
        self.ids
            .iter()
            .find(|(_, i)| i == id)
            .map(|(t, _)| t.clone())
    }
}

pub fn path_of(ti: &TemplInfo) -> String {
    ti.path_info
        .as_ref()
        .map(|p| p.path().to_string())
        .unwrap_or_default()
}

/// The base variants of a template: (base tree key, base descriptor, variant template).
pub fn variants_of(ti: &TemplInfo) -> Vec<(String, TemplateDescriptor, Arc<TemplInfo>)> {
    ti.base_variants_tree()
}

fn opt_array(v: Vec<J>) -> J {
    if v.is_empty() { J::Null } else { J::Array(v) }
}

fn entry(ids: &Ids, ti: &Arc<TemplInfo>) -> serde_json::Map<String, J> {
    let pi = ti.parse_info();
    let name = if ti.template().is_none() && ti.path_info.is_none() {
        "<nil>".to_string()
    } else {
        norm_name(&ti.name())
    };
    let mut m = serde_json::Map::new();
    m.insert("id".into(), json!(ids.id(Some(ti))));
    m.insert("category".into(), json!(opt_category_string(ti.category)));
    m.insert("subCategory".into(), json!(ti.sub_category().string()));
    m.insert("name".into(), json!(name));
    m.insert("path".into(), json!(path_of(ti)));
    m.insert("desc".into(), json!(desc_string(&ti.d())));
    m.insert("noBaseOf".into(), json!(ti.no_base_of()));
    m.insert("legacyMapped".into(), json!(ti.is_legacy_mapped()));
    m.insert("filename".into(), json!(ti.filename()));
    m.insert(
        "parseInfo".into(),
        json!({"isInner": pi.is_inner, "hasReturn": pi.has_return, "version": pi.config.version}),
    );
    m.insert("hasTemplate".into(), json!(ti.template().is_some()));
    m.insert(
        "isText".into(),
        json!(ti.template().is_some_and(|t| t.is_text())),
    );
    m.insert(
        "contentHash".into(),
        json!(nh_common::hashing::xxhash_from_string_hex_encoded(
            &ti.content()
        )),
    );
    m
}

fn sort_by_id(v: &mut [J]) {
    v.sort_by(|a, b| a["id"].as_str().cmp(&b["id"].as_str()));
}

/// The Rust store dumped like `OracleDump` (without the function names and the walk).
pub fn dump_store(store: &TemplateStore) -> J {
    let ids = Ids::new(store);
    let mut main: Vec<J> = Vec::new();
    let mut by_key: BTreeMap<String, Vec<J>> = BTreeMap::new();
    let mut key_order: Vec<String> = Vec::new();
    for (k, _nk, vv) in store.main_tree_entries() {
        let mut e = entry(&ids, &vv);
        e.insert("key".into(), json!(k));
        let mut variants = Vec::new();
        for (bk, bd, tb) in variants_of(&vv) {
            let mut ve = entry(&ids, &tb);
            ve.insert("baseKey".into(), json!(bk));
            ve.insert("baseDesc".into(), json!(desc_string(&bd)));
            ve.insert("baseId".into(), json!(ids.id(tb.base_template())));
            ve.insert("overlayId".into(), json!(ids.id(Some(&vv))));
            variants.push(J::Object(ve));
        }
        sort_by_id(&mut variants);
        e.insert("variants".into(), opt_array(variants));
        let mut overlays: Vec<String> = vv.overlays().iter().map(|o| ids.id(Some(o))).collect();
        overlays.sort();
        e.insert(
            "overlays".into(),
            opt_array(overlays.into_iter().map(J::String).collect()),
        );
        if !by_key.contains_key(&k) {
            key_order.push(k.clone());
        }
        by_key.entry(k).or_default().push(J::Object(e));
    }
    for k in key_order {
        let mut es = by_key.remove(&k).unwrap();
        sort_by_id(&mut es);
        main.extend(es);
    }

    let mut shortcodes: Vec<J> = Vec::new();
    let mut cur_key: Option<String> = None;
    let mut es: Vec<J> = Vec::new();
    for (k, name, _d, vv) in store.shortcode_tree_entries() {
        if cur_key.as_ref() != Some(&k) {
            sort_by_id(&mut es);
            shortcodes.append(&mut es);
            cur_key = Some(k.clone());
        }
        let mut e = entry(&ids, &vv);
        e.insert("key".into(), json!(k));
        e.insert("scName".into(), json!(name));
        es.push(J::Object(e));
    }
    sort_by_id(&mut es);
    shortcodes.append(&mut es);

    let by_path: Vec<J> = store
        .templates_by_path_entries()
        .into_iter()
        .map(|(k, v)| {
            let id = if k.starts_with("__hdeferred/") {
                "deferred".to_string()
            } else {
                ids.id(Some(&v))
            };
            let name = if v.template().is_none() && v.path_info.is_none() {
                "<nil>".to_string()
            } else {
                norm_name(&v.name())
            };
            json!([k, id, name])
        })
        .collect();
    let by_name: Vec<J> = store
        .shortcodes_by_name_entries()
        .into_iter()
        .map(|(k, v)| json!([k, ids.id(Some(&v))]))
        .collect();

    let mut templates: Vec<String> = store.templates().iter().map(|t| ids.id(Some(t))).collect();
    templates.sort();
    let mut unused: Vec<String> = store
        .unused_templates()
        .iter()
        .map(|t| ids.id(Some(t)))
        .collect();
    unused.sort();

    json!({
        "main": opt_array(main),
        "shortcodes": opt_array(shortcodes),
        "byPath": by_path,
        "shortcodesByName": by_name,
        "templates": opt_array(templates.into_iter().map(J::String).collect()),
        "unused": opt_array(unused.into_iter().map(J::String).collect()),
    })
}

/// Category from its Go `String()`.
pub fn category_from_string(s: &str) -> Category {
    match s {
        "CategoryLayout" => Category::Layout,
        "CategoryBaseof" => Category::Baseof,
        "CategoryMarkup" => Category::Markup,
        "CategoryShortcode" => Category::Shortcode,
        "CategoryPartial" => Category::Partial,
        "CategoryServer" => Category::Server,
        "CategoryHugo" => Category::Hugo,
        _ => panic!("unknown category {s}"),
    }
}

/// A descriptor from the oracle's `DescDump`.
pub fn desc_from_json(d: &J) -> TemplateDescriptor {
    let b = |k: &str| d[k].as_bool().unwrap();
    TemplateDescriptor {
        kind: s(d, "kind"),
        layout_from_template: s(d, "layoutFromTemplate"),
        layout_from_user: s(d, "layoutFromUser"),
        output_format: s(d, "outputFormat"),
        media_type: s(d, "mediaType"),
        lang: s(d, "lang"),
        variant1: s(d, "variant1"),
        variant2: s(d, "variant2"),
        layout_from_user_must_match: b("layoutFromUserMustMatch"),
        is_plain_text: b("isPlainText"),
        always_allow_plain_text: b("alwaysAllowPlainText"),
    }
}

/// Compares two JSON values, reporting the first differences with their paths.
pub fn diff_json(path: &str, go: &J, rust: &J, out: &mut Vec<String>) {
    if out.len() > 40 {
        return;
    }
    match (go, rust) {
        (J::Object(a), J::Object(b)) => {
            let keys: BTreeSet<&String> = a.keys().chain(b.keys()).collect();
            for k in keys {
                diff_json(
                    &format!("{path}.{k}"),
                    a.get(k).unwrap_or(&J::Null),
                    b.get(k).unwrap_or(&J::Null),
                    out,
                );
            }
        }
        (J::Array(a), J::Array(b)) => {
            for i in 0..a.len().max(b.len()) {
                diff_json(
                    &format!("{path}[{i}]"),
                    a.get(i).unwrap_or(&J::Null),
                    b.get(i).unwrap_or(&J::Null),
                    out,
                );
            }
        }
        _ => {
            if go != rust {
                out.push(format!("{path}: go {go} rust {rust}"));
            }
        }
    }
}
