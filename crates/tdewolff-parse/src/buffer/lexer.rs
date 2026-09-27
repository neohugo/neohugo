//! Go: parse/buffer/lexer.go (the older in-memory lexer buffer; new code uses
//! [`crate::Input`]).

use crate::error::GoError;
use crate::gobytes::GoBytes;
use crate::input::{GoReader, read_all};

/// Go: buffer.Lexer — a buffered reader that allows peeking forward and
/// shifting.
pub struct Lexer {
    buf: GoBytes,
    pos: isize,   // index in buf
    start: isize, // index in buf
    err: Option<GoError>,
    restore: Option<(GoBytes, usize, u8)>,
}

impl Lexer {
    // Go: parse/buffer/lexer.go:NewLexer
    /// Returns a new Lexer for a given reader (uses `Bytes()` when available).
    pub fn new(r: Option<&mut dyn GoReader>) -> Lexer {
        let mut b = GoBytes::nil();
        if let Some(r) = r {
            if let Some(bb) = r.bytes() {
                b = bb;
            } else {
                let (data, err) = read_all(r);
                if let Some(err) = err {
                    return Lexer {
                        buf: GoBytes::from_slice(&[0]),
                        pos: 0,
                        start: 0,
                        err: Some(err),
                        restore: None,
                    };
                }
                b = data;
            }
        }
        Lexer::new_bytes(b)
    }

    // Go: parse/buffer/lexer.go:NewLexerBytes
    /// Returns a new Lexer for a given byte slice, and appends NULL at the end.
    pub fn new_bytes(b: GoBytes) -> Lexer {
        let mut z = Lexer {
            buf: b.clone(),
            pos: 0,
            start: 0,
            err: None,
            restore: None,
        };
        let n = b.len();
        if n == 0 {
            z.buf = GoBytes::from_slice(&[0]); // nullBuffer
        } else if b.cap() > n {
            // Overwrite next byte but restore when done
            let b = b.slice(0, n + 1);
            let c = b.at(n);
            b.set(n, 0);
            z.buf = b.clone();
            z.restore = Some((b, n, c));
        } else {
            z.buf = b.append(&[0]);
        }
        z
    }

    // Go: parse/buffer/lexer.go:Restore
    /// Restores the replaced byte past the end of the buffer by NULL.
    pub fn restore(&mut self) {
        if let Some((b, n, c)) = self.restore.take() {
            b.set(n, c);
        }
    }

    // Go: parse/buffer/lexer.go:Err
    pub fn err(&self) -> Option<GoError> {
        self.peek_err(0)
    }

    // Go: parse/buffer/lexer.go:PeekErr
    pub fn peek_err(&self, pos: isize) -> Option<GoError> {
        if let Some(e) = &self.err {
            return Some(e.clone());
        } else if self.pos + pos >= self.buf.len() as isize - 1 {
            return Some(GoError::Eof);
        }
        None
    }

    // Go: parse/buffer/lexer.go:Peek
    pub fn peek(&self, pos: usize) -> u8 {
        let p = self.pos + pos as isize;
        assert!(p >= 0, "index out of range");
        self.buf.at(p as usize)
    }

    // Go: parse/buffer/lexer.go:PeekRune
    pub fn peek_rune(&self, pos: usize) -> (i32, usize) {
        // from unicode/utf8
        let c = self.peek(pos);
        if c < 0xC0 || self.peek(pos + 1) == 0 {
            (c as i32, 1)
        } else if c < 0xE0 || self.peek(pos + 2) == 0 {
            (
                (((c & 0x1F) as i32) << 6) | (self.peek(pos + 1) & 0x3F) as i32,
                2,
            )
        } else if c < 0xF0 || self.peek(pos + 3) == 0 {
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

    // Go: parse/buffer/lexer.go:Move
    pub fn move_(&mut self, n: isize) {
        self.pos += n;
    }

    // Go: parse/buffer/lexer.go:Pos
    pub fn pos(&self) -> usize {
        (self.pos - self.start) as usize
    }

    // Go: parse/buffer/lexer.go:Rewind
    pub fn rewind(&mut self, pos: usize) {
        self.pos = self.start + pos as isize;
    }

    // Go: parse/buffer/lexer.go:Lexeme
    pub fn lexeme(&self) -> GoBytes {
        self.buf
            .slice3(self.start as usize, self.pos as usize, self.pos as usize)
    }

    // Go: parse/buffer/lexer.go:Skip
    pub fn skip(&mut self) {
        self.start = self.pos;
    }

    // Go: parse/buffer/lexer.go:Shift
    pub fn shift(&mut self) -> GoBytes {
        let b = self.lexeme();
        self.start = self.pos;
        b
    }

    // Go: parse/buffer/lexer.go:Offset
    pub fn offset(&self) -> usize {
        self.pos as usize
    }

    // Go: parse/buffer/lexer.go:Bytes
    pub fn bytes(&self) -> GoBytes {
        let n = self.buf.len() - 1;
        self.buf.slice3(0, n, n)
    }

    // Go: parse/buffer/lexer.go:Reset
    pub fn reset(&mut self) {
        self.start = 0;
        self.pos = 0;
    }
}
