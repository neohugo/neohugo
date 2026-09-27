//! Ports of go1.27.1 `image/png/reader_test.go` (and the seed corpus of
//! `fuzz_test.go`). The PngSuite files, their `.sng` dumps and the other
//! testdata files are in `tests/fixtures/gotestdata` (BSD, see LICENSE-go).

mod common;

use std::fmt::Write as _;

use common::*;
use go_image::color::{self, Color, Model, Palette};
use go_image::{Gray, Image, Paletted};
use go_png::{BEST_COMPRESSION, BEST_SPEED, DEFAULT_COMPRESSION, Encoder, Error, NO_COMPRESSION};

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

// Go: reader_test.go:filenamesPaletted
const FILENAMES_PALETTED: [&str; 5] = [
    "basn3p01",
    "basn3p02",
    "basn3p04",
    "basn3p08",
    "basn3p08-trns",
];

const PNG_HEADER: &[u8] = b"\x89PNG\r\n\x1a\n";

// Go: reader_test.go:readPNG
fn read_png(filename: &str) -> Result<Box<dyn Image>, Error> {
    let data = read_fixture(&format!("gotestdata/{filename}"));
    go_png::decode(&mut &data[..])
}

// Go: reader_test.go:fakebKGDs
fn fake_bkgd(filename: &str) -> Option<&'static str> {
    Some(match filename {
        "ftbbn0g01" | "ftbbn0g02" | "ftbbn0g04" => "bKGD {gray: 0;}\n",
        "ftbbn2c16" => "bKGD {red: 0;  green: 0;  blue: 65535;}\n",
        "ftbbn3p08" | "ftbgn3p08" | "ftbyn3p08" => "bKGD {index: 245}\n",
        "ftbgn2c16" => "bKGD {red: 0;  green: 65535;  blue: 0;}\n",
        "ftbrn2c08" => "bKGD {red: 255;  green: 0;  blue: 0;}\n",
        "ftbwn0g16" => "bKGD {gray: 65535;}\n",
        "ftbwn3p08" => "bKGD {index: 0}\n",
        _ => return None,
    })
}

// Go: reader_test.go:fakegAMAs
fn fake_gama(filename: &str) -> Option<&'static str> {
    match filename {
        "ftbbn0g01" => Some(""),
        "ftbbn0g02" => Some("gAMA {0.45455}\n"),
        _ => None,
    }
}

// Go: reader_test.go:fakeIHDRUsings
fn fake_ihdr_using(filename: &str) -> Option<&'static str> {
    match filename {
        "ftbbn0g01" | "ftbbn0g02" | "ftbbn0g04" | "ftbwn0g16" => Some("    using grayscale;\n"),
        "ftbbn2c16" | "ftbgn2c16" | "ftbrn2c08" => Some("    using color;\n"),
        _ => None,
    }
}

