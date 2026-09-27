//! Shared helpers for the differential tests: fixture loading, digests
//! formatted exactly like the oracle's (tools/go-oracle/go-png/main.go), and
//! call-for-call ports of the oracle's splitmix64-driven generators
//! (`genSynth`, `genFilterImage`, `mutate`, ...), so the tests rebuild the
//! oracle's inputs instead of storing them.

#![allow(
    dead_code,
    clippy::upper_case_acronyms,
    // Index loops mirror the oracle's Go loops.
    clippy::needless_range_loop
)]

use std::any::Any;
use std::io::{self, Read, Write};
use std::path::PathBuf;

use go_image::color::{self, Color, Model, Palette};
use go_image::{
    Alpha, Alpha16, CMYK, Gray, Gray16, Image, NRGBA, NRGBA64, NYCbCrA, Paletted, PalettedImage,
    RGBA, RGBA64, RGBA64Image, Rectangle, YCbCr, YCbCrSubsampleRatio, pt, rect,
};
use go_png::{CompressionLevel, Encoder};
use sha2::{Digest, Sha256};

// ---------------------------------------------------------------------------
// Fixtures.

pub fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

/// Reads a fixture file, gunzipping `*.gz` files.
pub fn read_fixture(name: &str) -> Vec<u8> {
    let path = fixtures_dir().join(name);
    let data = std::fs::read(&path).unwrap_or_else(|e| panic!("reading {}: {e}", path.display()));
    if name.ends_with(".gz") {
        let mut out = Vec::new();
        flate2::read::GzDecoder::new(&data[..])
            .read_to_end(&mut out)
            .unwrap_or_else(|e| panic!("gunzip {name}: {e}"));
        return out;
    }
    data
}

pub fn read_tsv(name: &str) -> Vec<Vec<String>> {
    parse_tsv(read_fixture(name))
}

/// A file of the out-of-repo corpus directory named by `$GO_PNG_BIG` (see
/// PORTING.md), or `None` (with a note) when it is not available.
pub fn big_path(name: &str) -> Option<PathBuf> {
    let Some(dir) = std::env::var_os("GO_PNG_BIG") else {
        eprintln!("GO_PNG_BIG not set; skipping {name}");
        return None;
    };
    let p = PathBuf::from(dir).join(name);
    if !p.exists() {
        eprintln!("{} missing; skipping", p.display());
        return None;
    }
    Some(p)
}

pub fn parse_tsv(data: Vec<u8>) -> Vec<Vec<String>> {
    let s = String::from_utf8(data).expect("fixture is UTF-8");
    s.lines()
        .filter(|l| !l.is_empty())
        .map(|l| l.split('\t').map(str::to_string).collect())
        .collect()
}

/// `$GO_PNG_BIG/<name>` parsed as TSV.
pub fn big_tsv(name: &str) -> Option<Vec<Vec<String>>> {
    big_path(name).map(|p| parse_tsv(std::fs::read(p).expect("reading big corpus")))
}

/// Reads a corpus written by the oracle's writeCorpus (u32 LE length +
/// bytes, repeated).
pub fn read_corpus(name: &str) -> Vec<Vec<u8>> {
    parse_corpus(&read_fixture(name))
}

pub fn parse_corpus(mut data: &[u8]) -> Vec<Vec<u8>> {
    let mut files = Vec::new();
    while !data.is_empty() {
        let n = u32::from_le_bytes(data[..4].try_into().unwrap()) as usize;
        files.push(data[4..4 + n].to_vec());
        data = &data[4 + n..];
    }
    files
}

/// Compares `got` with the expected rows' columns and collects mismatches
/// (up to a limit) so one run reports many failures.
#[derive(Default)]
pub struct Mismatches {
    pub count: usize,
    pub checked: usize,
    pub report: Vec<String>,
}

impl Mismatches {
    pub fn check(&mut self, what: &str, got: &str, want: &str) {
        self.checked += 1;
        if got != want {
            self.count += 1;
            if self.report.len() < 30 {
                self.report
                    .push(format!("{what}:\n     got: {got}\n    want: {want}"));
            }
        }
    }

    pub fn finish(&self, name: &str) {
        eprintln!("{name}: {} checks, {} mismatches", self.checked, self.count);
        assert!(
            self.count == 0,
            "{name}: {} of {} checks differ:\n{}",
            self.count,
            self.checked,
            self.report.join("\n")
        );
    }
}

// ---------------------------------------------------------------------------
// Digests (the oracle's rectStr, paletteBytes, imgDigest, cfgStr, ...).

pub fn sha16(b: &[u8]) -> String {
    let d = Sha256::digest(b);
    d.iter().map(|x| format!("{x:02x}")).collect::<String>()[..16].to_string()
}

pub fn rect_str(r: Rectangle) -> String {
    format!("{},{},{},{}", r.min.x, r.min.y, r.max.x, r.max.y)
}

