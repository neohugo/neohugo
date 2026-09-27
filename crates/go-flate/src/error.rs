//! Go `error` values produced by `compress/flate` and `compress/zlib`.
//!
//! Go distinguishes errors by identity (`err == io.EOF`) or by dynamic type
//! (`CorruptInputError`). This enum carries every error the ported packages
//! can return, and `Display` reproduces Go's `Error()` strings exactly.

use std::fmt;
use std::sync::Arc;

/// An error returned by the flate / zlib writers and readers.
///
/// Errors are sticky in the Go code (stored in a struct field and returned
/// again by later calls), so the type is `Clone`; I/O errors from the
/// underlying reader/writer are shared through an `Arc`.
#[derive(Debug, Clone)]
pub enum Error {
    /// `io.EOF` ("EOF").
    Eof,
    /// `io.ErrUnexpectedEOF` ("unexpected EOF").
    UnexpectedEof,
    /// `flate.CorruptInputError(offset)`.
    CorruptInput(i64),
    /// `flate.InternalError(msg)`.
    Internal(String),
    /// `flate.errWriterClosed` ("flate: closed writer").
    WriterClosed,
    /// `flate.NewWriter` with a level outside [-2, 9].
    InvalidLevel(i64),
    /// `zlib.NewWriterLevelDict` with a level outside [-2, 9].
    ZlibInvalidLevel(i64),
    /// `zlib.ErrChecksum`.
    ZlibChecksum,
    /// `zlib.ErrDictionary`.
    ZlibDictionary,
    /// `zlib.ErrHeader`.
    ZlibHeader,
    /// An error returned by the underlying `io.Writer` / `io.Reader`.
    Io(Arc<std::io::Error>),
}

impl Error {
    pub(crate) fn io(e: std::io::Error) -> Error {
        Error::Io(Arc::new(e))
    }

    /// Reports whether `self` is `io.EOF`.
    pub fn is_eof(&self) -> bool {
        matches!(self, Error::Eof)
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Eof => f.write_str("EOF"),
            Error::UnexpectedEof => f.write_str("unexpected EOF"),
            // Go: "flate: corrupt input before offset " + strconv.FormatInt(int64(e), 10)
            Error::CorruptInput(off) => write!(f, "flate: corrupt input before offset {off}"),
            // Go: "flate: internal error: " + string(e)
            Error::Internal(msg) => write!(f, "flate: internal error: {msg}"),
            Error::WriterClosed => f.write_str("flate: closed writer"),
            Error::InvalidLevel(level) => write!(
                f,
                "flate: invalid compression level {level}: want value in range [-2, 9]"
            ),
            Error::ZlibInvalidLevel(level) => {
                write!(f, "zlib: invalid compression level: {level}")
            }
            Error::ZlibChecksum => f.write_str("zlib: invalid checksum"),
            Error::ZlibDictionary => f.write_str("zlib: invalid dictionary"),
            Error::ZlibHeader => f.write_str("zlib: invalid header"),
            Error::Io(e) => fmt::Display::fmt(e, f),
        }
    }
}

impl std::error::Error for Error {}

impl From<Error> for std::io::Error {
    fn from(e: Error) -> std::io::Error {
        match e {
            Error::Eof => std::io::Error::from(std::io::ErrorKind::UnexpectedEof),
            Error::UnexpectedEof => std::io::Error::from(std::io::ErrorKind::UnexpectedEof),
            Error::Io(inner) => match Arc::try_unwrap(inner) {
                Ok(e) => e,
                Err(shared) => std::io::Error::new(shared.kind(), shared.to_string()),
            },
            other => std::io::Error::other(other),
        }
    }
}
