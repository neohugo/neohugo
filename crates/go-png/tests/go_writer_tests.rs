//! Ports of go1.27.1 `image/png/writer_test.go` (plus the BufferPool
//! benchmark as a byte-identity test).

mod common;

use std::io::Read;
use std::sync::{Arc, Mutex};

use common::*;
use go_image::color::{self, Color, Palette};
use go_image::draw::{self, Op};
use go_image::{Gray, Image, NRGBA, Paletted, RGBA, rect};
use go_png::{Encoder, EncoderBuffer, EncoderBufferPool, NO_COMPRESSION};

// Go: writer_test.go:diff
fn diff(m0: &dyn Image, m1: &dyn Image) -> Result<(), String> {
    let (b0, b1) = (m0.bounds(), m1.bounds());
    if !b0.size().eq(b1.size()) {
        return Err(format!("dimensions differ: {b0} vs {b1}"));
    }
    let dx = b1.min.x - b0.min.x;
    let dy = b1.min.y - b0.min.y;
    for y in b0.min.y..b0.max.y {
        for x in b0.min.x..b0.max.x {
            let c0 = m0.at(x, y);
            let c1 = m1.at(x + dx, y + dy);
            if c0.rgba() != c1.rgba() {
                return Err(format!("colors differ at ({x}, {y}): {c0:?} vs {c1:?}"));
            }
        }
    }
    Ok(())
}

// Go: writer_test.go:encodeDecode
fn encode_decode(m: &dyn Image) -> Result<Box<dyn Image>, go_png::Error> {
    let mut b = Vec::new();
    go_png::encode(&mut b, m)?;
    go_png::decode(&mut &b[..])
}

// Go: writer_test.go:convertToNRGBA
fn convert_to_nrgba(m: &dyn Image) -> NRGBA {
    let b = m.bounds();
    let mut ret = NRGBA::new(b);
    draw::draw(&mut ret, b, m, b.min, Op::Src);
    ret
}

// Go: reader_test.go:filenames
const FILENAMES: [&str; 35] = [
    "basn0g01",
    "basn0g01-30",
    "basn0g02",
    "basn0g02-29",
    "basn0g04",
    "basn0g04-31",
    "basn0g08",
    "basn0g16",
    "basn2c08",
    "basn2c16",
    "basn3p01",
    "basn3p02",
    "basn3p04",
    "basn3p04-31i",
    "basn3p08",
    "basn3p08-trns",
    "basn4a08",
    "basn4a16",
    "basn6a08",
    "basn6a16",
    "ftbbn0g01",
    "ftbbn0g02",
    "ftbbn0g04",
    "ftbbn2c16",
    "ftbbn3p08",
    "ftbgn2c16",
    "ftbgn3p08",
    "ftbrn2c08",
    "ftbwn0g16",
    "ftbwn3p08",
    "ftbyn3p08",
    "ftp0n0g08",
    "ftp0n2c08",
    "ftp0n3p08",
    "ftp1n3p08",
];

// Go: writer_test.go:TestWriter
#[test]
fn test_writer() {
    for fn_ in FILENAMES {
        let data = read_fixture(&format!("gotestdata/pngsuite/{fn_}.png"));
        // Read the image.
        let m0 = go_png::decode(&mut &data[..]).unwrap_or_else(|e| panic!("{fn_}: {e}"));
        // Read the image again, encode it, and decode it.
        let m1 = go_png::decode(&mut &data[..]).unwrap_or_else(|e| panic!("{fn_}: {e}"));
        let m2 = encode_decode(&*m1).unwrap_or_else(|e| panic!("{fn_}: {e}"));
        // Compare the two.
        diff(&*m0, &*m2).unwrap_or_else(|e| panic!("{fn_}: {e}"));
    }
}

