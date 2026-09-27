//! Go: parse/common.go

use std::collections::{BTreeMap, HashMap};

use crate::base64;
use crate::error::GoError;
use crate::gobytes::{ByteView, GoBytes};
use crate::util::{is_newline, is_whitespace, trim_whitespace};

static DATA_SCHEME_BYTES: &[u8] = b"data:";
static BASE64_BYTES: &[u8] = b"base64";
static TEXT_MIME_BYTES: &[u8] = b"text/plain";

// Go: parse/common.go:Number
/// Returns the number of bytes that parse as a number of the regex format
/// `(+|-)?([0-9]+(\.[0-9]+)?|\.[0-9]+)((e|E)(+|-)?[0-9]+)?`.
pub fn number<B: ByteView + ?Sized>(b: &B) -> usize {
    if b.len() == 0 {
        return 0;
    }
    let mut i = 0;
    if b.at(i) == b'+' || b.at(i) == b'-' {
        i += 1;
        if i >= b.len() {
            return 0;
        }
    }
    let first_digit = b.at(i).is_ascii_digit();
    if first_digit {
        i += 1;
        while i < b.len() && b.at(i).is_ascii_digit() {
            i += 1;
        }
    }
    if i < b.len() && b.at(i) == b'.' {
        i += 1;
        if i < b.len() && b.at(i).is_ascii_digit() {
            i += 1;
            while i < b.len() && b.at(i).is_ascii_digit() {
                i += 1;
            }
        } else if first_digit {
            // . could belong to the next token
            i -= 1;
            return i;
        } else {
            return 0;
        }
    } else if !first_digit {
        return 0;
    }
    let i_old = i;
    if i < b.len() && (b.at(i) == b'e' || b.at(i) == b'E') {
        i += 1;
        if i < b.len() && (b.at(i) == b'+' || b.at(i) == b'-') {
            i += 1;
        }
        if i >= b.len() || b.at(i) < b'0' || b.at(i) > b'9' {
            // e could belong to next token
            return i_old;
        }
        while i < b.len() && b.at(i).is_ascii_digit() {
            i += 1;
        }
    }
    i
}

// Go: parse/common.go:Dimension
/// Parses a byte-slice and returns the length of the number and its unit.
pub fn dimension<B: ByteView + ?Sized>(b: &B) -> (usize, usize) {
    let num = number(b);
    if num == 0 || num == b.len() {
        return (num, 0);
    } else if b.at(num) == b'%' {
        return (num, 1);
    } else if b.at(num).is_ascii_alphabetic() {
        let mut i = num + 1;
        while i < b.len() && b.at(i).is_ascii_alphabetic() {
            i += 1;
        }
        return (num, i - num);
    }
    (num, 0)
}

/// Go `map[string]string` returned by [`mediatype`] (nil = `None`).
pub type Params = BTreeMap<Vec<u8>, Vec<u8>>;

// Go: parse/common.go:Mediatype
/// Parses a given mediatype and splits the mimetype from the parameters.
/// The returned mimetype aliases `b`.
pub fn mediatype(b: &GoBytes) -> (GoBytes, Option<Params>) {
    let mut i = 0;
    while i < b.len() && b.at(i) == b' ' {
        i += 1;
    }
    let b = b.slice_from(i);
    let n = b.len();
    let mut mimetype = b.clone();
    let mut params: Option<Params> = None;
    let mut i = 3; // mimetype is at least three characters long
    while i < n {
        if b.at(i) == b';' || b.at(i) == b' ' {
            mimetype = b.slice_to(i);
            if b.at(i) == b' ' {
                i += 1; // space
                while i < n && b.at(i) == b' ' {
                    i += 1;
                }
                if n <= i || b.at(i) != b';' {
                    break;
                }
            }
            let mut p = Params::new();
            let s = b.to_vec();
            // PARAM:
            loop {
                i += 1; // semicolon
                while i < n && s[i] == b' ' {
                    i += 1;
                }
                let mut start = i;
                while i < n && s[i] != b'=' && s[i] != b';' && s[i] != b' ' {
                    i += 1;
                }
                let key = s[start..i].to_vec();
                while i < n && s[i] == b' ' {
                    i += 1;
                }
                if i < n && s[i] == b'=' {
                    i += 1;
                    while i < n && s[i] == b' ' {
                        i += 1;
                    }
                    start = i;
                    while i < n && s[i] != b';' && s[i] != b' ' {
                        i += 1;
                    }
                } else {
                    start = i;
                }
                p.insert(key, s[start..i].to_vec());
                while i < n && s[i] == b' ' {
                    i += 1;
                }
                if i < n && s[i] == b';' {
                    continue; // goto PARAM
                }
                break;
            }
            params = Some(p);
            break;
        }
        i += 1;
    }
    (mimetype, params)
}

