//! Differential decoder tests (and re-encodings of decoded images) against
//! fixtures produced by the Go oracle (`tools/go-oracle/go-png`, go1.27.1):
//! real PNG files, every prefix of small files (exact error strings), a
//! corpus of generated PNGs (every colour type / bit depth / interlacing,
//! PLTE/tRNS variants, split IDATs, structural errors) and mutations of it,
//! decoded through `bytes.Reader`, `image.Decode` and short-read readers.

mod common;

use common::*;
use go_image::color::{self, Color, Palette, palette};
use go_image::draw::{self, Op};
use go_image::{Gray, Gray16, Image, NRGBA, NRGBA64, Paletted, RGBA, RGBA64, Rectangle, rect};

/// Go's `m.(interface{ SubImage(image.Rectangle) image.Image })` for the
/// types the decoder returns.
fn sub_image(m: &dyn Image, r: Rectangle) -> Option<Box<dyn Image>> {
    macro_rules! try_sub {
        ($t:ty) => {
            if let Some(m) = m.downcast_ref::<$t>() {
                return Some(Box::new(m.sub_image(r)));
            }
        };
    }
    try_sub!(Gray);
    try_sub!(Gray16);
    try_sub!(RGBA);
    try_sub!(RGBA64);
    try_sub!(NRGBA);
    try_sub!(NRGBA64);
    try_sub!(Paletted);
    None
}

/// `files.tsv.gz` (`go-png files . <list>`): Go's image/png and image
/// testdata (incl. PngSuite), the 11 PNGs of the golden seeksnack build and
/// a few repository PNGs: DecodeConfig, Decode, image.Decode, encodings of
/// the decoded image at the four levels, of its NRGBA/RGBA/Gray/WebSafe/
/// Plan9[:16]/{Black,Transparent} conversions and of an odd sub-image, and
/// a failing-writer encode.
#[test]
fn files() {
    let rows = read_tsv("files.tsv.gz");
    assert_eq!(rows.len(), 75);
    check_files(&rows, &|name| read_fixture(name)).finish("files");
}

/// `$GO_PNG_BIG/files.tsv` (`go-png files <root> <list>`; the root directory
/// is named in `$GO_PNG_BIG/files.root`).
#[test]
#[ignore]
fn files_big() {
    if let Some(rows) = big_tsv("files.tsv") {
        let root = big_root();
        check_files(&rows, &|name| std::fs::read(root.join(name)).unwrap()).finish("files_big");
    }
}

fn big_root() -> std::path::PathBuf {
    let root = std::fs::read_to_string(big_path("files.root").expect("files.root")).unwrap();
    std::path::PathBuf::from(root.trim())
}

fn check_files(rows: &[Vec<String>], read: &dyn Fn(&str) -> Vec<u8>) -> Mismatches {
    let mut mm = Mismatches::default();
    for row in rows {
        let name = &row[0];
        let data = read(name);
        mm.check(&format!("{name} sha"), &sha16(&data), &row[1]);
        mm.check(&format!("{name} cfg"), &cfg_str(&data), &row[2]);
        let (m, d) = dec_str(&data);
        mm.check(&format!("{name} dec"), &d, &row[3]);
        mm.check(&format!("{name} reg"), &reg_str(&data), &row[4]);
        let Some(m) = m else {
            assert_eq!(row.len(), 5, "{name}");
            continue;
        };
        let mut col = 5;
        for lvl in LEVELS4 {
            mm.check(
                &format!("{name} level {}", lvl.0),
                &enc_str(&*m, lvl).0,
                &row[col],
            );
            col += 1;
        }
        let b = m.bounds();
        let mut nrgba = NRGBA::new(b);
        draw::draw(&mut nrgba, b, &*m, b.min, Op::Src);
        let mut rgba = RGBA::new(b);
        draw::draw(&mut rgba, b, &*m, b.min, Op::Src);
        let mut gray = Gray::new(b);
        draw::draw(&mut gray, b, &*m, b.min, Op::Src);
        let mut pal8 = Paletted::new(b, palette::web_safe());
        draw::draw(&mut pal8, b, &*m, b.min, Op::Src);
        let mut pal4 = Paletted::new(b, Palette(palette::plan9()[..16].to_vec()));
        draw::draw(&mut pal4, b, &*m, b.min, Op::Src);
        let mut pal1 = Paletted::new(
            b,
            Palette(vec![
                Color::Gray16(color::BLACK),
                Color::Alpha16(color::TRANSPARENT),
            ]),
        );
        draw::draw(&mut pal1, b, &*m, b.min, Op::Src);
        let convs: [(&str, &dyn Image); 6] = [
            ("nrgba", &nrgba),
            ("rgba", &rgba),
            ("gray", &gray),
            ("pal8", &pal8),
            ("pal4", &pal4),
            ("pal1", &pal1),
        ];
        for (cname, c) in convs {
            mm.check(
                &format!("{name} {cname}"),
                &enc_str(c, DEFAULT).0,
                &row[col],
            );
            col += 1;
        }
        // A sub-image with odd offsets (Pix not starting at Rect.Min).
        let r = rect(
            b.min.x + b.dx() / 3,
            b.min.y + b.dy() / 5,
            b.max.x - b.dx() / 7,
            b.max.y - b.dy() / 9,
        );
        let sub = match sub_image(&*m, r) {
            Some(s) => enc_str(&*s, DEFAULT).0,
            None => "-".into(),
        };
        mm.check(&format!("{name} sub"), &sub, &row[col]);
        mm.check(
            &format!("{name} fail"),
            &fail_str(&*m, data.len() as u64),
            &row[col + 1],
        );
    }
    mm
}

