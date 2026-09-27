//! Randomized differential test against github.com/bep/gowebp@v0.3.0.
//!
//! `tools/go-oracle/libwebp-sys -mode fuzz` generates images with a portable
//! splitmix64-driven generator (odd sizes, 1×N / N×1 strips, sub-images with
//! a non-zero `Rect.Min`, padded / short strides, NRGBA / RGBA (valid and
//! invalid premultiplied data) / Gray, seven alpha modes, six pixel patterns)
//! and random options (lossless, lossy 1..100, out-of-range qualities,
//! presets 0..6 and -1, sharp YUV on/off). This file re-implements the
//! generator bit for bit; the fixture stores the case parameters, a hash of
//! the generated input (checked first, so a generator drift is reported as
//! such) and the length + FNV-1a-64 hash of the Go output.
//!
//! `LIBWEBP_FUZZ_TSV=<path>` runs an additional (larger) fuzz.tsv produced
//! by the oracle outside the repository.

use std::path::Path;

use libwebp_sys::{EncodingOptions, EncodingPreset, Image, PixView, Point, Rectangle};

/// splitmix64 (tools/go-oracle/libwebp-sys/fuzz.go `rng`).
struct Rng {
    s: u64,
}

impl Rng {
    fn next(&mut self) -> u64 {
        self.s = self.s.wrapping_add(0x9E3779B97F4A7C15);
        let mut z = self.s;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
        z ^ (z >> 31)
    }

    fn intn(&mut self, n: i64) -> i64 {
        (self.next() % n as u64) as i64
    }

    fn range_in(&mut self, lo: i64, hi: i64) -> i64 {
        lo + self.intn(hi - lo + 1)
    }
}

