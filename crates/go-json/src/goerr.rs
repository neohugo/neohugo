//! Go `error` values that flow through the ported packages (`io`,
//! `strconv`, `jsonwire`, `jsontext`, `json/v2`, `json/internal`), as one
//! enum. `Display` is Go's `Error()`. Sentinel errors are unit variants so
//! Go's `err == io.EOF` comparisons are `matches!`/`==` on the variant.

use std::fmt;

use crate::arshal::errors::SemanticError;
use crate::jsontext::errors::SyntacticError;
use crate::jsonwire::InvalidTextError;

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Err {
    /// `io.EOF`
    Eof,
    /// `io.ErrUnexpectedEOF`
    UnexpectedEof,
    /// `jsonwire.ErrInvalidUTF8`
    InvalidUtf8,
    /// `*jsonwire.InvalidTextError`
    InvalidText(InvalidTextError),
    /// `jsontext.ErrDuplicateName`
    DuplicateName,
    /// `jsontext.ErrNonStringName`
    NonStringName,
    /// `jsontext.errMissingValue`
    MissingValue,
    /// `jsontext.errMismatchDelim`
    MismatchDelim,
    /// `jsontext.errMaxDepth`
    MaxDepth,
    /// `jsontext.errInvalidNamespace`
    InvalidNamespace,
    /// `jsontext.errInvalidToken`
    InvalidToken,
    /// `*jsontext.ioError`
    Io { action: &'static str, err: String },
    /// `*jsontext.SyntacticError`
    Syntactic(Box<SyntacticError>),
    /// `strconv.ErrRange`
    StrconvRange,
    /// `strconv.ErrSyntax`
    StrconvSyntax,
    /// `*json.SemanticError` (encoding/json/v2)
    Semantic(Box<SemanticError>),
    /// `internal.ErrCycle`
    Cycle,
    /// `internal.ErrNilInterface`
    NilInterface,
    /// `json.errInvalidStringTag` (v2)
    InvalidStringTag,
    /// `json.errUnexpectedEnd` (v1)
    UnexpectedEnd,
    /// `*json.MarshalerError` (v1), created inside v2 through
    /// `internal.NewMarshalerError`.
    Marshaler(Box<MarshalerErr>),
    /// Any other error (`errors.New`, `fmt.Errorf`, host method errors):
    /// its message.
    Msg(String),
}

/// Go v1 `json.MarshalerError` while it still wraps an internal error.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct MarshalerErr {
    /// `reflect.TypeOf(val).String()`
    pub(crate) type_name: String,
    pub(crate) err: Err,
    pub(crate) source_func: &'static str,
}

impl fmt::Display for Err {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Err::Eof => f.write_str("EOF"),
            Err::UnexpectedEof => f.write_str("unexpected EOF"),
            Err::InvalidUtf8 => f.write_str("invalid UTF-8"),
            Err::InvalidText(e) => f.write_str(&e.error()),
            Err::DuplicateName => f.write_str("duplicate object member name"),
            Err::NonStringName => f.write_str("object member name must be a string"),
            Err::MissingValue => f.write_str("missing value after object name"),
            Err::MismatchDelim => f.write_str("mismatching structural token for object or array"),
            Err::MaxDepth => f.write_str("exceeded max depth"),
            Err::InvalidNamespace => f.write_str("object namespace is in an invalid state"),
            Err::InvalidToken => f.write_str("invalid jsontext.Token"),
            // Go: jsontext/errors.go:ioError.Error
            Err::Io { action, err } => write!(f, "jsontext: {} error: {}", action, err),
            Err::Syntactic(e) => f.write_str(&e.error()),
            Err::StrconvRange => f.write_str("value out of range"),
            Err::StrconvSyntax => f.write_str("invalid syntax"),
            Err::Semantic(e) => f.write_str(&e.error()),
            Err::Cycle => f.write_str("encountered a cycle"),
            Err::NilInterface => {
                f.write_str("cannot derive concrete type for nil interface with finite type set")
            }
            Err::InvalidStringTag => f.write_str("invalid use of `string` tag option"),
            Err::UnexpectedEnd => f.write_str("unexpected end of JSON input"),
            // Go: json/v2_encode.go:MarshalerError.Error
            Err::Marshaler(e) => {
                let mut src_func = e.source_func;
                if src_func.is_empty() {
                    src_func = "MarshalJSON";
                }
                write!(
                    f,
                    "json: error calling {} for type {}: {}",
                    src_func, e.type_name, e.err
                )
            }
            Err::Msg(m) => f.write_str(m),
        }
    }
}

impl Err {
    /// Go `errors.Unwrap(err)` for the wrapper types.
    pub(crate) fn unwrap(&self) -> Option<&Err> {
        match self {
            Err::Syntactic(e) => e.err.as_ref(),
            Err::Semantic(e) => e.err.as_ref(),
            Err::Marshaler(e) => Some(&e.err),
            _ => None,
        }
    }

    /// Go `errors.Is(err, target)` for a sentinel target.
    pub(crate) fn is(&self, target: &Err) -> bool {
        let mut cur = Some(self);
        while let Some(e) = cur {
            if e == target {
                return true;
            }
            cur = e.unwrap();
        }
        false
    }
}
