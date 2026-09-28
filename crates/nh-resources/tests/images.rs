//! The `images` oracle (tools/go-oracle/nh-resources/images, linux/arm64 under qemu): chained
//! image processing through the resource API — Resize -> Filter(images.Overlay watermark) ->
//! RelPermalink (the seeksnack template chains), the webp steps, render-image's filter on the
//! original, Fit/Fill/Crop/Process, JPEG targets with the background fill, PNG targets with
//! Floyd–Steinberg to the source palette, other filters — over repository images. Every step
//! decodes its parent's ENCODED bytes (the in-memory stand-in for Go's file cache) and the
//! overlay decodes the watermark's encoded PNG; the encoded bytes (sha256, full bytes for a
//! few), names, keys, sizes and the published files are compared with Go.

mod support;

use std::collections::HashMap;
use std::sync::Arc;

use base64::Engine;
use go_value::Value;
use nh_images::filters::{FilterObject, Filters};
use nh_images::image::ImageSource;
use nh_resources::resource::ResourceSourceDescriptor;
use nh_resources::transform::ResourceAdapter;
use serde_json::{Value as J, json};
use support::*;

struct Op {
    out: String,
    input: String,
    kind: &'static str,
    spec: String,
    filt: Vec<String>,
    link: bool,
}

fn op(
    out: String,
    input: String,
    kind: &'static str,
    spec: &str,
    filt: &[String],
    link: bool,
) -> Op {
    Op {
        out,
        input,
        kind,
        spec: spec.to_string(),
        filt: filt.to_vec(),
        link,
    }
}

fn filter_of(named: &HashMap<String, Arc<ResourceAdapter>>, spec: &str) -> FilterObject {
    let f = Filters;
    if spec == "grayscale" {
        return f.grayscale().unwrap();
    }
    if spec == "autoorient" {
        return f.auto_orient().unwrap();
    }
    if let Some(s) = spec.strip_prefix("process:") {
        return f.process(&Value::string(s)).unwrap();
    }
    if let Some(s) = spec.strip_prefix("gaussianblur:") {
        return f.gaussian_blur(&Value::string(s)).unwrap();
    }
    let parts: Vec<&str> = spec.strip_prefix("overlay:").unwrap().split(':').collect();
    let src = named[parts[0]].clone() as Arc<dyn ImageSource>;
    let x: i64 = parts[1].parse().unwrap();
    let y: i64 = parts[2].parse().unwrap();
    f.overlay(src, &Value::int(x), &Value::int(y)).unwrap()
}