// An approximation of the sng command-line tool.
// Go: reader_test.go:sng
fn sng(filename: &str, png: &dyn Image) -> String {
    let mut w = String::new();
    let bounds = png.bounds();
    let cm = png.color_model();
    let mut bitdepth = match cm {
        Model::RGBA | Model::NRGBA | Model::Alpha | Model::Gray => 8,
        _ => 16,
    };
    let cpm: Option<&Palette> = match &cm {
        Model::Palette(p) => Some(p),
        _ => None,
    };
    let mut paletted: Option<&Paletted> = None;
    if let Some(cpm) = cpm {
        bitdepth = if cpm.len() <= 2 {
            1
        } else if cpm.len() <= 4 {
            2
        } else if cpm.len() <= 16 {
            4
        } else {
            8
        };
        paletted = png.downcast_ref::<Paletted>();
    }

    // Write the filename and IHDR.
    w += &format!("#SNG: from {filename}.png\nIHDR {{\n");
    let _ = writeln!(
        w,
        "    width: {}; height: {}; bitdepth: {};",
        bounds.dx(),
        bounds.dy(),
        bitdepth
    );
    if let Some(s) = fake_ihdr_using(filename) {
        w += s;
    } else {
        w += match &cm {
            Model::RGBA | Model::RGBA64 => "    using color;\n",
            Model::NRGBA | Model::NRGBA64 => "    using color alpha;\n",
            Model::Gray | Model::Gray16 => "    using grayscale;\n",
            _ if cpm.is_some() => "    using color palette;\n",
            _ => "unknown PNG decoder color model\n",
        };
    }
    w += "}\n";

    // We fake a gAMA chunk. The test files have a gAMA chunk but the go PNG
    // parser ignores it (the PNG spec section 11.3 says "Ancillary chunks may
    // be ignored by a decoder").
    w += fake_gama(filename).unwrap_or("gAMA {1.0000}\n");

    // Write the PLTE and tRNS (if applicable).
    let mut use_transparent = false;
    if let Some(cpm) = cpm {
        let mut last_alpha: i64 = -1;
        w += "PLTE {\n";
        for (i, c) in cpm.iter().enumerate() {
            let (r, g, b, a) = match c {
                Color::RGBA(c) => (c.r, c.g, c.b, 0xff),
                Color::NRGBA(c) => (c.r, c.g, c.b, c.a),
                _ => panic!("unknown palette color type"),
            };
            if a != 0xff {
                last_alpha = i as i64;
            }
            let _ = writeln!(
                w,
                "    ({r:3},{g:3},{b:3})     # rgb = (0x{r:02x},0x{g:02x},0x{b:02x})"
            );
        }
        w += "}\n";
        if let Some(s) = fake_bkgd(filename) {
            w += s;
        }
        if last_alpha != -1 {
            w += "tRNS {\n";
            for c in &cpm[..=last_alpha as usize] {
                let (_, _, _, a) = c.rgba();
                let _ = write!(w, " {}", a >> 8);
            }
            w += "}\n";
        }
    } else if filename.starts_with("ft") {
        if let Some(s) = fake_bkgd(filename) {
            w += s;
        }
        // We fake a tRNS chunk. The test files' grayscale and truecolor
        // transparent images all have their top left corner transparent.
        match png.at(0, 0) {
            Color::NRGBA(c) if c.a == 0 => {
                use_transparent = true;
                w += "tRNS {\n";
                match filename {
                    "ftbbn0g01" | "ftbbn0g02" | "ftbbn0g04" => {
                        let _ = writeln!(w, "    gray: {};", c.r);
                    }
                    _ => {
                        let _ = writeln!(w, "    red: {}; green: {}; blue: {};", c.r, c.g, c.b);
                    }
                }
                w += "}\n";
            }
            Color::NRGBA64(c) if c.a == 0 => {
                use_transparent = true;
                w += "tRNS {\n";
                match filename {
                    "ftbwn0g16" => {
                        let _ = writeln!(w, "    gray: {};", c.r);
                    }
                    _ => {
                        let _ = writeln!(w, "    red: {}; green: {}; blue: {};", c.r, c.g, c.b);
                    }
                }
                w += "}\n";
            }
            _ => {}
        }
    }

    // Write the IMAGE.
    w += "IMAGE {\n    pixels hex\n";
    for y in bounds.min.y..bounds.max.y {
        match &cm {
            Model::Gray => {
                for x in bounds.min.x..bounds.max.x {
                    let Color::Gray(g) = png.at(x, y) else {
                        panic!("not color.Gray")
                    };
                    let _ = write!(w, "{:02x}", g.y);
                }
            }
            Model::Gray16 => {
                for x in bounds.min.x..bounds.max.x {
                    let Color::Gray16(g) = png.at(x, y) else {
                        panic!("not color.Gray16")
                    };
                    let _ = write!(w, "{:04x} ", g.y);
                }
            }
            Model::RGBA => {
                for x in bounds.min.x..bounds.max.x {
                    let Color::RGBA(c) = png.at(x, y) else {
                        panic!("not color.RGBA")
                    };
                    let _ = write!(w, "{:02x}{:02x}{:02x} ", c.r, c.g, c.b);
                }
            }
            Model::RGBA64 => {
                for x in bounds.min.x..bounds.max.x {
                    let Color::RGBA64(c) = png.at(x, y) else {
                        panic!("not color.RGBA64")
                    };
                    let _ = write!(w, "{:04x}{:04x}{:04x} ", c.r, c.g, c.b);
                }
            }
            Model::NRGBA => {
                for x in bounds.min.x..bounds.max.x {
                    let Color::NRGBA(c) = png.at(x, y) else {
                        panic!("not color.NRGBA")
                    };
                    match filename {
                        "ftbbn0g01" | "ftbbn0g02" | "ftbbn0g04" => {
                            let _ = write!(w, "{:02x}", c.r);
                        }
                        _ => {
                            if use_transparent {
                                let _ = write!(w, "{:02x}{:02x}{:02x} ", c.r, c.g, c.b);
                            } else {
                                let _ = write!(w, "{:02x}{:02x}{:02x}{:02x} ", c.r, c.g, c.b, c.a);
                            }
                        }
                    }
                }
            }
            Model::NRGBA64 => {
                for x in bounds.min.x..bounds.max.x {
                    let Color::NRGBA64(c) = png.at(x, y) else {
                        panic!("not color.NRGBA64")
                    };
                    match filename {
                        "ftbwn0g16" => {
                            let _ = write!(w, "{:04x} ", c.r);
                        }
                        _ => {
                            if use_transparent {
                                let _ = write!(w, "{:04x}{:04x}{:04x} ", c.r, c.g, c.b);
                            } else {
                                let _ = write!(w, "{:04x}{:04x}{:04x}{:04x} ", c.r, c.g, c.b, c.a);
                            }
                        }
                    }
                }
            }
            _ if cpm.is_some() => {
                let paletted = paletted.expect("paletted colour model on an *image.Paletted");
                let (mut b, mut c) = (0u32, 0u32);
                for x in bounds.min.x..bounds.max.x {
                    b = b << bitdepth | paletted.color_index_at(x, y) as u32;
                    c += 1;
                    if c == 8 / bitdepth {
                        let _ = write!(w, "{b:02x}");
                        b = 0;
                        c = 0;
                    }
                }
                if c != 0 {
                    while c != 8 / bitdepth {
                        b <<= bitdepth;
                        c += 1;
                    }
                    let _ = write!(w, "{b:02x}");
                }
            }
            _ => {}
        }
        w += "\n";
    }
    w += "}\n";
    w
}

