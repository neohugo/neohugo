//! Port of `common/text/position.go`, `common/text/transform.go`.
//!
//! Owner: Wave B task T02 (common-paths-text).


/// Go: `text.Position`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Position {
    pub filename: String,
    pub line_number: i64,
    pub column_number: i64,
    pub offset: i64,
}

impl Position {
    pub fn is_valid(&self) -> bool {
        self.line_number > 0
    }
}

/// Go: `text.Positioner`.
pub trait Positioner {
    fn position(&self) -> Position;
}

/// Go: `text.RemoveAccents` / `RemoveAccentsString` (used only with removePathAccents=true).
// Go: common/text/transform.go:RemoveAccentsString
pub fn remove_accents_string(s: &str) -> String {
    todo!("x/text transform chain: NFD, remove Mn, NFC")
}

/// Go: `text.Chomp`.
pub fn chomp(s: &str) -> &str {
    s.trim_end_matches(['\r', '\n'])
}

/// Go: `text.Puts` (ensures a trailing newline).
pub fn puts(s: &str) -> String {
    if s.ends_with('\n') { s.to_string() } else { format!("{s}\n") }
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: common/text/position.go (100 lines; 2/4 funcs executed)
//   types: Positioner, Position
//    L40-45: (pos Position) String() string
//    L48-50: (pos Position) IsValid() bool
// EX L54-96: createPositionStringFormatter(formatStr string) func(p Position) string
// EX L98-100: init()
// Source: common/text/transform.go (78 lines; 0/5 funcs executed)
//    L33-39: RemoveAccents(b []byte) []byte
//    L42-48: RemoveAccentsString(s string) string
//    L51-55: Chomp(s string) string
//    L58-63: Puts(s string) string
//    L66-78: VisitLinesAfter(s string, fn func(line string))
// ---------------------------------------------------------------------------
