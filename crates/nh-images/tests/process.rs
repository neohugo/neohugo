//! Differential tests against `tools/go-oracle/nh-images/process` (fixtures
//! `tests/fixtures/process/{process,bytes}.json.gz`, from the linux/arm64 oracle under qemu):
//! decoding, DecodeImageConfig + ApplyFiltersFromConfig, ImageProcessor.Filter with the
//! images.* filters, EncodeTo (JPEG, PNG, WebP) and the seeksnack template chains, over the
//! repository's raster images and synthetic images. Every output must be byte-identical.

mod common;

use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

use common::{fixture, repo_root};
use go_image::color::{self, Color};
use go_image::{
    Gray, Gray16, NRGBA, NRGBA64, Paletted, RGBA, RGBA64, YCbCr, YCbCrSubsampleRatio, rect,
};
use go_value::{Map, MapType, Value};
use nh_config::namespace::ConfigNamespace;
use nh_images::config::{self, ImagingConfig, ImagingConfigInternal};
use nh_images::filters::{FilterObject, Filters, filters_key};
use nh_images::image::{self, Format, GiftFilter, GoImage, ImageProcessor, ImageSource};
use serde_json::{Value as J, json};
use sha2::{Digest, Sha256};

type Ns = ConfigNamespace<ImagingConfig, ImagingConfigInternal>;

fn sha(b: &[u8]) -> String {
    Sha256::digest(b)
        .iter()
        .map(|x| format!("{x:02x}"))
        .collect()
}

// ---------------------------------------------------------------------------
// Synthetic sources (the oracle's generator)

