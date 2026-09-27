//! Go's `bytes` package (go1.27.1): the functions with text semantics, over
//! `&[u8]`. Kept separate from [`crate::strings`] because several functions
//! differ in detail between the two Go packages (explode limits, IndexAny /
//! LastIndexAny special cases, always-copying results).
//!
//! Go's distinction between a nil and an empty `[]byte` result is not
//! represented; both are an empty slice / `Vec`. `bytes.Buffer` and
//! `bytes.Reader` are not ported.

use std::cmp::Ordering;

use crate::strings::{ASCII_SPACE, make_ascii_set, should_use_ascii_set};
use crate::utf8::{self, RUNE_ERROR, RUNE_SELF};
use crate::{Rune, SpecialCase};

// Go: src/bytes/bytes.go:Equal
/// Equal reports whether a and b
/// are the same length and contain the same bytes.
#[inline]
pub fn equal(a: &[u8], b: &[u8]) -> bool {
    a == b
}

// Go: src/bytes/bytes.go:Compare
/// Compare returns an integer comparing two byte slices lexicographically.
/// The result will be 0 if a == b, -1 if a < b, and +1 if a > b.
pub fn compare(a: &[u8], b: &[u8]) -> isize {
    match a.cmp(b) {
        Ordering::Less => -1,
        Ordering::Equal => 0,
        Ordering::Greater => 1,
    }
}

// Go: src/bytes/bytes.go:explode
/// explode splits s into a slice of UTF-8 sequences, one per Unicode code point (still slices of bytes),
/// up to a maximum of n byte slices. Invalid UTF-8 sequences are chopped into individual bytes.
fn explode(mut s: &[u8], mut n: isize) -> Vec<&[u8]> {
    if n <= 0 || n > s.len() as isize {
        n = s.len() as isize;
    }
    let mut a: Vec<&[u8]> = Vec::with_capacity(n as usize);
    let mut na: isize = 0;
    while !s.is_empty() {
        if na + 1 >= n {
            a.push(s);
            break;
        }
        let (_, size) = utf8::decode_rune(s);
        a.push(&s[0..size]);
        s = &s[size..];
        na += 1;
    }
    a
}

// Go: src/bytes/bytes.go:Count
/// Count counts the number of non-overlapping instances of sep in s.
/// If sep is an empty slice, Count returns 1 + the number of UTF-8-encoded code points in s.
pub fn count(mut s: &[u8], sep: &[u8]) -> usize {
    // special case
    if sep.is_empty() {
        return utf8::rune_count(s) + 1;
    }
    if sep.len() == 1 {
        return memchr::memchr_iter(sep[0], s).count();
    }
    let mut n = 0;
    loop {
        let i = index(s, sep);
        if i == -1 {
            return n;
        }
        n += 1;
        s = &s[i as usize + sep.len()..];
    }
}

// Go: src/bytes/bytes.go:Contains
/// Contains reports whether subslice is within b.
pub fn contains(b: &[u8], subslice: &[u8]) -> bool {
    index(b, subslice) != -1
}

// Go: src/bytes/bytes.go:ContainsAny
/// ContainsAny reports whether any of the UTF-8-encoded code points in chars are within b.
pub fn contains_any(b: &[u8], chars: &[u8]) -> bool {
    index_any(b, chars) >= 0
}

// Go: src/bytes/bytes.go:ContainsRune
/// ContainsRune reports whether the rune is contained in the UTF-8-encoded byte slice b.
pub fn contains_rune(b: &[u8], r: Rune) -> bool {
    index_rune(b, r) >= 0
}

// Go: src/bytes/bytes.go:ContainsFunc
/// ContainsFunc reports whether any of the UTF-8-encoded code points r within b satisfy f(r).
pub fn contains_func(b: &[u8], f: impl FnMut(Rune) -> bool) -> bool {
    index_func(b, f) >= 0
}

// Go: src/bytes/bytes.go:IndexByte
/// IndexByte returns the index of the first instance of c in b, or -1 if c is not present in b.
#[inline]
pub fn index_byte(b: &[u8], c: u8) -> isize {
    crate::strings::index_byte(b, c)
}

// Go: src/bytes/bytes.go:LastIndex
/// LastIndex returns the index of the last instance of sep in s, or -1 if sep is not present in s.
pub fn last_index(s: &[u8], sep: &[u8]) -> isize {
    crate::strings::last_index(s, sep)
}

// Go: src/bytes/bytes.go:LastIndexByte
/// LastIndexByte returns the index of the last instance of c in s, or -1 if c is not present in s.
pub fn last_index_byte(s: &[u8], c: u8) -> isize {
    crate::strings::last_index_byte(s, c)
}

