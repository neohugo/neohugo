//! Go's `unicode` package (go1.27.1, Unicode 17.0.0), plus `unicode/utf8`,
//! `unicode/utf16` and the Unicode-aware parts of `strings` and `bytes`, all
//! operating on Go semantics: runes are `i32`, text is `&[u8]` that may hold
//! invalid UTF-8, and every table is generated from the Go sources by
//! `tools/go-oracle/go-unicode`.
//!
//! Layout:
//! - crate root: `unicode` (letter.go, graphic.go, digit.go, casetables.go,
//!   and tables.go via the generated `tables` module, re-exported here).
//! - `unicode/utf8` is the [`utf8`] module.
//! - `unicode/utf16` is the [`utf16`] module.
//! - `strings` functions over `&[u8]` (Go `string` semantics) are in [`strings`].
//! - `bytes` functions over `&[u8]` (Go `[]byte` semantics) are in [`bytes`].
//! - `strings.Replacer` is [`replacer::Replacer`].

// A faithful port keeps Go's index loops, argument lists, `x%n == 0` tests and
// branch structure (e.g. utf8.FullRune's two identical `return true` arms).
#![allow(
    clippy::needless_range_loop,
    clippy::too_many_arguments,
    clippy::manual_is_multiple_of,
    clippy::if_same_then_else
)]

pub mod bytes;
pub mod replacer;
pub mod strings;
#[rustfmt::skip]
pub mod tables;
pub mod utf16;
pub mod utf8;

pub use tables::*;

/// Go `rune` (an alias for `int32`).
pub type Rune = i32;

// Go: src/unicode/letter.go

/// Maximum valid Unicode code point.
pub const MAX_RUNE: Rune = 0x10FFFF;
/// Represents invalid code points.
pub const REPLACEMENT_CHAR: Rune = 0xFFFD;
/// Maximum ASCII value.
pub const MAX_ASCII: Rune = 0x7F;
/// Maximum Latin-1 value.
pub const MAX_LATIN1: Rune = 0xFF;

/// RangeTable defines a set of Unicode code points by listing the ranges of
/// code points within the set. The ranges are listed in two slices
/// to save space: a slice of 16-bit ranges and a slice of 32-bit ranges.
/// The two slices must be in sorted order and non-overlapping.
/// Also, R32 should contain only values >= 0x10000 (1<<16).
///
/// The slices are `'static` so tables can be `static` items like Go's
/// package-level variables (see PORTING.md).
#[derive(Debug, PartialEq, Eq, Hash)]
pub struct RangeTable {
    pub r16: &'static [Range16],
    pub r32: &'static [Range32],
    /// number of entries in R16 with Hi <= MaxLatin1
    pub latin_offset: usize,
}

/// Range16 represents of a range of 16-bit Unicode code points. The range runs from Lo to Hi
/// inclusive and has the specified stride.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Range16 {
    pub lo: u16,
    pub hi: u16,
    pub stride: u16,
}

/// Range32 represents of a range of Unicode code points and is used when one or
/// more of the values will not fit in 16 bits. The range runs from Lo to Hi
/// inclusive and has the specified stride. Lo and Hi must always be >= 1<<16.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Range32 {
    pub lo: u32,
    pub hi: u32,
    pub stride: u32,
}

/// CaseRange represents a range of Unicode code points for simple (one
/// code point to one code point) case conversion.
/// The range runs from Lo to Hi inclusive, with a fixed stride of 1. Deltas
/// are the number to add to the code point to reach the code point for a
/// different case for that character. They may be negative. If zero, it
/// means the character is in the corresponding case. There is a special
/// case representing sequences of alternating corresponding Upper and Lower
/// pairs. It appears with a fixed Delta of
///
/// ```text
/// {UpperLower, UpperLower, UpperLower}
/// ```
///
/// The constant UpperLower has an otherwise impossible delta value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CaseRange {
    pub lo: u32,
    pub hi: u32,
    pub delta: [Rune; MAX_CASE as usize],
}