fn splitmix64(x: u64) -> u64 {
    let x = x.wrapping_add(0x9e3779b97f4a7c15);
    let mut z = x;
    z = (z ^ (z >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94d049bb133111eb);
    z ^ (z >> 31)
}

fn pix(seed: u64, w: i64, x: i64, y: i64) -> [u8; 8] {
    let h = splitmix64(
        seed.wrapping_mul(1000003)
            .wrapping_add((y as u64).wrapping_mul(w as u64))
            .wrapping_add(x as u64),
    );
    let mut v = [0u8; 8];
    for (c, vc) in v.iter_mut().enumerate() {
        let c = c as i64;
        *vc = (x * (c + 1) * 3 + y * (c + 2) * 5 + ((h >> (8 * c)) & 0x1f) as i64) as u8;
    }
    v
}

fn kind_seed(kind: &str) -> u64 {
    let mut s: u64 = 0;
    for c in kind.chars() {
        s = s.wrapping_mul(131).wrapping_add(c as u64);
    }
    s
}

fn alpha(v: u8) -> u8 {
    if v < 40 {
        0
    } else if v > 200 {
        255
    } else {
        v
    }
}

fn synth(kind: &str, w: i64, h: i64) -> Box<dyn go_image::Image> {
    let r = rect(0, 0, w, h);
    let seed = kind_seed(kind)
        .wrapping_add((w as u64).wrapping_mul(7919))
        .wrapping_add(h as u64);
    let each = |f: &mut dyn FnMut(i64, i64, [u8; 8])| {
        for y in 0..h {
            for x in 0..w {
                f(x, y, pix(seed, w, x, y));
            }
        }
    };
    match kind {
        "nrgba" | "nrgbaa" => {
            let mut m = NRGBA::new(r);
            each(&mut |x, y, v| {
                let a = if kind == "nrgbaa" { alpha(v[3]) } else { 255 };
                m.set_nrgba(
                    x,
                    y,
                    color::NRGBA {
                        r: v[0],
                        g: v[1],
                        b: v[2],
                        a,
                    },
                );
            });
            Box::new(m)
        }
        "rgba" => {
            let mut m = RGBA::new(r);
            each(&mut |x, y, v| {
                m.set_rgba(
                    x,
                    y,
                    color::RGBA {
                        r: v[0],
                        g: v[1],
                        b: v[2],
                        a: 255,
                    },
                )
            });
            Box::new(m)
        }
        "gray" => {
            let mut m = Gray::new(r);
            each(&mut |x, y, v| m.set_gray(x, y, color::Gray { y: v[0] }));
            Box::new(m)
        }
        "gray16" => {
            let mut m = Gray16::new(r);
            each(&mut |x, y, v| {
                m.set_gray16(
                    x,
                    y,
                    color::Gray16 {
                        y: (v[0] as u16) << 8 | v[1] as u16,
                    },
                )
            });
            Box::new(m)
        }
        "nrgba64" => {
            let mut m = NRGBA64::new(r);
            each(&mut |x, y, v| {
                m.set_nrgba64(
                    x,
                    y,
                    color::NRGBA64 {
                        r: (v[0] as u16) << 8 | v[4] as u16,
                        g: (v[1] as u16) << 8 | v[5] as u16,
                        b: (v[2] as u16) << 8 | v[6] as u16,
                        a: (alpha(v[3]) as u16) << 8 | v[7] as u16,
                    },
                )
            });
            Box::new(m)
        }
        "rgba64" => {
            let mut m = RGBA64::new(r);
            each(&mut |x, y, v| {
                m.set_rgba64(
                    x,
                    y,
                    color::RGBA64 {
                        r: (v[0] as u16) << 8 | v[4] as u16,
                        g: (v[1] as u16) << 8 | v[5] as u16,
                        b: (v[2] as u16) << 8 | v[6] as u16,
                        a: 0xffff,
                    },
                )
            });
            Box::new(m)
        }
        "paletted" => {
            let mut p = Vec::new();
            for i in 0..40 {
                let v = pix(seed, 1, i, 0);
                if i % 3 == 0 {
                    p.push(Color::NRGBA(color::NRGBA {
                        r: v[0],
                        g: v[1],
                        b: v[2],
                        a: alpha(v[3]),
                    }));
                } else {
                    p.push(Color::RGBA(color::RGBA {
                        r: v[0],
                        g: v[1],
                        b: v[2],
                        a: 255,
                    }));
                }
            }
            let mut m = Paletted::new(r, color::Palette(p));
            each(&mut |x, y, v| m.set_color_index(x, y, v[0] % 40));
            Box::new(m)
        }
        "ycbcr444" | "ycbcr420" => {
            let ratio = if kind == "ycbcr420" {
                YCbCrSubsampleRatio::Ratio420
            } else {
                YCbCrSubsampleRatio::Ratio444
            };
            let mut m = YCbCr::new(r, ratio);
            for y in 0..h {
                for x in 0..w {
                    let v = pix(seed, w, x, y);
                    let yi = m.y_offset(x, y) as usize;
                    m.y[yi] = v[0];
                    let ci = m.c_offset(x, y) as usize;
                    m.cb[ci] = v[1];
                    m.cr[ci] = v[2];
                }
            }
            Box::new(m)
        }
        _ => panic!("unknown kind {kind}"),
    }
}

fn encode_synth(kind: &str, w: i64, h: i64, enc: &str) -> Vec<u8> {
    let m = synth(kind, w, h);
    let mut buf = Vec::new();
    if enc == "png" {
        go_png::encode(&mut buf, &*m).unwrap();
    } else {
        go_image::jpeg::encode(
            &mut buf,
            &*m,
            Some(&go_image::jpeg::Options { quality: 90 }),
        )
        .unwrap();
    }
    buf
}

/// The encoded bytes of a source id.
fn source_bytes(id: &str) -> Vec<u8> {
    if let Some(path) = id.strip_prefix("file:") {
        return std::fs::read(repo_root().join(path)).unwrap();
    }
    let parts: Vec<&str> = id.split(':').collect();
    assert_eq!(parts[0], "gen");
    let (w, h) = parts[2].split_once('x').unwrap();
    encode_synth(parts[1], w.parse().unwrap(), h.parse().unwrap(), parts[3])
}

// ---------------------------------------------------------------------------
// The oracle's operations

struct Env {
    cfgs: BTreeMap<&'static str, Arc<Ns>>,
    procs: BTreeMap<&'static str, Arc<ImageProcessor>>,
    wm: Vec<u8>,
    mask: Vec<u8>,
}

fn env() -> Env {
    let mut cfgs = BTreeMap::new();
    let mut procs = BTreeMap::new();
    let mut add = |name: &'static str, m: Map| {
        let ns = Arc::new(config::decode_config(&m).unwrap());
        procs.insert(name, ImageProcessor::new(ns.clone()).unwrap());
        cfgs.insert(name, ns);
    };
    add("d", Map::new(MapType::StringAny));
    let mut s = Map::new(MapType::StringAny);
    let mut exif = Map::new(MapType::StringAny);
    exif.insert("_merge", Value::string("none"));
    exif.insert("excludefields", Value::string(".*"));
    s.insert("_merge", Value::string("none"));
    s.insert("exif", Value::map(exif));
    add("s", s);
    let mut q0 = Map::new(MapType::StringAny);
    q0.insert("quality", Value::int(0));
    add("q0", q0);
    let mut lz = Map::new(MapType::StringAny);
    lz.insert("resampleFilter", Value::string("lanczos"));
    lz.insert("anchor", Value::string("topleft"));
    lz.insert("bgColor", Value::string("#abc123"));
    add("lz", lz);
    Env {
        cfgs,
        procs,
        wm: encode_synth("nrgbaa", 600, 480, "png"),
        mask: std::fs::read(repo_root().join("resources/testdata/mask.png")).unwrap(),
    }
}

