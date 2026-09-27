//! Ports of go1.27.1 `image/jpeg/reader_test.go` and `writer_test.go`.
//! Deviations: tests that need `math/rand` streams use the splitmix64
//! generator (only self-consistency is asserted there); TestWriter needs a
//! PNG decoder and is covered by `jpeg_synth.rs` instead.

mod common;

use common::*;
use go_image::color::{self, Color};
use go_image::jpeg::{self, Error};
use go_image::{Gray, Image, RGBA, YCbCr, YCbCrSubsampleRatio, rect};
use std::io::Read;

fn testdata(name: &str) -> Vec<u8> {
    std::fs::read(fixtures_dir().join("gotestdata").join(name)).unwrap()
}

fn decode_file(name: &str) -> Result<Box<dyn Image>, Error> {
    jpeg::decode(&mut &testdata(name)[..])
}

// Go: check (reader_test.go)
fn check(
    bounds: go_image::Rectangle,
    pix0: &[u8],
    pix1: &[u8],
    stride0: i64,
    stride1: i64,
) -> Result<(), String> {
    if stride0 <= 0 || stride0 % 8 != 0 {
        return Err(format!("bad stride {}", stride0));
    }
    if stride1 <= 0 || stride1 % 8 != 0 {
        return Err(format!("bad stride {}", stride1));
    }
    let mut y = 0i64;
    while y < pix0.len() as i64 / stride0 && y < pix1.len() as i64 / stride1 {
        let mut x = 0i64;
        while x < stride0 && x < stride1 {
            if x >= bounds.max.x || y >= bounds.max.y {
                x += 8;
                continue;
            }
            for j in 0..8 {
                for i in 0..8 {
                    let index0 = ((y + j) * stride0 + (x + i)) as usize;
                    let index1 = ((y + j) * stride1 + (x + i)) as usize;
                    if pix0[index0] != pix1[index1] {
                        return Err(format!("blocks at ({}, {}) differ", x, y));
                    }
                }
            }
            x += 8;
        }
        y += 8;
    }
    Ok(())
}

// Go: TestDecodeProgressive
#[test]
fn decode_progressive() {
    let cases = [
        "video-001",
        "video-001.q50.410",
        "video-001.q50.411",
        "video-001.q50.420",
        "video-001.q50.422",
        "video-001.q50.440",
        "video-001.q50.444",
        "video-005.gray.q50",
        "video-005.gray.q50.2x2",
        "video-001.separate.dc.progression",
    ];
    for tc in cases {
        let m0 = decode_file(&format!("{}.jpeg", tc)).unwrap();
        let m1 = decode_file(&format!("{}.progressive.jpeg", tc)).unwrap();
        assert_eq!(m0.bounds(), m1.bounds(), "{}", tc);
        assert_eq!(m0.bounds(), rect(0, 0, 150, 103), "{}", tc);
        if let Some(m0) = m0.downcast_ref::<YCbCr>() {
            let m1 = m1.downcast_ref::<YCbCr>().unwrap();
            check(m0.bounds(), &m0.y, &m1.y, m0.y_stride, m1.y_stride).unwrap();
            check(m0.bounds(), &m0.cb, &m1.cb, m0.c_stride, m1.c_stride).unwrap();
            check(m0.bounds(), &m0.cr, &m1.cr, m0.c_stride, m1.c_stride).unwrap();
        } else if let Some(m0) = m0.downcast_ref::<Gray>() {
            let m1 = m1.downcast_ref::<Gray>().unwrap();
            check(m0.bounds(), &m0.pix, &m1.pix, m0.stride, m1.stride).unwrap();
        } else {
            panic!("{}: unexpected image type", tc);
        }
    }
}

/// A reader delivering data in fixed-size chunks (Go's TestDecodeEOF uses a
/// reader returning the final data together with io.EOF; the Rust analogue
/// is arbitrary short reads, which must not change the result).
struct ChunkReader<'a> {
    data: &'a [u8],
    chunk: usize,
}

