//! Port of `github.com/bep/golibsass@v1.2.0/libsass/libsasserrors`.
//!
//! `JsonToError` uses `encoding/json`; this module carries a small decoder
//! that implements the parts of `json.Unmarshal` that matter for LibSass's
//! error JSON (see PORTING.md for the exact subset).

use std::fmt;

/// Go: `libsasserrors.Error` — a libsass error.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Error {
    /// `json:"status"`
    pub status: i64,
    /// `json:"column"`
    pub column: i64,
    /// `json:"file"`
    pub file: String,
    /// `json:"line"`
    pub line: i64,
    /// `json:"message"`
    pub message: String,
}

// Go: libsasserrors.go:(Error).Error
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // fmt.Sprintf("file %q, line %d, col %d: %s ", e.File, e.Line, e.Column, e.Message)
        write!(
            f,
            "file {}, line {}, col {}: {} ",
            go_quote(&self.file),
            self.line,
            self.column,
            self.message
        )
    }
}

impl std::error::Error for Error {}

// Go: libsasserrors.go:JsonToError
/// JsonToError converts a JSON string to an error. Decoding errors are
/// ignored (Go: `_ = json.Unmarshal(...)`): fields that could not be decoded
/// keep their zero value, and syntactically invalid JSON leaves every field
/// zero.
pub fn json_to_error(jsonstr: &[u8]) -> Error {
    let mut e = Error::default();
    let mut p = Parser { s: jsonstr, i: 0 };
    // json.Unmarshal validates the whole input before decoding anything.
    if !p.valid_document() {
        return e;
    }
    let mut p = Parser { s: jsonstr, i: 0 };
    p.decode_error(&mut e);
    e
}

/// `encoding/json/jsontext` `maxNestingDepth` (go1.27.1).
const MAX_NESTING_DEPTH: usize = 10000;

/// A tiny JSON reader implementing `json.Unmarshal` into the `Error` struct.
struct Parser<'a> {
    s: &'a [u8],
    i: usize,
}

enum Val {
    Str(String),
    Num(Vec<u8>),
    Other,
}

