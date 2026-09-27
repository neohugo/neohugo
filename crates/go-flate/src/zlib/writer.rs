//! Port of go1.27.1 `compress/zlib/writer.go`.

use std::io::Write;

use super::{BEST_COMPRESSION, DEFAULT_COMPRESSION, HUFFMAN_ONLY};
use crate::adler32;
use crate::error::Error;
use crate::flate;

/// Go's `z.w` and `z.compressor` refer to the same `io.Writer`; in Rust the
/// writer is owned by the flate compressor once it exists.
enum Sink<W: Write> {
    Raw(W),
    Flate(Box<flate::Writer<W>>),
    /// Transient state while moving the writer into the compressor.
    Empty,
}

/// A Writer takes data written to it and writes the compressed
/// form of that data to an underlying writer (see [`new_writer`]).
pub struct Writer<W: Write> {
    sink: Sink<W>,
    level: i32,
    dict: Option<Vec<u8>>,
    digest: Option<adler32::Digest>,
    err: Option<Error>,
    scratch: [u8; 4],
    wrote_header: bool,
}

// Go: compress/zlib/writer.go:NewWriter
/// NewWriter creates a new [`Writer`].
/// Writes to the returned Writer are compressed and written to w.
///
/// It is the caller's responsibility to call Close on the Writer when done.
/// Writes may be buffered and not flushed until Close.
pub fn new_writer<W: Write>(w: W) -> Writer<W> {
    new_writer_level_dict(w, DEFAULT_COMPRESSION, None).expect("default level is valid")
}

// Go: compress/zlib/writer.go:NewWriterLevel
/// NewWriterLevel is like [`new_writer`] but specifies the compression level instead
/// of assuming DefaultCompression.
///
/// The compression level can be DefaultCompression, NoCompression, HuffmanOnly
/// or any integer value between BestSpeed and BestCompression inclusive.
/// The error returned will be nil if the level is valid.
pub fn new_writer_level<W: Write>(w: W, level: i32) -> Result<Writer<W>, Error> {
    new_writer_level_dict(w, level, None)
}

// Go: compress/zlib/writer.go:NewWriterLevelDict
/// NewWriterLevelDict is like [`new_writer_level`] but specifies a dictionary to
/// compress with.
///
/// The dictionary may be nil (`None`). Go distinguishes a nil dictionary from an
/// empty one (an empty non-nil dictionary sets FDICT in the header).
pub fn new_writer_level_dict<W: Write>(
    w: W,
    level: i32,
    dict: Option<&[u8]>,
) -> Result<Writer<W>, Error> {
    if !(HUFFMAN_ONLY..=BEST_COMPRESSION).contains(&level) {
        return Err(Error::ZlibInvalidLevel(level as i64));
    }
    Ok(Writer {
        sink: Sink::Raw(w),
        level,
        dict: dict.map(|d| d.to_vec()),
        digest: None,
        err: None,
        scratch: [0; 4],
        wrote_header: false,
    })
}

impl<W: Write> Writer<W> {
    fn w_mut(&mut self) -> &mut W {
        match &mut self.sink {
            Sink::Raw(w) => w,
            Sink::Flate(f) => f.get_mut(),
            Sink::Empty => unreachable!("zlib writer sink moved"),
        }
    }

    /// Go: `z.w.Write(b)` — one `write_all` call.
    fn write_raw(&mut self, n: usize) -> Result<(), Error> {
        let scratch = self.scratch;
        crate::go_write(self.w_mut(), &scratch[..n]).map_err(Error::io)
    }

    // Go: compress/zlib/writer.go:(*Writer).Reset
    /// Reset clears the state of the [`Writer`] z such that it is equivalent to its
    /// initial state from NewWriterLevel or NewWriterLevelDict, but instead writing
    /// to w. Returns the previous underlying writer.
    pub fn reset(&mut self, w: W) -> W {
        // z.level and z.dict left unchanged.
        let old = match &mut self.sink {
            Sink::Raw(old) => std::mem::replace(old, w),
            Sink::Flate(f) => f.reset(w),
            Sink::Empty => unreachable!("zlib writer sink moved"),
        };
        if let Some(d) = self.digest.as_mut() {
            d.reset();
        }
        self.err = None;
        self.scratch = [0; 4];
        self.wrote_header = false;
        old
    }