// Go: reader_test.go:TestReader
#[test]
fn test_reader() {
    for fn_ in FILENAMES {
        // Read the .png file.
        let img = read_png(&format!("pngsuite/{fn_}.png")).unwrap_or_else(|e| panic!("{fn_}: {e}"));

        if fn_ == "basn4a16" {
            // basn4a16.sng is gray + alpha but sng() will produce true color + alpha
            // so we just check a single random pixel.
            let Color::NRGBA64(c) = img.at(2, 1) else {
                panic!("{fn_}: not NRGBA64")
            };
            assert!(
                c.r == 0x11a7 && c.g == 0x11a7 && c.b == 0x11a7 && c.a == 0x1085,
                "{fn_}: wrong pixel value at (2, 1): {c:?}"
            );
            continue;
        }

        let ps = sng(fn_, &*img);
        let sf =
            String::from_utf8(read_fixture(&format!("gotestdata/pngsuite/{fn_}.sng"))).unwrap();

        // Compare the two, in SNG format, line by line.
        let mut pl = ps.lines();
        let mut sl = sf.lines();
        loop {
            let (p, s) = (pl.next(), sl.next());
            let (p, s) = match (p, s) {
                (None, None) => break,
                (Some(p), Some(s)) => (p, s),
                _ => panic!("{fn_}: Different sizes"),
            };
            let mut s = s.trim_end_matches('\r');
            // Newer versions of the sng command line tool append an optional
            // color name to the RGB tuple. We strip any such name.
            if s.contains("# rgb = (")
                && !s.ends_with(')')
                && let Some(i) = s.rfind(") ")
            {
                s = &s[..i + 1];
            }
            assert_eq!(p, s, "{fn_}: Mismatch");
        }
    }
}

// Go: reader_test.go:readerErrors / TestReaderError
#[test]
fn test_reader_error() {
    let cases = [
        ("invalid-zlib.png", "zlib: invalid checksum"),
        ("invalid-crc32.png", "invalid checksum"),
        ("invalid-noend.png", "unexpected EOF"),
        ("invalid-trunc.png", "unexpected EOF"),
    ];
    for (file, want) in cases {
        match read_png(file) {
            Ok(_) => panic!("decoding {file}: missing error"),
            Err(e) => assert!(
                e.to_string().contains(want),
                "decoding {file}: {e}, want {want}"
            ),
        }
    }
}

// Go: reader_test.go:TestPalettedDecodeConfig
#[test]
fn test_paletted_decode_config() {
    for fn_ in FILENAMES_PALETTED {
        let data = read_fixture(&format!("gotestdata/pngsuite/{fn_}.png"));
        let cfg = go_png::decode_config(&mut &data[..]).unwrap_or_else(|e| panic!("{fn_}: {e}"));
        match cfg.color_model {
            Model::Palette(p) => assert!(!p.is_empty(), "{fn_}: palette not initialized"),
            _ => panic!("{fn_}: expected paletted color model"),
        }
    }
}

// Go: reader_test.go:TestInterlaced
#[test]
fn test_interlaced() {
    let a = read_png("gray-gradient.png").unwrap();
    let b = read_png("gray-gradient.interlaced.png").unwrap();
    let (a, b) = (
        a.downcast_ref::<Gray>().unwrap(),
        b.downcast_ref::<Gray>().unwrap(),
    );
    assert_eq!(a, b, "decodings differ");
}

// Go: reader_test.go:TestIncompleteIDATOnRowBoundary
#[test]
fn test_incomplete_idat_on_row_boundary() {
    // The following is an invalid 1x2 grayscale PNG image. The header is OK,
    // but the zlib-compressed IDAT payload contains two bytes "\x02\x00",
    // which is only one row of data (the leading "\x02" is a row filter).
    let ihdr =
        b"\x00\x00\x00\x0dIHDR\x00\x00\x00\x01\x00\x00\x00\x02\x08\x00\x00\x00\x00\xbc\xea\xe9\xfb";
    let idat = b"\x00\x00\x00\x0eIDAT\x78\x9c\x62\x62\x00\x04\x00\x00\xff\xff\x00\x06\x00\x03\xfa\xd0\x59\xae";
    let iend = b"\x00\x00\x00\x00IEND\xae\x42\x60\x82";
    let data = [PNG_HEADER, &ihdr[..], &idat[..], &iend[..]].concat();
    let err = go_png::decode(&mut &data[..])
        .err()
        .expect("got nil error, want non-nil");
    assert_eq!(
        err.to_string(),
        "png: invalid format: not enough pixel data"
    );
}

