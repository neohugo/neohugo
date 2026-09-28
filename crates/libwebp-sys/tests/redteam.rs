//! Red-team differential test against github.com/bep/gowebp@v0.3.0
//! (`tools/go-oracle/libwebp-sys -mode redteam`).
//!
//! Adversarial geometry (sub-images, negative / zero / truncated strides,
//! non-positive and over-size dimensions, allocation failures), Pix lengths
//! around the exact number of bytes libwebp reads, and an option sweep
//! (qualities and presets around and past their bounds, incl. int32/uint32
//! truncation). The Go oracle places Pix between guard bytes and encodes
//! again with other guard contents: `read_outside = 1` means Go read
//! outside Pix, where the Rust port must return `Error::PixOutOfRange`;
//! otherwise the Rust result must equal Go's exactly.
//!
//! Regression: the `strip` cases with more than 16383 columns or rows and
//! a short Pix (Go "failed to encode"; Rust returned `PixOutOfRange`).
//!
//! `LIBWEBP_REDTEAM_TSV=<path>` checks an additional (larger) corpus.

use std::path::Path;

use libwebp_sys::{EncodingOptions, EncodingPreset, Error, Image, PixView, Point, Rectangle};

fn splitmix(s: &mut u64) -> u64 {
    *s = s.wrapping_add(0x9E3779B97F4A7C15);
    let mut z = *s;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
    z ^ (z >> 31)
}

fn fnv64(b: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    for &c in b {
        h ^= c as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}

fn check_line(line: &str) -> Option<String> {
    let f: Vec<&str> = line.split('\t').collect();
    assert_eq!(f.len(), 17, "bad line {line}");
    let n = |i: usize| -> i64 { f[i].parse().unwrap() };
    let hx = |i: usize| -> u64 { u64::from_str_radix(f[i], 16).unwrap() };
    let seed: u64 = f[0].parse().unwrap();
    let (kind, stride, plen) = (f[2], n(7), n(8) as usize);
    let mut s = seed ^ 0x6a09e667f3bcc908;
    let pix: Vec<u8> = (0..plen).map(|_| splitmix(&mut s) as u8).collect();
    if fnv64(&pix) != hx(12) {
        return Some(format!("seed {seed}: pixel generator drift"));
    }
    let v = PixView {
        pix: &pix,
        stride,
        rect: Rectangle {
            min: Point { x: n(3), y: n(4) },
            max: Point { x: n(5), y: n(6) },
        },
    };
    let img = match kind {
        "nrgba" => Image::Nrgba(v),
        "rgba" => Image::Rgba(v),
        _ => Image::Gray(v),
    };
    let o = EncodingOptions {
        quality: n(9),
        encoding_preset: EncodingPreset(n(10)),
        use_sharp_yuv: n(11) != 0,
    };
    let got = libwebp_sys::encode_to_vec(&img, o);
    let read_outside = n(15) != 0;
    let expect = f[16];
    let what = format!("seed {seed} {} {}", f[1], f[2..12].join(" "));
    match (expect, got) {
        ("panic", Err(Error::EmptyPix)) => None,
        (_, Err(Error::PixOutOfRange)) if read_outside => None,
        (_, r) if read_outside => Some(format!(
            "{what}: Go read outside Pix, Rust did not refuse: {:?}",
            r.map(|b| b.len())
        )),
        ("ok", Ok(b)) => {
            if b.len() as i64 != n(13) || fnv64(&b) != hx(14) {
                Some(format!(
                    "{what}: bytes differ (got {} bytes fnv {:016x}, want {} fnv {})",
                    b.len(),
                    fnv64(&b),
                    f[13],
                    f[14]
                ))
            } else {
                None
            }
        }
        (e, Err(err)) if e.strip_prefix("err:") == Some(err.to_string().as_str()) => None,
        (e, Ok(b)) => Some(format!("{what}: want {e}, got {} bytes", b.len())),
        (e, Err(err)) => Some(format!("{what}: want {e}, got error {err}")),
    }
}

fn run_tsv(path: &Path) -> (usize, Vec<String>) {
    let text = std::fs::read_to_string(path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    let lines: Vec<&str> = text
        .lines()
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .collect();
    let next = std::sync::atomic::AtomicUsize::new(0);
    let fails = std::sync::Mutex::new(Vec::new());
    let threads = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4)
        .min(8);
    std::thread::scope(|s| {
        for _ in 0..threads {
            s.spawn(|| {
                loop {
                    let i = next.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    if i >= lines.len() {
                        break;
                    }
                    if let Some(f) = check_line(lines[i]) {
                        fails.lock().unwrap().push(f);
                    }
                }
            });
        }
    });
    let mut f = fails.into_inner().unwrap();
    f.sort();
    (lines.len(), f)
}

#[test]
fn redteam_fixtures_match_go() {
    let path =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/webp-redteam/redteam.tsv");
    let (n, fails) = run_tsv(&path);
    eprintln!("webp redteam: {n} cases, {} differ", fails.len());
    assert!(n >= 3000, "{n}");
    assert!(
        fails.is_empty(),
        "{} of {n} cases differ:\n{}",
        fails.len(),
        fails.join("\n")
    );
}

/// A larger corpus generated outside the repository:
/// `go run ./tools/go-oracle/libwebp-sys -mode redteam -out <dir> -n 200000 -seed 10000000`.
#[test]
fn redteam_external_corpus() {
    let Ok(p) = std::env::var("LIBWEBP_REDTEAM_TSV") else {
        eprintln!("LIBWEBP_REDTEAM_TSV not set; skipping");
        return;
    };
    let (n, fails) = run_tsv(Path::new(&p));
    eprintln!("webp redteam external: {n} cases, {} differ", fails.len());
    assert!(
        fails.is_empty(),
        "{} of {n} cases differ:\n{}",
        fails.len(),
        fails.join("\n")
    );
}