// Go: src/bytes/bytes.go:IndexRune
/// IndexRune interprets s as a sequence of UTF-8-encoded code points.
/// It returns the byte index of the first occurrence in s of the given rune.
/// It returns -1 if rune is not present in s.
/// If r is RuneError, it returns the first instance of any
/// invalid UTF-8 byte sequence.
pub fn index_rune(s: &[u8], r: Rune) -> isize {
    if (0..RUNE_SELF).contains(&r) {
        index_byte(s, r as u8)
    } else if r == RUNE_ERROR {
        let mut i = 0;
        while i < s.len() {
            let (r1, n) = utf8::decode_rune(&s[i..]);
            if r1 == RUNE_ERROR {
                return i as isize;
            }
            i += n;
        }
        -1
    } else if !utf8::valid_rune(r) {
        -1
    } else {
        // Search for the UTF-8 encoding of r (the Go last-byte scan and its
        // fallbacks all return the first occurrence).
        let mut b = [0u8; utf8::UTF_MAX];
        let n = utf8::encode_rune(&mut b, r);
        index(s, &b[..n])
    }
}

// Go: src/bytes/bytes.go:IndexAny
/// IndexAny interprets s as a sequence of UTF-8-encoded Unicode code points.
/// It returns the byte index of the first occurrence in s of any of the Unicode
/// code points in chars. It returns -1 if chars is empty or if there is no code
/// point in common.
pub fn index_any(s: &[u8], chars: &[u8]) -> isize {
    if chars.is_empty() {
        // Avoid scanning all of s.
        return -1;
    }
    if s.len() == 1 {
        let r = s[0] as Rune;
        if r >= RUNE_SELF {
            // search utf8.RuneError.
            for (_, r) in utf8::runes(chars) {
                if r == RUNE_ERROR {
                    return 0;
                }
            }
            return -1;
        }
        if index_byte(chars, s[0]) >= 0 {
            return 0;
        }
        return -1;
    }
    if chars.len() == 1 {
        let mut r = chars[0] as Rune;
        if r >= RUNE_SELF {
            r = RUNE_ERROR;
        }
        return index_rune(s, r);
    }
    if should_use_ascii_set(s.len())
        && let Some(as_) = make_ascii_set(chars)
    {
        for (i, &c) in s.iter().enumerate() {
            if as_.contains(c) {
                return i as isize;
            }
        }
        return -1;
    }
    let mut width;
    let mut i = 0;
    while i < s.len() {
        let mut r = s[i] as Rune;
        if r < RUNE_SELF {
            if index_byte(chars, s[i]) >= 0 {
                return i as isize;
            }
            i += 1;
            continue;
        }
        (r, width) = utf8::decode_rune(&s[i..]);
        if r != RUNE_ERROR {
            // r is 2 to 4 bytes
            let enc = utf8::rune_to_string(r);
            if chars.len() == width {
                if chars == enc.as_slice() {
                    return i as isize;
                }
                i += width;
                continue;
            }
            // Use bytealg.IndexString for performance if available
            // (bytealg.MaxLen is 32 on arm64, so always).
            if crate::strings::index(chars, &enc) >= 0 {
                return i as isize;
            }
            i += width;
            continue;
        }
        for (_, ch) in utf8::runes(chars) {
            if r == ch {
                return i as isize;
            }
        }
        i += width;
    }
    -1
}

// Go: src/bytes/bytes.go:LastIndexAny
/// LastIndexAny interprets s as a sequence of UTF-8-encoded Unicode code
/// points. It returns the byte index of the last occurrence in s of any of
/// the Unicode code points in chars. It returns -1 if chars is empty or if
/// there is no code point in common.
pub fn last_index_any(s: &[u8], chars: &[u8]) -> isize {
    if chars.is_empty() {
        // Avoid scanning all of s.
        return -1;
    }
    if should_use_ascii_set(s.len())
        && let Some(as_) = make_ascii_set(chars)
    {
        for i in (0..s.len()).rev() {
            if as_.contains(s[i]) {
                return i as isize;
            }
        }
        return -1;
    }
    if s.len() == 1 {
        let r = s[0] as Rune;
        if r >= RUNE_SELF {
            for (_, r) in utf8::runes(chars) {
                if r == RUNE_ERROR {
                    return 0;
                }
            }
            return -1;
        }
        if index_byte(chars, s[0]) >= 0 {
            return 0;
        }
        return -1;
    }
    if chars.len() == 1 {
        let mut cr = chars[0] as Rune;
        if cr >= RUNE_SELF {
            cr = RUNE_ERROR;
        }
        let mut i = s.len();
        while i > 0 {
            let (r, size) = utf8::decode_last_rune(&s[..i]);
            i -= size;
            if r == cr {
                return i as isize;
            }
        }
        return -1;
    }
    let mut i = s.len();
    while i > 0 {
        let r = s[i - 1] as Rune;
        if r < RUNE_SELF {
            if index_byte(chars, s[i - 1]) >= 0 {
                return i as isize - 1;
            }
            i -= 1;
            continue;
        }
        let (r, size) = utf8::decode_last_rune(&s[..i]);
        i -= size;
        if r != RUNE_ERROR {
            // r is 2 to 4 bytes
            let enc = utf8::rune_to_string(r);
            if chars.len() == size {
                if chars == enc.as_slice() {
                    return i as isize;
                }
                continue;
            }
            // Use bytealg.IndexString for performance if available.
            if crate::strings::index(chars, &enc) >= 0 {
                return i as isize;
            }
            continue;
        }
        for (_, ch) in utf8::runes(chars) {
            if r == ch {
                return i as isize;
            }
        }
    }
    -1
}