/// `trunc.tsv.gz` (`go-png trunc . <list>`): DecodeConfig and Decode of
/// every prefix of the small test files (sampled prefixes of larger ones):
/// the exact `unexpected EOF` / `not enough pixel data` / ... errors.
#[test]
fn truncated() {
    check_trunc(&read_tsv("trunc.tsv.gz"), &|name| read_fixture(name)).finish("truncated");
}

/// `$GO_PNG_BIG/trunc.tsv` (`go-png trunc <root> <list>`, root as for
/// `files_big`).
#[test]
#[ignore]
fn truncated_big() {
    if let Some(rows) = big_tsv("trunc.tsv") {
        let root = big_root();
        check_trunc(&rows, &|name| std::fs::read(root.join(name)).unwrap()).finish("truncated_big");
    }
}

fn check_trunc(rows: &[Vec<String>], read: &dyn Fn(&str) -> Vec<u8>) -> Mismatches {
    let mut mm = Mismatches::default();
    let mut cur: (String, Vec<u8>) = (String::new(), Vec::new());
    for row in rows {
        if row[0] != cur.0 {
            cur = (row[0].clone(), read(&row[0]));
        }
        let l: usize = row[1].parse().unwrap();
        let d = &cur.1[..l];
        mm.check(&format!("{} [:{l}] cfg", row[0]), &cfg_str(d), &row[2]);
        mm.check(&format!("{} [:{l}] dec", row[0]), &dec_str(d).1, &row[3]);
    }
    mm
}

/// The oracle's errReader: delivers `data[..k]`, then fails every read.
struct ErrReader<'a> {
    data: &'a [u8],
    k: usize,
    pos: usize,
}

impl std::io::Read for ErrReader<'_> {
    fn read(&mut self, p: &mut [u8]) -> std::io::Result<usize> {
        if self.pos >= self.k {
            return Err(std::io::Error::other("boom"));
        }
        let src = &self.data[self.pos..self.k.min(self.data.len())];
        let n = p.len().min(src.len());
        p[..n].copy_from_slice(&src[..n]);
        self.pos += n;
        Ok(n)
    }
}

/// `errread.tsv.gz` (`go-png errread . <list>`): Decode through a reader
/// that fails after k bytes. Depending on where the decoder meets the error
/// it is returned as is ("boom") or wrapped ("png: invalid format: boom",
/// when the zlib reader hits it while reading the Adler-32 checksum).
#[test]
fn reader_errors() {
    check_errread(&read_tsv("errread.tsv.gz"), &|name| read_fixture(name)).finish("reader_errors");
}

/// `$GO_PNG_BIG/errread.tsv` (`go-png errread <root> <list>`).
#[test]
#[ignore]
fn reader_errors_big() {
    if let Some(rows) = big_tsv("errread.tsv") {
        let root = big_root();
        check_errread(&rows, &|name| std::fs::read(root.join(name)).unwrap())
            .finish("reader_errors_big");
    }
}

