//! Port of `common/text/position.go`, `common/text/transform.go`.
//!
//! Owner: Wave B task T02 (common-paths-text).
//!
//! The submodules port the golang.org/x/text packages behind `RemoveAccents`: `norm`
//! (`unicode/norm`, NFC/NFD), `xtransform` (`transform`) and `runes`.

pub mod norm;
pub mod runes;
pub mod xtransform;

use std::sync::OnceLock;

use go_unicode::replacer::Replacer;
use go_unicode::{strings, utf8};
use go_value::{IntKind, Value};

/// Go: `text.Position` — a source position in a text file or stream.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Position {
    /// filename, if any
    pub filename: String,
    /// line number, starting at 1
    pub line_number: i64,
    /// column number, starting at 1 (character count per line)
    pub column_number: i64,
    /// byte offset, starting at 0. It's set to -1 if not provided.
    pub offset: i64,
}

impl Position {
    /// Go: `Position.String()` — formatted with `HUGO_FILE_LOG_FORMAT` (default
    /// `":file::line::col"` in quotes); an empty filename is `<stream>`.
    // Go: common/text/position.go:String
    pub fn string(&self) -> String {
        let mut pos = self.clone();
        if pos.filename.is_empty() {
            pos.filename = "<stream>".to_string();
        }
        position_string_format_func().format(&pos)
    }

    /// Go: `Position.IsValid()` — the line number is > 0.
    // Go: common/text/position.go:IsValid
    pub fn is_valid(&self) -> bool {
        self.line_number > 0
    }
}

/// Go: `text.Positioner`.
pub trait Positioner {
    fn position(&self) -> Position;
}

/// Go: the `func(p Position) string` returned by `createPositionStringFormatter`.
#[derive(Clone, Debug)]
pub struct PositionStringFormatter {
    format: Vec<u8>,
    identifiers_found: Vec<&'static str>,
}

impl PositionStringFormatter {
    /// Formats `pos` (Go: `fmt.Sprintf(format, args...)`). Go wraps the message in ANSI colours
    /// when stdout is a terminal; the port never does (message text only).
    pub fn format(&self, pos: &Position) -> String {
        let args: Vec<Value> = self
            .identifiers_found
            .iter()
            .map(|id| match *id {
                ":file" => Value::string(pos.filename.as_str()),
                ":line" => Value::Int(pos.line_number, IntKind::Int),
                _ => Value::Int(pos.column_number, IntKind::Int),
            })
            .collect();

        let msg = go_fmt::sprintf(&self.format, &args);
        // Diagnostics only (error messages), never page bytes.
        String::from_utf8_lossy(&msg).into_owned()
    }
}

// Go: common/text/position.go:createPositionStringFormatter
pub fn create_position_string_formatter(format_str: &str) -> PositionStringFormatter {
    create_position_string_formatter_bytes(format_str.as_bytes())
}

/// [`create_position_string_formatter`] over Go string bytes.
pub fn create_position_string_formatter_bytes(format_str: &[u8]) -> PositionStringFormatter {
    let b: &[u8] = if format_str.is_empty() {
        b"\":file::line::col\""
    } else {
        format_str
    };

    let identifiers = [":file", ":line", ":col"];
    let mut identifiers_found = Vec::new();

    for (i, _) in utf8::runes(b) {
        for id in identifiers {
            if b[i..].starts_with(id.as_bytes()) {
                identifiers_found.push(id);
            }
        }
    }

    let replacer = Replacer::new(&[":file", "%s", ":line", "%d", ":col", "%d"]);
    let format = replacer.replace(b).into_owned();

    PositionStringFormatter {
        format,
        identifiers_found,
    }
}

// Go: common/text/position.go:init
fn position_string_format_func() -> &'static PositionStringFormatter {
    static F: OnceLock<PositionStringFormatter> = OnceLock::new();
    F.get_or_init(|| {
        let v = std::env::var_os("HUGO_FILE_LOG_FORMAT").unwrap_or_default();
        create_position_string_formatter_bytes(&v.into_encoded_bytes())
    })
}