pub fn palette_bytes(p: &Palette) -> Vec<u8> {
    let mut b = Vec::new();
    for c in p.iter() {
        match c {
            Color::RGBA(c) => b.extend_from_slice(&[b'R', c.r, c.g, c.b, c.a]),
            Color::NRGBA(c) => b.extend_from_slice(&[b'N', c.r, c.g, c.b, c.a]),
            c => {
                let (r, g, bb, a) = c.rgba();
                b.push(b'O');
                for v in [r, g, bb, a] {
                    b.push((v >> 8) as u8);
                    b.push(v as u8);
                }
            }
        }
    }
    b
}

pub fn pal_str(p: &Palette) -> String {
    format!("{}:{}", p.len(), sha16(&palette_bytes(p)))
}

pub fn img_digest(m: &dyn Image) -> String {
    macro_rules! plain {
        ($t:ty, $name:expr) => {
            if let Some(m) = m.downcast_ref::<$t>() {
                return format!(
                    "{} {} {} {} {}",
                    $name,
                    rect_str(m.rect),
                    m.stride,
                    m.pix.len(),
                    sha16(&m.pix)
                );
            }
        };
    }
    plain!(Gray, "Gray");
    plain!(Gray16, "Gray16");
    plain!(RGBA, "RGBA");
    plain!(RGBA64, "RGBA64");
    plain!(NRGBA, "NRGBA");
    plain!(NRGBA64, "NRGBA64");
    if let Some(m) = m.downcast_ref::<Paletted>() {
        return format!(
            "Paletted {} {} {} {} pal={}",
            rect_str(m.rect),
            m.stride,
            m.pix.len(),
            sha16(&m.pix),
            pal_str(&m.palette)
        );
    }
    "other".to_string()
}

pub fn model_str(m: &Model) -> String {
    match m {
        Model::Gray => "Gray".into(),
        Model::Gray16 => "Gray16".into(),
        Model::RGBA => "RGBA".into(),
        Model::RGBA64 => "RGBA64".into(),
        Model::NRGBA => "NRGBA".into(),
        Model::NRGBA64 => "NRGBA64".into(),
        Model::Palette(p) => format!("Palette:{}", pal_str(p)),
        _ => "other".into(),
    }
}

pub fn cfg_str(data: &[u8]) -> String {
    match go_png::decode_config(&mut &data[..]) {
        Err(e) => format!("err:{e}"),
        Ok(cfg) => format!(
            "{},{},{}",
            model_str(&cfg.color_model),
            cfg.width,
            cfg.height
        ),
    }
}

pub fn dec_from(r: &mut dyn Read) -> (Option<Box<dyn Image>>, String) {
    match go_png::decode(r) {
        Err(e) => (None, format!("err:{e}")),
        Ok(m) => {
            let d = img_digest(&*m);
            (Some(m), d)
        }
    }
}

pub fn dec_str(data: &[u8]) -> (Option<Box<dyn Image>>, String) {
    dec_from(&mut &data[..])
}

pub fn reg_str(data: &[u8]) -> String {
    go_png::register();
    match go_image::decode(&mut &data[..]) {
        Err(e) => format!("err:{e}"),
        Ok((m, name)) => format!("{name} {}", img_digest(&*m)),
    }
}

/// The oracle's decWith: "=" when the result equals `want`.
pub fn dec_with(r: &mut dyn Read, want: &str) -> String {
    let (_, s) = dec_from(r);
    if s == want { "=".to_string() } else { s }
}

pub const DEFAULT: CompressionLevel = CompressionLevel(0);
pub const LEVELS4: [CompressionLevel; 4] = [
    CompressionLevel(0),
    CompressionLevel(-1),
    CompressionLevel(-2),
    CompressionLevel(-3),
];
pub const LEVELS5: [CompressionLevel; 5] = [
    CompressionLevel(0),
    CompressionLevel(-1),
    CompressionLevel(-2),
    CompressionLevel(-3),
    CompressionLevel(7),
];

/// The oracle's encStr: "sha16:len" or "err:<msg>:sha16:len".
pub fn enc_str(m: &dyn Image, level: CompressionLevel) -> (String, Option<Vec<u8>>) {
    let mut buf = Vec::new();
    let enc = Encoder {
        compression_level: level,
        ..Default::default()
    };
    match enc.encode(&mut buf, m) {
        Err(e) => (format!("err:{e}:{}:{}", sha16(&buf), buf.len()), None),
        Ok(()) => (format!("{}:{}", sha16(&buf), buf.len()), Some(buf)),
    }
}

pub fn err_str<E: std::fmt::Display>(r: &Result<(), E>) -> String {
    match r {
        Ok(()) => "nil".into(),
        Err(e) => e.to_string(),
    }
}

/// Counts Write calls.
#[derive(Default)]
pub struct CountWriter {
    pub calls: usize,
}