// Go: reader_test.go:TestTrailingIDATChunks
#[test]
fn test_trailing_idat_chunks() {
    // The following is a valid 1x1 PNG image containing color.Gray{255} and
    // a trailing zero-length IDAT chunk (see PNG specification section 12.9):
    let ihdr =
        b"\x00\x00\x00\x0dIHDR\x00\x00\x00\x01\x00\x00\x00\x01\x08\x00\x00\x00\x00\x3a\x7e\x9b\x55";
    let idat_white = b"\x00\x00\x00\x0eIDAT\x78\x9c\x62\xfa\x0f\x08\x00\x00\xff\xff\x01\x05\x01\x02\x5a\xdd\x39\xcd";
    let idat_zero = b"\x00\x00\x00\x00IDAT\x35\xaf\x06\x1e";
    let iend = b"\x00\x00\x00\x00IEND\xae\x42\x60\x82";
    let data = [
        PNG_HEADER,
        &ihdr[..],
        &idat_white[..],
        &idat_zero[..],
        &iend[..],
    ]
    .concat();
    go_png::decode(&mut &data[..]).expect("decoding valid image");

    // Non-zero-length trailing IDAT chunks should be ignored (recoverable error).
    // The following chunk contains a single pixel with color.Gray{0}.
    let idat_black = b"\x00\x00\x00\x0eIDAT\x78\x9c\x62\x62\x00\x04\x00\x00\xff\xff\x00\x06\x00\x03\xfa\xd0\x59\xae";
    let data = [
        PNG_HEADER,
        &ihdr[..],
        &idat_white[..],
        &idat_black[..],
        &iend[..],
    ]
    .concat();
    let img = go_png::decode(&mut &data[..]).expect("trailing IDAT not ignored");
    assert!(
        !matches!(img.at(0, 0), Color::Gray(color::Gray { y: 0 })),
        "decoded image from trailing IDAT chunk"
    );
}

// Go: reader_test.go:TestMultipletRNSChunks
#[test]
fn test_multiple_trns_chunks() {
    let ihdr =
        b"\x00\x00\x00\x0dIHDR\x00\x00\x00\x01\x00\x00\x00\x01\x08\x03\x00\x00\x00\x28\xcb\x34\xbb";
    let plte = b"\x00\x00\x00\x03PLTE\xff\x00\x00\x19\xe2\x09\x37";
    let trns = b"\x00\x00\x00\x01tRNS\x7f\x80\x5c\xb4\xcb";
    let idat = b"\x00\x00\x00\x0eIDAT\x78\x9c\x62\x62\x00\x04\x00\x00\xff\xff\x00\x06\x00\x03\xfa\xd0\x59\xae";
    let iend = b"\x00\x00\x00\x00IEND\xae\x42\x60\x82";
    for i in 0..4 {
        let mut b = Vec::new();
        b.extend_from_slice(PNG_HEADER);
        b.extend_from_slice(ihdr);
        b.extend_from_slice(plte);
        for _ in 0..i {
            b.extend_from_slice(trns);
        }
        b.extend_from_slice(idat);
        b.extend_from_slice(iend);

        let res = go_png::decode(&mut &b[..]);
        let want = match i {
            0 => Color::RGBA(color::RGBA {
                r: 0xff,
                g: 0x00,
                b: 0x00,
                a: 0xff,
            }),
            1 => Color::NRGBA(color::NRGBA {
                r: 0xff,
                g: 0x00,
                b: 0x00,
                a: 0x7f,
            }),
            _ => {
                let err = res
                    .err()
                    .unwrap_or_else(|| panic!("{i} tRNS chunks: got nil error"));
                assert_eq!(err.to_string(), "png: invalid format: chunk out of order");
                continue;
            }
        };
        let m = res.unwrap_or_else(|e| panic!("{i} tRNS chunks: {e}"));
        assert_eq!(m.at(0, 0), want, "{i} tRNS chunks");
    }
}

// Go: reader_test.go:TestUnknownChunkLengthUnderflow
#[test]
fn test_unknown_chunk_length_underflow() {
    let data: [u8; 47] = [
        0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
        0xff, 0x06, 0xf4, 0x7c, 0x55, 0x04, 0x1a, 0xd3, 0x11, 0x9a, 0x73, 0x00, 0x00, 0xf8, 0x1e,
        0xf3, 0x2e, 0x00, 0x00, 0x01, 0x00, 0xff, 0xff, 0xff, 0xff, 0x07, 0xf4, 0x7c, 0x55, 0x04,
        0x1a, 0xd3,
    ];
    let err = go_png::decode(&mut &data[..])
        .err()
        .expect("Didn't fail reading an unknown chunk with length 0xffffffff");
    assert_eq!(
        err.to_string(),
        "png: invalid format: Bad chunk length: 4294967295"
    );
}

// Go: reader_test.go:TestPaletted8OutOfRangePixel
#[test]
fn test_paletted8_out_of_range_pixel() {
    // IDAT contains a reference to a palette index that does not exist in the file.
    let img = read_png("invalid-palette.png").expect("decoding invalid-palette.png");
    // Expect that the palette is extended with opaque black.
    let want = Color::RGBA(color::RGBA {
        r: 0,
        g: 0,
        b: 0,
        a: 0xff,
    });
    assert_eq!(img.at(15, 15), want);
}

