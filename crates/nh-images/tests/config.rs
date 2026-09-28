//! Differential tests against `tools/go-oracle/nh-images/config` (fixture
//! `tests/fixtures/config/config.json.gz`, from the linux/arm64 oracle under qemu): DecodeConfig,
//! NewImageProcessor, DecodeImageConfig + Reanchor, the formats, the filter keys and the
//! colour helpers.

mod common;

use std::any::Any;
use std::sync::Arc;

use common::{color_desc, color_from_desc, decode, decode_map, fbits_enc, fixture, go_result};
use go_value::{Map, MapType, Value};
use nh_config::namespace::ConfigNamespace;
use nh_images::color;
use nh_images::config::{self, ImagingConfig, ImagingConfigInternal};
use nh_images::filters::{FilterObject, Filters, filters_key};
use nh_images::image::{Format, GoImage, ImageProcessor, ImageSource};
use serde_json::{Value as J, json};

type Ns = ConfigNamespace<ImagingConfig, ImagingConfigInternal>;

fn format(i: i64) -> Format {
    match i {
        1 => Format::Jpeg,
        2 => Format::Png,
        3 => Format::Gif,
        4 => Format::Tiff,
        5 => Format::Bmp,
        6 => Format::Webp,
        _ => panic!("format {i}"),
    }
}

fn resample_name(name: &str) -> J {
    if name.is_empty() {
        return J::Null;
    }
    let r = gift::hugo_resampling::image_filter(name).unwrap();
    let r: &dyn Any = r;
    J::String(r.downcast_ref::<gift::Resamp>().unwrap().name.to_string())
}

fn decode_config_json(ns: &Ns) -> J {
    let c = &ns.config;
    let mut r = json!({
        "hash": ns.source_hash,
        "cfg": {
            "quality": c.imaging.quality,
            "resampleFilter": c.imaging.resample_filter,
            "hint": c.imaging.hint,
            "anchor": c.imaging.anchor,
            "bgColor": c.imaging.bg_color,
            "exif": {
                "includeFields": c.imaging.exif.include_fields,
                "excludeFields": c.imaging.exif.exclude_fields,
                "disableDate": c.imaging.exif.disable_date,
                "disableLatLong": c.imaging.exif.disable_lat_long,
            },
        },
        "internal": {
            "bgColor": color_desc(c.bg_color),
            "hint": c.hint,
            "resample": resample_name(&c.resample_filter),
            "anchor": c.anchor,
        },
        "json": String::from_utf8(ns.marshal_json().unwrap()).unwrap(),
    });
    if let Err(e) = ImageProcessor::new(Arc::new(ns.clone())) {
        r["procErr"] = J::String(e.message().to_string());
    }
    r
}

fn check(name: &str, got: Result<J, String>, want: &J, fails: &mut Vec<String>) {
    let want = go_result(want).cloned();
    if got != want {
        fails.push(format!("{name}:\n  got  {got:?}\n  want {want:?}"));
    }
}

fn finish(fails: Vec<String>, n: usize, what: &str) {
    if !fails.is_empty() {
        for f in fails.iter().take(30) {
            eprintln!("{f}");
        }
        panic!("{what}: {} of {n} cases differ", fails.len());
    }
    eprintln!("{what}: {n} cases OK");
}

fn cases() -> Vec<J> {
    fixture("config/config.json.gz")["cases"]
        .as_array()
        .unwrap()
        .clone()
}

#[test]
fn decode_config_cases() {
    let mut fails = Vec::new();
    let mut n = 0;
    for c in cases().iter().filter(|c| c["kind"] == "decodeconfig") {
        n += 1;
        let input = decode_map(&c["in"]);
        let got = config::decode_config(&input)
            .map(|ns| decode_config_json(&ns))
            .map_err(|e| e.message().to_string());
        check(c["name"].as_str().unwrap(), got, &c["res"], &mut fails);
    }
    assert!(n > 80);
    finish(fails, n, "DecodeConfig");
}

