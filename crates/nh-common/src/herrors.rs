//! Port of `common/herrors/errors.go`, `common/herrors/file_error.go`, `common/herrors/error_locator.go`, `common/herrors/line_number_extractors.go`.
//!
//! Owner: Wave B task T01 (common-values).

//!
//! One error type for the whole Hugo layer. Error *texts* are not part of byte parity, so this is
//! deliberately simpler than Go's wrapped errors: a message, a coarse kind (for the few places
//! where Go code branches on the error type) and an optional file position.
//! Conversions exist to and from `go_value::Error` so `?` works across template boundaries.

use std::fmt;

/// Error kinds that Go code tests with `errors.Is`/type assertions.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ErrorKind {
    Generic,
    /// `os.ErrNotExist` / `herrors.IsNotExist`.
    NotExist,
    /// `herrors.ErrFeatureNotAvailable` (e.g. postcss binary missing -> file cache fallback).
    FeatureNotAvailable,
    /// `filecache.ErrFatal`.
    Fatal,
    /// `hexec.NotFoundError` (binary not found).
    ExecNotFound,
    /// Template execution error.
    Template,
}

/// A position in a source file (Go: `common/text.Position`).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FilePos {
    pub filename: String,
    pub line: i64,
    pub column: i64,
}

#[derive(Clone)]
pub struct Error {
    msg: String,
    kind: ErrorKind,
    pos: Option<FilePos>,
}

pub type Result<T> = std::result::Result<T, Error>;

impl Error {
    pub fn new(msg: impl Into<String>) -> Self {
        Error { msg: msg.into(), kind: ErrorKind::Generic, pos: None }
    }

    pub fn with_kind(kind: ErrorKind, msg: impl Into<String>) -> Self {
        Error { msg: msg.into(), kind, pos: None }
    }

    pub fn not_exist(what: impl Into<String>) -> Self {
        Self::with_kind(ErrorKind::NotExist, what)
    }

    pub fn feature_not_available(what: impl Into<String>) -> Self {
        Self::with_kind(ErrorKind::FeatureNotAvailable, what)
    }

    pub fn kind(&self) -> ErrorKind {
        self.kind
    }

    pub fn message(&self) -> &str {
        &self.msg
    }

    pub fn pos(&self) -> Option<&FilePos> {
        self.pos.as_ref()
    }

    /// Go: `herrors.NewFileErrorFromName` & friends (position attached for diagnostics only).
    pub fn at(mut self, pos: FilePos) -> Self {
        self.pos = Some(pos);
        self
    }

    /// Go: `fmt.Errorf("%s: %w", prefix, err)`.
    pub fn wrap(self, prefix: impl fmt::Display) -> Self {
        Error { msg: format!("{prefix}: {}", self.msg), ..self }
    }

    pub fn is_not_exist(&self) -> bool {
        self.kind == ErrorKind::NotExist
    }

    pub fn is_feature_not_available(&self) -> bool {
        self.kind == ErrorKind::FeatureNotAvailable
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(p) = &self.pos {
            write!(f, "\"{}:{}:{}\": {}", p.filename, p.line, p.column, self.msg)
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
        let kind = if e.kind() == std::io::ErrorKind::NotFound { ErrorKind::NotExist } else { ErrorKind::Generic };
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

/// Go: `herrors.IsNotExist(err)`.
pub fn is_not_exist(err: &Error) -> bool {
    err.is_not_exist()
}

/// Go: `herrors.IsFeatureNotAvailableError(err)`.
pub fn is_feature_not_available(err: &Error) -> bool {
    err.is_feature_not_available()
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: common/herrors/errors.go (187 lines; 2/16 funcs executed)
//   types: ErrorSender, TimeoutError, errMessage, FeatureNotAvailableError
//    L30-34: PrintStackTrace(w io.Writer)
//    L45-51: Recover(args ...any)
//    L54-56: IsTimeoutError(err error) bool
//    L62-64: (e *TimeoutError) Error() string
//    L66-69: (e *TimeoutError) Is(target error) bool
//    L77-79: (e *errMessage) Error() string
//    L81-83: (e *errMessage) Unwrap() error
// EX L86-88: IsFeatureNotAvailableError(err error) bool
//    L101-103: (e *FeatureNotAvailableError) Unwrap() error
//    L105-107: (e *FeatureNotAvailableError) Error() string
//    L109-112: (e *FeatureNotAvailableError) Is(target error) bool
//    L115-119: Must(err error)
// EX L123-134: IsNotExist(err error) bool
//    L138-149: IsExist(err error) bool
//    L158-170: ImproveRenderErr(inErr error) (outErr error)
//    L172-187: improveIfNilPointerMsg(inErr error) string
// Source: common/herrors/file_error.go (430 lines; 0/26 funcs executed)
//   types: FileError, Unwrapper, fileError, TextSegmentError
//    L62-65: (fe *fileError) SetFilename(filename string) FileError
//    L67-77: (fe *fileError) UpdatePosition(pos text.Position) FileError
//    L79-122: (fe *fileError) UpdateContent(r io.Reader, linematcher LineMatcherFn) FileError
//    L133-135: (e *fileError) ErrorContext() *ErrorContext
//    L138-140: (e fileError) Position() text.Position
//    L142-144: (e *fileError) Error() string
//    L146-159: (e *fileError) causeString() string
//    L161-163: (e *fileError) Unwrap() error
//    L167-171: NewFileError(err error) FileError
//    L176-185: NewFileErrorFromName(err error, name string) FileError
//    L188-195: NewFileErrorFromPos(err error, pos text.Position) FileError
//    L197-212: NewFileErrorFromFileInErr(err error, fs afero.Fs, linematcher LineMatcherFn) FileError
//    L214-225: NewFileErrorFromFileInPos(err error, pos text.Position, fs afero.Fs, linematcher LineMatcherFn) FileError
//    L228-238: NewFileErrorFromFile(err error, filename string, fs afero.Fs, linematcher LineMatcherFn) FileError
//    L240-259: openFile(filename string, fs afero.Fs) (afero.File, string, error)
//    L265-278: Cause(err error) error
//    L281-286: Unwrap(err error) error
//    L288-338: extractFileTypePos(err error) (string, text.Position)
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
// EX L35-38: newLineNumberErrHandlerFromRegexp(expression string) lineNumberExtractor
// EX L40-63: extractLineNo(re *regexp.Regexp) lineNumberExtractor
// ---------------------------------------------------------------------------
