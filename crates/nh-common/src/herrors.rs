//! Port of `common/herrors/errors.go`, `common/herrors/file_error.go`, `common/herrors/error_locator.go`, `common/herrors/line_number_extractors.go`.
//!
//! Owner: Wave B task T01 (common-values).

//!
//! One error type for the whole Hugo layer, simpler than Go's chains of wrapped errors: a
//! message, a coarse kind (for the places where Go code branches on the error type with
//! `errors.Is`/type assertions) and an optional file position (Go's `FileError`). The error
//! *text* is Go's, though: it reaches site output through `try`/`.Err` in templates, and the log.
//! Go composes it as follows (`tools/go-oracle/nh-common/herrors` records the matrix):
//!
//! - `(*fileError).Error()` is `"file:line:col": cause` (`text.Position.String`, `<stream>` for
//!   an empty filename);
//! - `fmt.Errorf("prefix: %w", err)` and `fmt.Errorf("prefix: %v", err)` both read
//!   `prefix: ` + `err.Error()`, so wrapping a file error puts the prefix in front of its
//!   position: `prefix: "file:line:col": cause` ([`Error::wrap`] keeps the prefixes apart from
//!   the cause for that). `%v` does not keep the chain (no position, no kind): it is a new
//!   [`Error`] made from the text;
//! - `NewFileError*` wraps whatever it gets, so a positioned error gets a second position in
//!   front (`"b:2:2": "a:1:1": cause`);
//! - `errors.Join` joins the texts with `\n` ([`join`]; `errors.Unwrap` of a join is nil, so the
//!   result has no position).
//!
//! Conversions exist to and from `go_value::Error` so `?` works across template boundaries.

use std::fmt;

/// Error kinds that Go code tests with `errors.Is`/type assertions.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ErrorKind {
    Generic,
    /// `os.ErrNotExist` / `herrors.IsNotExist`.
    NotExist,
    /// `os.ErrExist` / `herrors.IsExist`.
    Exist,
    /// `herrors.ErrFeatureNotAvailable` (e.g. postcss binary missing -> file cache fallback).
    FeatureNotAvailable,
    /// `filecache.ErrFatal`.
    Fatal,
    /// `hexec.NotFoundError` (binary not found).
    ExecNotFound,
    /// Template execution error.
    Template,
    /// `herrors.TimeoutError`.
    Timeout,
}

/// A position in a source file (Go: `common/text.Position`; `offset` -1 = not provided).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FilePos {
    pub filename: String,
    pub line: i64,
    pub column: i64,
}

impl FilePos {
    // Go: common/text/position.go:String (the default `":file::line::col"` format)
    fn string(&self) -> String {
        let filename = if self.filename.is_empty() {
            "<stream>"
        } else {
            &self.filename
        };
        format!("\"{}:{}:{}\"", filename, self.line, self.column)
    }
}

#[derive(Clone)]
pub struct Error {
    /// The text below the position: the whole text of an error without a position, the cause of
    /// a file error (`causeString`).
    msg: String,
    kind: ErrorKind,
    /// The outermost file error's position (Go: `UnwrapFileError(err).Position()`).
    pos: Option<FilePos>,
    /// The `fmt.Errorf("%s: %w")` prefixes around the file error, outermost first (always empty
    /// without a position).
    prefixes: Vec<String>,
    /// For an error without a position made by [`Error::wrap`]: where the wrapped error's text
    /// starts in `msg` (Go: `errors.Unwrap(err).Error()`).
    unwrapped_at: Option<std::num::NonZeroU32>,
}

// `Result<_, (T, Error)>` stays under clippy's `result_large_err` limit (128 bytes) in callers.
const _: () = assert!(std::mem::size_of::<Error>() <= 96);

pub type Result<T> = std::result::Result<T, Error>;