fn image_config_ns(name: &str) -> Ns {
    let mut m = Map::new(MapType::StringAny);
    match name {
        "defaults" => {}
        "seeksnack" => {
            let mut exif = Map::new(MapType::Params);
            exif.insert("_merge", Value::string("none"));
            exif.insert("disabledate", Value::Bool(false));
            exif.insert("disablelatlong", Value::Bool(false));
            exif.insert("excludefields", Value::string(".*"));
            exif.insert("includefields", Value::string(""));
            m = Map::new(MapType::Params);
            m.insert("_merge", Value::string("none"));
            m.insert("exif", Value::map(exif));
        }
        "topleft-lanczos-q90" => {
            m.insert("anchor", Value::string("topleft"));
            m.insert("resampleFilter", Value::string("lanczos"));
            m.insert("quality", Value::int(90));
            m.insert("bgColor", Value::string("#abc123"));
        }
        "quality0" => {
            m.insert("quality", Value::int(0));
        }
        _ => panic!("{name}"),
    }
    config::decode_config(&m).unwrap()
}

#[test]
fn decode_image_config_cases() {
    let names = ["defaults", "seeksnack", "topleft-lanczos-q90", "quality0"];
    let nss: Vec<Ns> = names.iter().map(|n| image_config_ns(n)).collect();
    let mut fails = Vec::new();
    let mut n = 0;
    for c in cases().iter().filter(|c| c["kind"] == "imageconfig") {
        n += 1;
        let ns = &nss[names.iter().position(|x| *x == c["cfg"]).unwrap()];
        let opts: Vec<String> = c["opts"]
            .as_array()
            .unwrap()
            .iter()
            .map(|s| s.as_str().unwrap().to_string())
            .collect();
        let f = format(c["fmt"].as_i64().unwrap());
        let got = config::decode_image_config(&opts, ns, f)
            .map(|conf| {
                json!({
                    "target": conf.target_format.map(|f| f as i64).unwrap_or(0),
                    "action": conf.action,
                    "key": conf.key,
                    "quality": conf.quality,
                    "qualitySet": conf.quality_set_for_image(),
                    "rotate": conf.rotate,
                    "bgColor": color_desc(conf.bg_color),
                    "hint": conf.hint,
                    "width": conf.width,
                    "height": conf.height,
                    "filter": resample_name(&conf.filter),
                    "anchor": conf.anchor,
                    "reanchor": conf.reanchor(gift::CENTER_ANCHOR.0).key,
                })
            })
            .map_err(|e| e.message().to_string());
        check(
            &format!("{} {:?} {}", c["cfg"], opts, f as i64),
            got,
            &c["res"],
            &mut fails,
        );
    }
    assert!(n > 1000);
    finish(fails, n, "DecodeImageConfig");
}

#[test]
fn format_cases() {
    let mut n = 0;
    for c in cases() {
        match c["kind"].as_str().unwrap() {
            "fromext" => {
                let f = config::image_format_from_ext(c["ext"].as_str().unwrap());
                assert_eq!(f.map(|f| f as i64).unwrap_or(0), c["fmt"].as_i64().unwrap());
                assert_eq!(f.is_some(), c["ok"].as_bool().unwrap());
            }
            "fromsubtype" => {
                let f = config::image_format_from_media_sub_type(c["sub"].as_str().unwrap());
                assert_eq!(f.map(|f| f as i64).unwrap_or(0), c["fmt"].as_i64().unwrap());
                assert_eq!(f.is_some(), c["ok"].as_bool().unwrap());
            }
            "format" => {
                let f = format(c["fmt"].as_i64().unwrap());
                assert_eq!(f.default_extension(), c["ext"].as_str().unwrap());
                assert_eq!(f.media_type().typ, c["mediaType"].as_str().unwrap());
                assert_eq!(
                    f.requires_default_quality(),
                    c["defaultQ"].as_bool().unwrap()
                );
                assert_eq!(
                    f.supports_transparency(),
                    c["transparency"].as_bool().unwrap()
                );
                assert_eq!(
                    f.to_image_meta_image_format() as i64,
                    c["imagemeta"].as_i64().unwrap()
                );
            }
            _ => continue,
        }
        n += 1;
    }
    assert_eq!(n, 15 + 9 + 6);
}