// Go: src/bytes/bytes.go:genSplit
/// Generic split: splits after each instance of sep,
/// including sepSave bytes of sep in the subslices.
fn gen_split<'a>(mut s: &'a [u8], sep: &[u8], sep_save: usize, mut n: isize) -> Vec<&'a [u8]> {
    if n == 0 {
        return Vec::new();
    }
    if sep.is_empty() {
        return explode(s, n);
    }
    if n < 0 {
        n = count(s, sep) as isize + 1;
    }
    n = n.min(s.len() as isize + 1);

    let mut a: Vec<&'a [u8]> = Vec::with_capacity(n as usize);
    n -= 1;
    let mut i = 0;
    while i < n {
        let m = index(s, sep);
        if m < 0 {
            break;
        }
        let m = m as usize;
        a.push(&s[..m + sep_save]);
        s = &s[m + sep.len()..];
        i += 1;
    }
    a.push(s);
    a
}

// Go: src/bytes/bytes.go:SplitN
/// SplitN slices s into subslices separated by sep and returns a slice of
/// the subslices between those separators.
/// If sep is empty, SplitN splits after each UTF-8 sequence.
pub fn split_n<'a>(s: &'a [u8], sep: &[u8], n: isize) -> Vec<&'a [u8]> {
    gen_split(s, sep, 0, n)
}

// Go: src/bytes/bytes.go:SplitAfterN
/// SplitAfterN slices s into subslices after each instance of sep and
/// returns a slice of those subslices.
pub fn split_after_n<'a>(s: &'a [u8], sep: &[u8], n: isize) -> Vec<&'a [u8]> {
    gen_split(s, sep, sep.len(), n)
}

// Go: src/bytes/bytes.go:Split
/// Split slices s into all subslices separated by sep and returns a slice of
/// the subslices between those separators.
pub fn split<'a>(s: &'a [u8], sep: &[u8]) -> Vec<&'a [u8]> {
    gen_split(s, sep, 0, -1)
}

// Go: src/bytes/bytes.go:SplitAfter
/// SplitAfter slices s into all subslices after each instance of sep and
/// returns a slice of those subslices.
pub fn split_after<'a>(s: &'a [u8], sep: &[u8]) -> Vec<&'a [u8]> {
    gen_split(s, sep, sep.len(), -1)
}

// Go: src/bytes/bytes.go:Fields
/// Fields interprets s as a sequence of UTF-8-encoded code points.
/// It splits the slice s around each instance of one or more consecutive white space
/// characters, as defined by [`crate::is_space`], returning a slice of subslices of s or an
/// empty slice if s contains only white space.
pub fn fields(s: &[u8]) -> Vec<&[u8]> {
    // First count the fields.
    // This is an exact count if s is ASCII, otherwise it is an approximation.
    let mut n: usize = 0;
    let mut was_space: usize = 1;
    // setBits is used to track which bits are set in the bytes of s.
    let mut set_bits: u8 = 0;
    for i in 0..s.len() {
        let r = s[i];
        set_bits |= r;
        let is_space = ASCII_SPACE[r as usize] as usize;
        n += was_space & !is_space;
        was_space = is_space;
    }

    if set_bits as Rune >= RUNE_SELF {
        // Some runes in the input slice are not ASCII.
        return fields_func(s, crate::is_space);
    }

    // ASCII fast path
    let mut a: Vec<&[u8]> = Vec::with_capacity(n);
    let mut i = 0;
    // Skip spaces in the front of the input.
    while i < s.len() && ASCII_SPACE[s[i] as usize] != 0 {
        i += 1;
    }
    let mut field_start = i;
    while i < s.len() {
        if ASCII_SPACE[s[i] as usize] == 0 {
            i += 1;
            continue;
        }
        a.push(&s[field_start..i]);
        i += 1;
        // Skip spaces in between fields.
        while i < s.len() && ASCII_SPACE[s[i] as usize] != 0 {
            i += 1;
        }
        field_start = i;
    }
    if field_start < s.len() {
        // Last field might end at EOF.
        a.push(&s[field_start..]);
    }
    a
}

// Go: src/bytes/bytes.go:FieldsFunc
/// FieldsFunc interprets s as a sequence of UTF-8-encoded code points.
/// It splits the slice s at each run of code points c satisfying f(c) and
/// returns a slice of subslices of s. If all code points in s satisfy f(c), or
/// len(s) == 0, an empty slice is returned.
pub fn fields_func(s: &[u8], mut f: impl FnMut(Rune) -> bool) -> Vec<&[u8]> {
    // A span is used to record a slice of s of the form s[start:end].
    // The start index is inclusive and the end index is exclusive.
    let mut spans: Vec<(usize, usize)> = Vec::with_capacity(32);

    // Find the field start and end indices.
    let mut start: isize = -1; // valid span start if >= 0
    let mut i = 0;
    while i < s.len() {
        let (r, size) = utf8::decode_rune(&s[i..]);
        if f(r) {
            if start >= 0 {
                spans.push((start as usize, i));
                start = -1;
            }
        } else if start < 0 {
            start = i as isize;
        }
        i += size;
    }

    // Last field might end at EOF.
    if start >= 0 {
        spans.push((start as usize, s.len()));
    }

    // Create subslices from recorded field indices.
    spans.iter().map(|&(a, b)| &s[a..b]).collect()
}

