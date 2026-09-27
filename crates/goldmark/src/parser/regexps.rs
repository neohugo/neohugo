//! Hand-written equivalents of goldmark's regular expressions
//! (parser/html_block.go, parser/raw_html.go). Each function documents the
//! Go pattern it replaces; the equivalence (match / no match, capture groups
//! used by goldmark, match length) is checked against Go's `regexp` by the
//! `regex.gmf.gz` oracle vectors. See PORTING.md, "regular expressions".

use go_unicode::{Rune, simple_fold, utf8};

use crate::text::RuneStream;

/// Go regexp `(?i)` literal matching: `r` equals `p` or is in its
/// `unicode.SimpleFold` orbit.
fn fold_eq(p: Rune, r: Rune) -> bool {
    if r == p {
        return true;
    }
    let mut f = simple_fold(p);
    while f != p {
        if f == r {
            return true;
        }
        f = simple_fold(f);
    }
    false
}

/// Matches the ASCII literal `lit` case-insensitively (Go `(?i)`) at
/// `line[i..]`, decoding runes as Go's regexp does; returns the end offset.
fn fold_match(line: &[u8], mut i: usize, lit: &[u8]) -> Option<usize> {
    for &p in lit {
        if i >= line.len() {
            return None;
        }
        let (r, size) = utf8::decode_rune(&line[i..]);
        if !fold_eq(p as Rune, r) {
            return None;
        }
        i += size;
    }
    Some(i)
}

/// `^[ ]{0,3}<` : the index after `<`.
fn indent_lt(line: &[u8]) -> Option<usize> {
    let mut i = 0;
    while i < 3 && i < line.len() && line[i] == b' ' {
        i += 1;
    }
    if i < line.len() && line[i] == b'<' {
        return Some(i + 1);
    }
    None
}

/// `(?:X.*|>.*|/>.*|)(?:\r\n|\n)?$` where X is a single character accepted
/// by `first` (`.` never matches `\n`).
fn rest_ok(rest: &[u8], first: impl Fn(u8) -> bool) -> bool {
    for b in [&b""[..], b"\n", b"\r\n"] {
        if !rest.ends_with(b) {
            continue;
        }
        let body = &rest[..rest.len() - b.len()];
        if body.is_empty() {
            return true;
        }
        if (first(body[0]) || body[0] == b'>') && !body[1..].contains(&b'\n') {
            return true;
        }
        if body.starts_with(b"/>") && !body[2..].contains(&b'\n') {
            return true;
        }
    }
    false
}

/// Go `\s` (Perl class): `[\t\n\f\r ]`.
fn is_perl_space(c: u8) -> bool {
    matches!(c, b'\t' | b'\n' | b'\x0c' | b'\r' | b' ')
}

const TYPE1_TAGS: [&[u8]; 4] = [b"script", b"pre", b"style", b"textarea"];

/// `(?i)^[ ]{0,3}<(script|pre|style|textarea)(?:\s.*|>.*|/>.*|)(?:\r\n|\n)?$`
pub fn html_block_type1_open(line: &[u8]) -> bool {
    let Some(i) = indent_lt(line) else {
        return false;
    };
    for tag in TYPE1_TAGS {
        if let Some(j) = fold_match(line, i, tag)
            && rest_ok(&line[j..], is_perl_space)
        {
            return true;
        }
    }
    false
}

/// `(?i)^.*</(?:script|pre|style|textarea)>.*`
pub fn html_block_type1_close(line: &[u8]) -> bool {
    let end = line.iter().position(|&c| c == b'\n').unwrap_or(line.len());
    let mut k = 0;
    while k + 1 < end {
        if line[k] == b'<' && line[k + 1] == b'/' {
            for tag in TYPE1_TAGS {
                if let Some(j) = fold_match(line, k + 2, tag)
                    && j < line.len()
                    && line[j] == b'>'
                {
                    return true;
                }
            }
        }
        k += 1;
    }
    false
}

/// `^[ ]{0,3}<!\-\-`
pub fn html_block_type2_open(line: &[u8]) -> bool {
    matches!(indent_lt(line), Some(i) if line[i..].starts_with(b"!--"))
}

/// `^[ ]{0,3}<\?`
pub fn html_block_type3_open(line: &[u8]) -> bool {
    matches!(indent_lt(line), Some(i) if line[i..].starts_with(b"?"))
}

