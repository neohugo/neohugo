//! The error types of the `encoding/json` v1 API (go1.27.1, jsonv2-backed:
//! `v2_scanner.go`, `v2_decode.go`, `v2_encode.go`), as one public enum
//! whose `Display` is Go's `Error()` text.

use std::fmt;

use crate::goerr::Err;

// Go: json/v2_scanner.go:SyntaxError
/// A SyntaxError is a description of a JSON syntax error.
/// [`crate::unmarshal`] will return a SyntaxError if the JSON can't be parsed.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct SyntaxError {
    /// description of error
    pub msg: String,
    /// error occurred after reading Offset bytes
    pub offset: i64,
}

// Go: json/v2_decode.go:UnmarshalTypeError
/// An UnmarshalTypeError describes a JSON value that was
/// not appropriate for a value of a specific Go type.
#[derive(Clone, PartialEq, Debug)]
pub struct UnmarshalTypeError {
    /// description of JSON value - "bool", "array", "number -5"
    pub value: String,
    /// type of Go value it could not be assigned to (Go `Type.String()`)
    pub type_name: String,
    /// error occurred after reading Offset bytes
    pub offset: i64,
    /// name of the root type containing the field
    pub struct_name: String,
    /// the full path from root node to the value
    pub field: String,
    /// may be nil
    pub err: Option<Box<Error>>,
}

impl UnmarshalTypeError {
    // Go: json/v2_decode.go:UnmarshalTypeError.Error
    pub fn error(&self) -> String {
        let mut s;
        if !self.struct_name.is_empty() || !self.field.is_empty() {
            // The design of UnmarshalTypeError overly assumes a struct-based
            // Go representation for the JSON value.
            // The logic in jsontext represents paths using a JSON Pointer,
            // which is agnostic to the Go type system.
            // Trying to convert a JSON Pointer into a UnmarshalTypeError.Field
            // is difficult. As a heuristic, if the last path token looks like
            // an index into a JSON array (e.g., ".foo.bar.0"),
            // avoid the phrase "Go struct field ".
            let mut into_what = "Go struct field ";
            let i = self.field.rfind('.').map_or(0, |i| i + 1);
            let last = &self.field[i..];
            if !last.is_empty()
                && last
                    .trim_end_matches(|c: char| c.is_ascii_digit())
                    .is_empty()
            {
                into_what = ""; // likely a Go slice or array
            }
            s = format!(
                "json: cannot unmarshal {} into {}{}.{} of type {}",
                self.value, into_what, self.struct_name, self.field, self.type_name
            );
        } else {
            s = format!(
                "json: cannot unmarshal {} into Go value of type {}",
                self.value, self.type_name
            );
        }
        if let Some(err) = &self.err {
            s.push_str(": ");
            s.push_str(&err.to_string());
        }
        s
    }
}

// Go: json/v2_encode.go:MarshalerError
/// A MarshalerError represents an error from calling a
/// `MarshalJSON`, `MarshalJSONTo` or `MarshalText` method.
#[derive(Clone, PartialEq, Debug)]
pub struct MarshalerError {
    /// Go `Type.String()` of the receiver (a pointer type, e.g. `*time.Time`).
    pub type_name: String,
    pub err: Box<Error>,
    /// "MarshalJSON", "MarshalJSONTo" or "MarshalText" ("" means "MarshalJSON").
    pub source_func: &'static str,
}

impl MarshalerError {
    // Go: json/v2_encode.go:MarshalerError.Error
    pub fn error(&self) -> String {
        let mut src_func = self.source_func;
        if src_func.is_empty() {
            src_func = "MarshalJSON";
        }
        format!(
            "json: error calling {} for type {}: {}",
            src_func, self.type_name, self.err
        )
    }
}

/// Every error this crate returns. `Display` (and [`Error::error`]) is Go's
/// `err.Error()` byte for byte.
#[derive(Clone, PartialEq, Debug)]
pub enum Error {
    /// `*json.SyntaxError`.
    Syntax(SyntaxError),
    /// `*json.UnmarshalTypeError`.
    UnmarshalType(UnmarshalTypeError),
    /// `*json.UnsupportedTypeError`: "json: unsupported type: " + Type.String().
    UnsupportedType { type_name: String },
    /// `*json.UnsupportedValueError`: "json: unsupported value: " + Str.
    UnsupportedValue { str: String },
    /// `*json.MarshalerError`.
    Marshaler(MarshalerError),
    /// Any other error (`fmt.Errorf`/`errors.New` values, host method
    /// errors, `*json.SemanticError`): its message.
    Message(String),
    /// `io.EOF` ("EOF") from `Decoder.Decode`/`Token` at the end of the input.
    Eof,
    /// `io.ErrUnexpectedEOF` ("unexpected EOF").
    UnexpectedEof,
    /// An error returned by the underlying reader (its message).
    Io(String),
}

impl Error {
    /// Go: `err.Error()`.
    pub fn error(&self) -> String {
        self.to_string()
    }

    /// The input offset of syntax and type errors (Go's
    /// `SyntaxError.Offset` / `UnmarshalTypeError.Offset`), which Hugo's
    /// `herrors` uses to position file errors.
    pub fn offset(&self) -> Option<i64> {
        match self {
            Error::Syntax(e) => Some(e.offset),
            Error::UnmarshalType(e) => Some(e.offset),
            _ => None,
        }
    }

    /// Go `errors.As(err, &*json.SyntaxError)`: the first SyntaxError in
    /// the chain of wrapped errors.
    pub fn as_syntax_error(&self) -> Option<&SyntaxError> {
        match self {
            Error::Syntax(e) => Some(e),
            Error::UnmarshalType(e) => e.err.as_deref().and_then(Error::as_syntax_error),
            Error::Marshaler(e) => e.err.as_syntax_error(),
            _ => None,
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Syntax(e) => f.write_str(&e.msg),
            Error::UnmarshalType(e) => f.write_str(&e.error()),
            // Go: json/v2_encode.go:UnsupportedTypeError.Error
            Error::UnsupportedType { type_name } => {
                write!(f, "json: unsupported type: {}", type_name)
            }
            // Go: json/v2_encode.go:UnsupportedValueError.Error
            Error::UnsupportedValue { str } => write!(f, "json: unsupported value: {}", str),
            Error::Marshaler(e) => f.write_str(&e.error()),
            Error::Message(m) => f.write_str(m),
            Error::Eof => f.write_str("EOF"),
            Error::UnexpectedEof => f.write_str("unexpected EOF"),
            Error::Io(m) => f.write_str(m),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Marshaler(e) => Some(&*e.err),
            Error::UnmarshalType(e) => e
                .err
                .as_deref()
                .map(|e| e as &(dyn std::error::Error + 'static)),
            _ => None,
        }
    }
}

/// The public form of an internal error that reaches the v1 API without a
/// Go transform: the same `Error()` text, with the sentinels and wrapper
/// types that have public equivalents mapped onto them.
pub(crate) fn public(err: Err) -> Error {
    match err {
        Err::Eof => Error::Eof,
        Err::UnexpectedEof => Error::UnexpectedEof,
        Err::Marshaler(m) => Error::Marshaler(MarshalerError {
            type_name: m.type_name,
            err: Box::new(public(m.err)),
            source_func: m.source_func,
        }),
        other => Error::Message(other.to_string()),
    }
}
