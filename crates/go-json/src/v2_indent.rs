//! Port of `encoding/json/v2_indent.go` (go1.27.1): `HTMLEscape`,
//! `Compact` and `Indent`, implemented with `jsontext.AppendFormat`.
//!
//! Go's `*bytes.Buffer` destination is a `&mut Vec<u8>`; as in Go nothing
//! is appended to it when an error is returned.

use crate::error::Error;
use crate::jsonflags;
use crate::jsonopts::{bool_opt, with_indent, with_indent_prefix};
use crate::jsontext;
use crate::v2_scanner::transform_syntactic_error;

// Go: v2_indent.go:HTMLEscape
/// HTMLEscape appends to dst the JSON-encoded src with <, >, &, U+2028 and U+2029
/// characters inside string literals changed to \u003c, \u003e, \u0026, \u2028, \u2029
/// so that the JSON will be safe to embed inside HTML <script> tags.
/// For historical reasons, web browsers don't honor standard HTML
/// escaping within <script> tags, so an alternative JSON encoding must be used.
pub fn html_escape(dst: &mut Vec<u8>, src: &[u8]) {
    dst.reserve(src.len());
    append_html_escape(dst, src);
}

// Go: v2_indent.go:appendHTMLEscape
fn append_html_escape(dst: &mut Vec<u8>, src: &[u8]) {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    // The characters can only appear in string literals,
    // so just scan the string one byte at a time.
    let mut start = 0;
    for (i, &c) in src.iter().enumerate() {
        if c == b'<' || c == b'>' || c == b'&' {
            dst.extend_from_slice(&src[start..i]);
            dst.extend_from_slice(&[
                b'\\',
                b'u',
                b'0',
                b'0',
                HEX[(c >> 4) as usize],
                HEX[(c & 0xF) as usize],
            ]);
            start = i + 1;
        }
        // Convert U+2028 and U+2029 (E2 80 A8 and E2 80 A9).
        if c == 0xE2 && i + 2 < src.len() && src[i + 1] == 0x80 && src[i + 2] & !1 == 0xA8 {
            dst.extend_from_slice(&src[start..i]);
            dst.extend_from_slice(&[
                b'\\',
                b'u',
                b'2',
                b'0',
                b'2',
                HEX[(src[i + 2] & 0xF) as usize],
            ]);
            start = i + "\u{2029}".len();
        }
    }
    dst.extend_from_slice(&src[start..]);
}

// Go: v2_indent.go:Compact
/// Compact appends to dst the JSON-encoded src with
/// insignificant space characters elided.
pub fn compact(dst: &mut Vec<u8>, src: &[u8]) -> Result<(), Error> {
    dst.reserve(src.len());
    let mut b = Vec::new();
    let err = jsontext::append_format(
        &mut b,
        src,
        &[
            bool_opt(jsonflags::REPORT_ERRORS_WITH_LEGACY_SEMANTICS, true),
            bool_opt(jsonflags::ALLOW_DUPLICATE_NAMES, true),
            bool_opt(jsonflags::ALLOW_INVALID_UTF8, true),
            bool_opt(jsonflags::PRESERVE_RAW_STRINGS, true),
        ],
    );
    if let Some(err) = err {
        return Err(transform_syntactic_error(err));
    }
    dst.extend_from_slice(&b);
    Ok(())
}

/// indentGrowthFactor specifies the growth factor of indenting JSON input.
const INDENT_GROWTH_FACTOR: usize = 2;

// Go: v2_indent.go:Indent
/// Indent appends to dst an indented form of the JSON-encoded src.
/// Each element in a JSON object or array begins on a new,
/// indented line beginning with prefix followed by one or more
/// copies of indent according to the indentation nesting.
/// The data appended to dst does not begin with the prefix nor
/// any indentation, to make it easier to embed inside other formatted JSON data.
/// Although leading space characters (space, tab, carriage return, newline)
/// at the beginning of src are dropped, trailing space characters
/// at the end of src are preserved and copied to dst.
/// For example, if src has no trailing spaces, neither will dst;
/// if src ends in a trailing newline, so will dst.
pub fn indent(
    dst: &mut Vec<u8>,
    src: &[u8],
    prefix: impl AsRef<[u8]>,
    indent: impl AsRef<[u8]>,
) -> Result<(), Error> {
    dst.reserve(INDENT_GROWTH_FACTOR * src.len());
    append_indent(dst, src, prefix.as_ref(), indent.as_ref())
}