/// `^[ ]{0,3}<![A-Z]+.*(?:\r\n|\n)?$`
pub fn html_block_type4_open(line: &[u8]) -> bool {
    let Some(i) = indent_lt(line) else {
        return false;
    };
    if !(i + 1 < line.len() && line[i] == b'!' && line[i + 1].is_ascii_uppercase()) {
        return false;
    }
    let rest = &line[i + 2..];
    for b in [&b""[..], b"\n", b"\r\n"] {
        if rest.ends_with(b) && !rest[..rest.len() - b.len()].contains(&b'\n') {
            return true;
        }
    }
    false
}

/// `^[ ]{0,3}<\!\[CDATA\[`
pub fn html_block_type5_open(line: &[u8]) -> bool {
    matches!(indent_lt(line), Some(i) if line[i..].starts_with(b"![CDATA["))
}

/// `[a-zA-Z]+[a-zA-Z0-9\-]*` at `line[i..]` (maximal; the only viable
/// length in both patterns that use it): the end index.
fn tag_name(line: &[u8], i: usize) -> Option<usize> {
    if i >= line.len() || !line[i].is_ascii_alphabetic() {
        return None;
    }
    let mut j = i + 1;
    while j < line.len() && (line[j].is_ascii_alphanumeric() || line[j] == b'-') {
        j += 1;
    }
    Some(j)
}

/// `^[ ]{0,3}<(?:/[ ]*)?([a-zA-Z]+[a-zA-Z0-9\-]*)(?:[ ].*|>.*|/>.*|)(?:\r\n|\n)?$`:
/// the capture group 1 (tag name) range.
pub fn html_block_type6(line: &[u8]) -> Option<(usize, usize)> {
    let mut i = indent_lt(line)?;
    if i < line.len() && line[i] == b'/' {
        i += 1;
        while i < line.len() && line[i] == b' ' {
            i += 1;
        }
    }
    let j = tag_name(line, i)?;
    if rest_ok(&line[j..], |c| c == b' ') {
        return Some((i, j));
    }
    None
}

/// A byte source for the attribute scanner: a line or a [`RuneStream`].
trait Bytes {
    fn at(&mut self, i: usize) -> Option<u8>;
}

impl Bytes for &[u8] {
    fn at(&mut self, i: usize) -> Option<u8> {
        self.get(i).copied()
    }
}

impl Bytes for RuneStream<'_> {
    fn at(&mut self, i: usize) -> Option<u8> {
        RuneStream::at(self, i)
    }
}

fn is_attr_ws(c: u8) -> bool {
    matches!(c, b'\r' | b'\n' | b' ' | b'\t')
}

fn is_attr_name_start(c: u8) -> bool {
    c.is_ascii_alphabetic() || c == b'_' || c == b':'
}

fn is_attr_name_char(c: u8) -> bool {
    c.is_ascii_alphanumeric() || c == b':' || c == b'.' || c == b'_' || c == b'-'
}

fn is_unquoted_value_char(c: u8) -> bool {
    // [^\"'=<>`\x00-\x20]
    !(matches!(c, b'"' | b'\'' | b'=' | b'<' | b'>' | b'`') || c <= 0x20)
}

/// One `attributePattern` at `i`:
/// `(?:[\r\n \t]+[a-zA-Z_:][a-zA-Z0-9:._-]*(?:[\r\n \t]*=[\r\n \t]*(?:[^\"'=<>`\x00-\x20]+|'[^']*'|"[^"]*"))?)`
/// Returns the end index. Greedy parsing decides the match (see PORTING.md).
fn attribute<B: Bytes>(b: &mut B, mut i: usize) -> Option<usize> {
    let start = i;
    while matches!(b.at(i), Some(c) if is_attr_ws(c)) {
        i += 1;
    }
    if i == start {
        return None;
    }
    if !matches!(b.at(i), Some(c) if is_attr_name_start(c)) {
        return None;
    }
    i += 1;
    while matches!(b.at(i), Some(c) if is_attr_name_char(c)) {
        i += 1;
    }
    // optional value
    let name_end = i;
    let mut k = i;
    while matches!(b.at(k), Some(c) if is_attr_ws(c)) {
        k += 1;
    }
    if b.at(k) != Some(b'=') {
        return Some(name_end);
    }
    k += 1;
    while matches!(b.at(k), Some(c) if is_attr_ws(c)) {
        k += 1;
    }
    match b.at(k) {
        Some(q @ (b'\'' | b'"')) => {
            let mut m = k + 1;
            loop {
                match b.at(m) {
                    None => return Some(name_end),
                    Some(c) if c == q => return Some(m + 1),
                    Some(_) => m += 1,
                }
            }
        }
        Some(c) if is_unquoted_value_char(c) => {
            let mut m = k + 1;
            while matches!(b.at(m), Some(c) if is_unquoted_value_char(c)) {
                m += 1;
            }
            Some(m)
        }
        _ => Some(name_end),
    }
}