/// The oracle's `bytesSource`.
struct BytesSource {
    key: String,
    b: Vec<u8>,
}

impl ImageSource for BytesSource {
    fn decode_image(&self) -> nh_common::Result<GoImage> {
        image::decode(&mut &self.b[..])
    }
    fn key(&self) -> String {
        self.key.clone()
    }
}

/// An output: the encoded bytes and the image, or the error text.
type Outcome = Result<(Vec<u8>, GoImage), String>;

fn describe(o: &Outcome) -> J {
    match o {
        Err(e) => json!({"err": e}),
        Ok((b, img)) => json!({
            "type": img.go_type_name(),
            "bounds": img.bounds().to_string(),
            "len": b.len(),
            "sha": sha(b),
        }),
    }
}

fn fields(s: &str) -> Vec<String> {
    go_unicode::strings::fields_str(s)
        .into_iter()
        .map(|s| s.to_string())
        .collect()
}

fn encode(conf: &config::ImageConfig, img: GoImage) -> Outcome {
    let mut buf = Vec::new();
    image::encode_to(conf, &img, &mut buf).map_err(|e| e.message().to_string())?;
    Ok((buf, img))
}

fn spec_op(env: &Env, src: &[u8], format: Format, op: &str) -> Outcome {
    let (name, spec) = op.split_once(':').unwrap();
    let conf = config::decode_image_config(&fields(spec), &env.cfgs[name], format)
        .map_err(|e| e.message().to_string())?;
    let img = image::decode(&mut &src[..]).map_err(|e| e.message().to_string())?;
    let converted = env.procs[name]
        .apply_filters_from_config(&img, &conf)
        .map_err(|e| e.message().to_string())?;
    encode(&conf, converted)
}

fn filter_op(
    env: &Env,
    cfg: &str,
    src: &[u8],
    format: Format,
    fs: &[FilterObject],
) -> (Outcome, String) {
    let mut options = Vec::new();
    for f in fs {
        if let GiftFilter::Process(p) = &f.filter {
            options.extend(fields(&p.spec));
        }
    }
    let mut conf_main = match config::decode_image_config(&options, &env.cfgs[cfg], format) {
        Ok(c) => c,
        Err(e) => return (Err(e.message().to_string()), String::new()),
    };
    conf_main.action = "filter".into();
    conf_main.key = filters_key(fs);
    let key = conf_main.key.clone();
    let run = || -> Outcome {
        let img = image::decode(&mut &src[..]).map_err(|e| e.message().to_string())?;
        let mut filters = Vec::new();
        for f in fs {
            if let GiftFilter::Process(p) = &f.filter {
                let conf = config::decode_image_config(&fields(&p.spec), &env.cfgs[cfg], format)
                    .map_err(|e| e.message().to_string())?;
                filters.extend(
                    env.procs[cfg]
                        .filters_from_config(&img, &conf)
                        .map_err(|e| e.message().to_string())?,
                );
            } else {
                filters.push(f.filter.clone());
            }
        }
        let converted = env.procs[cfg]
            .filter(&img, &filters)
            .map_err(|e| format!("panic: {}", e.message()))?;
        encode(&conf_main, converted)
    };
    (run(), key)
}

