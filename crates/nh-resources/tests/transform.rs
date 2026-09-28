//! The `transform` oracle (tools/go-oracle/nh-resources/transform, linux/arm64 under qemu): the
//! transformation chain engine over the synthetic site's assets — fingerprint (md5, sha256,
//! sha384, sha512; `.<hex>` target paths and `Data.Integrity`), minify (`.min`, T07's minifier
//! client), `resources.Copy`, failing transformations, links before and after `.Content`
//! (publishing and the transformation cache shared by adapters with the same key),
//! `resources.PostProcess` placeholders, `GetFieldString` and hugolib's post-publish
//! replacement loop, and `slice` / `commonResource.Slice` with the type check of
//! `resources.Concat`.
//!
//! The fingerprint and minify transformations are test-only ports of
//! `resource_transformers/integrity` and `resource_transformers/minifier` (T15's).

mod support;

use std::collections::HashMap;
use std::io::SeekFrom;
use std::sync::Arc;

use base64::Engine;
use go_value::Value;
use nh_common::Result;
use nh_resource::internal::key::ResourceTransformationKey;
use nh_resource::resourcetypes::Resource;
use nh_resources::postpub::postpub::PostPublishResource;
use nh_resources::resource::ResourceSourceDescriptor;
use nh_resources::transform::{ResourceAdapter, ResourceTransformation, ResourceTransformationCtx};
use nh_transform::minifiers::minifiers::Client as MinifyClient;
use serde_json::{Value as J, json};
use sha2::Digest;
use support::*;

/// Test port of integrity's `fingerprintTransformation`.
struct Fingerprint {
    algo: String,
}

enum Hasher {
    Md5(md5::Md5),
    Sha256(sha2::Sha256),
    Sha384(sha2::Sha384),
    Sha512(sha2::Sha512),
}

impl Hasher {
    fn update(&mut self, b: &[u8]) {
        match self {
            Hasher::Md5(h) => h.update(b),
            Hasher::Sha256(h) => h.update(b),
            Hasher::Sha384(h) => h.update(b),
            Hasher::Sha512(h) => h.update(b),
        }
    }
    fn sum(self) -> Vec<u8> {
        match self {
            Hasher::Md5(h) => h.finalize().to_vec(),
            Hasher::Sha256(h) => h.finalize().to_vec(),
            Hasher::Sha384(h) => h.finalize().to_vec(),
            Hasher::Sha512(h) => h.finalize().to_vec(),
        }
    }
}

impl ResourceTransformation for Fingerprint {
    fn key(&self) -> ResourceTransformationKey {
        ResourceTransformationKey::new("fingerprint", vec![Value::string(self.algo.as_str())])
    }

    fn transform(&self, ctx: &mut ResourceTransformationCtx<'_>) -> Result<()> {
        let mut h = match self.algo.as_str() {
            "md5" => Hasher::Md5(md5::Md5::new()),
            "sha256" => Hasher::Sha256(sha2::Sha256::new()),
            "sha384" => Hasher::Sha384(sha2::Sha384::new()),
            "sha512" => Hasher::Sha512(sha2::Sha512::new()),
            a => {
                return Err(nh_common::Error::new(format!(
                    "unsupported hash algorithm: {}, use either md5, sha256, sha384 or sha512",
                    go_strconv::quote(a.as_bytes())
                )));
            }
        };
        if let Some(rs) = ctx.from.as_read_seeker() {
            // This transformation does not change the content, so try to avoid writing to To.
            let mut b = Vec::new();
            let r = rs.read_to_end(&mut b);
            let _ = rs.seek(SeekFrom::Start(0));
            r?;
            h.update(&b);
        } else {
            let b = ctx.from.read_all()?;
            h.update(&b);
            ctx.to.extend_from_slice(&b);
        }
        let d = h.sum();
        let integrity = format!(
            "{}-{}",
            self.algo,
            base64::engine::general_purpose::STANDARD.encode(&d)
        );
        ctx.data.insert("Integrity", Value::string(integrity));
        let hex: String = d.iter().map(|b| format!("{b:02x}")).collect();
        ctx.add_out_path_identifier(&format!(".{hex}"));
        Ok(())
    }
}

/// Test port of the minifier's `minifyTransformation`.
struct Minify {
    m: MinifyClient,
}

impl ResourceTransformation for Minify {
    fn key(&self) -> ResourceTransformationKey {
        ResourceTransformationKey::new("minify", vec![])
    }

    fn transform(&self, ctx: &mut ResourceTransformationCtx<'_>) -> Result<()> {
        ctx.add_out_path_identifier(".min");
        let src = ctx.from.read_all()?;
        let out = self.m.minify(&ctx.in_media_type, &src)?;
        ctx.to.extend_from_slice(&out);
        Ok(())
    }
}

