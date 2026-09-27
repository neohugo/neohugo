//! Port of `encoding/json/jsontext/decode.go` (go1.27.1): the streaming
//! JSON decoder (`decoderState` + `decodeBuffer`).
//!
//! Deviations (none change results):
//! - `consumeValue`/`consumeObject`/`consumeArray` are one iterative
//!   function with an explicit frame stack instead of mutual recursion, so
//!   10000-deep input cannot overflow the Rust stack.
//! - Duplicate-name namespaces and `pointerSuffixError` are not ported
//!   (see `state.rs` and `errors.rs`).
//! - `bytes.Buffer` readers are not special-cased (the v1 `NewDecoder` hides
//!   them from jsontext anyway).
//! - Go's `cap(d.buf)` is tracked in `buf_cap`, since the growth policy
//!   decides how much is read ahead (visible through `Buffered`).

use super::errors::SyntacticError;
use super::state::{Kind, MAX_NESTING_DEPTH, State, normalize};
use super::token::Token;
use crate::goerr::Err;
use crate::jsonflags;
use crate::jsonopts::{Opt, Struct};
use crate::jsonwire::{self, ValueFlags};

/// Go: `invalidateBufferByte` — invalid starting character for JSON grammar.
pub(crate) const INVALIDATE_BUFFER_BYTE: u8 = b'#';

/// Go `io.Reader` as seen by the decoder. `Ok(0)` is `io.EOF`.
pub(crate) trait Source {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, String>;
}

/// The nil reader of buffered decoders (Go `d.rd == nil`).
pub(crate) struct NoReader;

impl Source for NoReader {
    fn read(&mut self, _buf: &mut [u8]) -> Result<usize, String> {
        unreachable!("buffered decoder has no reader")
    }
}

/// Adapts a `std::io::Read`.
pub(crate) struct IoSource<R>(pub(crate) R);

impl<R: std::io::Read> Source for IoSource<R> {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, String> {
        loop {
            match self.0.read(buf) {
                Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
                Err(e) => return Err(e.to_string()),
                Ok(n) => return Ok(n),
            }
        }
    }
}

// Go: decode.go:decoderState (+ decodeBuffer)
pub(crate) struct DecoderState<R> {
    pub(crate) state: State,

    // decodeBuffer:
    /// peekPos is non-zero if valid offset into buf for start of next token
    /// (-1 when peekErr is set).
    pub(crate) peek_pos: isize,
    /// peekErr implies peekPos is -1
    pub(crate) peek_err: Option<Err>,
    /// buf is the buffer of the input data (Go `buf[:len]`).
    pub(crate) buf: Vec<u8>,
    /// Go `cap(buf)`.
    buf_cap: usize,
    /// prevStart and prevEnd represent the raw value read previously
    /// (for ReadValue), which may be invalidated upon the next read.
    pub(crate) prev_start: usize,
    pub(crate) prev_end: usize,
    /// baseOffset is added to prevStart and prevEnd to obtain
    /// the absolute offset relative to the start of io.Reader stream.
    pub(crate) base_offset: i64,
    rd: Option<R>,
    /// Scratch space the reader fills (see fetch).
    read_scratch: Vec<u8>,

    pub(crate) opts: Struct,
}

impl DecoderState<NoReader> {
    // Go: pools.go:getBufferedDecoder
    /// A decoder over a complete input (Go `getBufferedDecoder(b, opts...)`).
    pub(crate) fn new_buffered(b: &[u8], opts: &[Opt]) -> DecoderState<NoReader> {
        let mut d = DecoderState::empty();
        d.reset(b.to_vec(), None, opts);
        d
    }
}

impl<R: Source> DecoderState<R> {
    fn empty() -> DecoderState<R> {
        DecoderState {
            state: State::default(),
            peek_pos: 0,
            peek_err: None,
            buf: Vec::new(),
            buf_cap: 0,
            prev_start: 0,
            prev_end: 0,
            base_offset: 0,
            rd: None,
            read_scratch: Vec::new(),
            opts: Struct::default(),
        }
    }

    // Go: decode.go:NewDecoder
    /// NewDecoder constructs a new streaming decoder reading from r.
    pub(crate) fn new_streaming(r: R, opts: &[Opt]) -> DecoderState<R> {
        let mut d = DecoderState::empty();
        d.reset(Vec::new(), Some(r), opts);
        d
    }

    // Go: decode.go:decoderState.reset
    fn reset(&mut self, b: Vec<u8>, r: Option<R>, opts: &[Opt]) {
        self.state.reset();
        self.peek_pos = 0;
        self.peek_err = None;
        self.buf_cap = b.len();
        self.buf = b;
        self.prev_start = 0;
        self.prev_end = 0;
        self.base_offset = 0;
        self.rd = r;
        let mut opts2 = Struct::default(); // avoid mutating d.Struct in case it is part of opts
        opts2.join(opts);
        self.opts = opts2;
    }