// Go: writer_test.go:TestWriterPaletted
#[test]
fn test_writer_paletted() {
    const WIDTH: usize = 32;
    const HEIGHT: usize = 16;
    let test_cases: [(usize, u8, usize); 5] = [
        (256, 8, (1 + WIDTH) * HEIGHT),
        (128, 8, (1 + WIDTH) * HEIGHT),
        (16, 4, (1 + WIDTH / 2) * HEIGHT),
        (4, 2, (1 + WIDTH / 4) * HEIGHT),
        (2, 1, (1 + WIDTH / 8) * HEIGHT),
    ];
    for (plen, bitdepth, datalen) in test_cases {
        // Create a paletted image with the correct palette length
        let palette: Palette = (0..plen)
            .map(|i| {
                Color::NRGBA(color::NRGBA {
                    r: i as u8,
                    g: i as u8,
                    b: i as u8,
                    a: 255,
                })
            })
            .collect();
        let mut m0 = Paletted::new(rect(0, 0, WIDTH as i64, HEIGHT as i64), palette);
        let mut i = 0;
        for y in 0..HEIGHT as i64 {
            for x in 0..WIDTH as i64 {
                m0.set_color_index(x, y, (i % plen) as u8);
                i += 1;
            }
        }

        // Encode the image
        let mut data = Vec::new();
        go_png::encode(&mut data, &m0).unwrap();
        const CHUNK_FIELDS_LENGTH: usize = 12; // 4 bytes for length, name and crc
        let mut i = 8;
        while i < data.len() - CHUNK_FIELDS_LENGTH {
            let length = u32::from_be_bytes(data[i..i + 4].try_into().unwrap()) as usize;
            match &data[i + 4..i + 8] {
                b"IHDR" => assert_eq!(data[i + 8 + 8], bitdepth, "plen-{plen}: bitdepth"),
                b"IDAT" => {
                    // Uncompress the image data
                    let mut out = Vec::new();
                    flate2::read::ZlibDecoder::new(&data[i + 8..i + 8 + length])
                        .read_to_end(&mut out)
                        .expect("got error while reading image data");
                    assert_eq!(out.len(), datalen, "plen-{plen}: uncompressed data length");
                }
                _ => {}
            }
            i += CHUNK_FIELDS_LENGTH + length;
        }
    }
}

// Go: writer_test.go:TestWriterLevels
#[test]
fn test_writer_levels() {
    let m = NRGBA::new(rect(0, 0, 100, 100));
    let (mut b1, mut b2) = (Vec::new(), Vec::new());
    Encoder::default().encode(&mut b1, &m).unwrap();
    let noenc = Encoder {
        compression_level: NO_COMPRESSION,
        ..Default::default()
    };
    noenc.encode(&mut b2, &m).unwrap();

    assert!(
        b2.len() > b1.len(),
        "DefaultCompression encoding was larger than NoCompression encoding"
    );
    go_png::decode(&mut &b1[..]).expect("cannot decode DefaultCompression");
    go_png::decode(&mut &b2[..]).expect("cannot decode NoCompression");
}

// Go: writer_test.go:TestSubImage
#[test]
fn test_sub_image() {
    let mut m0 = RGBA::new(rect(0, 0, 256, 256));
    for y in 0..256 {
        for x in 0..256 {
            m0.set(
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
    let m0 = m0.sub_image(rect(50, 30, 250, 130));
    let m1 = encode_decode(&m0).unwrap();
    diff(&m0, &*m1).unwrap();
}

// Go: writer_test.go:TestWriteRGBA
#[test]
fn test_write_rgba() {
    const WIDTH: i64 = 640;
    const HEIGHT: i64 = 480;
    let transparent_img = RGBA::new(rect(0, 0, WIDTH, HEIGHT));
    let mut opaque_img = RGBA::new(rect(0, 0, WIDTH, HEIGHT));
    let mut mixed_img = RGBA::new(rect(0, 0, WIDTH, HEIGHT));
    let mut translucent_img = RGBA::new(rect(0, 0, WIDTH, HEIGHT));
    for y in 0..HEIGHT {
        for x in 0..WIDTH {
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
            opaque_img.set(x, y, opaque_color);
            translucent_img.set(x, y, translucent_color);
            if y % 2 == 0 {
                mixed_img.set(x, y, opaque_color);
            }
        }
    }

    let test_cases: [(&str, &dyn Image); 4] = [
        ("Transparent RGBA", &transparent_img),
        ("Opaque RGBA", &opaque_img),
        ("50/50 Transparent/Opaque RGBA", &mixed_img),
        ("RGBA with variable alpha", &translucent_img),
    ];
    for (name, m0) in test_cases {
        let m1 = encode_decode(m0).unwrap_or_else(|e| panic!("{name}: {e}"));
        diff(&convert_to_nrgba(m0), &*m1).unwrap_or_else(|e| panic!("{name}: {e}"));
    }
}

// Go: writer_test.go:pool
#[derive(Default)]
struct Pool(Mutex<Option<Box<EncoderBuffer>>>);

impl EncoderBufferPool for Pool {
    fn get(&self) -> Option<Box<EncoderBuffer>> {
        self.0.lock().unwrap().take()
    }
    fn put(&self, b: Box<EncoderBuffer>) {
        *self.0.lock().unwrap() = Some(b);
    }
}

// Go: writer_test.go:BenchmarkEncodeGrayWithBufferPool, as a test: encoding
// through a pool that hands back the previous buffer gives the same bytes.
#[test]
fn encode_gray_with_buffer_pool() {
    let img = Gray::new(rect(0, 0, 640, 480));
    let mut want = Vec::new();
    go_png::encode(&mut want, &img).unwrap();
    let e = Encoder {
        buffer_pool: Some(Arc::new(Pool::default())),
        ..Default::default()
    };
    for _ in 0..3 {
        let mut got = Vec::new();
        e.encode(&mut got, &img).unwrap();
        assert_eq!(got, want);
    }
}
