//! Port of Go 1.27.1 `image/format.go` (the format registry) and
//! `image.Config` from `image/image.go`.
//!
//! Go registers formats from package `init` functions; Rust has no such
//! hook, so decoders register explicitly ([`crate::jpeg::register`] for this
//! crate; sibling crates such as go-png provide their own `register`).
//! Registration order determines sniffing order, exactly as in Go.

use std::fmt;
use std::io::Read;
use std::sync::RwLock;

use crate::color::Model;
use crate::image::Image;

/// A boxed decoder error.
pub type BoxError = Box<dyn std::error::Error + Send + Sync + 'static>;

/// Config holds an image's color model and dimensions.
///
/// Go: image/image.go:Config
#[derive(Clone, Debug, PartialEq)]
pub struct Config {
    pub color_model: Model,
    pub width: i64,
    pub height: i64,
}

/// ErrFormat indicates that decoding encountered an unknown format.
///
/// Go: image/format.go:ErrFormat
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ErrFormat;

impl fmt::Display for ErrFormat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("image: unknown format")
    }
}

impl std::error::Error for ErrFormat {}

/// A registered decode function (Go: `func(io.Reader) (Image, error)`).
pub type DecodeFn = fn(&mut dyn Read) -> Result<Box<dyn Image>, BoxError>;
/// A registered decode-config function (Go: `func(io.Reader) (Config, error)`).
pub type DecodeConfigFn = fn(&mut dyn Read) -> Result<Config, BoxError>;

// A format holds an image format's name, magic header and how to decode it.
// Go: image/format.go:format
#[derive(Clone)]
struct Format {
    name: String,
    magic: Vec<u8>,
    decode: DecodeFn,
    decode_config: DecodeConfigFn,
}

// Go: image/format.go:formatsMu, atomicFormats
static FORMATS: RwLock<Vec<Format>> = RwLock::new(Vec::new());

/// RegisterFormat registers an image format for use by [`decode`]. Name is the
/// name of the format, like "jpeg" or "png". Magic is the magic prefix that
/// identifies the format's encoding. The magic string can contain "?"
/// wildcards that each match any one byte.
///
/// Go: image/format.go:RegisterFormat
pub fn register_format(name: &str, magic: &[u8], decode: DecodeFn, decode_config: DecodeConfigFn) {
    let mut formats = FORMATS.write().unwrap_or_else(|e| e.into_inner());
    formats.push(Format {
        name: name.to_string(),
        magic: magic.to_vec(),
        decode,
        decode_config,
    });
}

// Go: image/format.go:asReader wraps readers without a Peek method in a
// bufio.NewReader (4096-byte buffer). This is a port of the parts of
// bufio.Reader that image.Decode uses (Peek and Read), so the registered
// decoder sees exactly the read chunking it sees in Go (4096, then 4094/2
// alternating for the JPEG decoder over a file or bytes.Reader).
const BUFIO_DEFAULT_BUF_SIZE: usize = 4096;
// Go: bufio.maxConsecutiveEmptyReads
const MAX_CONSECUTIVE_EMPTY_READS: usize = 100;

enum BufErr {
    // io.EOF (a Rust reader's Ok(0)).
    Eof,
    // io.ErrNoProgress.
    NoProgress,
    Io(std::io::Error),
}

struct BufioReader<'a> {
    buf: Vec<u8>,
    rd: &'a mut dyn Read,
    r: usize,
    w: usize,
    err: Option<BufErr>,
}

impl<'a> BufioReader<'a> {
    // Go: bufio.NewReader
    fn new(rd: &'a mut dyn Read) -> Self {
        BufioReader {
            buf: vec![0; BUFIO_DEFAULT_BUF_SIZE],
            rd,
            r: 0,
            w: 0,
            err: None,
        }
    }

    // Go: bufio.Reader.fill
    fn fill(&mut self) {
        // Slide existing data to beginning.
        if self.r > 0 {
            self.buf.copy_within(self.r..self.w, 0);
            self.w -= self.r;
            self.r = 0;
        }

        if self.w >= self.buf.len() {
            panic!("bufio: tried to fill full buffer");
        }

        // Read new data: try a limited number of times.
        let mut i = MAX_CONSECUTIVE_EMPTY_READS;
        while i > 0 {
            match self.rd.read(&mut self.buf[self.w..]) {
                Ok(0) => {
                    // A Rust reader reports io.EOF as Ok(0).
                    self.err = Some(BufErr::Eof);
                    return;
                }
                Ok(n) => {
                    self.w += n;
                    return;
                }
                Err(e) if e.kind() == std::io::ErrorKind::Interrupted => {}
                Err(e) => {
                    self.err = Some(BufErr::Io(e));
                    return;
                }
            }
            i -= 1;
        }
        self.err = Some(BufErr::NoProgress);
    }

