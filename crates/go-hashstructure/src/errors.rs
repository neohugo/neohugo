//! Go: github.com/gohugoio/hashstructure@v0.5.0 errors.go (+ the other
//! errors `Hash` can return).

use std::fmt;

/// An error returned by [`crate::hash`]. `Display` is Go's `err.Error()`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Error {
    /// Go: `*ErrNotStringer` — a field tagged `hash:"string"` whose value does
    /// not implement `fmt.Stringer`.
    NotStringer { field: String },
    /// Go: `fmt.Errorf("unknown kind to hash: %s", k)`.
    UnknownKind(&'static str),
    /// Go: `binary.Write` failing (only for `uintptr` values).
    BinaryWrite(String),
    /// Go: `time.Time.MarshalBinary` failing.
    Time(String),
    /// An error returned by a `Hashable`/`Includable`/`IncludableMap`
    /// implementation.
    Custom(String),
    /// A situation in which the Go code panics (e.g. `String()` through a
    /// nil `*time.Time`); the message is Go's panic value.
    GoPanic(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            // Go: errors.go:(*ErrNotStringer).Error
            Error::NotStringer { field } => write!(
                f,
                "hashstructure: {field} has hash:\"string\" set, but does not implement fmt.Stringer"
            ),
            Error::UnknownKind(k) => write!(f, "unknown kind to hash: {k}"),
            Error::BinaryWrite(t) => write!(
                f,
                "binary.Write: some values are not fixed-sized in type {t}"
            ),
            Error::Time(m) | Error::Custom(m) | Error::GoPanic(m) => f.write_str(m),
        }
    }
}

impl std::error::Error for Error {}