fn trim_spaces_tabs(s: &[u8]) -> &[u8] {
    let start = s
        .iter()
        .position(|&c| c != b' ' && c != b'\t')
        .unwrap_or(s.len());
    let end = s
        .iter()
        .rposition(|&c| c != b' ' && c != b'\t')
        .map_or(start, |i| i + 1);
    &s[start..end.max(start)]
}

// Go: v2_indent.go:appendIndent
/// On error `dst` is left as it was.
pub(crate) fn append_indent(
    dst: &mut Vec<u8>,
    src: &[u8],
    prefix: &[u8],
    indent: &[u8],
) -> Result<(), Error> {
    // In v2, only spaces and tabs are allowed, while v1 allowed any character.
    let dst_len = dst.len();
    let mut prefix = prefix.to_vec();
    let mut indent = indent.to_vec();
    let mut invalid: Option<(Vec<u8>, Vec<u8>)> = None;
    if trim_spaces_tabs(&prefix).len() + trim_spaces_tabs(&indent).len() > 0 {
        // Use placeholder spaces of correct length, and replace afterwards.
        invalid = Some((prefix.clone(), indent.clone()));
        prefix = vec![b' '; prefix.len()];
        indent = vec![b' '; indent.len()];
    }

    let err = jsontext::append_format(
        dst,
        src,
        &[
            bool_opt(jsonflags::REPORT_ERRORS_WITH_LEGACY_SEMANTICS, true),
            bool_opt(jsonflags::ALLOW_DUPLICATE_NAMES, true),
            bool_opt(jsonflags::ALLOW_INVALID_UTF8, true),
            bool_opt(jsonflags::PRESERVE_RAW_STRINGS, true),
            bool_opt(jsonflags::MULTILINE, true),
            with_indent_prefix(&prefix),
            with_indent(&indent),
        ],
    );
    if let Some(err) = err {
        // (Go's deferred fix-up still runs here, over `src`, which
        // AppendFormat appends on error. With an empty indent it never
        // returns once `src` has a '\n' followed by more spaces than the
        // prefix is long; see PORTING.md, deviation 9.)
        dst.truncate(dst_len);
        return Err(transform_syntactic_error(err));
    }

    // In v2, trailing whitespace is discarded, while v1 preserved it.
    let trimmed = src
        .iter()
        .rposition(|&c| !matches!(c, b' ' | b'\n' | b'\r' | b'\t'))
        .map_or(0, |i| i + 1);
    let n = src.len() - trimmed;
    if n > 0 {
        dst.extend_from_slice(&src[src.len() - n..]);
    }

    // Go: the deferred replacement of the placeholder spaces (it runs after
    // the trailing whitespace was appended, so it also rewrites spaces that
    // follow a trailing newline).
    if let Some((invalid_prefix, invalid_indent)) = invalid {
        let b = &mut dst[dst_len..];
        let mut pos = 0;
        while let Some(i) = b[pos..].iter().position(|&c| c == b'\n') {
            pos += i + 1;
            let n = b[pos..]
                .iter()
                .position(|&c| c != b' ')
                .unwrap_or(b.len() - pos); // len(prefix)+n*len(indent)
            let spaces = &mut b[pos..pos + n];
            let mut k = copy(spaces, &invalid_prefix);
            while k < spaces.len() {
                let m = copy(&mut spaces[k..], &invalid_indent);
                if m == 0 {
                    break; // (Go loops forever here when indent is empty)
                }
                k += m;
            }
            pos += n;
        }
    }
    Ok(())
}

/// Go's built-in `copy(dst, src)`.
fn copy(dst: &mut [u8], src: &[u8]) -> usize {
    let n = dst.len().min(src.len());
    dst[..n].copy_from_slice(&src[..n]);
    n
}