impl Error {
    pub fn new(msg: impl Into<String>) -> Self {
        Error {
            msg: msg.into(),
            kind: ErrorKind::Generic,
            pos: None,
            prefixes: Vec::new(),
            unwrapped_at: None,
        }
    }

    pub fn with_kind(kind: ErrorKind, msg: impl Into<String>) -> Self {
        Error {
            msg: msg.into(),
            kind,
            pos: None,
            prefixes: Vec::new(),
            unwrapped_at: None,
        }
    }

    pub fn not_exist(what: impl Into<String>) -> Self {
        Self::with_kind(ErrorKind::NotExist, what)
    }

    pub fn feature_not_available(what: impl Into<String>) -> Self {
        Self::with_kind(ErrorKind::FeatureNotAvailable, what)
    }

    // Go: common/herrors/errors.go:(*TimeoutError).Error
    /// Go: `&herrors.TimeoutError{Duration: d}` (`timeout after %s`).
    pub fn timeout(d: go_time::Duration) -> Self {
        Self::with_kind(ErrorKind::Timeout, format!("timeout after {}", d.string()))
    }

    pub fn kind(&self) -> ErrorKind {
        self.kind
    }

    /// The text without the position and the prefixes around it: the whole text of an error
    /// without a position, the cause (Go: `causeString`) of a file error.
    pub fn message(&self) -> &str {
        &self.msg
    }

    pub fn pos(&self) -> Option<&FilePos> {
        self.pos.as_ref()
    }

    /// Sets the position of the outermost file error, or makes an error without a position a
    /// file error at `pos` (Go: `FileError.UpdatePosition` + `SetFilename`; `NewFileErrorFromPos`
    /// of an error without a position). Prefixes around a file error stay in front of it.
    pub fn at(mut self, pos: FilePos) -> Self {
        if self.pos.is_none() {
            self.unwrapped_at = None;
        }
        self.pos = Some(pos);
        self
    }

    /// Go: `fmt.Errorf("%s: %w", prefix, err)`, which reads `prefix: ` + `err.Error()`; a file
    /// error keeps its position (and the kind) inside the new text.
    pub fn wrap(mut self, prefix: impl fmt::Display) -> Self {
        let prefix = prefix.to_string();
        if self.pos.is_some() {
            self.prefixes.insert(0, prefix);
        } else {
            self.unwrapped_at = u32::try_from(prefix.len() + 2)
                .ok()
                .and_then(std::num::NonZeroU32::new);
            self.msg = format!("{prefix}: {}", self.msg);
        }
        self
    }

    /// Go: `herrors.Unwrap(err).Error()`: the text one `errors.Unwrap` below this error (its own
    /// text when it does not wrap).
    fn unwrapped_text(&self) -> String {
        if !self.prefixes.is_empty() {
            let mut inner = self.clone();
            inner.prefixes.remove(0);
            return inner.to_string();
        }
        match (&self.pos, self.unwrapped_at) {
            (None, Some(i)) => self.msg[i.get() as usize..].to_string(),
            _ => self.msg.clone(),
        }
    }

    /// Go: `&fileError{cause: err, position: pos}`, a new file error around this one. The text
    /// of an error that already has a position becomes the cause, so both positions print.
    fn into_file_error(self, pos: FilePos) -> Self {
        if self.pos.is_none() {
            return self.at(pos);
        }
        Error {
            msg: self.to_string(),
            kind: self.kind,
            pos: Some(pos),
            prefixes: Vec::new(),
            unwrapped_at: None,
        }
    }

    pub fn is_not_exist(&self) -> bool {
        self.kind == ErrorKind::NotExist
    }

    pub fn is_feature_not_available(&self) -> bool {
        self.kind == ErrorKind::FeatureNotAvailable
    }
}

impl fmt::Display for Error {
    // Go: common/herrors/file_error.go:(*fileError).Error
    // Go: fmt/errors.go:Errorf (the `prefix: %w` wrappers around it)
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for p in &self.prefixes {
            write!(f, "{p}: ")?;
        }
        if let Some(p) = &self.pos {
            write!(f, "{}: {}", p.string(), self.msg)
        } else {
            f.write_str(&self.msg)
        }
    }
}

