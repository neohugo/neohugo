//! Port of `encoding/json/v2/errors.go` (go1.27.1): `SemanticError` and
//! the helpers that position it.

use crate::goerr::Err;
use crate::jsonflags;
use crate::jsontext::decode::{DecoderState, Source};
use crate::jsontext::encode::EncoderState;
use crate::jsontext::encode::value_kind;
use crate::jsontext::state::{Kind, pointer_contains};
use crate::jsonwire;

/// Go: `errorPrefix`.
const ERROR_PREFIX: &str = "json: ";

// Go: errors.go:isFatalError
/// isFatalError reports whether this error must terminate asharling.
/// All errors are considered fatal unless operating under
/// [jsonflags.ReportErrorsWithLegacySemantics] in which case only
/// syntactic errors and I/O errors are considered fatal.
pub(crate) fn is_fatal_error(err: &Err, flags: &jsonflags::Flags) -> bool {
    !flags.get(jsonflags::REPORT_ERRORS_WITH_LEGACY_SEMANTICS)
        || matches!(err, Err::Syntactic(_) | Err::Io { .. })
}

// Go: errors.go:SemanticError
/// SemanticError describes an error determining the meaning
/// of JSON data as Go data or vice-versa.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct SemanticError {
    /// either "marshal" or "unmarshal"
    pub(crate) action: &'static str,
    /// ByteOffset indicates that an error occurred after this byte offset.
    pub(crate) byte_offset: i64,
    /// JSONPointer indicates that an error occurred within this JSON value
    /// as indicated using the JSON Pointer notation (see RFC 6901).
    pub(crate) json_pointer: Vec<u8>,
    /// JSONKind is the JSON kind that could not be handled.
    pub(crate) json_kind: Kind, // may be zero if unknown
    /// JSONValue is the JSON number or string that could not be unmarshaled.
    /// It is not populated during marshaling.
    pub(crate) json_value: Vec<u8>, // may be nil if irrelevant or unknown
    /// GoType is the Go type that could not be handled
    /// (Go `reflect.Type.String()`).
    pub(crate) go_type: Option<String>, // may be nil if unknown
    /// Err is the underlying error.
    pub(crate) err: Option<Err>, // may be nil
}

impl SemanticError {
    // Go: errors.go:SemanticError.Error
    ///
    /// Deviation: Go picks "cannot" or "unable to" at random (by ranging
    /// over a map) once per process; this port always uses "cannot". The v1
    /// API converts semantic errors into its own error types, so the text
    /// is only visible through host errors that embed it.
    pub(crate) fn error(&self) -> String {
        let mut sb = String::new();
        sb.push_str(ERROR_PREFIX);
        sb.push_str("cannot");

        // Format action.
        let mut preposition;
        match self.action {
            "marshal" => {
                sb.push_str(" marshal");
                preposition = " from";
            }
            "unmarshal" => {
                sb.push_str(" unmarshal");
                preposition = " into";
            }
            _ => {
                sb.push_str(" handle");
                preposition = " with";
            }
        }

        // Format JSON kind.
        match self.json_kind {
            b'n' => sb.push_str(" JSON null"),
            b'f' | b't' => sb.push_str(" JSON boolean"),
            b'"' => sb.push_str(" JSON string"),
            b'0' => sb.push_str(" JSON number"),
            b'{' | b'}' => sb.push_str(" JSON object"),
            b'[' | b']' => sb.push_str(" JSON array"),
            _ => {
                if self.action.is_empty() {
                    preposition = "";
                }
            }
        }
        if !self.json_value.is_empty() && self.json_value.len() < 100 {
            sb.push(' ');
            sb.push_str(&String::from_utf8_lossy(&self.json_value));
        }

        // Format Go type.
        if let Some(t) = &self.go_type {
            sb.push_str(preposition);
            sb.push_str(" Go ");
            sb.push_str(t);
        }

        // Special handling for unknown names.
        // (ErrUnknownName and errAmbiguousName are not reachable from the v1 API.)

        // Format where.
        let serr = match &self.err {
            Some(Err::Syntactic(s)) => Some(s),
            _ => None,
        };
        if !self.json_pointer.is_empty() {
            if serr.is_none() || !pointer_contains(&self.json_pointer, &serr.unwrap().json_pointer)
            {
                sb.push_str(" within ");
                sb.push_str(&go_strconv::quote(jsonwire::truncate_pointer(
                    &self.json_pointer,
                    100,
                )));
            }
        } else if self.byte_offset > 0 {
            if serr.is_none() || !(self.byte_offset <= serr.unwrap().byte_offset) {
                sb.push_str(" after offset ");
                sb.push_str(&self.byte_offset.to_string());
            }
        }

        // Format underlying error.
        if let Some(err) = &self.err {
            let mut err_string = err.to_string();
            if matches!(err, Err::Syntactic(_)) {
                if let Some(s) = err_string.strip_prefix("jsontext: ") {
                    err_string = s.to_string();
                }
            }
            sb.push_str(": ");
            sb.push_str(&err_string);
        }
        sb
    }
}

