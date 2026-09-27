//! Go's `strings` package (go1.27.1) over `&[u8]`: a Go `string` is an
//! arbitrary byte sequence, so every function takes and returns bytes and
//! decodes UTF-8 with Go's rules (invalid byte = `RuneError`, width 1).
//!
//! Functions that return a substring of their input in Go return a borrowed
//! sub-slice; functions that may or may not allocate in Go return a
//! [`Cow`] that is `Borrowed` exactly when Go returns the input unchanged.
//! Go `int` results that may be `-1` are `isize`; counts are `usize`.
//!
//! `strings.Builder`, `strings.Reader` and the iterator functions of iter.go
//! are not ported (use `Vec<u8>` / slices); `strings.Replacer` is in
//! [`crate::replacer`].

use std::borrow::Cow;
use std::cmp::Ordering;

use crate::utf8::{self, RUNE_ERROR, RUNE_SELF, UTF_MAX};
use crate::{Rune, SpecialCase};

// Go: src/strings/strings.go:explode
/// explode splits s into a slice of UTF-8 strings,
/// one string per Unicode character up to a maximum of n (n < 0 means no limit).
/// Invalid UTF-8 bytes are sliced individually.
fn explode(mut s: &[u8], mut n: isize) -> Vec<&[u8]> {
    let l = utf8::rune_count_in_string(s) as isize;
    if n < 0 || n > l {
        n = l;
    }
    let mut a: Vec<&[u8]> = Vec::with_capacity(n as usize);
    let mut i = 0;
    while i < n - 1 {
        let (_, size) = utf8::decode_rune_in_string(s);
        a.push(&s[..size]);
        s = &s[size..];
        i += 1;
    }
    if n > 0 {
        a.push(s);
    }
    a
}

// Go: src/strings/strings.go:Count
/// Count counts the number of non-overlapping instances of substr in s.
/// If substr is an empty string, Count returns 1 + the number of Unicode code points in s.
pub fn count(mut s: &[u8], substr: &[u8]) -> usize {
    // special case
    if substr.is_empty() {
        return utf8::rune_count_in_string(s) + 1;
    }
    if substr.len() == 1 {
        return memchr::memchr_iter(substr[0], s).count();
    }
    let mut n = 0;
    loop {
        let i = index(s, substr);
        if i == -1 {
            return n;
        }
        n += 1;
        s = &s[i as usize + substr.len()..];
    }
}

// Go: src/strings/strings.go:Contains
/// Contains reports whether substr is within s.
pub fn contains(s: &[u8], substr: &[u8]) -> bool {
    index(s, substr) >= 0
}

// Go: src/strings/strings.go:ContainsAny
/// ContainsAny reports whether any Unicode code points in chars are within s.
pub fn contains_any(s: &[u8], chars: &[u8]) -> bool {
    index_any(s, chars) >= 0
}

// Go: src/strings/strings.go:ContainsRune
/// ContainsRune reports whether the Unicode code point r is within s.
pub fn contains_rune(s: &[u8], r: Rune) -> bool {
    index_rune(s, r) >= 0
}

// Go: src/strings/strings.go:ContainsFunc
/// ContainsFunc reports whether any Unicode code points r within s satisfy f(r).
/// It stops as soon as a call to f returns true.
pub fn contains_func(s: &[u8], f: impl FnMut(Rune) -> bool) -> bool {
    index_func(s, f) >= 0
}

// Go: src/strings/strings.go:LastIndex
/// LastIndex returns the index of the last instance of substr in s, or -1 if substr is not present in s.
pub fn last_index(s: &[u8], substr: &[u8]) -> isize {
    let n = substr.len();
    if n == 0 {
        return s.len() as isize;
    } else if n == 1 {
        return last_index_byte(s, substr[0]);
    } else if n == s.len() {
        if substr == s {
            return 0;
        }
        return -1;
    } else if n > s.len() {
        return -1;
    }
    // bytealg.LastIndexRabinKarp: the last occurrence.
    match memchr::memmem::rfind(s, substr) {
        Some(i) => i as isize,
        None => -1,
    }
}

// Go: src/strings/strings.go:IndexByte
/// IndexByte returns the index of the first instance of c in s, or -1 if c is not present in s.
#[inline]
pub fn index_byte(s: &[u8], c: u8) -> isize {
    match memchr::memchr(c, s) {
        Some(i) => i as isize,
        None => -1,
    }
}