    // Go: decode.go:decoderState.fetch
    /// fetch reads at least 1 byte from the underlying io.Reader.
    /// It returns io.ErrUnexpectedEOF if zero bytes were read and io.EOF was seen.
    fn fetch(&mut self) -> Option<Err> {
        if self.rd.is_none() {
            return Some(Err::UnexpectedEof);
        }

        // Inform objectNameStack that we are about to fetch new buffer content.
        self.state.names.copy_quoted_buffer(&mut self.buf);

        // Allocate initial buffer if empty.
        if self.buf_cap == 0 {
            self.buf = Vec::with_capacity(64);
            self.buf_cap = 64;
        }

        // Check whether to grow the buffer.
        const MAX_BUFFER_SIZE: usize = 4 << 10;
        const GROWTH_SIZE_FACTOR: usize = 2; // higher value is faster
        const GROWTH_RATE_FACTOR: i64 = 2; // higher value is slower
        // By default, grow if below the maximum buffer size.
        let mut grow = self.buf_cap <= MAX_BUFFER_SIZE / GROWTH_SIZE_FACTOR;
        // Growing can be expensive, so only grow
        // if a sufficient number of bytes have been processed.
        grow = grow && (self.buf_cap as i64) < self.previous_offset_end() / GROWTH_RATE_FACTOR;
        // If prevStart==0, then fetch was called in order to fetch more data
        // to finish consuming a large JSON value contiguously.
        // Grow if less than 25% of the remaining capacity is available.
        // Note that this may cause the input buffer to exceed maxBufferSize.
        grow = grow || (self.prev_start == 0 && self.buf.len() >= 3 * self.buf_cap / 4);

        if grow {
            // Allocate a new buffer and copy the contents of the old buffer over.
            // TODO: Provide a hard limit on the maximum internal buffer size?
            let new_cap = self.buf_cap * GROWTH_SIZE_FACTOR;
            let mut buf = Vec::with_capacity(new_cap);
            buf.extend_from_slice(&self.buf[self.prev_start..]);
            self.buf = buf;
            self.buf_cap = new_cap;
        } else {
            // Move unread portion of the data to the front.
            self.buf.drain(..self.prev_start);
        }
        self.base_offset += self.prev_start as i64;
        self.prev_end -= self.prev_start;
        self.prev_start = 0;

        // Read more data into the internal buffer.
        // (Go reads into d.buf[len(d.buf):cap(d.buf)]; the reader is handed a
        // scratch slice of that exact size, then the bytes are appended.)
        let need = self.buf_cap - self.buf.len();
        if self.read_scratch.len() < need {
            self.read_scratch.resize(need, 0);
        }
        let res = self
            .rd
            .as_mut()
            .expect("reader")
            .read(&mut self.read_scratch[..need]);
        match res {
            Ok(n) if n > 0 => {
                self.buf.extend_from_slice(&self.read_scratch[..n]);
                None // ignore errors if any bytes are read
            }
            Ok(_) => Some(Err::UnexpectedEof),
            Err(e) => Some(Err::Io {
                action: "read",
                err: e,
            }),
        }
    }

    // Go: decode.go:decodeBuffer.invalidatePreviousRead
    fn invalidate_previous_read(&mut self) {
        // Avoid mutating the buffer if d.rd is nil which implies that d.buf
        // is provided by the user code and may not expect mutations.
        if self.rd.is_some() && self.prev_start < self.prev_end && self.prev_start < self.buf.len()
        {
            self.buf[self.prev_start] = INVALIDATE_BUFFER_BYTE;
            self.prev_start = self.prev_end;
        }
    }

    // Go: decode.go:decodeBuffer.needMore
    /// needMore reports whether there are no more unread bytes.
    fn need_more(&self, pos: usize) -> bool {
        // NOTE: The arguments and logic are kept simple to keep this inlinable.
        pos == self.buf.len()
    }

    // Go: decode.go:decodeBuffer.offsetAt
    pub(crate) fn offset_at(&self, pos: usize) -> i64 {
        self.base_offset + pos as i64
    }

    // Go: decode.go:decodeBuffer.previousOffsetStart
    #[allow(dead_code)]
    pub(crate) fn previous_offset_start(&self) -> i64 {
        self.base_offset + self.prev_start as i64
    }

    // Go: decode.go:decodeBuffer.previousOffsetEnd
    pub(crate) fn previous_offset_end(&self) -> i64 {
        self.base_offset + self.prev_end as i64
    }

    // Go: decode.go:decodeBuffer.previousBuffer
    fn previous_buffer(&self) -> &[u8] {
        &self.buf[self.prev_start..self.prev_end]
    }

    // Go: decode.go:decodeBuffer.unreadBuffer
    pub(crate) fn unread_buffer(&self) -> &[u8] {
        &self.buf[self.prev_end..]
    }

    // Go: decode.go:decodeBuffer.PreviousTokenOrValue
    /// PreviousTokenOrValue returns the previously read token or value
    /// unless it has been invalidated by a call to PeekKind.
    /// If a token is just a delimiter, then this returns a 1-byte buffer.
    /// This method is used for error reporting at the semantic layer.
    pub(crate) fn previous_token_or_value(&self) -> Vec<u8> {
        let b = self.previous_buffer();
        // If peek was called, then the previous token or buffer is invalidated.
        if self.peek_pos > 0 || !b.is_empty() && b[0] == INVALIDATE_BUFFER_BYTE {
            return Vec::new();
        }
        // ReadToken does not preserve the buffer for null, bools, or delimiters.
        // Manually re-construct that buffer.
        if b.is_empty() {
            let b = &self.buf[..self.prev_end]; // entirety of the previous buffer
            for tok in [&b"null"[..], b"false", b"true", b"{", b"}", b"[", b"]"] {
                if b.len() >= tok.len() && &b[b.len() - tok.len()..] == tok {
                    return b[b.len() - tok.len()..].to_vec();
                }
            }
        }
        b.to_vec()
    }