// Go: common/text/transform.go:accentTransformerPool (New)
fn accent_transformer() -> xtransform::Chain {
    xtransform::chain(vec![
        Box::new(norm::NFD),
        Box::new(runes::remove(runes::in_table(go_unicode::tables::MN))),
        Box::new(norm::NFC),
    ])
}

/// Go: `text.RemoveAccents` — removes all accents from b: `transform.Bytes` over
/// `transform.Chain(norm.NFD, runes.Remove(runes.In(unicode.Mn)), norm.NFC)`. The error is
/// dropped, as in Go (the partial result is returned). Invalid UTF-8 bytes become U+FFFD.
// Go: common/text/transform.go:RemoveAccents
pub fn remove_accents(b: &[u8]) -> Vec<u8> {
    // Go takes the chain from a sync.Pool; transform.Bytes resets it first, so a new chain is
    // equivalent.
    let mut t = accent_transformer();
    let (b, _, _) = xtransform::bytes(&mut t, b);
    b
}

/// Go: `text.RemoveAccentsString` over Go string bytes: `transform.String` over the same chain
/// (which feeds the input in 128-byte chunks, unlike `transform.Bytes`).
// Go: common/text/transform.go:RemoveAccentsString
pub fn remove_accents_string_bytes(s: &[u8]) -> Vec<u8> {
    let mut t = accent_transformer();
    let (s, _, _) = xtransform::string(&mut t, s);
    s
}

/// Go: `text.RemoveAccentsString` — removes all accents from s.
///
/// The result is valid UTF-8: the chain's last stage (NFC) only receives the output of
/// `runes.Remove`, which replaces invalid bytes with U+FFFD, and it writes whole runes.
// Go: common/text/transform.go:RemoveAccentsString
pub fn remove_accents_string(s: &str) -> String {
    String::from_utf8(remove_accents_string_bytes(s.as_bytes()))
        .expect("RemoveAccentsString yields valid UTF-8")
}

/// Go: `text.Chomp` — removes trailing newline characters from s.
// Go: common/text/transform.go:Chomp
pub fn chomp(s: &str) -> &str {
    s.trim_end_matches(['\r', '\n'])
}

/// [`chomp`] over Go string bytes.
pub fn chomp_bytes(s: &[u8]) -> &[u8] {
    strings::trim_right_func(s, |r| r == '\n' as i32 || r == '\r' as i32)
}

/// Go: `text.Puts` — adds a trailing \n if none found (not to an empty string).
// Go: common/text/transform.go:Puts
pub fn puts(s: &str) -> String {
    if s.is_empty() || s.ends_with('\n') {
        s.to_string()
    } else {
        format!("{s}\n")
    }
}

/// Go: `text.VisitLinesAfter` — calls `f` for each line, including newlines, in the given string.
// Go: common/text/transform.go:VisitLinesAfter
pub fn visit_lines_after(s: &str, mut f: impl FnMut(&str)) {
    let mut s = s;
    let mut high = s.find('\n');
    while let Some(h) = high {
        f(&s[..h + 1]);
        s = &s[h + 1..];

        high = s.find('\n');
    }

    if !s.is_empty() {
        f(s);
    }
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: common/text/position.go (100 lines; 2/4 funcs executed)
//   types: Positioner, Position
// OK L40-45: (pos Position) String() string
// OK L48-50: (pos Position) IsValid() bool
// OK L54-96: createPositionStringFormatter(formatStr string) func(p Position) string (no ANSI)
// OK L98-100: init()
// Source: common/text/transform.go (78 lines; 0/5 funcs executed)
// OK L33-39: RemoveAccents(b []byte) []byte
// OK L42-48: RemoveAccentsString(s string) string
// OK L51-55: Chomp(s string) string
// OK L58-63: Puts(s string) string
// OK L66-78: VisitLinesAfter(s string, fn func(line string))
// ---------------------------------------------------------------------------