// Go: src/bytes/bytes.go:Join
/// Join concatenates the elements of s to create a new byte slice. The separator
/// sep is placed between elements in the resulting slice.
pub fn join<T: AsRef<[u8]>>(s: &[T], sep: &[u8]) -> Vec<u8> {
    crate::strings::join(s, sep)
}

// Go: src/bytes/bytes.go:HasPrefix
/// HasPrefix reports whether the byte slice s begins with prefix.
#[inline]
pub fn has_prefix(s: &[u8], prefix: &[u8]) -> bool {
    crate::strings::has_prefix(s, prefix)
}

// Go: src/bytes/bytes.go:HasSuffix
/// HasSuffix reports whether the byte slice s ends with suffix.
#[inline]
pub fn has_suffix(s: &[u8], suffix: &[u8]) -> bool {
    crate::strings::has_suffix(s, suffix)
}

// Go: src/bytes/bytes.go:Map
/// Map returns a copy of the byte slice s with all its characters modified
/// according to the mapping function. If mapping returns a negative value, the character is
/// dropped from the byte slice with no replacement. The characters in s and the
/// output are interpreted as UTF-8-encoded code points.
pub fn map(mut mapping: impl FnMut(Rune) -> Rune, s: &[u8]) -> Vec<u8> {
    // In the worst case, the slice can grow when mapped, making
    // things unpleasant. But it's so rare we barge in assuming it's
    // fine. It could also shrink but that falls out naturally.
    let mut b = Vec::with_capacity(s.len());
    let mut i = 0;
    while i < s.len() {
        let (r, wid) = utf8::decode_rune(&s[i..]);
        let r = mapping(r);
        if r >= 0 {
            utf8::append_rune(&mut b, r);
        }
        i += wid;
    }
    b
}

// Go: src/bytes/bytes.go:Repeat
/// Repeat returns a new byte slice consisting of count copies of b.
///
/// It panics if count is negative or if the result of (len(b) * count)
/// overflows (as Go does).
pub fn repeat(b: &[u8], count: isize) -> Vec<u8> {
    if count == 0 {
        return Vec::new();
    }
    if count < 0 {
        panic!("bytes: negative Repeat count");
    }
    let n = b
        .len()
        .checked_mul(count as usize)
        .filter(|&n| n <= isize::MAX as usize)
        .expect("bytes: Repeat output length overflow");
    if b.is_empty() {
        return Vec::new();
    }
    // Go allocates the result up front (Builder.Grow / MakeNoZero) and
    // panics with "makeslice: len out of range" when that is impossible; a
    // failed allocation panics here too instead of aborting the process.
    let mut nb = Vec::new();
    if nb.try_reserve_exact(n).is_err() {
        panic!("makeslice: len out of range");
    }
    for _ in 0..count {
        nb.extend_from_slice(b);
    }
    nb
}

// Go: src/bytes/bytes.go:ToUpper
/// ToUpper returns a copy of the byte slice s with all Unicode letters mapped to
/// their upper case.
pub fn to_upper(s: &[u8]) -> Vec<u8> {
    let (mut is_ascii, mut has_lower) = (true, false);
    for i in 0..s.len() {
        let c = s[i];
        if c as Rune >= RUNE_SELF {
            is_ascii = false;
            break;
        }
        has_lower = has_lower || c.is_ascii_lowercase();
    }

    if is_ascii {
        // optimize for ASCII-only byte slices.
        if !has_lower {
            // Just return a copy.
            return s.to_vec();
        }
        let mut b = Vec::with_capacity(s.len());
        for i in 0..s.len() {
            let mut c = s[i];
            if c.is_ascii_lowercase() {
                c -= b'a' - b'A';
            }
            b.push(c);
        }
        return b;
    }
    map(crate::to_upper, s)
}

// Go: src/bytes/bytes.go:ToLower
/// ToLower returns a copy of the byte slice s with all Unicode letters mapped to
/// their lower case.
pub fn to_lower(s: &[u8]) -> Vec<u8> {
    let (mut is_ascii, mut has_upper) = (true, false);
    for i in 0..s.len() {
        let c = s[i];
        if c as Rune >= RUNE_SELF {
            is_ascii = false;
            break;
        }
        has_upper = has_upper || c.is_ascii_uppercase();
    }

    if is_ascii {
        // optimize for ASCII-only byte slices.
        if !has_upper {
            return s.to_vec();
        }
        let mut b = Vec::with_capacity(s.len());
        for i in 0..s.len() {
            let mut c = s[i];
            if c.is_ascii_uppercase() {
                c += b'a' - b'A';
            }
            b.push(c);
        }
        return b;
    }
    map(crate::to_lower, s)
}

