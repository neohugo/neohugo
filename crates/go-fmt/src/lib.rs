//! Faithful port of Go's `fmt` printing functions as shipped in go1.27.1
//! (`src/fmt/print.go` + `src/fmt/format.go`), over [`go_value::Value`].
//!
//! Go strings are byte strings, so formats are `impl AsRef<[u8]>` and the
//! results are `Vec<u8>` (they are valid UTF-8 exactly when Go's would be).
//! Numbers go through `go-strconv` (Go's own float formatting and quoting,
//! including `IsPrint`), runes through `go-unicode`, and `time.Time` through
//! `go-time`.
//!
//! | Go | Rust |
//! |---|---|
//! | `fmt.Sprint(a...)` | [`sprint`] |
//! | `fmt.Sprintln(a...)` | [`sprintln`] |
//! | `fmt.Sprintf(format, a...)` | [`sprintf`] |
//! | `fmt.Append(b, a...)` / `Appendln` / `Appendf` | [`append`] / [`appendln`] / [`appendf`] |
//! | `fmt.Fprint(w, a...)` / `Fprintln` / `Fprintf` | [`fprint`] / [`fprintln`] / [`fprintf`] |
//! | `fmt.Errorf(format, a...).Error()` | [`errorf`] |
//!
//! See `PORTING.md` for how reflection maps onto the value model and for
//! the parts of `fmt` that the value model cannot express.

// Lints that fight a faithful line-by-line port.
#![allow(clippy::needless_range_loop)]
#![allow(clippy::too_many_arguments)]
#![allow(clippy::manual_range_contains)]
#![allow(clippy::absurd_extreme_comparisons)]

mod format;
mod print;

use std::io;

pub use go_value::Value;
pub use print::{NamedMethod, NilKind, named_method, register_named_method, typed_nil_kind};

use print::Pp;

// Go: fmt/print.go:Sprintf
/// Sprintf formats according to a format specifier and returns the resulting string.
pub fn sprintf(format: impl AsRef<[u8]>, a: &[Value]) -> Vec<u8> {
    let mut p = Pp::default();
    p.do_printf(format.as_ref(), a);
    p.fmt.buf
}

// Go: fmt/print.go:Appendf
/// Appendf formats according to a format specifier, appends the result to the byte
/// slice.
pub fn appendf(b: &mut Vec<u8>, format: impl AsRef<[u8]>, a: &[Value]) {
    let mut p = Pp::default();
    p.do_printf(format.as_ref(), a);
    b.extend_from_slice(&p.fmt.buf);
}

// Go: fmt/print.go:Fprintf
/// Fprintf formats according to a format specifier and writes to w.
/// It returns the number of bytes written and any write error encountered.
pub fn fprintf<W: io::Write + ?Sized>(
    w: &mut W,
    format: impl AsRef<[u8]>,
    a: &[Value],
) -> io::Result<usize> {
    let mut p = Pp::default();
    p.do_printf(format.as_ref(), a);
    w.write_all(&p.fmt.buf)?;
    Ok(p.fmt.buf.len())
}

// Go: fmt/print.go:Sprint
/// Sprint formats using the default formats for its operands and returns the resulting string.
/// Spaces are added between operands when neither is a string.
pub fn sprint(a: &[Value]) -> Vec<u8> {
    let mut p = Pp::default();
    p.do_print(a);
    p.fmt.buf
}

// Go: fmt/print.go:Append
/// Append formats using the default formats for its operands, appends the result to
/// the byte slice.
/// Spaces are added between operands when neither is a string.
pub fn append(b: &mut Vec<u8>, a: &[Value]) {
    let mut p = Pp::default();
    p.do_print(a);
    b.extend_from_slice(&p.fmt.buf);
}

// Go: fmt/print.go:Fprint
/// Fprint formats using the default formats for its operands and writes to w.
/// Spaces are added between operands when neither is a string.
/// It returns the number of bytes written and any write error encountered.
pub fn fprint<W: io::Write + ?Sized>(w: &mut W, a: &[Value]) -> io::Result<usize> {
    let mut p = Pp::default();
    p.do_print(a);
    w.write_all(&p.fmt.buf)?;
    Ok(p.fmt.buf.len())
}

// Go: fmt/print.go:Sprintln
/// Sprintln formats using the default formats for its operands and returns the resulting string.
/// Spaces are always added between operands and a newline is appended.
pub fn sprintln(a: &[Value]) -> Vec<u8> {
    let mut p = Pp::default();
    p.do_println(a);
    p.fmt.buf
}

// Go: fmt/print.go:Appendln
/// Appendln formats using the default formats for its operands, appends the result
/// to the byte slice. Spaces are always added
/// between operands and a newline is appended.
pub fn appendln(b: &mut Vec<u8>, a: &[Value]) {
    let mut p = Pp::default();
    p.do_println(a);
    b.extend_from_slice(&p.fmt.buf);
}

// Go: fmt/print.go:Fprintln
/// Fprintln formats using the default formats for its operands and writes to w.
/// Spaces are always added between operands and a newline is appended.
/// It returns the number of bytes written and any write error encountered.
pub fn fprintln<W: io::Write + ?Sized>(w: &mut W, a: &[Value]) -> io::Result<usize> {
    let mut p = Pp::default();
    p.do_println(a);
    w.write_all(&p.fmt.buf)?;
    Ok(p.fmt.buf.len())
}

// Go: fmt/errors.go:Errorf
/// The message of `fmt.Errorf(format, a...)` (its `Error()` text) and the
/// indexes of the arguments wrapped by `%w` (sorted and deduplicated, as in
/// Go). A `%w` operand must be an error (an `Object` with `go_error`, or a
/// value of a type registered with [`NamedMethod::Error`]); anything else
/// prints `%!w(...)`.
pub fn errorf(format: impl AsRef<[u8]>, a: &[Value]) -> (Vec<u8>, Vec<usize>) {
    let mut p = Pp::default();
    p.wrap_errs = true;
    p.do_printf(format.as_ref(), a);
    let s = std::mem::take(&mut p.fmt.buf);
    let mut wrapped: Vec<usize> = Vec::new();
    match p.wrapped_errs.len() {
        0 => {}
        1 => {
            // w.err, _ = a[p.wrappedErrs[0]].(error)
            if a.get(p.wrapped_errs[0]).is_some_and(print::is_error) {
                wrapped.push(p.wrapped_errs[0]);
            }
        }
        _ => {
            if p.reordered {
                p.wrapped_errs.sort_unstable();
            }
            let mut last: Option<usize> = None;
            for &argnum in &p.wrapped_errs {
                if Some(argnum) == last {
                    continue;
                }
                last = Some(argnum);
                if a.get(argnum).is_some_and(print::is_error) {
                    wrapped.push(argnum);
                }
            }
        }
    }
    (s, wrapped)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parsenum_table() {
        // Go: fmt_test.go:TestParsenum
        let cases: &[(&str, usize, usize, i64, bool, usize)] = &[
            ("a123", 0, 4, 0, false, 0),
            ("1234", 1, 1, 0, false, 1),
            ("123a", 0, 4, 123, true, 3),
            ("12a3", 0, 4, 12, true, 2),
            ("1234", 0, 4, 1234, true, 4),
            ("1a234", 1, 3, 0, false, 1),
        ];
        for &(s, start, end, num, isnum, newi) in cases {
            let got = print::parsenum(s.as_bytes(), start, end);
            assert_eq!(got, (num, isnum, newi), "parsenum({s:?}, {start}, {end})");
        }
    }
}