// Go: src/strings/strings.go:IndexRune
/// IndexRune returns the index of the first instance of the Unicode code point
/// r, or -1 if rune is not present in s.
/// If r is RuneError, it returns the first instance of any
/// invalid UTF-8 byte sequence.
///
/// (The Go last-byte scan with its brute-force fallback finds the first
/// occurrence of the UTF-8 encoding of r; that search is done directly here.)
pub fn index_rune(s: &[u8], r: Rune) -> isize {
    if (0..RUNE_SELF).contains(&r) {
        index_byte(s, r as u8)
    } else if r == RUNE_ERROR {
        for (i, r) in utf8::runes(s) {
            if r == RUNE_ERROR {
                return i as isize;
            }
        }
        -1
    } else if !utf8::valid_rune(r) {
        -1
    } else {
        let rs = utf8::rune_to_string(r);
        index(s, &rs)
    }
}

// Go: src/strings/strings.go:IndexAny
/// IndexAny returns the index of the first instance of any Unicode code point
/// from chars in s, or -1 if no Unicode code point from chars is present in s.
pub fn index_any(s: &[u8], chars: &[u8]) -> isize {
    if chars.is_empty() {
        // Avoid scanning all of s.
        return -1;
    }
    if chars.len() == 1 {
        // Avoid scanning all of s.
        let mut r = chars[0] as Rune;
        if r >= RUNE_SELF {
            r = RUNE_ERROR;
        }
        return index_rune(s, r);
    }
    if should_use_ascii_set(s.len())
        && let Some(as_) = make_ascii_set(chars)
    {
        for i in 0..s.len() {
            if as_.contains(s[i]) {
                return i as isize;
            }
        }
        return -1;
    }
    for (i, c) in utf8::runes(s) {
        if index_rune(chars, c) >= 0 {
            return i as isize;
        }
    }
    -1
}

// Go: src/strings/strings.go:LastIndexAny
/// LastIndexAny returns the index of the last instance of any Unicode code
/// point from chars in s, or -1 if no Unicode code point from chars is
/// present in s.
pub fn last_index_any(s: &[u8], chars: &[u8]) -> isize {
    if chars.is_empty() {
        // Avoid scanning all of s.
        return -1;
    }
    if s.len() == 1 {
        let mut rc = s[0] as Rune;
        if rc >= RUNE_SELF {
            rc = RUNE_ERROR;
        }
        if index_rune(chars, rc) >= 0 {
            return 0;
        }
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
    if chars.len() == 1 {
        let mut rc = chars[0] as Rune;
        if rc >= RUNE_SELF {
            rc = RUNE_ERROR;
        }
        let mut i = s.len();
        while i > 0 {
            let (r, size) = utf8::decode_last_rune_in_string(&s[..i]);
            i -= size;
            if rc == r {
                return i as isize;
            }
        }
        return -1;
    }
    let mut i = s.len();
    while i > 0 {
        let (r, size) = utf8::decode_last_rune_in_string(&s[..i]);
        i -= size;
        if index_rune(chars, r) >= 0 {
            return i as isize;
        }
    }
    -1
}

// Go: src/strings/strings.go:LastIndexByte
/// LastIndexByte returns the index of the last instance of c in s, or -1 if c is not present in s.
pub fn last_index_byte(s: &[u8], c: u8) -> isize {
    match memchr::memrchr(c, s) {
        Some(i) => i as isize,
        None => -1,
    }
}

// Go: src/strings/strings.go:genSplit
/// Generic split: splits after each instance of sep,
/// including sepSave bytes of sep in the subarrays.
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

// Go: src/strings/strings.go:SplitN
/// SplitN slices s into substrings separated by sep and returns a slice of
/// the substrings between those separators.
///
/// The count determines the number of substrings to return:
///   - n > 0: at most n substrings; the last substring will be the unsplit remainder;
///   - n == 0: the result is nil (zero substrings);
///   - n < 0: all substrings.
pub fn split_n<'a>(s: &'a [u8], sep: &[u8], n: isize) -> Vec<&'a [u8]> {
    gen_split(s, sep, 0, n)
}

// Go: src/strings/strings.go:SplitAfterN
/// SplitAfterN slices s into substrings after each instance of sep and
/// returns a slice of those substrings.
pub fn split_after_n<'a>(s: &'a [u8], sep: &[u8], n: isize) -> Vec<&'a [u8]> {
    gen_split(s, sep, sep.len(), n)
}

// Go: src/strings/strings.go:Split
/// Split slices s into all substrings separated by sep and returns a slice of
/// the substrings between those separators.
///
/// If s does not contain sep and sep is not empty, Split returns a
/// slice of length 1 whose only element is s.
///
/// If sep is empty, Split splits after each UTF-8 sequence. If both s
/// and sep are empty, Split returns an empty slice.
pub fn split<'a>(s: &'a [u8], sep: &[u8]) -> Vec<&'a [u8]> {
    gen_split(s, sep, 0, -1)
}

// Go: src/strings/strings.go:SplitAfter
/// SplitAfter slices s into all substrings after each instance of sep and
/// returns a slice of those substrings.
pub fn split_after<'a>(s: &'a [u8], sep: &[u8]) -> Vec<&'a [u8]> {
    gen_split(s, sep, sep.len(), -1)
}