// Go: src/bytes/bytes.go:ToTitle
/// ToTitle treats s as UTF-8-encoded bytes and returns a copy with all the Unicode letters mapped to their title case.
pub fn to_title(s: &[u8]) -> Vec<u8> {
    map(crate::to_title, s)
}

// Go: src/bytes/bytes.go:ToUpperSpecial
/// ToUpperSpecial treats s as UTF-8-encoded bytes and returns a copy with all the Unicode letters mapped to their
/// upper case, giving priority to the special casing rules.
pub fn to_upper_special(c: SpecialCase<'_>, s: &[u8]) -> Vec<u8> {
    map(|r| c.to_upper(r), s)
}

// Go: src/bytes/bytes.go:ToLowerSpecial
/// ToLowerSpecial treats s as UTF-8-encoded bytes and returns a copy with all the Unicode letters mapped to their
/// lower case, giving priority to the special casing rules.
pub fn to_lower_special(c: SpecialCase<'_>, s: &[u8]) -> Vec<u8> {
    map(|r| c.to_lower(r), s)
}

// Go: src/bytes/bytes.go:ToTitleSpecial
/// ToTitleSpecial treats s as UTF-8-encoded bytes and returns a copy with all the Unicode letters mapped to their
/// title case, giving priority to the special casing rules.
pub fn to_title_special(c: SpecialCase<'_>, s: &[u8]) -> Vec<u8> {
    map(|r| c.to_title(r), s)
}

// Go: src/bytes/bytes.go:ToValidUTF8
/// ToValidUTF8 treats s as UTF-8-encoded bytes and returns a copy with each run of bytes
/// representing invalid UTF-8 replaced with the bytes in replacement, which may be empty.
pub fn to_valid_utf8(s: &[u8], replacement: &[u8]) -> Vec<u8> {
    let mut b = Vec::with_capacity(s.len() + replacement.len());
    let mut invalid = false; // previous byte was from an invalid UTF-8 sequence
    let mut i = 0;
    while i < s.len() {
        let c = s[i];
        if (c as Rune) < RUNE_SELF {
            i += 1;
            invalid = false;
            b.push(c);
            continue;
        }
        let (_, wid) = utf8::decode_rune(&s[i..]);
        if wid == 1 {
            i += 1;
            if !invalid {
                invalid = true;
                b.extend_from_slice(replacement);
            }
            continue;
        }
        invalid = false;
        b.extend_from_slice(&s[i..i + wid]);
        i += wid;
    }
    b
}

// Go: src/bytes/bytes.go:Title
/// Title treats s as UTF-8-encoded bytes and returns a copy with all Unicode letters that begin
/// words mapped to their title case.
///
/// Deprecated in Go: The rule Title uses for word boundaries does not handle Unicode
/// punctuation properly.
pub fn title(s: &[u8]) -> Vec<u8> {
    // Use a closure here to remember state.
    // Hackish but effective. Depends on Map scanning in order and calling
    // the closure once per rune.
    let mut prev: Rune = ' ' as Rune;
    map(
        |r| {
            if crate::strings::is_separator(prev) {
                prev = r;
                return crate::to_title(r);
            }
            prev = r;
            r
        },
        s,
    )
}

// Go: src/bytes/bytes.go:TrimLeftFunc
/// TrimLeftFunc treats s as UTF-8-encoded bytes and returns a subslice of s by slicing off
/// all leading UTF-8-encoded code points c that satisfy f(c).
pub fn trim_left_func(s: &[u8], f: impl FnMut(Rune) -> bool) -> &[u8] {
    let i = index_func_impl(s, f, false);
    if i == -1 {
        return &s[s.len()..];
    }
    &s[i as usize..]
}

// Go: src/bytes/bytes.go:TrimRightFunc
/// TrimRightFunc returns a subslice of s by slicing off all trailing
/// UTF-8-encoded code points c that satisfy f(c).
pub fn trim_right_func(s: &[u8], f: impl FnMut(Rune) -> bool) -> &[u8] {
    let mut i = last_index_func_impl(s, f, false);
    if i >= 0 && s[i as usize] as Rune >= RUNE_SELF {
        let (_, wid) = utf8::decode_rune(&s[i as usize..]);
        i += wid as isize;
    } else {
        i += 1;
    }
    &s[0..i as usize]
}

// Go: src/bytes/bytes.go:TrimFunc
/// TrimFunc returns a subslice of s by slicing off all leading and trailing
/// UTF-8-encoded code points c that satisfy f(c).
pub fn trim_func(s: &[u8], mut f: impl FnMut(Rune) -> bool) -> &[u8] {
    trim_right_func(trim_left_func(s, &mut f), &mut f)
}

