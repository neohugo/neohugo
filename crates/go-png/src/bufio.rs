//! The parts of go1.27.1 `bufio` that `image/png` relies on.
//!
//! * [`Reader`]: `compress/zlib.NewReader` wraps the PNG decoder (an
//!   `io.Reader` that is not an `io.ByteReader`) in `bufio.NewReader` (4096
//!   bytes). The sequence and sizes of reads on the decoder decide which IDAT
//!   bytes have been consumed when the zlib stream ends, which is observable
//!   ("too much pixel data", which chunk header errors are reported), so the
//!   Go read logic (`fill`, `Read`, `ReadByte`) is ported exactly and exposed
//!   through `std::io::Read` / `std::io::BufRead` (the go-flate reader
//!   contract: `ReadByte` = `fill_buf` + `consume(1)`).
//! * [`Writer`]: the encoder writes the zlib stream through
//!   `bufio.NewWriterSize(e, 1<<15)`; every `Write` on the underlying writer
//!   becomes one IDAT chunk, so the Go `Write`/`Flush` logic (including the
//!   large-write bypass) is ported exactly.

use std::io::{self, BufRead, Read, Write};

// Go: bufio.defaultBufSize
pub(crate) const DEFAULT_BUF_SIZE: usize = 4096;
// Go: bufio.maxConsecutiveEmptyReads
const MAX_CONSECUTIVE_EMPTY_READS: usize = 100;

/// A sticky Go reader error (`b.err`).
enum ReadErr {
    /// `io.EOF` — a Rust reader's `Ok(0)`.
    Eof,
    /// `io.ErrNoProgress` (unreachable with Rust readers, whose `Ok(0)`
    /// always means EOF; kept for fidelity).
    NoProgress,
    Io(io::Error),
}

/// Port of Go's `bufio.Reader` (the subset used through `compress/flate`).
pub(crate) struct Reader<R: Read> {
    buf: Vec<u8>,
    rd: R,
    r: usize,
    w: usize,
    err: Option<ReadErr>,
}

impl<R: Read> Reader<R> {
    // Go: bufio.NewReader
    pub(crate) fn new(rd: R) -> Reader<R> {
        Reader {
            buf: vec![0; DEFAULT_BUF_SIZE],
            rd,
            r: 0,
            w: 0,
            err: None,
        }
    }

    // Go: bufio.(*Reader).fill
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
                    self.err = Some(ReadErr::Eof);
                    return;
                }
                Ok(n) => {
                    self.w += n;
                    return;
                }
                Err(e) if e.kind() == io::ErrorKind::Interrupted => {}
                Err(e) => {
                    self.err = Some(ReadErr::Io(e));
                    return;
                }
            }
            i -= 1;
        }
        self.err = Some(ReadErr::NoProgress);
    }

    // Go: bufio.(*Reader).readErr, mapped to Rust's Read convention
    // (io.EOF -> Ok(0)).
    fn read_err(&mut self) -> io::Result<usize> {
        match self.err.take() {
            None | Some(ReadErr::Eof) => Ok(0),
            Some(ReadErr::NoProgress) => Err(crate::Error::NoProgress.into_io()),
            Some(ReadErr::Io(e)) => Err(e),
        }
    }
}