pub(crate) static ASCII_SPACE: [u8; 256] = {
    let mut t = [0u8; 256];
    t[b'\t' as usize] = 1;
    t[b'\n' as usize] = 1;
    t[0x0B] = 1; // '\v'
    t[0x0C] = 1; // '\f'
    t[b'\r' as usize] = 1;
    t[b' ' as usize] = 1;
    t
};

// Go: src/strings/strings.go:Fields
/// Fields splits the string s around each instance of one or more consecutive white space
/// characters, as defined by [`crate::is_space`], returning a slice of substrings of s or an
/// empty slice if s contains only white space. Every element of the returned slice is
/// non-empty. Unlike [`split`], leading and trailing runs of white space characters
/// are discarded.
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
        // Some runes in the input string are not ASCII.
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

// Go: src/strings/strings.go:FieldsFunc
/// FieldsFunc splits the string s at each run of Unicode code points c satisfying f(c)
/// and returns an array of slices of s. If all code points in s satisfy f(c) or the
/// string is empty, an empty slice is returned. Every element of the returned slice is
/// non-empty. Unlike [`split`], leading and trailing runs of code points satisfying f(c)
/// are discarded.
pub fn fields_func(s: &[u8], mut f: impl FnMut(Rune) -> bool) -> Vec<&[u8]> {
    // A span is used to record a slice of s of the form s[start:end].
    // The start index is inclusive and the end index is exclusive.
    let mut spans: Vec<(usize, usize)> = Vec::with_capacity(32);

    // Find the field start and end indices.
    let mut start: isize = -1; // valid span start if >= 0
    for (end, rune) in utf8::runes(s) {
        if f(rune) {
            if start >= 0 {
                spans.push((start as usize, end));
                // Set start to a negative value.
                start = !start;
            }
        } else if start < 0 {
            start = end as isize;
        }
    }

    // Last field might end at EOF.
    if start >= 0 {
        spans.push((start as usize, s.len()));
    }

    // Create strings from recorded field indices.
    spans.iter().map(|&(a, b)| &s[a..b]).collect()
}

// Go: src/strings/strings.go:Join
/// Join concatenates the elements of its first argument to create a single string. The separator
/// string sep is placed between elements in the resulting string.
pub fn join<T: AsRef<[u8]>>(elems: &[T], sep: &[u8]) -> Vec<u8> {
    match elems.len() {
        0 => return Vec::new(),
        1 => return elems[0].as_ref().to_vec(),
        _ => {}
    }
    let mut n = sep.len() * (elems.len() - 1);
    for elem in elems {
        n += elem.as_ref().len();
    }
    let mut b = Vec::with_capacity(n);
    b.extend_from_slice(elems[0].as_ref());
    for s in &elems[1..] {
        b.extend_from_slice(sep);
        b.extend_from_slice(s.as_ref());
    }
    b
}

// Go: src/strings/strings.go:HasPrefix
/// HasPrefix reports whether the string s begins with prefix.
#[inline]
pub fn has_prefix(s: &[u8], prefix: &[u8]) -> bool {
    s.len() >= prefix.len() && &s[..prefix.len()] == prefix
}

// Go: src/strings/strings.go:HasSuffix
/// HasSuffix reports whether the string s ends with suffix.
#[inline]
pub fn has_suffix(s: &[u8], suffix: &[u8]) -> bool {
    s.len() >= suffix.len() && &s[s.len() - suffix.len()..] == suffix
}

// Go: src/strings/strings.go:Map
/// Map returns a copy of the string s with all its characters modified
/// according to the mapping function. If mapping returns a negative value, the character is
/// dropped from the string with no replacement.
///
/// Invalid UTF-8 bytes are passed to mapping as RuneError and replaced by the
/// encoding of its result (so an invalid byte always changes the string).
pub fn map(mut mapping: impl FnMut(Rune) -> Rune, s: &[u8]) -> Cow<'_, [u8]> {
    // In the worst case, the string can grow when mapped, making
    // things unpleasant. But it's so rare we barge in assuming it's
    // fine. It could also shrink but that falls out naturally.

    // The output buffer b is initialized on demand, the first
    // time a character differs.
    let mut b: Option<Vec<u8>> = None;
    let mut rest: &[u8] = s;

    for (i, c) in utf8::runes(s) {
        let r = mapping(c);
        if r == c && c != RUNE_ERROR {
            continue;
        }

        let width;
        if c == RUNE_ERROR {
            let (c2, w) = utf8::decode_rune_in_string(&s[i..]);
            width = w;
            if width != 1 && r == c2 {
                continue;
            }
        } else {
            width = utf8::rune_len(c) as usize;
        }

        let mut buf = Vec::with_capacity(s.len() + UTF_MAX);
        buf.extend_from_slice(&s[..i]);
        if r >= 0 {
            utf8::append_rune(&mut buf, r);
        }

        rest = &s[i + width..];
        b = Some(buf);
        break;
    }

    // Fast path for unchanged input
    let Some(mut b) = b else {
        // didn't call b.Grow above
        return Cow::Borrowed(s);
    };

    for (_, c) in utf8::runes(rest) {
        let r = mapping(c);

        if r >= 0 {
            // common case
            // Due to inlining, it is more performant to determine if WriteByte should be
            // invoked rather than always call WriteRune
            if r < RUNE_SELF {
                b.push(r as u8);
            } else {
                // r is not an ASCII rune.
                utf8::append_rune(&mut b, r);
            }
        }
    }

    Cow::Owned(b)
}