// Go: reader_test.go:TestGray8Transparent
#[test]
fn test_gray8_transparent() {
    // These bytes come from https://golang.org/issues/19553
    let data: &[u8] = &[
        0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, 0x00, 0x00, 0x00, 0x0d, 0x49, 0x48, 0x44,
        0x52, 0x00, 0x00, 0x00, 0x0f, 0x00, 0x00, 0x00, 0x0b, 0x08, 0x00, 0x00, 0x00, 0x00, 0x85,
        0x2c, 0x88, 0x80, 0x00, 0x00, 0x00, 0x02, 0x74, 0x52, 0x4e, 0x53, 0x00, 0xff, 0x5b, 0x91,
        0x22, 0xb5, 0x00, 0x00, 0x00, 0x02, 0x62, 0x4b, 0x47, 0x44, 0x00, 0xff, 0x87, 0x8f, 0xcc,
        0xbf, 0x00, 0x00, 0x00, 0x09, 0x70, 0x48, 0x59, 0x73, 0x00, 0x00, 0x0a, 0xf0, 0x00, 0x00,
        0x0a, 0xf0, 0x01, 0x42, 0xac, 0x34, 0x98, 0x00, 0x00, 0x00, 0x07, 0x74, 0x49, 0x4d, 0x45,
        0x07, 0xd5, 0x04, 0x02, 0x12, 0x11, 0x11, 0xf7, 0x65, 0x3d, 0x8b, 0x00, 0x00, 0x00, 0x4f,
        0x49, 0x44, 0x41, 0x54, 0x08, 0xd7, 0x63, 0xf8, 0xff, 0xff, 0xff, 0xb9, 0xbd, 0x70, 0xf0,
        0x8c, 0x01, 0xc8, 0xaf, 0x6e, 0x99, 0x02, 0x05, 0xd9, 0x7b, 0xc1, 0xfc, 0x6b, 0xff, 0xa1,
        0xa0, 0x87, 0x30, 0xff, 0xd9, 0xde, 0xbd, 0xd5, 0x4b, 0xf7, 0xee, 0xfd, 0x0e, 0xe3, 0xef,
        0xcd, 0x06, 0x19, 0x14, 0xf5, 0x1e, 0xce, 0xef, 0x01, 0x31, 0x92, 0xd7, 0x82, 0x41, 0x31,
        0x9c, 0x3f, 0x07, 0x02, 0xee, 0xa1, 0xaa, 0xff, 0xff, 0x9f, 0xe1, 0xd9, 0x56, 0x30, 0xf8,
        0x0e, 0xe5, 0x03, 0x00, 0xa9, 0x42, 0x84, 0x3d, 0xdf, 0x8f, 0xa6, 0x8f, 0x00, 0x00, 0x00,
        0x00, 0x49, 0x45, 0x4e, 0x44, 0xae, 0x42, 0x60, 0x82,
    ];
    let m = go_png::decode(&mut &data[..]).expect("Decode");

    const HEX: &[u8] = b"0123456789abcdef";
    let mut got = Vec::new();
    let bounds = m.bounds();
    for y in bounds.min.y..bounds.max.y {
        for x in bounds.min.x..bounds.max.x {
            let (r, _, _, a) = m.at(x, y).rgba();
            if a != 0 {
                got.extend_from_slice(&[
                    HEX[(0x0f & (r >> 12)) as usize],
                    HEX[(0x0f & (r >> 8)) as usize],
                    b' ',
                ]);
            } else {
                got.extend_from_slice(b".. ");
            }
        }
        got.push(b'\n');
    }

    let want = concat!(
        ".. .. .. ce bd bd bd bd bd bd bd bd bd bd e6 \n",
        ".. .. .. 7b 84 94 94 94 94 94 94 94 94 6b bd \n",
        ".. .. .. 7b d6 .. .. .. .. .. .. .. .. 8c bd \n",
        ".. .. .. 7b d6 .. .. .. .. .. .. .. .. 8c bd \n",
        ".. .. .. 7b d6 .. .. .. .. .. .. .. .. 8c bd \n",
        "e6 bd bd 7b a5 bd bd f7 .. .. .. .. .. 8c bd \n",
        "bd 6b 94 94 94 94 5a ef .. .. .. .. .. 8c bd \n",
        "bd 8c .. .. .. .. 63 ad ad ad ad ad ad 73 bd \n",
        "bd 8c .. .. .. .. 63 9c 9c 9c 9c 9c 9c 9c de \n",
        "bd 6b 94 94 94 94 5a ef .. .. .. .. .. .. .. \n",
        "e6 b5 b5 b5 b5 b5 b5 f7 .. .. .. .. .. .. .. \n",
    );
    assert_eq!(String::from_utf8(got).unwrap(), want);
}