impl fmt::Debug for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Error({:?}, {:?})", self.kind, self.to_string())
    }
}

impl std::error::Error for Error {}

impl From<go_value::Error> for Error {
    fn from(e: go_value::Error) -> Self {
        Error::new(e.message())
    }
}

impl From<Error> for go_value::Error {
    fn from(e: Error) -> Self {
        go_value::Error::new(e.to_string())
    }
}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        let kind = match e.kind() {
            std::io::ErrorKind::NotFound => ErrorKind::NotExist,
            std::io::ErrorKind::AlreadyExists => ErrorKind::Exist,
            _ => ErrorKind::Generic,
        };
        Error::with_kind(kind, e.to_string())
    }
}

impl From<std::fmt::Error> for Error {
    fn from(e: std::fmt::Error) -> Self {
        Error::new(e.to_string())
    }
}

/// `fmt.Errorf`-style constructor: `errorf!("failed to render %q", x)` becomes
/// `errorf!("failed to render {:?}", x)`.
#[macro_export]
macro_rules! errorf {
    ($($arg:tt)*) => { $crate::herrors::Error::new(format!($($arg)*)) };
}

// Go: common/herrors/errors.go:PrintStackTrace
/// PrintStackTrace prints the current stacktrace to w.
pub fn print_stack_trace(w: &mut dyn std::io::Write) {
    let _ = write!(w, "{}", std::backtrace::Backtrace::force_capture());
}

// Go: common/herrors/errors.go:Recover
/// Recover: Go recovers a panic in a deferred call and prints it. Rust code reports panics
/// through `std::panic::catch_unwind`; this prints a caught panic payload the same way.
pub fn recover(payload: &(dyn std::any::Any + Send), args: &[&str]) {
    let msg = payload
        .downcast_ref::<String>()
        .map(String::as_str)
        .or_else(|| payload.downcast_ref::<&str>().copied())
        .unwrap_or("panic");
    println!("ERR: {msg}");
    let mut parts: Vec<String> = args.iter().map(|s| s.to_string()).collect();
    parts.push(format!(
        "stacktrace from panic: \n{}",
        std::backtrace::Backtrace::force_capture()
    ));
    parts.push("\n".to_string());
    println!("{}", parts.join(" "));
}

// Go: common/herrors/errors.go:IsTimeoutError
/// IsTimeoutError returns true if the given error is or contains a TimeoutError.
pub fn is_timeout_error(err: &Error) -> bool {
    err.kind == ErrorKind::Timeout
}

// Go: common/herrors/errors.go:IsFeatureNotAvailableError
/// IsFeatureNotAvailableError returns true if the given error is or contains a
/// FeatureNotAvailableError.
pub fn is_feature_not_available(err: &Error) -> bool {
    err.is_feature_not_available()
}

/// Go: `herrors.ErrFeatureNotAvailable`.
pub fn err_feature_not_available() -> Error {
    Error::feature_not_available(
        "this feature is not available in your current Hugo version, see https://goo.gl/YMrWcn for more information",
    )
}

// Go: common/herrors/errors.go:Must
/// Must panics if err != nil.
pub fn must(err: Result<()>) {
    if let Err(e) = err {
        panic!("{e}");
    }
}

// Go: common/herrors/errors.go:IsNotExist
/// IsNotExist returns true if the error is a file not found error (also when wrapped).
pub fn is_not_exist(err: &Error) -> bool {
    err.is_not_exist()
}

// Go: common/herrors/errors.go:IsExist
/// IsExist returns true if the error is a file exists error (also when wrapped).
pub fn is_exist(err: &Error) -> bool {
    err.kind == ErrorKind::Exist
}

