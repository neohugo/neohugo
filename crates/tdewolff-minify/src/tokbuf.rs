//! The look-ahead buffer shared by `html/buffer.go`, `svg/buffer.go` and
//! `xml/buffer.go` (the three Go files contain the identical `Peek`/`Shift`
//! algorithm over their own `Token` types).
//!
//! The Go buffer is a `[]Token` whose length and capacity both matter:
//! `Peek` reuses the backing array while `2*p <= cap`, and `Shift` past the
//! end reads into `buf[:1][0]` without changing the length. `buf` here holds
//! all `cap` slots and `len` is the Go length, so the emulation is exact.

pub(crate) struct GoTokenBuf<T> {
    /// All `cap(z.buf)` slots of the backing array.
    pub(crate) buf: Vec<T>,
    /// `len(z.buf)`
    pub(crate) len: usize,
    /// `z.pos`
    pub(crate) pos: usize,
}

impl<T: Default + Clone> GoTokenBuf<T> {
    /// `make([]Token, 0, 8)`
    pub(crate) fn new() -> Self {
        GoTokenBuf {
            buf: vec![T::default(); 8],
            len: 0,
            pos: 0,
        }
    }

    // Go: html/buffer.go:TokenBuffer.Peek (identical in svg/ and xml/)
    /// Returns the index of the ith element and possibly does an allocation.
    pub(crate) fn peek(
        &mut self,
        pos: usize,
        is_error: impl Fn(&T) -> bool,
        mut read: impl FnMut(&mut T),
    ) -> usize {
        let mut pos = pos + self.pos;
        if pos >= self.len {
            if self.len > 0 && is_error(&self.buf[self.len - 1]) {
                return self.len - 1;
            }

            let c = self.buf.len();
            let d = self.len - self.pos;
            let p = pos - self.pos + 1; // required peek length
            if 2 * p > c {
                let mut buf = vec![T::default(); 2 * c + p];
                buf[..d].clone_from_slice(&self.buf[self.pos..self.len]);
                self.buf = buf;
            } else {
                // copy(buf[:d], z.buf[z.pos:]) within the same array (forward memmove)
                for k in 0..d {
                    let t = self.buf[self.pos + k].clone();
                    self.buf[k] = t;
                }
            }

            let mut len = p;
            pos -= self.pos;
            for i in d..p {
                read(&mut self.buf[i]);
                if is_error(&self.buf[i]) {
                    len = i + 1;
                    pos = i;
                    break;
                }
            }
            self.pos = 0;
            self.len = len;
        }
        pos
    }

    // Go: html/buffer.go:TokenBuffer.Shift (identical in svg/ and xml/)
    /// Returns the index of the first element and advances position.
    pub(crate) fn shift(&mut self, read: impl FnOnce(&mut T)) -> usize {
        if self.pos >= self.len {
            read(&mut self.buf[0]);
            return 0;
        }
        let t = self.pos;
        self.pos += 1;
        t
    }
}