// Go: src/strings/strings.go:Repeat
/// Repeat returns a new string consisting of count copies of the string s.
///
/// It panics if count is negative or if the result of (len(s) * count)
/// overflows (as Go does).
pub fn repeat(s: &[u8], count: isize) -> Vec<u8> {
    match count {
        0 => return Vec::new(),
        1 => return s.to_vec(),
        _ => {}
    }
    if count < 0 {
        panic!("strings: negative Repeat count");
    }
    let n = s
        .len()
        .checked_mul(count as usize)
        .filter(|&n| n <= isize::MAX as usize)
        .expect("strings: Repeat output length overflow");
    if s.is_empty() {
        return Vec::new();
    }
    // Go allocates the result up front (Builder.Grow / MakeNoZero) and
    // panics with "makeslice: len out of range" when that is impossible; a
    // failed allocation panics here too instead of aborting the process.
    let mut b = Vec::new();
    if b.try_reserve_exact(n).is_err() {
        panic!("makeslice: len out of range");
    }
    for _ in 0..count {
        b.extend_from_slice(s);
    }
    b
}

// Go: src/strings/strings.go:ToUpper
/// ToUpper returns s with all Unicode letters mapped to their upper case.
pub fn to_upper(s: &[u8]) -> Cow<'_, [u8]> {
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
        // optimize for ASCII-only strings.
        if !has_lower {
            return Cow::Borrowed(s);
        }
        let mut b = Vec::with_capacity(s.len());
        let mut pos = 0;
        for i in 0..s.len() {
            let mut c = s[i];
            if c.is_ascii_lowercase() {
                c -= b'a' - b'A';
                if pos < i {
                    b.extend_from_slice(&s[pos..i]);
                }
                b.push(c);
                pos = i + 1;
            }
        }
        if pos < s.len() {
            b.extend_from_slice(&s[pos..]);
        }
        return Cow::Owned(b);
    }
    map(crate::to_upper, s)
}

// Go: src/strings/strings.go:ToLower
/// ToLower returns s with all Unicode letters mapped to their lower case.
pub fn to_lower(s: &[u8]) -> Cow<'_, [u8]> {
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
        // optimize for ASCII-only strings.
        if !has_upper {
            return Cow::Borrowed(s);
        }
        let mut b = Vec::with_capacity(s.len());
        let mut pos = 0;
        for i in 0..s.len() {
            let mut c = s[i];
            if c.is_ascii_uppercase() {
                c += b'a' - b'A';
                if pos < i {
                    b.extend_from_slice(&s[pos..i]);
                }
                b.push(c);
                pos = i + 1;
            }
        }
        if pos < s.len() {
            b.extend_from_slice(&s[pos..]);
        }
        return Cow::Owned(b);
    }
    map(crate::to_lower, s)
}

// Go: src/strings/strings.go:ToTitle
/// ToTitle returns a copy of the string s with all Unicode letters mapped to
/// their Unicode title case.
pub fn to_title(s: &[u8]) -> Cow<'_, [u8]> {
    map(crate::to_title, s)
}

// Go: src/strings/strings.go:ToUpperSpecial
/// ToUpperSpecial returns a copy of the string s with all Unicode letters mapped to their
/// upper case using the case mapping specified by c.
pub fn to_upper_special<'a>(c: SpecialCase<'_>, s: &'a [u8]) -> Cow<'a, [u8]> {
    map(|r| c.to_upper(r), s)
}

// Go: src/strings/strings.go:ToLowerSpecial
/// ToLowerSpecial returns a copy of the string s with all Unicode letters mapped to their
/// lower case using the case mapping specified by c.
pub fn to_lower_special<'a>(c: SpecialCase<'_>, s: &'a [u8]) -> Cow<'a, [u8]> {
    map(|r| c.to_lower(r), s)
}

// Go: src/strings/strings.go:ToTitleSpecial
/// ToTitleSpecial returns a copy of the string s with all Unicode letters mapped to their
/// Unicode title case, giving priority to the special casing rules.
pub fn to_title_special<'a>(c: SpecialCase<'_>, s: &'a [u8]) -> Cow<'a, [u8]> {
    map(|r| c.to_title(r), s)
}

