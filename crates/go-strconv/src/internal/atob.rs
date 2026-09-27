// Port of go1.27.1 src/internal/strconv/atob.go.

use super::Error;

// Go: internal/strconv/atob.go:ParseBool
/// ParseBool returns the boolean value represented by the string.
/// It accepts 1, t, T, TRUE, true, True, 0, f, F, FALSE, false, False.
/// Any other value returns an error.
pub fn parse_bool(str: &[u8]) -> (bool, Option<Error>) {
    match str {
        b"1" | b"t" | b"T" | b"true" | b"TRUE" | b"True" => (true, None),
        b"0" | b"f" | b"F" | b"false" | b"FALSE" | b"False" => (false, None),
        _ => (false, Some(Error::Syntax)),
    }
}

// Go: internal/strconv/atob.go:FormatBool
/// FormatBool returns "true" or "false" according to the value of b.
pub fn format_bool(b: bool) -> &'static str {
    if b { "true" } else { "false" }
}

// Go: internal/strconv/atob.go:AppendBool
/// AppendBool appends "true" or "false", according to the value of b,
/// to dst.
pub fn append_bool(dst: &mut Vec<u8>, b: bool) {
    if b {
        dst.extend_from_slice(b"true");
    } else {
        dst.extend_from_slice(b"false");
    }
}
