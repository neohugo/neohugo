//! COLD-CACHE RULE (HUGO_LAYER.md §4.5): the port never reads `resources/_gen`. The synthetic
//! site is loaded with a source file system that records every access to a path containing
//! `/_gen`, and the resource dir holds a poisoned `_gen` (garbage under the names Go would read:
//! the processed watermark and the resize/filter chain of the `images` fixture, the
//! transformation-cache files of a `tocss` chain). Processing gives the bytes of the `images`
//! fixture, a failing `tocss` step is an error (Go's fallback would read the poisoned cache
//! file), and nothing under `_gen` is touched.

mod support;

use std::any::Any;
use std::sync::{Arc, Mutex};

use go_value::{Time, Value};
use nh_common::Result;
use nh_hugofs::afero::{File, Fs};
use nh_hugofs::fileinfo::FileMetaInfo;
use nh_resource::resourcetypes::Resource;
use nh_resources::resource::ResourceSourceDescriptor;
use serde_json::Value as J;
use support::*;

/// An `afero.Fs` that records the paths containing `/_gen` it is asked about.
struct PoisonFs {
    inner: Arc<dyn Fs>,
    touched: Arc<Mutex<Vec<String>>>,
}

impl PoisonFs {
    fn check(&self, name: &str) {
        if name.contains("/_gen") || name.starts_with("_gen") {
            self.touched.lock().unwrap().push(name.to_string());
        }
    }
}