// Go: src/bytes/bytes.go:TrimPrefix
/// TrimPrefix returns s without the provided leading prefix string.
/// If s doesn't start with prefix, s is returned unchanged.
pub fn trim_prefix<'a>(s: &'a [u8], prefix: &[u8]) -> &'a [u8] {
    if has_prefix(s, prefix) {
        return &s[prefix.len()..];
    }
    s
}

// Go: src/bytes/bytes.go:TrimSuffix
/// TrimSuffix returns s without the provided trailing suffix string.
/// If s doesn't end with suffix, s is returned unchanged.
pub fn trim_suffix<'a>(s: &'a [u8], suffix: &[u8]) -> &'a [u8] {
    if has_suffix(s, suffix) {
        return &s[..s.len() - suffix.len()];
    }
    s
}

// Go: src/bytes/bytes.go:IndexFunc
/// IndexFunc interprets s as a sequence of UTF-8-encoded code points.
/// It returns the byte index in s of the first Unicode
/// code point satisfying f(c), or -1 if none do.
pub fn index_func(s: &[u8], f: impl FnMut(Rune) -> bool) -> isize {
    index_func_impl(s, f, true)
}

// Go: src/bytes/bytes.go:LastIndexFunc
/// LastIndexFunc interprets s as a sequence of UTF-8-encoded code points.
/// It returns the byte index in s of the last Unicode
/// code point satisfying f(c), or -1 if none do.
pub fn last_index_func(s: &[u8], f: impl FnMut(Rune) -> bool) -> isize {
    last_index_func_impl(s, f, true)
}

// Go: src/bytes/bytes.go:indexFunc
/// indexFunc is the same as IndexFunc except that if
/// truth==false, the sense of the predicate function is
/// inverted.
fn index_func_impl(s: &[u8], mut f: impl FnMut(Rune) -> bool, truth: bool) -> isize {
    let mut start = 0;
    while start < s.len() {
        let (r, wid) = utf8::decode_rune(&s[start..]);
        if f(r) == truth {
            return start as isize;
        }
        start += wid;
    }
    -1
}

// Go: src/bytes/bytes.go:lastIndexFunc
/// lastIndexFunc is the same as LastIndexFunc except that if
/// truth==false, the sense of the predicate function is
/// inverted.
fn last_index_func_impl(s: &[u8], mut f: impl FnMut(Rune) -> bool, truth: bool) -> isize {
    let mut i = s.len();
    while i > 0 {
        let (mut r, mut size) = (s[i - 1] as Rune, 1);
        if r >= RUNE_SELF {
            (r, size) = utf8::decode_last_rune(&s[0..i]);
        }
        i -= size;
        if f(r) == truth {
            return i as isize;
        }
    }
    -1
}

// Go: src/bytes/bytes.go:containsRune
/// containsRune is a simplified version of strings.ContainsRune
/// to avoid importing the strings package.
fn contains_rune_str(s: &[u8], r: Rune) -> bool {
    for (_, c) in utf8::runes(s) {
        if c == r {
            return true;
        }
    }
    false
}

// Go: src/bytes/bytes.go:Trim
/// Trim returns a subslice of s by slicing off all leading and
/// trailing UTF-8-encoded code points contained in cutset.
pub fn trim<'a>(s: &'a [u8], cutset: &[u8]) -> &'a [u8] {
    if s.is_empty() {
        // This is what we've historically done.
        return s;
    }
    if cutset.is_empty() {
        return s;
    }
    if cutset.len() == 1 && (cutset[0] as Rune) < RUNE_SELF {
        return trim_left_byte(trim_right_byte(s, cutset[0]), cutset[0]);
    }
    if let Some(as_) = make_ascii_set(cutset) {
        return trim_left_ascii(trim_right_ascii(s, &as_), &as_);
    }
    trim_left_unicode(trim_right_unicode(s, cutset), cutset)
}

// Go: src/bytes/bytes.go:TrimLeft
/// TrimLeft returns a subslice of s by slicing off all leading
/// UTF-8-encoded code points contained in cutset.
pub fn trim_left<'a>(s: &'a [u8], cutset: &[u8]) -> &'a [u8] {
    if s.is_empty() {
        // This is what we've historically done.
        return s;
    }
    if cutset.is_empty() {
        return s;
    }
    if cutset.len() == 1 && (cutset[0] as Rune) < RUNE_SELF {
        return trim_left_byte(s, cutset[0]);
    }
    if let Some(as_) = make_ascii_set(cutset) {
        return trim_left_ascii(s, &as_);
    }
    trim_left_unicode(s, cutset)
}

// Go: src/bytes/bytes.go:trimLeftByte
fn trim_left_byte(mut s: &[u8], c: u8) -> &[u8] {
    while !s.is_empty() && s[0] == c {
        s = &s[1..];
    }
    s
}

// Go: src/bytes/bytes.go:trimLeftASCII
fn trim_left_ascii<'a>(mut s: &'a [u8], as_: &crate::strings::AsciiSet) -> &'a [u8] {
    while !s.is_empty() {
        if !as_.contains(s[0]) {
            break;
        }
        s = &s[1..];
    }
    s
}