fn fnv64(b: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    for &c in b {
        h ^= c as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}

#[derive(Debug, PartialEq)]
struct Params {
    kind: &'static str,
    w: i64,
    h: i64,
    mx: i64,
    my: i64,
    stride: i64,
    tail: i64,
    pattern: i64,
    alpha: i64,
    quality: i64,
    preset: i64,
    sharp: bool,
}

fn pixel_chan(pattern: i64, r: &mut Rng, x: i64, y: i64, ch: i64, w: i64, h: i64) -> u8 {
    match pattern {
        0 => r.next() as u8,
        1 => {
            let v = (x * 255) / w.max(1) + (y * 97) / h.max(1) + ch * 40 + r.intn(9);
            v as u8
        }
        2 => {
            const PAL: [[u8; 3]; 5] = [
                [250, 20, 20],
                [10, 40, 230],
                [255, 255, 255],
                [0, 0, 0],
                [30, 200, 90],
            ];
            PAL[(((x / 5) + (y / 3) * 3) % 5) as usize][ch as usize]
        }
        3 => {
            if (x + y * 2) % 7 == 0 || x % 11 == 0 {
                (40 * ch) as u8
            } else {
                (220 - 30 * ch) as u8
            }
        }
        4 => {
            let t = |v: i64| {
                let mut v = v % 128;
                if v > 64 {
                    v = 128 - v;
                }
                v
            };
            (t(x * 3 + ch * 17) * 2 + t(y * 5 + ch * 29) + r.intn(5)) as u8
        }
        _ => {
            if r.intn(50) == 0 {
                return r.next() as u8;
            }
            (128 + ch * 30) as u8
        }
    }
}

fn alpha_chan(mode: i64, r: &mut Rng, x: i64, y: i64, w: i64, _h: i64) -> u8 {
    match mode {
        0 | 1 => 255,
        2 => {
            if (x / 4 + y / 4) % 3 == 0 {
                0
            } else {
                255
            }
        }
        3 => r.next() as u8,
        4 => {
            if r.intn(30) == 0 {
                r.intn(255) as u8
            } else {
                255
            }
        }
        5 => 0,
        _ => (((x * 255) / w.max(1)) ^ (y * 3)) as u8,
    }
}

/// Port of fuzz.go `genFuzzCase`.
fn gen_case(seed: u64) -> (Params, Vec<u8>) {
    let r = &mut Rng { s: seed };
    let kind = match r.intn(10) {
        0..=3 => "nrgba",
        4..=6 => "rgba",
        _ => "gray",
    };
    let (w, h) = match r.intn(20) {
        0 | 1 => {
            if r.intn(2) == 0 {
                (1, r.range_in(1, 300))
            } else {
                (r.range_in(1, 300), 1)
            }
        }
        2 => {
            let w = r.range_in(131, 420);
            (w, r.range_in(131, 320))
        }
        3..=7 => {
            let w = r.range_in(41, 130);
            (w, r.range_in(41, 130))
        }
        _ => {
            let w = r.range_in(1, 40);
            (w, r.range_in(1, 40))
        }
    };
    let (mut mx, mut my) = (0, 0);
    if r.intn(5) == 0 {
        mx = r.range_in(0, 5);
        my = r.range_in(0, 5);
    }
    let bpp = if kind == "gray" { 1 } else { 4 };
    let (max_x, max_y) = (mx + w, my + h);
    let stride = match r.intn(10) {
        0 => w * bpp,
        1 | 2 => max_x * bpp + r.range_in(1, 17),
        _ => max_x * bpp,
    };
    let mut tail = 0;
    if r.intn(4) == 0 {
        tail = r.range_in(1, 9);
    }
    let pattern = r.intn(6);
    let alpha = r.intn(7);
    let q = r.intn(20);
    let quality = if q < 9 {
        75
    } else if q < 15 {
        r.range_in(1, 100)
    } else if q < 17 {
        0
    } else if q < 18 {
        100
    } else {
        [101, -1, 1000, 2][r.intn(4) as usize]
    };
    let pr = r.intn(20);
    let preset = if pr < 8 {
        2
    } else if pr < 19 {
        r.range_in(0, 5)
    } else {
        [6, -1][r.intn(2) as usize]
    };
    let sharp = r.intn(3) != 0;
    let p = Params {
        kind,
        w,
        h,
        mx,
        my,
        stride,
        tail,
        pattern,
        alpha,
        quality,
        preset,
        sharp,
    };

    let need = (max_y - 1) * stride + max_x * bpp;
    let mut buf = vec![0u8; (need + tail) as usize];
    for b in buf.iter_mut() {
        *b = r.next() as u8;
    }
    if kind == "gray" {
        for i in 0..buf.len() {
            let (y, x) = (i as i64 / stride, i as i64 % stride);
            buf[i] = pixel_chan(pattern, r, x, y, 0, w, h);
        }
        return (p, buf);
    }
    let mut i = 0usize;
    while i + 3 < buf.len() {
        let ii = i as i64;
        let (y, x) = (ii / stride, (ii % stride) / 4);
        let mut c = [
            pixel_chan(pattern, r, x, y, 0, w, h),
            pixel_chan(pattern, r, x, y, 1, w, h),
            pixel_chan(pattern, r, x, y, 2, w, h),
            0,
        ];
        c[3] = alpha_chan(alpha, r, x, y, w, h);
        if kind == "rgba" && r.intn(4) != 0 {
            for k in 0..3 {
                c[k] = (c[k] as u32 * c[3] as u32 / 255) as u8;
            }
        }
        buf[i..i + 4].copy_from_slice(&c);
        i += 4;
    }
    (p, buf)
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

fn check_line(line: &str) -> Option<String> {
    let f: Vec<&str> = line.split('\t').collect();
    assert_eq!(f.len(), 15, "bad line {line}");
    let n = |i: usize| -> i64 { f[i].parse().unwrap() };
    let hx = |i: usize| -> u64 { u64::from_str_radix(f[i], 16).unwrap() };
    let seed: u64 = f[0].parse().unwrap();
    let (p, buf) = gen_case(seed);
    let want = (f[1], n(2), n(3), n(4), n(5), n(6), n(7), n(8), n(9) != 0);
    let got = (
        p.kind, p.w, p.h, p.mx, p.my, p.stride, p.quality, p.preset, p.sharp,
    );
    if got != want || buf.len() as i64 != n(10) || fnv64(&buf) != hx(11) {
        return Some(format!(
            "seed {seed}: generator drift: got {got:?} len {} fnv {:016x}, want {want:?} len {} fnv {}",
            buf.len(),
            fnv64(&buf),
            f[10],
            f[11]
        ));
    }
    let v = PixView {
        pix: &buf,
        stride: p.stride,
        rect: Rectangle {
            min: Point { x: p.mx, y: p.my },
            max: Point {
                x: p.mx + p.w,
                y: p.my + p.h,
            },
        },
    };
    let img = match p.kind {
        "nrgba" => Image::Nrgba(v),
        "rgba" => Image::Rgba(v),
        _ => Image::Gray(v),
    };
    let o = EncodingOptions {
        quality: p.quality,
        encoding_preset: EncodingPreset(p.preset),
        use_sharp_yuv: p.sharp,
    };
    let got = libwebp_sys::encode_to_vec(&img, o);
    let expect = f[14];
    match (expect, got) {
        ("ok", Ok(b)) => {
            if b.len() as i64 != n(12) || fnv64(&b) != hx(13) {
                Some(format!(
                    "seed {seed} ({p:?}): bytes differ (got {} bytes fnv {:016x}, want {} fnv {})",
                    b.len(),
                    fnv64(&b),
                    f[12],
                    f[13]
                ))
            } else {
                None
            }
        }
        ("panic", Err(libwebp_sys::Error::EmptyPix)) => None,
        (e, Err(err)) if e.strip_prefix("err:") == Some(err.to_string().as_str()) => None,
        (e, Ok(b)) => Some(format!(
            "seed {seed} ({p:?}): want {e}, got {} bytes",
            b.len()
        )),
        (e, Err(err)) => Some(format!("seed {seed} ({p:?}): want {e}, got error {err}")),
    }
}

#[test]
fn fuzz_fixtures_match_go() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/webp/fuzz.tsv");
    let (n, fails) = run_tsv(&path);
    eprintln!("webp fuzz: {n} cases, {} differ", fails.len());
    assert!(n >= 3000, "{n}");
    assert!(
        fails.is_empty(),
        "{} of {n} cases differ:\n{}",
        fails.len(),
        fails.join("\n")
    );
}

/// A larger corpus generated outside the repository:
/// `go run ./tools/go-oracle/libwebp-sys -mode fuzz -out <dir> -n 100000 -seed 1000000`.
#[test]
fn fuzz_external_corpus() {
    let Ok(p) = std::env::var("LIBWEBP_FUZZ_TSV") else {
        eprintln!("LIBWEBP_FUZZ_TSV not set; skipping");
        return;
    };
    let (n, fails) = run_tsv(Path::new(&p));
    eprintln!("webp external fuzz: {n} cases, {} differ", fails.len());
    assert!(
        fails.is_empty(),
        "{} of {n} cases differ:\n{}",
        fails.len(),
        fails.join("\n")
    );
}
