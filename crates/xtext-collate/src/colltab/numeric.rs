//! Port of golang.org/x/text/internal/colltab/numeric.go.
//!
//! Go's `numberConverter` works on slices that share one backing array; the
//! port keeps a single `Vec<Elem>` plus the logical length of `nc.elems`, and
//! every "append to nc.elems" first truncates the vector back to that length
//! (exactly what appending to the shorter Go slice does to the shared array).

use super::collelem::{DEFAULT_SECONDARY, DEFAULT_TERTIARY, Elem, make_elem};
use super::weighter::Weighter;
use crate::goutf8::{decode_rune, is_nd};

// Go: internal/colltab/numeric.go:NewNumericWeighter
/// Wraps `w` to replace individual digits to sort based on their numeric
/// value.
pub fn new_numeric_weighter(w: Box<dyn Weighter>) -> NumericWeighter {
    let get_elem = |s: &str| -> Elem {
        let mut elems = Vec::new();
        w.append_next(&mut elems, s.as_bytes());
        elems[0]
    };
    let nine = get_elem("9");

    // Numbers should order before zero, but the DUCET has no room for this.
    // TODO: move before zero once we use fractional collation elements.
    let ns = make_elem(
        nine.primary() + 1,
        nine.secondary(),
        nine.tertiary() as i32,
        0,
    )
    .unwrap_or(Elem(0));

    NumericWeighter {
        zero: get_elem("0"),
        zero_special_lo: get_elem("\u{FF10}"), // U+FF10 FULLWIDTH DIGIT ZERO
        zero_special_hi: get_elem("\u{2080}"), // U+2080 SUBSCRIPT ZERO
        nine,
        nine_special_hi: get_elem("\u{2089}"), // U+2089 SUBSCRIPT NINE
        number_start: ns,
        weighter: w,
    }
}

/// A numericWeighter translates a stream of digits into a stream of weights
/// representing the numeric value.
pub struct NumericWeighter {
    weighter: Box<dyn Weighter>,

    zero: Elem,            // normal digit zero
    zero_special_lo: Elem, // special digit zero, low tertiary value
    zero_special_hi: Elem, // special digit zero, high tertiary value
    nine: Elem,            // normal digit nine
    nine_special_hi: Elem, // special digit nine
    number_start: Elem,
}

impl Weighter for NumericWeighter {
    // Go: internal/colltab/numeric.go:numericWeighter.AppendNext / AppendNextString
    /// Calls the namesake of the underlying weigher, but replaces single
    /// digits with weights representing their value.
    fn append_next(&self, buf: &mut Vec<Elem>, s: &[u8]) -> usize {
        let buf_len = buf.len();
        let mut n = self.weighter.append_next(buf, s);
        let mut nc = NumberConverter {
            w: self,
            elems_len: buf_len,
            n_digits: 0,
            len_index: 0,
            b: s,
        };
        let (is_zero, ok) = nc.check_next_digit(buf);
        if !ok {
            return n;
        }
        // ce might have been grown already, so take it instead of buf.
        nc.init(buf, buf_len, is_zero);
        while n < s.len() {
            buf.truncate(nc.elems_len);
            let sz = self.weighter.append_next(buf, &s[n..]);
            nc.b = s;
            n += sz;
            if !nc.update(buf) {
                break;
            }
        }
        nc.result(buf);
        n
    }

    fn top(&self) -> u32 {
        self.weighter.top()
    }
}

struct NumberConverter<'a> {
    w: &'a NumericWeighter,

    // len(nc.elems)
    elems_len: usize,
    n_digits: i32,
    len_index: usize,

    // The input (Go keeps either s or b); always the full slice passed to
    // AppendNext, never re-sliced.
    b: &'a [u8],
}

// We currently support a maximum of about 2M digits (the number of primary
// values).
const MAX_DIGITS: i32 = (1 << 21) - 1;

impl NumberConverter<'_> {
    // Go: internal/colltab/numeric.go:numberConverter.init
    /// Completes initialization and prepares it for adding more digits.
    /// `elems` is assumed to have a digit starting at old_len.
    fn init(&mut self, elems: &mut Vec<Elem>, old_len: usize, is_zero: bool) {
        // Insert a marker indicating the start of a number and a placeholder
        // for the number of digits.
        if is_zero {
            // elems = append(elems[:oldLen], nc.w.numberStart, 0)
            elems.truncate(old_len);
            elems.push(self.w.number_start);
            elems.push(Elem(0));
        } else {
            elems.splice(old_len..old_len, [self.w.number_start, Elem(0)]);
            self.n_digits = 1;
        }
        self.elems_len = elems.len();
        self.len_index = old_len + 1;
    }

    // Go: internal/colltab/numeric.go:numberConverter.checkNextDigit
    /// Reports whether bufNew adds a single digit relative to the old buffer.
    /// If it does, it also reports whether this digit is zero.
    fn check_next_digit(&self, buf_new: &[Elem]) -> (bool, bool) {
        if self.elems_len >= buf_new.len() {
            return (false, false);
        }
        let e = buf_new[self.elems_len];
        if e < self.w.zero_special_lo || self.w.nine < e {
            // Not a number.
            return (false, false);
        }
        let is_zero;
        if e < self.w.zero {
            if e > self.w.nine_special_hi {
                // Not a number.
                return (false, false);
            }
            if !self.is_digit() {
                return (false, false);
            }
            is_zero = e <= self.w.zero_special_hi;
        } else {
            // This is the common case if we encounter a digit.
            is_zero = e == self.w.zero;
        }
        // Test the remaining added collation elements have a zero primary value.
        let n = buf_new.len() - self.elems_len;
        if n > 1 {
            for &x in &buf_new[self.elems_len + 1..] {
                if x.primary() != 0 {
                    return (false, false);
                }
            }
            // In some rare cases, collation elements will encode runes in
            // unicode.No as a digit. For example Ethiopic digits (U+1369 -
            // U+1371) are not in Nd.
            if !self.is_digit() {
                return (false, false);
            }
        }
        (is_zero, true)
    }

    // Go: internal/colltab/numeric.go:numberConverter.isDigit
    /// Note: like Go, this decodes the first rune of the whole input passed
    /// to AppendNext, not of the current digit.
    fn is_digit(&self) -> bool {
        let (r, _) = decode_rune(self.b);
        is_nd(r)
    }

    // Go: internal/colltab/numeric.go:numberConverter.update
    fn update(&mut self, elems: &[Elem]) -> bool {
        let (is_zero, ok) = self.check_next_digit(elems);
        if self.n_digits == 0 && is_zero {
            return true;
        }
        self.elems_len = elems.len();
        if !ok {
            return false;
        }
        self.n_digits += 1;
        self.n_digits < MAX_DIGITS
    }

    // Go: internal/colltab/numeric.go:numberConverter.result
    /// Fills in the length element for the digit sequence and returns the
    /// completed collation elements (truncating `elems` to nc.elems).
    fn result(&self, elems: &mut Vec<Elem>) {
        let e = make_elem(self.n_digits, DEFAULT_SECONDARY, DEFAULT_TERTIARY, 0).unwrap_or(Elem(0));
        elems.truncate(self.elems_len);
        elems[self.len_index] = e;
    }
}