/// `attributePattern*` from `i`: the end index.
fn attributes<B: Bytes>(b: &mut B, mut i: usize) -> usize {
    while let Some(e) = attribute(b, i) {
        i = e;
    }
    i
}

/// Type 7 result: (isCloseTag, hasAttr, tag name range).
pub struct Type7 {
    pub is_close_tag: bool,
    pub has_attr: bool,
    pub tag: (usize, usize),
}

/// `^[ ]{0,3}<(/[ ]*)?([a-zA-Z]+[a-zA-Z0-9\-]*)(` + attributePattern + `*)[ ]*(?:>|/>)[ ]*(?:\r\n|\n)?$`
pub fn html_block_type7(line: &[u8]) -> Option<Type7> {
    let mut i = indent_lt(line)?;
    let mut group1: Option<(usize, usize)> = None;
    if i < line.len() && line[i] == b'/' {
        let s = i;
        i += 1;
        while i < line.len() && line[i] == b' ' {
            i += 1;
        }
        group1 = Some((s, i));
    }
    let tag_start = i;
    let tag_end = tag_name(line, i)?;
    let mut b: &[u8] = line;
    let attrs_end = attributes(&mut b, tag_end);
    let mut k = attrs_end;
    while k < line.len() && line[k] == b' ' {
        k += 1;
    }
    if k < line.len() && line[k] == b'>' {
        k += 1;
    } else if k + 1 < line.len() && line[k] == b'/' && line[k + 1] == b'>' {
        k += 2;
    } else {
        return None;
    }
    while k < line.len() && line[k] == b' ' {
        k += 1;
    }
    let rest = &line[k..];
    if !(rest.is_empty() || rest == b"\n" || rest == b"\r\n") {
        return None;
    }
    Some(Type7 {
        is_close_tag: matches!(group1, Some((s, e)) if &line[s..e] == b"/"),
        has_attr: attrs_end != tag_end,
        tag: (tag_start, tag_end),
    })
}

/// `(?:[ \t]|(?:\r\n|\n){0,1})*` from `i`: the end index.
fn space_or_one_newline<B: Bytes>(b: &mut B, mut i: usize) -> usize {
    loop {
        match b.at(i) {
            Some(b' ' | b'\t' | b'\n') => i += 1,
            Some(b'\r') if b.at(i + 1) == Some(b'\n') => i += 2,
            _ => return i,
        }
    }
}

/// `([A-Za-z][A-Za-z0-9-]*)` at `i` over a byte source.
fn tagname_pattern<B: Bytes>(b: &mut B, i: usize) -> Option<usize> {
    if !matches!(b.at(i), Some(c) if c.is_ascii_alphabetic()) {
        return None;
    }
    let mut j = i + 1;
    while matches!(b.at(j), Some(c) if c.is_ascii_alphanumeric() || c == b'-') {
        j += 1;
    }
    Some(j)
}

/// `openTagRegexp`: `"^<" + tagnamePattern + attributePattern + "*" + spaceOrOneNewline + "*/?>"`
/// over the reader's runes; the match length.
pub fn open_tag(s: &mut RuneStream<'_>) -> Option<i64> {
    if s.at(0) != Some(b'<') {
        return None;
    }
    let i = tagname_pattern(s, 1)?;
    let i = attributes(s, i);
    let mut i = space_or_one_newline(s, i);
    if s.at(i) == Some(b'/') && s.at(i + 1) == Some(b'>') {
        i += 2;
    } else if s.at(i) == Some(b'>') {
        i += 1;
    } else {
        return None;
    }
    Some(i as i64)
}

/// `closeTagRegexp`: `"^</" + tagnamePattern + spaceOrOneNewline + "*>"`
/// over the reader's runes; the match length.
pub fn close_tag(s: &mut RuneStream<'_>) -> Option<i64> {
    if s.at(0) != Some(b'<') || s.at(1) != Some(b'/') {
        return None;
    }
    let i = tagname_pattern(s, 2)?;
    let i = space_or_one_newline(s, i);
    if s.at(i) == Some(b'>') {
        return Some((i + 1) as i64);
    }
    None
}
