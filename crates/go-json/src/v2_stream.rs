//! Port of `encoding/json/v2_stream.go` (go1.27.1): the v1 `Decoder`
//! (including the `Token` API) and `Encoder`, implemented with jsontext and
//! encoding/json/v2.

use std::io::{Read, Write};

use go_value::{GoString, Map, MapType, Value};

use crate::arshal::{self, Target};
use crate::error::{Error, UnmarshalTypeError};
use crate::goerr::Err;
use crate::jsonflags;
use crate::jsonopts::{Opt, Struct, bool_opt, default_options_v1};
use crate::jsontext::decode::{DecoderState, IoSource};
use crate::jsontext::encode::EncoderState;
use crate::jsonwire::ValueFlags;
use crate::v2_indent;
use crate::v2_inject::{transform_marshal_error, transform_unmarshal_error};
use crate::v2_scanner::{ERR_UNEXPECTED_END, transform_syntactic_error};

// Go: v2_stream.go:Decoder
/// A Decoder reads and decodes JSON values from an input stream.
pub struct Decoder<R> {
    dec: DecoderState<IoSource<R>>,
    opts: Struct,
    err: Option<Error>,

    /// hadPeeked reports whether [Decoder.More] was called.
    /// It is reset by [Decoder.Decode] and [Decoder.Token].
    had_peeked: bool,
    /// hadEOF reports whether [Decoder.Token] returned io.EOF.
    /// It is reset by [Decoder.Decode] and [Decoder.Token].
    had_eof: bool,
}

/// A JSON token from [`Decoder::token`]: a delimiter (`[ ] { }`, Go
/// `json.Delim`) or a value: bool, float64 ([`crate::Number`] with
/// `UseNumber`), string, or nil ([`Value::Invalid`]).
#[derive(Clone, Debug, PartialEq)]
pub enum Token {
    Delim(u8),
    Value(Value),
}

impl<R: Read> Decoder<R> {
    // Go: v2_stream.go:NewDecoder
    /// NewDecoder returns a new decoder that reads from r.
    ///
    /// The decoder introduces its own buffering and may
    /// read data from r beyond the JSON values requested.
    pub fn new(r: R) -> Decoder<R> {
        let opts = default_options_v1();
        let dec = DecoderState::new_streaming(IoSource(r), &[Opt::Struct(Box::new(opts.clone()))]);
        Decoder {
            dec,
            opts,
            err: None,
            had_peeked: false,
            had_eof: false,
        }
    }

    // Go: v2_stream.go:Decoder.UseNumber
    /// UseNumber causes the Decoder to unmarshal a number into an
    /// interface value as a [`crate::Number`] instead of as a float64.
    pub fn use_number(&mut self) {
        let (use_number, _) = self
            .opts
            .get_option(jsonflags::UNMARSHAL_ANY_WITH_RAW_NUMBER);
        if !use_number {
            self.opts
                .join(&[bool_opt(jsonflags::UNMARSHAL_ANY_WITH_RAW_NUMBER, true)]);
        }
    }

    // Go: v2_stream.go:Decoder.Decode (target `*any`)
    /// Decode reads the next JSON-encoded value from its input and returns
    /// it as Go's `interface{}` decoding would (see [`crate::unmarshal`]).
    /// At the end of the input it returns [`Error::Eof`].
    pub fn decode(&mut self) -> Result<Value, Error> {
        match self.decode_target(Target::Any) {
            (v, None) => Ok(v),
            (_, Some(err)) => Err(err),
        }
    }

    /// Decode into a `map[string]any` (see [`crate::unmarshal_map`]).
    pub fn decode_map(&mut self) -> Result<Value, Error> {
        match self.decode_target(Target::Map(Map::new(MapType::StringAny))) {
            (v, None) => Ok(v),
            (_, Some(err)) => Err(err),
        }
    }

    /// [`Decoder::decode`] returning what Go leaves in the target together
    /// with the error.
    pub fn decode_partial(&mut self) -> (Value, Option<Error>) {
        self.decode_target(Target::Any)
    }

