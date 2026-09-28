//! The `keys` oracle (tools/go-oracle/nh-resources/keys): `TransformationKey` (the seeksnack
//! SCSS chain gives `scss/website.scss_ce37005bb9b0d2e87a9f0d33876c2b52`, the name of its
//! `resources/_gen/assets` file) and the cold image `Key` rule (the golden watermark vector
//! `/images/watermark_hu_3bf49ff914f6e68c.png_6519743917224815147` of specs/images.md §4.5, and
//! the synthetic watermark's processed images end to end).

mod support;

use std::borrow::Cow;
use std::sync::{Arc, Once};

use go_hashstructure::{GoMap, GoStruct, HashValue};
use go_value::{HostCtx, Object, Value};
use nh_resource::resourcetypes::Resource;
use nh_resources::transform::ResourceAdapter;
use serde_json::Value as J;
use support::*;

/// `scss.Options` as the tocss key element (T16 owns the real type; this mirrors its fields for
/// hashstructure: TargetPath, IncludePaths, OutputStyle, Precision, EnableSourceMap, Vars).
struct ScssOptions {
    target_path: String,
    include_paths: Option<Vec<String>>,
    output_style: String,
    precision: i64,
    enable_source_map: bool,
    vars: Option<Vec<(String, Value)>>,
}

impl Object for ScssOptions {
    fn type_name(&self) -> Cow<'_, str> {
        Cow::Borrowed("scss.Options")
    }
    fn kind(&self) -> go_value::Kind {
        go_value::Kind::Struct
    }
    fn has_method(&self, _name: &str) -> bool {
        false
    }
    fn call_method(&self, _: HostCtx<'_>, _: &str, _: &[Value]) -> Option<go_value::Result<Value>> {
        None
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

fn register() {
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        go_hashstructure::register_object::<ScssOptions>(|o| {
            let include = match &o.include_paths {
                None => HashValue::Slice(None),
                Some(v) => HashValue::string_slice(v.iter().map(String::as_str)),
            };
            let vars = match &o.vars {
                None => HashValue::Map(GoMap::nil()),
                Some(v) => HashValue::string_any_map(
                    v.iter()
                        .map(|(k, x)| (k.as_str(), HashValue::Value(x.clone()))),
                ),
            };
            HashValue::Struct(
                GoStruct::new("Options")
                    .field("TargetPath", o.target_path.as_str())
                    .field("IncludePaths", include)
                    .field("OutputStyle", o.output_style.as_str())
                    .field("Precision", HashValue::int(o.precision))
                    .field("EnableSourceMap", o.enable_source_map)
                    .field("Vars", vars),
            )
        });
    });
}

fn element(e: &J) -> Value {
    if e["t"] == "scss.Options" {
        let v = &e["v"];
        return Value::object(ScssOptions {
            target_path: v["TargetPath"].as_str().unwrap().to_string(),
            include_paths: v["IncludePaths"]
                .as_array()
                .map(|a| a.iter().map(|s| s.as_str().unwrap().to_string()).collect()),
            output_style: v["OutputStyle"].as_str().unwrap().to_string(),
            precision: v["Precision"].as_i64().unwrap(),
            enable_source_map: v["EnableSourceMap"].as_bool().unwrap(),
            vars: match dec(&v["Vars"]) {
                Value::Map(m) => Some(
                    m.entries
                        .iter()
                        .map(|(k, x)| (k.to_str_lossy().into_owned(), x.clone()))
                        .collect(),
                ),
                _ => None,
            },
        });
    }
    dec(e)
}

fn get(site: &Site, p: &str) -> Arc<ResourceAdapter> {
    let root = repo_root();
    let file = root
        .join("crates/nh-resources/tests/fixtures/site/assets")
        .join(p);
    let mut rd = nh_resources::resource::ResourceSourceDescriptor {
        open_read_seek_closer: Some(open_file(file.clone())),
        name_normalized: format!(
            "/{}",
            String::from_utf8_lossy(&go_unicode::strings::to_lower(p.as_bytes()))
        ),
        name_original: format!("/{p}"),
        target_path: format!("/{p}"),
        source_filename_or_path: file.to_string_lossy().into_owned(),
        lazy_publish: true,
        ..Default::default()
    };
    site.specs[0].new_resource_adapter(&mut rd).unwrap()
}