impl Read for ChunkReader<'_> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let n = self.chunk.min(buf.len()).min(self.data.len());
        buf[..n].copy_from_slice(&self.data[..n]);
        self.data = &self.data[n..];
        Ok(n)
    }
}

// Go: TestDecodeEOF (adapted)
#[test]
fn decode_eof_and_short_reads() {
    for name in [
        "video-001.jpeg",
        "video-001.progressive.jpeg",
        "video-001.restart2.jpeg",
    ] {
        let data = testdata(name);
        let want = img_digest(jpeg::decode(&mut &data[..]).unwrap().as_ref());
        for chunk in [1usize, 2, 3, 7, 64, 4095, 4096, 4097, 100000] {
            let mut r = ChunkReader { data: &data, chunk };
            let got = img_digest(jpeg::decode(&mut r).unwrap().as_ref());
            assert_eq!(got, want, "{} chunk {}", name, chunk);
        }
    }
}

// Go: TestTruncatedSOSDataDoesntPanic
#[test]
fn truncated_sos_data_doesnt_panic() {
    let b = testdata("video-005.gray.q50.jpeg");
    let mut i = b.windows(2).position(|w| w == [0xff, 0xda]).unwrap();
    i += 2;
    let j = (i + 10).min(b.len());
    while i < j {
        let _ = jpeg::decode(&mut &b[..i]);
        i += 1;
    }
}

// Go: TestLargeImageWithShortData
#[test]
fn large_image_with_short_data() {
    let input: &[u8] = b"\xff\xd8\xff\xe0\x00\x10\x4a\x46\x49\x46\x00\x01\x01\x00\x00\x01\
\x00\x01\x00\x00\xff\xdb\x00\x43\x00\x10\x0b\x0c\x0e\x0c\x0a\x10\
\x0e\x89\x0e\x12\x11\x10\x13\x18\xff\xd8\xff\xe0\x00\x10\x4a\x46\
\x49\x46\x00\x01\x01\x00\x00\x01\x00\x01\x00\x00\xff\xdb\x00\x43\
\x00\x10\x0b\x0c\x0e\x0c\x0a\x10\x0e\x0d\x0e\x12\x11\x10\x13\x18\
\x28\x1a\x18\x16\x16\x18\x31\x23\x25\x1d\x28\x3a\x33\x3d\x3c\x39\
\x33\x38\x37\x40\x48\x5c\x4e\x40\x44\x57\x45\x37\x38\x50\x6d\x51\
\x57\x5f\x62\x67\x68\x67\x3e\x4d\x71\x79\x70\x64\x78\x5c\x65\x67\
\x63\xff\xc0\x00\x0b\x08\x20\x00\x20\x00\x01\x01\x11\x00\xff\xc4\
\x00\x1f\x00\x00\x01\x05\x01\x01\x01\x01\x01\x01\x00\x00\x00\x00\
\x00\x00\x00\x00\x01\x02\x03\x04\x05\x06\x07\x08\x09\x0a\x0b\xff\
\xc4\x00\xb5\x10\x00\x02\x01\x03\x03\x02\x04\x03\x05\x05\x04\x04\
\x00\x00\x01\x7d\x01\x02\x03\x00\x04\x11\x05\x12\x21\x31\x01\x06\
\x13\x51\x61\x07\x22\x71\x14\x32\x81\x91\xa1\x08\x23\xd8\xff\xdd\
\x42\xb1\xc1\x15\x52\xd1\xf0\x24\x33\x62\x72\x82\x09\x0a\x16\x17\
\x18\x19\x1a\x25\x26\x27\x28\x29\x2a\x34\x35\x36\x37\x38\x39\x3a\
\x43\x44\x45\x46\x47\x48\x49\x4a\x53\x54\x55\x56\x57\x58\x59\x5a\
\x00\x63\x64\x65\x66\x67\x68\x69\x6a\x73\x74\x75\x76\x77\x78\x79\
\x7a\x83\x84\x85\x86\x87\x88\x89\x8a\x92\x93\x94\x95\x96\x97\x98\
\x99\x9a\xa2\xa3\xa4\xa5\xa6\xa7\xa8\xa9\xaa\xb2\xb3\xb4\xb5\xb6\
\xb7\xb8\xb9\xba\xc2\xc3\xc4\xc5\xc6\xc7\xff\xd8\xff\xe0\x00\x10\
\x4a\x46\x49\x46\x00\x01\x01\x00\x00\x01\x00\x01\x00\x00\xff\xdb\
\x00\x43\x00\x10\x0b\x0c\x0e\x0c\x0a\x10\x0e\x0d\x0e\x12\x11\x10\
\x13\x18\x28\x1a\x18\x16\x16\x18\x31\x23\x25\x1d\xc8\xc9\xca\xd2\
\xd3\xd4\xd5\xd6\xd7\xd8\xd9\xda\xe1\xe2\xe3\xe4\xe5\xe6\xe7\xe8\
\xe9\xea\xf1\xf2\xf3\xf4\xf5\xf6\xf7\xf8\xf9\xfa\xff\xda\x00\x08\
\x01\x01\x00\x00\x3f\x00\xb9\xeb\x50\xb0\xdb\xc8\xa8\xe4\x63\x80\
\xdd\x31\xd6\x9d\xbb\xf2\xc5\x42\x1f\x6c\x6f\xf4\x34\xdd\x3c\xfc\
\xac\xe7\x3d\x80\xa9\xcc\x87\x34\xb3\x37\xfa\x2b\x9f\x6a\xad\x63\
\x20\x36\x9f\x78\x64\x75\xe6\xab\x7d\xb2\xde\x29\x70\xd3\x20\x27\
\xde\xaf\xa4\xf0\xca\x9f\x24\xa8\xdf\x46\xa8\x24\x84\x96\xe3\x77\
\xf9\x2e\xe0\x0a\x62\x7f\xdf\xd9";
    assert_eq!(input.len(), 504);
    assert!(jpeg::decode(&mut &input[..]).is_err());
}