/// An ImageSource with a given key (the oracle's `keySource`).
struct KeySource(String);

impl ImageSource for KeySource {
    fn decode_image(&self) -> nh_common::Result<GoImage> {
        Err(nh_common::Error::new("not decodable"))
    }
    fn key(&self) -> String {
        self.0.clone()
    }
}

fn build_filter(fn_name: &str, a: &[Value]) -> nh_common::Result<FilterObject> {
    let f = Filters;
    let key = |v: &Value| String::from_utf8(v.as_go_string().unwrap().to_vec()).unwrap();
    match fn_name {
        "Process" => f.process(&a[0]),
        "Overlay" => f.overlay(Arc::new(KeySource(key(&a[0]))), &a[1], &a[2]),
        "Mask" => f.mask(Arc::new(KeySource(key(&a[0])))),
        "Opacity" => f.opacity(&a[0]),
        "Text" => f.text(&a[0], &a[1..]),
        "Padding" => f.padding(a),
        "Dither" => f.dither(a),
        "AutoOrient" => f.auto_orient(),
        "Brightness" => f.brightness(&a[0]),
        "ColorBalance" => f.color_balance(&a[0], &a[1], &a[2]),
        "Colorize" => f.colorize(&a[0], &a[1], &a[2]),
        "Contrast" => f.contrast(&a[0]),
        "Gamma" => f.gamma(&a[0]),
        "GaussianBlur" => f.gaussian_blur(&a[0]),
        "Grayscale" => f.grayscale(),
        "Hue" => f.hue(&a[0]),
        "Invert" => f.invert(),
        "Pixelate" => f.pixelate(&a[0]),
        "Saturation" => f.saturation(&a[0]),
        "Sepia" => f.sepia(&a[0]),
        "Sigmoid" => f.sigmoid(&a[0], &a[1]),
        "UnsharpMask" => f.unsharp_mask(&a[0], &a[1], &a[2]),
        _ => panic!("{fn_name}"),
    }
}

#[test]
fn filter_key_cases() {
    let mut fails = Vec::new();
    let mut n = 0;
    let mut unsupported = 0;
    // The filters Go built (None: built by Go, unsupported by the port).
    let mut built: Vec<Option<FilterObject>> = Vec::new();
    for c in cases() {
        match c["kind"].as_str().unwrap() {
            "filter" => {
                n += 1;
                let fn_name = c["fn"].as_str().unwrap();
                let args: Vec<Value> = c["args"]
                    .as_array()
                    .map(|a| a.iter().map(decode).collect())
                    .unwrap_or_default();
                let res = build_filter(fn_name, &args);
                if matches!(fn_name, "Text" | "Dither") {
                    // Not supported by the port (PORTING.md): an explicit error.
                    let e = res.err().expect("unsupported filter");
                    assert!(e.message().contains("is not supported"), "{}", e.message());
                    if go_result(&c["res"]).is_ok() {
                        built.push(None);
                    }
                    unsupported += 1;
                    continue;
                }
                let got = res
                    .map(|f| {
                        let r = json!({
                            "key": filters_key(std::slice::from_ref(&f)),
                            "single": nh_common::hashing::hash_string(&[Value::object(f.clone())]),
                        });
                        built.push(Some(f));
                        r
                    })
                    .map_err(|e| e.message().to_string());
                check(
                    &format!("{fn_name} {:?}", c["args"]),
                    got,
                    &c["res"],
                    &mut fails,
                );
            }
            "filterlist" => {
                let idx: Vec<usize> = c["idx"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|i| i.as_u64().unwrap() as usize)
                    .collect();
                let fs: Option<Vec<FilterObject>> = idx.iter().map(|&i| built[i].clone()).collect();
                let Some(fs) = fs else {
                    continue;
                };
                n += 1;
                assert_eq!(filters_key(&fs), c["key"].as_str().unwrap(), "{idx:?}");
            }
            _ => {}
        }
    }
    assert!(unsupported >= 3);
    finish(fails, n, "filter keys");
}