// Go: src/strings/strings.go:ToValidUTF8
/// ToValidUTF8 returns a copy of the string s with each run of invalid UTF-8 byte sequences
/// replaced by the replacement string, which may be empty.
pub fn to_valid_utf8<'a>(s: &'a [u8], replacement: &[u8]) -> Cow<'a, [u8]> {
    let mut b: Option<Vec<u8>> = None;
    let mut rest = s;

    for (i, c) in utf8::runes(s) {
        if c != RUNE_ERROR {
            continue;
        }

        let (_, wid) = utf8::decode_rune_in_string(&s[i..]);
        if wid == 1 {
            let mut buf = Vec::with_capacity(s.len() + replacement.len());
            buf.extend_from_slice(&s[..i]);
            rest = &s[i..];
            b = Some(buf);
            break;
        }
    }

    // Fast path for unchanged input
    let Some(mut b) = b else {
        // didn't call b.Grow above
        return Cow::Borrowed(s);
    };
    let s = rest;

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
        let (_, wid) = utf8::decode_rune_in_string(&s[i..]);
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

    Cow::Owned(b)
}

// Go: src/strings/strings.go:isSeparator
/// isSeparator reports whether the rune could mark a word boundary.
pub(crate) fn is_separator(r: Rune) -> bool {
    // ASCII alphanumerics and underscore are not separators
    if r <= 0x7F {
        if ('0' as Rune..='9' as Rune).contains(&r)
            || ('a' as Rune..='z' as Rune).contains(&r)
            || ('A' as Rune..='Z' as Rune).contains(&r)
            || r == '_' as Rune
        {
            return false;
        }
        return true;
    }
    // Letters and digits are not separators
    if crate::is_letter(r) || crate::is_digit(r) {
        return false;
    }
    // Otherwise, all we can do for now is treat spaces as separators.
    crate::is_space(r)
}

// Go: src/strings/strings.go:Title
/// Title returns a copy of the string s with all Unicode letters that begin words
/// mapped to their Unicode title case.
///
/// Deprecated in Go: The rule Title uses for word boundaries does not handle Unicode
/// punctuation properly.
pub fn title(s: &[u8]) -> Cow<'_, [u8]> {
    // Use a closure here to remember state.
    // Hackish but effective. Depends on Map scanning in order and calling
    // the closure once per rune.
    let mut prev: Rune = ' ' as Rune;
    map(
        |r| {
            if is_separator(prev) {
                prev = r;
                return crate::to_title(r);
            }
            prev = r;
            r
        },
        s,
    )
}

// Go: src/strings/strings.go:TrimLeftFunc
/// TrimLeftFunc returns a slice of the string s with all leading
/// Unicode code points c satisfying f(c) removed.
pub fn trim_left_func(s: &[u8], f: impl FnMut(Rune) -> bool) -> &[u8] {
    let i = index_func_impl(s, f, false);
    if i == -1 {
        return &s[s.len()..];
    }
    &s[i as usize..]
}

// Go: src/strings/strings.go:TrimRightFunc
/// TrimRightFunc returns a slice of the string s with all trailing
/// Unicode code points c satisfying f(c) removed.
pub fn trim_right_func(s: &[u8], f: impl FnMut(Rune) -> bool) -> &[u8] {
    let mut i = last_index_func_impl(s, f, false);
    if i >= 0 {
        let (_, wid) = utf8::decode_rune_in_string(&s[i as usize..]);
        i += wid as isize;
    } else {
        i += 1;
    }
    &s[0..i as usize]
}

// Go: src/strings/strings.go:TrimFunc
/// TrimFunc returns a slice of the string s with all leading
/// and trailing Unicode code points c satisfying f(c) removed.
pub fn trim_func(s: &[u8], mut f: impl FnMut(Rune) -> bool) -> &[u8] {
    trim_right_func(trim_left_func(s, &mut f), &mut f)
}

// Go: src/strings/strings.go:IndexFunc
/// IndexFunc returns the index into s of the first Unicode
/// code point satisfying f(c), or -1 if none do.
pub fn index_func(s: &[u8], f: impl FnMut(Rune) -> bool) -> isize {
    index_func_impl(s, f, true)
}

// Go: src/strings/strings.go:LastIndexFunc
/// LastIndexFunc returns the index into s of the last
/// Unicode code point satisfying f(c), or -1 if none do.
pub fn last_index_func(s: &[u8], f: impl FnMut(Rune) -> bool) -> isize {
    last_index_func_impl(s, f, true)
}

// Go: src/strings/strings.go:indexFunc
/// indexFunc is the same as IndexFunc except that if
/// truth==false, the sense of the predicate function is
/// inverted.
fn index_func_impl(s: &[u8], mut f: impl FnMut(Rune) -> bool, truth: bool) -> isize {
    for (i, r) in utf8::runes(s) {
        if f(r) == truth {
            return i as isize;
        }
    }
    -1
}