const NIL_POINTER_SUFFIX: &str =
    ": runtime error: invalid memory address or nil pointer dereference";

const DEFERRED_PREFIX: &str = "__hdeferred/";

// Go: common/herrors/errors.go:ImproveRenderErr
/// ImproveRenderErr improves the error message for rendering errors: nil-pointer method calls get
/// a "wrap it in if or with" hint, and deferred-template names are removed.
/// The rewrites apply to the message; a position and the prefixes around it are kept (Go
/// rewrites the whole text, which holds no template call or deferred name outside the message).
pub fn improve_render_err(in_err: Error) -> Error {
    let mut out = in_err.clone();
    if let Some(msg) = improve_if_nil_pointer_msg(&in_err.msg) {
        out.msg = msg;
        // Go: an `errMessage` whose Unwrap is inErr.
        out.unwrapped_at = None;
    }

    if in_err.msg.contains(DEFERRED_PREFIX) {
        // deferredStringToRemove = `executing "__hdeferred/.*?" ` -> "executing "
        out.msg = remove_deferred(&in_err.msg);
        out.unwrapped_at = None;
    }
    out
}

/// `regexp.MustCompile(`executing "__hdeferred/.*?" `).ReplaceAllString(s, "executing ")`
/// (`.` does not match a newline).
fn remove_deferred(s: &str) -> String {
    const START: &str = "executing \"__hdeferred/";
    let mut out = String::new();
    let mut rest = s;
    'outer: while let Some(i) = rest.find(START) {
        let after = &rest[i + START.len()..];
        // Lazy `.*?` followed by `" `, not crossing a newline.
        let line_end = after.find('\n').unwrap_or(after.len());
        if let Some(j) = after[..line_end].find("\" ") {
            out.push_str(&rest[..i]);
            out.push_str("executing ");
            rest = &after[j + 2..];
            continue 'outer;
        }
        out.push_str(&rest[..i + START.len()]);
        rest = after;
    }
    out.push_str(rest);
    out
}

// Go: common/herrors/errors.go:improveIfNilPointerMsg
/// The nil-pointer hint for `at <(.*)>: error calling (.*?): runtime error: invalid memory
/// address or nil pointer dereference` (first match on the first line holding one).
fn improve_if_nil_pointer_msg(s: &str) -> Option<String> {
    // Leftmost match; `.` does not match '\n', so a match lies within one line.
    let mut line_start = 0;
    for line in s.split('\n') {
        if let Some((m_start, m_end, call, field)) = nil_pointer_match(line) {
            let parts: Vec<&str> = call.split('.').collect();
            if parts.len() < 2 {
                return None;
            }
            let receiver_name = parts[parts.len() - 2];
            let receiver = parts[..parts.len() - 1].join(".");
            let repl = format!(
                "– {receiver_name} is nil; wrap it in if or with: {{{{ with {receiver} }}}}{{{{ .{field} }}}}{{{{ end }}}}"
            );
            let mut out = String::new();
            out.push_str(&s[..line_start + m_start]);
            out.push_str(&repl);
            out.push_str(&s[line_start + m_end..]);
            return Some(out);
        }
        line_start += line.len() + 1;
    }
    None
}

/// Finds `at <CALL>: error calling FIELD: runtime error: ...` in one line: the leftmost `at <`,
/// greedy CALL (the last `>: error calling ` that allows a match), lazy FIELD.
fn nil_pointer_match(line: &str) -> Option<(usize, usize, &str, &str)> {
    const OPEN: &str = "at <";
    const MID: &str = ">: error calling ";
    let mut search_from = 0;
    while let Some(i) = line[search_from..].find(OPEN).map(|i| i + search_from) {
        let call_start = i + OPEN.len();
        // Greedy (.*): try the last MID first.
        let mut mids: Vec<usize> = line[call_start..]
            .match_indices(MID)
            .map(|(j, _)| call_start + j)
            .collect();
        mids.reverse();
        for mid in mids {
            let field_start = mid + MID.len();
            // Lazy (.*?): the first suffix occurrence.
            if let Some(k) = line[field_start..].find(NIL_POINTER_SUFFIX) {
                let field_end = field_start + k;
                return Some((
                    i,
                    field_end + NIL_POINTER_SUFFIX.len(),
                    &line[call_start..mid],
                    &line[field_start..field_end],
                ));
            }
        }
        search_from = i + 1;
    }
    None
}