fn base64_decode(s: &str) -> Vec<u8> {
    let mut out = Vec::new();
    let (mut acc, mut n) = (0u32, 0u32);
    for c in s.bytes() {
        let v = match c {
            b'A'..=b'Z' => c - b'A',
            b'a'..=b'z' => c - b'a' + 26,
            b'0'..=b'9' => c - b'0' + 52,
            b'+' => 62,
            b'/' => 63,
            _ => continue,
        } as u32;
        acc = acc << 6 | v;
        n += 6;
        if n >= 8 {
            n -= 8;
            out.push((acc >> n) as u8);
        }
    }
    out
}

// Go: TestPaddedRSTMarker
#[test]
fn padded_rst_marker() {
    let b64 = std::fs::read_to_string(fixtures_dir().join("padded-rst-marker.b64")).unwrap();
    let data = base64_decode(&b64);
    jpeg::decode(&mut &data[..]).unwrap();
}

// Go: averageDelta
fn average_delta(m0: &dyn Image, m1: &dyn Image) -> i64 {
    let b = m0.bounds();
    let (mut sum, mut n) = (0i64, 0i64);
    for y in b.min.y..b.max.y {
        for x in b.min.x..b.max.x {
            let (r0, g0, b0, _) = m0.at(x, y).rgba();
            let (r1, g1, b1, _) = m1.at(x, y).rgba();
            sum += (r0 as i64 - r1 as i64).abs();
            sum += (g0 as i64 - g1 as i64).abs();
            sum += (b0 as i64 - b1 as i64).abs();
            n += 3;
        }
    }
    sum / n
}

