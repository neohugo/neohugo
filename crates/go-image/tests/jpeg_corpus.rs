//! Corpus-based differential tests (oracle `go-image record`, see
//! tools/go-oracle/go-image/corpus.go). Each corpus entry is a JPEG (or a
//! mutated JPEG) and the record compares, field by field:
//!
//! * `jpeg.DecodeConfig` and `image.DecodeConfig` (format registry),
//! * `jpeg.Decode` and `image.Decode` (bufio-wrapped reader) digests,
//! * encodings of the decoded image at q1/50/75/100,
//! * `draw.Draw` into RGBA (fast paths), its q75 encoding,
//! * `draw.DrawMask` Over with an Alpha mask onto a patterned RGBA,
//! * `draw.Draw` into NRGBA (generic FALLBACK1.17 path),
//! * a sub-image with odd offsets (bounds, q75 encoding, RGBA conversion),
//! * `FloydSteinberg.Draw` onto a Plan9 paletted image (small images).
//!
//! Checked-in corpora live in tests/fixtures/corpus/; larger ones are run
//! with `GO_IMAGE_CORPUS_BIG=<dir>` (every `<name>.bin` with a matching
//! `<name>.tsv` in that directory).

mod common;

use common::*;
use go_image::color::palette;
use go_image::draw::{self, Drawer, Op};
use go_image::jpeg;
use go_image::{Alpha, CMYK, Gray, Image, NRGBA, Paletted, RGBA, YCbCr, rect};

const MAGIC: &[u8] = b"GOIMGC1\n";

pub fn read_corpus(path: &std::path::Path) -> Vec<(String, Vec<u8>)> {
    let b = std::fs::read(path).unwrap_or_else(|e| panic!("{}: {}", path.display(), e));
    assert!(
        b.starts_with(MAGIC),
        "bad corpus magic in {}",
        path.display()
    );
    let mut b = &b[MAGIC.len()..];
    let mut out = Vec::new();
    while !b.is_empty() {
        let n = u32::from_le_bytes(b[..4].try_into().unwrap()) as usize;
        let name = String::from_utf8(b[4..4 + n].to_vec()).unwrap();
        b = &b[4 + n..];
        let m = u32::from_le_bytes(b[..4].try_into().unwrap()) as usize;
        let data = b[4..4 + m].to_vec();
        b = &b[4 + m..];
        out.push((name, data));
    }
    out
}

/// Port of the oracle's imgDigest16.
fn img_digest16(m: &dyn Image) -> String {
    if let Some(m) = m.downcast_ref::<Gray>() {
        return format!(
            "Gray {} {} {} {}",
            rect_str(m.rect),
            m.stride,
            m.pix.len(),
            sha16(&m.pix)
        );
    }
    if let Some(m) = m.downcast_ref::<RGBA>() {
        return format!(
            "RGBA {} {} {} {}",
            rect_str(m.rect),
            m.stride,
            m.pix.len(),
            sha16(&m.pix)
        );
    }
    if let Some(m) = m.downcast_ref::<CMYK>() {
        return format!(
            "CMYK {} {} {} {}",
            rect_str(m.rect),
            m.stride,
            m.pix.len(),
            sha16(&m.pix)
        );
    }
    if let Some(m) = m.downcast_ref::<YCbCr>() {
        return format!(
            "YCbCr {} {} {} {} {} {} {} {} {} {}",
            rect_str(m.rect),
            m.subsample_ratio as i32,
            m.y_stride,
            m.c_stride,
            m.y.len(),
            m.cb.len(),
            m.cr.len(),
            sha16(&m.y),
            sha16(&m.cb),
            sha16(&m.cr)
        );
    }
    "other".to_string()
}

fn enc_sha16(m: &dyn Image, q: i64) -> String {
    let mut buf = Vec::new();
    match jpeg::encode(&mut buf, m, Some(&jpeg::Options { quality: q })) {
        Ok(()) => sha16(&buf),
        Err(e) => format!("err:{}", e),
    }
}

/// Go's `m.(interface{ SubImage(image.Rectangle) image.Image }).SubImage(r)`
/// for the decoder's output types.
fn sub_image(m: &dyn Image, r: go_image::Rectangle) -> Box<dyn Image> {
    if let Some(m) = m.downcast_ref::<Gray>() {
        return Box::new(m.sub_image(r));
    }
    if let Some(m) = m.downcast_ref::<RGBA>() {
        return Box::new(m.sub_image(r));
    }
    if let Some(m) = m.downcast_ref::<CMYK>() {
        return Box::new(m.sub_image(r));
    }
    if let Some(m) = m.downcast_ref::<YCbCr>() {
        return Box::new(m.sub_image(r));
    }
    panic!("unexpected decoder output type");
}