// Go: src/strings/strings.go:lastIndexFunc
/// lastIndexFunc is the same as LastIndexFunc except that if
/// truth==false, the sense of the predicate function is
/// inverted.
fn last_index_func_impl(s: &[u8], mut f: impl FnMut(Rune) -> bool, truth: bool) -> isize {
    let mut i = s.len();
    while i > 0 {
        let (r, size) = utf8::decode_last_rune_in_string(&s[0..i]);
        i -= size;
        if f(r) == truth {
            return i as isize;
        }
    }
    -1
}

/// asciiSet is a 256-byte lookup table for fast ASCII character membership testing.
pub(crate) struct AsciiSet([bool; 256]);

impl AsciiSet {
    // Go: src/strings/strings.go:asciiSet.contains
    /// contains reports whether c is inside the set.
    #[inline]
    pub(crate) fn contains(&self, c: u8) -> bool {
        self.0[c as usize]
    }
}

// Go: src/strings/strings.go:makeASCIISet
/// makeASCIISet creates a set of ASCII characters and reports whether all
/// characters in chars are ASCII.
pub(crate) fn make_ascii_set(chars: &[u8]) -> Option<AsciiSet> {
    let mut as_ = [false; 256];
    for i in 0..chars.len() {
        let c = chars[i];
        if c as Rune >= RUNE_SELF {
            return None;
        }
        as_[c as usize] = true;
    }
    Some(AsciiSet(as_))
}

// Go: src/strings/strings.go:shouldUseASCIISet
/// shouldUseASCIISet returns whether to use the lookup table optimization.
#[inline]
pub(crate) fn should_use_ascii_set(buf_len: usize) -> bool {
    buf_len > 8
}

// Go: src/strings/strings.go:Trim
/// Trim returns a slice of the string s with all leading and
/// trailing Unicode code points contained in cutset removed.
pub fn trim<'a>(s: &'a [u8], cutset: &[u8]) -> &'a [u8] {
    if s.is_empty() || cutset.is_empty() {
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

// Go: src/strings/strings.go:TrimLeft
/// TrimLeft returns a slice of the string s with all leading
/// Unicode code points contained in cutset removed.
pub fn trim_left<'a>(s: &'a [u8], cutset: &[u8]) -> &'a [u8] {
    if s.is_empty() || cutset.is_empty() {
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

// Go: src/strings/strings.go:trimLeftByte
fn trim_left_byte(mut s: &[u8], c: u8) -> &[u8] {
    while !s.is_empty() && s[0] == c {
        s = &s[1..];
    }
    s
}

// Go: src/strings/strings.go:trimLeftASCII
fn trim_left_ascii<'a>(mut s: &'a [u8], as_: &AsciiSet) -> &'a [u8] {
    while !s.is_empty() {
        if !as_.contains(s[0]) {
            break;
        }
        s = &s[1..];
    }
    s
}

// Go: src/strings/strings.go:trimLeftUnicode
fn trim_left_unicode<'a>(mut s: &'a [u8], cutset: &[u8]) -> &'a [u8] {
    while !s.is_empty() {
        let (r, n) = utf8::decode_rune_in_string(s);
        if !contains_rune(cutset, r) {
            break;
        }
        s = &s[n..];
    }
    s
}

// Go: src/strings/strings.go:TrimRight
/// TrimRight returns a slice of the string s, with all trailing
/// Unicode code points contained in cutset removed.
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

// Go: src/strings/strings.go:trimRightByte
fn trim_right_byte(mut s: &[u8], c: u8) -> &[u8] {
    while !s.is_empty() && s[s.len() - 1] == c {
        s = &s[..s.len() - 1];
    }
    s
}

// Go: src/strings/strings.go:trimRightASCII
fn trim_right_ascii<'a>(mut s: &'a [u8], as_: &AsciiSet) -> &'a [u8] {
    while !s.is_empty() {
        if !as_.contains(s[s.len() - 1]) {
            break;
        }
        s = &s[..s.len() - 1];
    }
    s
}

// Go: src/strings/strings.go:trimRightUnicode
fn trim_right_unicode<'a>(mut s: &'a [u8], cutset: &[u8]) -> &'a [u8] {
    while !s.is_empty() {
        let (mut r, mut n) = (s[s.len() - 1] as Rune, 1);
        if r >= RUNE_SELF {
            (r, n) = utf8::decode_last_rune_in_string(s);
        }
        if !contains_rune(cutset, r) {
            break;
        }
        s = &s[..s.len() - n];
    }
    s
}

