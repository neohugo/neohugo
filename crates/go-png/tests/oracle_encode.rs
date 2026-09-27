//! Differential encoder tests against fixtures produced by the Go oracle
//! (`tools/go-oracle/go-png`, go1.27.1, linux/arm64 under qemu — see
//! PORTING.md): synthetic images of every colour model at every
//! compression level, filter-heuristic edge cases, `Encoder.BufferPool`
//! reuse sequences, and the images of Go's writer tests and benchmarks.

mod common;

use std::sync::{Arc, Mutex};

use common::*;
use go_image::color::{self, Color, Palette};
use go_image::{Gray, Image, NRGBA, Paletted, RGBA, Rectangle, pt, rect};
use go_png::{CompressionLevel, Encoder, EncoderBuffer, EncoderBufferPool};

/// `synth.tsv.gz` (`go-png synth 0 2000 200`): per image, the encodings at
/// Default/No/BestSpeed/BestCompression/7, the decoded default encoding and
/// a failing-writer encode.
#[test]
fn synth() {
    let rows = read_tsv("synth.tsv.gz");
    assert_eq!(rows.len(), 2000);
    check_synth(&rows, 200).finish("synth");
}

/// `$GO_PNG_BIG/synth.tsv` (`go-png synth 0 N 400`).
#[test]
#[ignore]
fn synth_big() {
    if let Some(rows) = big_tsv("synth.tsv") {
        check_synth(&rows, 400).finish("synth_big");
    }
}

fn check_synth(rows: &[Vec<String>], max_dim: i64) -> Mismatches {
    let mut mm = Mismatches::default();
    for row in rows {
        let i: u64 = row[0].parse().unwrap();
        let mut r = Rng::seeded(i, 12345);
        let (m, desc) = gen_synth(&mut r, max_dim);
        mm.check(&format!("{i} desc"), &desc, &row[1]);
        let mut def = None;
        for (k, lvl) in LEVELS5.iter().enumerate() {
            let (s, b) = enc_str(&*m, *lvl);
            if *lvl == DEFAULT {
                def = b;
            }
            mm.check(&format!("{i} {desc} level {}", lvl.0), &s, &row[2 + k]);
        }
        let rt = match &def {
            Some(b) => dec_str(b).1,
            None => "-".into(),
        };
        mm.check(&format!("{i} {desc} roundtrip"), &rt, &row[7]);
        mm.check(&format!("{i} {desc} fail"), &fail_str(&*m, i), &row[8]);
    }
    mm
}

/// `filters.tsv.gz` (`go-png filters 0 4000`): the per-row filter types the
/// heuristic picks (ties, early breaks, every bytes-per-pixel) and the
/// encodings at the four levels.
#[test]
fn filters() {
    let rows = read_tsv("filters.tsv.gz");
    assert_eq!(rows.len(), 4000);
    check_filters(&rows).finish("filters");
}

/// `$GO_PNG_BIG/filters.tsv` (`go-png filters 0 N`).
#[test]
#[ignore]
fn filters_big() {
    if let Some(rows) = big_tsv("filters.tsv") {
        check_filters(&rows).finish("filters_big");
    }
}

fn check_filters(rows: &[Vec<String>]) -> Mismatches {
    let mut mm = Mismatches::default();
    for row in rows {
        let i: u64 = row[0].parse().unwrap();
        let mut r = Rng::seeded(i, 555);
        let (m, desc) = gen_filter_image(&mut r);
        mm.check(&format!("{i} desc"), &desc, &row[1]);
        let mut def = Vec::new();
        for (k, lvl) in LEVELS4.iter().enumerate() {
            let (s, b) = enc_str(&*m, *lvl);
            if *lvl == DEFAULT {
                def = b.expect("filter images encode");
            }
            mm.check(&format!("{i} {desc} level {}", lvl.0), &s, &row[3 + k]);
        }
        mm.check(&format!("{i} {desc} filters"), &row_filters(&def), &row[2]);
    }
    mm
}

/// The pool of Go's BenchmarkEncodeGrayWithBufferPool (holds one buffer).
#[derive(Default)]
struct OnePool(Mutex<Option<Box<EncoderBuffer>>>);

impl EncoderBufferPool for OnePool {
    fn get(&self) -> Option<Box<EncoderBuffer>> {
        self.0.lock().unwrap().take()
    }
    fn put(&self, b: Box<EncoderBuffer>) {
        *self.0.lock().unwrap() = Some(b);
    }
}

const POOL_LEVELS: [CompressionLevel; 5] = LEVELS5;

/// `pool.tsv.gz` (`go-png pool 0 400 120`): sequences of encodes through one
/// Encoder whose BufferPool always returns the same EncoderBuffer (zlib
/// writer, bufio.Writer and row buffers reused across images, levels and
/// failed encodes).
#[test]
fn pool() {
    let rows = read_tsv("pool.tsv.gz");
    assert_eq!(rows.len(), 400);
    check_pool(&rows, 120).finish("pool");
}