fn check_errread(rows: &[Vec<String>], read: &dyn Fn(&str) -> Vec<u8>) -> Mismatches {
    let mut mm = Mismatches::default();
    let mut cur: (String, Vec<u8>) = (String::new(), Vec::new());
    for row in rows {
        if row[0] != cur.0 {
            cur = (row[0].clone(), read(&row[0]));
        }
        let k: usize = row[1].parse().unwrap();
        let mut r = ErrReader {
            data: &cur.1,
            k,
            pos: 0,
        };
        mm.check(&format!("{} k={k}", row[0]), &dec_from(&mut r).1, &row[2]);
    }
    mm
}

/// The oracle's record: DecodeConfig, Decode, image.Decode, Decode through
/// iotest.OneByteReader and iotest.HalfReader ("=" when equal to Decode) and
/// the default encoding of the decoded image.
fn check_record(mm: &mut Mismatches, tag: &str, data: &[u8], row: &[String]) {
    mm.check(&format!("{tag} cfg"), &cfg_str(data), &row[1]);
    if let Ok(cfg) = go_png::decode_config(&mut &data[..])
        && cfg.width * cfg.height > 4_000_000
    {
        // Refused by the oracle (allocation would exhaust memory).
        mm.check(&format!("{tag} skip"), "skip", &row[2]);
        return;
    }
    let (m, d) = dec_str(data);
    mm.check(&format!("{tag} dec"), &d, &row[2]);
    mm.check(&format!("{tag} reg"), &reg_str(data), &row[3]);
    mm.check(
        &format!("{tag} one-byte reads"),
        &dec_with(&mut OneByteReader(data), &d),
        &row[4],
    );
    mm.check(
        &format!("{tag} half reads"),
        &dec_with(&mut HalfReader(data), &d),
        &row[5],
    );
    match m {
        Some(m) => mm.check(&format!("{tag} enc"), &enc_str(&*m, DEFAULT).0, &row[6]),
        None => assert_eq!(row.len(), 6, "{tag}"),
    }
}

/// `gen.bin.gz` (`go-png mkpng 0 3000`) + `gen.tsv.gz` (`go-png record`).
#[test]
fn generated_corpus() {
    let corpus = read_corpus("gen.bin.gz");
    assert_eq!(corpus.len(), 3000);
    check_corpus(&corpus, &read_tsv("gen.tsv.gz"), "gen").finish("generated_corpus");
}

/// `mut.tsv.gz` (`go-png mutate 0 6000 gen.bin mut.bin` + `record`); the
/// mutations are rebuilt here from the seeds with the ported mutator.
#[test]
fn mutated_corpus() {
    let seeds = read_corpus("gen.bin.gz");
    let rows = read_tsv("mut.tsv.gz");
    assert_eq!(rows.len(), 6000);
    check_mutations(&seeds, &rows).finish("mutated_corpus");
}

/// `$GO_PNG_BIG/{gen.bin,gen.tsv,mut.tsv}` (`mkpng 0 N`, `record`,
/// `mutate 0 M`, `record`).
#[test]
#[ignore]
fn corpus_big() {
    let (Some(bin), Some(rows)) = (big_path("gen.bin"), big_tsv("gen.tsv")) else {
        return;
    };
    let corpus = parse_corpus(&std::fs::read(bin).unwrap());
    check_corpus(&corpus, &rows, "gen").finish("corpus_big gen");
    if let Some(rows) = big_tsv("mut.tsv") {
        check_mutations(&corpus, &rows).finish("corpus_big mut");
    }
}

fn check_corpus(corpus: &[Vec<u8>], rows: &[Vec<String>], tag: &str) -> Mismatches {
    assert_eq!(rows.len(), corpus.len());
    let mut mm = Mismatches::default();
    for (i, (data, row)) in corpus.iter().zip(rows).enumerate() {
        assert_eq!(row[0], i.to_string());
        check_record(&mut mm, &format!("{tag} {i}"), data, row);
    }
    mm
}

fn check_mutations(seeds: &[Vec<u8>], rows: &[Vec<String>]) -> Mismatches {
    let mut mm = Mismatches::default();
    for (i, row) in rows.iter().enumerate() {
        assert_eq!(row[0], i.to_string());
        let mut r = Rng::seeded(i as u64, 4242);
        let seed = &seeds[r.intn(seeds.len() as i64) as usize];
        let data = mutate(&mut r, seed);
        check_record(&mut mm, &format!("mut {i}"), &data, row);
    }
    mm
}
