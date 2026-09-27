//! Go: github.com/yuin/goldmark@v1.7.12/util — utility functions for goldmark.

use std::any::Any;
use std::borrow::Cow;

use go_unicode::Rune;
use go_unicode::utf8;

mod html5entities;
#[rustfmt::skip]
mod html5entities_gen;
mod tables;
mod unicode_case_folding;
#[rustfmt::skip]
mod unicode_case_folding_gen;
mod util_cjk;

pub use html5entities::{HTML5Entity, lookup_html5_entity_by_name};
pub use util_cjk::{east_asian_width, is_east_asian_wide_rune, is_space_discarding_unicode_rune};

use tables::{EMAIL_TABLE, PUNCT_TABLE, SPACE_TABLE, URL_ESCAPE_TABLE, URL_TABLE, UTF8LEN_TABLE};

/// Go `bytes.IndexByte` with Go's `int` result (-1 when absent).
pub(crate) fn index_byte(b: &[u8], c: u8) -> i64 {
    match b.iter().position(|&x| x == c) {
        Some(i) => i as i64,
        None => -1,
    }
}

/// Go `bytes.Index` with Go's `int` result (-1 when absent).
pub(crate) fn index(s: &[u8], sep: &[u8]) -> i64 {
    go_unicode::bytes::index(s, sep) as i64
}

/// A CopyOnWriteBuffer is a byte buffer that copies buffer when
/// it need to be changed.
pub struct CopyOnWriteBuffer<'a> {
    buffer: Cow<'a, [u8]>,
    copied: bool,
}

impl<'a> CopyOnWriteBuffer<'a> {
    // Go: util/util.go:NewCopyOnWriteBuffer
    /// NewCopyOnWriteBuffer returns a new CopyOnWriteBuffer.
    pub fn new(buffer: &'a [u8]) -> Self {
        CopyOnWriteBuffer {
            buffer: Cow::Borrowed(buffer),
            copied: false,
        }
    }

    // Go: util/util.go:CopyOnWriteBuffer.Write
    /// Write writes given bytes to the buffer.
    /// Write allocate new buffer and clears it at the first time.
    pub fn write(&mut self, value: &[u8]) {
        if !self.copied {
            self.buffer = Cow::Owned(Vec::with_capacity(self.buffer.len() + 20));
            self.copied = true;
        }
        self.buffer.to_mut().extend_from_slice(value);
    }

    // Go: util/util.go:CopyOnWriteBuffer.Append
    /// Append appends given bytes to the buffer.
    /// Append copy buffer at the first time.
    pub fn append(&mut self, value: &[u8]) {
        if !self.copied {
            let mut tmp = Vec::with_capacity(self.buffer.len() + 20);
            tmp.extend_from_slice(&self.buffer);
            self.buffer = Cow::Owned(tmp);
            self.copied = true;
        }
        self.buffer.to_mut().extend_from_slice(value);
    }

    // Go: util/util.go:CopyOnWriteBuffer.WriteByte
    /// WriteByte writes the given byte to the buffer.
    /// WriteByte allocate new buffer and clears it at the first time.
    pub fn write_byte(&mut self, c: u8) {
        if !self.copied {
            self.buffer = Cow::Owned(Vec::with_capacity(self.buffer.len() + 20));
            self.copied = true;
        }
        self.buffer.to_mut().push(c);
    }

    // Go: util/util.go:CopyOnWriteBuffer.AppendByte
    /// AppendByte appends given bytes to the buffer.
    /// AppendByte copy buffer at the first time.
    pub fn append_byte(&mut self, c: u8) {
        if !self.copied {
            let mut tmp = Vec::with_capacity(self.buffer.len() + 20);
            tmp.extend_from_slice(&self.buffer);
            self.buffer = Cow::Owned(tmp);
            self.copied = true;
        }
        self.buffer.to_mut().push(c);
    }

    // Go: util/util.go:CopyOnWriteBuffer.Bytes
    /// Bytes returns bytes of this buffer.
    pub fn bytes(&self) -> &[u8] {
        &self.buffer
    }

    /// Consumes the buffer, returning its bytes.
    pub fn into_bytes(self) -> Cow<'a, [u8]> {
        self.buffer
    }

    // Go: util/util.go:CopyOnWriteBuffer.IsCopied
    /// IsCopied returns true if buffer has been copied, otherwise false.
    pub fn is_copied(&self) -> bool {
        self.copied
    }
}

// Go: util/util.go:IsEscapedPunctuation
/// IsEscapedPunctuation returns true if character at a given index i
/// is an escaped punctuation, otherwise false.
pub fn is_escaped_punctuation(source: &[u8], i: usize) -> bool {
    source[i] == b'\\' && i < source.len() - 1 && is_punct(source[i + 1])
}

