//! Port of `encoding/json/jsontext/encode.go` (go1.27.1): the JSON encoder
//! state machine.
//!
//! The v1 API only ever uses buffered encoders (`getBufferedEncoder`,
//! `wr == nil`): output accumulates in `buf` and is never flushed, so the
//! writer, flushing and buffer statistics are not ported. Object-name
//! tracking (`Names`, `Namespaces`) is not ported either: it only feeds
//! JSON Pointers of marshal errors and duplicate-name checks, neither of
//! which the v1 API can observe.

use super::errors::SyntacticError;
use super::state::{INVALID_KIND, Kind, MAX_NESTING_DEPTH, State, normalize};
use super::token::Token;
use crate::goerr::Err;
use crate::jsonflags;
use crate::jsonopts::{Opt, Struct};
use crate::jsonwire;

// Go: encode.go:encoderState (+ encodeBuffer)
pub(crate) struct EncoderState {
    pub(crate) state: State,
    /// Buf is the output buffer.
    pub(crate) buf: Vec<u8>,
    /// baseOffset is added to len(buf) to obtain the absolute offset
    /// relative to the start of io.Writer stream.
    base_offset: i64,
    pub(crate) opts: Struct,
}

impl EncoderState {
    // Go: pools.go:getBufferedEncoder + encode.go:encoderState.reset
    pub(crate) fn new_buffered(opts: &[Opt]) -> EncoderState {
        let mut e = EncoderState {
            state: State::default(),
            buf: Vec::with_capacity(64),
            base_offset: 0,
            opts: Struct::default(),
        };
        e.state.reset();
        let mut opts2 = Struct::default(); // avoid mutating e.Struct in case it is part of opts
        opts2.join(opts);
        e.opts = opts2;
        if e.opts.flags.get(jsonflags::MULTILINE) {
            e.opts.initialize_multiline();
        }
        e
    }

    // Go: encode.go:encodeBuffer.offsetAt
    fn offset_at(&self, pos: usize) -> i64 {
        self.base_offset + pos as i64
    }

    // Go: encode.go:encodeBuffer.previousOffsetEnd
    fn previous_offset_end(&self) -> i64 {
        self.base_offset + self.buf.len() as i64
    }

    // Go: encode.go:Encoder.OutputOffset
    pub(crate) fn output_offset(&self) -> i64 {
        self.previous_offset_end()
    }

    // Go: encode.go:encoderState.WriteToken
    /// WriteToken writes the next token and advances the internal write offset.
    pub(crate) fn write_token(&mut self, t: &Token) -> Option<Err> {
        let k = t.kind();
        let orig_len = self.buf.len(); // Go uses a local copy of e.Buf to avoid mutating e in case of error
        let mut b = std::mem::take(&mut self.buf);
        self.state.tokens.may_append_delim(&mut b, k);
        if self.opts.flags.get(jsonflags::ANY_WHITESPACE) {
            self.append_whitespace(&mut b, k);
        }
        let pos = b.len(); // offset before the token

        let mut err: Option<Err>;
        match k {
            b'n' => {
                b.extend_from_slice(b"null");
                err = self.state.tokens.append_literal();
            }
            b'f' => {
                b.extend_from_slice(b"false");
                err = self.state.tokens.append_literal();
            }
            b't' => {
                b.extend_from_slice(b"true");
                err = self.state.tokens.append_literal();
            }
            b'"' => {
                err = t.append_string(&mut b, &self.opts.flags);
                if err.is_none() {
                    // (object names are not tracked; see the module docs)
                    err = self.state.tokens.append_string();
                }
            }
            b'0' => {
                err = t.append_number(&mut b, &self.opts.flags);
                if err.is_none() {
                    err = self.state.tokens.append_number();
                }
            }
            b'{' => {
                b.push(b'{');
                err = self.state.tokens.push_object();
                if err.is_none() {
                    self.opts.flags.clear(jsonflags::TAG_FLAGS); // tags only apply to current depth
                }
            }
            b'}' => {
                b.push(b'}');
                err = self.state.tokens.pop_object();
            }
            b'[' => {
                b.push(b'[');
                err = self.state.tokens.push_array();
                self.opts.flags.clear(jsonflags::TAG_FLAGS); // tags only apply to current depth
            }
            b']' => {
                b.push(b']');
                err = self.state.tokens.pop_array();
            }
            _ => err = Some(Err::InvalidToken),
        }
        if let Some(err) = err {
            b.truncate(orig_len);
            self.buf = b;
            return Some(self.wrap_syntactic_error(err, pos, 1));
        }

        // Finish off the buffer and store it back into e.
        self.buf = b;
        None
    }