fn filter_chains(env: &Env) -> Vec<(&'static str, Vec<FilterObject>)> {
    let f = Filters;
    let wm_src: Arc<dyn ImageSource> = Arc::new(BytesSource {
        key: "/images/watermark_hu_3bf49ff914f6e68c.png_6519743917224815147".into(),
        b: env.wm.clone(),
    });
    let mask_src: Arc<dyn ImageSource> = Arc::new(BytesSource {
        key: "/mask.png_1".into(),
        b: env.mask.clone(),
    });
    let i = |v: i64| Value::int(v);
    let fl = |v: f64| Value::float64(v);
    vec![
        (
            "overlay",
            vec![f.overlay(wm_src.clone(), &i(0), &i(0)).unwrap()],
        ),
        (
            "overlay-off",
            vec![f.overlay(wm_src.clone(), &i(10), &i(-5)).unwrap()],
        ),
        ("brightness", vec![f.brightness(&i(30)).unwrap()]),
        ("contrast", vec![f.contrast(&i(-20)).unwrap()]),
        ("gamma", vec![f.gamma(&fl(1.5)).unwrap()]),
        ("blur", vec![f.gaussian_blur(&fl(1.2)).unwrap()]),
        ("gray", vec![f.grayscale().unwrap()]),
        ("hue", vec![f.hue(&i(-45)).unwrap()]),
        ("invert", vec![f.invert().unwrap()]),
        ("pixelate", vec![f.pixelate(&i(5)).unwrap()]),
        ("saturation", vec![f.saturation(&i(50)).unwrap()]),
        ("sepia", vec![f.sepia(&i(70)).unwrap()]),
        ("sigmoid", vec![f.sigmoid(&fl(0.5), &i(7)).unwrap()]),
        (
            "unsharp",
            vec![f.unsharp_mask(&i(1), &fl(0.8), &fl(0.02)).unwrap()],
        ),
        (
            "colorize",
            vec![f.colorize(&i(180), &i(50), &i(30)).unwrap()],
        ),
        (
            "colorbalance",
            vec![f.color_balance(&i(10), &i(-10), &i(20)).unwrap()],
        ),
        ("opacity", vec![f.opacity(&fl(0.6)).unwrap()]),
        ("opacity-over", vec![f.opacity(&fl(1.7)).unwrap()]),
        (
            "padding",
            vec![
                f.padding(&[i(10), i(20), Value::string("#abc123")])
                    .unwrap(),
            ],
        ),
        ("padding-neg", vec![f.padding(&[i(-2)]).unwrap()]),
        ("padding-bad", vec![f.padding(&[i(-1000)]).unwrap()]),
        ("mask", vec![f.mask(mask_src.clone()).unwrap()]),
        (
            "process",
            vec![f.process(&Value::string("resize 50x webp")).unwrap()],
        ),
        (
            "chain",
            vec![
                f.brightness(&i(10)).unwrap(),
                f.process(&Value::string("resize 40x")).unwrap(),
                f.overlay(wm_src.clone(), &i(0), &i(0)).unwrap(),
            ],
        ),
    ]
}

