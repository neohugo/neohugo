//! The subset of Go's `unicode/utf8` and `unicode` used by tdewolff/parse,
//! delegating to the shared `go-unicode` crate (go1.27.1, Unicode 17.0.0).

use crate::gobytes::{ByteView, view_to_vec};

pub use go_unicode::Rune;
pub use go_unicode::utf8::{MAX_RUNE, RUNE_ERROR, RUNE_SELF};

// Go: unicode/utf8/utf8.go:DecodeRune
/// Decodes the first rune of `p[off..]`; invalid encodings give
/// `(RuneError, 1)` and an empty input `(RuneError, 0)`.
pub fn decode_rune<B: ByteView + ?Sized>(p: &B, off: usize) -> (Rune, usize) {
    // DecodeRune never looks past 4 bytes (and `len(p) < size` is unchanged
    // by truncating to 4), so a 4-byte window is exact.
    let n = (p.len() - off).min(4);
    let mut tmp = [0u8; 4];
    for (k, slot) in tmp.iter_mut().enumerate().take(n) {
        *slot = p.at(off + k);
    }
    go_unicode::utf8::decode_rune(&tmp[..n])
}

// Go: unicode/utf8/utf8.go:RuneLen
pub fn rune_len(r: Rune) -> isize {
    go_unicode::utf8::rune_len(r)
}

// Go: unicode/utf8/utf8.go:AppendRune
/// Appends the UTF-8 encoding of `r` (invalid runes encode as U+FFFD).
pub fn append_rune(out: &mut Vec<u8>, r: Rune) {
    go_unicode::utf8::append_rune(out, r)
}

/// Go `[]rune(string(b))`.
pub fn to_runes<B: ByteView + ?Sized>(b: &B) -> Vec<Rune> {
    go_unicode::utf8::to_runes(&view_to_vec(b))
}

/// Go `string(rs)` for a `[]rune`.
pub fn runes_to_string(rs: &[Rune]) -> Vec<u8> {
    go_unicode::utf8::from_runes(rs)
}

// Go: unicode/graphic.go:IsGraphic
pub fn is_graphic(r: Rune) -> bool {
    go_unicode::is_graphic(r)
}