    // Go: encode.go:encoderState.AppendRaw
    /// AppendRaw appends either a raw string (without double quotes) or number.
    /// Specify safeASCII if the string output is guaranteed to be ASCII
    /// without any characters (including '<', '>', and '&') that need escaping,
    /// otherwise this will validate whether the string needs escaping.
    /// The appended bytes for a JSON number must be valid.
    ///
    /// This is a specialized implementation of Encoder.WriteValue
    /// that allows appending directly into the buffer.
    /// It is only called from marshal logic in the "json" package.
    pub(crate) fn append_raw(
        &mut self,
        k: Kind,
        safe_ascii: bool,
        append_fn: &mut dyn FnMut(&mut Vec<u8>) -> Option<Err>,
    ) -> Option<Err> {
        let orig_len = self.buf.len(); // Go uses a local copy of e.Buf to avoid mutating e in case of error
        let mut b = std::mem::take(&mut self.buf);
        self.state.tokens.may_append_delim(&mut b, k);
        if self.opts.flags.get(jsonflags::ANY_WHITESPACE) {
            self.append_whitespace(&mut b, k);
        }
        let pos = b.len(); // offset before the token

        let res = match k {
            b'"' => {
                b.push(b'"');
                if let Some(err) = append_fn(&mut b) {
                    b.truncate(orig_len);
                    self.buf = b;
                    return Some(err);
                }
                b.push(b'"');

                // Check whether we need to escape the string and if necessary
                // perform the escaping in a separate buffer.
                let is_verbatim = safe_ascii || !jsonwire::need_escape(&b[pos + 1..b.len() - 1]);
                let mut res = None;
                if !is_verbatim {
                    let b2 = b[pos + 1..b.len() - 1].to_vec();
                    b.truncate(pos);
                    if let Some(err) = jsonwire::append_quote(&mut b, &b2, &self.opts.flags) {
                        res = Some((err, pos));
                    }
                }
                if res.is_none() {
                    // (object names are not tracked; see the module docs)
                    if let Some(err) = self.state.tokens.append_string() {
                        res = Some((err, pos));
                    }
                }
                res
            }
            b'0' => {
                if let Some(err) = append_fn(&mut b) {
                    b.truncate(orig_len);
                    self.buf = b;
                    return Some(err);
                }
                self.state.tokens.append_number().map(|err| (err, pos))
            }
            _ => panic!("BUG: invalid kind"),
        };
        if let Some((err, pos)) = res {
            b.truncate(orig_len);
            self.buf = b;
            return Some(self.wrap_syntactic_error(err, pos, 1));
        }

        // Finish off the buffer and store it back into e.
        self.buf = b;
        None
    }

