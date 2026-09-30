//! Test support: the oracle's synthetic images, source paths, PSNR, an in-memory sink.

use std::collections::BTreeMap;
use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use image::{
    DynamicImage, ExtendedColorType, ImageBuffer, ImageEncoder, Luma, Rgb, Rgba, RgbaImage,
    codecs::jpeg::JpegEncoder, codecs::png::PngEncoder,
};
use neohugo_base::Sink;
use neohugo_base::paths::OutputPath;
use neohugo_testkit::fixture::rust_dir;

/// The repository root (the Go sources and Hugo's own test images live there).
pub fn repo_dir() -> PathBuf {
    rust_dir().join("..")
}

/// One table of `crates/images/expected_diffs.toml`: case or rule → reason.
pub fn expected_diffs(section: &str) -> BTreeMap<String, String> {
    let path = rust_dir().join("crates/images/expected_diffs.toml");
    let text = std::fs::read_to_string(&path).expect("expected_diffs.toml");
    let doc: toml::Table = text.parse().expect("expected_diffs.toml is TOML");
    doc.get(section)
        .and_then(toml::Value::as_table)
        .map(|t| {
            t.iter()
                .map(|(k, v)| (k.clone(), v.as_str().unwrap_or_default().to_owned()))
                .collect()
        })
        .unwrap_or_default()
}

/// The Go standard library's `src/` for the oracle sources taken from Go's image test data:
/// `NEOHUGO_GOROOT` or `GOROOT`, when set and present.
fn goroot_src() -> Option<PathBuf> {
    ["NEOHUGO_GOROOT", "GOROOT"]
        .into_iter()
        .filter_map(std::env::var_os)
        .map(|r| PathBuf::from(r).join("src"))
        .find(|p| p.is_dir())
}

/// The file of an oracle source id (`file:<path as the oracle saw it>`), when available.
pub fn source_path(id: &str) -> Option<PathBuf> {
    let rel = id.strip_prefix("file:")?;
    let map = [
        (
            "crates/go-image/tests/fixtures/site/",
            "rust/testdata/site-assets/site/",
        ),
        (
            "crates/go-png/tests/fixtures/golden/",
            "rust/testdata/site-assets/golden/",
        ),
        (
            "crates/go-png/tests/fixtures/repo/",
            "rust/testdata/site-assets/repo/",
        ),
    ];
    for (from, to) in map {
        if let Some(rest) = rel.strip_prefix(from) {
            return Some(repo_dir().join(to).join(rest)).filter(|p| p.is_file());
        }
    }
    let go = [
        (
            "crates/go-image/tests/fixtures/gotestdata/",
            "image/testdata/",
        ),
        (
            "crates/go-png/tests/fixtures/gotestdata/image/",
            "image/testdata/",
        ),
        (
            "crates/go-png/tests/fixtures/gotestdata/",
            "image/png/testdata/",
        ),
    ];
    for (from, to) in go {
        if let Some(rest) = rel.strip_prefix(from) {
            return goroot_src()
                .map(|g| g.join(to).join(rest))
                .filter(|p| p.is_file());
        }
    }
    Some(repo_dir().join(rel)).filter(|p| p.is_file())
}

// ---------------------------------------------------------------------------------------
// The oracle's synthetic images (tools/go-oracle/nh-images/process/main.go).

fn splitmix64(x: u64) -> u64 {
    let x = x.wrapping_add(0x9e37_79b9_7f4a_7c15);
    let mut z = x;
    z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    z ^ (z >> 31)
}

fn pix(seed: u64, w: u32, x: u32, y: u32) -> [u8; 8] {
    let h = splitmix64(
        seed.wrapping_mul(1_000_003)
            .wrapping_add(u64::from(y) * u64::from(w) + u64::from(x)),
    );
    let mut v = [0u8; 8];
    for (c, out) in (0u64..).zip(v.iter_mut()) {
        let n = u64::from(x) * (c + 1) * 3 + u64::from(y) * (c + 2) * 5 + ((h >> (8 * c)) & 0x1f);
        *out = n.to_le_bytes()[0];
    }
    v
}

fn kind_seed(kind: &str) -> u64 {
    kind.bytes()
        .fold(0u64, |s, c| s.wrapping_mul(131).wrapping_add(u64::from(c)))
}

fn alpha(v: u8) -> u8 {
    match v {
        0..40 => 0,
        201.. => 255,
        _ => v,
    }
}