// Go: util/util.go:ReadWhile
/// ReadWhile read the given source while pred is true.
pub fn read_while(source: &[u8], index: [usize; 2], pred: impl Fn(u8) -> bool) -> (usize, bool) {
    let mut j = index[0];
    let mut ok = false;
    while j < index[1] {
        let c1 = source[j];
        if pred(c1) {
            ok = true;
            j += 1;
            continue;
        }
        break;
    }
    (j, ok)
}

// Go: util/util.go:IsBlank
/// IsBlank returns true if the given string is all space characters.
pub fn is_blank(bs: &[u8]) -> bool {
    for &b in bs {
        if !is_space(b) {
            return false;
        }
    }
    true
}

// Go: util/util.go:VisualizeSpaces
/// VisualizeSpaces visualize invisible space characters.
pub fn visualize_spaces(bs: &[u8]) -> Vec<u8> {
    let bs = go_unicode::bytes::replace(bs, b" ", b"[SPACE]", -1);
    let bs = go_unicode::bytes::replace(&bs, b"\t", b"[TAB]", -1);
    let bs = go_unicode::bytes::replace(&bs, b"\n", b"[NEWLINE]\n", -1);
    let bs = go_unicode::bytes::replace(&bs, b"\r", b"[CR]", -1);
    let bs = go_unicode::bytes::replace(&bs, b"\x0b", b"[VTAB]", -1);
    let bs = go_unicode::bytes::replace(&bs, b"\x00", b"[NUL]", -1);
    go_unicode::bytes::replace(&bs, "\u{fffd}".as_bytes(), b"[U+FFFD]", -1)
}

// Go: util/util.go:TabWidth
/// TabWidth calculates actual width of a tab at the given position.
pub fn tab_width(current_pos: i64) -> i64 {
    4 - current_pos % 4
}

// Go: util/util.go:IndentPosition
/// IndentPosition searches an indent position with the given width for the given line.
/// If the line contains tab characters, paddings may be not zero.
pub fn indent_position(bs: &[u8], current_pos: i64, width: i64) -> (i64, i64) {
    indent_position_padding(bs, current_pos, 0, width)
}

// Go: util/util.go:IndentPositionPadding
/// IndentPositionPadding searches an indent position with the given width for the given line.
/// This function is mostly same as IndentPosition except this function
/// takes account into additional paddings.
pub fn indent_position_padding(
    bs: &[u8],
    current_pos: i64,
    paddingv: i64,
    width: i64,
) -> (i64, i64) {
    if width == 0 {
        return (0, paddingv);
    }
    let mut w: i64 = 0;
    let mut i: usize = 0;
    let l = bs.len();
    let mut p = paddingv;
    while i < l {
        if p > 0 {
            p -= 1;
            w += 1;
            i += 1;
            continue;
        }
        if bs[i] == b'\t' && w < width {
            w += tab_width(current_pos + w);
        } else if bs[i] == b' ' && w < width {
            w += 1;
        } else {
            break;
        }
        i += 1;
    }
    if w >= width {
        return (i as i64 - paddingv, w - width);
    }
    (-1, -1)
}

// Go: util/util.go:DedentPosition
/// DedentPosition dedents lines by the given width.
///
/// Deprecated: This function has bugs. Use util.IndentPositionPadding and util.FirstNonSpacePosition.
pub fn dedent_position(bs: &[u8], current_pos: i64, width: i64) -> (i64, i64) {
    if width == 0 {
        return (0, 0);
    }
    let mut w: i64 = 0;
    let l = bs.len();
    let mut i = 0;
    while i < l {
        if bs[i] == b'\t' {
            w += tab_width(current_pos + w);
        } else if bs[i] == b' ' {
            w += 1;
        } else {
            break;
        }
        i += 1;
    }
    if w >= width {
        return (i as i64, w - width);
    }
    (i as i64, 0)
}

// Go: util/util.go:DedentPositionPadding
/// DedentPositionPadding dedents lines by the given width.
/// This function is mostly same as DedentPosition except this function
/// takes account into additional paddings.
///
/// Deprecated: This function has bugs. Use util.IndentPositionPadding and util.FirstNonSpacePosition.
pub fn dedent_position_padding(
    bs: &[u8],
    current_pos: i64,
    paddingv: i64,
    width: i64,
) -> (i64, i64) {
    if width == 0 {
        return (0, paddingv);
    }

    let mut w: i64 = 0;
    let mut i = 0;
    let l = bs.len();
    while i < l {
        if bs[i] == b'\t' {
            w += tab_width(current_pos + w);
        } else if bs[i] == b' ' {
            w += 1;
        } else {
            break;
        }
        i += 1;
    }
    if w >= width {
        return (i as i64 - paddingv, w - width);
    }
    (i as i64 - paddingv, 0)
}