    // Go: decode.go:decoderState.PeekKind
    /// PeekKind retrieves the next token kind, but does not advance the read offset.
    ///
    /// It returns 0 if an error occurs. Any such error is cached until
    /// the next read call and it is the caller's responsibility to eventually
    /// follow up a PeekKind call with a read call.
    pub(crate) fn peek_kind(&mut self) -> Kind {
        // Check whether we have a cached peek result.
        if self.peek_pos > 0 {
            return normalize(self.buf[self.peek_pos as usize]);
        }

        self.invalidate_previous_read();

        // Consume leading whitespace.
        let mut pos = self.prev_end;
        pos += jsonwire::consume_whitespace(&self.buf[pos..]);
        if self.need_more(pos) {
            let (p, err) = self.consume_whitespace(pos);
            pos = p;
            if let Some(mut err) = err {
                if err == Err::UnexpectedEof && self.state.tokens.depth() == 1 {
                    err = Err::Eof; // EOF possibly if no Tokens present after top-level value
                }
                self.peek_pos = -1;
                self.peek_err = Some(self.wrap_syntactic_error(err, pos, 0));
                return 0;
            }
        }

        // Consume colon or comma.
        let mut delim = 0u8;
        let c = self.buf[pos];
        if c == b':' || c == b',' {
            delim = c;
            pos += 1;
            pos += jsonwire::consume_whitespace(&self.buf[pos..]);
            if self.need_more(pos) {
                let (p, err) = self.consume_whitespace(pos);
                pos = p;
                if let Some(err) = err {
                    let err = self.wrap_syntactic_error(err, pos, 0);
                    self.peek_pos = -1;
                    self.peek_err = Some(self.check_delim_before_io_error(delim, err));
                    return 0;
                }
            }
        }
        let next = normalize(self.buf[pos]);
        if self.state.tokens.need_delim(next) != delim {
            self.peek_pos = -1;
            self.peek_err = Some(self.check_delim(delim, next));
            return 0;
        }

        // This may set peekPos to zero, which is indistinguishable from
        // the uninitialized state. While a small hit to performance, it is correct
        // since ReadValue and ReadToken will disregard the cached result and
        // recompute the next kind.
        self.peek_pos = pos as isize;
        self.peek_err = None;
        next
    }

    // Go: decode.go:decoderState.checkDelimBeforeIOError
    /// checkDelimBeforeIOError checks whether the delim is even valid
    /// before returning an IO error, which occurs after the delim.
    fn check_delim_before_io_error(&mut self, delim: u8, err: Err) -> Err {
        // Since an IO error occurred, we do not know what the next kind is.
        // However, knowing the next kind is necessary to validate
        // whether the current delim is at least potentially valid.
        // Since a JSON string is always valid as the next token,
        // conservatively assume that is the next kind for validation.
        const NEXT: Kind = b'"';
        if self.state.tokens.need_delim(NEXT) != delim {
            return self.check_delim(delim, NEXT);
        }
        err
    }

    // Go: decode.go:decoderState.CountNextDelimWhitespace
    /// CountNextDelimWhitespace counts the number of upcoming bytes of
    /// delimiter or whitespace characters.
    /// This method is used for error reporting at the semantic layer.
    pub(crate) fn count_next_delim_whitespace(&mut self) -> usize {
        self.peek_kind(); // populate unreadBuffer
        let u = self.unread_buffer();
        let trimmed = u
            .iter()
            .position(|&c| !matches!(c, b',' | b':' | b' ' | b'\n' | b'\r' | b'\t'))
            .unwrap_or(u.len());
        trimmed
    }

    // Go: decode.go:decoderState.checkDelim
    /// checkDelim checks whether delim is valid for the given next kind.
    fn check_delim(&mut self, delim: u8, next: Kind) -> Err {
        let mut where_ = "at start of value";
        let need = self.state.tokens.need_delim(next);
        if need == delim {
            // Go returns nil here; callers only call checkDelim on a mismatch.
            unreachable!("checkDelim called with a valid delimiter");
        } else if need == b':' {
            where_ = "after object name (expecting ':')";
        } else if need == b',' {
            if self.state.tokens.last.is_object() {
                where_ = "after object value (expecting ',' or '}')";
            } else {
                where_ = "after array element (expecting ',' or ']')";
            }
        }
        let mut pos = self.prev_end; // restore position to right after leading whitespace
        pos += jsonwire::consume_whitespace(&self.buf[pos..]);
        let err = jsonwire::new_invalid_character_error(&self.buf[pos..], where_);
        self.wrap_syntactic_error(err, pos, 0)
    }

    // Go: decode.go:decoderState.SkipValue
    /// SkipValue is semantically equivalent to calling ReadValue and discarding
    /// the result except that memory is not wasted trying to hold the entire result.
    pub(crate) fn skip_value(&mut self) -> Option<Err> {
        match self.peek_kind() {
            b'{' | b'[' => {
                // For JSON objects and arrays, keep skipping all tokens
                // until the depth matches the starting depth.
                let depth = self.state.tokens.depth();
                loop {
                    if let Err(err) = self.read_token() {
                        return Some(err);
                    }
                    if depth >= self.state.tokens.depth() {
                        return None;
                    }
                }
            }
            _ => {
                // Trying to skip a value when the next token is a '}' or ']'
                // will result in an error being returned here.
                let mut flags = ValueFlags::default();
                if let Err(err) = self.read_value(&mut flags) {
                    return Some(err);
                }
                None
            }
        }
    }