/// The encoded synthetic source `gen:<kind>:<w>x<h>:<png|jpg>`. PNG sources decode to the
/// oracle's pixels exactly; JPEG sources only have its size (Go's encoder differs).
pub fn synth(id: &str) -> Option<Vec<u8>> {
    let mut parts = id.strip_prefix("gen:")?.split(':');
    let kind = parts.next()?;
    let (w, h) = parts.next()?.split_once('x')?;
    let (w, h): (u32, u32) = (w.parse().ok()?, h.parse().ok()?);
    let enc = parts.next()?;
    let seed = kind_seed(kind)
        .wrapping_add(u64::from(w) * 7919)
        .wrapping_add(u64::from(h));
    let hi = |a: u8, b: u8| u16::from(a) << 8 | u16::from(b);
    let img: DynamicImage = match kind {
        "nrgba" | "nrgbaa" => DynamicImage::ImageRgba8(RgbaImage::from_fn(w, h, |x, y| {
            let v = pix(seed, w, x, y);
            let a = if kind == "nrgbaa" { alpha(v[3]) } else { 255 };
            Rgba([v[0], v[1], v[2], a])
        })),
        "rgba" => DynamicImage::ImageRgb8(ImageBuffer::from_fn(w, h, |x, y| {
            let v = pix(seed, w, x, y);
            Rgb([v[0], v[1], v[2]])
        })),
        "gray" => DynamicImage::ImageLuma8(ImageBuffer::from_fn(w, h, |x, y| {
            Luma([pix(seed, w, x, y)[0]])
        })),
        "gray16" => DynamicImage::ImageLuma16(ImageBuffer::from_fn(w, h, |x, y| {
            let v = pix(seed, w, x, y);
            Luma([hi(v[0], v[1])])
        })),
        "nrgba64" => DynamicImage::ImageRgba16(ImageBuffer::from_fn(w, h, |x, y| {
            let v = pix(seed, w, x, y);
            Rgba([
                hi(v[0], v[4]),
                hi(v[1], v[5]),
                hi(v[2], v[6]),
                hi(alpha(v[3]), v[7]),
            ])
        })),
        "rgba64" => DynamicImage::ImageRgb16(ImageBuffer::from_fn(w, h, |x, y| {
            let v = pix(seed, w, x, y);
            Rgb([hi(v[0], v[4]), hi(v[1], v[5]), hi(v[2], v[6])])
        })),
        "paletted" => {
            let palette: Vec<[u8; 4]> = (0..40u32)
                .map(|i| {
                    let v = pix(seed, 1, i, 0);
                    let a = if i % 3 == 0 { alpha(v[3]) } else { 255 };
                    [v[0], v[1], v[2], a]
                })
                .collect();
            DynamicImage::ImageRgba8(RgbaImage::from_fn(w, h, |x, y| {
                Rgba(palette[usize::from(pix(seed, w, x, y)[0] % 40)])
            }))
        }
        "ycbcr444" | "ycbcr420" => DynamicImage::ImageRgb8(ImageBuffer::from_fn(w, h, |x, y| {
            let v = pix(seed, w, x, y);
            let [yy, cb, cr] = [v[0], v[1], v[2]].map(f32::from);
            let r = yy + 1.402 * (cr - 128.0);
            let g = yy - 0.344_136 * (cb - 128.0) - 0.714_136 * (cr - 128.0);
            let b = yy + 1.772 * (cb - 128.0);
            Rgb([r, g, b].map(|c| c.round().clamp(0.0, 255.0) as u8))
        })),
        _ => return None,
    };
    let mut out = Vec::new();
    if enc == "png" {
        img.write_to(&mut Cursor::new(&mut out), image::ImageFormat::Png)
            .ok()?;
    } else {
        let rgb = img.to_rgb8();
        JpegEncoder::new_with_quality(&mut out, 90)
            .write_image(rgb.as_raw(), w, h, ExtendedColorType::Rgb8)
            .ok()?;
    }
    Some(out)
}

/// Writes a PNG of `img` (test helper).
pub fn png(img: &RgbaImage) -> Vec<u8> {
    let mut out = Vec::new();
    PngEncoder::new(&mut out)
        .write_image(
            img.as_raw(),
            img.width(),
            img.height(),
            ExtendedColorType::Rgba8,
        )
        .expect("encode png");
    out
}

/// Decodes an encoded image to RGBA.
pub fn decode(bytes: &[u8]) -> RgbaImage {
    image::load_from_memory(bytes).expect("decode").into_rgba8()
}

/// PSNR in dB of two same-size images, over RGB composited on black and on white (the lower
/// of the two) so that alpha differences count. `f64::INFINITY` for identical images.
pub fn psnr(a: &RgbaImage, b: &RgbaImage) -> f64 {
    assert_eq!(a.dimensions(), b.dimensions(), "PSNR needs equal sizes");
    let mut worst = f64::INFINITY;
    for bg in [0.0f64, 255.0] {
        let mut se = 0.0f64;
        for (p, q) in a.pixels().zip(b.pixels()) {
            let (pa, qa) = (f64::from(p.0[3]) / 255.0, f64::from(q.0[3]) / 255.0);
            for i in 0..3 {
                let x = f64::from(p.0[i]) * pa + bg * (1.0 - pa);
                let y = f64::from(q.0[i]) * qa + bg * (1.0 - qa);
                se += (x - y) * (x - y);
            }
        }
        let n = f64::from(a.width()) * f64::from(a.height()) * 3.0;
        let mse = se / n;
        let v = if mse == 0.0 {
            f64::INFINITY
        } else {
            10.0 * (255.0 * 255.0 / mse).log10()
        };
        worst = worst.min(v);
    }
    worst
}

/// A sink that keeps the files in memory.
#[derive(Default)]
pub struct MemorySink {
    pub files: Mutex<BTreeMap<OutputPath, Vec<u8>>>,
}

impl Sink for MemorySink {
    fn write(&self, path: &OutputPath, bytes: &[u8]) -> std::io::Result<()> {
        self.files
            .lock()
            .expect("sink lock")
            .insert(path.clone(), bytes.to_vec());
        Ok(())
    }

    fn exists(&self, path: &OutputPath) -> bool {
        self.files.lock().expect("sink lock").contains_key(path)
    }
}

/// Writes `bytes` to `dir/name` and returns the path.
pub fn write_file(dir: &Path, name: &str, bytes: &[u8]) -> PathBuf {
    let p = dir.join(name);
    std::fs::write(&p, bytes).expect("write test file");
    p
}