// Go: util/util.go:IndentWidth
/// IndentWidth calculate an indent width for the given line.
pub fn indent_width(bs: &[u8], current_pos: i64) -> (i64, i64) {
    let mut width: i64 = 0;
    let mut pos: i64 = 0;
    for &b in bs {
        if b == b' ' {
            width += 1;
            pos += 1;
        } else if b == b'\t' {
            width += tab_width(current_pos + width);
            pos += 1;
        } else {
            break;
        }
    }
    (width, pos)
}

// Go: util/util.go:FirstNonSpacePosition
/// FirstNonSpacePosition returns a position line that is a first nonspace
/// character.
pub fn first_non_space_position(bs: &[u8]) -> i64 {
    for (i, &c) in bs.iter().enumerate() {
        if c == b' ' || c == b'\t' {
            continue;
        }
        if c == b'\n' {
            return -1;
        }
        return i as i64;
    }
    -1
}

// Go: util/util.go:FindClosure
/// FindClosure returns a position that closes the given opener.
/// If codeSpan is set true, it ignores characters in code spans.
/// If allowNesting is set true, closures correspond to nested opener will be
/// ignored.
///
/// Deprecated: This function can not handle newlines. Many elements
/// can be existed over multiple lines(e.g. link labels).
/// Use text.Reader.FindClosure.
pub fn find_closure(
    bs: &[u8],
    opener: u8,
    closure: u8,
    code_span: bool,
    allow_nesting: bool,
) -> i64 {
    let mut i: i64 = 0;
    let mut opened = 1;
    let mut code_span_opener = 0;
    let len = bs.len() as i64;
    while i < len {
        let c = bs[i as usize];
        if code_span && code_span_opener != 0 && c == b'`' {
            let mut code_span_closer = 0;
            while i < len {
                if bs[i as usize] == b'`' {
                    code_span_closer += 1;
                } else {
                    i -= 1;
                    break;
                }
                i += 1;
            }
            if code_span_closer == code_span_opener {
                code_span_opener = 0;
            }
        } else if code_span_opener == 0
            && c == b'\\'
            && i < len - 1
            && is_punct(bs[(i + 1) as usize])
        {
            i += 2;
            continue;
        } else if code_span && code_span_opener == 0 && c == b'`' {
            while i < len {
                if bs[i as usize] == b'`' {
                    code_span_opener += 1;
                } else {
                    i -= 1;
                    break;
                }
                i += 1;
            }
        } else if (code_span && code_span_opener == 0) || !code_span {
            if c == closure {
                opened -= 1;
                if opened == 0 {
                    return i;
                }
            } else if c == opener {
                if !allow_nesting {
                    return -1;
                }
                opened += 1;
            }
        }
        i += 1;
    }
    -1
}

// Go: util/util.go:TrimLeft
/// TrimLeft trims characters in the given s from head of the source.
/// bytes.TrimLeft offers same functionalities, but bytes.TrimLeft
/// allocates new buffer for the result.
pub fn trim_left<'a>(source: &'a [u8], b: &[u8]) -> &'a [u8] {
    let mut i = 0;
    while i < source.len() {
        let c = source[i];
        let mut found = false;
        for &bj in b {
            if c == bj {
                found = true;
                break;
            }
        }
        if !found {
            break;
        }
        i += 1;
    }
    &source[i..]
}

// Go: util/util.go:TrimRight
/// TrimRight trims characters in the given s from tail of the source.
pub fn trim_right<'a>(source: &'a [u8], b: &[u8]) -> &'a [u8] {
    let mut i = source.len() as i64 - 1;
    while i >= 0 {
        let c = source[i as usize];
        let mut found = false;
        for &bj in b {
            if c == bj {
                found = true;
                break;
            }
        }
        if !found {
            break;
        }
        i -= 1;
    }
    &source[..(i + 1) as usize]
}

// Go: util/util.go:TrimLeftLength
/// TrimLeftLength returns a length of leading specified characters.
pub fn trim_left_length(source: &[u8], s: &[u8]) -> usize {
    source.len() - trim_left(source, s).len()
}

// Go: util/util.go:TrimRightLength
/// TrimRightLength returns a length of trailing specified characters.
pub fn trim_right_length(source: &[u8], s: &[u8]) -> usize {
    source.len() - trim_right(source, s).len()
}

// Go: util/util.go:TrimLeftSpaceLength
/// TrimLeftSpaceLength returns a length of leading space characters.
pub fn trim_left_space_length(source: &[u8]) -> usize {
    let mut i = 0;
    while i < source.len() {
        if !is_space(source[i]) {
            break;
        }
        i += 1;
    }
    i
}

// Go: util/util.go:TrimRightSpaceLength
/// TrimRightSpaceLength returns a length of trailing space characters.
pub fn trim_right_space_length(source: &[u8]) -> usize {
    let l = source.len() as i64;
    let mut i = l - 1;
    while i >= 0 {
        if !is_space(source[i as usize]) {
            break;
        }
        i -= 1;
    }
    if i < 0 {
        return l as usize;
    }
    (l - 1 - i) as usize
}