// Go: src/strings/strings.go:TrimSpace
/// TrimSpace returns a slice (substring) of the string s,
/// with all leading and trailing white space removed,
/// as defined by Unicode.
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
                return trim_right_func(&s[..hi + 1], crate::is_space);
            }
            if ASCII_SPACE[c as usize] == 0 {
                // At this point, s[:hi+1] starts and ends with ASCII
                // non-space bytes, so we're done. Non-ASCII cases have
                // already been handled above.
                return &s[..hi + 1];
            }
        }
    }
    &s[s.len()..]
}

// Go: src/strings/strings.go:TrimPrefix
/// TrimPrefix returns s without the provided leading prefix string.
/// If s doesn't start with prefix, s is returned unchanged.
pub fn trim_prefix<'a>(s: &'a [u8], prefix: &[u8]) -> &'a [u8] {
    if has_prefix(s, prefix) {
        return &s[prefix.len()..];
    }
    s
}

// Go: src/strings/strings.go:TrimSuffix
/// TrimSuffix returns s without the provided trailing suffix string.
/// If s doesn't end with suffix, s is returned unchanged.
pub fn trim_suffix<'a>(s: &'a [u8], suffix: &[u8]) -> &'a [u8] {
    if has_suffix(s, suffix) {
        return &s[..s.len() - suffix.len()];
    }
    s
}

// Go: src/strings/strings.go:Replace
/// Replace returns a copy of the string s with the first n
/// non-overlapping instances of old replaced by new.
/// If old is empty, it matches at the beginning of the string
/// and after each UTF-8 sequence, yielding up to k+1 replacements
/// for a k-rune string.
/// If n < 0, there is no limit on the number of replacements.
pub fn replace<'a>(s: &'a [u8], old: &[u8], new: &[u8], mut n: isize) -> Cow<'a, [u8]> {
    if old == new || n == 0 {
        return Cow::Borrowed(s); // avoid allocation
    }

    // Compute number of replacements.
    let m = count(s, old) as isize;
    if m == 0 {
        return Cow::Borrowed(s); // avoid allocation
    } else if n < 0 || m < n {
        n = m;
    }

    // Apply replacements to buffer.
    let mut b = Vec::with_capacity(
        (s.len() as isize + n * (new.len() as isize - old.len() as isize)).max(0) as usize,
    );
    let mut start = 0usize;
    if !old.is_empty() {
        for _ in 0..n {
            let j = start + index(&s[start..], old) as usize;
            b.extend_from_slice(&s[start..j]);
            b.extend_from_slice(new);
            start = j + old.len();
        }
    } else {
        // len(old) == 0
        b.extend_from_slice(new);
        for _ in 0..n - 1 {
            let (_, wid) = utf8::decode_rune_in_string(&s[start..]);
            let j = start + wid;
            b.extend_from_slice(&s[start..j]);
            b.extend_from_slice(new);
            start = j;
        }
    }
    b.extend_from_slice(&s[start..]);
    Cow::Owned(b)
}

// Go: src/strings/strings.go:ReplaceAll
/// ReplaceAll returns a copy of the string s with all
/// non-overlapping instances of old replaced by new.
pub fn replace_all<'a>(s: &'a [u8], old: &[u8], new: &[u8]) -> Cow<'a, [u8]> {
    replace(s, old, new, -1)
}

// Go: src/strings/strings.go:EqualFold
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
    let s = &s[i..];
    let mut t = &t[i..];
    for (_, sr) in utf8::runes(s) {
        let mut sr = sr;
        // If t is exhausted the strings are not equal.
        if t.is_empty() {
            return false;
        }

        // Extract first rune from second string.
        let (mut tr, size) = utf8::decode_rune_in_string(t);
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

    // First string is empty, so check if the second one is also empty.
    t.is_empty()
}

// Go: src/strings/strings.go:Index (internal/stringslite.Index)
/// Index returns the index of the first instance of substr in s, or -1 if substr is not present in s.
#[inline]
pub fn index(s: &[u8], substr: &[u8]) -> isize {
    let n = substr.len();
    if n == 0 {
        return 0;
    } else if n == 1 {
        return index_byte(s, substr[0]);
    } else if n == s.len() {
        if substr == s {
            return 0;
        }
        return -1;
    } else if n > s.len() {
        return -1;
    }
    match memchr::memmem::find(s, substr) {
        Some(i) => i as isize,
        None => -1,
    }
}

// Go: src/strings/strings.go:Cut (internal/stringslite.Cut)
/// Cut slices s around the first instance of sep,
/// returning the text before and after sep.
/// The found result reports whether sep appears in s.
/// If sep does not appear in s, cut returns s, "", false.
pub fn cut<'a>(s: &'a [u8], sep: &[u8]) -> (&'a [u8], &'a [u8], bool) {
    let i = index(s, sep);
    if i >= 0 {
        let i = i as usize;
        return (&s[..i], &s[i + sep.len()..], true);
    }
    (s, &s[s.len()..], false)
}