// Go: common/herrors/file_error.go:Cause
/// Cause returns the underlying error. This error type does not wrap, so it is itself.
pub fn cause(err: &Error) -> &Error {
    err
}

// Go: common/herrors/file_error.go:Unwrap
/// Unwrap returns the underlying error or itself (this error type does not wrap).
pub fn unwrap(err: &Error) -> &Error {
    err
}

// Go: common/herrors/file_error.go:NewFileError
/// NewFileError creates a new FileError that wraps err, taking the line and column from the
/// error message (see [`extract_line_no`]); line 1, column 1 if none is found.
/// The filename is empty (Go takes one only from a Sass error; deviation 9).
pub fn new_file_error(err: Error) -> Error {
    let (line, col) = extract_file_pos(&err.unwrapped_text());
    err.into_file_error(FilePos {
        filename: String::new(),
        line,
        column: col,
    })
}

// Go: common/herrors/file_error.go:NewFileErrorFromName
/// NewFileErrorFromName creates a new FileError that wraps err; name identifies the file.
pub fn new_file_error_from_name(err: Error, name: &str) -> Error {
    let (line, col) = extract_file_pos(&err.unwrapped_text());
    err.into_file_error(FilePos {
        filename: name.to_string(),
        line,
        column: col,
    })
}

// Go: common/herrors/file_error.go:NewFileErrorFromPos
/// NewFileErrorFromPos uses the filename and line number from pos, wrapping err.
pub fn new_file_error_from_pos(err: Error, pos: FilePos) -> Error {
    err.into_file_error(pos)
}

// Go: errors/join.go:Join
/// Join returns an error that wraps the given errors (`None` entries are discarded; `None` when
/// nothing is left). Its text is the errors' texts joined by newlines. `errors.Is` sees every
/// joined error (the kind is the first one that is not `Generic`), `UnwrapFileError` none (no
/// position: `errors.Unwrap` of a join is nil).
pub fn join(errs: impl IntoIterator<Item = Option<Error>>) -> Option<Error> {
    let errs: Vec<Error> = errs.into_iter().flatten().collect();
    if errs.is_empty() {
        return None;
    }
    let kind = errs
        .iter()
        .map(|e| e.kind)
        .find(|k| *k != ErrorKind::Generic)
        .unwrap_or(ErrorKind::Generic);
    let msg = errs
        .iter()
        .map(|e| e.to_string())
        .collect::<Vec<_>>()
        .join("\n");
    Some(Error::with_kind(kind, msg))
}

// Go: common/herrors/file_error.go:extractFileTypePos (the message part)
/// Line 1, column 1 by default; else the first line-number extractor that finds a line > 0.
fn extract_file_pos(msg: &str) -> (i64, i64) {
    for re in LINE_NUMBER_EXTRACTORS {
        let (lno, col) = extract_line_no(*re, msg);
        if lno > 0 {
            return (lno, col);
        }
    }
    (1, 1)
}

/// The patterns of Go's `lineNumberExtractors`, in order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LineNumberPattern {
    /// `:(\d+):(\d*):` (template/shortcode parse errors).
    ColonLineColCol,
    /// `:(\d+):`.
    ColonLine,
    /// `line (\d+):` (YAML parse errors).
    YamlLine,
    /// `\((\d+),\s(\d*)` (i18n bundle errors).
    ParenLineCol,
}