fn cfg_str(c: &go_image::Config) -> String {
    format!("{},{},{}", model_name(&c.color_model), c.width, c.height)
}

/// Port of the oracle's record.
pub fn record(name: &str, data: &[u8]) -> Vec<String> {
    jpeg::register();
    let mut f = vec![name.to_string(), sha16(data)];
    let mut large = false;
    match jpeg::decode_config(&mut &data[..]) {
        Err(e) => f.push(format!("err:{}", e)),
        Ok(c) => {
            f.push(cfg_str(&c));
            large = c.width * c.height > 4 << 20;
        }
    }
    match go_image::decode_config(&mut &data[..]) {
        Err(e) => f.push(format!("err:{}", e)),
        Ok((c, fname)) => f.push(format!("{}:{}", fname, cfg_str(&c))),
    }
    if large {
        f.push("skip".to_string());
        return f;
    }
    let m = jpeg::decode(&mut &data[..]);
    match &m {
        Err(e) => f.push(format!("err:{}", e)),
        Ok(m) => f.push(img_digest16(m.as_ref())),
    }
    match go_image::decode(&mut &data[..]) {
        Err(e) => f.push(format!("err:{}", e)),
        Ok((m2, fname)) => f.push(format!("{}:{}", fname, img_digest16(m2.as_ref()))),
    }
    let Ok(m) = m else {
        return f;
    };
    let m = m.as_ref();
    let b = m.bounds();

    // Encodings of the decoded image.
    let qs: Vec<String> = [1, 50, 75, 100].iter().map(|&q| enc_sha16(m, q)).collect();
    f.push(qs.join(","));

    // draw.Draw into RGBA (Src).
    let mut rgba = RGBA::new(b);
    draw::draw(&mut rgba, b, m, b.min, Op::Src);
    f.push(sha16(&rgba.pix));
    f.push(enc_sha16(&rgba, 75));

    // DrawMask Over with an Alpha mask onto a patterned RGBA.
    let mut pat = RGBA::new(b);
    for (i, p) in pat.pix.iter_mut().enumerate() {
        *p = (i as u64).wrapping_mul(7).wrapping_add(3) as u8;
    }
    let mut mask = Alpha::new(b);
    for (i, p) in mask.pix.iter_mut().enumerate() {
        *p = (i as u64).wrapping_mul(13).wrapping_add(1) as u8;
    }
    draw::draw_mask(
        &mut pat,
        b,
        m,
        b.min,
        Some(&mask as &dyn Image),
        b.min,
        Op::Over,
    );
    f.push(sha16(&pat.pix));

    // draw.Draw into NRGBA (Src).
    let mut nrgba = NRGBA::new(b);
    draw::draw(&mut nrgba, b, m, b.min, Op::Src);
    f.push(sha16(&nrgba.pix));

    // A sub-image with odd offsets.
    let r2 = rect(b.min.x + 1, b.min.y + 3, b.max.x - 2, b.max.y);
    let sub = sub_image(m, r2);
    let sb = sub.bounds();
    let mut sub_rgba = RGBA::new(sb);
    draw::draw(&mut sub_rgba, sb, sub.as_ref(), sb.min, Op::Src);
    f.push(rect_str(sb));
    f.push(enc_sha16(sub.as_ref(), 75));
    f.push(sha16(&sub_rgba.pix));

    // Floyd-Steinberg onto a Plan9 paletted image (small images only).
    if b.dx() * b.dy() <= 64 * 64 {
        let mut p = Paletted::new(b, palette::plan9());
        draw::FLOYD_STEINBERG.draw(&mut p, b, m, b.min);
        f.push(sha16(&p.pix));
    } else {
        f.push("-".to_string());
    }
    f
}

pub fn check_corpus(bin: &std::path::Path, tsv: &std::path::Path) -> (usize, usize) {
    let entries = read_corpus(bin);
    let want = parse_tsv(&std::fs::read_to_string(tsv).unwrap());
    assert_eq!(entries.len(), want.len(), "{}", bin.display());
    let mut failures = 0;
    for ((name, data), w) in entries.iter().zip(want.iter()) {
        let got = record(name, data);
        if &got != w {
            failures += 1;
            if failures <= 20 {
                eprintln!("MISMATCH {}", name);
                for (i, (g, ww)) in got.iter().zip(w.iter()).enumerate() {
                    if g != ww {
                        eprintln!("  field {}: got {} want {}", i, g, ww);
                    }
                }
                if got.len() != w.len() {
                    eprintln!("  len got {} want {}", got.len(), w.len());
                }
            }
        }
    }
    (failures, entries.len())
}