#[test]
fn images() {
    let fx = fixture("images/images.json.gz");
    assert_eq!(fx["arch"], "arm64");
    let root = repo_root();
    let dir = root.join("crates/nh-resources/tests/fixtures/site");
    let site = load_site(&dir.to_string_lossy(), None);
    let spec = site.specs[0].clone();

    let new_res = |name: &str, file: &str| -> Arc<ResourceAdapter> {
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

    let mut named: HashMap<String, Arc<ResourceAdapter>> = HashMap::new();
    named.insert(
        "W".into(),
        new_res(
            "watermark.png",
            "crates/nh-resources/tests/fixtures/site/assets/images/watermark.png",
        ),
    );

    let mut ops = Vec::new();
    for (i, s) in fx["sources"].as_array().unwrap().iter().enumerate() {
        let sn = format!("S{i}");
        let r = new_res(s[0].as_str().unwrap(), s[1].as_str().unwrap());
        let (w, h) = (r.width().unwrap(), r.height().unwrap());
        named.insert(sn.clone(), r);
        let s = |x: &str| format!("{sn}{x}");
        let v = |x: &[&str]| x.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        let wn = format!("W{w}x{h}");
        ops.extend([
            op(s("A300"), sn.clone(), "resize", "300x240", &[], false),
            op("W300".into(), "W".into(), "resize", "300x240", &[], false),
            op(
                s("B300"),
                s("A300"),
                "filter",
                "",
                &v(&["overlay:W300:0:0"]),
                true,
            ),
            op(s("C300"), s("B300"), "resize", "300x240 webp", &[], true),
            op(s("D600"), s("B300"), "resize", "600x480 webp", &[], true),
            op(s("A600"), sn.clone(), "resize", "600x480", &[], false),
            op("W600".into(), "W".into(), "resize", "600x480", &[], false),
            op(
                s("B600"),
                s("A600"),
                "filter",
                "",
                &v(&["overlay:W600:0:0"]),
                true,
            ),
            op(s("C600"), s("B600"), "resize", "600x480 webp", &[], true),
            op(
                wn.clone(),
                "W".into(),
                "resize",
                &format!("{w}x{h}"),
                &[],
                false,
            ),
            op(
                s("Br"),
                sn.clone(),
                "filter",
                "",
                &[format!("overlay:{wn}:0:0")],
                true,
            ),
            op(
                s("Cr"),
                s("Br"),
                "resize",
                &format!("{w}x{h} webp"),
                &[],
                true,
            ),
            op(s("fit"), sn.clone(), "fit", "200x200", &[], true),
            op(s("fill"), sn.clone(), "fill", "150x100 center", &[], true),
            op(
                s("filltl"),
                sn.clone(),
                "fill",
                "90x120 TopLeft lanczos",
                &[],
                true,
            ),
            op(
                s("crop"),
                sn.clone(),
                "crop",
                "100x100 bottomright",
                &[],
                true,
            ),
            op(s("q50"), sn.clone(), "resize", "x100 q50", &[], true),
            op(s("jpg"), sn.clone(), "resize", "120x jpg", &[], true),
            op(
                s("jpgbg"),
                sn.clone(),
                "resize",
                "120x jpg #ff0000",
                &[],
                true,
            ),
            op(s("png"), sn.clone(), "resize", "90x png", &[], true),
            op(
                s("pngbg"),
                sn.clone(),
                "resize",
                "90x png #00ff00",
                &[],
                true,
            ),
            op(s("r90"), sn.clone(), "resize", "100x r90", &[], true),
            op(
                s("proc"),
                sn.clone(),
                "process",
                "resize 80x webp q60 photo",
                &[],
                true,
            ),
            op(
                s("gray"),
                sn.clone(),
                "filter",
                "",
                &v(&["grayscale"]),
                true,
            ),
            op(
                s("multi"),
                sn.clone(),
                "filter",
                "",
                &v(&["process:fit 50x50", "overlay:W300:5:5", "gaussianblur:1"]),
                true,
            ),
            op(
                s("orient"),
                sn.clone(),
                "filter",
                "",
                &v(&["autoorient"]),
                true,
            ),
            op(s("chain2"), s("fit"), "resize", "100x png", &[], true),
            op(
                s("chain3"),
                s("chain2"),
                "filter",
                "",
                &v(&["overlay:W300:-10:-10"]),
                true,
            ),
        ]);
    }

    let want_ops = fx["ops"].as_array().unwrap();
    assert_eq!(ops.len(), want_ops.len());
    let mut failures = Vec::new();
    let ctx = nh_tpl::template::TplContext::default();
    for (o, want) in ops.iter().zip(want_ops) {
        let what = format!("{} = {} {} {} {:?}", o.out, o.input, o.kind, o.spec, o.filt);
        assert_eq!(J::String(o.out.clone()), want["out"], "{what}");
        let mut rec = serde_json::Map::new();
        for k in ["out", "in", "kind", "spec", "filters", "link"] {
            rec.insert(k.into(), want[k].clone());
        }
        let input = named[&o.input].clone();
        let res = match o.kind {
            "resize" => input.resize(&o.spec),
            "fit" => input.fit(&o.spec),
            "fill" => input.fill(&o.spec),
            "crop" => input.crop(&o.spec),
            "process" => input.process(&o.spec),
            "filter" => {
                let fs: Vec<Value> = o
                    .filt
                    .iter()
                    .map(|s| Value::object(filter_of(&named, s)))
                    .collect();
                input.filter(&fs)
            }
            _ => unreachable!(),
        };
        let r = match res {
            Ok(r) => r,
            Err(e) => {
                rec.insert(
                    "err".into(),
                    json!(e.message().replace(&*root.to_string_lossy(), PLACEHOLDER)),
                );
                failures.extend(diff(&what, want, &J::Object(rec), &[]));
                continue;
            }
        };
        named.insert(o.out.clone(), r.clone());
        rec.insert("key".into(), json!(r.key()));
        rec.insert("name".into(), json!(r.name()));
        rec.insert("mediaType".into(), json!(r.media_type().typ));
        rec.insert("width".into(), json!(r.width().unwrap()));
        rec.insert("height".into(), json!(r.height().unwrap()));
        match r.content(ctx.as_host()) {
            Err(e) => {
                rec.insert("contentErr".into(), json!(e.message()));
            }
            Ok(v) => {
                let b = v.as_go_string().unwrap().as_bytes().to_vec();
                rec.insert("len".into(), json!(b.len()));
                rec.insert("sha".into(), json!(sha(&b)));
                if want.get("bytes").is_some() {
                    rec.insert(
                        "bytes".into(),
                        json!(base64::engine::general_purpose::STANDARD.encode(&b)),
                    );
                }
                // The in-memory "file cache" holds the same encoded bytes.
                let enc = spec
                    .image_cache
                    .encoded_bytes(&r.target().target_path())
                    .expect("encoded bytes in the image cache");
                assert_eq!(enc.as_slice(), b.as_slice(), "{what}: encoded bytes");
            }
        }
        if o.link {
            rec.insert("relPermalink".into(), json!(r.rel_permalink()));
        }
        failures.extend(diff(&what, want, &J::Object(rec), &[]));
    }

    failures.extend(diff(
        "published",
        &fx["published"],
        &site.published_files(),
        &[],
    ));
    assert!(
        failures.is_empty(),
        "{} failures:\n{}",
        failures.len(),
        failures.join("\n")
    );
    eprintln!("images: {} operations identical", ops.len());
}
