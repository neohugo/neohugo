//! Port of `golang.org/x/net@v0.41.0/html/escape.go` (the unescaping half; the entity tables are
//! identical to the standard library's and come from `go-html`).

use go_unicode::utf8;

/// These replacements permit compatibility with old numeric entities that assumed Windows-1252
/// encoding.
// Go: html/escape.go:replacementTable
const REPLACEMENT_TABLE: [i32; 32] = [
    0x20AC, // First entry is what 0x80 should be replaced with.
    0x0081, 0x201A, 0x0192, 0x201E, 0x2026, 0x2020, 0x2021, 0x02C6, 0x2030, 0x0160, 0x2039, 0x0152,
    0x008D, 0x017D, 0x008F, 0x0090, 0x2018, 0x2019, 0x201C, 0x201D, 0x2022, 0x2013, 0x2014, 0x02DC,
    0x2122, 0x0161, 0x203A, 0x0153, 0x009D, 0x017E, 0x0178, // Last entry is 0x9F.
];

/// Go: `longestEntityWithoutSemicolon`.
const LONGEST_ENTITY_WITHOUT_SEMICOLON: usize = 6;

/// Appends the UTF-8 encoding of r (Go `utf8.EncodeRune`: invalid runes become U+FFFD).
fn push_rune(out: &mut Vec<u8>, r: i32) {
    let mut buf = [0u8; 4];
    let n = utf8::encode_rune(&mut buf, r);
    out.extend_from_slice(&buf[..n]);
}

/// Go: `unescapeEntity(b, dst, src, attribute)` — reads an entity like "&lt;" from `s` (which
/// starts with '&') and appends the corresponding "<" to `out`, returning the number of source
/// bytes consumed. Go writes into `b[dst:]` in place (dst <= src always holds, so the result is
/// the same as writing to a separate buffer).
// Go: html/escape.go:unescapeEntity
fn unescape_entity(out: &mut Vec<u8>, s: &[u8], attribute: bool) -> usize {
    // i starts at 1 because we already know that s[0] == '&'.
    let mut i = 1usize;

    if s.len() <= 1 {
        out.push(s[0]);
        return 1;
    }

    if s[i] == b'#' {
        if s.len() <= 3 {
            // We need to have at least "&#.".
            out.push(s[0]);
            return 1;
        }
        i += 1;
        let mut c = s[i];
        let mut hex = false;
        if c == b'x' || c == b'X' {
            hex = true;
            i += 1;
        }

        // Go `rune` arithmetic (int32, wrapping).
        let mut x: i32 = 0;
        while i < s.len() {
            c = s[i];
            i += 1;
            if hex {
                if c.is_ascii_digit() {
                    x = x
                        .wrapping_mul(16)
                        .wrapping_add(i32::from(c) - i32::from(b'0'));
                    continue;
                } else if (b'a'..=b'f').contains(&c) {
                    x = x
                        .wrapping_mul(16)
                        .wrapping_add(i32::from(c) - i32::from(b'a') + 10);
                    continue;
                } else if (b'A'..=b'F').contains(&c) {
                    x = x
                        .wrapping_mul(16)
                        .wrapping_add(i32::from(c) - i32::from(b'A') + 10);
                    continue;
                }
            } else if c.is_ascii_digit() {
                x = x
                    .wrapping_mul(10)
                    .wrapping_add(i32::from(c) - i32::from(b'0'));
                continue;
            }
            if c != b';' {
                i -= 1;
            }
            break;
        }

        if i <= 3 {
            // No characters matched.
            out.push(s[0]);
            return 1;
        }

        if (0x80..=0x9F).contains(&x) {
            // Replace characters from Windows-1252 with UTF-8 equivalents.
            x = REPLACEMENT_TABLE[(x - 0x80) as usize];
        } else if x == 0 || (0xD800..=0xDFFF).contains(&x) || x > 0x10FFFF {
            // Replace invalid characters with the replacement character.
            x = 0xFFFD;
        }

        push_rune(out, x);
        return i;
    }

    // Consume the maximum number of characters possible, with the
    // consumed characters matching one of the named references.

    while i < s.len() {
        let c = s[i];
        i += 1;
        // Lower-cased characters are more common in entities, so we check for them first.
        if c.is_ascii_lowercase() || c.is_ascii_uppercase() || c.is_ascii_digit() {
            continue;
        }
        if c != b';' {
            i -= 1;
        }
        break;
    }

    let entity_name = &s[1..i];
    if entity_name.is_empty() {
        // No-op.
    } else if attribute && entity_name[entity_name.len() - 1] != b';' && s.len() > i && s[i] == b'='
    {
        // No-op.
    } else {
        let x = go_html::entity(entity_name);
        if x != 0 {
            push_rune(out, x);
            return i;
        }
        let x2 = go_html::entity2(entity_name);
        if x2[0] != 0 {
            push_rune(out, x2[0]);
            push_rune(out, x2[1]);
            return i;
        }
        if !attribute {
            let max_len = (entity_name.len() - 1).min(LONGEST_ENTITY_WITHOUT_SEMICOLON);
            let mut j = max_len;
            while j > 1 {
                let x = go_html::entity(&entity_name[..j]);
                if x != 0 {
                    push_rune(out, x);
                    return j + 1;
                }
                j -= 1;
            }
        }
    }

    out.extend_from_slice(&s[..i]);
    i
}

/// Go: `unescape(b, attribute)` — unescapes b's entities, so that "a&lt;b" becomes "a<b".
// Go: html/escape.go:unescape
pub fn unescape(b: &[u8], attribute: bool) -> Vec<u8> {
    let Some(first) = b.iter().position(|&c| c == b'&') else {
        return b.to_vec();
    };
    let mut out = Vec::with_capacity(b.len());
    out.extend_from_slice(&b[..first]);
    let mut src = first;
    src += unescape_entity(&mut out, &b[src..], attribute);
    while src < b.len() {
        let c = b[src];
        if c == b'&' {
            src += unescape_entity(&mut out, &b[src..], attribute);
        } else {
            out.push(c);
            src += 1;
        }
    }
    out
}

/// Go: `lower(b)` — lower-cases the A-Z bytes in b, so that "aBc" becomes "abc".
// Go: html/escape.go:lower
pub fn lower(b: &[u8]) -> Vec<u8> {
    b.to_ascii_lowercase()
}
