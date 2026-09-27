//! Go: parse/error.go, plus a model of the Go `error` values this library
//! produces or passes through.

use std::fmt;

use crate::gobytes::GoBytes;
use crate::input::{GoReader, Input};
use crate::position::position;

/// A Go `error` value as produced or passed through by tdewolff/parse.
/// `Option<GoError>` models a Go `error` that may be nil.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GoError {
    /// `io.EOF`
    Eof,
    /// `*parse.Error`
    Parse(Box<Error>),
    /// `parse.ErrBadDataURI`
    BadDataUri,
    /// `base64.CorruptInputError(n)`
    Base64CorruptInput(i64),
    /// Any other error (e.g. from an `io.Reader`), by its `Error()` string.
    Other(Vec<u8>),
}

impl GoError {
    /// The exact bytes of Go's `err.Error()`.
    pub fn error_bytes(&self) -> Vec<u8> {
        match self {
            GoError::Eof => b"EOF".to_vec(),
            GoError::Parse(e) => e.error_bytes(),
            GoError::BadDataUri => b"not a data URI".to_vec(),
            GoError::Base64CorruptInput(n) => {
                format!("illegal base64 data at input byte {}", n).into_bytes()
            }
            GoError::Other(m) => m.clone(),
        }
    }

    pub fn is_eof(&self) -> bool {
        matches!(self, GoError::Eof)
    }
}

impl fmt::Display for GoError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&String::from_utf8_lossy(&self.error_bytes()))
    }
}

impl std::error::Error for GoError {}

/// Whether a Go `error` (nil = `None`) is `io.EOF`.
pub fn is_eof(err: &Option<GoError>) -> bool {
    matches!(err, Some(GoError::Eof))
}

/// Go: parse/error.go:Error — a parsing error with position information.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Error {
    /// Go `string` (arbitrary bytes).
    pub message: Vec<u8>,
    pub line: isize,
    pub column: isize,
    /// Go `string` (always valid UTF-8, built from runes).
    pub context: Vec<u8>,
}

// Go: parse/error.go:NewError
/// Creates a new error. Go's `fmt.Sprintf(message, a...)` formatting (only
/// applied when arguments are given) is done by the caller.
pub fn new_error(r: Option<&mut dyn GoReader>, offset: isize, message: Vec<u8>) -> Error {
    let (line, column, context) = position(r, offset);
    Error {
        message,
        line,
        column,
        context,
    }
}

// Go: parse/error.go:NewErrorLexer
/// Creates a new error from an active Lexer.
pub fn new_error_lexer(l: &Input, message: Vec<u8>) -> Error {
    let mut r = crate::buffer::Reader::new(l.bytes()); // bytes.NewBuffer(l.Bytes())
    let offset = l.offset() as isize;
    new_error(Some(&mut r), offset, message)
}

/// Convenience: `new_error_lexer` wrapped as a Go `error` value.
pub fn new_error_lexer_err(l: &Input, message: &str) -> GoError {
    GoError::Parse(Box::new(new_error_lexer(l, message.as_bytes().to_vec())))
}

impl Error {
    // Go: parse/error.go:Error.Position
    pub fn position(&self) -> (isize, isize, &[u8]) {
        (self.line, self.column, &self.context)
    }

    // Go: parse/error.go:Error.Error
    pub fn error_bytes(&self) -> Vec<u8> {
        let mut s = self.message.clone();
        s.extend_from_slice(
            format!(" on line {} and column {}\n", self.line, self.column).as_bytes(),
        );
        s.extend_from_slice(&self.context);
        s
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&String::from_utf8_lossy(&self.error_bytes()))
    }
}

impl std::error::Error for Error {}

impl From<Error> for GoError {
    fn from(e: Error) -> GoError {
        GoError::Parse(Box::new(e))
    }
}

/// Helper used by tests and downstream crates: `bytes.NewBuffer(b)` as a reader.
pub fn bytes_reader(b: GoBytes) -> crate::buffer::Reader {
    crate::buffer::Reader::new(b)
}
