//! Go: parse/buffer/reader.go

use crate::error::GoError;
use crate::gobytes::GoBytes;
use crate::input::GoReader;

/// Go: buffer.Reader — an `io.Reader` over a byte slice (with `Bytes()`).
#[derive(Clone)]
pub struct Reader {
    buf: GoBytes,
    pos: usize,
}

impl Reader {
    // Go: parse/buffer/reader.go:NewReader
    /// Returns a new Reader for a given byte slice.
    pub fn new(buf: GoBytes) -> Reader {
        Reader { buf, pos: 0 }
    }

    // Go: parse/buffer/reader.go:Reader.Read
    /// Reads bytes into the given byte slice and returns the number of bytes
    /// read and an error if occurred.
    pub fn read(&mut self, b: &mut [u8]) -> (usize, Option<GoError>) {
        if b.is_empty() {
            return (0, None);
        }
        if self.pos >= self.buf.len() {
            return (0, Some(GoError::Eof));
        }
        let src = self.buf.slice_from(self.pos);
        let n = b.len().min(src.len());
        for (k, slot) in b.iter_mut().take(n).enumerate() {
            *slot = src.at(k);
        }
        self.pos += n;
        (n, None)
    }

    // Go: parse/buffer/reader.go:Reader.Bytes
    /// Returns the underlying byte slice.
    pub fn bytes(&self) -> GoBytes {
        self.buf.clone()
    }

    // Go: parse/buffer/reader.go:Reader.Reset
    /// Resets the position of the read pointer to the beginning.
    pub fn reset(&mut self) {
        self.pos = 0;
    }

    // Go: parse/buffer/reader.go:Reader.Len
    /// Returns the length of the buffer.
    pub fn len(&self) -> usize {
        self.buf.len()
    }

    pub fn is_empty(&self) -> bool {
        self.buf.is_empty()
    }
}

impl GoReader for Reader {
    fn read(&mut self, p: &mut [u8]) -> (usize, Option<GoError>) {
        Reader::read(self, p)
    }
    fn bytes(&self) -> Option<GoBytes> {
        Some(self.buf.clone())
    }
}

impl std::io::Read for Reader {
    fn read(&mut self, b: &mut [u8]) -> std::io::Result<usize> {
        Ok(Reader::read(self, b).0)
    }
}