    // Go: decode.go:decoderState.SkipValueRemainder
    /// SkipValueRemainder skips the remainder of a value
    /// after reading a '{' or '[' token.
    pub(crate) fn skip_value_remainder(&mut self) -> Option<Err> {
        if self.state.tokens.depth() - 1 > 0 && self.state.tokens.last.length() == 0 {
            let n = self.state.tokens.depth();
            while self.state.tokens.depth() >= n {
                if let Err(err) = self.read_token() {
                    return Some(err);
                }
            }
        }
        None
    }

    // Go: decode.go:decoderState.SkipUntil
    /// SkipUntil skips all tokens until the state machine
    /// is at or past the specified depth and length.
    pub(crate) fn skip_until(&mut self, depth: usize, length: i64) -> Option<Err> {
        while self.state.tokens.depth() > depth
            || (self.state.tokens.depth() == depth && self.state.tokens.last.length() < length)
        {
            if let Err(err) = self.read_token() {
                return Some(err);
            }
        }
        None
    }

    /// Consumes leading whitespace and an optional delimiter before the next
    /// token or value, as the first half of Go's ReadToken and ReadValue do.
    /// Returns the position of the next token and its kind.
    fn advance_to_next(&mut self) -> Result<(usize, Kind), Err> {
        let pos = self.peek_pos;
        if pos != 0 {
            // Check whether there is a cached peek result.
            if let Some(err) = self.peek_err.take() {
                self.peek_pos = 0; // possibly a transient I/O error
                return Err(err);
            }
            let next = normalize(self.buf[pos as usize]);
            self.peek_pos = 0; // reset cache
            return Ok((pos as usize, next));
        }
        self.invalidate_previous_read();
        let mut pos = self.prev_end;

        // Consume leading whitespace.
        pos += jsonwire::consume_whitespace(&self.buf[pos..]);
        if self.need_more(pos) {
            let (p, err) = self.consume_whitespace(pos);
            pos = p;
            if let Some(mut err) = err {
                if err == Err::UnexpectedEof && self.state.tokens.depth() == 1 {
                    err = Err::Eof; // EOF possibly if no Tokens present after top-level value
                }
                return Err(self.wrap_syntactic_error(err, pos, 0));
            }
        }

        // Consume colon or comma.
        let mut delim = 0u8;
        let c = self.buf[pos];
        if c == b':' || c == b',' {
            delim = c;
            pos += 1;
            pos += jsonwire::consume_whitespace(&self.buf[pos..]);
            if self.need_more(pos) {
                let (p, err) = self.consume_whitespace(pos);
                pos = p;
                if let Some(err) = err {
                    let err = self.wrap_syntactic_error(err, pos, 0);
                    return Err(self.check_delim_before_io_error(delim, err));
                }
            }
        }
        let next = normalize(self.buf[pos]);
        if self.state.tokens.need_delim(next) != delim {
            return Err(self.check_delim(delim, next));
        }
        Ok((pos, next))
    }

    // Go: decode.go:decoderState.ReadToken
    /// ReadToken reads the next Token, advancing the read offset.
    /// It returns io.EOF if there are no more tokens.
    pub(crate) fn read_token(&mut self) -> Result<Token, Err> {
        let (mut pos, next) = self.advance_to_next()?;

        // Handle the next token.
        let mut n: usize;
        match next {
            b'n' | b'f' | b't' => {
                let (lit, tok): (&str, Token) = match next {
                    b'n' => ("null", Token::Null),
                    b'f' => ("false", Token::False),
                    _ => ("true", Token::True),
                };
                let consumed = match next {
                    b'n' => jsonwire::consume_null(&self.buf[pos..]),
                    b'f' => jsonwire::consume_false(&self.buf[pos..]),
                    _ => jsonwire::consume_true(&self.buf[pos..]),
                };
                if consumed == 0 {
                    let (p, err) = self.consume_literal(pos, lit);
                    pos = p;
                    if let Some(err) = err {
                        return Err(self.wrap_syntactic_error(err, pos, 1));
                    }
                } else {
                    pos += lit.len();
                }
                if let Some(err) = self.state.tokens.append_literal() {
                    return Err(self.wrap_syntactic_error(err, pos - lit.len(), 1)); // report position at start of literal
                }
                self.prev_start = pos;
                self.prev_end = pos;
                Ok(tok)
            }
            b'"' => {
                let mut flags = ValueFlags::default(); // TODO: Preserve this in Token?
                n = jsonwire::consume_simple_string(&self.buf[pos..]);
                if n == 0 {
                    let old_abs_pos = self.base_offset + pos as i64;
                    let (p, err) = self.consume_string(&mut flags, pos);
                    pos = p;
                    let new_abs_pos = self.base_offset + pos as i64;
                    n = (new_abs_pos - old_abs_pos) as usize;
                    if let Some(err) = err {
                        return Err(self.wrap_syntactic_error(err, pos, 1));
                    }
                } else {
                    pos += n;
                }
                if self.state.tokens.last.need_object_name() {
                    // (duplicate-name checks are disabled under v1 options)
                    self.state.names.replace_last_quoted_offset(pos - n); // only replace if insertQuoted succeeds
                }
                if let Some(err) = self.state.tokens.append_string() {
                    return Err(self.wrap_syntactic_error(err, pos - n, 1)); // report position at start of string
                }
                self.prev_start = pos - n;
                self.prev_end = pos;
                Ok(Token::Raw(self.buf[pos - n..pos].to_vec()))
            }
            b'0' => {
                // NOTE: Since JSON numbers are not self-terminating,
                // we need to make sure that the next byte is not part of a number.
                n = jsonwire::consume_simple_number(&self.buf[pos..]);
                if n == 0 || self.need_more(pos + n) {
                    let old_abs_pos = self.base_offset + pos as i64;
                    let (p, err) = self.consume_number(pos);
                    pos = p;
                    let new_abs_pos = self.base_offset + pos as i64;
                    n = (new_abs_pos - old_abs_pos) as usize;
                    if let Some(err) = err {
                        return Err(self.wrap_syntactic_error(err, pos, 1));
                    }
                } else {
                    pos += n;
                }
                if let Some(err) = self.state.tokens.append_number() {
                    return Err(self.wrap_syntactic_error(err, pos - n, 1)); // report position at start of number
                }
                self.prev_start = pos - n;
                self.prev_end = pos;
                Ok(Token::Raw(self.buf[pos - n..pos].to_vec()))
            }
            b'{' => {
                if let Some(err) = self.state.tokens.push_object() {
                    return Err(self.wrap_syntactic_error(err, pos, 1));
                }
                self.state.names.push();
                self.opts.flags.clear(jsonflags::TAG_FLAGS); // tags only apply to current depth
                pos += 1;
                self.prev_start = pos;
                self.prev_end = pos;
                Ok(Token::BeginObject)
            }
            b'}' => {
                if let Some(err) = self.state.tokens.pop_object() {
                    return Err(self.wrap_syntactic_error(err, pos, 1));
                }
                self.state.names.pop();
                pos += 1;
                self.prev_start = pos;
                self.prev_end = pos;
                Ok(Token::EndObject)
            }
            b'[' => {
                if let Some(err) = self.state.tokens.push_array() {
                    return Err(self.wrap_syntactic_error(err, pos, 1));
                }
                self.opts.flags.clear(jsonflags::TAG_FLAGS); // tags only apply to current depth
                pos += 1;
                self.prev_start = pos;
                self.prev_end = pos;
                Ok(Token::BeginArray)
            }
            b']' => {
                if let Some(err) = self.state.tokens.pop_array() {
                    return Err(self.wrap_syntactic_error(err, pos, 1));
                }
                pos += 1;
                self.prev_start = pos;
                self.prev_end = pos;
                Ok(Token::EndArray)
            }
            _ => {
                let err =
                    jsonwire::new_invalid_character_error(&self.buf[pos..], "at start of value");
                Err(self.wrap_syntactic_error(err, pos, 1))
            }
        }
    }