// Go: common/herrors/line_number_extractors.go:lineNumberExtractors
pub const LINE_NUMBER_EXTRACTORS: &[LineNumberPattern] = &[
    LineNumberPattern::ColonLineColCol,
    LineNumberPattern::ColonLine,
    LineNumberPattern::YamlLine,
    LineNumberPattern::ParenLineCol,
];

fn digits(s: &[u8]) -> usize {
    s.iter().take_while(|c| c.is_ascii_digit()).count()
}

/// Leftmost match of the pattern in s: (line digits, optional column digits).
fn find_line_no(p: LineNumberPattern, s: &[u8]) -> Option<(&[u8], Option<&[u8]>)> {
    for i in 0..s.len() {
        let rest = &s[i..];
        match p {
            LineNumberPattern::ColonLineColCol => {
                if rest[0] != b':' {
                    continue;
                }
                let n = digits(&rest[1..]);
                if n == 0 || rest.get(1 + n) != Some(&b':') {
                    continue;
                }
                let r2 = &rest[2 + n..];
                let m = digits(r2);
                if r2.get(m) != Some(&b':') {
                    continue;
                }
                return Some((&rest[1..1 + n], Some(&r2[..m])));
            }
            LineNumberPattern::ColonLine => {
                if rest[0] != b':' {
                    continue;
                }
                let n = digits(&rest[1..]);
                if n == 0 || rest.get(1 + n) != Some(&b':') {
                    continue;
                }
                return Some((&rest[1..1 + n], None));
            }
            LineNumberPattern::YamlLine => {
                let Some(r) = rest.strip_prefix(b"line ") else {
                    continue;
                };
                let n = digits(r);
                if n == 0 || r.get(n) != Some(&b':') {
                    continue;
                }
                return Some((&r[..n], None));
            }
            LineNumberPattern::ParenLineCol => {
                if rest[0] != b'(' {
                    continue;
                }
                let n = digits(&rest[1..]);
                if n == 0 || rest.get(1 + n) != Some(&b',') {
                    continue;
                }
                // \s = [\t\n\f\r ]
                match rest.get(2 + n) {
                    Some(b'\t' | b'\n' | b'\x0c' | b'\r' | b' ') => {}
                    _ => continue,
                }
                let r2 = &rest[3 + n..];
                let m = digits(r2);
                return Some((&rest[1..1 + n], Some(&r2[..m])));
            }
        }
    }
    None
}

/// `strconv.Atoi` of a digit run (0 on error, e.g. overflow or empty).
fn atoi(d: &[u8]) -> i64 {
    std::str::from_utf8(d)
        .ok()
        .and_then(|s| s.parse::<i64>().ok())
        .unwrap_or(0)
}