/// `$GO_PNG_BIG/pool.tsv` (`go-png pool 0 N 200`).
#[test]
#[ignore]
fn pool_big() {
    if let Some(rows) = big_tsv("pool.tsv") {
        check_pool(&rows, 200).finish("pool_big");
    }
}

fn check_pool(rows: &[Vec<String>], max_dim: i64) -> Mismatches {
    let mut mm = Mismatches::default();
    for row in rows {
        let i: u64 = row[0].parse().unwrap();
        let mut r = Rng::seeded(i, 999);
        let mut enc = Encoder {
            buffer_pool: Some(Arc::new(OnePool::default())),
            ..Default::default()
        };
        let steps = 2 + r.intn(6) as usize;
        assert_eq!(row.len(), 1 + steps, "row {i}");
        for s in 0..steps {
            let (m, desc) = gen_synth(&mut r, max_dim);
            let li = r.intn(POOL_LEVELS.len() as i64) as usize;
            enc.compression_level = POOL_LEVELS[li];
            let res = if r.intn(4) == 0 {
                let mut fw = FailWriter::new(r.intn(10) as usize);
                let e = enc.encode(&mut fw, &*m);
                format!(
                    "fail{}:{}:{}:{}",
                    fw.k,
                    sha16(&fw.buf),
                    fw.buf.len(),
                    err_str(&e)
                )
            } else {
                let mut buf = Vec::new();
                let e = enc.encode(&mut buf, &*m);
                format!("{}:{}:{}", sha16(&buf), buf.len(), err_str(&e))
            };
            mm.check(
                &format!("{i} step {s}"),
                &format!("{desc}|L{li}|{res}"),
                &row[1 + s],
            );
        }
    }
    mm
}