// Go: TestExtraneousData (random stream from splitmix64 instead of math/rand)
#[test]
fn extraneous_data() {
    let mut src = RGBA::new(rect(0, 0, 1, 1));
    src.set(
        0,
        0,
        Color::RGBA(color::RGBA {
            r: 0xff,
            g: 0,
            b: 0,
            a: 0xff,
        }),
    );
    let mut enc = Vec::new();
    jpeg::encode(&mut enc, &src, None).unwrap();
    assert!(enc.len() >= 64);
    assert_eq!(&enc[enc.len() - 2..], b"\xff\xd9");
    assert!(enc[enc.len() - 64..].windows(2).any(|w| w == b"\xff\xda"));
    let mut rnd = Rng::new(1);
    for i in 0..1000 {
        let mut buf = enc[..enc.len() - 2].to_vec();
        let mut n = rnd.intn(10);
        while n > 0 {
            let x = rnd.intn(256) as u8;
            if x != 0xff {
                buf.push(x);
            } else {
                buf.extend_from_slice(b"\xff\x00");
            }
            n -= 1;
        }
        buf.extend_from_slice(b"\xff\xd9");
        let got = jpeg::decode(&mut &buf[..]).unwrap_or_else(|e| panic!("#{}: {}", i, e));
        assert_eq!(got.bounds(), src.bounds());
        assert!(average_delta(got.as_ref(), &src) <= 2 << 8);
    }
}

// Go: TestIssue56724
#[test]
fn issue_56724() {
    let b = testdata("video-001.jpeg");
    let err = jpeg::decode(&mut &b[..24]).err().unwrap();
    assert_eq!(err, Error::UnexpectedEof);
    assert_eq!(err.to_string(), "unexpected EOF");
}

// Go: TestIssue78368
#[test]
fn issue_78368() {
    let data: &[u8] = &[
        0xff, 0xd8, 0xff, 0xdb, 0x00, 0x84, 0x00, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0xff, 0x20,
        0x20, 0x20, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
        0xff, 0x20, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
        0xff, 0xff, 0x20, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0x01, 0xff, 0xff, 0xff,
        0x20, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0x20, 0xff,
        0xff, 0x20, 0x20, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0x20, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
        0xff, 0xff, 0xff, 0xff, 0xc2, 0x00, 0x11, 0x08, 0x00, 0x20, 0x00, 0x20, 0x03, 0x01, 0x11,
        0x00, 0x20, 0x21, 0x01, 0xff, 0x11, 0x01, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20,
        0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20,
        0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20,
        0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20,
        0x20, 0x20, 0x20, 0xff, 0xc4, 0x00, 0x27, 0x10, 0x01, 0x00, 0x02, 0x01, 0x04, 0x01, 0x03,
        0x04, 0x03, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20,
        0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0xff,
        0xda, 0x00, 0x08, 0x01, 0x20, 0x20, 0x20, 0x20, 0xed, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20,
        0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20,
        0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20,
        0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20,
        0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0xff, 0xee, 0x00,
        0x0e, 0x41, 0x64, 0x6f, 0x62, 0x65, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x00, 0xff, 0xdb,
        0x00, 0x43, 0x00, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20,
        0xff, 0xff, 0xff, 0xff, 0xff, 0x20, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
        0xff, 0x20, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
        0xff, 0xff, 0xff, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20,
        0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20,
        0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20,
        0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20,
        0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20,
        0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0xff, 0xd9, 0x20, 0x20, 0x20, 0x20,
        0x20, 0x20, 0x20, 0x20, 0x20,
    ];
    // Must not panic (the result is irrelevant).
    let _ = jpeg::decode(&mut &data[..]);
}

// Go: TestBadRestartMarker
#[test]
fn bad_restart_marker() {
    let b = testdata("video-001.restart2.jpeg");
    assert_eq!(b.len(), 4855);
    assert_eq!((b[2816], b[2817]), (0xff, 0xd1));
    let (prefix, suffix) = b.split_at(2816);
    let cases: [(&str, &[u8]); 10] = [
        ("PASS", b""),
        ("PASS", b"\x00"),
        ("PASS", b"\x61"),
        ("PASS", b"\x61\x62\x63\xff\x00\x64"),
        ("PASS", b"\xff"),
        ("PASS", b"\xff\x00"),
        ("PASS", b"\xff\xff\xff\x00\xff\x00\x00\xff\xff\xff"),
        ("FAIL", b"\xff\x03"),
        ("FAIL", b"\xff\xd5"),
        ("FAIL", b"\xff\xff\xd5"),
    ];
    for (want, infix) in cases {
        let mut data = prefix.to_vec();
        data.extend_from_slice(infix);
        data.extend_from_slice(suffix);
        let got = jpeg::decode(&mut &data[..]).is_ok();
        assert_eq!(got, want == "PASS", "{:?}", infix);
    }
}

