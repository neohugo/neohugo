//! Port of `golang.org/x/text/runes` (x/text v0.26.0): `Set`, `In`, `NotIn`, `Predicate` and
//! `Remove`, as used by `common/text.RemoveAccents` (`runes.Remove(runes.In(unicode.Mn))`).
//!
//! The range tables are go-unicode's (Go 1.27.1's `unicode` package, Unicode 17.0.0), as in the
//! Go build: only `unicode/norm` carries x/text's own (15.0.0) data. Not ported (no caller):
//! `If`, `Map`, `ReplaceIllFormed` and the `Span` methods.

use go_unicode::{RangeTable, utf8};

use super::xtransform::{TransformError, Transformer};

/// Go: `runes.Set` — a collection of runes.
pub trait Set {
    /// Go: `Contains` — true if r is contained in the set.
    fn contains(&self, r: i32) -> bool;
}

/// Go: `runes.setFunc`.
pub struct SetFunc<F>(F);

impl<F: Fn(i32) -> bool> Set for SetFunc<F> {
    // Go: runes.go:setFunc.Contains
    fn contains(&self, r: i32) -> bool {
        (self.0)(r)
    }
}

/// Go: `runes.In(rt)` — a Set that contains the runes of the range table.
// Go: runes.go:In
pub fn in_table(rt: &'static RangeTable) -> SetFunc<impl Fn(i32) -> bool> {
    SetFunc(move |r| go_unicode::is(rt, r))
}

/// Go: `runes.NotIn(rt)` — a Set that contains the runes not in the range table.
// Go: runes.go:NotIn
pub fn not_in(rt: &'static RangeTable) -> SetFunc<impl Fn(i32) -> bool> {
    SetFunc(move |r| !go_unicode::is(rt, r))
}

/// Go: `runes.Predicate(f)`.
// Go: runes.go:Predicate
pub fn predicate<F: Fn(i32) -> bool>(f: F) -> SetFunc<F> {
    SetFunc(f)
}

/// Go: `runes.Remove(s)` — a Transformer that removes the runes of s. Illegal input bytes are
/// replaced by RuneError (U+FFFD) unless RuneError is in s.
// Go: runes.go:Remove
pub fn remove<S: Set>(s: S) -> Remove<S> {
    Remove(s)
}

/// Go: `runes.remove` (wrapped by `runes.Transformer`).
pub struct Remove<S>(S);

impl<S: Set> Transformer for Remove<S> {
    // Go: runes.go:remove.Reset
    fn reset(&mut self) {}

    // Go: runes.go:remove.Transform
    fn transform(
        &mut self,
        dst: &mut [u8],
        src: &[u8],
        at_eof: bool,
    ) -> (usize, usize, Option<TransformError>) {
        let (mut n_dst, mut n_src, mut err) = (0usize, 0usize, None);
        while n_src < src.len() {
            let (r, size);
            if (src[n_src] as i32) < utf8::RUNE_SELF {
                r = src[n_src] as i32;
                size = 1;
            } else {
                (r, size) = utf8::decode_rune(&src[n_src..]);
                if size == 1 {
                    // Invalid rune.
                    if !at_eof && !utf8::full_rune(&src[n_src..]) {
                        err = Some(TransformError::ShortSrc);
                        break;
                    }
                    // We replace illegal bytes with RuneError. Not doing so might
                    // otherwise turn a sequence of invalid UTF-8 into valid UTF-8.
                    // The resulting byte sequence may subsequently contain runes
                    // for which t(r) is true that were passed unnoticed.
                    if !self.0.contains(utf8::RUNE_ERROR) {
                        if n_dst + 3 > dst.len() {
                            err = Some(TransformError::ShortDst);
                            break;
                        }
                        dst[n_dst..n_dst + 3].copy_from_slice("\u{FFFD}".as_bytes());
                        n_dst += 3;
                    }
                    n_src += 1;
                    continue;
                }
            }
            if self.0.contains(r) {
                n_src += size;
                continue;
            }
            if n_dst + size > dst.len() {
                err = Some(TransformError::ShortDst);
                break;
            }
            dst[n_dst..n_dst + size].copy_from_slice(&src[n_src..n_src + size]);
            n_dst += size;
            n_src += size;
        }
        (n_dst, n_src, err)
    }
}