/// Port of the oracle's encsweep: every successfully decoded image encoded at
/// every quality 1..100.
pub fn check_encsweep(bin: &std::path::Path, tsv: &std::path::Path) -> (usize, usize) {
    let entries = read_corpus(bin);
    let want = parse_tsv(&std::fs::read_to_string(tsv).unwrap());
    let mut got_rows = Vec::new();
    for (name, data) in &entries {
        let Ok(m) = jpeg::decode(&mut &data[..]) else {
            continue;
        };
        let b = m.bounds();
        if b.dx() * b.dy() > 1 << 20 {
            continue;
        }
        let hs: Vec<String> = (1..=100)
            .map(|q| enc_sha16(m.as_ref(), q)[..8].to_string())
            .collect();
        got_rows.push(vec![name.clone(), hs.join(",")]);
    }
    assert_eq!(got_rows.len(), want.len(), "{}", tsv.display());
    let mut failures = 0;
    for (g, w) in got_rows.iter().zip(want.iter()) {
        if g != w {
            failures += 1;
            if failures <= 10 {
                eprintln!("ENC MISMATCH {}\n  got  {}\n  want {}", g[0], g[1], w[1]);
            }
        }
    }
    (failures, want.len())
}

fn run_dir(dir: &std::path::Path) {
    let mut names: Vec<_> = std::fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|x| x == "bin"))
        .collect();
    names.sort();
    let mut total_fail = 0;
    let mut total = 0;
    for bin in names {
        let tsv = bin.with_extension("tsv");
        if !tsv.exists() {
            continue;
        }
        let (fail, n) = check_corpus(&bin, &tsv);
        eprintln!("{}: {} of {} differ", bin.display(), fail, n);
        total_fail += fail;
        total += n;
        let enc = bin.with_extension("enc.tsv");
        if enc.exists() {
            let (fail, n) = check_encsweep(&bin, &enc);
            eprintln!("{}: {} of {} q1..100 sweeps differ", enc.display(), fail, n);
            total_fail += fail;
            total += n;
        }
    }
    assert_eq!(
        total_fail, 0,
        "{} of {} corpus entries differ",
        total_fail, total
    );
}

#[test]
fn corpus_checked_in() {
    run_dir(&fixtures_dir().join("corpus"));
}

/// Large corpora kept outside the repository.
#[test]
fn corpus_big() {
    let Ok(dir) = std::env::var("GO_IMAGE_CORPUS_BIG") else {
        return;
    };
    run_dir(std::path::Path::new(&dir));
}

/// A reader that returns at most `n` bytes per call (Go: iotest.OneByteReader
/// for n == 1, iotest.HalfReader-like for other sizes).
struct ChunkReader<'a> {
    data: &'a [u8],
    n: usize,
}

impl std::io::Read for ChunkReader<'_> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let k = buf.len().min(self.n).min(self.data.len());
        buf[..k].copy_from_slice(&self.data[..k]);
        self.data = &self.data[k..];
        Ok(k)
    }
}

fn decode_digest(r: &mut dyn std::io::Read) -> String {
    match jpeg::decode(r) {
        Err(e) => format!("err:{}", e),
        Ok(m) => img_digest16(m.as_ref()),
    }
}

/// Go's decoder result does not depend on how the reader chunks its data
/// (checked in Go over 200 000 mutated inputs with bytes.Reader, bufio,
/// iotest.OneByteReader, iotest.HalfReader and fixed-size chunkings). The
/// port must not either: the byte-stuffing unread logic around buffer refills
/// is only reached with short reads.
#[test]
fn corpus_chunked_reads() {
    let dir = fixtures_dir().join("corpus");
    let mut n = 0;
    for name in ["gen.bin", "mut.bin"] {
        for (name, data) in read_corpus(&dir.join(name)) {
            if let Ok(c) = jpeg::decode_config(&mut &data[..])
                && c.width * c.height > 1 << 20
            {
                continue;
            }
            let want = decode_digest(&mut &data[..]);
            for chunk in [1usize, 2, 3, 7, 4093, 4094, 4095] {
                let got = decode_digest(&mut ChunkReader {
                    data: &data,
                    n: chunk,
                });
                assert_eq!(got, want, "{} with {}-byte reads", name, chunk);
            }
            n += 1;
        }
    }
    assert!(n > 1000);
}