    // Go: bufio.Reader.readErr, mapped to Read's result (io.EOF -> Ok(0)).
    fn read_err(&mut self) -> std::io::Result<usize> {
        match self.err.take() {
            None | Some(BufErr::Eof) => Ok(0),
            Some(BufErr::NoProgress) => Err(std::io::Error::other(
                "multiple Read calls return no data or error",
            )),
            Some(BufErr::Io(e)) => Err(e),
        }
    }

    // Go: bufio.Reader.Peek — returns the next n bytes without advancing, or
    // None when Go returns an error (fewer than n bytes available).
    fn peek(&mut self, n: usize) -> Option<&[u8]> {
        while self.w - self.r < n && self.w - self.r < self.buf.len() && self.err.is_none() {
            self.fill(); // self.w-self.r < len(self.buf) => buffer is not full
        }

        if n > self.buf.len() {
            return None;
        }

        // 0 <= n <= len(self.buf)
        if self.w - self.r < n {
            // not enough data in buffer; Go returns (and clears) the error.
            self.err = None;
            return None;
        }
        Some(&self.buf[self.r..self.r + n])
    }
}

impl Read for BufioReader<'_> {
    // Go: bufio.Reader.Read
    fn read(&mut self, p: &mut [u8]) -> std::io::Result<usize> {
        if p.is_empty() {
            return Ok(0);
        }
        if self.r == self.w {
            if self.err.is_some() {
                return self.read_err();
            }
            if p.len() >= self.buf.len() {
                // Large read, empty buffer.
                // Read directly into p to avoid copy.
                return loop {
                    match self.rd.read(p) {
                        Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
                        r => break r,
                    }
                };
            }
            // One read.
            // Do not use self.fill, which will loop.
            self.r = 0;
            self.w = 0;
            let n = loop {
                match self.rd.read(&mut self.buf) {
                    Ok(n) => break n,
                    Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
                    Err(e) => return Err(e),
                }
            };
            if n == 0 {
                return Ok(0);
            }
            self.w += n;
        }

        // copy as much as we can
        let n = p.len().min(self.w - self.r);
        p[..n].copy_from_slice(&self.buf[self.r..self.r + n]);
        self.r += n;
        Ok(n)
    }
}

// Match reports whether magic matches b. Magic may contain "?" wildcards.
// Go: image/format.go:match
fn match_magic(magic: &[u8], b: &[u8]) -> bool {
    if magic.len() != b.len() {
        return false;
    }
    for (i, &c) in b.iter().enumerate() {
        if magic[i] != c && magic[i] != b'?' {
            return false;
        }
    }
    true
}

// Sniff determines the format of r's data.
// Go: image/format.go:sniff
fn sniff(r: &mut BufioReader<'_>) -> Option<Format> {
    let formats = FORMATS.read().unwrap_or_else(|e| e.into_inner()).clone();
    for f in formats {
        if let Some(b) = r.peek(f.magic.len()) {
            if match_magic(&f.magic, b) {
                return Some(f);
            }
        }
    }
    None
}

/// Decode decodes an image that has been encoded in a registered format.
/// The string returned is the format name used during format registration.
///
/// Go: image/format.go:Decode. On a decoder error Go also returns the
/// format name; the port returns only the error. The reader is wrapped in a
/// port of `bufio.Reader` exactly as Go's `asReader` does.
pub fn decode(r: &mut dyn Read) -> Result<(Box<dyn Image>, String), BoxError> {
    let mut rr = BufioReader::new(r);
    let f = match sniff(&mut rr) {
        Some(f) => f,
        None => return Err(Box::new(ErrFormat)),
    };
    let m = (f.decode)(&mut rr)?;
    Ok((m, f.name))
}

/// DecodeConfig decodes the color model and dimensions of an image that has
/// been encoded in a registered format.
///
/// Go: image/format.go:DecodeConfig
pub fn decode_config(r: &mut dyn Read) -> Result<(Config, String), BoxError> {
    let mut rr = BufioReader::new(r);
    let f = match sniff(&mut rr) {
        Some(f) => f,
        None => return Err(Box::new(ErrFormat)),
    };
    let c = (f.decode_config)(&mut rr)?;
    Ok((c, f.name))
}
