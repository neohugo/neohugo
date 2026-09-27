//! Go: parse/buffer/writer.go

use crate::error::GoError;
use crate::gobytes::GoBytes;

/// Go: buffer.Writer — an `io.Writer` over a byte slice.
#[derive(Clone)]
pub struct Writer {
    buf: GoBytes,
    err: Option<GoError>,
    expand: bool,
}

impl Writer {
    // Go: parse/buffer/writer.go:NewWriter
    /// Returns a new Writer for a given byte slice.
    pub fn new(buf: GoBytes) -> Writer {
        Writer {
            buf,
            err: None,
            expand: true,
        }
    }

    // Go: parse/buffer/writer.go:NewStaticWriter
    /// Returns a new Writer that does not reallocate and expand the slice.
    pub fn new_static(buf: GoBytes) -> Writer {
        Writer {
            buf,
            err: None,
            expand: false,
        }
    }

    // Go: parse/buffer/writer.go:Writer.Write
    /// Writes bytes and returns the number of bytes written and an error if
    /// occurred. When err != nil, n == 0.
    pub fn write(&mut self, b: &[u8]) -> (usize, Option<GoError>) {
        let n = b.len();
        let end = self.buf.len();
        if end + n > self.buf.cap() {
            if !self.expand {
                self.err = Some(GoError::Eof);
                return (0, Some(GoError::Eof));
            }
            let buf = GoBytes::make(end, 2 * self.buf.cap() + n);
            buf.copy_from(&self.buf);
            self.buf = buf;
        }
        self.buf = self.buf.slice_to(end + n);
        (self.buf.slice_from(end).copy_from_slice(b), None)
    }

    /// `Write` of a Go slice (read before writing, so aliasing is safe).
    pub fn write_bytes(&mut self, b: &GoBytes) -> (usize, Option<GoError>) {
        let v = b.to_vec();
        self.write(&v)
    }

    // Go: parse/buffer/writer.go:Writer.Len
    /// Returns the length of the underlying byte slice.
    pub fn len(&self) -> usize {
        self.buf.len()
    }

    pub fn is_empty(&self) -> bool {
        self.buf.is_empty()
    }

    // Go: parse/buffer/writer.go:Writer.Bytes
    /// Returns the underlying byte slice (aliases the internal buffer).
    pub fn bytes(&self) -> GoBytes {
        self.buf.clone()
    }

    // Go: parse/buffer/writer.go:Writer.Reset
    /// Empties and reuses the current buffer. Subsequent writes overwrite the
    /// buffer, so slices returned by `bytes` observe them.
    pub fn reset(&mut self) {
        self.buf = self.buf.slice_to(0);
    }

    // Go: parse/buffer/writer.go:Writer.Close
    /// Returns the last error.
    pub fn close(&self) -> Option<GoError> {
        self.err.clone()
    }
}

impl std::io::Write for Writer {
    fn write(&mut self, b: &[u8]) -> std::io::Result<usize> {
        match Writer::write(self, b) {
            (n, None) => Ok(n),
            (_, Some(e)) => Err(std::io::Error::other(e.to_string())),
        }
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