// Go: src/bytes/bytes.go:trimLeftUnicode
fn trim_left_unicode<'a>(mut s: &'a [u8], cutset: &[u8]) -> &'a [u8] {
    while !s.is_empty() {
        let (r, n) = utf8::decode_rune(s);
        if !contains_rune_str(cutset, r) {
            break;
        }
        s = &s[n..];
    }
    s
}

// Go: src/bytes/bytes.go:TrimRight
/// TrimRight returns a subslice of s by slicing off all trailing
/// UTF-8-encoded code points that are contained in cutset.
pub fn trim_right<'a>(s: &'a [u8], cutset: &[u8]) -> &'a [u8] {
    if s.is_empty() || cutset.is_empty() {
        return s;
    }
    if cutset.len() == 1 && (cutset[0] as Rune) < RUNE_SELF {
        return trim_right_byte(s, cutset[0]);
    }
    if let Some(as_) = make_ascii_set(cutset) {
        return trim_right_ascii(s, &as_);
    }
    trim_right_unicode(s, cutset)
}

// Go: src/bytes/bytes.go:trimRightByte
fn trim_right_byte(mut s: &[u8], c: u8) -> &[u8] {
    while !s.is_empty() && s[s.len() - 1] == c {
        s = &s[..s.len() - 1];
    }
    s
}

// Go: src/bytes/bytes.go:trimRightASCII
fn trim_right_ascii<'a>(mut s: &'a [u8], as_: &crate::strings::AsciiSet) -> &'a [u8] {
    while !s.is_empty() {
        if !as_.contains(s[s.len() - 1]) {
            break;
        }
        s = &s[..s.len() - 1];
    }
    s
}

// Go: src/bytes/bytes.go:trimRightUnicode
fn trim_right_unicode<'a>(mut s: &'a [u8], cutset: &[u8]) -> &'a [u8] {
    while !s.is_empty() {
        let (mut r, mut n) = (s[s.len() - 1] as Rune, 1);
        if r >= RUNE_SELF {
            (r, n) = utf8::decode_last_rune(s);
        }
        if !contains_rune_str(cutset, r) {
            break;
        }
        s = &s[..s.len() - n];
    }
    s
}

// Go: src/bytes/bytes.go:TrimSpace
/// TrimSpace returns a subslice of s by slicing off all leading and
/// trailing white space, as defined by Unicode.
pub fn trim_space(s: &[u8]) -> &[u8] {
    // Fast path for ASCII: look for the first ASCII non-space byte.
    for (lo, &c) in s.iter().enumerate() {
        if c as Rune >= RUNE_SELF {
            // If we run into a non-ASCII byte, fall back to the
            // slower unicode-aware method on the remaining bytes.
            return trim_func(&s[lo..], crate::is_space);
        }
        if ASCII_SPACE[c as usize] != 0 {
            continue;
        }
        let s = &s[lo..];
        // Now look for the first ASCII non-space byte from the end.
        for hi in (0..s.len()).rev() {
            let c = s[hi];
            if c as Rune >= RUNE_SELF {
                return trim_func(&s[..hi + 1], crate::is_space);
            }
            if ASCII_SPACE[c as usize] == 0 {
                // At this point, s[:hi+1] starts and ends with ASCII
                // non-space bytes, so we're done. Non-ASCII cases have
                // already been handled above.
                return &s[..hi + 1];
            }
        }
    }
    // Special case to preserve previous TrimLeftFunc behavior,
    // returning nil instead of empty slice if all spaces.
    &s[s.len()..]
}

// Go: src/bytes/bytes.go:Runes
/// Runes interprets s as a sequence of UTF-8-encoded code points.
/// It returns a slice of runes (Unicode code points) equivalent to s.
pub fn runes(mut s: &[u8]) -> Vec<Rune> {
    let mut t = Vec::with_capacity(utf8::rune_count(s));
    while !s.is_empty() {
        let (r, l) = utf8::decode_rune(s);
        t.push(r);
        s = &s[l..];
    }
    t
}

// Go: src/bytes/bytes.go:Replace
/// Replace returns a copy of the slice s with the first n
/// non-overlapping instances of old replaced by new.
/// If old is empty, it matches at the beginning of the slice
/// and after each UTF-8 sequence, yielding up to k+1 replacements
/// for a k-rune slice.
/// If n < 0, there is no limit on the number of replacements.
pub fn replace(s: &[u8], old: &[u8], new: &[u8], mut n: isize) -> Vec<u8> {
    let mut m: isize = 0;
    if n != 0 {
        // Compute number of replacements.
        m = count(s, old) as isize;
    }
    if m == 0 {
        // Just return a copy.
        return s.to_vec();
    }
    if n < 0 || m < n {
        n = m;
    }

    // Apply replacements to buffer.
    let mut t = Vec::with_capacity(
        (s.len() as isize + n * (new.len() as isize - old.len() as isize)).max(0) as usize,
    );
    let mut start = 0usize;
    if !old.is_empty() {
        for _ in 0..n {
            let j = start + index(&s[start..], old) as usize;
            t.extend_from_slice(&s[start..j]);
            t.extend_from_slice(new);
            start = j + old.len();
        }
    } else {
        // len(old) == 0
        t.extend_from_slice(new);
        for _ in 0..n - 1 {
            let (_, wid) = utf8::decode_rune(&s[start..]);
            let j = start + wid;
            t.extend_from_slice(&s[start..j]);
            t.extend_from_slice(new);
            start = j;
        }
    }
    t.extend_from_slice(&s[start..]);
    t
}