/// SpecialCase represents language-specific case mappings such as Turkish.
/// Methods of SpecialCase customize (by overriding) the standard mappings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SpecialCase<'a>(pub &'a [CaseRange]);

// Indices into the Delta arrays inside CaseRanges for case mapping.
// (Go untyped constants used as `int`.)
pub const UPPER_CASE: i64 = 0;
pub const LOWER_CASE: i64 = 1;
pub const TITLE_CASE: i64 = 2;
pub const MAX_CASE: i64 = 3;

/// If the Delta field of a [`CaseRange`] is UpperLower, it means
/// this CaseRange represents a sequence of the form (say)
/// `[Upper] [Lower] [Upper] [Lower]`.
pub const UPPER_LOWER: Rune = MAX_RUNE + 1; // (Cannot be a valid delta.)

/// linearMax is the maximum size table for linear search for non-Latin1 rune.
/// Derived by running 'go test -calibrate'.
const LINEAR_MAX: usize = 18;

// Go: src/unicode/letter.go:is16
/// is16 reports whether r is in the sorted slice of 16-bit ranges.
fn is16(ranges: &[Range16], r: u16) -> bool {
    if ranges.len() <= LINEAR_MAX || r <= MAX_LATIN1 as u16 {
        for range_ in ranges {
            if r < range_.lo {
                return false;
            }
            if r <= range_.hi {
                return range_.stride == 1 || (r - range_.lo) % range_.stride == 0;
            }
        }
        return false;
    }

    // binary search over ranges
    let mut lo = 0usize;
    let mut hi = ranges.len();
    while lo < hi {
        let m = (lo + hi) >> 1;
        let range_ = &ranges[m];
        if range_.lo <= r && r <= range_.hi {
            return range_.stride == 1 || (r - range_.lo) % range_.stride == 0;
        }
        if r < range_.lo {
            hi = m;
        } else {
            lo = m + 1;
        }
    }
    false
}

// Go: src/unicode/letter.go:is32
/// is32 reports whether r is in the sorted slice of 32-bit ranges.
fn is32(ranges: &[Range32], r: u32) -> bool {
    if ranges.len() <= LINEAR_MAX {
        for range_ in ranges {
            if r < range_.lo {
                return false;
            }
            if r <= range_.hi {
                return range_.stride == 1 || (r - range_.lo) % range_.stride == 0;
            }
        }
        return false;
    }

    // binary search over ranges
    let mut lo = 0usize;
    let mut hi = ranges.len();
    while lo < hi {
        let m = (lo + hi) >> 1;
        let range_ = ranges[m];
        if range_.lo <= r && r <= range_.hi {
            return range_.stride == 1 || (r - range_.lo) % range_.stride == 0;
        }
        if r < range_.lo {
            hi = m;
        } else {
            lo = m + 1;
        }
    }
    false
}

// Go: src/unicode/letter.go:Is
/// Is reports whether the rune is in the specified table of ranges.
pub fn is(range_tab: &RangeTable, r: Rune) -> bool {
    let r16 = range_tab.r16;
    // Compare as uint32 to correctly handle negative runes.
    if !r16.is_empty() && (r as u32) <= r16[r16.len() - 1].hi as u32 {
        return is16(r16, r as u16);
    }
    let r32 = range_tab.r32;
    if !r32.is_empty() && r >= r32[0].lo as Rune {
        return is32(r32, r as u32);
    }
    false
}

// Go: src/unicode/letter.go:isExcludingLatin
fn is_excluding_latin(range_tab: &RangeTable, r: Rune) -> bool {
    let r16 = range_tab.r16;
    // Compare as uint32 to correctly handle negative runes.
    let off = range_tab.latin_offset;
    if r16.len() > off && (r as u32) <= r16[r16.len() - 1].hi as u32 {
        return is16(&r16[off..], r as u16);
    }
    let r32 = range_tab.r32;
    if !r32.is_empty() && r >= r32[0].lo as Rune {
        return is32(r32, r as u32);
    }
    false
}