// Go: errors.go:newMarshalErrorBefore
/// newMarshalErrorBefore wraps err in a SemanticError assuming that e
/// is positioned right before the next token or value, which causes an error.
/// (The JSON Pointer is not tracked for marshal errors; see jsontext/encode.rs.)
pub(crate) fn new_marshal_error_before(e: &EncoderState, t: &str, err: Option<Err>) -> Err {
    Err::Semantic(Box::new(SemanticError {
        action: "marshal",
        go_type: Some(t.to_string()),
        err: err.map(to_unexpected_eof),
        byte_offset: e.output_offset() + e.count_next_delim_whitespace() as i64,
        json_pointer: Vec::new(),
        json_kind: 0,
        json_value: Vec::new(),
    }))
}

// Go: errors.go:newUnmarshalErrorBefore
/// newUnmarshalErrorBefore wraps err in a SemanticError assuming that d
/// is positioned right before the next token or value, which causes an error.
pub(crate) fn new_unmarshal_error_before<R: Source>(
    d: &mut DecoderState<R>,
    t: &str,
    err: Option<Err>,
) -> Err {
    let mut k: Kind = 0;
    if d.opts
        .flags
        .get(jsonflags::REPORT_ERRORS_WITH_LEGACY_SEMANTICS)
    {
        k = d.peek_kind();
    }
    let byte_offset = d.input_offset() + d.count_next_delim_whitespace() as i64;
    let mut ptr = Vec::new();
    d.append_stack_pointer(&mut ptr, 1);
    Err::Semantic(Box::new(SemanticError {
        action: "unmarshal",
        go_type: Some(t.to_string()),
        err: err.map(to_unexpected_eof),
        byte_offset,
        json_pointer: ptr,
        json_kind: k,
        json_value: Vec::new(),
    }))
}

// Go: errors.go:newUnmarshalErrorBeforeWithSkipping
/// newUnmarshalErrorBeforeWithSkipping is like [newUnmarshalErrorBefore],
/// but automatically skips the next value if
/// [jsonflags.ReportErrorsWithLegacySemantics] is specified.
#[allow(dead_code)]
pub(crate) fn new_unmarshal_error_before_with_skipping<R: Source>(
    d: &mut DecoderState<R>,
    t: &str,
    err: Option<Err>,
) -> Err {
    let err = new_unmarshal_error_before(d, t, err);
    if d.opts
        .flags
        .get(jsonflags::REPORT_ERRORS_WITH_LEGACY_SEMANTICS)
    {
        if let Some(err2) = d.skip_value() {
            return err2;
        }
    }
    err
}

// Go: errors.go:newUnmarshalErrorAfter
/// newUnmarshalErrorAfter wraps err in a SemanticError assuming that d
/// is positioned right after the previous token or value, which caused an error.
pub(crate) fn new_unmarshal_error_after<R: Source>(
    d: &mut DecoderState<R>,
    t: &str,
    err: Option<Err>,
) -> SemanticError {
    let tok_or_val = d.previous_token_or_value();
    let mut byte_offset = d.input_offset() - tok_or_val.len() as i64;
    if d.opts
        .flags
        .get(jsonflags::REPORT_ERRORS_WITH_LEGACY_SEMANTICS)
    {
        // TODO(https://go.dev/issue/75516): Reporting the offset
        // after the problematic value is consistent with v1,
        // but is inconsistent with everywhere else in v2.
        let k = value_kind(&tok_or_val);
        if k == b'[' || k == b'{' {
            byte_offset += 1; // add just the '[' or '{'
        } else {
            byte_offset += tok_or_val.len() as i64;
        }
    }
    let mut ptr = Vec::new();
    d.append_stack_pointer(&mut ptr, -1);
    SemanticError {
        action: "unmarshal",
        go_type: Some(t.to_string()),
        err: err.map(to_unexpected_eof),
        byte_offset,
        json_pointer: ptr,
        json_kind: value_kind(&tok_or_val),
        json_value: Vec::new(),
    }
}

// Go: errors.go:newUnmarshalErrorAfterWithValue
/// newUnmarshalErrorAfterWithValue is like [newUnmarshalErrorAfter],
/// but also records the previous value (if it is a JSON string or number).
pub(crate) fn new_unmarshal_error_after_with_value<R: Source>(
    d: &mut DecoderState<R>,
    t: &str,
    err: Option<Err>,
) -> Err {
    let mut serr = new_unmarshal_error_after(d, t, err);
    if serr.json_kind == b'"' || serr.json_kind == b'0' {
        serr.json_value = d.previous_token_or_value();
    }
    Err::Semantic(Box::new(serr))
}

// Go: errors.go:newUnmarshalErrorAfterWithSkipping
/// newUnmarshalErrorAfterWithSkipping is like [newUnmarshalErrorAfter],
/// but automatically skips the remainder of the current value if
/// [jsonflags.ReportErrorsWithLegacySemantics] is specified.
pub(crate) fn new_unmarshal_error_after_with_skipping<R: Source>(
    d: &mut DecoderState<R>,
    t: &str,
    err: Option<Err>,
) -> Err {
    let err = Err::Semantic(Box::new(new_unmarshal_error_after(d, t, err)));
    if d.opts
        .flags
        .get(jsonflags::REPORT_ERRORS_WITH_LEGACY_SEMANTICS)
    {
        if let Some(err2) = d.skip_value_remainder() {
            return err2;
        }
    }
    err
}

// Go: errors.go:toUnexpectedEOF
fn to_unexpected_eof(err: Err) -> Err {
    if err == Err::Eof {
        return Err::UnexpectedEof;
    }
    err
}
