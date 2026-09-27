//! Go: parse/buffer/streamlexer.go

use crate::buffer::DEFAULT_BUF_SIZE;
use crate::error::GoError;
use crate::gobytes::GoBytes;
use crate::input::GoReader;

#[derive(Clone)]
struct Block {
    buf: GoBytes,
    next: usize, // index in pool plus one
    active: bool,
}

#[derive(Default)]
struct BufferPool {
    pool: Vec<Block>,
    head: usize, // index in pool plus one
    tail: usize, // index in pool plus one

    pos: isize, // byte pos in tail
}

impl BufferPool {
    // Go: parse/buffer/streamlexer.go:bufferPool.swap
    fn swap(&mut self, old_buf: GoBytes, size: usize) -> GoBytes {
        // find new buffer that can be reused
        let mut swap: isize = -1;
        for i in 0..self.pool.len() {
            if !self.pool[i].active && size <= self.pool[i].buf.cap() {
                swap = i as isize;
                break;
            }
        }
        if swap == -1 {
            // no free buffer found for reuse
            if self.tail == 0 && self.pos >= old_buf.len() as isize && size <= old_buf.cap() {
                // but we can reuse the current buffer!
                self.pos -= old_buf.len() as isize;
                return old_buf.slice_to(0);
            }
            // allocate new
            self.pool.push(Block {
                buf: GoBytes::make(0, size),
                next: 0,
                active: true,
            });
            swap = self.pool.len() as isize - 1;
        }
        let swap = swap as usize;

        let new_buf = self.pool[swap].buf.clone();

        // put current buffer into pool
        self.pool[swap] = Block {
            buf: old_buf,
            next: 0,
            active: true,
        };
        if self.head != 0 {
            self.pool[self.head - 1].next = swap + 1;
        }
        self.head = swap + 1;
        if self.tail == 0 {
            self.tail = swap + 1;
        }

        new_buf.slice_to(0)
    }

    // Go: parse/buffer/streamlexer.go:bufferPool.free
    fn free(&mut self, n: isize) {
        self.pos += n;
        // move the tail over to next buffers
        while self.tail != 0 && self.pos >= self.pool[self.tail - 1].buf.len() as isize {
            self.pos -= self.pool[self.tail - 1].buf.len() as isize;
            let new_tail = self.pool[self.tail - 1].next;
            self.pool[self.tail - 1].active = false; // after this, any thread may pick up the inactive buffer, so it can't be used anymore
            self.tail = new_tail;
        }
        if self.tail == 0 {
            self.head = 0;
        }
    }
}

/// Go: buffer.StreamLexer — a buffered reader that reads a limited amount at a
/// time, allowing to parse from streaming sources.
pub struct StreamLexer<'r> {
    r: Option<&'r mut dyn GoReader>,
    err: Option<GoError>,

    pool: BufferPool,

    buf: GoBytes,
    start: usize, // index in buf
    pos: usize,   // index in buf
    prev_start: usize,

    free: isize,
}