// Go: common/herrors/line_number_extractors.go:newLineNumberErrHandlerFromRegexp
// Go: common/herrors/line_number_extractors.go:extractLineNo
/// The (line, column) the pattern finds in the error message; (0, 1) when it does not match. A
/// column that is missing, empty or not positive is 1.
pub fn extract_line_no(p: LineNumberPattern, msg: &str) -> (i64, i64) {
    let mut col = 1;
    if let Some((lno, c)) = find_line_no(p, msg.as_bytes()) {
        let lno = atoi(lno);
        if let Some(c) = c {
            col = atoi(c);
        }
        if col <= 0 {
            col = 1;
        }
        return (lno, col);
    }
    (0, col)
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// The error model is simplified, the error texts are Go's (module docs). Items
// without a prefix need a filesystem and error-context rendering (only used to print a failing
// build's error) and are not ported.
// Source: common/herrors/errors.go (187 lines; 2/16 funcs executed)
//   types: ErrorSender, TimeoutError, errMessage, FeatureNotAvailableError
// OK L30-34: PrintStackTrace(w io.Writer)
// OK L45-51: Recover(args ...any)
// OK L54-56: IsTimeoutError(err error) bool
// OK L62-64: (e *TimeoutError) Error() string
// OK L66-69: (e *TimeoutError) Is(target error) bool
// OK L77-79: (e *errMessage) Error() string
// OK L81-83: (e *errMessage) Unwrap() error
// OK L86-88: IsFeatureNotAvailableError(err error) bool
// OK L101-103: (e *FeatureNotAvailableError) Unwrap() error
// OK L105-107: (e *FeatureNotAvailableError) Error() string
// OK L109-112: (e *FeatureNotAvailableError) Is(target error) bool
// OK L115-119: Must(err error)
// OK L123-134: IsNotExist(err error) bool
// OK L138-149: IsExist(err error) bool
// OK L158-170: ImproveRenderErr(inErr error) (outErr error)
// OK L172-187: improveIfNilPointerMsg(inErr error) string
// Source: common/herrors/file_error.go (430 lines; 0/26 funcs executed)
//   types: FileError, Unwrapper, fileError, TextSegmentError
// OK L62-65: (fe *fileError) SetFilename(filename string) FileError (Error::at)
// OK L67-77: (fe *fileError) UpdatePosition(pos text.Position) FileError (Error::at)
//    L79-122: (fe *fileError) UpdateContent(r io.Reader, linematcher LineMatcherFn) FileError
//    L133-135: (e *fileError) ErrorContext() *ErrorContext
// OK L138-140: (e fileError) Position() text.Position (Error::pos)
// OK L142-144: (e *fileError) Error() string
// OK L146-159: (e *fileError) causeString() string
// OK L161-163: (e *fileError) Unwrap() error
// OK L167-171: NewFileError(err error) FileError
// OK L176-185: NewFileErrorFromName(err error, name string) FileError
// OK L188-195: NewFileErrorFromPos(err error, pos text.Position) FileError
//    L197-212: NewFileErrorFromFileInErr(err error, fs afero.Fs, linematcher LineMatcherFn) FileError
//    L214-225: NewFileErrorFromFileInPos(err error, pos text.Position, fs afero.Fs, linematcher LineMatcherFn) FileError
//    L228-238: NewFileErrorFromFile(err error, filename string, fs afero.Fs, linematcher LineMatcherFn) FileError
//    L240-259: openFile(filename string, fs afero.Fs) (afero.File, string, error)
// OK L265-278: Cause(err error) error
// OK L281-286: Unwrap(err error) error
// OK L288-338: extractFileTypePos(err error) (string, text.Position) (message extractors only)
//    L342-352: UnwrapFileError(err error) FileError
//    L355-364: UnwrapFileErrors(err error) []FileError
//    L367-376: UnwrapFileErrorsWithErrorContext(err error) []FileError
//    L378-387: extractOffsetAndType(e error) (int, string)
//    L389-399: extractLineNumberAndColumnNumber(e error) (int, int)
//    L401-416: extractPosition(e error) (pos text.Position)
//    L424-426: (e TextSegmentError) Unwrap() error
//    L428-430: (e TextSegmentError) Error() string
// Source: common/herrors/error_locator.go (170 lines; 0/6 funcs executed)
//   types: LineMatcher, LineMatcherFn, ErrorContext
//    L65-72: ContainsMatcher(text string) func(m LineMatcher) int
//    L93-99: chromaLexerFromType(fileType string) string
//    L101-103: extNoDelimiter(filename string) string
//    L105-112: chromaLexerFromFilename(filename string) string
//    L114-116: locateErrorInString(src string, matcher LineMatcherFn) *ErrorContext
//    L118-170: locateError(r io.Reader, le FileError, matches LineMatcherFn) *ErrorContext
// Source: common/herrors/line_number_extractors.go (63 lines; 2/2 funcs executed)
//   types: lineNumberExtractor
// OK L35-38: newLineNumberErrHandlerFromRegexp(expression string) lineNumberExtractor
// OK L40-63: extractLineNo(re *regexp.Regexp) lineNumberExtractor
// ---------------------------------------------------------------------------