// Go: src/unicode/letter.go:IsUpper
/// IsUpper reports whether the rune is an upper case letter.
pub fn is_upper(r: Rune) -> bool {
    // See comment in IsGraphic.
    if r as u32 <= MAX_LATIN1 as u32 {
        return LATIN1_PROPERTIES[r as u8 as usize] & P_LMASK == P_LU;
    }
    is_excluding_latin(UPPER, r)
}

// Go: src/unicode/letter.go:IsLower
/// IsLower reports whether the rune is a lower case letter.
pub fn is_lower(r: Rune) -> bool {
    // See comment in IsGraphic.
    if r as u32 <= MAX_LATIN1 as u32 {
        return LATIN1_PROPERTIES[r as u8 as usize] & P_LMASK == P_LL;
    }
    is_excluding_latin(LOWER, r)
}

// Go: src/unicode/letter.go:IsTitle
/// IsTitle reports whether the rune is a title case letter.
pub fn is_title(r: Rune) -> bool {
    if r <= MAX_LATIN1 {
        return false;
    }
    is_excluding_latin(TITLE, r)
}

// Go: src/unicode/letter.go:lookupCaseRange
/// lookupCaseRange returns the CaseRange mapping for rune r or nil if no
/// mapping exists for r.
fn lookup_case_range(r: Rune, case_range: &[CaseRange]) -> Option<&CaseRange> {
    // binary search over ranges
    let mut lo = 0usize;
    let mut hi = case_range.len();
    while lo < hi {
        let m = (lo + hi) >> 1;
        let cr = &case_range[m];
        if cr.lo as Rune <= r && r <= cr.hi as Rune {
            return Some(cr);
        }
        if r < cr.lo as Rune {
            hi = m;
        } else {
            lo = m + 1;
        }
    }
    None
}

// Go: src/unicode/letter.go:convertCase
/// convertCase converts r to _case using CaseRange cr.
fn convert_case(case: i64, r: Rune, cr: &CaseRange) -> Rune {
    let delta = cr.delta[case as usize];
    if delta > MAX_RUNE {
        // In an Upper-Lower sequence, which always starts with
        // an UpperCase letter, the real deltas always look like:
        //	{0, 1, 0}    UpperCase (Lower is next)
        //	{-1, 0, -1}  LowerCase (Upper, Title are previous)
        // The characters at even offsets from the beginning of the
        // sequence are upper case; the ones at odd offsets are lower.
        // The correct mapping can be done by clearing or setting the low
        // bit in the sequence offset.
        // The constants UpperCase and TitleCase are even while LowerCase
        // is odd so we take the low bit from _case.
        let lo = cr.lo as Rune;
        return lo.wrapping_add((r.wrapping_sub(lo) & !1) | (case & 1) as Rune);
    }
    r.wrapping_add(delta)
}

// Go: src/unicode/letter.go:to
/// to maps the rune using the specified case mapping.
/// It additionally reports whether caseRange contained a mapping for r.
fn to_case(case: i64, r: Rune, case_range: &[CaseRange]) -> (Rune, bool) {
    if !(0..MAX_CASE).contains(&case) {
        return (REPLACEMENT_CHAR, false); // as reasonable an error as any
    }
    if let Some(cr) = lookup_case_range(r, case_range) {
        return (convert_case(case, r, cr), true);
    }
    (r, false)
}

// Go: src/unicode/letter.go:To
/// To maps the rune to the specified case: [`UPPER_CASE`], [`LOWER_CASE`], or [`TITLE_CASE`].
pub fn to(case: i64, r: Rune) -> Rune {
    to_case(case, r, CASE_RANGES).0
}