const SPACES: &[u8] = b" \t\n\x0b\x0c\x0d";

// Go: util/util.go:TrimLeftSpace
/// TrimLeftSpace returns a subslice of the given string by slicing off all leading
/// space characters.
pub fn trim_left_space(source: &[u8]) -> &[u8] {
    trim_left(source, SPACES)
}

// Go: util/util.go:TrimRightSpace
/// TrimRightSpace returns a subslice of the given string by slicing off all trailing
/// space characters.
pub fn trim_right_space(source: &[u8]) -> &[u8] {
    trim_right(source, SPACES)
}

// Go: util/util.go:DoFullUnicodeCaseFolding
/// DoFullUnicodeCaseFolding performs full unicode case folding to given bytes.
pub fn do_full_unicode_case_folding(v: &[u8]) -> Cow<'_, [u8]> {
    let mut cob = CopyOnWriteBuffer::new(v);
    let mut n = 0;
    let mut i = 0;
    while i < v.len() {
        let c = v[i];
        if c < 0xb5 {
            if (0x41..=0x5a).contains(&c) {
                // A-Z to a-z
                cob.write(&v[n..i]);
                cob.write_byte(c + 32);
                n = i + 1;
            }
            i += 1;
            continue;
        }

        if !utf8::rune_start(c) {
            i += 1;
            continue;
        }
        let (r, length) = utf8::decode_rune(&v[i..]);
        if r == utf8::RUNE_ERROR {
            i += 1;
            continue;
        }
        let Some(folded) = unicode_case_folding::lookup(r) else {
            i += 1;
            continue;
        };

        cob.write(&v[n..i]);
        for &f in folded {
            let mut rbuf = Vec::with_capacity(4);
            utf8::append_rune(&mut rbuf, f);
            cob.write(&rbuf);
        }
        i += length - 1;
        n = i + 1;
        i += 1;
    }
    if cob.is_copied() {
        cob.write(&v[n..]);
    }
    cob.into_bytes()
}

// Go: util/util.go:ReplaceSpaces
/// ReplaceSpaces replaces sequence of spaces with the given repl.
pub fn replace_spaces(source: &[u8], repl: u8) -> Cow<'_, [u8]> {
    let mut ret: Option<Vec<u8>> = None;
    let mut start: i64 = -1;
    for (i, &c) in source.iter().enumerate() {
        let iss = is_space(c);
        if start < 0 && iss {
            start = i as i64;
            continue;
        } else if start >= 0 && iss {
            continue;
        } else if start >= 0 {
            if ret.is_none() {
                let mut r = Vec::with_capacity(source.len());
                r.extend_from_slice(&source[..start as usize]);
                ret = Some(r);
            }
            ret.as_mut().unwrap().push(repl);
            start = -1;
        }
        if let Some(r) = ret.as_mut() {
            r.push(c);
        }
    }
    if start >= 0
        && let Some(r) = ret.as_mut()
    {
        r.push(repl);
    }
    match ret {
        None => Cow::Borrowed(source),
        Some(r) => Cow::Owned(r),
    }
}

// Go: util/util.go:ToRune
/// ToRune decode given bytes start at pos and returns a rune.
pub fn to_rune(source: &[u8], pos: usize) -> Rune {
    let mut i = pos as i64;
    while i >= 0 {
        if utf8::rune_start(source[i as usize]) {
            break;
        }
        i -= 1;
    }
    // Go: utf8.DecodeRune(source[i:]) panics when i == -1.
    assert!(i >= 0, "slice bounds out of range [-1:]");
    let (r, _) = utf8::decode_rune(&source[i as usize..]);
    r
}

// Go: util/util.go:ToValidRune
/// ToValidRune returns 0xFFFD if the given rune is invalid, otherwise v.
pub fn to_valid_rune(v: Rune) -> Rune {
    if v == 0 || !utf8::valid_rune(v) {
        return 0xFFFD;
    }
    v
}

// Go: util/util.go:ToLinkReference
/// ToLinkReference converts given bytes into a valid link reference string.
/// ToLinkReference performs unicode case folding, trims leading and trailing spaces,  converts into lower
/// case and replace spaces with a single space character.
pub fn to_link_reference(v: &[u8]) -> Vec<u8> {
    let v = trim_left_space(v);
    let v = trim_right_space(v);
    let v = do_full_unicode_case_folding(v);
    replace_spaces(&v, b' ').into_owned()
}

const HTML_QUOTE: &[u8] = b"&quot;";
const HTML_AMP: &[u8] = b"&amp;";
const HTML_LESS: &[u8] = b"&lt;";
const HTML_GREATER: &[u8] = b"&gt;";