impl Parser<'_> {
    fn ws(&mut self) {
        while self.i < self.s.len() && matches!(self.s[self.i], b' ' | b'\t' | b'\n' | b'\r') {
            self.i += 1;
        }
    }

    fn peek(&self) -> Option<u8> {
        self.s.get(self.i).copied()
    }

    fn valid_document(&mut self) -> bool {
        self.ws();
        if !self.skip_value(0) {
            return false;
        }
        self.ws();
        self.i == self.s.len()
    }

    /// Validates and skips one value. `depth` is the number of enclosing
    /// objects/arrays. Iterative (an explicit stack of the open containers)
    /// like jsontext's decoder: Go accepts 10000 nesting levels, which a
    /// recursive validator overflows on a 2 MiB thread in a debug build.
    fn skip_value(&mut self, depth: usize) -> bool {
        // The containers open inside this value: true = object, false = array.
        let mut open: Vec<bool> = Vec::new();
        loop {
            // One value (or the opening of a non-empty container).
            self.ws();
            let ok = match self.peek() {
                Some(b'{' | b'[') if depth + open.len() >= MAX_NESTING_DEPTH => {
                    // jsontext: a container at nesting depth maxNestingDepth+1
                    // is a syntax error ("exceeded max depth").
                    false
                }
                Some(b'{') => {
                    self.i += 1;
                    self.ws();
                    if self.peek() == Some(b'}') {
                        self.i += 1;
                    } else {
                        if !self.object_key() {
                            return false;
                        }
                        open.push(true);
                        continue;
                    }
                    true
                }
                Some(b'[') => {
                    self.i += 1;
                    self.ws();
                    if self.peek() == Some(b']') {
                        self.i += 1;
                    } else {
                        open.push(false);
                        continue;
                    }
                    true
                }
                Some(b'"') => self.read_string().is_some(),
                Some(b't') => self.lit(b"true"),
                Some(b'f') => self.lit(b"false"),
                Some(b'n') => self.lit(b"null"),
                Some(b'-' | b'0'..=b'9') => self.read_number().is_some(),
                _ => false,
            };
            if !ok {
                return false;
            }
            // A value is complete: close containers until one continues.
            loop {
                let Some(&object) = open.last() else {
                    return true;
                };
                self.ws();
                match self.peek() {
                    Some(b',') => {
                        self.i += 1;
                        if object && !self.object_key() {
                            return false;
                        }
                        break;
                    }
                    Some(b'}') if object => {
                        self.i += 1;
                        open.pop();
                    }
                    Some(b']') if !object => {
                        self.i += 1;
                        open.pop();
                    }
                    _ => return false,
                }
            }
        }
    }

    /// An object member's name and colon: `ws "name" ws :`.
    fn object_key(&mut self) -> bool {
        self.ws();
        if self.peek() != Some(b'"') || self.read_string().is_none() {
            return false;
        }
        self.ws();
        if self.peek() != Some(b':') {
            return false;
        }
        self.i += 1;
        true
    }

    fn lit(&mut self, l: &[u8]) -> bool {
        if self.s[self.i..].starts_with(l) {
            self.i += l.len();
            true
        } else {
            false
        }
    }

    /// JSON number grammar (encoding/json scanner).
    fn read_number(&mut self) -> Option<Vec<u8>> {
        let start = self.i;
        if self.peek() == Some(b'-') {
            self.i += 1;
        }
        match self.peek() {
            Some(b'0') => self.i += 1,
            Some(b'1'..=b'9') => {
                while matches!(self.peek(), Some(b'0'..=b'9')) {
                    self.i += 1;
                }
            }
            _ => return None,
        }
        if self.peek() == Some(b'.') {
            self.i += 1;
            if !matches!(self.peek(), Some(b'0'..=b'9')) {
                return None;
            }
            while matches!(self.peek(), Some(b'0'..=b'9')) {
                self.i += 1;
            }
        }
        if matches!(self.peek(), Some(b'e' | b'E')) {
            self.i += 1;
            if matches!(self.peek(), Some(b'+' | b'-')) {
                self.i += 1;
            }
            if !matches!(self.peek(), Some(b'0'..=b'9')) {
                return None;
            }
            while matches!(self.peek(), Some(b'0'..=b'9')) {
                self.i += 1;
            }
        }
        Some(self.s[start..self.i].to_vec())
    }

    /// Reads a JSON string, decoding it like encoding/json `unquote`:
    /// invalid UTF-8 bytes and invalid `\u` surrogates become U+FFFD.
    fn read_string(&mut self) -> Option<String> {
        debug_assert_eq!(self.peek(), Some(b'"'));
        self.i += 1;
        let mut out = String::new();
        loop {
            let c = *self.s.get(self.i)?;
            match c {
                b'"' => {
                    self.i += 1;
                    return Some(out);
                }
                b'\\' => {
                    self.i += 1;
                    let e = *self.s.get(self.i)?;
                    self.i += 1;
                    match e {
                        b'"' => out.push('"'),
                        b'\\' => out.push('\\'),
                        b'/' => out.push('/'),
                        b'b' => out.push('\u{8}'),
                        b'f' => out.push('\u{c}'),
                        b'n' => out.push('\n'),
                        b'r' => out.push('\r'),
                        b't' => out.push('\t'),
                        b'u' => {
                            let r = self.hex4()?;
                            if (0xD800..0xE000).contains(&r) {
                                // utf16.IsSurrogate: try to combine with a following \uXXXX.
                                let save = self.i;
                                let mut combined = None;
                                if self.s.get(self.i) == Some(&b'\\')
                                    && self.s.get(self.i + 1) == Some(&b'u')
                                {
                                    self.i += 2;
                                    // utf16.DecodeRune
                                    if let Some(r2) = self.hex4()
                                        && (0xD800..0xDC00).contains(&r)
                                        && (0xDC00..0xE000).contains(&r2)
                                    {
                                        combined = char::from_u32(
                                            0x10000 + ((r - 0xD800) << 10) + (r2 - 0xDC00),
                                        );
                                    }
                                    if combined.is_none() {
                                        self.i = save;
                                    }
                                }
                                out.push(combined.unwrap_or('\u{FFFD}'));
                            } else {
                                out.push(char::from_u32(r).unwrap_or('\u{FFFD}'));
                            }
                        }
                        _ => return None,
                    }
                }
                0..=0x1f => return None,
                0x20..=0x7f => {
                    out.push(c as char);
                    self.i += 1;
                }
                _ => {
                    // Decode one UTF-8 sequence; invalid bytes become U+FFFD (width 1).
                    let (ch, w) = decode_rune(&self.s[self.i..]);
                    out.push(ch);
                    self.i += w;
                }
            }
        }
    }

    fn hex4(&mut self) -> Option<u32> {
        let h = self.s.get(self.i..self.i + 4)?;
        let mut r = 0u32;
        for &c in h {
            let d = match c {
                b'0'..=b'9' => c - b'0',
                b'a'..=b'f' => c - b'a' + 10,
                b'A'..=b'F' => c - b'A' + 10,
                _ => return None,
            };
            r = r * 16 + d as u32;
        }
        self.i += 4;
        Some(r)
    }

    fn read_value(&mut self) -> Val {
        self.ws();
        match self.peek() {
            Some(b'"') => Val::Str(self.read_string().unwrap_or_default()),
            Some(b'-' | b'0'..=b'9') => Val::Num(self.read_number().unwrap_or_default()),
            _ => {
                self.skip_value(0);
                Val::Other
            }
        }
    }

    /// Decodes the (already validated) document into `e`.
    fn decode_error(&mut self, e: &mut Error) {
        self.ws();
        if self.peek() != Some(b'{') {
            // Non-object: UnmarshalTypeError, nothing set.
            return;
        }
        self.i += 1;
        loop {
            self.ws();
            match self.peek() {
                Some(b'}') | None => return,
                Some(b',') => {
                    self.i += 1;
                    continue;
                }
                _ => {}
            }
            let key = self.read_string().unwrap_or_default();
            self.ws();
            self.i += 1; // ':'
            let v = self.read_value();
            let field = field_for_key(&key);
            match (field, v) {
                (Some(Field::File), Val::Str(s)) => e.file = s,
                (Some(Field::Message), Val::Str(s)) => e.message = s,
                (Some(Field::Status), Val::Num(n)) => {
                    if let Some(n) = parse_go_int(&n) {
                        e.status = n
                    }
                }
                (Some(Field::Column), Val::Num(n)) => {
                    if let Some(n) = parse_go_int(&n) {
                        e.column = n
                    }
                }
                (Some(Field::Line), Val::Num(n)) => {
                    if let Some(n) = parse_go_int(&n) {
                        e.line = n
                    }
                }
                // Type mismatches (and null) leave the field unchanged.
                _ => {}
            }
        }
    }
}