    fn decode_target(&mut self, target: Target) -> (Value, Option<Error>) {
        let current = match &target {
            Target::Any => Value::Invalid,
            Target::Map(m) => Value::map(m.clone()),
        };
        if let Some(err) = &self.err {
            return (current, Some(err.clone()));
        }
        let mut flags = ValueFlags::default();
        let r = match self.dec.read_value(&mut flags) {
            Ok(r) => r,
            Err(err) => {
                let mut e = transform_syntactic_error(err);
                if e.to_string() == ERR_UNEXPECTED_END {
                    e = Error::UnexpectedEof;
                }
                self.err = Some(e.clone());
                return (current, Some(e));
            }
        };
        self.had_peeked = false;
        self.had_eof = false;
        let b = self.dec.buf[r].to_vec();
        // Go: jsonv2.Unmarshal(b, v, dec.opts)
        let (v, err) = arshal::unmarshal(&b, target, &[Opt::Struct(Box::new(self.opts.clone()))]);
        (v, err.map(|e| transform_unmarshal_error("", e)))
    }

    // Go: v2_stream.go:Decoder.Buffered
    /// Buffered returns the data remaining in the unread buffer,
    /// which may contain zero or more bytes.
    pub fn buffered(&self) -> &[u8] {
        self.dec.unread_buffer()
    }

    // Go: v2_stream.go:Decoder.Token
    /// Token returns the next JSON token in the input stream.
    /// At the end of the input stream, Token returns [`Error::Eof`].
    ///
    /// Token guarantees that the delimiters [ ] { } it returns are
    /// properly nested and matched: if Token encounters an unexpected
    /// delimiter in the input, it will return an error.
    ///
    /// The input stream consists of basic JSON values—bool, string,
    /// number, and null—along with delimiters [ ] { } of type [`Token::Delim`]
    /// to mark the start and end of arrays and objects.
    /// Commas and colons are elided.
    pub fn token(&mut self) -> Result<Token, Error> {
        if let Some(err) = &self.err {
            return Err(err.clone());
        }
        let tok = match self.dec.read_token() {
            Ok(tok) => tok,
            Err(err) => {
                // Historically, v1 would report just [io.EOF] if
                // the stream is a prefix of a valid JSON value.
                // It reports an unexpected EOF if in the middle
                // of a JSON value.
                if err.is(&Err::UnexpectedEof) && !matches!(err, Err::Io { .. }) {
                    let unread = self.dec.unread_buffer();
                    if unread
                        .iter()
                        .all(|&c| matches!(c, b' ' | b'\r' | b'\n' | b'\t' | b',' | b':'))
                    {
                        self.had_eof = true;
                        return Err(Error::Eof);
                    }
                    return Err(Error::UnexpectedEof);
                }
                return Err(transform_syntactic_error(err));
            }
        };
        self.had_peeked = false;
        self.had_eof = false;
        match tok.kind() {
            b'n' => Ok(Token::Value(Value::Invalid)),
            b'f' => Ok(Token::Value(Value::Bool(false))),
            b't' => Ok(Token::Value(Value::Bool(true))),
            b'"' => Ok(Token::Value(Value::String(GoString::from(
                tok.to_go_string(),
            )))),
            b'0' => {
                let (use_number, _) = self
                    .opts
                    .get_option(jsonflags::UNMARSHAL_ANY_WITH_RAW_NUMBER);
                if use_number {
                    return Ok(Token::Value(Value::object(crate::Number(GoString::from(
                        tok.to_go_string(),
                    )))));
                }
                let (v, ok) = tok.float_value();
                if !ok {
                    return Err(Error::UnmarshalType(UnmarshalTypeError {
                        value: format!("number {}", String::from_utf8_lossy(&tok.to_go_string())),
                        type_name: "float64".to_string(),
                        offset: self.input_offset(),
                        struct_name: String::new(),
                        field: String::new(),
                        err: None,
                    }));
                }
                Ok(Token::Value(Value::float64(v)))
            }
            k @ (b'{' | b'}' | b'[' | b']') => Ok(Token::Delim(k)),
            _ => panic!("unreachable"),
        }
    }

    // Go: v2_stream.go:Decoder.More
    /// More reports whether there is another element in the
    /// current array or object being parsed.
    pub fn more(&mut self) -> bool {
        self.had_peeked = true;
        let k = self.dec.peek_kind();
        if k == 0 {
            if self.err.is_none() {
                let err = match self.dec.read_token() {
                    Ok(_) => Err::Msg("json: successful read after failed peek".to_string()),
                    Err(err) => err,
                };
                self.err = Some(transform_syntactic_error(err));
            }
            return self.err != Some(Error::Eof);
        }
        k != b']' && k != b'}'
    }