fn html_escape_table(b: u8) -> Option<&'static [u8]> {
    match b {
        34 => Some(HTML_QUOTE),
        38 => Some(HTML_AMP),
        60 => Some(HTML_LESS),
        62 => Some(HTML_GREATER),
        _ => None,
    }
}

// Go: util/util.go:EscapeHTMLByte
/// EscapeHTMLByte returns HTML escaped bytes if the given byte should be escaped,
/// otherwise nil.
pub fn escape_html_byte(b: u8) -> Option<&'static [u8]> {
    html_escape_table(b)
}

// Go: util/util.go:EscapeHTML
/// EscapeHTML escapes characters that should be escaped in HTML text.
pub fn escape_html(v: &[u8]) -> Cow<'_, [u8]> {
    let mut cob = CopyOnWriteBuffer::new(v);
    let mut n = 0;
    for i in 0..v.len() {
        let c = v[i];
        if let Some(escaped) = html_escape_table(c) {
            cob.write(&v[n..i]);
            cob.write(escaped);
            n = i + 1;
        }
    }
    if cob.is_copied() {
        cob.write(&v[n..]);
    }
    cob.into_bytes()
}

// Go: util/util.go:UnescapePunctuations
/// UnescapePunctuations unescapes blackslash escaped punctuations.
pub fn unescape_punctuations(source: &[u8]) -> Cow<'_, [u8]> {
    let mut cob = CopyOnWriteBuffer::new(source);
    let limit = source.len();
    let mut n = 0;
    let mut i = 0;
    while i < limit {
        let c = source[i];
        if i + 1 < limit && c == b'\\' && is_punct(source[i + 1]) {
            cob.write(&source[n..i]);
            cob.write_byte(source[i + 1]);
            i += 2;
            n = i;
            continue;
        }
        i += 1;
    }
    if cob.is_copied() {
        cob.write(&source[n..]);
    }
    cob.into_bytes()
}

/// Go `v, _ := strconv.ParseUint(s, base, 32)`: the value Go returns even on
/// error (0 on a syntax error, the maximum on a range error).
pub(crate) fn parse_uint32_lossy(s: &[u8], base: i64) -> u64 {
    go_strconv::internal::parse_uint(s, base, 32).0
}

fn encode_rune(r: Rune) -> Vec<u8> {
    let mut b = Vec::with_capacity(4);
    utf8::append_rune(&mut b, r);
    b
}

// Go: util/util.go:ResolveNumericReferences
/// ResolveNumericReferences resolve numeric references like '&#1234;" .
pub fn resolve_numeric_references(source: &[u8]) -> Cow<'_, [u8]> {
    let mut cob = CopyOnWriteBuffer::new(source);
    let limit = source.len();
    let mut ok;
    let mut n = 0;
    let mut i = 0;
    while i < limit {
        if source[i] == b'&' {
            let pos = i;
            let next = i + 1;
            if next < limit && source[next] == b'#' {
                let nnext = next + 1;
                if nnext < limit {
                    let nc = source[nnext];
                    // code point like #x22;
                    if nnext < limit && nc == b'x' || nc == b'X' {
                        let start = nnext + 1;
                        (i, ok) = read_while(source, [start, limit], is_hex_decimal);
                        if ok && i < limit && source[i] == b';' {
                            let v = parse_uint32_lossy(&source[start..i], 16);
                            cob.write(&source[n..pos]);
                            n = i + 1;
                            cob.write(&encode_rune(to_valid_rune(v as i32)));
                            i += 1;
                            continue;
                        }
                        // code point like #1234;
                    } else if nc.is_ascii_digit() {
                        let start = nnext;
                        (i, ok) = read_while(source, [start, limit], is_numeric);
                        if ok && i < limit && i - start < 8 && source[i] == b';' {
                            let v = parse_uint32_lossy(&source[start..i], 0);
                            cob.write(&source[n..pos]);
                            n = i + 1;
                            cob.write(&encode_rune(to_valid_rune(v as i32)));
                            i += 1;
                            continue;
                        }
                    }
                }
            }
            i = next - 1;
        }
        i += 1;
    }
    if cob.is_copied() {
        cob.write(&source[n..]);
    }
    cob.into_bytes()
}

// Go: util/util.go:ResolveEntityNames
/// ResolveEntityNames resolve entity references like '&ouml;" .
pub fn resolve_entity_names(source: &[u8]) -> Cow<'_, [u8]> {
    let mut cob = CopyOnWriteBuffer::new(source);
    let limit = source.len();
    let mut ok;
    let mut n = 0;
    let mut i = 0;
    while i < limit {
        if source[i] == b'&' {
            let pos = i;
            let next = i + 1;
            if !(next < limit && source[next] == b'#') {
                let start = next;
                (i, ok) = read_while(source, [start, limit], is_alpha_numeric);
                if ok && i < limit && source[i] == b';' {
                    let name = &source[start..i];
                    if let Some(entity) = lookup_html5_entity_by_name(name) {
                        cob.write(&source[n..pos]);
                        n = i + 1;
                        cob.write(entity.characters);
                        i += 1;
                        continue;
                    }
                }
            }
            i = next - 1;
        }
        i += 1;
    }
    if cob.is_copied() {
        cob.write(&source[n..]);
    }
    cob.into_bytes()
}

