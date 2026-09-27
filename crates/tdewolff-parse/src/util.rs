//! Go: parse/util.go

use std::io::Write;

use crate::gobytes::{ByteView, GoBytes};
use crate::utf8::is_graphic;

// Go: parse/util.go:Copy
/// Returns a copy of the given byte slice (`cap == len`).
pub fn copy(src: &GoBytes) -> GoBytes {
    let dst = GoBytes::make(src.len(), src.len());
    dst.copy_from(src);
    dst
}

// Go: parse/util.go:ToLower
/// Converts all characters in the byte slice from A-Z to a-z, in place.
pub fn to_lower(src: GoBytes) -> GoBytes {
    for i in 0..src.len() {
        let c = src.at(i);
        if c.is_ascii_uppercase() {
            src.set(i, c + (b'a' - b'A'));
        }
    }
    src
}

/// [`to_lower`] on a Rust slice.
pub fn to_lower_slice(src: &mut [u8]) {
    for c in src.iter_mut() {
        if c.is_ascii_uppercase() {
            *c += b'a' - b'A';
        }
    }
}

// Go: parse/util.go:EqualFold
/// Returns true when `s` matches case-insensitively `target_lower` (which
/// must be lowercase).
pub fn equal_fold<A: ByteView + ?Sized, B: ByteView + ?Sized>(s: &A, target_lower: &B) -> bool {
    if s.len() != target_lower.len() {
        return false;
    }
    for i in 0..target_lower.len() {
        let c = target_lower.at(i);
        let d = s.at(i);
        if d != c && (!d.is_ascii_uppercase() || d.wrapping_add(b'a' - b'A') != c) {
            return false;
        }
    }
    true
}

// Go: parse/util.go:Printable
/// Returns a printable string for given rune.
pub fn printable(r: i32) -> Vec<u8> {
    if is_graphic(r) {
        let mut out = Vec::new();
        crate::utf8::append_rune(&mut out, r); // fmt "%c"
        out
    } else if r < 128 {
        // fmt "0x%02X" prints negative values with a sign ("0x-1")
        if r < 0 {
            format!("0x-{:X}", -(r as i64)).into_bytes()
        } else {
            format!("0x{:02X}", r).into_bytes()
        }
    } else {
        format_u(r).into_bytes()
    }
}

/// Go `fmt.Sprintf("%U", r)`.
pub(crate) fn format_u(r: i32) -> String {
    // fmt converts the rune to uint64 (sign-extended) before printing.
    format!("U+{:04X}", (r as i64) as u64)
}

static WHITESPACE_TABLE: [bool; 256] = {
    let mut t = [false; 256];
    t[b'\t' as usize] = true;
    t[b'\n' as usize] = true;
    t[0x0C] = true; // form feed
    t[b'\r' as usize] = true;
    t[b' ' as usize] = true;
    t
};

// Go: parse/util.go:IsWhitespace
/// Returns true for space, \n, \r, \t, \f.
#[inline]
pub fn is_whitespace(c: u8) -> bool {
    WHITESPACE_TABLE[c as usize]
}

static NEWLINE_TABLE: [bool; 256] = {
    let mut t = [false; 256];
    t[b'\n' as usize] = true;
    t[b'\r' as usize] = true;
    t
};

// Go: parse/util.go:IsNewline
/// Returns true for \n, \r.
#[inline]
pub fn is_newline(c: u8) -> bool {
    NEWLINE_TABLE[c as usize]
}

// Go: parse/util.go:IsAllWhitespace
/// Returns true when the entire byte slice consists of space, \n, \r, \t, \f.
pub fn is_all_whitespace<B: ByteView + ?Sized>(b: &B) -> bool {
    for i in 0..b.len() {
        if !is_whitespace(b.at(i)) {
            return false;
        }
    }
    true
}

// Go: parse/util.go:TrimWhitespace
/// Removes any leading and trailing whitespace characters.
pub fn trim_whitespace(b: &GoBytes) -> GoBytes {
    let (start, end) = trim_whitespace_bounds(b);
    b.slice(start, end)
}

/// The `(start, end)` bounds that [`trim_whitespace`] slices to.
pub fn trim_whitespace_bounds<B: ByteView + ?Sized>(b: &B) -> (usize, usize) {
    let n = b.len();
    let mut start = n;
    for i in 0..n {
        if !is_whitespace(b.at(i)) {
            start = i;
            break;
        }
    }
    let mut end = n;
    let mut i = n as isize - 1;
    while i >= start as isize {
        if !is_whitespace(b.at(i as usize)) {
            end = i as usize + 1;
            break;
        }
        i -= 1;
    }
    (start, end)
}

/// Go: parse/util.go:Indenter — an `io.Writer` that indents every line after
/// a newline by `n` spaces.
pub struct Indenter<W: Write> {
    pub writer: W,
    b: Vec<u8>,
}

// Go: parse/util.go:NewIndenter
impl<W: Write> Indenter<W> {
    pub fn new(w: W, n: usize) -> Indenter<W> {
        Indenter {
            writer: w,
            b: vec![b' '; n],
        }
    }

    /// `NewIndenter(w, n)` where `w` is itself an `Indenter`: indentation adds up.
    pub fn nested(w: Indenter<W>, n: usize) -> Indenter<W> {
        let total = n + w.b.len();
        Indenter::new(w.writer, total)
    }

    // Go: parse/util.go:Indenter.Indent
    pub fn indent(&self) -> usize {
        self.b.len()
    }
}

impl<W: Write> Indenter<W> {
    // Go: parse/util.go:Indenter.Write
    /// Go-faithful `Write`: the returned count includes the indentation bytes.
    pub fn write_go(&mut self, b: &[u8]) -> (usize, Option<std::io::Error>) {
        let (mut n, mut j) = (0usize, 0usize);
        for (i, &c) in b.iter().enumerate() {
            if c == b'\n' {
                let m = self.writer.write(&b[j..i + 1]).unwrap_or(0);
                n += m;
                let m = self.writer.write(&self.b).unwrap_or(0);
                n += m;
                j = i + 1;
            }
        }
        match self.writer.write(&b[j..]) {
            Ok(m) => (n + m, None),
            Err(e) => (n, Some(e)),
        }
    }
}

impl<W: Write> Write for Indenter<W> {
    fn write(&mut self, b: &[u8]) -> std::io::Result<usize> {
        match self.write_go(b) {
            (_, None) => Ok(b.len()),
            (_, Some(e)) => Err(e),
        }
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.writer.flush()
    }
}
