//! Go: parse/html/util.go

use crate::gobytes::GoBytes;

static SINGLE_QUOTE_ENTITY_BYTES: &[u8] = b"&#39;";
static DOUBLE_QUOTE_ENTITY_BYTES: &[u8] = b"&#34;";

// Go: parse/html/util.go:EscapeAttrVal
/// Returns the escaped attribute value bytes with quotes. Either single or
/// double quotes are used, whichever is shorter. If there are no quotes
/// present in the value and the value is in HTML (not XML), it will return
/// the value without quotes. The result may alias `*buf` (which is replaced
/// by a bigger allocation when too small) or `b`.
pub fn escape_attr_val(
    buf: &mut GoBytes,
    b: &GoBytes,
    orig_quote: u8,
    must_quote: bool,
) -> GoBytes {
    let mut singles = 0usize;
    let mut doubles = 0usize;
    let mut unquoted = true;
    for c in b.iter() {
        if CHAR_TABLE[c as usize] {
            unquoted = false;
            if c == b'"' {
                doubles += 1;
            } else if c == b'\'' {
                singles += 1;
            }
        }
    }
    if unquoted && (!must_quote || orig_quote == 0) {
        return b.clone();
    } else if singles == 0 && orig_quote == b'\'' || doubles == 0 && orig_quote == b'"' {
        if b.len() + 2 > buf.cap() {
            *buf = GoBytes::make(0, b.len() + 2);
        }
        let t = buf.slice_to(b.len() + 2);
        t.set(0, orig_quote);
        t.slice_from(1).copy_from(b);
        t.set(1 + b.len(), orig_quote);
        return t;
    }

    let mut n = b.len() + 2;
    let quote: u8;
    let escaped_quote: &[u8];
    if singles > doubles || singles == doubles && orig_quote != b'\'' {
        n += doubles * 4;
        quote = b'"';
        escaped_quote = DOUBLE_QUOTE_ENTITY_BYTES;
    } else {
        n += singles * 4;
        quote = b'\'';
        escaped_quote = SINGLE_QUOTE_ENTITY_BYTES;
    }
    if n > buf.cap() {
        *buf = GoBytes::make(0, n); // maximum size, not actual size
    }
    let t = buf.slice_to(n); // maximum size, not actual size
    t.set(0, quote);
    let mut j = 1;
    let mut start = 0;
    for i in 0..b.len() {
        let c = b.at(i);
        if c == quote {
            j += t.slice_from(j).copy_from(&b.slice(start, i));
            j += t.slice_from(j).copy_from_slice(escaped_quote);
            start = i + 1;
        }
    }
    j += t.slice_from(j).copy_from(&b.slice_from(start));
    t.set(j, quote);
    t.slice_to(j + 1)
}

static CHAR_TABLE: [bool; 256] = {
    let mut t = [false; 256];
    t[b'\t' as usize] = true; // tab
    t[b'\n' as usize] = true; // line feed
    t[0x0C] = true; // form feed
    t[b'\r' as usize] = true; // carriage return
    t[b' ' as usize] = true; // space
    t[b'"' as usize] = true;
    t[b'\'' as usize] = true;
    t[b'<' as usize] = true;
    t[b'=' as usize] = true;
    t[b'>' as usize] = true;
    t[b'`' as usize] = true;
    t
};
