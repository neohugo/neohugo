//! Port of `encoding/json/jsontext/errors.go` (go1.27.1).
//!
//! Deviation: the JSON Pointer of a `SyntacticError` is not tracked
//! (Go's `pointerSuffixError` wrapping is dropped). The v1 API never exposes
//! it: `transformSyntacticError` keeps only the wrapped error and the offset.

use super::state::pointer_last_token;
use super::state::pointer_parent;
use crate::goerr::Err;
use crate::jsonwire;

/// Go: `errorPrefix`.
const ERROR_PREFIX: &str = "jsontext: ";

// Go: errors.go:SyntacticError
/// SyntacticError is a description of a syntactic error that occurred when
/// encoding or decoding JSON according to the grammar.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct SyntacticError {
    /// ByteOffset indicates that an error occurred after this byte offset.
    pub(crate) byte_offset: i64,
    /// JSONPointer indicates that an error occurred within this JSON value
    /// as indicated using the JSON Pointer notation (see RFC 6901).
    pub(crate) json_pointer: Vec<u8>,
    /// Err is the underlying error.
    pub(crate) err: Option<Err>,
}

impl SyntacticError {
    // Go: errors.go:SyntacticError.Error
    pub(crate) fn error(&self) -> String {
        let mut pointer: &[u8] = &self.json_pointer;
        let mut offset = self.byte_offset;
        let mut b = ERROR_PREFIX.to_string();
        match &self.err {
            Some(err) => {
                b.push_str(&err.to_string());
                if *err == Err::DuplicateName {
                    b.push(' ');
                    b.push_str(&go_strconv::quote(pointer_last_token(pointer)));
                    pointer = pointer_parent(pointer);
                    offset = 0; // not useful to print offset for duplicate names
                }
            }
            None => b.push_str("syntactic error"),
        }
        if !pointer.is_empty() {
            b.push_str(" within ");
            b.push_str(&go_strconv::quote(jsonwire::truncate_pointer(pointer, 100)));
        }
        if offset > 0 {
            b.push_str(" after offset ");
            b.push_str(&offset.to_string());
        }
        b
    }
}