impl Fs for PoisonFs {
    fn name(&self) -> &str {
        "PoisonFs"
    }
    fn embedded(&self) -> Option<&dyn Fs> {
        Some(self.inner.as_ref())
    }
    fn create(&self, name: &str) -> Result<Box<dyn File>> {
        self.check(name);
        self.inner.create(name)
    }
    fn mkdir(&self, name: &str, perm: u32) -> Result<()> {
        self.check(name);
        self.inner.mkdir(name, perm)
    }
    fn mkdir_all(&self, path: &str, perm: u32) -> Result<()> {
        self.check(path);
        self.inner.mkdir_all(path, perm)
    }
    fn open(&self, name: &str) -> Result<Box<dyn File>> {
        self.check(name);
        self.inner.open(name)
    }
    fn open_file(&self, name: &str, flag: i32, perm: u32) -> Result<Box<dyn File>> {
        self.check(name);
        self.inner.open_file(name, flag, perm)
    }
    fn remove(&self, name: &str) -> Result<()> {
        self.check(name);
        self.inner.remove(name)
    }
    fn remove_all(&self, path: &str) -> Result<()> {
        self.check(path);
        self.inner.remove_all(path)
    }
    fn rename(&self, old: &str, new: &str) -> Result<()> {
        self.check(old);
        self.check(new);
        self.inner.rename(old, new)
    }
    fn stat(&self, name: &str) -> Result<FileMetaInfo> {
        self.check(name);
        self.inner.stat(name)
    }
    fn chmod(&self, name: &str, mode: u32) -> Result<()> {
        self.check(name);
        self.inner.chmod(name, mode)
    }
    fn chtimes(&self, name: &str, atime: &Time, mtime: &Time) -> Result<()> {
        self.check(name);
        self.inner.chtimes(name, atime, mtime)
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

#[test]
fn never_reads_resources_gen() {
    let root = repo_root();
    let dir = root.join("crates/nh-resources/tests/fixtures/site");
    let images = fixture("images/images.json.gz");
    let keys = fixture("keys/keys.json.gz");

    let tmp = TempDir::new("gen");
    // The poisoned file caches, under the names Go's file caches use (`:resourceDir/_gen`).
    let gen_dir = tmp.path.join("resources/_gen");
    let mut poisoned = 0;
    for o in images["ops"].as_array().unwrap().iter().take(12) {
        if let Some(k) = o["key"].as_str() {
            let rel = k.rsplit_once('_').unwrap().0; // the target path
            let p = gen_dir.join("images").join(rel.trim_start_matches('/'));
            std::fs::create_dir_all(p.parent().unwrap()).unwrap();
            std::fs::write(&p, b"POISON: not an image").unwrap();
            poisoned += 1;
        }
    }
    for c in keys["chains"].as_array().unwrap() {
        let k = c["transformationKey"].as_str().unwrap();
        for ext in ["json", "content"] {
            let p = gen_dir.join("assets").join(format!("{k}.{ext}"));
            std::fs::create_dir_all(p.parent().unwrap()).unwrap();
            let body = if ext == "json" {
                r#"{"Target":"/POISON.css","MediaType":"text/css","Data":{}}"#
            } else {
                "POISON"
            };
            std::fs::write(&p, body).unwrap();
            poisoned += 1;
        }
    }
    assert!(poisoned > 20);

    let touched: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
    let t2 = touched.clone();
    let wrap = move |inner: Arc<dyn Fs>| -> Arc<dyn Fs> {
        Arc::new(PoisonFs {
            inner,
            touched: t2.clone(),
        })
    };
    let site = load_site_in(&dir.to_string_lossy(), tmp, Some(&wrap));
    let spec = site.specs[0].clone();

    // The first source's chains of the images fixture (the watermark resizes, Resize ->
    // Filter(Overlay) -> webp).
    let new_res = |name: &str, file: &str| {
        let abs = root.join(file);
        let mut d = ResourceSourceDescriptor {
            target_path: format!("/images/{name}"),
            open_read_seek_closer: Some(open_file(abs.clone())),
            source_filename_or_path: abs.to_string_lossy().into_owned(),
            lazy_publish: true,
            ..Default::default()
        };
        spec.new_resource_adapter(&mut d).unwrap()
    };
    let wm = new_res(
        "watermark.png",
        "crates/nh-resources/tests/fixtures/site/assets/images/watermark.png",
    );
    let src = &images["sources"][0];
    let s0 = new_res(src[0].as_str().unwrap(), src[1].as_str().unwrap());
    let ops = images["ops"].as_array().unwrap();
    let a = s0.resize("300x240").unwrap();
    let w = wm.resize("300x240").unwrap();
    let f = nh_images::filters::Filters
        .overlay(w.clone(), &Value::int(0), &Value::int(0))
        .unwrap();
    let b = a.filter(&[Value::object(f)]).unwrap();
    let c = b.resize("300x240 webp").unwrap();
    let ctx = nh_tpl::template::TplContext::default();
    for (r, want) in [(&a, &ops[0]), (&w, &ops[1]), (&b, &ops[2]), (&c, &ops[3])] {
        let content = r.content(ctx.as_host()).unwrap();
        let bytes = content.as_go_string().unwrap().as_bytes().to_vec();
        assert_eq!(J::String(sha(&bytes)), want["sha"], "{}", want["out"]);
        assert_eq!(J::String(r.key()), want["key"], "{}", want["out"]);
    }
    assert_eq!(J::String(b.rel_permalink()), ops[2]["relPermalink"]);

    // A tocss step that is not available: Go's fallback reads `<key>.json`/`.content` from
    // `resources/_gen/assets`; the port finds nothing there.
    let res = fixture("resources/synth.json.gz");
    let rd = res["records"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["where"] == "scss/website.scss")
        .unwrap()["rd"]
        .clone();
    let mut d = ResourceSourceDescriptor {
        open_read_seek_closer: Some(open_file(root.join(rd["file"].as_str().unwrap()))),
        name_normalized: rd["nameNormalized"].as_str().unwrap().to_string(),
        name_original: rd["nameOriginal"].as_str().unwrap().to_string(),
        target_path: rd["targetPath"].as_str().unwrap().to_string(),
        lazy_publish: true,
        ..Default::default()
    };
    let scss = spec.new_resource_adapter(&mut d).unwrap();
    let t = scss
        .transform(vec![
            nh_resources::resource::new_feature_not_available_transformer("tocss", vec![]),
        ])
        .unwrap();
    let err = t.content(ctx.as_host()).unwrap_err();
    assert!(
        err.message().starts_with("TOCSS: failed to transform \"/scss/website.scss\" (text/x-scss): this feature is not available"),
        "{}",
        err.message()
    );
    assert_eq!(
        Resource::rel_permalink(t.as_ref()),
        "/sub/scss/website.scss"
    );

    {
        let touched = touched.lock().unwrap();
        assert!(
            touched.is_empty(),
            "resources/_gen was accessed: {touched:?}"
        );
    }

    // Control: a read through the images file cache does go through the wrapped file system.
    let fc = spec.common.file_caches.get("images").unwrap();
    let (_, b) = fc
        .get_bytes(&ops[1]["key"].as_str().unwrap().rsplit_once('_').unwrap().0[1..])
        .unwrap();
    assert_eq!(b.as_deref(), Some(&b"POISON: not an image"[..]));
    assert!(
        !touched.lock().unwrap().is_empty(),
        "the control read was not seen"
    );
}