// Go: reader_test.go:TestDimensionOverflow (64-bit ints).
#[test]
fn test_dimension_overflow() {
    struct Case {
        src: &'static [u8],
        unsupported_config: bool,
        width: i64,
        height: i64,
    }
    let test_cases = [
        // These bytes come from https://golang.org/issues/22304
        //
        // It encodes a 2147483646 × 2147483646 (i.e. 0x7ffffffe × 0x7ffffffe)
        // NRGBA image. The (width × height) per se doesn't overflow an int64, but
        // (width × height × bytesPerPixel) will.
        Case {
            src: &[
                0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, 0x00, 0x00, 0x00, 0x0d, 0x49, 0x48,
                0x44, 0x52, 0x7f, 0xff, 0xff, 0xfe, 0x7f, 0xff, 0xff, 0xfe, 0x08, 0x06, 0x00, 0x00,
                0x00, 0x30, 0x57, 0xb3, 0xfd, 0x00, 0x00, 0x00, 0x15, 0x49, 0x44, 0x41, 0x54, 0x78,
                0x9c, 0x62, 0x62, 0x20, 0x12, 0x8c, 0x2a, 0xa4, 0xb3, 0x42, 0x40, 0x00, 0x00, 0x00,
                0xff, 0xff, 0x13, 0x38, 0x00, 0x15, 0x2d, 0xef, 0x5f, 0x0f, 0x00, 0x00, 0x00, 0x00,
                0x49, 0x45, 0x4e, 0x44, 0xae, 0x42, 0x60, 0x82,
            ],
            unsupported_config: true,
            width: 0x7ffffffe,
            height: 0x7ffffffe,
        },
        // The next three cases come from https://golang.org/issues/38435
        Case {
            src: &[
                0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, 0x00, 0x00, 0x00, 0x0d, 0x49, 0x48,
                0x44, 0x52, 0x00, 0x00, 0xb5, 0x04, 0x00, 0x00, 0xb5, 0x04, 0x08, 0x06, 0x00, 0x00,
                0x00, 0xf5, 0x60, 0x2c, 0xb8, 0x00, 0x00, 0x00, 0x15, 0x49, 0x44, 0x41, 0x54, 0x78,
                0x9c, 0x62, 0x62, 0x20, 0x12, 0x8c, 0x2a, 0xa4, 0xb3, 0x42, 0x40, 0x00, 0x00, 0x00,
                0xff, 0xff, 0x13, 0x38, 0x00, 0x15, 0x2d, 0xef, 0x5f, 0x0f, 0x00, 0x00, 0x00, 0x00,
                0x49, 0x45, 0x4e, 0x44, 0xae, 0x42, 0x60, 0x82,
            ],
            // Here, width * height = 0x7ffea810, just under MaxInt32, but at 4
            // bytes per pixel, the number of pixels overflows an int32.
            unsupported_config: false, // have32BitInts
            width: 0x0000b504,
            height: 0x0000b504,
        },
        Case {
            src: &[
                0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, 0x00, 0x00, 0x00, 0x0d, 0x49, 0x48,
                0x44, 0x52, 0x04, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x08, 0x06, 0x00, 0x00,
                0x00, 0x30, 0x6e, 0xc5, 0x21, 0x00, 0x00, 0x00, 0x15, 0x49, 0x44, 0x41, 0x54, 0x78,
                0x9c, 0x62, 0x62, 0x20, 0x12, 0x8c, 0x2a, 0xa4, 0xb3, 0x42, 0x40, 0x00, 0x00, 0x00,
                0xff, 0xff, 0x13, 0x38, 0x00, 0x15, 0x2d, 0xef, 0x5f, 0x0f, 0x00, 0x00, 0x00, 0x00,
                0x49, 0x45, 0x4e, 0x44, 0xae, 0x42, 0x60, 0x82,
            ],
            unsupported_config: false,
            width: 0x04000000,
            height: 0x00000001,
        },
        Case {
            src: &[
                0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, 0x00, 0x00, 0x00, 0x0d, 0x49, 0x48,
                0x44, 0x52, 0x08, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x08, 0x06, 0x00, 0x00,
                0x00, 0xaa, 0xd4, 0x7c, 0xda, 0x00, 0x00, 0x00, 0x15, 0x49, 0x44, 0x41, 0x54, 0x78,
                0x9c, 0x62, 0x66, 0x20, 0x12, 0x30, 0x8d, 0x2a, 0xa4, 0xaf, 0x42, 0x40, 0x00, 0x00,
                0x00, 0xff, 0xff, 0x14, 0xd2, 0x00, 0x16, 0x00, 0x00, 0x00,
            ],
            unsupported_config: false,
            width: 0x08000000,
            height: 0x00000001,
        },
    ];

    for (i, tc) in test_cases.iter().enumerate() {
        let res = go_png::decode_config(&mut &tc.src[..]);
        if tc.unsupported_config {
            match res {
                Err(Error::Unsupported(_)) => {}
                Err(e) => panic!("i={i}: got {e:?}, want png.UnsupportedError"),
                Ok(_) => panic!("i={i}: DecodeConfig: got nil error, want non-nil"),
            }
            continue;
        }
        let cfg = res.unwrap_or_else(|e| panic!("i={i}: DecodeConfig: {e}"));
        assert_eq!(cfg.width, tc.width, "i={i}: width");
        assert_eq!(cfg.height, tc.height, "i={i}: height");

        if cfg.width * cfg.height > 0x7f000000 {
            // In theory, calling Decode would succeed, given several gigabytes
            // of memory. In practice, trying to make a []uint8 big enough to
            // hold all of the pixels can often result in OOM (out of memory).
            continue;
        }

        // Even if we don't panic, these aren't valid PNG images. (The pixel
        // and row buffers are zero-allocated, so this is cheap.)
        assert!(
            go_png::decode(&mut &tc.src[..]).is_err(),
            "i={i}: Decode: got nil error, want non-nil"
        );
    }
}

