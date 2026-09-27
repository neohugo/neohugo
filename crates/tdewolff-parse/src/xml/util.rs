//! Go: parse/xml/util.go

use crate::gobytes::GoBytes;

static LT_ENTITY_BYTES: &[u8] = b"&lt;";
static AMP_ENTITY_BYTES: &[u8] = b"&amp;";
static SINGLE_QUOTE_ENTITY_BYTES: &[u8] = b"&#39;";
static DOUBLE_QUOTE_ENTITY_BYTES: &[u8] = b"&#34;";

// Go: parse/xml/util.go:EscapeAttrVal
/// Returns the escaped attribute value bytes with quotes. The result aliases
/// `*buf` (replaced by a bigger allocation when too small).
pub fn escape_attr_val(buf: &mut GoBytes, b: &GoBytes) -> GoBytes {
    let mut singles = 0usize;
    let mut doubles = 0usize;
    for c in b.iter() {
        if c == b'"' {
            doubles += 1;
        } else if c == b'\'' {
            singles += 1;
        }
    }

    let mut n = b.len() + 2;
    let quote: u8;
    let escaped_quote: &[u8];
    if doubles > singles {
        n += singles * 4;
        quote = b'\'';
        escaped_quote = SINGLE_QUOTE_ENTITY_BYTES;
    } else {
        n += doubles * 4;
        quote = b'"';
        escaped_quote = DOUBLE_QUOTE_ENTITY_BYTES;
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

// Go: parse/xml/util.go:EscapeCDATAVal
/// Returns the escaped text bytes, or `(b, false)` when escaping would be
/// longer than wrapping in CDATA.
pub fn escape_cdata_val(buf: &mut GoBytes, b: &GoBytes) -> (GoBytes, bool) {
    let mut n = 0usize;
    for c in b.iter() {
        if c == b'<' || c == b'&' {
            if c == b'<' {
                n += 3; // &lt;
            } else {
                n += 4; // &amp;
            }
            if n > b"<![CDATA[]]>".len() {
                return (b.clone(), false);
            }
        }
    }
    if b.len() + n > buf.cap() {
        *buf = GoBytes::make(0, b.len() + n);
    }
    let t = buf.slice_to(b.len() + n);
    let mut j = 0;
    let mut start = 0;
    for i in 0..b.len() {
        let c = b.at(i);
        if c == b'<' {
            j += t.slice_from(j).copy_from(&b.slice(start, i));
            j += t.slice_from(j).copy_from_slice(LT_ENTITY_BYTES);
            start = i + 1;
        } else if c == b'&' {
            j += t.slice_from(j).copy_from(&b.slice(start, i));
            j += t.slice_from(j).copy_from_slice(AMP_ENTITY_BYTES);
            start = i + 1;
        }
    }
    j += t.slice_from(j).copy_from(&b.slice_from(start));
    (t.slice_to(j), true)
}