impl Write for CountWriter {
    fn write(&mut self, p: &[u8]) -> io::Result<usize> {
        self.calls += 1;
        Ok(p.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

/// Accepts Write calls until call index k (0-based) and fails that call
/// and every later one with "boom".
pub struct FailWriter {
    pub k: usize,
    pub calls: usize,
    pub buf: Vec<u8>,
}

impl FailWriter {
    pub fn new(k: usize) -> FailWriter {
        FailWriter {
            k,
            calls: 0,
            buf: Vec::new(),
        }
    }
}

impl Write for FailWriter {
    fn write(&mut self, p: &[u8]) -> io::Result<usize> {
        self.calls += 1;
        if self.calls > self.k {
            return Err(io::Error::other("boom"));
        }
        self.buf.extend_from_slice(p);
        Ok(p.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

/// The oracle's failStr.
pub fn fail_str(m: &dyn Image, seed: u64) -> String {
    let mut cw = CountWriter::default();
    if go_png::encode(&mut cw, m).is_err() {
        return "-".into();
    }
    let mut rk = Rng::new(seed ^ 0xabcdef);
    let k = rk.intn(cw.calls as i64 + 1) as usize;
    let mut fw = FailWriter::new(k);
    let r = go_png::encode(&mut fw, m);
    format!(
        "k:{k} calls:{} {}:{} err:{}",
        fw.calls,
        sha16(&fw.buf),
        fw.buf.len(),
        err_str(&r)
    )
}

/// Go's testing/iotest.OneByteReader.
pub struct OneByteReader<'a>(pub &'a [u8]);

impl Read for OneByteReader<'_> {
    fn read(&mut self, p: &mut [u8]) -> io::Result<usize> {
        if p.is_empty() {
            return Ok(0);
        }
        self.0.read(&mut p[..1])
    }
}

/// Go's testing/iotest.HalfReader.
pub struct HalfReader<'a>(pub &'a [u8]);

impl Read for HalfReader<'_> {
    fn read(&mut self, p: &mut [u8]) -> io::Result<usize> {
        let n = p.len().div_ceil(2);
        self.0.read(&mut p[..n])
    }
}

pub fn channels(ct: u8) -> usize {
    match ct {
        2 => 3,
        4 => 2,
        6 => 4,
        _ => 1,
    }
}

/// The oracle's rowFilters: the filter type of every row of a
/// non-interlaced PNG, as digits.
pub fn row_filters(data: &[u8]) -> String {
    let mut idat = Vec::new();
    let (mut w, mut depth, mut ct) = (0usize, 0usize, 0u8);
    let mut off = 8;
    while off + 12 <= data.len() {
        let l = u32::from_be_bytes(data[off..off + 4].try_into().unwrap()) as usize;
        if off + 12 + l > data.len() {
            return "-".into();
        }
        let body = &data[off + 8..off + 8 + l];
        match &data[off + 4..off + 8] {
            b"IHDR" => {
                if l != 13 {
                    return "-".into();
                }
                w = u32::from_be_bytes(body[0..4].try_into().unwrap()) as usize;
                depth = body[8] as usize;
                ct = body[9];
            }
            b"IDAT" => idat.extend_from_slice(body),
            _ => {}
        }
        off += 12 + l;
    }
    let mut raw = Vec::new();
    if flate2::read::ZlibDecoder::new(&idat[..])
        .read_to_end(&mut raw)
        .is_err()
    {
        return "-".into();
    }
    let row_size = 1 + (depth * channels(ct) * w).div_ceil(8);
    let mut s = String::new();
    let mut i = 0;
    while i < raw.len() {
        s.push((b'0' + raw[i]) as char);
        i += row_size;
    }
    s
}

// ---------------------------------------------------------------------------
// splitmix64 and the oracle's generators.

pub struct Rng {
    s: u64,
}

impl Rng {
    pub fn new(s: u64) -> Rng {
        Rng { s }
    }
    /// The oracle seeds generator i with `uint64(i)*0x9e3779b97f4a7c15 + add`.
    pub fn seeded(i: u64, add: u64) -> Rng {
        Rng::new(i.wrapping_mul(0x9e3779b97f4a7c15).wrapping_add(add))
    }
    pub fn next(&mut self) -> u64 {
        self.s = self.s.wrapping_add(0x9e3779b97f4a7c15);
        let mut z = self.s;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d049bb133111eb);
        z ^ (z >> 31)
    }
    pub fn intn(&mut self, n: i64) -> i64 {
        (self.next() % n as u64) as i64
    }
    pub fn byte(&mut self) -> u8 {
        self.next() as u8
    }
    pub fn u16(&mut self) -> u16 {
        self.next() as u16
    }
}

/// The oracle's fill.
pub fn fill(r: &mut Rng, b: &mut [u8], stride: i64) {
    match r.intn(4) {
        0 => {
            for x in b.iter_mut() {
                *x = r.byte();
            }
        }
        1 => {
            let k1 = r.intn(7);
            let k2 = r.intn(7);
            let c = r.intn(256);
            let noise = r.intn(8);
            let stride = if stride <= 0 { 1 } else { stride };
            for (i, x) in b.iter_mut().enumerate() {
                let i = i as i64;
                let mut v = (i % stride) * k1 + (i / stride) * k2 + c;
                if noise > 0 {
                    v += r.intn(noise);
                }
                *x = v as u8;
            }
        }
        2 => {
            let vals = [r.byte(), r.byte(), r.byte(), r.byte()];
            for x in b.iter_mut() {
                *x = vals[r.intn(4) as usize];
            }
        }
        _ => {
            let mut c = r.byte();
            for x in b.iter_mut() {
                if r.intn(64) == 0 {
                    c = r.byte();
                }
                *x = c;
            }
        }
    }
}

/// The oracle's fixAlpha.
pub fn fix_alpha(
    r: &mut Rng,
    pix: &mut [u8],
    psize: usize,
    aoff: usize,
    asize: usize,
    premul: bool,
) {
    let n = pix.len() / psize;
    let mode = r.intn(5);
    let set_a = |pix: &mut [u8], p: usize, v: u16| {
        pix[p * psize + aoff] = (v >> 8) as u8;
        if asize != 1 {
            pix[p * psize + aoff + 1] = v as u8;
        }
    };
    let get_a = |pix: &[u8], p: usize| -> u16 {
        if asize == 1 {
            let v = pix[p * psize + aoff] as u16;
            return v << 8 | v;
        }
        (pix[p * psize + aoff] as u16) << 8 | pix[p * psize + aoff + 1] as u16
    };
    match mode {
        0 => {
            for p in 0..n {
                set_a(pix, p, 0xffff);
            }
        }
        1 => {
            // Leave the random alpha.
        }
        2 => {
            for p in 0..n {
                if get_a(pix, p) & 0x100 != 0 {
                    set_a(pix, p, 0xffff);
                } else {
                    set_a(pix, p, 0);
                }
            }
        }
        3 => {
            for p in 0..n {
                set_a(pix, p, 0xffff);
            }
            if n > 0 {
                let p = r.intn(n as i64) as usize;
                let v = r.u16();
                set_a(pix, p, v);
            }
        }
        _ => {
            for p in 0..n {
                set_a(pix, p, 0);
            }
        }
    }
    if premul && r.intn(2) == 0 {
        for p in 0..n {
            let a = get_a(pix, p);
            let mut c = 0;
            while c < aoff {
                let o = p * psize + c;
                if asize == 1 {
                    if pix[o] as u16 > a >> 8 {
                        pix[o] = (a >> 8) as u8;
                    }
                } else {
                    let v = (pix[o] as u16) << 8 | pix[o + 1] as u16;
                    if v > a {
                        pix[o] = (a >> 8) as u8;
                        pix[o + 1] = a as u8;
                    }
                }
                c += asize;
            }
        }
    }
}

/// The oracle's randColor.
pub fn rand_color(r: &mut Rng) -> Color {
    match r.intn(11) {
        0 => Color::RGBA(color::RGBA {
            r: r.byte(),
            g: r.byte(),
            b: r.byte(),
            a: r.byte(),
        }),
        1 => Color::NRGBA(color::NRGBA {
            r: r.byte(),
            g: r.byte(),
            b: r.byte(),
            a: r.byte(),
        }),
        2 => Color::RGBA64(color::RGBA64 {
            r: r.u16(),
            g: r.u16(),
            b: r.u16(),
            a: r.u16(),
        }),
        3 => Color::NRGBA64(color::NRGBA64 {
            r: r.u16(),
            g: r.u16(),
            b: r.u16(),
            a: r.u16(),
        }),
        4 => Color::Gray(color::Gray { y: r.byte() }),
        5 => Color::Gray16(color::Gray16 { y: r.u16() }),
        6 => Color::Alpha(color::Alpha { a: r.byte() }),
        7 => Color::Alpha16(color::Alpha16 { a: r.u16() }),
        8 => Color::YCbCr(color::YCbCr {
            y: r.byte(),
            cb: r.byte(),
            cr: r.byte(),
        }),
        9 => {
            let ycbcr = color::YCbCr {
                y: r.byte(),
                cb: r.byte(),
                cr: r.byte(),
            };
            Color::NYCbCrA(color::NYCbCrA { ycbcr, a: r.byte() })
        }
        _ => Color::CMYK(color::CMYK {
            c: r.byte(),
            m: r.byte(),
            y: r.byte(),
            k: r.byte(),
        }),
    }
}

/// The oracle's genPaletted.
pub fn gen_paletted(r: &mut Rng, rc: Rectangle) -> Paletted {
    let np: i64 = match r.intn(8) {
        0 => 1 + r.intn(2),
        1 => 3 + r.intn(2),
        2 => 5 + r.intn(12),
        3 => 256,
        4 => {
            if r.intn(4) == 0 {
                if r.intn(2) == 0 { 0 } else { 257 + r.intn(10) }
            } else {
                17 + r.intn(239)
            }
        }
        _ => 1 + r.intn(256),
    };
    let mut p = Vec::with_capacity(np as usize);
    for _ in 0..np {
        let c = match r.intn(4) {
            0 => Color::RGBA(color::RGBA {
                r: r.byte(),
                g: r.byte(),
                b: r.byte(),
                a: 0xff,
            }),
            1 => Color::NRGBA(color::NRGBA {
                r: r.byte(),
                g: r.byte(),
                b: r.byte(),
                a: r.byte(),
            }),
            2 => rand_color(r),
            _ => Color::NRGBA(color::NRGBA {
                r: r.byte(),
                g: r.byte(),
                b: r.byte(),
                a: 0xff,
            }),
        };
        p.push(c);
    }
    let mut m = Paletted::new(rc, Palette(p));
    let mut mode = r.intn(4);
    if np == 0 {
        mode = 1;
    }
    match mode {
        0 => {
            for x in m.pix.iter_mut() {
                *x = r.intn(np) as u8;
            }
        }
        1 => {
            for x in m.pix.iter_mut() {
                *x = r.byte();
            }
        }
        2 => {
            let s = m.stride;
            fill(r, &mut m.pix, s);
            for x in m.pix.iter_mut() {
                *x = (*x as i64 % np) as u8;
            }
        }
        _ => {
            for x in m.pix.iter_mut() {
                if r.intn(8) == 0 {
                    *x = r.intn(np) as u8;
                }
            }
        }
    }
    m
}

pub const RATIOS: [YCbCrSubsampleRatio; 6] = [
    YCbCrSubsampleRatio::Ratio444,
    YCbCrSubsampleRatio::Ratio422,
    YCbCrSubsampleRatio::Ratio420,
    YCbCrSubsampleRatio::Ratio440,
    YCbCrSubsampleRatio::Ratio411,
    YCbCrSubsampleRatio::Ratio410,
];

/// The concrete images genBase creates (all with a SubImage method).
pub enum Base {
    RGBA(RGBA),
    NRGBA(NRGBA),
    RGBA64(RGBA64),
    NRGBA64(NRGBA64),
    Alpha(Alpha),
    Alpha16(Alpha16),
    Gray(Gray),
    Gray16(Gray16),
    CMYK(CMYK),
    Paletted(Paletted),
}

impl Base {
    pub fn boxed(self) -> Box<dyn Image> {
        match self {
            Base::RGBA(m) => Box::new(m),
            Base::NRGBA(m) => Box::new(m),
            Base::RGBA64(m) => Box::new(m),
            Base::NRGBA64(m) => Box::new(m),
            Base::Alpha(m) => Box::new(m),
            Base::Alpha16(m) => Box::new(m),
            Base::Gray(m) => Box::new(m),
            Base::Gray16(m) => Box::new(m),
            Base::CMYK(m) => Box::new(m),
            Base::Paletted(m) => Box::new(m),
        }
    }

