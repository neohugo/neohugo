//! Port of `github.com/pelletier/go-toml/v2@v2.2.4/errors.go`.

use std::fmt;

use super::parser::ParserError;

/// Go: `toml.DecodeError` — an error encountered during the parsing or decoding of a TOML
/// document, with the position in the document where it happened and a human-readable
/// representation that shows where the error occurred.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DecodeError {
    message: Vec<u8>,
    line: i64,
    column: i64,
    human: Vec<u8>,
}

impl DecodeError {
    /// Go: `Error()` — `"toml: " + message`, byte for byte.
    // Go: errors.go:Error
    pub fn error_bytes(&self) -> Vec<u8> {
        let mut m = b"toml: ".to_vec();
        m.extend_from_slice(&self.message);
        m
    }

    /// Go: `String()` — the human-readable contextualized error (multi-line).
    // Go: errors.go:String
    pub fn human(&self) -> &[u8] {
        &self.human
    }

    /// Go: `Position()` — the 1-indexed (line, column) where the error occurred.
    // Go: errors.go:Position
    pub fn position(&self) -> (i64, i64) {
        (self.line, self.column)
    }
}

impl fmt::Display for DecodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&String::from_utf8_lossy(&self.error_bytes()))
    }
}

/// An error returned by `toml.Unmarshal`: a `*toml.DecodeError` (parse errors, with a position)
/// or a plain Go error (`fmt.Errorf`, e.g. duplicate keys from the seen tracker).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Error {
    Decode(DecodeError),
    Plain(Vec<u8>),
}

impl Error {
    /// Go's `err.Error()`, byte for byte.
    pub fn error_bytes(&self) -> Vec<u8> {
        match self {
            Error::Decode(d) => d.error_bytes(),
            Error::Plain(m) => m.clone(),
        }
    }

    /// Go's `err.Error()` (invalid UTF-8 replaced).
    pub fn message(&self) -> String {
        String::from_utf8_lossy(&self.error_bytes()).into_owned()
    }

    /// The position of a `*toml.DecodeError`.
    pub fn position(&self) -> Option<(i64, i64)> {
        match self {
            Error::Decode(d) => Some(d.position()),
            Error::Plain(_) => None,
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message())
    }
}

impl std::error::Error for Error {}

/// decodeErrorFromHighlight creates a DecodeError referencing a highlighted
/// range of bytes from document.
// Go: errors.go:wrapDecodeError
pub(crate) fn wrap_decode_error(
    document: &[u8],
    de: &ParserError<'_>,
    offset: usize,
) -> DecodeError {
    let err_message = &de.message;
    let (err_line, err_column) = position_at_end(&document[..offset]);
    let (before, after) = lines_of_context(document, de.highlight, offset, 3);

    let mut buf: Vec<u8> = Vec::new();

    let max_line = err_line + after.len() as i64 - 1;
    let line_column_width = max_line.to_string().len();

    // Write the lines of context strictly before the error.
    let mut i = before.len() as i64 - 1;
    while i > 0 {
        let line = err_line - i;
        buf.extend_from_slice(format_line_number(line, line_column_width).as_bytes());
        buf.push(b'|');

        if !before[i as usize].is_empty() {
            buf.push(b' ');
            buf.extend_from_slice(before[i as usize]);
        }

        buf.push(b'\n');
        i -= 1;
    }

    // Write the document line that contains the error.

    buf.extend_from_slice(format_line_number(err_line, line_column_width).as_bytes());
    buf.extend_from_slice(b"| ");

    if !before.is_empty() {
        buf.extend_from_slice(before[0]);
    }

    buf.extend_from_slice(de.highlight);

    if !after.is_empty() {
        buf.extend_from_slice(after[0]);
    }

    buf.push(b'\n');

    // Write the line with the error message itself (so it does not have a line
    // number).

    buf.extend(std::iter::repeat_n(b' ', line_column_width));
    buf.extend_from_slice(b"| ");

    if !before.is_empty() {
        buf.extend(std::iter::repeat_n(b' ', before[0].len()));
    }

    buf.extend(std::iter::repeat_n(b'~', de.highlight.len()));

    if !err_message.is_empty() {
        buf.push(b' ');
        buf.extend_from_slice(err_message);
    }

    // Write the lines of context strictly after the error.

    for (i, a) in after.iter().enumerate().skip(1) {
        buf.push(b'\n');
        let line = err_line + i as i64;
        buf.extend_from_slice(format_line_number(line, line_column_width).as_bytes());
        buf.push(b'|');

        if !a.is_empty() {
            buf.push(b' ');
            buf.extend_from_slice(a);
        }
    }

    DecodeError {
        message: err_message.clone(),
        line: err_line,
        column: err_column,
        human: buf,
    }
}

// Go: errors.go:formatLineNumber
fn format_line_number(line: i64, width: usize) -> String {
    format!("{line:>width$}")
}

// Go: errors.go:linesOfContext
fn lines_of_context<'a>(
    document: &'a [u8],
    highlight: &[u8],
    offset: usize,
    lines_around: usize,
) -> (Vec<&'a [u8]>, Vec<&'a [u8]>) {
    (
        before_lines(document, offset, lines_around),
        after_lines(document, highlight, offset, lines_around),
    )
}

// Go: errors.go:beforeLines
fn before_lines(document: &[u8], offset: usize, lines_around: usize) -> Vec<&[u8]> {
    let mut before_lines: Vec<&[u8]> = Vec::new();

    // Walk the document backward from the highlight to find previous lines
    // of context.
    let mut rest = &document[..offset];
    let mut o = rest.len() as isize - 1;
    while o >= 0 && before_lines.len() <= lines_around && !rest.is_empty() {
        if rest[o as usize] == b'\n' {
            // handle individual lines
            before_lines.push(&rest[o as usize + 1..]);
            rest = &rest[..o as usize];
            o = rest.len() as isize - 1;
        } else if o == 0 {
            // add the first line only if it's non-empty
            before_lines.push(rest);

            break;
        } else {
            o -= 1;
        }
    }

    before_lines
}

// Go: errors.go:afterLines
fn after_lines<'a>(
    document: &'a [u8],
    highlight: &[u8],
    offset: usize,
    lines_around: usize,
) -> Vec<&'a [u8]> {
    let mut after_lines: Vec<&[u8]> = Vec::new();

    // Walk the document forward from the highlight to find the following
    // lines of context.
    let mut rest = &document[offset + highlight.len()..];
    let mut o = 0;
    while o < rest.len() && after_lines.len() <= lines_around {
        if rest[o] == b'\n' {
            // handle individual lines
            after_lines.push(&rest[..o]);
            rest = &rest[o + 1..];
            o = 0;
        } else if o == rest.len() - 1 {
            // add last line only if it's non-empty
            after_lines.push(rest);

            break;
        } else {
            o += 1;
        }
    }

    after_lines
}

// Go: errors.go:positionAtEnd
fn position_at_end(b: &[u8]) -> (i64, i64) {
    let mut row = 1;
    let mut column = 1;

    for &c in b {
        if c == b'\n' {
            row += 1;
            column = 1;
        } else {
            column += 1;
        }
    }

    (row, column)
}