#[test]
fn transformation_keys() {
    register();
    let fx = fixture("keys/keys.json.gz");
    let dir = repo_root().join("crates/nh-resources/tests/fixtures/site");
    let site = load_site(&dir.to_string_lossy(), None);

    let mut n = 0;
    for c in fx["chains"].as_array().unwrap() {
        let name = c["name"].as_str().unwrap();
        let r = get(&site, c["target"].as_str().unwrap());
        assert_eq!(J::String(r.key()), c["targetKey"], "{name}: target key");
        let mut trs = Vec::new();
        for t in c["transformations"].as_array().unwrap_or(&Vec::new()) {
            let elements: Vec<Value> = t["elements"]
                .as_array()
                .unwrap_or(&Vec::new())
                .iter()
                .map(element)
                .collect();
            let tr = nh_resources::resource::new_feature_not_available_transformer(
                t["name"].as_str().unwrap(),
                elements,
            );
            assert_eq!(
                J::String(tr.key().value()),
                t["value"],
                "{name}: {}",
                t["name"]
            );
            trs.push(tr);
        }
        let rt = r.transform(trs).unwrap();
        assert_eq!(
            J::String(rt.transformation_key()),
            c["transformationKey"],
            "{name}: TransformationKey"
        );
        n += 1;
    }
    assert!(n >= 12);

    // The vector of specs/resources-pipeline.md §3.2.
    let seeksnack = &fx["chains"][0];
    assert_eq!(seeksnack["name"], "seeksnack-scss");
    assert_eq!(
        seeksnack["transformationKey"],
        "scss/website.scss_ce37005bb9b0d2e87a9f0d33876c2b52"
    );
}

#[test]
fn cold_image_keys() {
    let fx = fixture("keys/keys.json.gz");
    let dir = repo_root().join("crates/nh-resources/tests/fixtures/site");
    let site = load_site(&dir.to_string_lossy(), None);
    let spec = &site.specs[0];

    // The golden vector (the private watermark's source hash is known).
    let g = &fx["golden"];
    assert_eq!(
        g["key"],
        "/images/watermark_hu_3bf49ff914f6e68c.png_6519743917224815147"
    );
    let cfg = &spec.imaging.cfg;
    assert_eq!(J::String(cfg.source_hash.clone()), g["imagingSourceHash"]);
    let conf = nh_images::config::decode_image_config(
        &["resize".to_string(), "600x480".to_string()],
        cfg,
        nh_images::image::Format::Png,
    )
    .unwrap();
    assert_eq!(J::String(conf.key.clone()), g["resize600Key"]);
    let hash: u64 = g["hash"].as_str().unwrap().parse().unwrap();
    let paths = nh_resource::internal::resourcepaths::ResourcePaths {
        dir: "/images".to_string(),
        file: "watermark.png".to_string(),
        ..Default::default()
    };
    let rp = nh_resources::image::rel_target_path_for_hash(
        &paths,
        nh_images::image::Format::Png,
        &conf,
        hash,
        &cfg.source_hash,
    );
    assert_eq!(J::String(rp.file.clone()), g["name"]);
    // Key() = RelPermalink without the base path + "_" + the root source hash (cold).
    let key = format!(
        "{}_{hash}",
        nh_common::paths::path::path_escape(&rp.target_link())
    );
    assert_eq!(J::String(key.clone()), g["key"]);

    // The Overlay filter key embeds the watermark's Key.
    struct Src(String);
    impl nh_images::image::ImageSource for Src {
        fn decode_image(&self) -> nh_common::Result<nh_images::image::GoImage> {
            Err(nh_common::Error::new("not used"))
        }
        fn key(&self) -> String {
            self.0.clone()
        }
    }
    let f = nh_images::filters::Filters
        .overlay(Arc::new(Src(key)), &Value::int(0), &Value::int(0))
        .unwrap();
    assert_eq!(
        J::String(nh_images::filters::filters_key(&[f])),
        g["filterKey"]
    );

    // End to end: the synthetic watermark.
    let wm = get(&site, "images/watermark.png");
    let w = &fx["watermark"];
    assert_eq!(J::String(wm.key()), w["key"]);
    let mut n = 0;
    for rc in w["resize"].as_array().unwrap() {
        let s = rc["spec"].as_str().unwrap();
        let ir = wm.resize(s).unwrap();
        assert_eq!(J::String(ir.key()), rc["key"], "{s}: key");
        assert_eq!(J::String(ir.name()), rc["name"], "{s}: name");
        for (label, x, y) in [
            ("overlay 0 0", Value::int(0), Value::int(0)),
            ("overlay 20 10", Value::int(20), Value::int(10)),
            (
                "overlay 0.5 3",
                Value::Float(0.5, go_value::FloatKind::F64),
                Value::int64(3),
            ),
        ] {
            let f = nh_images::filters::Filters
                .overlay(ir.clone() as Arc<dyn nh_images::image::ImageSource>, &x, &y)
                .unwrap();
            assert_eq!(
                J::String(nh_images::filters::filters_key(&[f])),
                rc[label],
                "{s}: {label}"
            );
        }
        if let Some(ck) = rc.get("chainKey") {
            let ir2 = ir.resize("50x40").unwrap();
            assert_eq!(&J::String(ir2.key()), ck, "{s}: chain key");
            assert_eq!(
                J::String(Resource::rel_permalink(ir2.as_ref())),
                rc["chainRelPermalink"],
                "{s}: chain"
            );
        }
        assert_eq!(
            J::String(ir.rel_permalink()),
            rc["relPermalink"],
            "{s}: link"
        );
        n += 1;
    }
    assert!(n >= 7);
}

