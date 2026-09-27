//! Go: parse/input.go

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use crate::error::GoError;
use crate::gobytes::GoBytes;

/// A Go `io.Reader`, optionally also implementing `interface{ Bytes() []byte }`.
pub trait GoReader {
    /// `Read(p []byte) (n int, err error)`
    fn read(&mut self, p: &mut [u8]) -> (usize, Option<GoError>);
    /// `Bytes() []byte` when the reader implements it (e.g. `*bytes.Buffer`,
    /// `*buffer.Reader`), `None` otherwise.
    fn bytes(&self) -> Option<GoBytes> {
        None
    }
}

/// Adapts a Rust `std::io::Read` into a [`GoReader`] (no `Bytes()` method).
pub struct IoReader<R: std::io::Read>(pub R);

impl<R: std::io::Read> GoReader for IoReader<R> {
    fn read(&mut self, p: &mut [u8]) -> (usize, Option<GoError>) {
        if p.is_empty() {
            return (0, None);
        }
        match self.0.read(p) {
            Ok(0) => (0, Some(GoError::Eof)),
            Ok(n) => (n, None),
            Err(e) => (0, Some(GoError::Other(e.to_string().into_bytes()))),
        }
    }
}

// Go: io/io.go:ReadAll (go1.27.1)
/// Reads until EOF; returns the data and the first non-EOF error. Ported from
/// go1.27.1, which reads into exponentially growing chunks and copies them
/// into one right-sized slice at the end (so the capacity of the result is
/// Go's, too).
pub fn read_all(r: &mut dyn GoReader) -> (GoBytes, Option<GoError>) {
    // Build slices of exponentially growing size,
    // then copy into a perfectly-sized slice at the end.
    let mut b = GoBytes::make(0, 512);
    // Starting with next equal to 256 (instead of say 512 or 1024)
    // allows less memory usage for small inputs that finish in the
    // early growth stages, but we grow the read sizes quickly such that
    // it does not materially impact medium or large inputs.
    let mut next: usize = 256;
    let mut chunks: Vec<GoBytes> = Vec::with_capacity(4);
    // Invariant: finalSize = sum(len(c) for c in chunks)
    let mut final_size: usize = 0;
    loop {
        let n0 = b.len();
        let mut tmp = vec![0u8; b.cap() - n0]; // b[len(b):cap(b)]
        let (n, err) = r.read(&mut tmp);
        b = b.slice(0, n0 + n);
        b.slice(n0, n0 + n).copy_from_slice(&tmp[..n]);
        if let Some(e) = err {
            let err = if e == GoError::Eof { None } else { Some(e) };
            if chunks.is_empty() {
                return (b, err);
            }

            // Build our final right-sized slice.
            final_size += b.len();
            let mut fin = GoBytes::nil().append(&vec![0u8; final_size]).slice_to(0);
            for chunk in &chunks {
                fin = fin.append_bytes(chunk);
            }
            fin = fin.append_bytes(&b);
            return (fin, err);
        }

        if b.cap() - b.len() < b.cap() / 16 {
            // Move to the next intermediate slice.
            chunks.push(b.clone());
            final_size += b.len();
            b = GoBytes::nil().append(&vec![0u8; next]).slice_to(0);
            next += next / 2;
        }
    }
}

struct InputState {
    buf: GoBytes,
    pos: Cell<isize>,   // index in buf
    start: Cell<isize>, // index in buf
    err: Option<GoError>,
    /// `(b[:n+1], n, c)`: the byte `c` that was overwritten by the NULL.
    restore: RefCell<Option<(GoBytes, usize, u8)>>,
}

/// Go: parse/input.go:Input — a `*parse.Input`.
///
/// Cloning an `Input` clones the *pointer*: all clones share position and
/// buffer, like Go code sharing one `*parse.Input` between a lexer and its
/// caller.
#[derive(Clone)]
pub struct Input(Rc<InputState>);

impl Input {
    fn from_parts(
        buf: GoBytes,
        err: Option<GoError>,
        restore: Option<(GoBytes, usize, u8)>,
    ) -> Input {
        Input(Rc::new(InputState {
            buf,
            pos: Cell::new(0),
            start: Cell::new(0),
            err,
            restore: RefCell::new(restore),
        }))
    }

    // Go: parse/input.go:NewInput
    /// Returns a new Input for a given reader; uses `Bytes()` when the reader
    /// implements it, otherwise reads everything.
    pub fn new(r: Option<&mut dyn GoReader>) -> Input {
        let mut b = GoBytes::nil();
        if let Some(r) = r {
            if let Some(bb) = r.bytes() {
                b = bb;
            } else {
                let (data, err) = read_all(r);
                if let Some(err) = err {
                    return Input::from_parts(GoBytes::from_slice(&[0]), Some(err), None);
                }
                b = data;
            }
        }
        Input::new_bytes(b)
    }

    // Go: parse/input.go:NewInputString
    /// Returns a new Input for a given string (copied) and appends NULL.
    pub fn new_string(s: &[u8]) -> Input {
        Input::new_bytes(GoBytes::from_slice(s))
    }

    // Go: parse/input.go:NewInputBytes
    /// Returns a new Input for a given byte slice and appends NULL at the end.
    /// When `b` has spare capacity the byte after it is overwritten by NULL
    /// (shared with `b`) until [`Input::restore`] is called.
    pub fn new_bytes(b: GoBytes) -> Input {
        let n = b.len();
        if n == 0 {
            return Input::from_parts(GoBytes::from_slice(&[0]), None, None); // nullBuffer
        }
        if b.cap() > n {
            // Overwrite next byte but restore when done
            let b = b.slice(0, n + 1);
            let c = b.at(n);
            b.set(n, 0);
            let restore = (b.clone(), n, c);
            Input::from_parts(b, None, Some(restore))
        } else {
            Input::from_parts(b.append(&[0]), None, None)
        }
    }