// Go: reader_test.go:TestDecodePalettedWithTransparency
#[test]
fn test_decode_paletted_with_transparency() {
    // These bytes come from https://go.dev/issue/54325
    let src: &[u8] = &[
        0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, 0x00, 0x00, 0x00, 0x0d, 0x49, 0x48, 0x44,
        0x52, 0x00, 0x00, 0x00, 0x20, 0x00, 0x00, 0x00, 0x20, 0x04, 0x03, 0x00, 0x00, 0x00, 0x81,
        0x54, 0x67, 0xc7, 0x00, 0x00, 0x00, 0x30, 0x50, 0x4c, 0x54, 0x45, 0x00, 0x00, 0x00, 0x00,
        0xff, 0xff, 0x0e, 0x00, 0x23, 0x27, 0x7b, 0xb1, 0x2d, 0x0a, 0x49, 0x3f, 0x19, 0x78, 0x5f,
        0xcd, 0xe4, 0x69, 0x69, 0xe4, 0x71, 0x59, 0x53, 0x80, 0x11, 0x14, 0x8b, 0x00, 0xa9, 0x8d,
        0x95, 0xcb, 0x99, 0x2f, 0x6b, 0xd7, 0x29, 0x91, 0xd7, 0x7b, 0xba, 0xff, 0xe3, 0xd7, 0x13,
        0xc6, 0xd3, 0x58, 0x00, 0x00, 0x00, 0x01, 0x74, 0x52, 0x4e, 0x53, 0x00, 0x40, 0xe6, 0xd8,
        0x66, 0x00, 0x00, 0x00, 0xfd, 0x49, 0x44, 0x41, 0x54, 0x28, 0xcf, 0x63, 0x60, 0x00, 0x83,
        0x55, 0x0c, 0x68, 0x60, 0x9d, 0x02, 0x9a, 0x80, 0xde, 0x23, 0x74, 0x15, 0xef, 0x50, 0x94,
        0x70, 0x2d, 0xd2, 0x7b, 0x87, 0xa2, 0x84, 0xeb, 0xee, 0xbb, 0x77, 0x6f, 0x51, 0x94, 0xe8,
        0xbd, 0x7d, 0xf7, 0xee, 0x12, 0xb2, 0x80, 0xd2, 0x3d, 0x54, 0x01, 0x26, 0x10, 0x1f, 0x59,
        0x40, 0x0f, 0xc8, 0xd7, 0x7e, 0x84, 0x70, 0x1c, 0xd7, 0xba, 0xb7, 0x4a, 0xda, 0xda, 0x77,
        0x11, 0xf6, 0xac, 0x5a, 0xa5, 0xf4, 0xf9, 0xbf, 0xfd, 0x3d, 0x24, 0x6b, 0x98, 0x94, 0xf4,
        0xff, 0x7f, 0x52, 0x42, 0x16, 0x30, 0x0e, 0xd9, 0xed, 0x6a, 0x8c, 0xec, 0x10, 0x65, 0x53,
        0x97, 0x60, 0x23, 0x64, 0x1d, 0x8a, 0x2e, 0xc6, 0x2e, 0x42, 0x08, 0x3d, 0x4c, 0xca, 0x81,
        0xc1, 0x82, 0xa6, 0xa2, 0x46, 0x08, 0x3d, 0x4a, 0xa1, 0x82, 0xc6, 0x82, 0xa1, 0x4a, 0x08,
        0x3d, 0xfa, 0xa6, 0x81, 0xa1, 0xa2, 0xc1, 0x9f, 0x10, 0x66, 0xd4, 0x2b, 0x87, 0x0a, 0x86,
        0x1a, 0x7d, 0x57, 0x80, 0x9b, 0x99, 0xaf, 0x62, 0x1a, 0x1a, 0xec, 0xf0, 0x0d, 0x66, 0x2a,
        0x7b, 0x5a, 0xba, 0xd2, 0x64, 0x63, 0x4b, 0xa6, 0xb2, 0xb4, 0x02, 0xa8, 0x12, 0xb5, 0x24,
        0xa5, 0x99, 0x2e, 0x33, 0x95, 0xd4, 0x92, 0x10, 0xee, 0xd0, 0x59, 0xb9, 0x6a, 0xd6, 0x21,
        0x24, 0xb7, 0x33, 0x9d, 0x01, 0x01, 0x64, 0xbf, 0xac, 0x59, 0xb2, 0xca, 0xeb, 0x14, 0x92,
        0x80, 0xd6, 0x9a, 0x53, 0x4a, 0x6b, 0x4e, 0x2d, 0x42, 0x52, 0xa1, 0x73, 0x28, 0x54, 0xe7,
        0x90, 0x6a, 0x00, 0x92, 0x92, 0x45, 0xa1, 0x40, 0x84, 0x2c, 0xe0, 0xc4, 0xa0, 0xb2, 0x28,
        0x14, 0xc1, 0x67, 0xe9, 0x50, 0x60, 0x60, 0xea, 0x70, 0x40, 0x12, 0x00, 0x79, 0x54, 0x09,
        0x22, 0x00, 0x00, 0x30, 0xf3, 0x52, 0x87, 0xc6, 0xe4, 0xbd, 0x70, 0x00, 0x00, 0x00, 0x00,
        0x49, 0x45, 0x4e, 0x44, 0xae, 0x42, 0x60, 0x82,
    ];

    let cfg = go_png::decode_config(&mut &src[..]).expect("DecodeConfig");
    let Model::Palette(p) = &cfg.color_model else {
        panic!("DecodeConfig: not a palette")
    };
    assert_eq!(p[0].rgba().3, 0, "DecodeConfig");

    let img = go_png::decode(&mut &src[..]).expect("Decode");
    let Model::Palette(p) = img.color_model() else {
        panic!("Decode: not a palette")
    };
    assert_eq!(p[0].rgba().3, 0, "Decode");
}