// Go: parse/common.go:DataURI
/// Parses the given data URI and returns the mediatype and data. For
/// non-base64 data the percent-decoding happens in place in `data_uri`.
pub fn data_uri(data_uri: &GoBytes) -> Result<(GoBytes, GoBytes), GoError> {
    if data_uri.len() > 5 && data_uri.slice_to(5).equal(DATA_SCHEME_BYTES) {
        let data_uri = data_uri.slice_from(5);
        let mut in_base64 = false;
        let mut mediatype = GoBytes::nil();
        let mut i = 0;
        for j in 0..data_uri.len() {
            let c = data_uri.at(j);
            if c == b'=' || c == b';' || c == b',' {
                if c != b'=' && trim_whitespace(&data_uri.slice(i, j)).equal(BASE64_BYTES) {
                    if mediatype.len() > 0 {
                        mediatype = mediatype.slice_to(mediatype.len() - 1);
                    }
                    in_base64 = true;
                    i = j;
                } else if c != b',' {
                    mediatype = mediatype
                        .append_bytes(&trim_whitespace(&data_uri.slice(i, j)))
                        .append_byte(c);
                    i = j + 1;
                } else {
                    mediatype = mediatype.append_bytes(&trim_whitespace(&data_uri.slice(i, j)));
                }
                if c == b',' {
                    if mediatype.len() == 0 || mediatype.at(0) == b';' {
                        mediatype = GoBytes::from_static(TEXT_MIME_BYTES);
                    }
                    let mut data = data_uri.slice_from(j + 1);
                    if in_base64 {
                        let decoded = GoBytes::make(
                            base64::std_decoded_len(data.len()),
                            base64::std_decoded_len(data.len()),
                        );
                        let src = data.to_vec();
                        let mut dst = vec![0u8; decoded.len()];
                        let (n, err) = base64::std_decode(&mut dst, &src);
                        if let Some(err) = err {
                            return Err(err);
                        }
                        decoded.copy_from_slice(&dst);
                        data = decoded.slice_to(n);
                    } else {
                        data = decode_url(data);
                    }
                    return Ok((mediatype, data));
                }
            }
        }
    }
    Err(GoError::BadDataUri)
}

// Go: parse/common.go:QuoteEntity
/// Parses the given byte slice and returns the quote that got matched (' or ")
/// and its entity length.
pub fn quote_entity<B: ByteView + ?Sized>(b: &B) -> (u8, usize) {
    if b.len() < 5 || b.at(0) != b'&' {
        return (0, 0);
    }
    if b.at(1) == b'#' {
        if b.at(2) == b'x' {
            let mut i = 3;
            while i < b.len() && b.at(i) == b'0' {
                i += 1;
            }
            if i + 2 < b.len() && b.at(i) == b'2' && b.at(i + 2) == b';' {
                if b.at(i + 1) == b'2' {
                    return (b'"', i + 3); // &#x22;
                } else if b.at(i + 1) == b'7' {
                    return (b'\'', i + 3); // &#x27;
                }
            }
        } else {
            let mut i = 2;
            while i < b.len() && b.at(i) == b'0' {
                i += 1;
            }
            if i + 2 < b.len() && b.at(i) == b'3' && b.at(i + 2) == b';' {
                if b.at(i + 1) == b'4' {
                    return (b'"', i + 3); // &#34;
                } else if b.at(i + 1) == b'9' {
                    return (b'\'', i + 3); // &#39;
                }
            }
        }
    } else if b.len() >= 6 && b.at(5) == b';' {
        let w = [b.at(1), b.at(2), b.at(3), b.at(4)];
        if &w == b"quot" {
            return (b'"', 6); // &quot;
        } else if &w == b"apos" {
            return (b'\'', 6); // &apos;
        }
    }
    (0, 0)
}