// Go: src/strings/strings.go:CutPrefix (internal/stringslite.CutPrefix)
/// CutPrefix returns s without the provided leading prefix string
/// and reports whether it found the prefix.
pub fn cut_prefix<'a>(s: &'a [u8], prefix: &[u8]) -> (&'a [u8], bool) {
    if !has_prefix(s, prefix) {
        return (s, false);
    }
    (&s[prefix.len()..], true)
}

// Go: src/strings/strings.go:CutSuffix (internal/stringslite.CutSuffix)
/// CutSuffix returns s without the provided ending suffix string
/// and reports whether it found the suffix.
pub fn cut_suffix<'a>(s: &'a [u8], suffix: &[u8]) -> (&'a [u8], bool) {
    if !has_suffix(s, suffix) {
        return (s, false);
    }
    (&s[..s.len() - suffix.len()], true)
}

// Go: src/strings/strings.go:CutLast
/// CutLast slices s around the last instance of sep,
/// returning the text before and after sep.
pub fn cut_last<'a>(s: &'a [u8], sep: &[u8]) -> (&'a [u8], &'a [u8], bool) {
    let i = last_index(s, sep);
    if i >= 0 {
        let i = i as usize;
        return (&s[..i], &s[i + sep.len()..], true);
    }
    (s, &s[s.len()..], false)
}

// Go: src/strings/compare.go:Compare
/// Compare returns an integer comparing two strings lexicographically.
/// The result will be 0 if a == b, -1 if a < b, and +1 if a > b.
pub fn compare(a: &[u8], b: &[u8]) -> isize {
    match a.cmp(b) {
        Ordering::Less => -1,
        Ordering::Equal => 0,
        Ordering::Greater => 1,
    }
}

// ---------------------------------------------------------------------------
// Conveniences for valid-UTF-8 Rust strings. Every function above maps valid
// UTF-8 to valid UTF-8 at rune boundaries, so these only re-type the result.

fn sub_str<'a>(s: &'a str, sub: &[u8]) -> &'a str {
    let off = sub.as_ptr() as usize - s.as_ptr() as usize;
    &s[off..off + sub.len()]
}

fn cow_str(c: Cow<'_, [u8]>) -> Cow<'_, str> {
    match c {
        Cow::Borrowed(b) => {
            Cow::Borrowed(std::str::from_utf8(b).expect("valid UTF-8 in, valid UTF-8 out"))
        }
        Cow::Owned(v) => Cow::Owned(String::from_utf8(v).expect("valid UTF-8 in, valid UTF-8 out")),
    }
}

/// [`to_lower`] for `&str`.
pub fn to_lower_str(s: &str) -> Cow<'_, str> {
    cow_str(to_lower(s.as_bytes()))
}

/// [`to_upper`] for `&str`.
pub fn to_upper_str(s: &str) -> Cow<'_, str> {
    cow_str(to_upper(s.as_bytes()))
}

/// [`to_title`] for `&str`.
pub fn to_title_str(s: &str) -> Cow<'_, str> {
    cow_str(to_title(s.as_bytes()))
}

/// [`title`] for `&str`.
pub fn title_str(s: &str) -> Cow<'_, str> {
    cow_str(title(s.as_bytes()))
}

/// [`map`] for `&str`.
pub fn map_str(mapping: impl FnMut(Rune) -> Rune, s: &str) -> Cow<'_, str> {
    cow_str(map(mapping, s.as_bytes()))
}

/// [`trim_space`] for `&str`.
pub fn trim_space_str(s: &str) -> &str {
    sub_str(s, trim_space(s.as_bytes()))
}

/// [`trim_func`] for `&str`.
pub fn trim_func_str(s: &str, f: impl FnMut(Rune) -> bool) -> &str {
    sub_str(s, trim_func(s.as_bytes(), f))
}

/// [`trim_left_func`] for `&str`.
pub fn trim_left_func_str(s: &str, f: impl FnMut(Rune) -> bool) -> &str {
    sub_str(s, trim_left_func(s.as_bytes(), f))
}

/// [`trim_right_func`] for `&str`.
pub fn trim_right_func_str(s: &str, f: impl FnMut(Rune) -> bool) -> &str {
    sub_str(s, trim_right_func(s.as_bytes(), f))
}

/// [`fields`] for `&str`.
pub fn fields_str(s: &str) -> Vec<&str> {
    fields(s.as_bytes())
        .into_iter()
        .map(|f| sub_str(s, f))
        .collect()
}

/// [`fields_func`] for `&str`.
pub fn fields_func_str(s: &str, f: impl FnMut(Rune) -> bool) -> Vec<&str> {
    fields_func(s.as_bytes(), f)
        .into_iter()
        .map(|x| sub_str(s, x))
        .collect()
}

/// [`equal_fold`] for `&str`.
pub fn equal_fold_str(s: &str, t: &str) -> bool {
    equal_fold(s.as_bytes(), t.as_bytes())
}
