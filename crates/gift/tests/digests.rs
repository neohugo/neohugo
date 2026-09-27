//! Dense sweeps compared through chunked FNV-1a digests (Go: extra.go):
//!
//! * `mathdigest`: gift::gomath Exp, Log, Pow, Sin, Cos, Sincos over dense
//!   float64 sweeps (tens of millions of inputs).
//! * `kerneldigest`: every resampling kernel over every 64th float32 in
//!   [0, 4.5] (and a coarser negative sweep).
//! * `weights`: 1-row and 1-column NRGBA64 resizes over ~4100 (src, dst)
//!   size pairs with every kernel (prepareResampWeights + resizeLine).
//! * `rotbounds`: Rotate(...).Bounds over 2221 angles and ~1400 sizes.
//!
//! GIFT_MATHDIGEST_BIG / GIFT_KERNELDIGEST_BIG point at larger oracle runs
//! (`mathdigest 64`, `kerneldigest 3`).

mod common;

use std::collections::BTreeMap;
use std::f64;

use common::{ImgSpec, fixtures_dir, gen_image, read_tsv, resampling};
use gift::gomath;
use go_image::{NRGBA64, rect};

const CHUNK: usize = 1 << 20;

/// Go: extra.go:digester (FNV-1a 64 over little-endian u64 values, one
/// digest per chunk of 1<<20 values).
struct Digester {
    name: String,
    h: u64,
    n: usize,
    idx: usize,
    out: Vec<(String, usize, usize, String)>,
}

impl Digester {
    fn new(name: &str) -> Digester {
        Digester {
            name: name.to_string(),
            h: 14695981039346656037,
            n: 0,
            idx: 0,
            out: Vec::new(),
        }
    }
    fn add(&mut self, v: u64) {
        for i in 0..8 {
            self.h ^= (v >> (8 * i)) & 0xff;
            self.h = self.h.wrapping_mul(1099511628211);
        }
        self.n += 1;
        if self.n == CHUNK {
            self.flush();
        }
    }
    fn flush(&mut self) {
        if self.n == 0 {
            return;
        }
        self.out.push((
            self.name.clone(),
            self.idx,
            self.n,
            format!("{:016x}", self.h),
        ));
        self.idx += 1;
        self.h = 14695981039346656037;
        self.n = 0;
    }
    fn finish(mut self) -> Vec<(String, usize, usize, String)> {
        self.flush();
        self.out
    }
}

/// Go: extra.go:sweep64
fn sweep64(lo: f64, hi: f64, n: usize, mut f: impl FnMut(f64)) {
    let (a, b) = (lo.to_bits(), hi.to_bits());
    let step = (b - a) / n as u64;
    for i in 0..n as u64 {
        f(f64::from_bits(a + i * step));
    }
}

/// Go: extra.go:mathDigest
fn math_digest(scale: usize) -> Vec<(String, usize, usize, String)> {
    let n = scale << 20;
    let mut res = Vec::new();

    let mut d = Digester::new("exp");
    sweep64(1e-12, 720.0, 2 * n, |x| d.add(gomath::exp(x).to_bits()));
    sweep64(-1e-12, -750.0, 2 * n, |x| d.add(gomath::exp(x).to_bits()));
    res.extend(d.finish());

    let mut d = Digester::new("log");
    sweep64(1e-300, 1e300, 2 * n, |x| d.add(gomath::log(x).to_bits()));
    sweep64(1e-6, 1e6, 2 * n, |x| d.add(gomath::log(x).to_bits()));
    res.extend(d.finish());

    let mut d = Digester::new("pow");
    let mut ys: Vec<f64> = Vec::new();
    for g in [0.5f32, 1.5, 2.2, 0.1, 3.0, 1e-5, 0.7, 1.3] {
        ys.push((1.0f32 / g.max(1.0e-5)) as f64);
    }
    ys.push(2.4);
    ys.push(f64::from_bits(0x3fdaaaaaaaaaaaab)); // 1/2.4
    for k in 0..22u32 {
        ys.push(f32::from_bits(0x3e000000 + (k.wrapping_mul(0x01234567) % 0x02000000)) as f64);
    }
    let qf16 = 1.0f32 / 65535.0;
    for &y in &ys {
        for k in 0..65536 {
            let x = (k as f32 * qf16) as f64;
            d.add(gomath::pow(x, y).to_bits());
            d.add(gomath::pow(((x as f32 + 0.055) / 1.055) as f64, y).to_bits());
        }
    }
    sweep64(1e-6, 1e6, n, |x| d.add(gomath::pow(x, 0.3).to_bits()));
    res.extend(d.finish());

    for name in ["sin", "cos", "sincos"] {
        let mut d = Digester::new(name);
        let mut f = |x: f64| match name {
            "sin" => d.add(gomath::sin(x).to_bits()),
            "cos" => d.add(gomath::cos(x).to_bits()),
            _ => {
                let (s, c) = gomath::sincos(x);
                d.add(s.to_bits());
                d.add(c.to_bits());
            }
        };
        sweep64(1e-9, 20.0, 2 * n, &mut f);
        sweep64(-1e-9, -20.0, n, &mut f);
        sweep64(20.0, 1e15, n / 4, &mut f);
        res.extend(d.finish());
    }
    res
}