// Go: parse/common.go:ReplaceMultipleWhitespace
/// Replaces character series of space, \n, \t, \f, \r into a single space or
/// newline (when the serie contained a \n or \r). Works in place.
pub fn replace_multiple_whitespace(b: GoBytes) -> GoBytes {
    let (mut j, mut k) = (0usize, 0usize); // j is write position, k is start of next text section
    let mut i = 0usize;
    while i < b.len() {
        if is_whitespace(b.at(i)) {
            let start = i;
            let mut newline = is_newline(b.at(i));
            i += 1;
            while i < b.len() && is_whitespace(b.at(i)) {
                if is_newline(b.at(i)) {
                    newline = true;
                }
                i += 1;
            }
            if newline {
                b.set(start, b'\n');
            } else {
                b.set(start, b' ');
            }
            if 1 < i - start {
                // more than one whitespace
                if j == 0 {
                    j = start + 1;
                } else {
                    j += b.slice_from(j).copy_from(&b.slice(k, start + 1));
                }
                k = i;
            }
        }
        i += 1;
    }
    if j == 0 {
        return b;
    } else if j == 1 {
        // only if starts with whitespace
        b.set(k - 1, b.at(0));
        return b.slice_from(k - 1);
    } else if k < b.len() {
        j += b.slice_from(j).copy_from(&b.slice_from(k));
    }
    b.slice_to(j)
}

/// Go `map[string][]byte` entity table (name without `&`/`;` → replacement).
pub trait EntityMap {
    fn lookup_entity(&self, name: &[u8]) -> Option<&[u8]>;
}

/// Go `map[byte][]byte` reverse entity table (byte → entity).
pub trait RevEntityMap {
    fn lookup_rev(&self, c: u8) -> Option<&[u8]>;
}

/// A nil Go map.
pub struct NilMap;

impl EntityMap for NilMap {
    fn lookup_entity(&self, _: &[u8]) -> Option<&[u8]> {
        None
    }
}

impl RevEntityMap for NilMap {
    fn lookup_rev(&self, _: u8) -> Option<&[u8]> {
        None
    }
}

impl<K, V> EntityMap for HashMap<K, V>
where
    K: std::borrow::Borrow<[u8]> + std::hash::Hash + Eq,
    V: AsRef<[u8]>,
{
    fn lookup_entity(&self, name: &[u8]) -> Option<&[u8]> {
        self.get(name).map(|v| v.as_ref())
    }
}

impl<K, V> EntityMap for BTreeMap<K, V>
where
    K: std::borrow::Borrow<[u8]> + Ord,
    V: AsRef<[u8]>,
{
    fn lookup_entity(&self, name: &[u8]) -> Option<&[u8]> {
        self.get(name).map(|v| v.as_ref())
    }
}

impl<V: AsRef<[u8]>> RevEntityMap for HashMap<u8, V> {
    fn lookup_rev(&self, c: u8) -> Option<&[u8]> {
        self.get(&c).map(|v| v.as_ref())
    }
}

impl<V: AsRef<[u8]>> RevEntityMap for BTreeMap<u8, V> {
    fn lookup_rev(&self, c: u8) -> Option<&[u8]> {
        self.get(&c).map(|v| v.as_ref())
    }
}

#[inline]
fn is_alnum(c: u8) -> bool {
    c.is_ascii_digit() || c.is_ascii_lowercase() || c.is_ascii_uppercase()
}