enum Field {
    Status,
    Column,
    File,
    Line,
    Message,
}

/// encoding/json field matching: exact name first, then case-insensitive
/// (ASCII folding plus the two non-ASCII folds that map onto the ASCII
/// letters used here: U+017F `ſ` -> `s` and U+212A KELVIN SIGN -> `k`).
fn field_for_key(key: &str) -> Option<Field> {
    let folded: String = key
        .chars()
        .map(|c| match c {
            '\u{017F}' => 's',
            '\u{212A}' => 'k',
            c => c.to_ascii_lowercase(),
        })
        .collect();
    match folded.as_str() {
        "status" => Some(Field::Status),
        "column" => Some(Field::Column),
        "file" => Some(Field::File),
        "line" => Some(Field::Line),
        "message" => Some(Field::Message),
        _ => None,
    }
}

/// `strconv.ParseInt(s, 10, 64)` as used by encoding/json for `int`
/// fields: a literal with a fraction or exponent, or out of range, is an
/// error (the field keeps its value).
fn parse_go_int(n: &[u8]) -> Option<i64> {
    if n.iter().any(|&c| matches!(c, b'.' | b'e' | b'E')) {
        return None;
    }
    std::str::from_utf8(n).ok()?.parse::<i64>().ok()
}

/// Go `utf8.DecodeRune` on a non-ASCII lead byte: returns U+FFFD with width
/// 1 for invalid encodings.
fn decode_rune(s: &[u8]) -> (char, usize) {
    let n = match s[0] {
        0xC2..=0xDF => 2,
        0xE0..=0xEF => 3,
        0xF0..=0xF4 => 4,
        _ => return ('\u{FFFD}', 1),
    };
    if s.len() < n {
        return ('\u{FFFD}', 1);
    }
    match std::str::from_utf8(&s[..n]) {
        Ok(st) => (st.chars().next().unwrap(), n),
        Err(_) => ('\u{FFFD}', 1),
    }
}

/// Go `strconv.Quote` (the `%q` verb of `fmt.Sprintf` for a string), from
/// the `go-strconv` port.
pub fn go_quote(s: &str) -> String {
    go_strconv::quote(s)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn libsass_error_json() {
        let e = json_to_error(
            br#"{
	"status": 1,
	"file": "stdin",
	"line": 3,
	"column": 14,
	"message": "Undefined variable: \"$blue\".",
	"formatted": "Error: Undefined variable: \"$blue\".\n        on line 3:14 of stdin\n>> div { color: $blue; }\n   -------------^\n"
}"#,
        );
        assert_eq!(
            e,
            Error {
                status: 1,
                column: 14,
                file: "stdin".into(),
                line: 3,
                message: "Undefined variable: \"$blue\".".into()
            }
        );
        assert_eq!(
            e.to_string(),
            "file \"stdin\", line 3, col 14: Undefined variable: \"$blue\". "
        );
    }

    #[test]
    fn json_go_semantics() {
        // Invalid document: nothing decoded.
        assert_eq!(json_to_error(br#"{"line": 3,}"#), Error::default());
        assert_eq!(json_to_error(b""), Error::default());
        // Type mismatch leaves the field, other fields still decode.
        let e = json_to_error(
            br#"{"line": 1.5, "column": "x", "file": 7, "Message": "m", "STATUS": 2}"#,
        );
        assert_eq!(
            e,
            Error {
                status: 2,
                column: 0,
                file: String::new(),
                line: 0,
                message: "m".into()
            }
        );
        // Surrogates and invalid UTF-8.
        let e = json_to_error(b"{\"file\": \"\\ud83d\\ude00 \\ud800 \xff\"}");
        assert_eq!(e.file, "\u{1F600} \u{FFFD} \u{FFFD}");
        // Last duplicate wins.
        assert_eq!(json_to_error(br#"{"line": 1, "line": 2}"#).line, 2);
    }

    #[test]
    fn quote() {
        assert_eq!(
            go_quote("a\"b\\c\n\x01\x7fé"),
            "\"a\\\"b\\\\c\\n\\x01\\x7fé\""
        );
        assert_eq!(go_quote("\u{a0}\u{feff}"), "\"\\u00a0\\ufeff\"");
    }
}