    // Go: v2_stream.go:Decoder.InputOffset
    /// InputOffset returns the input stream byte offset of the current decoder position.
    /// The offset gives the location of the end of the most recently returned token
    /// and the beginning of the next token.
    pub fn input_offset(&self) -> i64 {
        let mut offset = self.dec.input_offset();
        if self.had_peeked || self.had_eof {
            // Historically, InputOffset reported the location of
            // the end of the most recently returned token
            // unless [Decoder.More] is called, in which case, it reported
            // the beginning of the next token.
            let unread = self.dec.unread_buffer();
            let lead = unread
                .iter()
                .position(|&c| !matches!(c, b' ' | b'\n' | b'\r' | b'\t'))
                .unwrap_or(unread.len());
            let trailing = &unread[lead..];
            if !trailing.is_empty() {
                offset += lead as i64;
                if self.had_eof && (trailing[0] == b',' || trailing[0] == b':') {
                    offset += 1;
                }
            }
        }
        offset
    }
}

// Go: v2_stream.go:Encoder
/// An Encoder writes JSON values to an output stream.
pub struct Encoder<W> {
    w: W,
    opts: Struct,
    err: Option<Error>,

    indent_prefix: Vec<u8>,
    indent_value: Vec<u8>,
}

impl<W: Write> Encoder<W> {
    // Go: v2_stream.go:NewEncoder
    /// NewEncoder returns a new encoder that writes to w.
    pub fn new(w: W) -> Encoder<W> {
        Encoder {
            w,
            opts: default_options_v1(),
            err: None,
            indent_prefix: Vec::new(),
            indent_value: Vec::new(),
        }
    }

    // Go: v2_stream.go:Encoder.Encode
    /// Encode writes the JSON encoding of v to the stream,
    /// with insignificant space characters elided,
    /// followed by a newline character.
    pub fn encode(&mut self, v: &Value) -> Result<(), Error> {
        if let Some(err) = &self.err {
            return Err(err.clone());
        }

        let mut e = EncoderState::new_buffered(&[Opt::Struct(Box::new(self.opts.clone()))]);
        // Go: jsonv2.MarshalEncode(e, v)
        if let Some(err) = arshal::marshal_encode(&mut e, v) {
            return Err(transform_marshal_error(err));
        }
        let mut b = e.buf;
        if self.indent_prefix.len() + self.indent_value.len() > 0 {
            let mut ib = Vec::new();
            v2_indent::indent(&mut ib, &b, &self.indent_prefix, &self.indent_value)?;
            b = ib;
        }
        b.push(b'\n');

        if let Err(err) = self.w.write_all(&b) {
            let err = Error::Io(err.to_string());
            self.err = Some(err.clone());
            return Err(err);
        }
        Ok(())
    }

    // Go: v2_stream.go:Encoder.SetIndent
    /// SetIndent instructs the encoder to format each subsequent encoded
    /// value as if indented by the package-level function Indent(dst, src, prefix, indent).
    /// Calling SetIndent("", "") disables indentation.
    pub fn set_indent(&mut self, prefix: impl AsRef<[u8]>, indent: impl AsRef<[u8]>) {
        self.indent_prefix = prefix.as_ref().to_vec();
        self.indent_value = indent.as_ref().to_vec();
    }

    // Go: v2_stream.go:Encoder.SetEscapeHTML
    /// SetEscapeHTML specifies whether problematic HTML characters
    /// should be escaped inside JSON quoted strings.
    /// The default behavior is to escape &, <, and > to \u0026, \u003c, and \u003e
    /// to avoid certain safety problems that can arise when embedding JSON in HTML.
    ///
    /// In non-HTML settings where the escaping interferes with the readability
    /// of the output, SetEscapeHTML(false) disables this behavior.
    pub fn set_escape_html(&mut self, on: bool) {
        let (escape, _) = self.opts.get_option(jsonflags::ESCAPE_FOR_HTML);
        if escape != on {
            self.opts.join(&[bool_opt(jsonflags::ESCAPE_FOR_HTML, on)]);
        }
    }

    /// The underlying writer.
    pub fn get_ref(&self) -> &W {
        &self.w
    }

    /// Consumes the encoder, returning the writer.
    pub fn into_inner(self) -> W {
        self.w
    }
}