// Go: src/bytes/bytes.go:ReplaceAll
/// ReplaceAll returns a copy of the slice s with all
/// non-overlapping instances of old replaced by new.
pub fn replace_all(s: &[u8], old: &[u8], new: &[u8]) -> Vec<u8> {
    replace(s, old, new, -1)
}

// Go: src/bytes/bytes.go:EqualFold
/// EqualFold reports whether s and t, interpreted as UTF-8 strings,
/// are equal under simple Unicode case-folding, which is a more general
/// form of case-insensitivity.
pub fn equal_fold(s: &[u8], t: &[u8]) -> bool {
    // ASCII fast path
    let mut i = 0;
    let n = s.len().min(t.len());
    let mut has_unicode = false;
    while i < n {
        let mut sr = s[i];
        let mut tr = t[i];
        if (sr | tr) as Rune >= RUNE_SELF {
            has_unicode = true;
            break;
        }

        // Easy case.
        if tr == sr {
            i += 1;
            continue;
        }

        // Make sr < tr to simplify what follows.
        if tr < sr {
            std::mem::swap(&mut tr, &mut sr);
        }
        // ASCII only, sr/tr must be upper/lower case
        if sr.is_ascii_uppercase() && tr == sr + b'a' - b'A' {
            i += 1;
            continue;
        }
        return false;
    }
    if !has_unicode {
        // Check if we've exhausted both strings.
        return s.len() == t.len();
    }

    // hasUnicode:
    let mut s = &s[i..];
    let mut t = &t[i..];
    while !s.is_empty() && !t.is_empty() {
        // Extract first rune from each.
        let (mut sr, size) = utf8::decode_rune(s);
        s = &s[size..];
        let (mut tr, size) = utf8::decode_rune(t);
        t = &t[size..];

        // If they match, keep going; if not, return false.

        // Easy case.
        if tr == sr {
            continue;
        }

        // Make sr < tr to simplify what follows.
        if tr < sr {
            std::mem::swap(&mut tr, &mut sr);
        }
        // Fast check for ASCII.
        if tr < RUNE_SELF {
            // ASCII only, sr/tr must be upper/lower case
            if ('A' as Rune..='Z' as Rune).contains(&sr) && tr == sr + 'a' as Rune - 'A' as Rune {
                continue;
            }
            return false;
        }

        // General case. SimpleFold(x) returns the next equivalent rune > x
        // or wraps around to smaller values.
        let mut r = crate::simple_fold(sr);
        while r != sr && r < tr {
            r = crate::simple_fold(r);
        }
        if r == tr {
            continue;
        }
        return false;
    }

    // One string is empty. Are both?
    s.len() == t.len()
}

// Go: src/bytes/bytes.go:Index
/// Index returns the index of the first instance of sep in s, or -1 if sep is not present in s.
#[inline]
pub fn index(s: &[u8], sep: &[u8]) -> isize {
    crate::strings::index(s, sep)
}

// Go: src/bytes/bytes.go:Cut
/// Cut slices s around the first instance of sep,
/// returning the text before and after sep.
/// The found result reports whether sep appears in s.
/// If sep does not appear in s, cut returns s, nil, false.
pub fn cut<'a>(s: &'a [u8], sep: &[u8]) -> (&'a [u8], &'a [u8], bool) {
    crate::strings::cut(s, sep)
}

// Go: src/bytes/bytes.go:Clone
/// Clone returns a copy of b.
pub fn clone(b: &[u8]) -> Vec<u8> {
    b.to_vec()
}

// Go: src/bytes/bytes.go:CutPrefix
/// CutPrefix returns s without the provided leading prefix byte slice
/// and reports whether it found the prefix.
pub fn cut_prefix<'a>(s: &'a [u8], prefix: &[u8]) -> (&'a [u8], bool) {
    crate::strings::cut_prefix(s, prefix)
}

// Go: src/bytes/bytes.go:CutSuffix
/// CutSuffix returns s without the provided ending suffix byte slice
/// and reports whether it found the suffix.
pub fn cut_suffix<'a>(s: &'a [u8], suffix: &[u8]) -> (&'a [u8], bool) {
    crate::strings::cut_suffix(s, suffix)
}

// Go: src/bytes/bytes.go:CutLast
/// CutLast slices s around the last instance of sep,
/// returning the text before and after sep.
pub fn cut_last<'a>(s: &'a [u8], sep: &[u8]) -> (&'a [u8], &'a [u8], bool) {
    crate::strings::cut_last(s, sep)
}
