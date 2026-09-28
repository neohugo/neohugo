//! Differential tests against `tools/go-oracle/nh-images/exif` (fixture
//! `tests/fixtures/exif/exif.json.gz`, from the linux/arm64 oracle under qemu with TZ=UTC):
//! `exif.Decoder.Decode` over the repository's JPEG/PNG/WebP files, synthetic EXIF blocks in
//! every container, and mutations of those.

mod common;

use std::io::Cursor;

use common::{fbits_enc, fixture, repo_root};
use go_time::GoTimeExt;
use nh_images::exif::{Decoder, DecoderOption, ImageFormat};
use serde_json::{Value as J, json};

fn decoder(name: &str) -> Decoder {
    let default_exclude = "GPS|Exif|Exposure[M|P|B]|Contrast|Resolution|Sharp|JPEG|Metering|Sensing|Saturation|ColorSpace|Flash|WhiteBalance";
    let (include, exclude, disable) = match name {
        "default" => ("", default_exclude, false),
        "none" => ("", ".*", false),
        "all" => (".*", "", false),
        "nodate" => (".*", "", true),
        "incl" => ("Orientation|Date|GPS|Model", "Offset", false),
        _ => panic!("{name}"),
    };
    Decoder::new(&[
        DecoderOption::WithDateDisabled(disable),
        DecoderOption::WithLatLongDisabled(disable),
        DecoderOption::ExcludeFields(exclude.into()),
        DecoderOption::IncludeFields(include.into()),
    ])
    .unwrap()
}

fn base64_decode(s: &str) -> Vec<u8> {
    let val = |c: u8| -> u32 {
        match c {
            b'A'..=b'Z' => (c - b'A') as u32,
            b'a'..=b'z' => (c - b'a' + 26) as u32,
            b'0'..=b'9' => (c - b'0' + 52) as u32,
            b'+' => 62,
            b'/' => 63,
            _ => panic!("bad base64"),
        }
    };
    let b: Vec<u8> = s.bytes().filter(|&c| c != b'=').collect();
    let mut out = Vec::new();
    for chunk in b.chunks(4) {
        let mut n = 0u32;
        for (i, &c) in chunk.iter().enumerate() {
            n |= val(c) << (18 - 6 * i);
        }
        out.push((n >> 16) as u8);
        if chunk.len() > 2 {
            out.push((n >> 8) as u8);
        }
        if chunk.len() > 3 {
            out.push(n as u8);
        }
    }
    out
}

fn format(i: i64) -> ImageFormat {
    match i {
        1 => ImageFormat::Jpeg,
        2 => ImageFormat::Tiff,
        3 => ImageFormat::Png,
        4 => ImageFormat::WebP,
        _ => panic!("{i}"),
    }
}

fn run(d: &Decoder, format: ImageFormat, b: &[u8]) -> J {
    match d.decode("x", format, &mut Cursor::new(b.to_vec())) {
        Err(e) => json!({"err": e.message()}),
        Ok(ex) => {
            let tags: Vec<J> = ex
                .tags
                .iter()
                .map(|(k, v)| {
                    let s = go_fmt::sprintf("%v", std::slice::from_ref(v));
                    json!([k, v.go_type_name(), common_str(&s)])
                })
                .collect();
            json!({
                "lat": fbits_enc(ex.lat),
                "long": fbits_enc(ex.long),
                "date": ex.date.format(go_time::RFC3339_NANO),
                "tags": if tags.is_empty() { J::Null } else { J::Array(tags) },
            })
        }
    }
}

/// goval.Str: the string, or {"hex": ...} when it is not valid UTF-8.
fn common_str(b: &[u8]) -> J {
    match std::str::from_utf8(b) {
        Ok(s) => J::String(s.to_string()),
        Err(_) => json!({"hex": b.iter().map(|x| format!("{x:02x}")).collect::<String>()}),
    }
}

#[test]
fn exif_fixture() {
    go_time::set_local(go_time::utc());
    let fx = fixture("exif/exif.json.gz");
    let names = ["default", "none", "all", "nodate", "incl"];
    let decoders: Vec<Decoder> = names.iter().map(|n| decoder(n)).collect();
    let mut fails = Vec::new();
    let (mut n, mut with_tags, mut errs) = (0, 0, 0);
    for c in fx["cases"].as_array().unwrap() {
        let src = c["src"].as_str().unwrap();
        let b = match c.get("b") {
            Some(b) => base64_decode(b.as_str().unwrap()),
            None => std::fs::read(repo_root().join(src.strip_prefix("file:").unwrap())).unwrap(),
        };
        let f = format(c["format"].as_i64().unwrap());
        for (name, want) in c["res"].as_object().unwrap() {
            n += 1;
            let d = &decoders[names.iter().position(|x| x == name).unwrap()];
            let got = run(d, f, &b);
            if want.get("err").is_some() {
                errs += 1;
            } else if want["tags"].is_array() {
                with_tags += 1;
            }
            if &got != want {
                fails.push(format!("{src} [{name}]:\n   got  {got}\n   want {want}"));
            }
        }
    }
    eprintln!(
        "exif: {n} decodes, {with_tags} with tags, {errs} errors, {} differ",
        fails.len()
    );
    if !fails.is_empty() {
        for f in fails.iter().take(20) {
            eprintln!("{f}");
        }
        panic!("{} of {n} decodes differ", fails.len());
    }
    assert!(with_tags > 100 && errs > 50);
}

/// Hugo's auto-orient filter maps the EXIF orientation to the gift transform.
#[test]
fn auto_orient_from_exif() {
    use nh_images::auto_orient::{AutoOrientFilter, ImageFilterFromOrientationProvider};
    let b = std::fs::read(repo_root().join("resources/testdata/exif/orientation6.jpg")).unwrap();
    let ex = decoder("all")
        .decode("x", ImageFormat::Jpeg, &mut Cursor::new(b))
        .unwrap();
    let f = AutoOrientFilter
        .auto_orient(Some(&ex))
        .expect("orientation 6");
    let f: &dyn std::any::Any = &*f;
    let t = f
        .downcast_ref::<gift::TransformFilter>()
        .expect("transform");
    assert_eq!(format!("{:?}", t.tt), "Rotate270");
    assert!(AutoOrientFilter.auto_orient(None).is_none());
}