// Go: src/unicode/letter.go:ToUpper
/// ToUpper maps the rune to upper case.
pub fn to_upper(mut r: Rune) -> Rune {
    if r <= MAX_ASCII {
        if 'a' as Rune <= r && r <= 'z' as Rune {
            r -= 'a' as Rune - 'A' as Rune;
        }
        return r;
    }
    to(UPPER_CASE, r)
}

// Go: src/unicode/letter.go:ToLower
/// ToLower maps the rune to lower case.
pub fn to_lower(mut r: Rune) -> Rune {
    if r <= MAX_ASCII {
        if 'A' as Rune <= r && r <= 'Z' as Rune {
            r += 'a' as Rune - 'A' as Rune;
        }
        return r;
    }
    to(LOWER_CASE, r)
}

// Go: src/unicode/letter.go:ToTitle
/// ToTitle maps the rune to title case.
pub fn to_title(mut r: Rune) -> Rune {
    if r <= MAX_ASCII {
        if 'a' as Rune <= r && r <= 'z' as Rune {
            // title case is upper case for ASCII
            r -= 'a' as Rune - 'A' as Rune;
        }
        return r;
    }
    to(TITLE_CASE, r)
}

impl SpecialCase<'_> {
    // Go: src/unicode/letter.go:SpecialCase.ToUpper
    /// ToUpper maps the rune to upper case giving priority to the special mapping.
    pub fn to_upper(&self, r: Rune) -> Rune {
        let (mut r1, had_mapping) = to_case(UPPER_CASE, r, self.0);
        if r1 == r && !had_mapping {
            r1 = to_upper(r);
        }
        r1
    }

    // Go: src/unicode/letter.go:SpecialCase.ToTitle
    /// ToTitle maps the rune to title case giving priority to the special mapping.
    pub fn to_title(&self, r: Rune) -> Rune {
        let (mut r1, had_mapping) = to_case(TITLE_CASE, r, self.0);
        if r1 == r && !had_mapping {
            r1 = to_title(r);
        }
        r1
    }

    // Go: src/unicode/letter.go:SpecialCase.ToLower
    /// ToLower maps the rune to lower case giving priority to the special mapping.
    pub fn to_lower(&self, r: Rune) -> Rune {
        let (mut r1, had_mapping) = to_case(LOWER_CASE, r, self.0);
        if r1 == r && !had_mapping {
            r1 = to_lower(r);
        }
        r1
    }
}

/// caseOrbit is defined in tables.go as []foldPair. Right now all the
/// entries fit in uint16, so use uint16.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FoldPair {
    pub from: u16,
    pub to: u16,
}

// Go: src/unicode/letter.go:SimpleFold
/// SimpleFold iterates over Unicode code points equivalent under
/// the Unicode-defined simple case folding. Among the code points
/// equivalent to rune (including rune itself), SimpleFold returns the
/// smallest rune > r if one exists, or else the smallest rune >= 0.
/// If r is not a valid Unicode code point, SimpleFold(r) returns r.
pub fn simple_fold(r: Rune) -> Rune {
    if !(0..=MAX_RUNE).contains(&r) {
        return r;
    }

    if (r as usize) < ASCII_FOLD.len() {
        return ASCII_FOLD[r as usize] as Rune;
    }

    // Consult caseOrbit table for special cases.
    let mut lo = 0usize;
    let mut hi = CASE_ORBIT.len();
    while lo < hi {
        let m = (lo + hi) >> 1;
        if (CASE_ORBIT[m].from as Rune) < r {
            lo = m + 1;
        } else {
            hi = m;
        }
    }
    if lo < CASE_ORBIT.len() && CASE_ORBIT[lo].from as Rune == r {
        return CASE_ORBIT[lo].to as Rune;
    }

    // No folding specified. This is a one- or two-element
    // equivalence class containing rune and ToLower(rune)
    // and ToUpper(rune) if they are different from rune.
    if let Some(cr) = lookup_case_range(r, CASE_RANGES) {
        let l = convert_case(LOWER_CASE, r, cr);
        if l != r {
            return l;
        }
        return convert_case(UPPER_CASE, r, cr);
    }
    r
}