fn gomarshal(v: &Value) -> String {
    String::from_utf8(go_json::marshal(v).unwrap()).unwrap()
}

/// The assets, created like the create client (cached by path: `resources.Get`).
struct Assets {
    site: Site,
    rds: HashMap<String, J>,
    cache: HashMap<String, Arc<ResourceAdapter>>,
}

impl Assets {
    fn get(&mut self, p: &str) -> Arc<ResourceAdapter> {
        if let Some(r) = self.cache.get(p) {
            return r.clone();
        }
        let root = repo_root();
        let rd = &self.rds[p];
        let mut d = ResourceSourceDescriptor {
            open_read_seek_closer: Some(open_file(root.join(rd["file"].as_str().unwrap()))),
            name_normalized: rd["nameNormalized"].as_str().unwrap().to_string(),
            name_original: rd["nameOriginal"].as_str().unwrap().to_string(),
            title: rd["title"].as_str().unwrap().to_string(),
            target_path: rd["targetPath"].as_str().unwrap().to_string(),
            source_filename_or_path: rd["sourceFilenameOrPath"]
                .as_str()
                .unwrap()
                .replace(PLACEHOLDER, &root.to_string_lossy()),
            lazy_publish: rd["lazyPublish"].as_bool().unwrap(),
            ..Default::default()
        };
        let r = self.site.specs[0].new_resource_adapter(&mut d).unwrap();
        self.cache.insert(p.to_string(), r.clone());
        r
    }
}

fn apply(
    assets: &mut Assets,
    m: &MinifyClient,
    a: &str,
    chain: &[J],
) -> Result<Arc<ResourceAdapter>> {
    let mut r = assets.get(a);
    for s in chain {
        let s = s.as_str().unwrap();
        let tr: Arc<dyn ResourceTransformation> = if s == "minify" {
            Arc::new(Minify { m: m.clone() })
        } else if let Some(algo) = s.strip_prefix("fingerprint:") {
            let algo = if algo.is_empty() { "sha256" } else { algo };
            Arc::new(Fingerprint {
                algo: algo.to_string(),
            })
        } else if let Some(p) = s.strip_prefix("copy:") {
            let c = nh_resources::resource::copy(&(r.clone() as Arc<dyn Resource>), p)?;
            r = nh_resources::transform::resource_adapter(&c).unwrap();
            continue;
        } else if let Some(n) = s.strip_prefix("na:") {
            nh_resources::resource::new_feature_not_available_transformer(n, vec![])
        } else {
            panic!("unknown step {s}");
        };
        r = r.transform(vec![tr])?;
    }
    Ok(r)
}

fn field_of(p: &PostPublishResource, f: &str) -> String {
    p.field(f)
}

/// hugolib's postProcess loop (hugolib/hugo_sites_build.go handleFile).
fn replace(content: &[u8], to_post_process: &[Arc<PostPublishResource>]) -> Vec<u8> {
    let prefix = nh_resources::postpub::postpub::POST_PROCESS_PREFIX.as_bytes();
    let suffix = nh_resources::postpub::postpub::POST_PROCESS_SUFFIX.as_bytes();
    let find = |h: &[u8], n: &[u8]| h.windows(n.len()).position(|w| w == n);
    let mut content = content.to_vec();
    let mut k = 0;
    loop {
        let Some(l) = find(&content[k..], prefix) else {
            break;
        };
        let m = find(&content[k + l..], suffix).unwrap() + suffix.len();
        let (low, high) = (k + l, k + l + m);
        let field = String::from_utf8_lossy(&content[low..high]).into_owned();
        let mut forward = l + m;
        for r in to_post_process {
            if let Some(v) = r.get_field_string(&field) {
                let v = v.unwrap();
                let mut c = content[..low].to_vec();
                c.extend_from_slice(v.as_bytes());
                c.extend_from_slice(&content[high..]);
                content = c;
                forward = v.len();
                break;
            }
        }
        k += forward;
    }
    content
}

/// The type switch of tpl/resources `Namespace.Concat`.
fn concat_check(v: &Value) -> String {
    match v {
        Value::List(l) if &*l.ty.go_name() == "resource.Resources" => {
            if l.items.is_empty() {
                "must provide one or more Resource objects to concat".into()
            } else {
                "ok".into()
            }
        }
        v => format!(
            "expected slice of Resource objects, received {} instead",
            v.go_type_name()
        ),
    }
}