const KERNELS: [&str; 16] = [
    "nearestneighbor",
    "box",
    "linear",
    "cubic",
    "lanczos",
    "hermite",
    "mitchellnetravali",
    "catmullrom",
    "bspline",
    "gaussian",
    "hann",
    "hamming",
    "blackman",
    "bartlett",
    "welch",
    "cosine",
];

/// Go: extra.go:kernelDigest
fn kernel_digest(stride: u32) -> Vec<(String, usize, usize, String)> {
    let mut res = Vec::new();
    for name in KERNELS {
        let r = resampling(name);
        let mut d = Digester::new(name);
        let mut b: u32 = 0;
        while b <= 0x40900000 {
            d.add(r.kernel(f32::from_bits(b)).to_bits() as u64);
            b += stride;
        }
        let mut b: u32 = 0x80000000;
        while b <= 0xc0900000 {
            d.add(r.kernel(f32::from_bits(b)).to_bits() as u64);
            b += stride * 16;
        }
        res.extend(d.finish());
    }
    res
}

/// Go: extra.go:weightPairs
fn weight_pairs() -> Vec<(i64, i64)> {
    let mut ps = Vec::new();
    let mut seen = std::collections::HashSet::new();
    let mut add = |s: i64, d: i64| {
        if s < 1 || d < 1 || !seen.insert((s, d)) {
            return;
        }
        ps.push((s, d));
    };
    for s in 1..=160i64 {
        for d in 1..=12 {
            add(s, d);
        }
        for d in s - 2..=s + 2 {
            add(s, d);
        }
        for d in [
            2 * s,
            3 * s,
            s / 2,
            s / 3,
            2 * s / 3,
            3 * s / 2,
            5 * s / 4,
            4 * s / 5,
        ] {
            add(s, d);
        }
    }
    for s in [250i64, 480, 599, 600, 640, 1000, 1500, 2047] {
        for d in [1i64, 7, 32, 100, 128, 200, 240, 300, 480, 600, 999] {
            add(s, d);
        }
    }
    ps
}

/// Go: extra.go:weightsDigest
fn weights_digest() -> Vec<(String, usize, usize, String)> {
    let pairs = weight_pairs();
    let mut res = Vec::new();
    for name in KERNELS {
        let r = resampling(name);
        let mut d = Digester::new(name);
        for &(s, n) in &pairs {
            for vertical in [false, true] {
                let (sr, dr) = if vertical {
                    (rect(0, 0, 1, s), rect(0, 0, 1, n))
                } else {
                    (rect(0, 0, s, 1), rect(0, 0, n, 1))
                };
                let src = gen_image(&ImgSpec {
                    typ: "nrgba64".to_string(),
                    rect: sr,
                    seed: (s * 7919 + n) as u64,
                    vmode: 0,
                    amode: 2,
                    valid: true,
                    blank: false,
                });
                let mut dst = NRGBA64::new(dr);
                gift::new(vec![gift::resize(dr.dx(), dr.dy(), r)]).draw(&mut dst, src.image());
                for px in dst.pix.chunks_exact(8) {
                    let mut v = 0u64;
                    for &b in px {
                        v = v << 8 | b as u64;
                    }
                    d.add(v);
                }
            }
        }
        res.extend(d.finish());
    }
    res.push(("pairs".to_string(), pairs.len(), 0, String::new()));
    res
}