/// The seeksnack template chains (the oracle's `chains`).
fn chains(env: &Env, src: &[u8], format: Format, dims: (i64, i64)) -> HashMap<String, Outcome> {
    let mut out = HashMap::new();
    let step = |input: &[u8], format: Format, spec: &str| {
        spec_op(env, input, format, &format!("s:{spec}"))
    };
    let overlay = |input: &[u8], format: Format, w: &[u8]| {
        let f = Filters
            .overlay(
                Arc::new(BytesSource {
                    key: "wm".into(),
                    b: w.to_vec(),
                }),
                &Value::int(0),
                &Value::int(0),
            )
            .unwrap();
        filter_op(env, "s", input, format, &[f]).0
    };
    for sz in ["600x480", "300x240", "600x200"] {
        let a = step(src, format, &format!("resize {sz}"));
        let w = step(&env.wm, Format::Png, &format!("resize {sz}"));
        out.insert(format!("chain:{sz}:A"), a.clone());
        out.insert(format!("chain:{sz}:W"), w.clone());
        let (Ok((ab, _)), Ok((wb, _))) = (&a, &w) else {
            continue;
        };
        let b = overlay(ab, format, wb);
        out.insert(format!("chain:{sz}:B"), b.clone());
        let Ok((bb, _)) = &b else {
            continue;
        };
        out.insert(
            format!("chain:{sz}:C"),
            step(bb, format, &format!("resize {sz} webp")),
        );
        if sz == "300x240" {
            out.insert(
                format!("chain:{sz}:C600"),
                step(bb, format, "resize 600x480 webp"),
            );
        }
        if sz == "600x480" {
            out.insert(
                format!("chain:{sz}:A-webp"),
                step(ab, format, "resize 600x480 webp"),
            );
        }
    }
    let d = format!("{}x{}", dims.0, dims.1);
    let w = step(&env.wm, Format::Png, &format!("resize {d}"));
    out.insert("chain:render:W".into(), w.clone());
    if let Ok((wb, _)) = &w {
        let b = overlay(src, format, wb);
        out.insert("chain:render:B".into(), b.clone());
        if let Ok((bb, _)) = &b {
            out.insert(
                "chain:render:C".into(),
                step(bb, format, &format!("resize {d} webp")),
            );
        }
    }
    out
}

/// Whether Go's result is one the port reports as unsupported: TIFF/BMP encoding, and
/// smart cropping that reaches the (unported) smartcrop analyzer.
fn unsupported_in_port(op: &str, got: &Outcome) -> bool {
    let Err(e) = got else {
        return false;
    };
    let target_unsupported = ["tif", "tiff", "bmp"]
        .iter()
        .any(|t| op.split(' ').any(|w| w == *t));
    (target_unsupported && e.contains("encoding is not supported"))
        || (e.contains("smart cropping") && (op.contains(" smart") || op.starts_with("d:crop ")))
}