// Go: parse/common.go:replaceEntities
/// Replaces in `b` at index `i`, assuming that `b[i] == '&'` and that
/// `i+3 < len(b)`. The returned index is the last character of the entity, so
/// that the next iteration can safely do `i++`.
fn replace_entities_at(
    b: GoBytes,
    i: usize,
    entities_map: &dyn EntityMap,
    rev_entities_map: &dyn RevEntityMap,
) -> (GoBytes, usize) {
    const MAX_ENTITY_LENGTH: usize = 31; // longest HTML entity: CounterClockwiseContourIntegral
    let mut r: Vec<u8>;
    let mut j = i + 1;
    if b.at(j) == b'#' {
        j += 1;
        if b.at(j) == b'x' {
            j += 1;
            let mut c: i64 = 0;
            while j < b.len() && (b.at(j).is_ascii_hexdigit()) {
                let d = b.at(j);
                if d <= b'9' {
                    c = (c << 4).wrapping_add((d - b'0') as i64);
                } else if d <= b'F' {
                    c = (c << 4).wrapping_add((d - b'A') as i64 + 10);
                } else if d <= b'f' {
                    c = (c << 4).wrapping_add((d - b'a') as i64 + 10);
                }
                j += 1;
            }
            if j <= i + 3 || 10000 <= c {
                return (b, j - 1);
            }
            if c < 128 {
                r = vec![c as u8];
            } else {
                r = vec![b'&', b'#'];
                r.extend_from_slice(c.to_string().as_bytes()); // strconv.AppendInt(r, c, 10)
                r.push(b';');
            }
        } else {
            let mut c: i64 = 0;
            while j < b.len() && c < 128 && b.at(j).is_ascii_digit() {
                c = c * 10 + (b.at(j) - b'0') as i64;
                j += 1;
            }
            if j <= i + 2 || 128 <= c {
                return (b, j - 1);
            }
            r = vec![c as u8];
        }
    } else {
        while j < b.len() && j - i - 1 <= MAX_ENTITY_LENGTH && b.at(j) != b';' {
            if !is_alnum(b.at(j)) {
                // invalid character reference character
                break;
            }
            j += 1;
        }
        if b.len() <= j || j == i + 1 || b.at(j) != b';' {
            return (b, i);
        }

        let name = b.slice(i + 1, j).to_vec();
        match entities_map.lookup_entity(&name) {
            Some(v) => r = v.to_vec(),
            None => return (b, j),
        }
    }

    // j is at semicolon
    let n = j + 1 - i;
    if j < b.len() && b.at(j) == b';' && 2 < n {
        if r.len() == 1 {
            if let Some(q) = rev_entities_map.lookup_rev(r[0]) {
                if q.len() == j + 1 - i && b.slice(i, j + 1).equal(q) {
                    return (b, j);
                }
                r = q.to_vec();
            } else if r[0] == b'&' {
                // check if for example &amp; is followed by something that could potentially be an entity
                let k = j + 1;
                if k < b.len() && (is_alnum(b.at(k)) || b.at(k) == b'#') {
                    return (b, k);
                }
            }
        }

        b.slice_from(i).copy_from_slice(&r);
        b.slice_from(i + r.len()).copy_from(&b.slice_from(j + 1));
        let b = b.slice_to(b.len() - n + r.len());
        let idx = (i + r.len()).wrapping_sub(1);
        return (b, idx);
    }
    (b, i)
}

// Go: parse/common.go:ReplaceEntities
/// Replaces all occurrences of entities (such as `&quot;`) by their
/// respective unencoded bytes, in place.
pub fn replace_entities(
    mut b: GoBytes,
    entities_map: &dyn EntityMap,
    rev_entities_map: &dyn RevEntityMap,
) -> GoBytes {
    let mut i = 0usize;
    while i < b.len() {
        if b.at(i) == b'&' && i + 3 < b.len() {
            (b, i) = replace_entities_at(b, i, entities_map, rev_entities_map);
        }
        i = i.wrapping_add(1);
    }
    b
}

// Go: parse/common.go:ReplaceMultipleWhitespaceAndEntities
/// A combination of [`replace_multiple_whitespace`] and [`replace_entities`].
pub fn replace_multiple_whitespace_and_entities(
    mut b: GoBytes,
    entities_map: &dyn EntityMap,
    rev_entities_map: &dyn RevEntityMap,
) -> GoBytes {
    let (mut j, mut k) = (0usize, 0usize); // j is write position, k is start of next text section
    let mut i = 0usize;
    while i < b.len() {
        if is_whitespace(b.at(i)) {
            let start = i;
            let mut newline = is_newline(b.at(i));
            i += 1;
            while i < b.len() && is_whitespace(b.at(i)) {
                if is_newline(b.at(i)) {
                    newline = true;
                }
                i += 1;
            }
            if newline {
                b.set(start, b'\n');
            } else {
                b.set(start, b' ');
            }
            if 1 < i - start {
                // more than one whitespace
                if j == 0 {
                    j = start + 1;
                } else {
                    j += b.slice_from(j).copy_from(&b.slice(k, start + 1));
                }
                k = i;
            }
        }
        if i + 3 < b.len() && b.at(i) == b'&' {
            (b, i) = replace_entities_at(b, i, entities_map, rev_entities_map);
        }
        i = i.wrapping_add(1);
    }
    if j == 0 {
        return b;
    } else if j == 1 {
        // only if starts with whitespace
        b.set(k - 1, b.at(0)); // move newline to end of whitespace
        return b.slice_from(k - 1);
    } else if k < b.len() {
        j += b.slice_from(j).copy_from(&b.slice_from(k));
    }
    b.slice_to(j)
}