    // Go: compress/zlib/writer.go:(*Writer).writeHeader
    /// writeHeader writes the ZLIB header.
    fn write_header(&mut self) -> Result<(), Error> {
        self.wrote_header = true;
        // ZLIB has a two-byte header (as documented in RFC 1950).
        // The first four bits is the CINFO (compression info), which is 7 for the default deflate window size.
        // The next four bits is the CM (compression method), which is 8 for deflate.
        self.scratch[0] = 0x78;
        // The next two bits is the FLEVEL (compression level). The four values are:
        // 0=fastest, 1=fast, 2=default, 3=best.
        // The next bit, FDICT, is set if a dictionary is given.
        // The final five FCHECK bits form a mod-31 checksum.
        self.scratch[1] = match self.level {
            -2 | 0 | 1 => 0 << 6,
            2..=5 => 1 << 6,
            6 | -1 => 2 << 6,
            7..=9 => 3 << 6,
            _ => panic!("unreachable"),
        };
        if self.dict.is_some() {
            self.scratch[1] |= 1 << 5;
        }
        let h = u16::from_be_bytes([self.scratch[0], self.scratch[1]]);
        self.scratch[1] = self.scratch[1].wrapping_add((31 - h % 31) as u8);
        self.write_raw(2)?;
        if let Some(dict) = &self.dict {
            // The next four bytes are the Adler-32 checksum of the dictionary.
            self.scratch = adler32::checksum(dict).to_be_bytes();
            self.write_raw(4)?;
        }
        if matches!(self.sink, Sink::Raw(_)) {
            // Initialize deflater unless the Writer is being reused
            // after a Reset call.
            let Sink::Raw(w) = std::mem::replace(&mut self.sink, Sink::Empty) else {
                unreachable!()
            };
            let dict: &[u8] = self.dict.as_deref().unwrap_or(&[]);
            // The level was validated by NewWriterLevelDict, so this cannot fail.
            let c = flate::new_writer_dict(w, self.level, dict)?;
            self.sink = Sink::Flate(Box::new(c));
            self.digest = Some(adler32::Digest::new());
        }
        Ok(())
    }

    fn compressor(&mut self) -> &mut flate::Writer<W> {
        match &mut self.sink {
            Sink::Flate(f) => f,
            _ => unreachable!("zlib compressor not initialized"),
        }
    }

    // Go: compress/zlib/writer.go:(*Writer).Write
    /// Write writes a compressed form of p to the underlying writer. The
    /// compressed bytes are not necessarily flushed until the [`Writer`] is closed or
    /// explicitly flushed.
    pub fn write(&mut self, p: &[u8]) -> Result<usize, Error> {
        if !self.wrote_header {
            self.err = self.write_header().err();
        }
        if let Some(e) = &self.err {
            return Err(e.clone());
        }
        if p.is_empty() {
            return Ok(0);
        }
        match self.compressor().write(p) {
            Err(e) => {
                self.err = Some(e.clone());
                Err(e)
            }
            Ok(n) => {
                self.digest.as_mut().unwrap().write(p);
                Ok(n)
            }
        }
    }

    // Go: compress/zlib/writer.go:(*Writer).Flush
    /// Flush flushes the Writer to its underlying writer.
    pub fn flush(&mut self) -> Result<(), Error> {
        if !self.wrote_header {
            self.err = self.write_header().err();
        }
        if let Some(e) = &self.err {
            return Err(e.clone());
        }
        self.err = self.compressor().flush().err();
        match &self.err {
            Some(e) => Err(e.clone()),
            None => Ok(()),
        }
    }

    // Go: compress/zlib/writer.go:(*Writer).Close
    /// Close closes the Writer, flushing any unwritten data to the underlying
    /// writer, but does not close the underlying writer.
    pub fn close(&mut self) -> Result<(), Error> {
        if !self.wrote_header {
            self.err = self.write_header().err();
        }
        if let Some(e) = &self.err {
            return Err(e.clone());
        }
        self.err = self.compressor().close().err();
        if let Some(e) = &self.err {
            return Err(e.clone());
        }
        let checksum = self.digest.as_ref().unwrap().sum32();
        // ZLIB (RFC 1950) is big-endian, unlike GZIP (RFC 1952).
        self.scratch = checksum.to_be_bytes();
        self.err = self.write_raw(4).err();
        match &self.err {
            Some(e) => Err(e.clone()),
            None => Ok(()),
        }
    }

    /// Returns a reference to the underlying writer.
    pub fn get_ref(&self) -> &W {
        match &self.sink {
            Sink::Raw(w) => w,
            Sink::Flate(f) => f.get_ref(),
            Sink::Empty => unreachable!("zlib writer sink moved"),
        }
    }

    /// Returns a mutable reference to the underlying writer.
    pub fn get_mut(&mut self) -> &mut W {
        self.w_mut()
    }

    /// Consumes the Writer (without closing it) and returns the underlying writer.
    pub fn into_inner(self) -> W {
        match self.sink {
            Sink::Raw(w) => w,
            Sink::Flate(f) => f.into_inner(),
            Sink::Empty => unreachable!("zlib writer sink moved"),
        }
    }
}

/// `std::io::Write` adapter. `write` is Go's `Write`; `flush` is Go's
/// `Flush` (it emits a deflate sync marker).
impl<W: Write> Write for Writer<W> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        Writer::write(self, buf).map_err(Into::into)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Writer::flush(self).map_err(Into::into)
    }
}