#[test]
fn transform() {
    let fx = fixture("transform/transform.json.gz");
    assert_eq!(fx["arch"], "arm64");
    let dir = repo_root().join("crates/nh-resources/tests/fixtures/site");
    let site = load_site(&dir.to_string_lossy(), None);
    let res = fixture("resources/synth.json.gz");
    let mut rds = HashMap::new();
    for r in res["records"].as_array().unwrap() {
        if r["kind"] == "asset" {
            rds.insert(r["where"].as_str().unwrap().to_string(), r["rd"].clone());
        }
    }
    let spec = site.specs[0].clone();
    let m = MinifyClient::new(
        &spec.media_types(),
        &spec.output_formats(),
        spec.path_spec.cfg.as_ref(),
    )
    .unwrap();
    let mut assets = Assets {
        site,
        rds,
        cache: HashMap::new(),
    };

    let mut failures = Vec::new();
    let mut pps: Vec<Arc<PostPublishResource>> = Vec::new();
    let ctx = nh_tpl::template::TplContext::default();
    let cases = fx["cases"].as_array().unwrap();
    for (i, c) in cases.iter().enumerate() {
        let a = c["asset"].as_str().unwrap();
        let what = format!("case {i} {a} {} {}", c["chain"], c["order"]);
        let chain = c["chain"].as_array().unwrap();
        let before = assets.site.published_files();
        let mut o = serde_json::Map::new();
        for k in ["asset", "chain", "order", "chainIndex"] {
            o.insert(k.into(), c[k].clone());
        }
        let r = match apply(&mut assets, &m, a, chain) {
            Ok(r) => r,
            Err(e) => {
                o.insert("applyErr".into(), json!(e.message()));
                failures.extend(diff(&what, c, &J::Object(o), &[]));
                continue;
            }
        };
        o.insert("transformationKey".into(), json!(r.transformation_key()));
        let content = |o: &mut serde_json::Map<String, J>| match r.content(ctx.as_host()) {
            Err(e) => {
                o.insert("contentErr".into(), json!(e.message()));
            }
            Ok(v) => {
                let s = v.as_go_string().unwrap().as_bytes().to_vec();
                if a == "images/watermark.png" {
                    o.insert("contentSha".into(), json!(sha(&s)));
                } else {
                    o.insert("content".into(), json!(String::from_utf8(s).unwrap()));
                }
            }
        };
        let links = |o: &mut serde_json::Map<String, J>| {
            o.insert("relPermalink".into(), json!(r.rel_permalink()));
            o.insert("permalink".into(), json!(r.permalink()));
        };
        if c["order"] == "content-first" {
            content(&mut o);
            links(&mut o);
        } else {
            links(&mut o);
            content(&mut o);
        }
        o.insert("key".into(), json!(r.key()));
        o.insert("name".into(), json!(r.name()));
        o.insert("title".into(), json!(r.title()));
        o.insert("mediaType".into(), json!(r.media_type().typ));
        o.insert("resourceType".into(), json!(r.resource_type()));
        o.insert("data".into(), json!(gomarshal(&r.data())));
        o.insert(
            "params".into(),
            json!(gomarshal(&Value::Map(r.params().unwrap()))),
        );
        let after = assets.site.published_files();
        let mut added = serde_json::Map::new();
        for (k, v) in after.as_object().unwrap() {
            if before.get(k) != Some(v) {
                added.insert(k.clone(), v.clone());
            }
        }
        o.insert("published".into(), J::Object(added));

        if c.get("pp").is_some() {
            let pp = spec.post_process(r.clone() as Arc<dyn Resource>).unwrap();
            let pp2 = spec.post_process(r.clone() as Arc<dyn Resource>).unwrap();
            o.insert("ppSame".into(), json!(Arc::ptr_eq(&pp, &pp2)));
            let mut p = serde_json::Map::new();
            p.insert(
                "content".into(),
                json!(pp.content().unwrap().as_go_string().unwrap().to_str_lossy()),
            );
            p.insert("relPermalink".into(), json!(pp.rel_permalink()));
            p.insert("permalink".into(), json!(pp.permalink()));
            p.insert("name".into(), json!(pp.name()));
            p.insert("title".into(), json!(pp.title()));
            p.insert("resourceType".into(), json!(pp.resource_type()));
            p.insert("data".into(), json!(gomarshal(&pp.data())));
            p.insert("mediaType".into(), json!(gomarshal(&pp.media_type())));
            o.insert("pp".into(), J::Object(p));
            pps.push(pp);
        }
        failures.extend(diff(&what, c, &J::Object(o), &[]));
    }

    // The post-publish replacement.
    let mut doc = String::from("<html>__h_pp_l1 not a field __e= <x>");
    for p in &pps {
        for f in [
            "Content",
            "RelPermalink",
            "Permalink",
            "Name",
            "Title",
            "ResourceType",
            "Data.Integrity",
            "MediaType.Type",
            "MediaType.MainType",
            "MediaType.SubType",
            "MediaType.Delimiter",
            "MediaType.FirstSuffix",
            "MediaType.SuffixesCSV",
            "MediaType.IsText",
            "MediaType.IsHTML",
            "MediaType.IsMarkdown",
            "MediaType.IsZero",
            "MediaType.MarshalJSON",
            "MediaType.String",
            "MediaType.Suffixes",
            "MediaType.NoSuch",
        ] {
            doc.push_str(&format!("[{f}:{}]\n", field_of(p, f)));
        }
    }
    doc.push_str("</html>");
    assert_eq!(J::String(doc.clone()), fx["doc"], "placeholder document");
    let replaced = String::from_utf8(replace(doc.as_bytes(), &pps)).unwrap();
    if J::String(replaced.clone()) != fx["replaced"] {
        failures.push(format!(
            "replaced:\nwant {}\ngot  {replaced}",
            fx["replaced"]
        ));
    }

    // Unknown accessors (Go panics).
    for (i, pc) in fx["panics"].as_array().unwrap().iter().enumerate() {
        let f = pc["field"].as_str().unwrap();
        let got = pps[0].get_field_string(&field_of(&pps[0], f)).unwrap();
        match got {
            Err(e) => assert_eq!(J::String(e.message().to_string()), pc["panic"], "panic {i}"),
            Ok(v) => assert_eq!(J::String(v), pc["value"], "panic {i}"),
        }
    }

    // slice / Slice / Concat's type check.
    let r1 = assets.get("js/a.js");
    let r2 = assets.get("js/b.js");
    let v1 = r1.clone().to_value();
    let v2 = r2.clone().to_value();
    let sl = &fx["slices"];
    let slice_rec = |args: &[Value]| -> J {
        let v = nh_common::collections::slice::slice(ctx.as_host(), args);
        let mut m = serde_json::Map::new();
        m.insert("type".into(), json!(v.go_type_name()));
        m.insert("concat".into(), json!(concat_check(&v)));
        if let Value::List(l) = &v
            && &*l.ty.go_name() == "resource.Resources"
        {
            let names: Vec<String> = l
                .items
                .iter()
                .map(|x| {
                    nh_resource::resourcetypes::resource_from_value_any(x)
                        .unwrap()
                        .name()
                })
                .collect();
            m.insert("names".into(), json!(names));
        }
        J::Object(m)
    };
    let direct = |input: &Value| -> J {
        match nh_resources::resource::common_resource_slice(input) {
            Ok(v) => json!({"type": v.go_type_name()}),
            Err(e) => json!({"type": "<nil>", "err": e.message()}),
        }
    };
    let rs_list = |items: Vec<Value>| {
        Value::list(
            go_value::SliceType::Named(Arc::from("resource.Resources")),
            items,
        )
    };
    let got = json!({
        "two": slice_rec(&[v1.clone(), v2.clone()]),
        "one": slice_rec(std::slice::from_ref(&v1)),
        "mixed": slice_rec(&[v1.clone(), Value::string("x")]),
        "stringFirst": slice_rec(&[Value::string("x"), v1.clone()]),
        "none": slice_rec(&[]),
        "direct-any": direct(&Value::any_list(vec![v1.clone(), v2.clone()])),
        "direct-res": direct(&rs_list(vec![v1.clone()])),
        "direct-bad": direct(&Value::any_list(vec![v1.clone(), Value::int(5)])),
        "direct-str": direct(&Value::string("x")),
        "direct-nil": direct(&Value::Invalid),
        "concat-any": {"concat": concat_check(&Value::any_list(vec![v1.clone(), v2.clone()]))},
        "concat-res": {"concat": concat_check(&rs_list(vec![v1.clone(), v2.clone()]))},
        "concat-str": {"concat": concat_check(&Value::string_list(["a"]))},
    });
    failures.extend(diff("slices", sl, &got, &[]));
    // The resource method table includes Slice (a template reaches it through `slice`).
    assert!(Resource::tpl_has_method(r1.as_ref(), "Slice"));

    failures.extend(diff(
        "published",
        &fx["published"],
        &assets.site.published_files(),
        &[],
    ));

    assert!(
        failures.is_empty(),
        "{} failures:\n{}",
        failures.len(),
        failures.join("\n")
    );
    assert!(cases.len() >= 400, "{}", cases.len());
}