    // Go: parse/input.go:Restore
    /// Restores the replaced byte past the end of the buffer by NULL.
    pub fn restore(&self) {
        if let Some((b, n, c)) = self.0.restore.borrow_mut().take() {
            b.set(n, c);
        }
    }

    // Go: parse/input.go:Err
    /// Returns the error returned from the reader or `io.EOF` when the end has
    /// been reached.
    pub fn err(&self) -> Option<GoError> {
        self.peek_err(0)
    }

    // Go: parse/input.go:PeekErr
    pub fn peek_err(&self, pos: isize) -> Option<GoError> {
        if let Some(e) = &self.0.err {
            return Some(e.clone());
        } else if self.0.buf.len() as isize - 1 <= self.0.pos.get() + pos {
            return Some(GoError::Eof);
        }
        None
    }

    /// `Err() != nil` without cloning the error.
    #[inline]
    pub fn has_err(&self) -> bool {
        self.0.err.is_some() || self.0.buf.len() as isize - 1 <= self.0.pos.get()
    }

    // Go: parse/input.go:Peek
    /// Returns the ith byte relative to the end position.
    #[inline]
    pub fn peek(&self, pos: usize) -> u8 {
        let p = self.0.pos.get() + pos as isize;
        assert!(p >= 0, "index out of range [{}]", p);
        self.0.buf.at(p as usize)
    }

    /// Go `Peek(pos)` for any (also negative) `pos`, as used by the js lexer
    /// (`l.r.Peek(-1)`); panics like Go when out of range.
    #[inline]
    pub fn peek_at(&self, pos: isize) -> u8 {
        let p = self.0.pos.get() + pos;
        assert!(p >= 0, "index out of range [{}]", p);
        self.0.buf.at(p as usize)
    }

    // Go: parse/input.go:PeekRune
    /// Returns the rune and rune length of the ith byte relative to the end
    /// position (no UTF-8 validation).
    pub fn peek_rune(&self, pos: usize) -> (i32, usize) {
        // from unicode/utf8
        let c = self.peek(pos);
        let rem = self.0.buf.len() as isize - 1 - self.0.pos.get();
        if c < 0xC0 || rem < 2 {
            (c as i32, 1)
        } else if c < 0xE0 || rem < 3 {
            (
                (((c & 0x1F) as i32) << 6) | (self.peek(pos + 1) & 0x3F) as i32,
                2,
            )
        } else if c < 0xF0 || rem < 4 {
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

    // Go: parse/input.go:Move
    /// Advances the position (may be negative).
    #[inline]
    pub fn move_(&self, n: isize) {
        self.0.pos.set(self.0.pos.get() + n);
    }

    // Go: parse/input.go:MoveRune
    /// Advances the position by the length of the current rune.
    pub fn move_rune(&self) {
        let c = self.peek(0);
        let rem = self.0.buf.len() as isize - 1 - self.0.pos.get();
        let n = if c < 0xC0 || rem < 2 {
            1
        } else if c < 0xE0 || rem < 3 {
            2
        } else if c < 0xF0 || rem < 4 {
            3
        } else {
            4
        };
        self.move_(n);
    }

    // Go: parse/input.go:Pos
    /// Returns a mark to which can be rewinded.
    #[inline]
    pub fn pos(&self) -> usize {
        (self.0.pos.get() - self.0.start.get()) as usize
    }

    // Go: parse/input.go:Rewind
    /// Rewinds the position to the given position.
    #[inline]
    pub fn rewind(&self, pos: usize) {
        self.0.pos.set(self.0.start.get() + pos as isize);
    }

    // Go: parse/input.go:Lexeme
    /// Returns the bytes of the current selection (`buf[start:pos:pos]`).
    pub fn lexeme(&self) -> GoBytes {
        let (s, p) = (self.0.start.get() as usize, self.0.pos.get() as usize);
        self.0.buf.slice3(s, p, p)
    }

    // Go: parse/input.go:Skip
    /// Collapses the position to the end of the selection.
    #[inline]
    pub fn skip(&self) {
        self.0.start.set(self.0.pos.get());
    }

    // Go: parse/input.go:Shift
    /// Returns the bytes of the current selection and collapses the position
    /// to the end of the selection.
    pub fn shift(&self) -> GoBytes {
        let b = self.lexeme();
        self.0.start.set(self.0.pos.get());
        b
    }

    // Go: parse/input.go:Offset
    /// Returns the character position in the buffer.
    #[inline]
    pub fn offset(&self) -> usize {
        self.0.pos.get() as usize
    }

    // Go: parse/input.go:Bytes
    /// Returns the underlying buffer (`buf[:len-1:len-1]`).
    pub fn bytes(&self) -> GoBytes {
        let n = self.0.buf.len() - 1;
        self.0.buf.slice3(0, n, n)
    }

    // Go: parse/input.go:Len
    /// Returns the length of the underlying buffer.
    pub fn len(&self) -> usize {
        self.0.buf.len() - 1
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    // Go: parse/input.go:Reset
    /// Resets position to the underlying buffer.
    pub fn reset(&self) {
        self.0.start.set(0);
        self.0.pos.set(0);
    }

    /// Whether two handles point at the same `*parse.Input`.
    pub fn ptr_eq(&self, other: &Input) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}
