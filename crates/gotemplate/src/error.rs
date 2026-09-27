//! Errors returned by the template engine.

use std::fmt;
use std::sync::Arc;

use crate::parse::ParseError;

/// Go: `ExecError` — the custom error type returned when Execute has an
/// error evaluating its template.
#[derive(Clone, Debug)]
pub struct ExecError {
    /// Name of template.
    pub name: String,
    /// The formatted message (Go: `Err.Error()`).
    pub message: String,
    /// The error a function or method returned, when the error is Go's
    /// `error calling %s: %w` (Go: what `errors.Unwrap` reaches).
    pub cause: Option<go_value::Error>,
}

impl fmt::Display for ExecError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

/// An error from parsing, escaping or executing a template.
#[derive(Clone, Debug)]
pub enum Error {
    /// A parse error.
    Parse(ParseError),
    /// Go: `template.ExecError`.
    Exec(ExecError),
    /// Go: `*html/template.Error` (and other html/template errors), as text.
    Html(Arc<dyn std::error::Error + Send + Sync>),
    /// An error writing the output (Go strips the `writeError` wrapper).
    Write(Arc<std::io::Error>),
    /// Any other error, by its message (Go `fmt.Errorf`/`errors.New`).
    Other(String),
}

impl Error {
    /// Go: `err.Error()`.
    pub fn message(&self) -> String {
        self.to_string()
    }

    /// Go `herrors.Cause(err)`: the innermost wrapped error's message.
    pub fn cause_message(&self) -> String {
        match self {
            Error::Exec(e) => match &e.cause {
                Some(c) => c.message().to_string(),
                None => e.message.clone(),
            },
            _ => self.to_string(),
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Parse(e) => fmt::Display::fmt(e, f),
            Error::Exec(e) => fmt::Display::fmt(e, f),
            Error::Html(e) => fmt::Display::fmt(e, f),
            Error::Write(e) => fmt::Display::fmt(e, f),
            Error::Other(s) => f.write_str(s),
        }
    }
}

impl std::error::Error for Error {}

impl From<Error> for go_value::Error {
    fn from(e: Error) -> go_value::Error {
        go_value::Error::new(e.to_string())
    }
}