const HTML_SPACE: &[u8] = b"%20";

// Go: util/util.go:URLEscape
/// URLEscape escape the given URL.
/// If resolveReference is set true:
///  1. unescape punctuations
///  2. resolve numeric references
///  3. resolve entity references
///
/// URL encoded values (%xx) are kept as is.
pub fn url_escape(v: &[u8], resolve_reference: bool) -> Vec<u8> {
    let resolved: Vec<u8>;
    let v: &[u8] = if resolve_reference {
        let a = unescape_punctuations(v);
        let b = resolve_numeric_references(&a).into_owned();
        resolved = resolve_entity_names(&b).into_owned();
        &resolved
    } else {
        v
    };
    let mut cob = CopyOnWriteBuffer::new(v);
    let limit = v.len();
    let mut n = 0;

    let mut i = 0;
    while i < limit {
        let c = v[i];
        if URL_ESCAPE_TABLE[c as usize] == 1 {
            i += 1;
            continue;
        }
        // Go checks IsHexDecimal(v[i+1]) twice (v[i+2] is never checked).
        if c == b'%' && i + 2 < limit && is_hex_decimal(v[i + 1]) && is_hex_decimal(v[i + 1]) {
            i += 3;
            continue;
        }
        let mut u8len = UTF8LEN_TABLE[c as usize];
        if u8len == 99 {
            // invalid utf8 leading byte, skip it
            i += 1;
            continue;
        }
        if c == b' ' {
            cob.write(&v[n..i]);
            cob.write(HTML_SPACE);
            i += 1;
            n = i;
            continue;
        }
        if u8len as usize > v.len() {
            u8len = (v.len() as i64 - 1) as i8;
        }
        if u8len == 0 {
            i += 1;
            n = i;
            continue;
        }
        cob.write(&v[n..i]);
        let stop = i + u8len as usize;
        if stop > v.len() {
            i += 1;
            n = i;
            continue;
        }
        cob.write(&go_url::query_escape(&v[i..stop]));
        i += u8len as usize;
        n = i;
    }
    if cob.is_copied() && n < limit {
        cob.write(&v[n..]);
    }
    cob.into_bytes().into_owned()
}

// Go: util/util.go:FindURLIndex
/// FindURLIndex returns a stop index value if the given bytes seem an URL.
/// This function is equivalent to [A-Za-z][A-Za-z0-9.+-]{1,31}:[^<>\x00-\x20]* .
pub fn find_url_index(b: &[u8]) -> i64 {
    let mut i = 0;
    if !(!b.is_empty() && URL_TABLE[b[i] as usize] & 7 == 7) {
        return -1;
    }
    i += 1;
    while i < b.len() {
        let c = b[i];
        if URL_TABLE[c as usize] & 4 != 4 {
            break;
        }
        i += 1;
    }
    if i == 1 || i > 33 || i >= b.len() {
        return -1;
    }
    if b[i] != b':' {
        return -1;
    }
    i += 1;
    while i < b.len() {
        let c = b[i];
        if URL_TABLE[c as usize] & 1 != 1 {
            break;
        }
        i += 1;
    }
    i as i64
}

fn is_email_alnum(c: u8) -> bool {
    c.is_ascii_alphanumeric()
}

/// Go: `emailDomainRegexp.FindSubmatchIndex(b)` for
/// `^[a-zA-Z0-9](?:[a-zA-Z0-9-]{0,61}[a-zA-Z0-9])?(?:\.[a-zA-Z0-9](?:[a-zA-Z0-9-]{0,61}[a-zA-Z0-9])?)*`,
/// returning `match[1]` (the end of the leftmost-first match) or None.
///
/// Leftmost-first (backtracking) semantics: each label takes the largest
/// k in 0..=61 such that `[a-zA-Z0-9-]{k}` is followed by an alphanumeric,
/// and the `(?:\.label)*` loop continues while a `.` is followed by an
/// alphanumeric. See PORTING.md ("regular expressions").
pub fn email_domain_regexp_match_end(b: &[u8]) -> Option<usize> {
    fn label(b: &[u8], start: usize) -> Option<usize> {
        if start >= b.len() || !is_email_alnum(b[start]) {
            return None;
        }
        let mut end = start + 1;
        // (?:[a-zA-Z0-9-]{0,61}[a-zA-Z0-9])? greedy
        let mut k = 0;
        let mut best: Option<usize> = None;
        while k <= 61 {
            let p = start + 1 + k;
            if p >= b.len() {
                break;
            }
            if is_email_alnum(b[p]) {
                best = Some(p + 1);
            }
            if !(is_email_alnum(b[p]) || b[p] == b'-') {
                break;
            }
            k += 1;
        }
        if let Some(e) = best {
            end = e;
        }
        Some(end)
    }
    let mut end = label(b, 0)?;
    loop {
        if end < b.len() && b[end] == b'.' {
            if let Some(e) = label(b, end + 1) {
                end = e;
                continue;
            }
        }
        break;
    }
    Some(end)
}