impl<R: Read> Read for Reader<R> {
    // Go: bufio.(*Reader).Read
    fn read(&mut self, p: &mut [u8]) -> io::Result<usize> {
        let n = p.len();
        if n == 0 {
            if self.w - self.r > 0 {
                return Ok(0);
            }
            return self.read_err();
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
                        Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
                        r => break r,
                    }
                };
            }
            // One read.
            // Do not use b.fill, which will loop.
            self.r = 0;
            self.w = 0;
            let n = loop {
                match self.rd.read(&mut self.buf) {
                    Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
                    Err(e) => return Err(e),
                    Ok(n) => break n,
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

impl<R: Read> BufRead for Reader<R> {
    /// The buffering half of Go's `bufio.(*Reader).ReadByte`:
    ///
    /// ```go
    /// for b.r == b.w {
    ///     if b.err != nil {
    ///         return 0, b.readErr()
    ///     }
    ///     b.fill() // buffer is empty
    /// }
    /// ```
    fn fill_buf(&mut self) -> io::Result<&[u8]> {
        while self.r == self.w {
            if self.err.is_some() {
                self.read_err()?;
                return Ok(&[]);
            }
            self.fill(); // buffer is empty
        }
        Ok(&self.buf[self.r..self.w])
    }

    fn consume(&mut self, amt: usize) {
        self.r += amt;
        debug_assert!(self.r <= self.w);
    }
}

/// Port of Go's `bufio.Writer` (Write / Flush / Reset).
///
/// `write` consumes the whole slice (like every Go `io.Writer`) and returns
/// the sticky error otherwise, so a `write_all` by the zlib writer is exactly
/// one Go `Write` call.
pub(crate) struct Writer<W: Write> {
    err: Option<crate::Error>,
    buf: Vec<u8>,
    n: usize,
    wr: W,
}

impl<W: Write> Writer<W> {
    // Go: bufio.NewWriterSize
    pub(crate) fn new_size(w: W, mut size: usize) -> Writer<W> {
        if size == 0 {
            size = DEFAULT_BUF_SIZE;
        }
        Writer {
            err: None,
            buf: vec![0; size],
            n: 0,
            wr: w,
        }
    }

    /// A zero-capacity stand-in used while the real writer is moved around
    /// (never written to).
    pub(crate) fn placeholder(w: W) -> Writer<W> {
        Writer {
            err: None,
            buf: Vec::new(),
            n: 0,
            wr: w,
        }
    }

    // Go: bufio.(*Writer).Reset
    pub(crate) fn reset(&mut self, w: W) {
        if self.buf.is_empty() {
            self.buf = vec![0; DEFAULT_BUF_SIZE];
        }
        self.err = None;
        self.n = 0;
        self.wr = w;
    }

    pub(crate) fn get_mut(&mut self) -> &mut W {
        &mut self.wr
    }

    /// Records a Write error of the underlying writer that the caller
    /// observed on its behalf (Go stores it in `b.err` when `b.wr.Write`
    /// fails; see the IDAT sink in writer.rs).
    pub(crate) fn set_err(&mut self, err: crate::Error) {
        if self.err.is_none() {
            self.err = Some(err);
        }
    }

    // Go: bufio.(*Writer).Available
    fn available(&self) -> usize {
        self.buf.len() - self.n
    }

    // Go: bufio.(*Writer).Buffered
    fn buffered(&self) -> usize {
        self.n
    }

    // Go: `n, err := b.wr.Write(p)` for a Go-style writer: all or nothing.
    fn wr_write(wr: &mut W, p: &[u8]) -> (usize, Option<crate::Error>) {
        match wr.write_all(p) {
            Ok(()) => (p.len(), None),
            Err(e) => (0, Some(crate::Error::from_io(e))),
        }
    }

    // Go: bufio.(*Writer).Flush
    pub(crate) fn flush(&mut self) -> Result<(), crate::Error> {
        if let Some(e) = &self.err {
            return Err(e.clone());
        }
        if self.n == 0 {
            return Ok(());
        }
        let (n, mut err) = Self::wr_write(&mut self.wr, &self.buf[0..self.n]);
        if n < self.n && err.is_none() {
            err = Some(crate::Error::Io(std::sync::Arc::new(io::Error::new(
                io::ErrorKind::WriteZero,
                "short write",
            ))));
        }
        if let Some(err) = err {
            if n > 0 && n < self.n {
                self.buf.copy_within(n..self.n, 0);
            }
            self.n -= n;
            self.err = Some(err.clone());
            return Err(err);
        }
        self.n = 0;
        Ok(())
    }

    // Go: bufio.(*Writer).Write
    pub(crate) fn go_write(&mut self, mut p: &[u8]) -> (usize, Option<crate::Error>) {
        let mut nn = 0;
        while p.len() > self.available() && self.err.is_none() {
            let n;
            if self.buffered() == 0 {
                // Large write, empty buffer.
                // Write directly from p to avoid copy.
                let (n1, err) = Self::wr_write(&mut self.wr, p);
                n = n1;
                self.err = err;
            } else {
                n = self.available().min(p.len());
                self.buf[self.n..self.n + n].copy_from_slice(&p[..n]);
                self.n += n;
                let _ = self.flush();
            }
            nn += n;
            p = &p[n..];
        }
        if let Some(e) = &self.err {
            return (nn, Some(e.clone()));
        }
        let n = self.available().min(p.len());
        self.buf[self.n..self.n + n].copy_from_slice(&p[..n]);
        self.n += n;
        nn += n;
        (nn, None)
    }
}

impl<W: Write> Write for Writer<W> {
    fn write(&mut self, p: &[u8]) -> io::Result<usize> {
        match self.go_write(p) {
            (n, None) => Ok(n),
            (_, Some(e)) => Err(e.into_io()),
        }
    }

    /// Not called by the zlib writer (Go's flate never flushes its
    /// underlying writer); provided as Go's `Flush`.
    fn flush(&mut self) -> io::Result<()> {
        Writer::flush(self).map_err(crate::Error::into_io)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Records the size of each Write call.
    #[derive(Default)]
    struct Rec(Vec<usize>, Vec<u8>);
    impl Write for Rec {
        fn write(&mut self, p: &[u8]) -> io::Result<usize> {
            self.0.push(p.len());
            self.1.extend_from_slice(p);
            Ok(p.len())
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    #[test]
    fn writer_large_write_bypass() {
        let mut w = Writer::new_size(Rec::default(), 8);
        assert!(matches!(w.go_write(b"abc"), (3, None)));
        // 3 buffered + 10 > 8: fill to 8 and flush, then 5 remain buffered.
        assert!(matches!(w.go_write(b"0123456789"), (10, None)));
        assert_eq!(w.get_mut().0, vec![8]);
        w.flush().unwrap();
        assert_eq!(w.get_mut().0, vec![8, 5]);
        // Empty buffer, large write: written directly.
        assert_eq!(w.go_write(b"0123456789ABCDEFGH").0, 18);
        assert_eq!(w.get_mut().0, vec![8, 5, 18]);
        assert_eq!(w.get_mut().1, b"abc01234567890123456789ABCDEFGH");
    }

    #[test]
    fn reader_read_and_bytes() {
        let data: Vec<u8> = (0..10000u32).map(|i| i as u8).collect();
        let mut r = Reader::new(&data[..]);
        let mut p = [0u8; 2];
        assert_eq!(r.read(&mut p).unwrap(), 2);
        assert_eq!(r.fill_buf().unwrap().len(), 4094);
        r.consume(4094);
        let mut big = vec![0u8; 5000];
        assert_eq!(r.read(&mut big).unwrap(), 5000);
        assert_eq!(big[0], (4096 % 256) as u8);
        let mut rest = Vec::new();
        r.read_to_end(&mut rest).unwrap();
        assert_eq!(rest.len(), 10000 - 9096);
    }
}