#[test]
fn multihost() {
    let fx = fixture("keys/keys.json.gz");
    let root = repo_root();
    let dir = root.join("crates/nh-resources/tests/fixtures/site-multihost");
    let site = load_site(&dir.to_string_lossy(), None);
    let abs = root.join("crates/nh-resources/tests/fixtures/site/assets/images/watermark.png");
    let want = fx["multihost"].as_array().unwrap();
    assert_eq!(site.specs.len(), want.len());
    for (spec, w) in site.specs.iter().zip(want) {
        assert_eq!(J::String(spec.lang()), w["lang"]);
        let mut rd = nh_resources::resource::ResourceSourceDescriptor {
            target_path: "/images/watermark.png".to_string(),
            open_read_seek_closer: Some(open_file(abs.clone())),
            source_filename_or_path: abs.to_string_lossy().into_owned(),
            lazy_publish: true,
            ..Default::default()
        };
        let r = spec.new_resource_adapter(&mut rd).unwrap();
        let lang = w["lang"].as_str().unwrap();
        assert_eq!(J::String(r.key()), w["key"], "{lang}");
        assert_eq!(J::String(r.rel_permalink()), w["relPermalink"], "{lang}");
        assert_eq!(J::String(r.permalink()), w["permalink"], "{lang}");
        for s in ["300x240", "120x jpg"] {
            let ir = r.resize(s).unwrap();
            let ws = &w[s];
            // The second language finds the file the first one created (Go's read path):
            // its Key has no source hash.
            assert_eq!(J::String(ir.key()), ws["key"], "{lang} {s}");
            assert_eq!(
                J::String(ir.rel_permalink()),
                ws["relPermalink"],
                "{lang} {s}"
            );
            assert_eq!(J::String(ir.permalink()), ws["permalink"], "{lang} {s}");
            assert_eq!(
                J::String(ir.media_type().typ),
                ws["mediaType"],
                "{lang} {s}"
            );
            assert_eq!(J::from(ir.width().unwrap()), ws["width"], "{lang} {s}");
            assert_eq!(J::from(ir.height().unwrap()), ws["height"], "{lang} {s}");
        }
    }
    let mut names: Vec<String> = site
        .published_files()
        .as_object()
        .unwrap()
        .keys()
        .cloned()
        .collect();
    names.sort();
    assert_eq!(J::from(names), fx["multihostPublished"]);
}