impl<'r> StreamLexer<'r> {
    // Go: parse/buffer/streamlexer.go:NewStreamLexer
    /// Returns a new StreamLexer with a 4kB estimated buffer size.
    pub fn new(r: &'r mut dyn GoReader) -> StreamLexer<'r> {
        StreamLexer::new_size(r, DEFAULT_BUF_SIZE)
    }

    // Go: parse/buffer/streamlexer.go:NewStreamLexerSize
    /// Returns a new StreamLexer with an estimated required buffer size. If the
    /// reader implements `Bytes()`, that buffer is used instead.
    pub fn new_size(r: &'r mut dyn GoReader, size: usize) -> StreamLexer<'r> {
        // if reader has the bytes in memory already, use that instead
        if let Some(b) = r.bytes() {
            return StreamLexer {
                r: None,
                err: Some(GoError::Eof),
                pool: BufferPool::default(),
                buf: b,
                start: 0,
                pos: 0,
                prev_start: 0,
                free: 0,
            };
        }
        StreamLexer {
            r: Some(r),
            err: None,
            pool: BufferPool::default(),
            buf: GoBytes::make(0, size),
            start: 0,
            pos: 0,
            prev_start: 0,
            free: 0,
        }
    }

    // Go: parse/buffer/streamlexer.go:StreamLexer.read
    fn read(&mut self, mut pos: usize) -> u8 {
        if self.err.is_some() {
            return 0;
        }

        // free unused bytes
        self.pool.free(self.free);
        self.free = 0;

        // get new buffer
        let mut c = self.buf.cap();
        let p = pos - self.start + 1;
        if 2 * p > c {
            // if the token is larger than half the buffer, increase buffer size
            c = 2 * c + p;
        }
        let mut d = self.buf.len() - self.start;
        let buf = self.pool.swap(self.buf.slice_to(self.start), c);
        buf.slice_to(d).copy_from(&self.buf.slice_from(self.start)); // copy the left-overs (unfinished token) from the old buffer

        // read in new data for the rest of the buffer
        while pos - self.start >= d && self.err.is_none() {
            let dst = buf.slice(d, buf.cap());
            let mut tmp = vec![0u8; dst.len()];
            let (n, err) = self.r.as_mut().expect("reader").read(&mut tmp);
            dst.copy_from_slice(&tmp[..n]);
            self.err = err;
            d += n;
        }
        pos -= self.start;
        self.pos -= self.start;
        self.start = 0;
        self.buf = buf.slice_to(d);
        if pos >= d {
            return 0;
        }
        self.buf.at(pos)
    }

    // Go: parse/buffer/streamlexer.go:StreamLexer.Err
    /// Returns the error returned from the reader; it may still return valid
    /// bytes for a while though.
    pub fn err(&self) -> Option<GoError> {
        if self.err == Some(GoError::Eof) && self.pos < self.buf.len() {
            return None;
        }
        self.err.clone()
    }

    // Go: parse/buffer/streamlexer.go:StreamLexer.Free
    /// Frees up bytes of length n from previously shifted tokens.
    pub fn free(&mut self, n: isize) {
        self.free += n;
    }

    // Go: parse/buffer/streamlexer.go:StreamLexer.Peek
    /// Returns the ith byte relative to the end position and possibly does an
    /// allocation. Returns zero when an error has occurred.
    pub fn peek(&mut self, pos: usize) -> u8 {
        let pos = pos + self.pos;
        if pos < self.buf.len() {
            return self.buf.at(pos);
        }
        self.read(pos)
    }

    // Go: parse/buffer/streamlexer.go:StreamLexer.PeekRune
    pub fn peek_rune(&mut self, pos: usize) -> (i32, usize) {
        // from unicode/utf8
        let c = self.peek(pos);
        if c < 0xC0 {
            (c as i32, 1)
        } else if c < 0xE0 {
            (
                (((c & 0x1F) as i32) << 6) | (self.peek(pos + 1) & 0x3F) as i32,
                2,
            )
        } else if c < 0xF0 {
            (
                (((c & 0x0F) as i32) << 12)
                    | (((self.peek(pos + 1) & 0x3F) as i32) << 6)
                    | (self.peek(pos + 2) & 0x3F) as i32,
                3,
            )
        } else {
            (
                (((c & 0x07) as i32) << 18)
                    | (((self.peek(pos + 1) & 0x3F) as i32) << 12)
                    | (((self.peek(pos + 2) & 0x3F) as i32) << 6)
                    | (self.peek(pos + 3) & 0x3F) as i32,
                4,
            )
        }
    }

    // Go: parse/buffer/streamlexer.go:StreamLexer.Move
    pub fn move_(&mut self, n: isize) {
        self.pos = (self.pos as isize + n) as usize;
    }

    // Go: parse/buffer/streamlexer.go:StreamLexer.Pos
    pub fn pos(&self) -> usize {
        self.pos - self.start
    }

    // Go: parse/buffer/streamlexer.go:StreamLexer.Rewind
    pub fn rewind(&mut self, pos: usize) {
        self.pos = self.start + pos;
    }

    // Go: parse/buffer/streamlexer.go:StreamLexer.Lexeme
    pub fn lexeme(&self) -> GoBytes {
        self.buf.slice(self.start, self.pos)
    }

    // Go: parse/buffer/streamlexer.go:StreamLexer.Skip
    pub fn skip(&mut self) {
        self.start = self.pos;
    }

    // Go: parse/buffer/streamlexer.go:StreamLexer.Shift
    /// Returns the bytes of the current selection and collapses the position.
    pub fn shift(&mut self) -> GoBytes {
        if self.pos > self.buf.len() {
            // make sure we peeked at least as much as we shift
            self.read(self.pos - 1);
        }
        let b = self.buf.slice(self.start, self.pos);
        self.start = self.pos;
        b
    }

    // Go: parse/buffer/streamlexer.go:StreamLexer.ShiftLen
    /// Returns the number of bytes moved since the last call to ShiftLen.
    /// (Like Go, this is negative after a buffer swap reset `start`.)
    pub fn shift_len(&mut self) -> isize {
        let n = self.start as isize - self.prev_start as isize;
        self.prev_start = self.start;
        n
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Go: parse/buffer/streamlexer_test.go:TestBufferPool
    #[test]
    fn buffer_pool() {
        let mut z = BufferPool::default();

        let lorem = GoBytes::from_slice(b"Lorem ipsum");
        let dolor = GoBytes::from_slice(b"dolor sit amet");
        let consectetur = GoBytes::from_slice(b"consectetur adipiscing elit");

        // set lorem as first buffer and get new dolor buffer
        let mut b = z.swap(lorem.clone(), dolor.len());
        assert!(b.len() == 0);
        assert!(b.cap() == dolor.len());
        b = b.append_bytes(&dolor);

        // free first buffer so it will be reused
        z.free(lorem.len() as isize);
        b = z.swap(b, lorem.len());
        b = b.slice_to(lorem.len());
        assert_eq!(b, lorem);

        b = z.swap(b, consectetur.len());
        b = b.append_bytes(&consectetur);

        // free in advance to reuse the same buffer
        z.free((dolor.len() + lorem.len() + consectetur.len()) as isize);
        assert!(z.head == 0);
        b = z.swap(b, consectetur.len());
        b = b.slice_to(consectetur.len());
        assert_eq!(b, consectetur);

        // free in advance but request larger buffer
        z.free(consectetur.len() as isize);
        b = z.swap(b, consectetur.len() + 1);
        b = b.append_bytes(&consectetur);
        b = b.append_byte(b'.');
        assert!(b.cap() == consectetur.len() + 1);
    }
}
