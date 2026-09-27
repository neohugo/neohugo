//! PNG parity: PNGs written by Go's image/png (neohugo's golden site output,
//! and the oracle's `png` command) are re-derived from their own IDAT data:
//! the filtered rows are inflated, recompressed with our zlib writer using
//! one `Write` per row (exactly like image/png's writeImage), passed through
//! a port of `bufio.Writer` (size 1<<15) whose flushes become IDAT chunks,
//! and both the zlib bytes and the IDAT chunk boundaries must match.

use std::io::{BufReader, Write};
use std::path::{Path, PathBuf};

use go_flate::zlib;

struct Png {
    width: usize,
    height: usize,
    bit_depth: u8,
    color_type: u8,
    interlace: u8,
    idat_sizes: Vec<usize>,
    idat: Vec<u8>,
}

fn parse_png(b: &[u8]) -> Png {
    assert_eq!(&b[..8], b"\x89PNG\r\n\x1a\n");
    let mut pos = 8;
    let mut png = Png {
        width: 0,
        height: 0,
        bit_depth: 0,
        color_type: 0,
        interlace: 0,
        idat_sizes: Vec::new(),
        idat: Vec::new(),
    };
    while pos < b.len() {
        let len = u32::from_be_bytes(b[pos..pos + 4].try_into().unwrap()) as usize;
        let typ = &b[pos + 4..pos + 8];
        let data = &b[pos + 8..pos + 8 + len];
        match typ {
            b"IHDR" => {
                png.width = u32::from_be_bytes(data[0..4].try_into().unwrap()) as usize;
                png.height = u32::from_be_bytes(data[4..8].try_into().unwrap()) as usize;
                png.bit_depth = data[8];
                png.color_type = data[9];
                png.interlace = data[12];
            }
            b"IDAT" => {
                png.idat_sizes.push(len);
                png.idat.extend_from_slice(data);
            }
            _ => {}
        }
        pos += 12 + len;
    }
    png
}

/// Port of Go's `bufio.Writer` (Write / Flush) over a sink that records the
/// size of each Write call (image/png turns each into an IDAT chunk).
struct BufioWriter {
    buf: Vec<u8>,
    n: usize,
    chunks: Vec<Vec<u8>>,
}

impl BufioWriter {
    fn new(size: usize) -> BufioWriter {
        BufioWriter {
            buf: vec![0; size],
            n: 0,
            chunks: Vec::new(),
        }
    }
    fn available(&self) -> usize {
        self.buf.len() - self.n
    }
    // Go: bufio.(*Writer).Flush
    fn go_flush(&mut self) {
        if self.n == 0 {
            return;
        }
        self.chunks.push(self.buf[..self.n].to_vec());
        self.n = 0;
    }
}