#[test]
fn process_fixture() {
    let fx = fixture("process/process.json.gz");
    let env = env();
    assert_eq!(
        sha(&env.wm),
        fx["wmsha"].as_str().unwrap(),
        "watermark source"
    );
    let chains_tbl = filter_chains(&env);

    let full: HashMap<(String, String), String> = fixture("process/bytes.json.gz")["cases"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| {
            (
                (
                    c["src"].as_str().unwrap().to_string(),
                    c["op"].as_str().unwrap().to_string(),
                ),
                c["b"].as_str().unwrap().to_string(),
            )
        })
        .collect();

    let mut by_src: Vec<(String, Vec<J>)> = Vec::new();
    for c in fx["cases"].as_array().unwrap() {
        let src = c["src"].as_str().unwrap().to_string();
        match by_src.last_mut() {
            Some((s, v)) if *s == src => v.push(c.clone()),
            _ => by_src.push((src, vec![c.clone()])),
        }
    }

    let mut fails: Vec<String> = Vec::new();
    let (mut n, mut unsupported, mut full_checked, mut full_unsupported) = (0, 0, 0, 0);
    for (src_id, cases) in &by_src {
        let src = source_bytes(src_id);
        let head = &cases[0];
        assert_eq!(head["op"], "decode");
        assert_eq!(
            sha(&src),
            head["srcsha"].as_str().unwrap(),
            "{src_id}: source bytes"
        );
        let format = match head["format"].as_i64().unwrap() {
            1 => Format::Jpeg,
            _ => Format::Png,
        };
        let decoded = image::decode(&mut &src[..]);
        let dims = decoded
            .as_ref()
            .map(|i| (i.bounds().dx(), i.bounds().dy()))
            .unwrap_or((0, 0));
        let mut chain_results: Option<HashMap<String, Outcome>> = None;
        for c in cases {
            n += 1;
            let op = c["op"].as_str().unwrap();
            let got: Outcome = if op == "decode" {
                decoded
                    .clone()
                    .map(|img| (src.clone(), img))
                    .map_err(|e| e.message().to_string())
            } else if let Some(name) = op.strip_prefix("filter:") {
                let fs = &chains_tbl.iter().find(|(n, _)| *n == name).unwrap().1;
                let (o, key) = filter_op(&env, "d", &src, format, fs);
                if key != c["key"].as_str().unwrap() {
                    fails.push(format!("{src_id} {op}: key {key} want {}", c["key"]));
                }
                o
            } else if op.starts_with("chain:") {
                let r = chain_results.get_or_insert_with(|| chains(&env, &src, format, dims));
                r.get(op).cloned().unwrap_or_else(|| Err("missing".into()))
            } else {
                spec_op(&env, &src, format, op)
            };
            let want = &c["res"];
            if want.get("err").is_none() && unsupported_in_port(op, &got) {
                unsupported += 1;
                if full.contains_key(&(src_id.clone(), op.to_string())) {
                    full_unsupported += 1;
                }
                continue;
            }
            let d = describe(&got);
            if &d != want {
                fails.push(format!("{src_id} {op}:\n   got  {d}\n   want {want}"));
                continue;
            }
            if let (Some(b64), Ok((b, _))) = (full.get(&(src_id.clone(), op.to_string())), &got) {
                full_checked += 1;
                assert_eq!(&base64_encode(b), b64, "{src_id} {op}: full bytes");
            }
        }
    }
    eprintln!(
        "process: {n} ops over {} sources, {unsupported} unsupported by the port, {full_checked} compared in full, {} differ",
        by_src.len(),
        fails.len()
    );
    if !fails.is_empty() {
        for f in fails.iter().take(40) {
            eprintln!("{f}");
        }
        panic!("{} of {n} ops differ", fails.len());
    }
    assert_eq!(full_checked + full_unsupported, full.len());
}

fn base64_encode(b: &[u8]) -> String {
    const T: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::new();
    for chunk in b.chunks(3) {
        let n = match chunk.len() {
            3 => (chunk[0] as u32) << 16 | (chunk[1] as u32) << 8 | chunk[2] as u32,
            2 => (chunk[0] as u32) << 16 | (chunk[1] as u32) << 8,
            _ => (chunk[0] as u32) << 16,
        };
        out.push(T[(n >> 18) as usize & 63] as char);
        out.push(T[(n >> 12) as usize & 63] as char);
        out.push(if chunk.len() > 1 {
            T[(n >> 6) as usize & 63] as char
        } else {
            '='
        });
        out.push(if chunk.len() > 2 {
            T[n as usize & 63] as char
        } else {
            '='
        });
    }
    out
}

/// Sources in formats the port does not decode give an explicit error.
#[test]
fn unsupported_source_formats() {
    for rel in [
        "resources/testdata/sunset.webp",
        "resources/testdata/giphy.gif",
        "resources/testdata/pix.gif",
    ] {
        let b = std::fs::read(repo_root().join(rel)).unwrap();
        let e = image::decode(&mut &b[..]).expect_err(rel);
        assert!(
            e.message().contains("is not supported"),
            "{rel}: {}",
            e.message()
        );
        let e = image::decode_config(&mut &b[..]).expect_err(rel);
        assert!(
            e.message().contains("is not supported"),
            "{rel}: {}",
            e.message()
        );
    }
}
