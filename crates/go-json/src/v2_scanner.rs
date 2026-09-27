//! Port of `encoding/json/v2_scanner.go` (go1.27.1): `Valid` and the
//! translation of `jsontext` syntax errors into v1 `SyntaxError`s.

use std::sync::OnceLock;

use go_unicode::replacer::Replacer;

use crate::arshal::syntactic_unexpected_eof;
use crate::error::{Error, SyntaxError, public};
use crate::goerr::Err;
use crate::jsonflags;
use crate::jsontext::decode::DecoderState;
use crate::jsonwire::ValueFlags;

// Go: v2_scanner.go:Valid
/// Valid reports whether data is a valid JSON encoding.
pub fn valid(data: &[u8]) -> bool {
    check_valid(data).is_none()
}

// Go: v2_scanner.go:checkValid
pub(crate) fn check_valid(data: &[u8]) -> Option<Error> {
    let mut d = DecoderState::new_buffered(data, &[]);
    d.opts.flags.set(
        jsonflags::REPORT_ERRORS_WITH_LEGACY_SEMANTICS
            | jsonflags::ALLOW_DUPLICATE_NAMES
            | jsonflags::ALLOW_INVALID_UTF8
            | 1,
    );
    let mut flags = ValueFlags::default();
    if let Err(mut err) = d.read_value(&mut flags) {
        if err == Err::Eof {
            let offset = d.input_offset() + d.unread_buffer().len() as i64;
            err = syntactic_unexpected_eof(offset);
        }
        return Some(transform_syntactic_error(err));
    }
    if let Some(err) = d.check_eof() {
        return Some(transform_syntactic_error(err));
    }
    None
}

/// Go: `errUnexpectedEnd`.
pub(crate) const ERR_UNEXPECTED_END: &str = "unexpected end of JSON input";

// Go: v2_scanner.go:transformSyntacticError
pub(crate) fn transform_syntactic_error(err: Err) -> Error {
    match err {
        Err::Syntactic(serr) => {
            // If the SyntacticError wraps an IO error, unwrap it
            // to match v1 behavior which returned IO errors directly.
            if let Some(Err::Io { err, .. }) = &serr.err {
                return Error::Io(err.clone());
            }
            let mut inner = serr.err.expect("SyntacticError without an error");
            if inner == Err::UnexpectedEof {
                inner = Err::UnexpectedEnd;
            }
            let mut msg = inner.to_string();
            if let Some(i) = msg.find(" (expecting") {
                if !msg.contains(" in literal") {
                    msg.truncate(i);
                }
            }
            let msg = syntax_error_replacer().replace(msg.as_bytes()).into_owned();
            Error::Syntax(SyntaxError {
                offset: serr.byte_offset,
                msg: String::from_utf8_lossy(&msg).into_owned(),
            })
        }
        // v1 historically did not wrap IO errors
        Err::Io { err, .. } => Error::Io(err),
        other => public(other),
    }
}

// Go: v2_scanner.go:syntaxErrorReplacer
/// syntaxErrorReplacer replaces certain string literals in the v2 error
/// to better match the historical string rendering of syntax errors.
/// In particular, v2 uses the terminology "object name" to match RFC 8259,
/// while v1 uses "object key", which is not a term found in JSON literature.
fn syntax_error_replacer() -> &'static Replacer {
    static R: OnceLock<Replacer> = OnceLock::new();
    R.get_or_init(|| {
        Replacer::new(&[
            "object name",
            "object key",
            "at start of value",
            "looking for beginning of value",
            "at start of string",
            "looking for beginning of object key string",
            "after object value",
            "after object key:value pair",
            "in number",
            "in numeric literal",
        ])
    })
}