const fn url_encoding_table() -> [bool; 256] {
    let mut t = [true; 256];
    // printable ASCII that is NOT escaped
    let keep: &[u8] = b"!'()*-.0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ_abcdefghijklmnopqrstuvwxyz~";
    let mut i = 0;
    while i < keep.len() {
        t[keep[i] as usize] = false;
        i += 1;
    }
    t
}

/// Go: parse/common.go:URLEncodingTable — which characters need escaping in
/// the URL encoding scheme.
pub static URL_ENCODING_TABLE: [bool; 256] = url_encoding_table();

const fn data_uri_encoding_table() -> [bool; 256] {
    let mut t = [false; 256];
    let mut c = 0;
    while c < 256 {
        t[c] = c < 0x20 || c >= 0x7F;
        c += 1;
    }
    let esc: &[u8] = b" \"#%&<>[\\]^`{|}";
    let mut i = 0;
    while i < esc.len() {
        t[esc[i] as usize] = true;
        i += 1;
    }
    t
}

/// Go: parse/common.go:DataURIEncodingTable — which characters need escaping
/// in the Data URI encoding scheme.
pub static DATA_URI_ENCODING_TABLE: [bool; 256] = data_uri_encoding_table();

const HEX_UPPER: &[u8; 16] = b"0123456789ABCDEF";

// Go: parse/common.go:EncodeURL
/// Encodes bytes using the URL encoding scheme (may append in place).
pub fn encode_url(mut b: GoBytes, table: &[bool; 256]) -> GoBytes {
    let mut i = 0;
    while i < b.len() {
        let c = b.at(i);
        if table[c as usize] {
            b = b.append(&[0, 0]);
            b.slice_from(i + 3).copy_from(&b.slice_from(i + 1));
            b.set(i, b'%');
            b.set(i + 1, HEX_UPPER[(c >> 4) as usize]);
            b.set(i + 2, HEX_UPPER[(c & 15) as usize]);
        }
        i += 1;
    }
    b
}

// Go: parse/common.go:DecodeURL
/// Decodes an URL encoded using the URL encoding scheme, in place.
pub fn decode_url(mut b: GoBytes) -> GoBytes {
    let mut i = 0;
    while i < b.len() {
        if b.at(i) == b'%' && i + 2 < b.len() {
            let mut j = i + 1;
            let mut c: i64 = 0;
            while j < i + 3 && b.at(j).is_ascii_hexdigit() {
                let d = b.at(j);
                if d <= b'9' {
                    c = (c << 4) + (d - b'0') as i64;
                } else if d <= b'F' {
                    c = (c << 4) + (d - b'A') as i64 + 10;
                } else if d <= b'f' {
                    c = (c << 4) + (d - b'a') as i64 + 10;
                }
                j += 1;
            }
            if j == i + 3 {
                b.set(i, c as u8);
                b = b.slice_to(i + 1).append_bytes(&b.slice_from(i + 3));
            }
        } else if b.at(i) == b'+' {
            b.set(i, b' ');
        }
        i += 1;
    }
    b
}

// Go: parse/common.go:AppendEscape
/// Appends `s` to `b`, escaping every byte in `chars` (and `escape` itself)
/// with `escape`.
pub fn append_escape<S: ByteView + ?Sized>(
    mut b: GoBytes,
    s: &S,
    chars: &[u8],
    escape: u8,
) -> GoBytes {
    let s = crate::gobytes::view_to_vec(s); // str is only read
    let mut i = 0;
    for j in 0..s.len() {
        let has = chars.contains(&s[j]);
        if has || s[j] == escape {
            if i < j {
                b = b.append(&s[i..j]);
            }
            b = b.append_byte(escape);
            i = j;
        }
    }
    if i < s.len() {
        b = b.append(&s[i..]);
    }
    b
}