    pub fn sub_image(&self, r: Rectangle) -> Box<dyn Image> {
        match self {
            Base::RGBA(m) => Box::new(m.sub_image(r)),
            Base::NRGBA(m) => Box::new(m.sub_image(r)),
            Base::RGBA64(m) => Box::new(m.sub_image(r)),
            Base::NRGBA64(m) => Box::new(m.sub_image(r)),
            Base::Alpha(m) => Box::new(m.sub_image(r)),
            Base::Alpha16(m) => Box::new(m.sub_image(r)),
            Base::Gray(m) => Box::new(m.sub_image(r)),
            Base::Gray16(m) => Box::new(m.sub_image(r)),
            Base::CMYK(m) => Box::new(m.sub_image(r)),
            Base::Paletted(m) => Box::new(m.sub_image(r)),
        }
    }
}

/// The oracle's genBase.
pub fn gen_base(r: &mut Rng, kind: i64, rc: Rectangle) -> Base {
    match kind {
        0 => {
            let mut m = RGBA::new(rc);
            let s = m.stride;
            fill(r, &mut m.pix, s);
            fix_alpha(r, &mut m.pix, 4, 3, 1, true);
            Base::RGBA(m)
        }
        1 => {
            let mut m = NRGBA::new(rc);
            let s = m.stride;
            fill(r, &mut m.pix, s);
            fix_alpha(r, &mut m.pix, 4, 3, 1, false);
            Base::NRGBA(m)
        }
        2 => {
            let mut m = RGBA64::new(rc);
            let s = m.stride;
            fill(r, &mut m.pix, s);
            fix_alpha(r, &mut m.pix, 8, 6, 2, true);
            Base::RGBA64(m)
        }
        3 => {
            let mut m = NRGBA64::new(rc);
            let s = m.stride;
            fill(r, &mut m.pix, s);
            fix_alpha(r, &mut m.pix, 8, 6, 2, false);
            Base::NRGBA64(m)
        }
        4 => {
            let mut m = Alpha::new(rc);
            let s = m.stride;
            fill(r, &mut m.pix, s);
            fix_alpha(r, &mut m.pix, 1, 0, 1, false);
            Base::Alpha(m)
        }
        5 => {
            let mut m = Alpha16::new(rc);
            let s = m.stride;
            fill(r, &mut m.pix, s);
            fix_alpha(r, &mut m.pix, 2, 0, 2, false);
            Base::Alpha16(m)
        }
        6 => {
            let mut m = Gray::new(rc);
            let s = m.stride;
            fill(r, &mut m.pix, s);
            Base::Gray(m)
        }
        7 => {
            let mut m = Gray16::new(rc);
            let s = m.stride;
            fill(r, &mut m.pix, s);
            Base::Gray16(m)
        }
        8 => {
            let mut m = CMYK::new(rc);
            let s = m.stride;
            fill(r, &mut m.pix, s);
            Base::CMYK(m)
        }
        _ => Base::Paletted(gen_paletted(r, rc)),
    }
}

/// The oracle's hiddenImage: no optional methods, not a concrete type.
pub struct Hidden(pub Box<dyn Image>);

impl Image for Hidden {
    fn color_model(&self) -> Model {
        self.0.color_model()
    }
    fn bounds(&self) -> Rectangle {
        self.0.bounds()
    }
    fn at(&self, x: i64, y: i64) -> Color {
        self.0.at(x, y)
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn into_any(self: Box<Self>) -> Box<dyn Any> {
        self
    }
}

/// The oracle's wrappedPaletted: `struct{ *image.Paletted }` (every
/// *Paletted method promoted, but not an *image.Paletted).
pub struct WrappedPaletted(pub Paletted);

impl Image for WrappedPaletted {
    fn color_model(&self) -> Model {
        self.0.color_model()
    }
    fn bounds(&self) -> Rectangle {
        self.0.bounds()
    }
    fn at(&self, x: i64, y: i64) -> Color {
        self.0.at(x, y)
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn into_any(self: Box<Self>) -> Box<dyn Any> {
        self
    }
    fn as_rgba64_image(&self) -> Option<&dyn RGBA64Image> {
        Some(self)
    }
    fn as_paletted_image(&self) -> Option<&dyn PalettedImage> {
        Some(self)
    }
    fn try_opaque(&self) -> Option<bool> {
        Some(self.0.opaque())
    }
}

impl RGBA64Image for WrappedPaletted {
    fn rgba64_at(&self, x: i64, y: i64) -> color::RGBA64 {
        self.0.rgba64_at(x, y)
    }
}

impl PalettedImage for WrappedPaletted {
    fn color_index_at(&self, x: i64, y: i64) -> u8 {
        self.0.color_index_at(x, y)
    }
}

pub const N_SYNTH_KINDS: i64 = 16;

/// The oracle's genSynth.
pub fn gen_synth(r: &mut Rng, max_dim: i64) -> (Box<dyn Image>, String) {
    let (mut w, mut h) = (1 + r.intn(24), 1 + r.intn(24));
    if r.intn(6) == 0 {
        (w, h) = (1 + r.intn(max_dim), 1 + r.intn(max_dim));
    }
    if r.intn(40) == 0 {
        if r.intn(2) == 0 {
            w = 0;
        } else {
            h = 0;
        }
    }
    let (ox, oy) = (r.intn(7) - 3, r.intn(7) - 3);
    let mut rc = rect(ox, oy, ox + w, oy + h);
    let kind = r.intn(N_SYNTH_KINDS);
    let desc = format!("k{kind} {}", rect_str(rc));
    match kind {
        10 => {
            // Go's chroma indexing is only consistent for non-negative
            // coordinates, so YCbCr images are shifted into the positive quadrant.
            rc = rc.add(pt(16, 16));
            let ratio = RATIOS[r.intn(6) as usize];
            let mut m = YCbCr::new(rc, ratio);
            let (ys, cs) = (m.y_stride, m.c_stride);
            fill(r, &mut m.y, ys);
            fill(r, &mut m.cb, cs);
            fill(r, &mut m.cr, cs);
            (Box::new(m), desc)
        }
        11 => {
            rc = rc.add(pt(16, 16));
            let ratio = RATIOS[r.intn(6) as usize];
            let mut m = NYCbCrA::new(rc, ratio);
            let (ys, cs, as_) = (m.ycbcr.y_stride, m.ycbcr.c_stride, m.a_stride);
            fill(r, &mut m.ycbcr.y, ys);
            fill(r, &mut m.ycbcr.cb, cs);
            fill(r, &mut m.ycbcr.cr, cs);
            fill(r, &mut m.a, as_);
            fix_alpha(r, &mut m.a, 1, 0, 1, false);
            (Box::new(m), desc)
        }
        12 => {
            let pk = r.intn(10);
            let (l, t, rr, b) = (r.intn(4), r.intn(4), r.intn(4), r.intn(4));
            let parent = gen_base(
                r,
                pk,
                rect(rc.min.x - l, rc.min.y - t, rc.max.x + rr, rc.max.y + b),
            );
            (parent.sub_image(rc), format!("{desc} sub{pk}"))
        }
        13 => (Box::new(rc), desc),
        14 => {
            let pk = r.intn(9);
            let m = gen_base(r, pk, rc).boxed();
            (Box::new(Hidden(m)), format!("{desc} hidden{pk}"))
        }
        15 => {
            let m = gen_paletted(r, rc);
            (Box::new(WrappedPaletted(m)), format!("{desc} wrapped"))
        }
        _ => (gen_base(r, kind, rc).boxed(), desc),
    }
}

/// The oracle's genFilterImage.
pub fn gen_filter_image(r: &mut Rng) -> (Box<dyn Image>, String) {
    let (mut w, mut h) = (1 + r.intn(9), 1 + r.intn(6));
    if r.intn(4) == 0 {
        (w, h) = (1 + r.intn(40), 1 + r.intn(12));
    }
    let rc = rect(0, 0, w, h);
    let vals = [0u8, 1, 2, 127, 128, 129, 254, 255, r.byte(), r.byte()];
    let mut set = vec![0u8; 1 + r.intn(4) as usize];
    for s in set.iter_mut() {
        *s = vals[r.intn(vals.len() as i64) as usize];
    }
    let kind = r.intn(6);
    let mode = r.intn(5);
    let base = set[r.intn(set.len() as i64) as usize];
    let (dx, dy) = (r.intn(5) - 2, r.intn(5) - 2);
    let (psize, aoff, asize): (usize, usize, usize) = match kind {
        0 => (1, 0, 1),
        1 => (2, 0, 1),
        2 | 3 => (4, 3, 1),
        _ => (8, 6, 2),
    };
    let (w, h) = (w as usize, h as usize);
    let mut pix = vec![0u8; w * h * psize];
    let row_len = w * psize;
    let pick = |r: &mut Rng, set: &[u8]| set[r.intn(set.len() as i64) as usize];
    for y in 0..h {
        match mode {
            0 => {
                for i in 0..row_len {
                    pix[y * row_len + i] = pick(r, &set);
                }
            }
            1 => {
                if y > 0 && r.intn(2) == 0 {
                    pix.copy_within((y - 1) * row_len..y * row_len, y * row_len);
                } else {
                    for i in 0..row_len {
                        pix[y * row_len + i] = pick(r, &set);
                    }
                }
            }
            2 => {
                let c = pick(r, &set);
                for i in 0..row_len {
                    pix[y * row_len + i] = c;
                }
            }
            3 => {
                for i in 0..row_len {
                    pix[y * row_len + i] = (base as i64 + i as i64 * dx + y as i64 * dy) as u8;
                }
            }
            _ => {
                if y == 0 {
                    for i in 0..row_len {
                        pix[i] = pick(r, &set);
                    }
                } else {
                    let d = pick(r, &set);
                    for i in 0..row_len {
                        pix[y * row_len + i] = pix[(y - 1) * row_len + i].wrapping_add(d);
                    }
                }
            }
        }
    }
    if kind == 2 || kind == 4 {
        // Opaque RGBA/RGBA64 (TC8/TC16).
        for p in 0..w * h {
            for k in 0..asize {
                pix[p * psize + aoff + k] = 0xff;
            }
        }
    }
    let m: Box<dyn Image> = match kind {
        0 => {
            let mut m = Gray::new(rc);
            m.pix = pix;
            Box::new(m)
        }
        1 => {
            let mut m = Gray16::new(rc);
            m.pix = pix;
            Box::new(m)
        }
        2 => {
            let mut m = RGBA::new(rc);
            m.pix = pix;
            Box::new(m)
        }
        3 => {
            let mut m = NRGBA::new(rc);
            m.pix = pix;
            Box::new(m)
        }
        4 => {
            let mut m = RGBA64::new(rc);
            m.pix = pix;
            Box::new(m)
        }
        _ => {
            let mut m = NRGBA64::new(rc);
            m.pix = pix;
            Box::new(m)
        }
    };
    (m, format!("f{kind} m{mode} {w}x{h}"))
}

// ---------------------------------------------------------------------------
// Corpus mutation (the oracle's chunks, fixCRC, writeChunk, mutate).

pub fn write_chunk(w: &mut Vec<u8>, name: &[u8], data: &[u8]) {
    w.extend_from_slice(&(data.len() as u32).to_be_bytes());
    w.extend_from_slice(name);
    w.extend_from_slice(data);
    let mut crc = crc32fast::Hasher::new();
    crc.update(name);
    crc.update(data);
    w.extend_from_slice(&crc.finalize().to_be_bytes());
}

#[derive(Clone, Copy)]
struct ChunkPos {
    off: usize,
    length: usize,
}

fn chunks(b: &[u8]) -> Vec<ChunkPos> {
    let mut cs = Vec::new();
    let mut off = 8;
    while off + 12 <= b.len() {
        let l = u32::from_be_bytes(b[off..off + 4].try_into().unwrap()) as usize;
        if off + 12 + l > b.len() {
            break;
        }
        cs.push(ChunkPos { off, length: l });
        off += 12 + l;
    }
    cs
}

fn fix_crc(b: &mut [u8], c: ChunkPos) {
    let crc = crc32fast::hash(&b[c.off + 4..c.off + 8 + c.length]);
    b[c.off + 8 + c.length..c.off + 12 + c.length].copy_from_slice(&crc.to_be_bytes());
}

fn put_u32(b: &mut [u8], off: usize, v: u32) {
    b[off..off + 4].copy_from_slice(&v.to_be_bytes());
}

/// The oracle's mutate.
pub fn mutate(r: &mut Rng, input: &[u8]) -> Vec<u8> {
    let mut b = input.to_vec();
    let nm = 1 + r.intn(3);
    for _ in 0..nm {
        if b.is_empty() {
            break;
        }
        let cs = chunks(&b);
        let mut op = r.intn(11);
        if cs.is_empty() && op >= 2 {
            op = r.intn(2);
        }
        let pick = |r: &mut Rng| cs[r.intn(cs.len() as i64) as usize];
        match op {
            0 => {
                let i = r.intn(b.len() as i64) as usize;
                b[i] ^= 1 << r.intn(8);
            }
            1 => {
                let n = r.intn(b.len() as i64) as usize;
                b.truncate(n);
            }
            2 => {
                let c = pick(r);
                if c.length > 0 {
                    let i = c.off + 8 + r.intn(c.length as i64) as usize;
                    b[i] = r.byte();
                    fix_crc(&mut b, c);
                }
            }
            3 => {
                let c = pick(r);
                let v = (c.length as i64 + r.intn(5) - 2) as u32;
                put_u32(&mut b, c.off, v);
            }
            4 => {
                let c = pick(r);
                b.drain(c.off..c.off + 12 + c.length);
            }
            5 => {
                let c = pick(r);
                let dup = b[c.off..c.off + 12 + c.length].to_vec();
                b.splice(c.off..c.off, dup);
            }
            6 => {
                if cs.len() >= 2 {
                    let i = r.intn(cs.len() as i64 - 1) as usize;
                    let (c0, c1) = (cs[i], cs[i + 1]);
                    let a = b[c0.off..c0.off + 12 + c0.length].to_vec();
                    let bb = b[c1.off..c1.off + 12 + c1.length].to_vec();
                    let rest = b[c1.off + 12 + c1.length..].to_vec();
                    b.truncate(c0.off);
                    b.extend_from_slice(&bb);
                    b.extend_from_slice(&a);
                    b.extend_from_slice(&rest);
                }
            }
            7 => {
                let names: [&[u8]; 8] = [
                    b"IHDR", b"PLTE", b"tRNS", b"IDAT", b"IEND", b"tEXt", b"gAMA", b"IDAT",
                ];
                let mut data = vec![0u8; r.intn(8) as usize];
                for d in data.iter_mut() {
                    *d = r.byte();
                }
                let mut nb = Vec::new();
                let name = names[r.intn(names.len() as i64) as usize];
                write_chunk(&mut nb, name, &data);
                let c = pick(r);
                b.splice(c.off..c.off, nb);
            }
            8 => {
                let c = cs[0];
                if &b[c.off + 4..c.off + 8] == b"IHDR" && c.length >= 13 {
                    let d = c.off + 8;
                    match r.intn(5) {
                        0 => b[d + 8] = [1u8, 2, 4, 8, 16][r.intn(5) as usize],
                        1 => b[d + 9] = [0u8, 2, 3, 4, 6][r.intn(5) as usize],
                        2 => b[d + 12] ^= 1,
                        3 => b[d + 3] = (b[d + 3] as i64 + r.intn(5) - 2) as u8,
                        _ => b[d + 7] = (b[d + 7] as i64 + r.intn(5) - 2) as u8,
                    }
                    fix_crc(&mut b, c);
                }
            }
            9 => {
                let c = pick(r);
                let lens = [
                    0x7fffffffu32,
                    0x80000000,
                    0xfffffff0,
                    0xffffffff,
                    r.next() as u32,
                ];
                let v = lens[r.intn(lens.len() as i64) as usize];
                put_u32(&mut b, c.off, v);
            }
            _ => {
                let idats: Vec<ChunkPos> = cs
                    .iter()
                    .copied()
                    .filter(|c| &b[c.off + 4..c.off + 8] == b"IDAT" && c.length > 0)
                    .collect();
                if !idats.is_empty() {
                    let c = idats[r.intn(idats.len() as i64) as usize];
                    let i = c.off + 8 + r.intn(c.length as i64) as usize;
                    b[i] ^= 1 << r.intn(8);
                    fix_crc(&mut b, c);
                }
            }
        }
    }
    b
}