    // Go: decode.go:decoderState.ReadValue
    /// ReadValue returns the next raw JSON value, advancing the read offset
    /// (as the range of `buf` it occupies, valid until the next read).
    pub(crate) fn read_value(
        &mut self,
        flags: &mut ValueFlags,
    ) -> Result<std::ops::Range<usize>, Err> {
        let (pos, next) = self.advance_to_next()?;

        // Read the value.
        let old_abs_pos = self.base_offset + pos as i64;
        let depth = self.state.tokens.depth();
        let (pos, err) = self.consume_value(flags, pos, depth);
        let new_abs_pos = self.base_offset + pos as i64;
        let n = (new_abs_pos - old_abs_pos) as usize;
        if let Some(err) = err {
            return Err(self.wrap_syntactic_error(err, pos, 1));
        }
        let mut err: Option<Err> = None;
        match next {
            b'n' | b't' | b'f' => err = self.state.tokens.append_literal(),
            b'"' => {
                if self.state.tokens.last.need_object_name() {
                    // (duplicate-name checks are disabled under v1 options)
                    self.state.names.replace_last_quoted_offset(pos - n); // only replace if insertQuoted succeeds
                }
                err = self.state.tokens.append_string();
            }
            b'0' => err = self.state.tokens.append_number(),
            b'{' => {
                err = self.state.tokens.push_object();
                if err.is_none() {
                    if let Some(e) = self.state.tokens.pop_object() {
                        panic!(
                            "BUG: popObject should never fail immediately after pushObject: {}",
                            e
                        );
                    }
                }
            }
            b'[' => {
                err = self.state.tokens.push_array();
                if err.is_none() {
                    if let Some(e) = self.state.tokens.pop_array() {
                        panic!(
                            "BUG: popArray should never fail immediately after pushArray: {}",
                            e
                        );
                    }
                }
            }
            _ => {}
        }
        if let Some(err) = err {
            return Err(self.wrap_syntactic_error(err, pos - n, 1)); // report position at start of value
        }
        self.prev_end = pos;
        self.prev_start = pos - n;
        Ok(pos - n..pos)
    }

    // Go: decode.go:decoderState.CheckNextValue
    /// CheckNextValue checks whether the next value is syntactically valid,
    /// but does not advance the read offset.
    /// If last, it verifies that the stream cleanly terminates with io.EOF.
    pub(crate) fn check_next_value(&mut self, last: bool) -> Option<Err> {
        self.peek_kind(); // populates d.peekPos and d.peekErr
        let pos = self.peek_pos;
        let err = self.peek_err.take();
        self.peek_pos = 0;
        if err.is_some() {
            return err;
        }

        let mut flags = ValueFlags::default();
        let depth = self.state.tokens.depth();
        let (pos, err) = self.consume_value(&mut flags, pos as usize, depth);
        if let Some(err) = err {
            return Some(self.wrap_syntactic_error(err, pos, 1));
        } else if last {
            return self.check_eof_at(pos);
        }
        None
    }