#[test]
fn color_cases() {
    let mut fails = Vec::new();
    let mut n = 0;
    for c in cases() {
        match c["kind"].as_str().unwrap() {
            "color" => {
                n += 1;
                let input = c["in"].as_str().unwrap();
                let got = color::hex_string_to_color(input.as_bytes())
                    .map(|col| {
                        json!({
                            "hex": col.color_hex(),
                            "string": col.string(),
                            "luminance": fbits_enc(col.luminance()),
                            "color": color_desc(Some(col.color_go())),
                            "hash": col.hash().to_string(),
                        })
                    })
                    .map_err(|e| e.message().to_string());
                check(&format!("color {input:?}"), got, &c["res"], &mut fails);
            }
            "luminance" => {
                for v in c["values"].as_array().unwrap() {
                    n += 1;
                    let (hex, bits) = v.as_str().unwrap().split_once('=').unwrap();
                    let b = common::hex(&hex[1..]);
                    let col = color::color_go_to_color(go_image::color::Color::RGBA(
                        go_image::color::RGBA {
                            r: b[0],
                            g: b[1],
                            b: b[2],
                            a: 255,
                        },
                    ));
                    assert_eq!(col.color_hex(), hex);
                    if fbits_enc(col.luminance()) != bits {
                        fails.push(format!(
                            "luminance {hex}: {} want {bits}",
                            fbits_enc(col.luminance())
                        ));
                    }
                }
            }
            "colorgo" => {
                n += 1;
                let gc = color_from_desc(&c["color"]);
                assert_eq!(color_desc(Some(gc)), c["color"]);
                assert_eq!(
                    color::color_go_to_hex_string(gc),
                    c["hex"].as_str().unwrap()
                );
                let col = color::color_go_to_color(gc);
                assert_eq!(
                    fbits_enc(col.luminance()),
                    c["luminance"].as_str().unwrap(),
                    "{gc:?}"
                );
            }
            "palette" => {
                n += 1;
                let pal = go_image::color::Palette(vec![
                    go_image::color::Color::RGBA(go_image::color::RGBA {
                        r: 0,
                        g: 0,
                        b: 0,
                        a: 255,
                    }),
                    go_image::color::Color::RGBA(go_image::color::RGBA {
                        r: 255,
                        g: 255,
                        b: 255,
                        a: 255,
                    }),
                    go_image::color::Color::NRGBA(go_image::color::NRGBA {
                        r: 255,
                        g: 0,
                        b: 0,
                        a: 128,
                    }),
                ]);
                let gc = color_from_desc(&c["color"]);
                let added = color::add_color_to_palette(gc, pal.clone());
                let got: Vec<J> = added.0.iter().map(|x| color_desc(Some(*x))).collect();
                assert_eq!(J::Array(got), c["added"]);
                let mut replaced = pal.clone();
                color::replace_color_in_palette(gc, &mut replaced);
                let got: Vec<J> = replaced.0.iter().map(|x| color_desc(Some(*x))).collect();
                assert_eq!(J::Array(got), c["replaced"]);
            }
            _ => {}
        }
    }
    finish(fails, n, "colors");
}

/// The worked vectors of specs/images.md §4.5.
#[test]
fn spec_vectors() {
    let ns = image_config_ns("seeksnack");
    assert_eq!(ns.source_hash, "4bf645f71319dd1d");
    let key = |spec: &str| {
        let mut opts = vec!["resize".to_string()];
        opts.extend(spec.split(' ').map(|s| s.to_string()));
        config::decode_image_config(&opts, &ns, Format::Jpeg)
            .unwrap()
            .key
    };
    assert_eq!(key("600x480"), "7bfd4638d4eb3be2");
    assert_eq!(key("300x240"), "a08a22b9e9d91e29");
    assert_eq!(key("600x480 webp"), "590f9512b18cac1e");
    assert_eq!(key("640x480 webp"), "73816495dd661eee");
    let f = Filters
        .overlay(
            Arc::new(KeySource(
                "/images/watermark_hu_3bf49ff914f6e68c.png_6519743917224815147".into(),
            )),
            &Value::int(0),
            &Value::int(0),
        )
        .unwrap();
    assert_eq!(filters_key(&[f]), "1682858112077426900");
}
