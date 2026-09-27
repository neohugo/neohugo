//! Go `error` values returned by `image/png`.
//!
//! Go distinguishes errors by identity (`err == io.EOF`) or by dynamic type
//! (`FormatError`, `UnsupportedError`). This enum carries every error the
//! ported package can return; `Display` reproduces Go's `Error()` strings.

use std::fmt;
use std::sync::Arc;

/// An error returned by [`crate::decode`], [`crate::decode_config`] or
/// [`crate::Encoder::encode`].
#[derive(Debug, Clone)]
pub enum Error {
    /// `png.FormatError(s)`: "png: invalid format: " + s.
    Format(String),
    /// `png.UnsupportedError(s)`: "png: unsupported feature: " + s.
    Unsupported(String),
    /// `io.EOF` ("EOF").
    Eof,
    /// `io.ErrUnexpectedEOF` ("unexpected EOF").
    UnexpectedEof,
    /// `io.ErrNoProgress`.
    NoProgress,
    /// An error from `compress/zlib` / `compress/flate` (e.g.
    /// "flate: corrupt input before offset 7", "zlib: invalid header").
    Flate(go_flate::Error),
    /// An error returned by the underlying `io.Reader` / `io.Writer`.
    Io(Arc<std::io::Error>),
}

impl Error {
    /// Go: `FormatError(s)`.
    pub(crate) fn format(s: impl Into<String>) -> Error {
        Error::Format(s.into())
    }

    /// Go: `UnsupportedError(s)`.
    pub(crate) fn unsupported(s: impl Into<String>) -> Error {
        Error::Unsupported(s.into())
    }

    /// Reports whether `self` is `io.EOF`.
    pub fn is_eof(&self) -> bool {
        matches!(self, Error::Eof)
    }

    /// Wraps an I/O error of the underlying reader or writer. A
    /// `std::io::Error` that carries a png [`Error`] (the way errors travel
    /// through the zlib reader in this port) is unwrapped.
    pub(crate) fn from_io(e: std::io::Error) -> Error {
        if e.get_ref().is_some_and(|inner| inner.is::<Error>()) {
            let inner = e.into_inner().expect("checked above");
            return *inner.downcast::<Error>().expect("checked above");
        }
        Error::Io(Arc::new(e))
    }

    /// Converts an error returned by the zlib reader/writer. Go returns the
    /// underlying reader's error values unchanged through `compress/flate`
    /// (`io.EOF`, `io.ErrUnexpectedEOF`, the png decoder's own errors).
    pub(crate) fn from_flate(e: go_flate::Error) -> Error {
        match e {
            go_flate::Error::Eof => Error::Eof,
            go_flate::Error::UnexpectedEof => Error::UnexpectedEof,
            go_flate::Error::Io(io) => {
                if io.get_ref().is_some_and(|inner| inner.is::<Error>()) {
                    let inner = io.get_ref().unwrap().downcast_ref::<Error>().unwrap();
                    return inner.clone();
                }
                Error::Io(io)
            }
            e => Error::Flate(e),
        }
    }

    /// Wraps `self` in a `std::io::Error` so it can travel through
    /// `std::io::Read` / `std::io::Write` interfaces (see [`Error::from_io`]).
    pub(crate) fn into_io(self) -> std::io::Error {
        match self {
            Error::Io(e) => match Arc::try_unwrap(e) {
                Ok(e) => e,
                Err(e) => std::io::Error::other(Error::Io(e)),
            },
            e => std::io::Error::other(e),
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            // Go: func (e FormatError) Error() string { return "png: invalid format: " + string(e) }
            Error::Format(s) => write!(f, "png: invalid format: {s}"),
            // Go: func (e UnsupportedError) Error() string { return "png: unsupported feature: " + string(e) }
            Error::Unsupported(s) => write!(f, "png: unsupported feature: {s}"),
            Error::Eof => f.write_str("EOF"),
            Error::UnexpectedEof => f.write_str("unexpected EOF"),
            Error::NoProgress => f.write_str("multiple Read calls return no data or error"),
            Error::Flate(e) => fmt::Display::fmt(e, f),
            Error::Io(e) => fmt::Display::fmt(e, f),
        }
    }
}

impl std::error::Error for Error {}

impl From<Error> for std::io::Error {
    fn from(e: Error) -> std::io::Error {
        e.into_io()
    }
}
