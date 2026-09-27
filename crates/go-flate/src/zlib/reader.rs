//! Port of go1.27.1 `compress/zlib/reader.go`.

use std::io::BufRead;

use crate::adler32;
use crate::error::Error;
use crate::flate;
use crate::flate::Decompressor;

const ZLIB_DEFLATE: u8 = 8;
const ZLIB_MAX_WINDOW: u8 = 7;

/// The zlib reader (Go's unexported `reader`, returned by `NewReader` as an
/// `io.ReadCloser` that also implements `Resetter`).
///
/// `R` plays the role of Go's `flate.Reader` (io.Reader + io.ByteReader).
/// Go wraps a plain `io.Reader` in `bufio.NewReader` (4096 bytes); do the same
/// with `std::io::BufReader::with_capacity(4096, r)`.
pub struct Reader<R: BufRead> {
    /// Go's `z.r` when it is not (yet) owned by the decompressor.
    r: Option<R>,
    decompressor: Option<Decompressor<R>>,
    digest: adler32::Digest,
    err: Option<Error>,
    scratch: [u8; 4],
}

// Go: compress/zlib/reader.go:NewReader
/// NewReader creates a new zlib reader.
/// Reads from the returned reader read and decompress data from r.
/// It is the caller's responsibility to call Close on the reader when done.
pub fn new_reader<R: BufRead>(r: R) -> Result<Reader<R>, Error> {
    new_reader_dict(r, None)
}

// Go: compress/zlib/reader.go:NewReaderDict
/// NewReaderDict is like [`new_reader`] but uses a preset dictionary.
/// NewReaderDict ignores the dictionary if the compressed data does not refer to it.
/// If the compressed data refers to a different dictionary, NewReaderDict returns
/// `Error::ZlibDictionary`.
pub fn new_reader_dict<R: BufRead>(r: R, dict: Option<&[u8]>) -> Result<Reader<R>, Error> {
    let mut z = Reader {
        r: None,
        decompressor: None,
        digest: adler32::Digest::new(),
        err: None,
        scratch: [0; 4],
    };
    z.reset(r, dict)?;
    Ok(z)
}

impl<R: BufRead> Reader<R> {
    fn r_mut(&mut self) -> &mut R {
        match (&mut self.r, &mut self.decompressor) {
            (Some(r), _) => r,
            (None, Some(d)) => d.get_mut(),
            (None, None) => unreachable!("zlib reader without input"),
        }
    }

    // Go: compress/zlib/reader.go:(*reader).Read
    /// Go semantics: returns the number of bytes read and an error
    /// (`Error::Eof` at the end of the stream, after the checksum is verified).
    pub fn read(&mut self, p: &mut [u8]) -> (usize, Option<Error>) {
        if let Some(e) = &self.err {
            return (0, Some(e.clone()));
        }

        let (n, err) = self.decompressor.as_mut().unwrap().read(p);
        self.err = err;
        self.digest.write(&p[0..n]);
        if !matches!(self.err, Some(Error::Eof)) {
            // In the normal case we return here.
            return (n, self.err.clone());
        }

        // Finished file; check checksum.
        let mut scratch = [0u8; 4];
        let (_, err) = crate::flate_read_full(self.r_mut(), &mut scratch);
        self.scratch = scratch;
        if let Some(mut err) = err {
            if let Error::Eof = err {
                err = Error::UnexpectedEof;
            }
            self.err = Some(err);
            return (n, self.err.clone());
        }
        // ZLIB (RFC 1950) is big-endian, unlike GZIP (RFC 1952).
        let checksum = u32::from_be_bytes(self.scratch);
        if checksum != self.digest.sum32() {
            self.err = Some(Error::ZlibChecksum);
            return (n, self.err.clone());
        }
        (n, Some(Error::Eof))
    }

    // Go: compress/zlib/reader.go:(*reader).Close
    /// Calling Close does not close the wrapped reader originally passed to NewReader.
    /// In order for the ZLIB checksum to be verified, the reader must be
    /// fully consumed until the EOF.
    pub fn close(&mut self) -> Result<(), Error> {
        if let Some(e) = &self.err
            && !matches!(e, Error::Eof)
        {
            return Err(e.clone());
        }
        self.err = self.decompressor.as_mut().unwrap().close().err();
        match &self.err {
            Some(e) => Err(e.clone()),
            None => Ok(()),
        }
    }

    // Go: compress/zlib/reader.go:(*reader).Reset
    /// Reset discards any buffered data and resets the reader as if it was
    /// newly initialized with the given reader.
    pub fn reset(&mut self, r: R, dict: Option<&[u8]>) -> Result<(), Error> {
        // *z = reader{decompressor: z.decompressor}
        self.r = Some(r);
        self.digest = adler32::Digest::new();
        self.err = None;
        self.scratch = [0; 4];

        // Read the header (RFC 1950 section 2.2.).
        let mut scratch = [0u8; 4];
        let (_, err) = crate::flate_read_full(self.r.as_mut().unwrap(), &mut scratch[0..2]);
        self.scratch = scratch;
        if let Some(mut e) = err {
            if let Error::Eof = e {
                e = Error::UnexpectedEof;
            }
            self.err = Some(e.clone());
            return Err(e);
        }
        let h = u16::from_be_bytes([self.scratch[0], self.scratch[1]]);
        if (self.scratch[0] & 0x0f != ZLIB_DEFLATE)
            || (self.scratch[0] >> 4 > ZLIB_MAX_WINDOW)
            || (h % 31 != 0)
        {
            self.err = Some(Error::ZlibHeader);
            return Err(Error::ZlibHeader);
        }
        let have_dict = self.scratch[1] & 0x20 != 0;
        if have_dict {
            let mut scratch = [0u8; 4];
            let (_, err) = crate::flate_read_full(self.r.as_mut().unwrap(), &mut scratch[0..4]);
            self.scratch = scratch;
            if let Some(mut e) = err {
                if let Error::Eof = e {
                    e = Error::UnexpectedEof;
                }
                self.err = Some(e.clone());
                return Err(e);
            }
            let checksum = u32::from_be_bytes(self.scratch);
            if checksum != adler32::checksum(dict.unwrap_or(&[])) {
                self.err = Some(Error::ZlibDictionary);
                return Err(Error::ZlibDictionary);
            }
        }

        let r = self.r.take().unwrap();
        match self.decompressor.as_mut() {
            None => {
                if have_dict {
                    self.decompressor = Some(flate::new_reader_dict(r, dict.unwrap_or(&[])));
                } else {
                    self.decompressor = Some(flate::new_reader(r));
                }
            }
            Some(d) => {
                d.reset(r, dict.unwrap_or(&[]));
            }
        }
        self.digest = adler32::Digest::new();
        Ok(())
    }

    /// Returns a mutable reference to the underlying reader.
    pub fn get_mut(&mut self) -> &mut R {
        self.r_mut()
    }
}

/// `std::io::Read` adapter: data is returned first, a pending error on the
/// next call; `Error::Eof` becomes `Ok(0)`.
impl<R: BufRead> std::io::Read for Reader<R> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        if buf.is_empty() {
            return Ok(0);
        }
        let (n, err) = Reader::read(self, buf);
        if n > 0 {
            return Ok(n);
        }
        match err {
            None | Some(Error::Eof) => Ok(0),
            Some(e) => Err(e.into()),
        }
    }
}