// Go: util/util.go:FindEmailIndex
/// FindEmailIndex returns a stop index value if the given bytes seem an email address.
pub fn find_email_index(b: &[u8]) -> i64 {
    // TODO: eliminate regexps
    let mut i = 0;
    while i < b.len() {
        let c = b[i];
        if EMAIL_TABLE[c as usize] & 1 != 1 {
            break;
        }
        i += 1;
    }
    if i == 0 {
        return -1;
    }
    if i >= b.len() || b[i] != b'@' {
        return -1;
    }
    i += 1;
    if i >= b.len() {
        return -1;
    }
    let Some(m1) = email_domain_regexp_match_end(&b[i..]) else {
        return -1;
    };
    (i + m1) as i64
}

// Go: util/util.go:UTF8Len
/// UTF8Len returns a byte length of the utf-8 character.
pub fn utf8_len(b: u8) -> i8 {
    UTF8LEN_TABLE[b as usize]
}

// Go: util/util.go:IsPunct
/// IsPunct returns true if the given character is a punctuation, otherwise false.
pub fn is_punct(c: u8) -> bool {
    PUNCT_TABLE[c as usize] == 1
}

// Go: util/util.go:IsPunctRune
/// IsPunctRune returns true if the given rune is a punctuation, otherwise false.
pub fn is_punct_rune(r: Rune) -> bool {
    go_unicode::is_symbol(r) || go_unicode::is_punct(r)
}

// Go: util/util.go:IsSpace
/// IsSpace returns true if the given character is a space, otherwise false.
pub fn is_space(c: u8) -> bool {
    SPACE_TABLE[c as usize] == 1
}

// Go: util/util.go:IsSpaceRune
/// IsSpaceRune returns true if the given rune is a space, otherwise false.
pub fn is_space_rune(r: Rune) -> bool {
    r <= 256 && is_space(r as u8) || go_unicode::is_space(r)
}

// Go: util/util.go:IsNumeric
/// IsNumeric returns true if the given character is a numeric, otherwise false.
pub fn is_numeric(c: u8) -> bool {
    c.is_ascii_digit()
}

// Go: util/util.go:IsHexDecimal
/// IsHexDecimal returns true if the given character is a hexdecimal, otherwise false.
pub fn is_hex_decimal(c: u8) -> bool {
    c.is_ascii_hexdigit()
}

// Go: util/util.go:IsAlphaNumeric
/// IsAlphaNumeric returns true if the given character is a alphabet or a numeric, otherwise false.
pub fn is_alpha_numeric(c: u8) -> bool {
    c.is_ascii_alphanumeric()
}

