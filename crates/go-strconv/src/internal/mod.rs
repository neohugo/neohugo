//! Port of go1.27.1 `src/internal/strconv`: the numeric conversion core.
//!
//! Functions here keep Go's `(value, error)` shape: they return the value
//! together with an `Option<Error>`, because Go callers can observe the value
//! even when an error is returned (e.g. `±Inf` with `ErrRange`, or the
//! clamped integer from `ParseInt`). The public crate-root API wraps these in
//! `Result<_, NumError>` like Go's `strconv` package does.

pub(crate) mod atob;
pub(crate) mod atoc;
pub(crate) mod atof;
pub(crate) mod atoi;
pub(crate) mod ctoa;
pub(crate) mod decimal;
pub(crate) mod deps;
pub(crate) mod ftoa;
pub(crate) mod itoa;
pub(crate) mod pow10tab;
pub(crate) mod uscale;

pub use atob::{append_bool, format_bool, parse_bool};
pub use atoc::parse_complex;
pub use atof::{parse_float, parse_float_prefix, set_optimize};
pub use atoi::{INT_SIZE, atoi, parse_int, parse_uint};
pub use ctoa::{append_complex, format_complex};
pub use decimal::Decimal;
pub use deps::{f32_to_f64, f64_to_f32};
pub use ftoa::{append_float, format_float, ftoa32, ftoa64};
pub use itoa::{append_int, append_uint, format_int, format_uint, itoa, runtime_format_base10};

/// Test hooks mirroring go1.27.1 `internal/strconv/export_test.go`.
#[doc(hidden)]
pub mod export_test {
    pub use super::uscale::{log2_pow10, log10_pow2};
}

// Go: internal/strconv/atoi.go:Error
/// The error codes of `internal/strconv` (`type Error int`).
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum Error {
    /// ErrRange indicates that a value is out of range for the target type.
    Range,
    /// ErrSyntax indicates that a value does not have the right syntax for the target type.
    Syntax,
    /// ErrBase indicates that a base is invalid.
    Base,
    /// ErrBitSize indicates that a bit size is invalid.
    BitSize,
}

impl std::fmt::Display for Error {
    // Go: internal/strconv/atoi.go:Error.Error
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Error::Range => "value out of range",
            Error::Syntax => "invalid syntax",
            Error::Base => "invalid base",
            Error::BitSize => "invalid bit size",
        })
    }
}

impl std::error::Error for Error {}
