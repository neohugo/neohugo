//! Reader for the fixture packs written by tools/go-oracle/libwebp-sys.

use std::fs;
use std::path::Path;

use libwebp_sys::{EncodingOptions, EncodingPreset, Image, PixView, Point, Rectangle};

pub struct Case {
    pub name: String,
    pub kind: String,
    pub stride: i64,
    pub rect: Rectangle,
    pub opts: EncodingOptions,
    pub input: Vec<u8>,
    pub expected: Vec<u8>,
    pub expect: String,
}

impl Case {
    pub fn image(&self) -> Image<'_> {
        let v = PixView {
            pix: &self.input,
            stride: self.stride,
            rect: self.rect,
        };
        match self.kind.as_str() {
            "nrgba" => Image::Nrgba(v),
            "rgba" => Image::Rgba(v),
            "gray" => Image::Gray(v),
            k => panic!("unknown kind {k}"),
        }
    }
}

pub struct Pack {
    inputs: Vec<u8>,
    outputs: Vec<u8>,
    lines: Vec<String>,
}

impl Pack {
    pub fn open(dir: &Path) -> Pack {
        let manifest = fs::read_to_string(dir.join("manifest.tsv")).expect("manifest.tsv");
        Pack {
            inputs: fs::read(dir.join("inputs.bin")).expect("inputs.bin"),
            outputs: fs::read(dir.join("outputs.bin")).expect("outputs.bin"),
            lines: manifest
                .lines()
                .filter(|l| !l.starts_with('#') && !l.is_empty())
                .map(String::from)
                .collect(),
        }
    }

    pub fn len(&self) -> usize {
        self.lines.len()
    }

    pub fn case(&self, i: usize) -> Case {
        let f: Vec<&str> = self.lines[i].split('\t').collect();
        assert_eq!(f.len(), 16, "bad manifest line {}", self.lines[i]);
        let n = |i: usize| -> i64 { f[i].parse().unwrap() };
        let (in_off, in_zlen, in_len) = (n(10) as usize, n(11) as usize, n(12) as usize);
        let (out_off, out_len) = (n(13) as usize, n(14) as usize);
        let input =
            miniz_oxide::inflate::decompress_to_vec_zlib(&self.inputs[in_off..in_off + in_zlen])
                .expect("zlib input");
        assert_eq!(input.len(), in_len);
        Case {
            name: f[0].to_string(),
            kind: f[1].to_string(),
            stride: n(2),
            rect: Rectangle {
                min: Point { x: n(3), y: n(4) },
                max: Point { x: n(5), y: n(6) },
            },
            opts: EncodingOptions {
                quality: n(7),
                encoding_preset: EncodingPreset(n(8)),
                use_sharp_yuv: n(9) != 0,
            },
            input,
            expected: self.outputs[out_off..out_off + out_len].to_vec(),
            expect: f[15].to_string(),
        }
    }
}

/// Runs one case; returns a description of the mismatch, if any.
pub fn check(c: &Case) -> Option<String> {
    let got = libwebp_sys::encode_to_vec(&c.image(), c.opts);
    match (c.expect.as_str(), got) {
        ("ok", Ok(b)) => {
            if b == c.expected {
                None
            } else {
                Some(format!(
                    "{}: bytes differ (got {} want {})",
                    c.name,
                    b.len(),
                    c.expected.len()
                ))
            }
        }
        ("panic", Err(libwebp_sys::Error::EmptyPix)) => None,
        (e, Err(err)) if e.strip_prefix("err:") == Some(err.to_string().as_str()) => None,
        (e, Ok(b)) => Some(format!("{}: want {e}, got {} bytes", c.name, b.len())),
        (e, Err(err)) => Some(format!("{}: want {e}, got error {err}", c.name)),
    }
}