    // Go: decode.go:decoderState.AtEOF
    /// AtEOF reports whether the decoder is already at EOF.
    #[allow(dead_code)]
    pub(crate) fn at_eof(&mut self) -> bool {
        let (_, err) = self.consume_whitespace(self.prev_end);
        err == Some(Err::UnexpectedEof)
    }

    // Go: decode.go:decoderState.CheckEOF
    /// CheckEOF verifies that the input has no more data.
    pub(crate) fn check_eof(&mut self) -> Option<Err> {
        self.check_eof_at(self.prev_end)
    }

    // Go: decode.go:decoderState.checkEOF
    fn check_eof_at(&mut self, pos: usize) -> Option<Err> {
        let (pos, err) = self.consume_whitespace(pos);
        match err {
            None => {
                let err = jsonwire::new_invalid_character_error(
                    &self.buf[pos..],
                    "after top-level value",
                );
                Some(self.wrap_syntactic_error(err, pos, 0))
            }
            Some(Err::UnexpectedEof) => None,
            Some(err) => Some(err),
        }
    }

    // Go: decode.go:decoderState.consumeWhitespace
    /// consumeWhitespace consumes all whitespace starting at d.buf[pos:].
    /// It returns the new position in d.buf immediately after the last whitespace.
    /// If it returns nil, there is guaranteed to at least be one unread byte.
    ///
    /// The following pattern is common in this implementation:
    ///
    /// ```text
    ///     pos += jsonwire.ConsumeWhitespace(d.buf[pos:])
    ///     if d.needMore(pos) {
    ///         if pos, err = d.consumeWhitespace(pos); err != nil {
    ///             return ...
    ///         }
    ///     }
    /// ```
    ///
    /// It is difficult to simplify this without sacrificing performance since
    /// consumeWhitespace must be inlined. The body of the if statement is
    /// executed only in rare situations where we need to fetch more data.
    /// Since fetching may return an error, we also need to check the error.
    fn consume_whitespace(&mut self, pos: usize) -> (usize, Option<Err>) {
        let mut pos = pos;
        loop {
            pos += jsonwire::consume_whitespace(&self.buf[pos..]);
            if self.need_more(pos) {
                let abs_pos = self.base_offset + pos as i64;
                let err = self.fetch(); // will mutate d.buf and invalidate pos
                pos = (abs_pos - self.base_offset) as usize;
                if err.is_some() {
                    return (pos, err);
                }
                continue;
            }
            return (pos, None);
        }
    }

    /// The whitespace-then-maybe-fetch pattern (see [`Self::consume_whitespace`]).
    fn skip_ws(&mut self, pos: usize) -> Result<usize, (usize, Err)> {
        let mut pos = pos + jsonwire::consume_whitespace(&self.buf[pos..]);
        if self.need_more(pos) {
            let (p, err) = self.consume_whitespace(pos);
            pos = p;
            if let Some(err) = err {
                return Err((pos, err));
            }
        }
        Ok(pos)
    }

    // Go: decode.go:decoderState.consumeValue (+ consumeObject, consumeArray)
    /// consumeValue consumes a single JSON value starting at d.buf[pos:].
    /// It returns the new position in d.buf immediately after the value.
    ///
    /// Go recurses through consumeObject and consumeArray; this port keeps
    /// the same statements in the same order but tracks the open objects
    /// and arrays in `frames`. Every error returns straight to the caller,
    /// as Go's unwinding does (the pointer suffixes it adds are not kept).
    fn consume_value(
        &mut self,
        flags: &mut ValueFlags,
        pos: usize,
        depth: usize,
    ) -> (usize, Option<Err>) {
        /// An object or array being consumed (Go's consumeObject/consumeArray frames).
        #[derive(Clone, Copy, PartialEq)]
        enum Frame {
            Object,
            Array,
        }
        let mut frames: Vec<Frame> = Vec::new();
        let mut pos = pos;
        let mut depth = depth;
        loop {
            // Go: consumeValue.
            let (mut p, err): (usize, Option<Err>) = 'value: loop {
                let n;
                let mut err: Option<Err> = None;
                let next = normalize(self.buf[pos]);
                match next {
                    b'n' | b'f' | b't' => {
                        let (lit, simple) = match next {
                            b'n' => ("null", jsonwire::consume_null(&self.buf[pos..])),
                            b'f' => ("false", jsonwire::consume_false(&self.buf[pos..])),
                            _ => ("true", jsonwire::consume_true(&self.buf[pos..])),
                        };
                        if simple == 0 {
                            let (m, e) = jsonwire::consume_literal(&self.buf[pos..], lit);
                            n = m;
                            err = e;
                        } else {
                            n = simple;
                        }
                    }
                    b'"' => {
                        n = jsonwire::consume_simple_string(&self.buf[pos..]);
                        if n == 0 {
                            break 'value self.consume_string(flags, pos);
                        }
                    }
                    b'0' => {
                        n = jsonwire::consume_simple_number(&self.buf[pos..]);
                        if n == 0 || self.need_more(pos + n) {
                            break 'value self.consume_number(pos);
                        }
                    }
                    b'{' => {
                        // Go: consumeObject (prologue).
                        if pos >= self.buf.len() || self.buf[pos] != b'{' {
                            panic!(
                                "BUG: consumeObject must be called with a buffer that starts with '{{'"
                            );
                        } else if depth == MAX_NESTING_DEPTH + 1 {
                            break 'value (pos, Some(Err::MaxDepth));
                        }
                        pos += 1;

                        // Handle zero-length objects.
                        match self.skip_ws(pos) {
                            Ok(p) => pos = p,
                            Err((p, e)) => break 'value (p, Some(e)),
                        }
                        if self.buf[pos] == b'}' {
                            pos += 1;
                            break 'value (pos, None);
                        }

                        depth += 1;
                        frames.push(Frame::Object);
                        match self.consume_object_member_start(flags, pos) {
                            Ok(p) => pos = p,
                            Err((p, e)) => return (p, Some(e)),
                        }
                        continue 'value; // consume the member value
                    }
                    b'[' => {
                        // Go: consumeArray (prologue).
                        if pos >= self.buf.len() || self.buf[pos] != b'[' {
                            panic!(
                                "BUG: consumeArray must be called with a buffer that starts with '['"
                            );
                        } else if depth == MAX_NESTING_DEPTH + 1 {
                            break 'value (pos, Some(Err::MaxDepth));
                        }
                        pos += 1;

                        // Handle zero-length arrays.
                        match self.skip_ws(pos) {
                            Ok(p) => pos = p,
                            Err((p, e)) => break 'value (p, Some(e)),
                        }
                        if self.buf[pos] == b']' {
                            pos += 1;
                            break 'value (pos, None);
                        }

                        depth += 1;
                        frames.push(Frame::Array);
                        // Consume the first array element.
                        match self.skip_ws(pos) {
                            Ok(p) => pos = p,
                            Err((p, e)) => return (p, Some(e)),
                        }
                        continue 'value;
                    }
                    _ => {
                        if (self.state.tokens.last.is_object() && next == b']')
                            || (self.state.tokens.last.is_array() && next == b'}')
                        {
                            break 'value (pos, Some(Err::MismatchDelim));
                        }
                        break 'value (
                            pos,
                            Some(jsonwire::new_invalid_character_error(
                                &self.buf[pos..],
                                "at start of value",
                            )),
                        );
                    }
                }
                if err == Some(Err::UnexpectedEof) {
                    let abs_pos = self.base_offset + pos as i64;
                    let ferr = self.fetch(); // will mutate d.buf and invalidate pos
                    pos = (abs_pos - self.base_offset) as usize;
                    if ferr.is_some() {
                        break 'value (pos + n, ferr);
                    }
                    continue;
                }
                break 'value (pos + n, err);
            };