/// The images of `go-png gotests` (Go's writer tests, benchmarks and
/// ExampleEncode, palette-length and size errors), built exactly like the
/// oracle.
fn gotests_images() -> Vec<(String, Box<dyn Image>)> {
    let mut cases: Vec<(String, Box<dyn Image>)> = Vec::new();
    // TestWriterPaletted.
    for plen in [256usize, 128, 16, 4, 2] {
        let pal: Palette = (0..plen)
            .map(|i| {
                Color::NRGBA(color::NRGBA {
                    r: i as u8,
                    g: i as u8,
                    b: i as u8,
                    a: 255,
                })
            })
            .collect();
        let mut m = Paletted::new(rect(0, 0, 32, 16), pal);
        let mut i = 0;
        for y in 0..16 {
            for x in 0..32 {
                m.set_color_index(x, y, (i % plen) as u8);
                i += 1;
            }
        }
        cases.push((format!("writer-paletted-{plen}"), Box::new(m)));
    }
    // TestWriterLevels.
    cases.push((
        "writer-levels".into(),
        Box::new(NRGBA::new(rect(0, 0, 100, 100))),
    ));
    // TestSubImage.
    let mut sub = RGBA::new(rect(0, 0, 256, 256));
    for y in 0..256 {
        for x in 0..256 {
            sub.set(
                x,
                y,
                Color::RGBA(color::RGBA {
                    r: x as u8,
                    g: y as u8,
                    b: 0,
                    a: 255,
                }),
            );
        }
    }
    cases.push((
        "subimage".into(),
        Box::new(sub.sub_image(rect(50, 30, 250, 130))),
    ));
    // TestWriteRGBA.
    let (width, height) = (640i64, 480i64);
    let transparent = RGBA::new(rect(0, 0, width, height));
    let mut opaque = RGBA::new(rect(0, 0, width, height));
    let mut mixed = RGBA::new(rect(0, 0, width, height));
    let mut translucent = RGBA::new(rect(0, 0, width, height));
    for y in 0..height {
        for x in 0..width {
            let opaque_color = Color::RGBA(color::RGBA {
                r: x as u8,
                g: y as u8,
                b: (y + x) as u8,
                a: 255,
            });
            let translucent_color = Color::RGBA(color::RGBA {
                r: x as u8 % 128,
                g: y as u8 % 128,
                b: (y + x) as u8 % 128,
                a: 128,
            });
            opaque.set(x, y, opaque_color);
            translucent.set(x, y, translucent_color);
            if y % 2 == 0 {
                mixed.set(x, y, opaque_color);
            }
        }
    }
    cases.push(("rgba-transparent".into(), Box::new(transparent)));
    cases.push(("rgba-opaque".into(), Box::new(opaque)));
    cases.push(("rgba-mixed".into(), Box::new(mixed)));
    cases.push(("rgba-translucent".into(), Box::new(translucent)));
    // Benchmarks.
    cases.push((
        "bench-gray".into(),
        Box::new(Gray::new(rect(0, 0, width, height))),
    ));
    let mut nrgb_opaque = NRGBA::new(rect(0, 0, width, height));
    let mut rgb_opaque = RGBA::new(rect(0, 0, width, height));
    let mut bench_rgba = RGBA::new(rect(0, 0, width, height));
    for y in 0..height {
        for x in 0..width {
            nrgb_opaque.set(
                x,
                y,
                Color::NRGBA(color::NRGBA {
                    r: 0,
                    g: 0,
                    b: 0,
                    a: 255,
                }),
            );
            rgb_opaque.set(
                x,
                y,
                Color::RGBA(color::RGBA {
                    r: 0,
                    g: 0,
                    b: 0,
                    a: 255,
                }),
            );
            let percent = (x + y) % 100;
            let a = if percent < 10 {
                percent as u8
            } else if percent < 40 {
                0
            } else {
                255
            };
            bench_rgba.set(
                x,
                y,
                Color::NRGBA(color::NRGBA {
                    r: x as u8,
                    g: y as u8,
                    b: (x * y) as u8,
                    a,
                }),
            );
        }
    }
    cases.push(("bench-nrgb-opaque".into(), Box::new(nrgb_opaque)));
    cases.push((
        "bench-nrgba".into(),
        Box::new(NRGBA::new(rect(0, 0, width, height))),
    ));
    cases.push((
        "bench-paletted".into(),
        Box::new(Paletted::new(
            rect(0, 0, width, height),
            Palette(vec![
                Color::RGBA(color::RGBA {
                    r: 0,
                    g: 0,
                    b: 0,
                    a: 255,
                }),
                Color::RGBA(color::RGBA {
                    r: 255,
                    g: 255,
                    b: 255,
                    a: 255,
                }),
            ]),
        )),
    ));
    cases.push(("bench-rgb-opaque".into(), Box::new(rgb_opaque)));
    cases.push(("bench-rgba".into(), Box::new(bench_rgba)));
    // ExampleEncode.
    let mut ex = NRGBA::new(rect(0, 0, 256, 256));
    for y in 0..256i64 {
        for x in 0..256i64 {
            ex.set(
                x,
                y,
                Color::NRGBA(color::NRGBA {
                    r: ((x + y) & 255) as u8,
                    g: ((x + y) << 1 & 255) as u8,
                    b: ((x + y) << 2 & 255) as u8,
                    a: 255,
                }),
            );
        }
    }
    cases.push(("example-encode".into(), Box::new(ex)));
    // Palette length errors (the header and IHDR are still written).
    let pal257: Palette = (0..257)
        .map(|i| {
            Color::NRGBA(color::NRGBA {
                r: i as u8,
                g: i as u8,
                b: i as u8,
                a: i as u8,
            })
        })
        .collect();
    cases.push((
        "pal-0".into(),
        Box::new(Paletted::new(rect(0, 0, 3, 2), Palette(vec![]))),
    ));
    cases.push((
        "pal-257".into(),
        Box::new(Paletted::new(rect(0, 0, 3, 2), pal257)),
    ));
    // Size errors (image.Rectangle is itself an image).
    cases.push(("size-wide".into(), Box::new(rect(0, 0, 1 << 32, 1))));
    cases.push(("size-tall".into(), Box::new(rect(0, 0, 1, 1 << 32))));
    cases.push((
        "size-neg".into(),
        Box::new(Rectangle {
            min: pt(0, 0),
            max: pt(-1, -2),
        }),
    ));
    cases.push(("size-zero".into(), Box::new(rect(0, 0, 0, 5))));
    cases
}

/// `gotests.tsv` (`go-png gotests`). Several images are 640x480, i.e.
/// multi-block flate streams whose Huffman-table reuse decisions go through
/// the float `EstimatedBits` path.
#[test]
fn gotests() {
    let rows = read_tsv("gotests.tsv");
    let cases = gotests_images();
    assert_eq!(rows.len(), cases.len());
    let mut mm = Mismatches::default();
    for ((name, m), row) in cases.iter().zip(&rows) {
        assert_eq!(name, &row[0]);
        let mut def = None;
        for (k, lvl) in LEVELS4.iter().enumerate() {
            let (s, b) = enc_str(&**m, *lvl);
            if *lvl == DEFAULT {
                def = b;
            }
            mm.check(&format!("{name} level {}", lvl.0), &s, &row[1 + k]);
        }
        let fs = match &def {
            Some(b) => sha16(row_filters(b).as_bytes()),
            None => "-".into(),
        };
        mm.check(&format!("{name} filters"), &fs, &row[5]);
    }
    mm.finish("gotests");
}