// Go: fuzz_test.go:FuzzDecode, run over its seed corpus (Go's image/testdata
// PNGs) and the image/png testdata.
#[test]
fn fuzz_decode_seeds() {
    go_png::register();
    let mut names: Vec<String> = [
        "video-001.221212.png",
        "video-001.cmyk.png",
        "video-001.png",
        "video-001.progressive.truncated.png",
        "video-001.rgb.png",
        "video-005.gray.png",
    ]
    .iter()
    .map(|n| format!("gotestdata/image/{n}"))
    .collect();
    for n in FILENAMES {
        names.push(format!("gotestdata/pngsuite/{n}.png"));
    }
    let mut decoded = 0;
    for name in &names {
        let b = read_fixture(name);
        let Ok((cfg, _)) = go_image::decode_config(&mut &b[..]) else {
            continue;
        };
        if cfg.width * cfg.height > 1_000_000 {
            continue;
        }
        let Ok((img, typ)) = go_image::decode(&mut &b[..]) else {
            continue;
        };
        if typ != "png" {
            continue;
        }
        decoded += 1;
        for l in [
            DEFAULT_COMPRESSION,
            NO_COMPRESSION,
            BEST_SPEED,
            BEST_COMPRESSION,
        ] {
            let mut w = Vec::new();
            let e = Encoder {
                compression_level: l,
                ..Default::default()
            };
            e.encode(&mut w, &*img)
                .unwrap_or_else(|err| panic!("{name}: failed to encode valid image: {err}"));
            let img1 = go_png::decode(&mut &w[..])
                .unwrap_or_else(|err| panic!("{name}: failed to decode roundtripped image: {err}"));
            assert!(
                img1.bounds().eq(img.bounds()),
                "{name}: roundtripped image bounds have changed"
            );
        }
    }
    assert_eq!(decoded, names.len());
}

/// Decoding a paletted interlaced image whose pixels use indices beyond the
/// PLTE length extends the palette (Go's mergePassInto adjustment).
#[test]
fn paletted_interlaced_out_of_range_indices() {
    let mut data = PNG_HEADER.to_vec();
    // 3x3 P2 image, Adam7.
    write_chunk(&mut data, b"IHDR", &[0, 0, 0, 3, 0, 0, 0, 3, 2, 3, 0, 0, 1]);
    write_chunk(&mut data, b"PLTE", &[10, 20, 30]);
    // Pass images: pass 0 (1x1), pass 3 (1x1: x=2,y=0), pass 4 (2x1: y=2),
    // pass 5 (1x2: x=1), pass 6 (3x1: y=1). Passes 1 and 2 are empty.
    let raw: &[u8] = &[
        0,
        0b1100_0000, // pass 0: index 3
        0,
        0b0100_0000, // pass 3: index 1
        0,
        0b0010_0000, // pass 4: indices 0, 2
        0,
        0b0000_0000,
        0,
        0b1100_0000, // pass 5: indices 0, 3
        0,
        0b0001_1000, // pass 6: indices 0, 1, 2
    ];
    let mut z = go_flate::zlib::new_writer(Vec::new());
    z.write(raw).unwrap();
    z.close().unwrap();
    write_chunk(&mut data, b"IDAT", &z.into_inner());
    write_chunk(&mut data, b"IEND", &[]);
    let img = go_png::decode(&mut &data[..]).unwrap();
    let p = img.downcast_ref::<Paletted>().unwrap();
    assert_eq!(p.palette.len(), 4);
    assert_eq!(p.pix, vec![3, 0, 1, 0, 1, 2, 0, 3, 2]);
    assert_eq!(
        p.palette[3],
        Color::RGBA(color::RGBA {
            r: 0,
            g: 0,
            b: 0,
            a: 0xff
        })
    );
    // DecodeConfig reports the PLTE length only.
    let cfg = go_png::decode_config(&mut &data[..]).unwrap();
    let Model::Palette(cp) = cfg.color_model else {
        panic!()
    };
    assert_eq!(cp.len(), 1);
}