    // Go: encode.go:encoderState.WriteValue
    /// WriteValue writes the next raw value and advances the internal write offset.
    /// The input value is reformatted according to the encoder options.
    pub(crate) fn write_value(&mut self, v: &[u8]) -> Option<Err> {
        let k = value_kind(v);
        let orig_len = self.buf.len(); // Go uses a local copy of e.Buf to avoid mutating e in case of error
        let mut b = std::mem::take(&mut self.buf);
        self.state.tokens.may_append_delim(&mut b, k);
        if self.opts.flags.get(jsonflags::ANY_WHITESPACE) {
            self.append_whitespace(&mut b, k);
        }
        let pos = b.len(); // offset before the value

        // Append the value the output.
        let mut n = 0;
        n += jsonwire::consume_whitespace(&v[n..]);
        let depth = self.state.tokens.depth();
        let (m, err) = reformat_value(&self.opts, &mut b, &v[n..], depth);
        if let Some(err) = err {
            b.truncate(orig_len);
            self.buf = b;
            return Some(self.wrap_syntactic_error(err, pos + n + m, 1));
        }
        n += m;
        n += jsonwire::consume_whitespace(&v[n..]);
        if v.len() > n {
            let err = jsonwire::new_invalid_character_error(&v[n..], "after top-level value");
            b.truncate(orig_len);
            self.buf = b;
            return Some(self.wrap_syntactic_error(err, pos + n, 0));
        }

        // Append the kind to the state machine.
        let mut err = None;
        match k {
            b'n' | b'f' | b't' => err = self.state.tokens.append_literal(),
            b'"' => {
                // (object names are not tracked; see the module docs)
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
                    // (ReorderRawObjects is never set by the v1 API)
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
            b.truncate(orig_len);
            self.buf = b;
            return Some(self.wrap_syntactic_error(err, pos, 1));
        }

        // Finish off the buffer and store it back into e.
        self.buf = b;
        None
    }

    // Go: encode.go:encoderState.CountNextDelimWhitespace
    /// CountNextDelimWhitespace counts the number of bytes of delimiter and
    /// whitespace bytes assuming the upcoming token is a JSON value.
    /// This method is used for error reporting at the semantic layer.
    pub(crate) fn count_next_delim_whitespace(&self) -> usize {
        let mut n = 0;
        const NEXT: Kind = b'"'; // arbitrary kind as next JSON value
        let delim = self.state.tokens.need_delim(NEXT);
        if delim > 0 {
            n += 1; // len(",") | len(":")
        }
        if delim == b':' {
            if self.opts.flags.get(jsonflags::SPACE_AFTER_COLON) {
                n += 1;
            }
        } else {
            if delim == b',' && self.opts.flags.get(jsonflags::SPACE_AFTER_COMMA) {
                n += 1;
            }
            if self.opts.flags.get(jsonflags::MULTILINE) {
                let m = self.state.tokens.need_indent(NEXT);
                if m > 0 {
                    n += 1 + self.opts.indent_prefix.len() + (m - 1) * self.opts.indent.len();
                }
            }
        }
        n
    }

    // Go: encode.go:encoderState.appendWhitespace
    /// appendWhitespace appends whitespace that immediately precedes the next token.
    fn append_whitespace(&self, b: &mut Vec<u8>, next: Kind) {
        let delim = self.state.tokens.need_delim(next);
        if delim == b':' {
            if self.opts.flags.get(jsonflags::SPACE_AFTER_COLON) {
                b.push(b' ');
            }
        } else {
            if delim == b',' && self.opts.flags.get(jsonflags::SPACE_AFTER_COMMA) {
                b.push(b' ');
            }
            if self.opts.flags.get(jsonflags::MULTILINE) {
                append_indent(&self.opts, b, self.state.tokens.need_indent(next));
            }
        }
    }

    // Go: encode.go:Encoder.StackDepth
    pub(crate) fn stack_depth(&self) -> usize {
        self.state.tokens.depth() - 1
    }

    // Go: encode.go:Encoder.StackIndex
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

    // Go: errors.go:wrapSyntacticError (for an encoderState)
    pub(crate) fn wrap_syntactic_error(&self, err: Err, pos: usize, _where: i32) -> Err {
        if matches!(err, Err::Eof | Err::Io { .. }) {
            return err;
        }
        let mut offset = self.offset_at(pos);
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

// Go: value.go:Value.Kind
/// Kind returns the starting token kind.
/// For a valid value, this will never include '}' or ']'.
pub(crate) fn value_kind(v: &[u8]) -> Kind {
    let v = &v[jsonwire::consume_whitespace(v)..];
    if !v.is_empty() {
        return normalize(v[0]);
    }
    INVALID_KIND
}

// Go: encode.go:encoderState.AppendIndent
/// AppendIndent appends the appropriate number of indentation characters
/// for the current nested level, n.
pub(crate) fn append_indent(opts: &Struct, b: &mut Vec<u8>, n: usize) {
    if n == 0 {
        return;
    }
    b.push(b'\n');
    b.extend_from_slice(&opts.indent_prefix);
    let mut n = n;
    while n > 1 {
        b.extend_from_slice(&opts.indent);
        n -= 1;
    }
}

// Go: encode.go:encoderState.reformatValue
/// reformatValue parses a JSON value from the start of src and
/// appends it to the end of dst, reformatting whitespace and strings as needed.
/// It returns the extended dst buffer and the number of consumed input bytes.
fn reformat_value(
    opts: &Struct,
    dst: &mut Vec<u8>,
    src: &[u8],
    depth: usize,
) -> (usize, Option<Err>) {
    // TODO: Should this update ValueFlags as input?
    if src.is_empty() {
        return (0, Some(Err::UnexpectedEof));
    }
    match normalize(src[0]) {
        b'n' => {
            if jsonwire::consume_null(src) == 0 {
                return jsonwire::consume_literal(src, "null");
            }
            dst.extend_from_slice(b"null");
            (4, None)
        }
        b'f' => {
            if jsonwire::consume_false(src) == 0 {
                return jsonwire::consume_literal(src, "false");
            }
            dst.extend_from_slice(b"false");
            (5, None)
        }
        b't' => {
            if jsonwire::consume_true(src) == 0 {
                return jsonwire::consume_literal(src, "true");
            }
            dst.extend_from_slice(b"true");
            (4, None)
        }
        b'"' => {
            let n = jsonwire::consume_simple_string(src);
            if n > 0 {
                dst.extend_from_slice(&src[..n]); // copy simple strings verbatim
                return (n, None);
            }
            jsonwire::reformat_string(dst, src, &opts.flags)
        }
        b'0' => {
            let n = jsonwire::consume_simple_number(src);
            if n > 0 && !opts.flags.get(jsonflags::CANONICALIZE_NUMBERS) {
                dst.extend_from_slice(&src[..n]); // copy simple numbers verbatim
                return (n, None);
            }
            jsonwire::reformat_number(dst, src, &opts.flags)
        }
        b'{' => crate::stack::guard(|| reformat_object(opts, dst, src, depth)),
        b'[' => crate::stack::guard(|| reformat_array(opts, dst, src, depth)),
        _ => (
            0,
            Some(jsonwire::new_invalid_character_error(
                src,
                "at start of value",
            )),
        ),
    }
}

// Go: encode.go:encoderState.reformatObject
/// reformatObject parses a JSON object from the start of src and
/// appends it to the end of src, reformatting whitespace and strings as needed.
/// It returns the extended dst buffer and the number of consumed input bytes.
fn reformat_object(
    opts: &Struct,
    dst: &mut Vec<u8>,
    src: &[u8],
    depth: usize,
) -> (usize, Option<Err>) {
    // Append object start.
    if src.is_empty() || src[0] != b'{' {
        panic!("BUG: reformatObject must be called with a buffer that starts with '{{'");
    } else if depth == MAX_NESTING_DEPTH + 1 {
        return (0, Some(Err::MaxDepth));
    }
    dst.push(b'{');
    let mut n = 1;

    // Append (possible) object end.
    n += jsonwire::consume_whitespace(&src[n..]);
    if src.len() <= n {
        return (n, Some(Err::UnexpectedEof));
    }
    if src[n] == b'}' {
        dst.push(b'}');
        n += 1;
        return (n, None);
    }

    // (duplicate-name checks are disabled under v1 options)
    let depth = depth + 1;
    loop {
        // Append optional newline and indentation.
        if opts.flags.get(jsonflags::MULTILINE) {
            append_indent(opts, dst, depth);
        }

        // Append object name.
        n += jsonwire::consume_whitespace(&src[n..]);
        if src.len() <= n {
            return (n, Some(Err::UnexpectedEof));
        }
        let mut m = jsonwire::consume_simple_string(&src[n..]);
        let is_verbatim = m > 0;
        if is_verbatim {
            dst.extend_from_slice(&src[n..n + m]);
        } else {
            let (mm, err) = jsonwire::reformat_string(dst, &src[n..], &opts.flags);
            m = mm;
            if let Some(err) = err {
                return (n + m, Some(err));
            }
        }
        n += m;

        // Append colon.
        n += jsonwire::consume_whitespace(&src[n..]);
        if src.len() <= n {
            return (n, Some(Err::UnexpectedEof));
        }
        if src[n] != b':' {
            let err = jsonwire::new_invalid_character_error(
                &src[n..],
                "after object name (expecting ':')",
            );
            return (n, Some(err));
        }
        dst.push(b':');
        n += 1;
        if opts.flags.get(jsonflags::SPACE_AFTER_COLON) {
            dst.push(b' ');
        }

        // Append object value.
        n += jsonwire::consume_whitespace(&src[n..]);
        if src.len() <= n {
            return (n, Some(Err::UnexpectedEof));
        }
        let (m, err) = reformat_value(opts, dst, &src[n..], depth);
        if let Some(err) = err {
            return (n + m, Some(err));
        }
        n += m;

        // Append comma or object end.
        n += jsonwire::consume_whitespace(&src[n..]);
        if src.len() <= n {
            return (n, Some(Err::UnexpectedEof));
        }
        match src[n] {
            b',' => {
                dst.push(b',');
                if opts.flags.get(jsonflags::SPACE_AFTER_COMMA) {
                    dst.push(b' ');
                }
                n += 1;
                continue;
            }
            b'}' => {
                if opts.flags.get(jsonflags::MULTILINE) {
                    append_indent(opts, dst, depth - 1);
                }
                dst.push(b'}');
                n += 1;
                return (n, None);
            }
            _ => {
                return (
                    n,
                    Some(jsonwire::new_invalid_character_error(
                        &src[n..],
                        "after object value (expecting ',' or '}')",
                    )),
                );
            }
        }
    }
}

// Go: encode.go:encoderState.reformatArray
/// reformatArray parses a JSON array from the start of src and
/// appends it to the end of dst, reformatting whitespace and strings as needed.
/// It returns the extended dst buffer and the number of consumed input bytes.
fn reformat_array(
    opts: &Struct,
    dst: &mut Vec<u8>,
    src: &[u8],
    depth: usize,
) -> (usize, Option<Err>) {
    // Append array start.
    if src.is_empty() || src[0] != b'[' {
        panic!("BUG: reformatArray must be called with a buffer that starts with '['");
    } else if depth == MAX_NESTING_DEPTH + 1 {
        return (0, Some(Err::MaxDepth));
    }
    dst.push(b'[');
    let mut n = 1;

    // Append (possible) array end.
    n += jsonwire::consume_whitespace(&src[n..]);
    if src.len() <= n {
        return (n, Some(Err::UnexpectedEof));
    }
    if src[n] == b']' {
        dst.push(b']');
        n += 1;
        return (n, None);
    }

    let depth = depth + 1;
    loop {
        // Append optional newline and indentation.
        if opts.flags.get(jsonflags::MULTILINE) {
            append_indent(opts, dst, depth);
        }

        // Append array value.
        n += jsonwire::consume_whitespace(&src[n..]);
        if src.len() <= n {
            return (n, Some(Err::UnexpectedEof));
        }
        let (m, err) = reformat_value(opts, dst, &src[n..], depth);
        if let Some(err) = err {
            return (n + m, Some(err));
        }
        n += m;

        // Append comma or array end.
        n += jsonwire::consume_whitespace(&src[n..]);
        if src.len() <= n {
            return (n, Some(Err::UnexpectedEof));
        }
        match src[n] {
            b',' => {
                dst.push(b',');
                if opts.flags.get(jsonflags::SPACE_AFTER_COMMA) {
                    dst.push(b' ');
                }
                n += 1;
                continue;
            }
            b']' => {
                if opts.flags.get(jsonflags::MULTILINE) {
                    append_indent(opts, dst, depth - 1);
                }
                dst.push(b']');
                n += 1;
                return (n, None);
            }
            _ => {
                return (
                    n,
                    Some(jsonwire::new_invalid_character_error(
                        &src[n..],
                        "after array value (expecting ',' or ']')",
                    )),
                );
            }
        }
    }
}