impl Write for BufioWriter {
    // Go: bufio.(*Writer).Write
    fn write(&mut self, mut p: &[u8]) -> std::io::Result<usize> {
        let mut nn = 0;
        while p.len() > self.available() {
            let n;
            if self.n == 0 {
                // Large write, empty buffer.
                // Write directly from p to avoid copy.
                self.chunks.push(p.to_vec());
                n = p.len();
            } else {
                n = self.available();
                self.buf[self.n..self.n + n].copy_from_slice(&p[..n]);
                self.n += n;
                self.go_flush();
            }
            nn += n;
            p = &p[n..];
        }
        let n = p.len();
        self.buf[self.n..self.n + n].copy_from_slice(p);
        self.n += n;
        nn += n;
        Ok(nn)
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

fn channels(color_type: u8) -> usize {
    match color_type {
        0 => 1,
        2 => 3,
        3 => 1,
        4 => 2,
        6 => 4,
        _ => panic!("bad color type"),
    }
}

/// Checks one Go-encoded PNG; returns an error description on mismatch.
fn check_png(path: &Path, zlib_level: i32) -> Result<(), String> {
    let b = std::fs::read(path).unwrap();
    let png = parse_png(&b);
    assert_eq!(png.interlace, 0, "Go never writes interlaced PNGs");

    // Inflate with our zlib reader (Go wraps a non-ByteReader in bufio.NewReader).
    let mut r = zlib::new_reader(BufReader::with_capacity(4096, &png.idat[..]))
        .map_err(|e| format!("{}: zlib header: {e}", path.display()))?;
    let mut raw = Vec::new();
    let mut tmp = vec![0u8; 7919];
    loop {
        let (n, err) = r.read(&mut tmp);
        raw.extend_from_slice(&tmp[..n]);
        match err {
            None => {}
            Some(e) if e.is_eof() => break,
            Some(e) => return Err(format!("{}: inflate: {e}", path.display())),
        }
    }
    let bpp = channels(png.color_type) * png.bit_depth as usize;
    let row_len = 1 + (png.width * bpp).div_ceil(8);
    if raw.len() != row_len * png.height {
        return Err(format!(
            "{}: raw {} != {}x{}",
            path.display(),
            raw.len(),
            row_len,
            png.height
        ));
    }

    // Recompress exactly like image/png: one Write per row, then Close, then bw.Flush.
    let mut zw = zlib::new_writer_level(BufioWriter::new(1 << 15), zlib_level).unwrap();
    for row in raw.chunks(row_len) {
        zw.write(row).unwrap();
    }
    zw.close().unwrap();
    let mut bw = zw.into_inner();
    bw.go_flush();
    let got: Vec<u8> = bw.chunks.concat();
    let got_sizes: Vec<usize> = bw.chunks.iter().map(|c| c.len()).collect();
    if got != png.idat {
        let first = got
            .iter()
            .zip(png.idat.iter())
            .position(|(a, b)| a != b)
            .unwrap_or(got.len().min(png.idat.len()));
        return Err(format!(
            "{}: zlib stream differs (got {} bytes, want {}, first diff at {first})",
            path.display(),
            got.len(),
            png.idat.len()
        ));
    }
    if got_sizes != png.idat_sizes {
        return Err(format!(
            "{}: IDAT chunks differ: got {:?} want {:?}",
            path.display(),
            got_sizes,
            png.idat_sizes
        ));
    }
    Ok(())
}

fn pngs_in(dir: &Path) -> Vec<PathBuf> {
    let mut v: Vec<PathBuf> = std::fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|e| e == "png"))
        .collect();
    v.sort();
    v
}

/// The Go-encoded PNGs of neohugo's golden seeksnack build (DefaultCompression).
#[test]
fn golden_site_pngs() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/png");
    let files = pngs_in(&dir);
    assert!(files.len() >= 11);
    let failures: Vec<String> = files
        .iter()
        .filter_map(|p| check_png(p, zlib::DEFAULT_COMPRESSION).err())
        .collect();
    assert!(failures.is_empty(), "{failures:#?}");
}

/// PNGs written by `go-oracle/go-flate png` (file name suffix `.l<k>.png`
/// encodes png.CompressionLevel):
/// `GO_FLATE_PNG_DIR=dir cargo test --release -- --ignored png_corpus`
#[test]
#[ignore]
fn png_corpus() {
    let Ok(dir) = std::env::var("GO_FLATE_PNG_DIR") else {
        eprintln!("GO_FLATE_PNG_DIR not set; skipping");
        return;
    };
    let files = pngs_in(Path::new(&dir));
    let mut failures = Vec::new();
    for p in &files {
        let name = p.file_name().unwrap().to_string_lossy();
        // png.CompressionLevel -> zlib level (image/png levelToZlib).
        let level = if name.ends_with(".l0.png") {
            zlib::DEFAULT_COMPRESSION
        } else if name.ends_with(".l1.png") {
            zlib::NO_COMPRESSION
        } else if name.ends_with(".l2.png") {
            zlib::BEST_SPEED
        } else if name.ends_with(".l3.png") {
            zlib::BEST_COMPRESSION
        } else {
            zlib::DEFAULT_COMPRESSION
        };
        if let Err(e) = check_png(p, level) {
            failures.push(e);
        }
    }
    eprintln!(
        "png corpus: {} files, {} failures",
        files.len(),
        failures.len()
    );
    assert!(failures.is_empty(), "{failures:#?}");
}