/// A BufWriter is a subset of the bufio.Writer .
///
/// Writes are infallible (goldmark ignores write errors everywhere and only
/// reports `Flush`). `as_any_mut` lets a renderer downcast the writer, as
/// Hugo does with `w.(*render.Context)`.
pub trait BufWriter {
    /// io.Writer.Write.
    fn write(&mut self, p: &[u8]);
    /// Available returns how many bytes are unused in the buffer.
    fn available(&self) -> i64;
    /// Buffered returns the number of bytes that have been written into the current buffer.
    fn buffered(&self) -> i64;
    /// Flush writes any buffered data to the underlying io.Writer.
    fn flush(&mut self) -> Result<(), crate::Error>;
    /// WriteByte writes a single byte.
    fn write_byte(&mut self, c: u8) {
        self.write(&[c]);
    }
    /// WriteRune writes a single Unicode code point (invalid runes as U+FFFD).
    fn write_rune(&mut self, r: Rune) {
        let mut b = Vec::with_capacity(4);
        utf8::append_rune(&mut b, r);
        self.write(&b);
    }
    /// WriteString writes a string.
    fn write_string(&mut self, s: &str) {
        self.write(s.as_bytes());
    }
    /// For downcasting to a concrete writer type.
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

impl BufWriter for Vec<u8> {
    fn write(&mut self, p: &[u8]) {
        self.extend_from_slice(p);
    }
    fn available(&self) -> i64 {
        i64::MAX
    }
    fn buffered(&self) -> i64 {
        self.len() as i64
    }
    fn flush(&mut self) -> Result<(), crate::Error> {
        Ok(())
    }
    fn write_byte(&mut self, c: u8) {
        self.push(c);
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

/// A PrioritizedValue struct holds pair of an arbitrary value and a priority.
pub struct PrioritizedValue<T> {
    /// Value is an arbitrary value that you want to prioritize.
    pub value: T,
    /// Priority is a priority of the value.
    pub priority: i64,
}

// Go: util/util.go:Prioritized
/// Prioritized returns a new PrioritizedValue.
pub fn prioritized<T>(v: T, priority: i64) -> PrioritizedValue<T> {
    PrioritizedValue { value: v, priority }
}

// Go: util/util.go:PrioritizedSlice.Sort
/// Sort sorts the PrioritizedSlice in ascending order (Go `sort.Slice`,
/// unstable pdqsort; ties keep Go's order via go-sort).
pub fn sort_prioritized<T>(s: &mut [PrioritizedValue<T>]) {
    go_sort::sort_by(s, |a, b| a.priority < b.priority);
}

// Go: util/util.go:bytesHash
fn bytes_hash(b: &[u8]) -> u64 {
    let mut hash: u64 = 5381;
    for &c in b {
        hash = (hash << 5).wrapping_add(hash).wrapping_add(c as u64);
    }
    hash
}

/// BytesFilter is a efficient data structure for checking whether bytes exist or not.
/// BytesFilter is thread-safe.
#[derive(Clone, Debug)]
pub struct BytesFilter {
    chars: [u8; 256],
    threshold: usize,
    slots: Vec<Vec<Vec<u8>>>,
}

// Go: util/util.go:NewBytesFilter
/// NewBytesFilter returns a new BytesFilter.
pub fn new_bytes_filter(elements: &[&[u8]]) -> BytesFilter {
    let mut s = BytesFilter {
        chars: [0; 256],
        threshold: 3,
        slots: vec![Vec::new(); 64],
    };
    for element in elements {
        s.add(element);
    }
    s
}

// Go: util/util.go:NewBytesFilterString
/// NewBytesFilterString returns a new BytesFilter.
/// Given string must be separated by a comma.
pub fn new_bytes_filter_string(elements: &str) -> BytesFilter {
    let mut s = BytesFilter {
        chars: [0; 256],
        threshold: 3,
        slots: vec![Vec::new(); 64],
    };
    s.add_comma_separated(elements.as_bytes());
    s
}

impl BytesFilter {
    fn add_comma_separated(&mut self, elements: &[u8]) {
        let mut start = 0;
        for i in 0..elements.len() {
            if elements[i] == b',' {
                self.add(&elements[start..i]);
                start = i + 1;
            }
        }
        if start < elements.len() {
            self.add(&elements[start..]);
        }
    }

    // Go: util/util.go:bytesFilter.Add
    /// Add adds given bytes to this set.
    pub fn add(&mut self, b: &[u8]) {
        let l = b.len();
        let m = if l < self.threshold {
            l
        } else {
            self.threshold
        };
        for i in 0..m {
            self.chars[b[i] as usize] |= 1 << (i as u8);
        }
        let h = (bytes_hash(b) % self.slots.len() as u64) as usize;
        self.slots[h].push(b.to_vec());
    }

    // Go: util/util.go:bytesFilter.Extend
    /// Extend copies this filter and adds given bytes to new filter.
    ///
    /// Go shares the slot slices with the original filter (see PORTING.md:
    /// the aliasing is unobservable for goldmark's filters).
    pub fn extend(&self, bs: &[&[u8]]) -> BytesFilter {
        let mut new_filter = self.clone();
        for b in bs {
            new_filter.add(b);
        }
        new_filter
    }

    // Go: util/util.go:bytesFilter.ExtendString
    /// ExtendString copies this filter and adds given bytes to new filter.
    /// Given string must be separated by a comma.
    pub fn extend_string(&self, elements: &str) -> BytesFilter {
        let mut new_filter = self.clone();
        new_filter.add_comma_separated(elements.as_bytes());
        new_filter
    }

    // Go: util/util.go:bytesFilter.Contains
    /// Contains return true if this set contains given bytes, otherwise false.
    pub fn contains(&self, b: &[u8]) -> bool {
        let l = b.len();
        let m = if l < self.threshold {
            l
        } else {
            self.threshold
        };
        for i in 0..m {
            if (self.chars[b[i] as usize] & (1 << (i as u8))) == 0 {
                return false;
            }
        }
        let h = (bytes_hash(b) % self.slots.len() as u64) as usize;
        let slot = &self.slots[h];
        if slot.is_empty() {
            return false;
        }
        for element in slot {
            if element.as_slice() == b {
                return true;
            }
        }
        false
    }
}