// Go: src/unicode/graphic.go

// Bit masks for each code point under U+0100, for fast lookup.
pub(crate) const P_C: u8 = 1 << 0; // a control character.
pub(crate) const P_P: u8 = 1 << 1; // a punctuation character.
pub(crate) const P_N: u8 = 1 << 2; // a numeral.
pub(crate) const P_S: u8 = 1 << 3; // a symbolic character.
pub(crate) const P_Z: u8 = 1 << 4; // a spacing character.
pub(crate) const P_LU: u8 = 1 << 5; // an upper-case letter.
pub(crate) const P_LL: u8 = 1 << 6; // a lower-case letter.
pub(crate) const P_PRINT: u8 = 1 << 7; // Go `pp`: a printable character according to Go's definition.
pub(crate) const P_G: u8 = P_PRINT | P_Z; // a graphical character according to the Unicode definition.
pub(crate) const P_LO: u8 = P_LL | P_LU; // a letter that is neither upper nor lower case.
pub(crate) const P_LMASK: u8 = P_LL | P_LU;

/// GraphicRanges defines the set of graphic characters according to Unicode.
pub static GRAPHIC_RANGES: &[&RangeTable] = &[
    &tables::TAB_L,
    &tables::TAB_M,
    &tables::TAB_N,
    &tables::TAB_P,
    &tables::TAB_S,
    &tables::TAB_ZS,
];

/// PrintRanges defines the set of printable characters according to Go.
/// ASCII space, U+0020, is handled separately.
pub static PRINT_RANGES: &[&RangeTable] = &[
    &tables::TAB_L,
    &tables::TAB_M,
    &tables::TAB_N,
    &tables::TAB_P,
    &tables::TAB_S,
];

// Go: src/unicode/graphic.go:IsGraphic
/// IsGraphic reports whether the rune is defined as a Graphic by Unicode.
/// Such characters include letters, marks, numbers, punctuation, symbols, and
/// spaces, from categories L, M, N, P, S, Zs.
pub fn is_graphic(r: Rune) -> bool {
    // We convert to uint32 to avoid the extra test for negative,
    // and in the index we convert to uint8 to avoid the range check.
    if r as u32 <= MAX_LATIN1 as u32 {
        return LATIN1_PROPERTIES[r as u8 as usize] & P_G != 0;
    }
    r#in(r, GRAPHIC_RANGES)
}

// Go: src/unicode/graphic.go:IsPrint
/// IsPrint reports whether the rune is defined as printable by Go. Such
/// characters include letters, marks, numbers, punctuation, symbols, and the
/// ASCII space character, from categories L, M, N, P, S and the ASCII space
/// character. This categorization is the same as [`is_graphic`] except that the
/// only spacing character is ASCII space, U+0020.
pub fn is_print(r: Rune) -> bool {
    if r as u32 <= MAX_LATIN1 as u32 {
        return LATIN1_PROPERTIES[r as u8 as usize] & P_PRINT != 0;
    }
    r#in(r, PRINT_RANGES)
}

// Go: src/unicode/graphic.go:IsOneOf
/// IsOneOf reports whether the rune is a member of one of the ranges.
/// The function "In" provides a nicer signature and should be used in preference to IsOneOf.
pub fn is_one_of(ranges: &[&RangeTable], r: Rune) -> bool {
    for inside in ranges {
        if is(inside, r) {
            return true;
        }
    }
    false
}

// Go: src/unicode/graphic.go:In
/// In reports whether the rune is a member of one of the ranges.
/// (Go `unicode.In`; `in` is a Rust keyword, call it as `go_unicode::r#in`.)
pub fn r#in(r: Rune, ranges: &[&RangeTable]) -> bool {
    for inside in ranges {
        if is(inside, r) {
            return true;
        }
    }
    false
}