            // An error unwinds every enclosing consumeObject/consumeArray.
            if err.is_some() {
                return (p, err);
            }

            // The value ended at p: continue the enclosing object or array.
            loop {
                match frames.last() {
                    None => return (p, None),
                    Some(Frame::Object) => {
                        // Go: consumeObject, after the member value.
                        match self.skip_ws(p) {
                            Ok(q) => p = q,
                            Err((q, e)) => return (q, Some(e)),
                        }
                        match self.buf[p] {
                            b',' => {
                                p += 1;
                                match self.consume_object_member_start(flags, p) {
                                    Ok(q) => pos = q,
                                    Err((q, e)) => return (q, Some(e)),
                                }
                                break; // consume the next member value
                            }
                            b'}' => {
                                p += 1;
                                frames.pop();
                                depth -= 1;
                                continue; // the object is the value that ended
                            }
                            _ => {
                                let e = jsonwire::new_invalid_character_error(
                                    &self.buf[p..],
                                    "after object value (expecting ',' or '}')",
                                );
                                return (p, Some(e));
                            }
                        }
                    }
                    Some(Frame::Array) => {
                        // Go: consumeArray, after an element.
                        match self.skip_ws(p) {
                            Ok(q) => p = q,
                            Err((q, e)) => return (q, Some(e)),
                        }
                        match self.buf[p] {
                            b',' => {
                                p += 1;
                                match self.skip_ws(p) {
                                    Ok(q) => pos = q,
                                    Err((q, e)) => return (q, Some(e)),
                                }
                                break; // consume the next element
                            }
                            b']' => {
                                p += 1;
                                frames.pop();
                                depth -= 1;
                                continue; // the array is the value that ended
                            }
                            _ => {
                                let e = jsonwire::new_invalid_character_error(
                                    &self.buf[p..],
                                    "after array element (expecting ',' or ']')",
                                );
                                return (p, Some(e));
                            }
                        }
                    }
                }
            }
        }
    }

    /// Go: the head of consumeObject's member loop: whitespace, the member
    /// name, whitespace, ':' and whitespace. Returns the position of the
    /// member value.
    fn consume_object_member_start(
        &mut self,
        flags: &mut ValueFlags,
        pos: usize,
    ) -> Result<usize, (usize, Err)> {
        // Handle before name.
        let mut pos = self.skip_ws(pos)?;

        // Handle name.
        let mut flags2 = ValueFlags::default();
        let n = jsonwire::consume_simple_string(&self.buf[pos..]);
        if n == 0 {
            let (p, err) = self.consume_string(&mut flags2, pos);
            pos = p;
            flags.join(flags2);
            if let Some(err) = err {
                return Err((pos, err));
            }
        } else {
            pos += n;
        }
        // (duplicate-name checks are disabled under v1 options)

        // Handle after name.
        pos = self.skip_ws(pos)?;
        if self.buf[pos] != b':' {
            let err = jsonwire::new_invalid_character_error(
                &self.buf[pos..],
                "after object name (expecting ':')",
            );
            return Err((pos, err));
        }
        pos += 1;

        // Handle before value.
        self.skip_ws(pos)
    }

    // Go: decode.go:decoderState.consumeLiteral
    /// consumeLiteral consumes a single JSON literal starting at d.buf[pos:].
    /// It returns the new position in d.buf immediately after the literal.
    fn consume_literal(&mut self, pos: usize, lit: &str) -> (usize, Option<Err>) {
        let mut pos = pos;
        loop {
            let (n, err) = jsonwire::consume_literal(&self.buf[pos..], lit);
            if err == Some(Err::UnexpectedEof) {
                let abs_pos = self.base_offset + pos as i64;
                let err = self.fetch(); // will mutate d.buf and invalidate pos
                pos = (abs_pos - self.base_offset) as usize;
                if err.is_some() {
                    return (pos + n, err);
                }
                continue;
            }
            return (pos + n, err);
        }
    }

    // Go: decode.go:decoderState.consumeString
    /// consumeString consumes a single JSON string starting at d.buf[pos:].
    /// It returns the new position in d.buf immediately after the string.
    fn consume_string(&mut self, flags: &mut ValueFlags, pos: usize) -> (usize, Option<Err>) {
        let mut pos = pos;
        let mut n = 0;
        loop {
            let (m, err) = jsonwire::consume_string_resumable(
                flags,
                &self.buf[pos..],
                n,
                !self.opts.flags.get(jsonflags::ALLOW_INVALID_UTF8),
            );
            n = m;
            if err == Some(Err::UnexpectedEof) {
                let abs_pos = self.base_offset + pos as i64;
                let err = self.fetch(); // will mutate d.buf and invalidate pos
                pos = (abs_pos - self.base_offset) as usize;
                if err.is_some() {
                    return (pos + n, err);
                }
                continue;
            }
            return (pos + n, err);
        }
    }

    // Go: decode.go:decoderState.consumeNumber
    /// consumeNumber consumes a single JSON number starting at d.buf[pos:].
    /// It returns the new position in d.buf immediately after the number.
    fn consume_number(&mut self, pos: usize) -> (usize, Option<Err>) {
        let mut pos = pos;
        let mut n = 0;
        let mut state = jsonwire::CONSUME_NUMBER_INIT;
        loop {
            let (m, st, err) = jsonwire::consume_number_resumable(&self.buf[pos..], n, state);
            n = m;
            state = st;
            // NOTE: Since JSON numbers are not self-terminating,
            // we need to make sure that the next byte is not part of a number.
            if err == Some(Err::UnexpectedEof) || self.need_more(pos + n) {
                let may_terminate = err.is_none();
                let abs_pos = self.base_offset + pos as i64;
                let err = self.fetch(); // will mutate d.buf and invalidate pos
                pos = (abs_pos - self.base_offset) as usize;
                if let Some(err) = err {
                    if may_terminate && err == Err::UnexpectedEof {
                        return (pos + n, None);
                    }
                    return (pos, Some(err));
                }
                continue;
            }
            return (pos + n, err);
        }
    }

    // Go: decode.go:Decoder.InputOffset
    /// InputOffset returns the current input byte offset. It gives the location
    /// of the next byte immediately after the most recently returned token or value.
    pub(crate) fn input_offset(&self) -> i64 {
        self.previous_offset_end()
    }

    // Go: decode.go:Decoder.StackIndex
    /// StackIndex returns information about the specified stack level.
    pub(crate) fn stack_index(&mut self, i: usize) -> (Kind, i64) {
        let s = *self.state.tokens.index(i);
        if i > 0 && s.is_object() {
            (b'{', s.length())
        } else if i > 0 && s.is_array() {
            (b'[', s.length())
        } else {
            (0, s.length())
        }
    }

    // Go: decode.go:Decoder.StackDepth
    pub(crate) fn stack_depth(&self) -> usize {
        self.state.tokens.depth() - 1
    }

    // Go: decode.go:decoderState.AppendStackPointer
    /// AppendStackPointer appends a JSON Pointer (RFC 6901) to the current value.
    pub(crate) fn append_stack_pointer(&mut self, b: &mut Vec<u8>, where_: i32) {
        self.state.names.copy_quoted_buffer(&mut self.buf);
        self.state.append_stack_pointer(b, where_);
    }

    // Go: errors.go:wrapSyntacticError (for a decoderState)
    /// wrapSyntacticError wraps an error and annotates it with a precise location
    /// using the provided encoder or decoder state.
    /// If err is an *ioError or io.EOF, then it is returned unmodified.
    pub(crate) fn wrap_syntactic_error(&mut self, err: Err, pos: usize, _where: i32) -> Err {
        if matches!(err, Err::Eof | Err::Io { .. }) {
            return err;
        }
        let mut offset = self.offset_at(pos);
        let mut err = err;
        if err == Err::MismatchDelim {
            let mut where_ = "at start of value";
            if !self.state.tokens.stack.is_empty() && self.state.tokens.last.length() > 0 {
                if self.state.tokens.last.is_array() {
                    where_ = "after array element (expecting ',' or ']')";
                } else if self.state.tokens.last.is_object() {
                    where_ = "after object value (expecting ',' or '}')";
                }
            }
            err = jsonwire::new_invalid_character_error(&self.buf[pos..], where_);
        }
        if self
            .opts
            .flags
            .get(jsonflags::REPORT_ERRORS_WITH_LEGACY_SEMANTICS)
            && err != Err::UnexpectedEof
        {
            if let Err::InvalidText(werr) = &err {
                offset += werr.what.len() as i64;
            } else {
                offset += 1;
            }
        }
        Err::Syntactic(Box::new(SyntacticError {
            byte_offset: offset,
            json_pointer: Vec::new(),
            err: Some(err),
        }))
    }
}
