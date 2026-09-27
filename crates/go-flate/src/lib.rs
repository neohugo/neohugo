//! Byte-exact port of Go 1.27.1 `compress/flate` and `compress/zlib`
//! (plus the `hash/adler32` checksum they use).
//!
//! * [`flate::Writer`] produces exactly the bytes Go's `flate.Writer` produces
//!   for the same level, dictionary and sequence of `Write` / `Flush` /
//!   `Reset` / `Close` calls, and issues the same sequence of `Write` calls
//!   on the underlying writer (each Go `Write` is one `write_all`).
//! * [`zlib::Writer`] wraps it with Go's header / Adler-32 trailer logic.
//! * [`flate::Decompressor`] / [`zlib::Reader`] port Go's inflater,
//!   including its error values ([`Error`]).
//!
//! See `PORTING.md` for the Go file → module map, deviations and FMA sites.

// Lints that fight a faithful port of Go code.
#![allow(
    clippy::too_many_arguments,
    clippy::needless_range_loop,
    clippy::excessive_precision,
    clippy::collapsible_else_if,
    clippy::collapsible_if,
    clippy::identity_op,
    clippy::precedence,
    clippy::manual_range_contains,
    clippy::new_without_default,
    clippy::field_reassign_with_default,
    clippy::manual_memcpy,
    clippy::comparison_chain,
    clippy::neg_multiply,
    clippy::int_plus_one,
    clippy::manual_clamp,
    clippy::manual_is_multiple_of
)]

pub mod adler32;
mod error;
pub mod flate;
pub mod zlib;

pub use error::Error;

pub(crate) use flate::read_full_internal as flate_read_full;

/// One Go `io.Writer.Write(b)` call: `write_all` for non-empty `b` (a
/// Go-style writer consumes everything in one call), and a single `write`
/// for an empty `b` so that zero-length Go writes are forwarded too.
pub(crate) fn go_write<W: std::io::Write>(w: &mut W, b: &[u8]) -> std::io::Result<()> {
    if b.is_empty() {
        return w.write(b).map(|_| ());
    }
    w.write_all(b)
}