/// Go: extra.go:rotBoundsDigest (random angles from main.go:rf).
fn rot_bounds_digest() -> Vec<(String, usize, usize, String)> {
    let mut d = Digester::new("rotbounds");
    let mut angles: Vec<f32> = (-360..=360).map(|a| a as f32).collect();
    let mut r = common::Rng::new(99);
    for _ in 0..1500 {
        // Go: main.go:rf(r, -400, 400), inlined: lo + (hi-lo)*u is FMADDS.
        let u = (r.next() % 1000003) as f32 / 1000003.0;
        angles.push(u.mul_add(800.0, -400.0));
    }
    for &a in &angles {
        let f = gift::rotate(
            a,
            go_image::color::Color::Alpha16(go_image::color::Alpha16 { a: 0 }),
            gift::CUBIC_INTERPOLATION,
        );
        for w in 1..=64 {
            let mut h = 1;
            while h <= 64 {
                let b = f.bounds(rect(0, 0, w, h));
                d.add((b.dx() as u64) << 32 | b.dy() as u64);
                h += 3;
            }
        }
        for (w, h) in [(600, 480), (640, 480), (1500, 994), (97, 1033), (2000, 3)] {
            let b = f.bounds(rect(0, 0, w, h));
            d.add((b.dx() as u64) << 32 | b.dy() as u64);
        }
    }
    d.finish()
}

fn compare(fixture: &std::path::Path, got: Vec<(String, usize, usize, String)>) {
    let want = read_tsv(fixture);
    let mut want_map = BTreeMap::new();
    for row in &want {
        if row[0] == "pairs" {
            want_map.insert((row[0].clone(), 0), (row[1].clone(), String::new()));
        } else {
            want_map.insert(
                (row[0].clone(), row[1].parse::<usize>().unwrap()),
                (row[2].clone(), row[3].clone()),
            );
        }
    }
    let mut bad = 0;
    let mut got_n = 0;
    for (name, idx, n, h) in got {
        got_n += 1;
        let key = if name == "pairs" {
            (name.clone(), 0)
        } else {
            (name.clone(), idx)
        };
        let w = want_map.get(&key);
        let ok = match w {
            Some((wn, wh)) if name == "pairs" => *wn == idx.to_string() && wh.is_empty(),
            Some((wn, wh)) => *wn == n.to_string() && *wh == h,
            None => false,
        };
        if !ok {
            bad += 1;
            eprintln!(
                "MISMATCH {} chunk {} (n={} digest={}) want {:?}",
                name, idx, n, h, w
            );
        }
    }
    eprintln!(
        "{}: {} chunks, {} mismatches",
        fixture.display(),
        got_n,
        bad
    );
    assert_eq!(got_n, want.len(), "chunk count");
    assert_eq!(bad, 0);
}

#[test]
fn math_digest_fixture() {
    compare(&fixtures_dir().join("mathdigest.tsv"), math_digest(4));
}

#[test]
fn kernel_digest_fixture() {
    compare(&fixtures_dir().join("kerneldigest.tsv"), kernel_digest(64));
}

#[test]
fn weights_digest_fixture() {
    compare(&fixtures_dir().join("weights.tsv"), weights_digest());
}

#[test]
fn rot_bounds_digest_fixture() {
    compare(&fixtures_dir().join("rotbounds.tsv"), rot_bounds_digest());
}

#[test]
fn math_digest_big() {
    if let Ok(p) = std::env::var("GIFT_MATHDIGEST_BIG") {
        compare(std::path::Path::new(&p), math_digest(64));
    }
}

#[test]
fn kernel_digest_big() {
    if let Ok(p) = std::env::var("GIFT_KERNELDIGEST_BIG") {
        compare(std::path::Path::new(&p), kernel_digest(3));
    }
}