// Go: src/unicode/graphic.go:IsControl
/// IsControl reports whether the rune is a control character.
/// The C (Other) Unicode category includes more code points
/// such as surrogates; use Is(C, r) to test for them.
pub fn is_control(r: Rune) -> bool {
    if r as u32 <= MAX_LATIN1 as u32 {
        return LATIN1_PROPERTIES[r as u8 as usize] & P_C != 0;
    }
    // All control characters are < MaxLatin1.
    false
}

// Go: src/unicode/graphic.go:IsLetter
/// IsLetter reports whether the rune is a letter (category L).
pub fn is_letter(r: Rune) -> bool {
    if r as u32 <= MAX_LATIN1 as u32 {
        return LATIN1_PROPERTIES[r as u8 as usize] & P_LMASK != 0;
    }
    is_excluding_latin(LETTER, r)
}

// Go: src/unicode/graphic.go:IsMark
/// IsMark reports whether the rune is a mark character (category M).
pub fn is_mark(r: Rune) -> bool {
    // There are no mark characters in Latin-1.
    is_excluding_latin(MARK, r)
}

// Go: src/unicode/graphic.go:IsNumber
/// IsNumber reports whether the rune is a number (category N).
pub fn is_number(r: Rune) -> bool {
    if r as u32 <= MAX_LATIN1 as u32 {
        return LATIN1_PROPERTIES[r as u8 as usize] & P_N != 0;
    }
    is_excluding_latin(NUMBER, r)
}

// Go: src/unicode/graphic.go:IsPunct
/// IsPunct reports whether the rune is a Unicode punctuation character
/// (category P).
pub fn is_punct(r: Rune) -> bool {
    if r as u32 <= MAX_LATIN1 as u32 {
        return LATIN1_PROPERTIES[r as u8 as usize] & P_P != 0;
    }
    is(PUNCT, r)
}

// Go: src/unicode/graphic.go:IsSpace
/// IsSpace reports whether the rune is a space character as defined
/// by Unicode's White Space property; in the Latin-1 space
/// this is
///
/// ```text
/// '\t', '\n', '\v', '\f', '\r', ' ', U+0085 (NEL), U+00A0 (NBSP).
/// ```
///
/// Other definitions of spacing characters are set by category
/// Z and property Pattern_White_Space.
pub fn is_space(r: Rune) -> bool {
    // This property isn't the same as Z; special-case it.
    if r as u32 <= MAX_LATIN1 as u32 {
        return matches!(r, 0x09 | 0x0A | 0x0B | 0x0C | 0x0D | 0x20 | 0x85 | 0xA0);
    }
    is_excluding_latin(WHITE_SPACE, r)
}

// Go: src/unicode/graphic.go:IsSymbol
/// IsSymbol reports whether the rune is a symbolic character.
pub fn is_symbol(r: Rune) -> bool {
    if r as u32 <= MAX_LATIN1 as u32 {
        return LATIN1_PROPERTIES[r as u8 as usize] & P_S != 0;
    }
    is_excluding_latin(SYMBOL, r)
}

// Go: src/unicode/digit.go:IsDigit
/// IsDigit reports whether the rune is a decimal digit.
pub fn is_digit(r: Rune) -> bool {
    if r <= MAX_LATIN1 {
        return '0' as Rune <= r && r <= '9' as Rune;
    }
    is_excluding_latin(DIGIT, r)
}

/// Looks `name` up in one of the generated Go maps ([`CATEGORIES`],
/// [`SCRIPTS`], [`PROPERTIES`], [`FOLD_CATEGORY`], [`FOLD_SCRIPT`],
/// [`CATEGORY_ALIASES`]); equivalent to Go's `m[name]` with the `ok` result.
pub fn lookup<T: Copy>(map: &[(&str, T)], name: &str) -> Option<T> {
    map.binary_search_by(|(k, _)| k.as_bytes().cmp(name.as_bytes()))
        .ok()
        .map(|i| map[i].1)
}