// Go: TestDecodeFlexSubsampling
#[test]
fn decode_flex_subsampling() {
    for f in [
        "video-001.q50.221122.jpeg",
        "video-001.q50.211211.jpeg",
        "video-001.q50.222112.jpeg",
        "video-001.q50.121121.jpeg",
    ] {
        let m = decode_file(f).unwrap();
        assert_eq!(m.bounds(), rect(0, 0, 150, 103));
        let y = m.downcast_ref::<YCbCr>().unwrap();
        assert_eq!(y.subsample_ratio, YCbCrSubsampleRatio::Ratio444);
    }
}

// Go: TestWriteGrayscale
#[test]
fn write_grayscale() {
    let mut m0 = Gray::new(rect(0, 0, 32, 32));
    for (i, p) in m0.pix.iter_mut().enumerate() {
        *p = i as u8;
    }
    let mut buf = Vec::new();
    jpeg::encode(&mut buf, &m0, None).unwrap();
    let m1 = jpeg::decode(&mut &buf[..]).unwrap();
    assert_eq!(m0.bounds(), m1.bounds());
    assert!(m1.is::<Gray>());
    assert!(average_delta(&m0, m1.as_ref()) <= 2 << 8);
}

// Go: TestEncodeYCbCr (random stream from splitmix64 instead of math/rand)
#[test]
fn encode_ycbcr() {
    let bo = rect(0, 0, 640, 480);
    let mut img_rgba = RGBA::new(bo);
    let mut img_ycbcr = YCbCr::new(bo, YCbCrSubsampleRatio::Ratio444);
    let mut rnd = Rng::new(123);
    for y in bo.min.y..bo.max.y {
        for x in bo.min.x..bo.max.x {
            let col = color::RGBA {
                r: rnd.intn(256) as u8,
                g: rnd.intn(256) as u8,
                b: rnd.intn(256) as u8,
                a: 255,
            };
            img_rgba.set_rgba(x, y, col);
            let yo = img_ycbcr.y_offset(x, y) as usize;
            let co = img_ycbcr.c_offset(x, y) as usize;
            let (cy, ccr, ccb) = color::rgb_to_ycbcr(col.r, col.g, col.b);
            img_ycbcr.y[yo] = cy;
            img_ycbcr.cb[co] = ccr;
            img_ycbcr.cr[co] = ccb;
        }
    }
    let mut buf_rgba = Vec::new();
    let mut buf_ycbcr = Vec::new();
    jpeg::encode(&mut buf_rgba, &img_rgba, None).unwrap();
    jpeg::encode(&mut buf_ycbcr, &img_ycbcr, None).unwrap();
    assert!(buf_rgba == buf_ycbcr, "RGBA and YCbCr encoded bytes differ");
}

#[test]
fn encode_too_large() {
    let m = go_image::Uniform::new(Color::Gray(color::Gray { y: 1 }));
    let mut buf = Vec::new();
    let err = jpeg::encode(&mut buf, &m, None).err().unwrap();
    assert_eq!(err.to_string(), "jpeg: image is too large to encode");
}

#[test]
fn registry_decode() {
    jpeg::register();
    let data = testdata("video-001.jpeg");
    let (m, name) = go_image::decode(&mut &data[..]).unwrap();
    assert_eq!(name, "jpeg");
    assert_eq!(
        img_digest(m.as_ref()),
        img_digest(jpeg::decode(&mut &data[..]).unwrap().as_ref())
    );
    let (c, name) = go_image::decode_config(&mut &data[..]).unwrap();
    assert_eq!(name, "jpeg");
    assert_eq!((c.width, c.height), (150, 103));
    let err = go_image::decode(&mut &b"GIF89a"[..]).err().unwrap();
    assert_eq!(err.to_string(), "image: unknown format");
}
