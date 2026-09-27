//! Port of Go's `html` package (go1.27.1: `src/html/escape.go`,
//! `src/html/entity.go`): [`escape_string`] and [`unescape_string`].
//!
//! Go strings are bytes: the `*_bytes` forms take and return arbitrary bytes;
//! the `&str` forms are exact because both functions only replace ASCII-led
//! sequences and always emit valid UTF-8 for what they replace.

// Lints that fight a faithful line-by-line port of the Go control flow.
#![allow(clippy::if_same_then_else)]

mod entity;
mod utf8;

/// All entities that do not end with ';' are 6 or fewer bytes long.
const LONGEST_ENTITY_WITHOUT_SEMICOLON: usize = 6;

// These replacements permit compatibility with old numeric entities that
// assumed Windows-1252 encoding.
// https://html.spec.whatwg.org/multipage/parsing.html#numeric-character-reference-end-state
// Go: html/escape.go:replacementTable
static REPLACEMENT_TABLE: [i32; 32] = [
    0x20AC, // First entry is what 0x80 should be replaced with.
    0x0081, 0x201A, 0x0192, 0x201E, 0x2026, 0x2020, 0x2021, 0x02C6, 0x2030, 0x0160, 0x2039, 0x0152,
    0x008D, 0x017D, 0x008F, 0x0090, 0x2018, 0x2019, 0x201C, 0x201D, 0x2022, 0x2013, 0x2014, 0x02DC,
    0x2122, 0x0161, 0x203A, 0x0153, 0x009D, 0x017E,
    0x0178, // Last entry is 0x9F.
            // 0x00->'�' is handled programmatically.
            // 0x0D->'\u000D' is a no-op.
];

/// Go `entity[name]`: the rune for a named reference, or 0 if unknown.
/// `name` excludes the leading `&` and includes the trailing `;` if any.
pub fn entity(name: &[u8]) -> i32 {
    match entity::ENTITY.binary_search_by(|(k, _)| (*k).cmp(name)) {
        Ok(i) => entity::ENTITY[i].1,
        Err(_) => 0,
    }
}

/// Go `entity2[name]`: the two runes for a two-codepoint named reference, or
/// `[0, 0]` if unknown.
pub fn entity2(name: &[u8]) -> [i32; 2] {
    match entity::ENTITY2.binary_search_by(|(k, _)| (*k).cmp(name)) {
        Ok(i) => entity::ENTITY2[i].1,
        Err(_) => [0, 0],
    }
}

/// Iterates over the `entity` table (name, rune) in byte order of the name.
pub fn entities() -> impl Iterator<Item = (&'static [u8], i32)> {
    entity::ENTITY.iter().copied()
}

/// Iterates over the `entity2` table (name, [rune, rune]) in byte order of the name.
pub fn entities2() -> impl Iterator<Item = (&'static [u8], [i32; 2])> {
    entity::ENTITY2.iter().copied()
}

// Go: html/escape.go:unescapeEntity
/// Reads an entity like "&lt;" from b[src:] and writes the corresponding "<"
/// to b[dst:], returning the incremented dst and src cursors.
/// Precondition: b[src] == '&' && dst <= src.
fn unescape_entity(b: &mut [u8], dst: usize, src: usize) -> (usize, usize) {
    const ATTRIBUTE: bool = false;

    // http://www.whatwg.org/specs/web-apps/current-work/multipage/tokenization.html#consume-a-character-reference

    // i starts at 1 because we already know that s[0] == '&'.
    let mut i = 1usize;
    let slen = b.len() - src;

    if slen <= 1 {
        b[dst] = b[src];
        return (dst + 1, src + 1);
    }

    if b[src + i] == b'#' {
        if slen <= 3 {
            // We need to have at least "&#.".
            b[dst] = b[src];
            return (dst + 1, src + 1);
        }
        i += 1;
        let mut c = b[src + i];
        let mut hex = false;
        if c == b'x' || c == b'X' {
            hex = true;
            i += 1;
        }

        // Go rune (int32) arithmetic wraps.
        let mut x: i32 = 0;
        while i < slen {
            c = b[src + i];
            i += 1;
            if hex {
                if c.is_ascii_digit() {
                    x = 16i32
                        .wrapping_mul(x)
                        .wrapping_add(c as i32)
                        .wrapping_sub('0' as i32);
                    continue;
                } else if (b'a'..=b'f').contains(&c) {
                    x = 16i32
                        .wrapping_mul(x)
                        .wrapping_add(c as i32)
                        .wrapping_sub('a' as i32)
                        .wrapping_add(10);
                    continue;
                } else if (b'A'..=b'F').contains(&c) {
                    x = 16i32
                        .wrapping_mul(x)
                        .wrapping_add(c as i32)
                        .wrapping_sub('A' as i32)
                        .wrapping_add(10);
                    continue;
                }
            } else if c.is_ascii_digit() {
                x = 10i32
                    .wrapping_mul(x)
                    .wrapping_add(c as i32)
                    .wrapping_sub('0' as i32);
                continue;
            }
            if c != b';' {
                i -= 1;
            }
            break;
        }

        if i <= 3 {
            // No characters matched.
            b[dst] = b[src];
            return (dst + 1, src + 1);
        }

        if (0x80..=0x9F).contains(&x) {
            // Replace characters from Windows-1252 with UTF-8 equivalents.
            x = REPLACEMENT_TABLE[(x - 0x80) as usize];
        } else if x == 0 || (0xD800..=0xDFFF).contains(&x) || x > 0x10FFFF {
            // Replace invalid characters with the replacement character.
            x = 0xFFFD;
        }

        return (dst + utf8::encode_rune(&mut b[dst..], x), src + i);
    }

    // Consume the maximum number of characters possible, with the
    // consumed characters matching one of the named references.

    while i < slen {
        let c = b[src + i];
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

    let entity_name: Vec<u8> = b[src + 1..src + i].to_vec();
    if entity_name.is_empty() {
        // No-op.
    } else if ATTRIBUTE
        && entity_name[entity_name.len() - 1] != b';'
        && slen > i
        && b[src + i] == b'='
    {
        // No-op.
    } else {
        let x = entity(&entity_name);
        if x != 0 {
            return (dst + utf8::encode_rune(&mut b[dst..], x), src + i);
        }
        let x = entity2(&entity_name);
        if x[0] != 0 {
            let dst1 = dst + utf8::encode_rune(&mut b[dst..], x[0]);
            return (dst1 + utf8::encode_rune(&mut b[dst1..], x[1]), src + i);
        }
        if !ATTRIBUTE {
            let mut max_len = entity_name.len() as isize - 1;
            if max_len > LONGEST_ENTITY_WITHOUT_SEMICOLON as isize {
                max_len = LONGEST_ENTITY_WITHOUT_SEMICOLON as isize;
            }
            let mut j = max_len;
            while j > 1 {
                let x = entity(&entity_name[..j as usize]);
                if x != 0 {
                    return (
                        dst + utf8::encode_rune(&mut b[dst..], x),
                        src + j as usize + 1,
                    );
                }
                j -= 1;
            }
        }
    }

    let (dst1, src1) = (dst + i, src + i);
    b.copy_within(src..src1, dst);
    (dst1, src1)
}

// Go: html/escape.go:htmlEscaper + EscapeString
/// Escapes `<`, `>`, `&`, `'` and `"` (Go `html.EscapeString`), using
/// `&#39;` and `&#34;` for the quotes.
pub fn escape_string_bytes(s: &[u8]) -> Vec<u8> {
    // strings.NewReplacer with single-byte olds is a byteStringReplacer: each
    // byte is replaced independently.
    let mut out = Vec::with_capacity(s.len());
    for &c in s {
        match c {
            b'&' => out.extend_from_slice(b"&amp;"),
            b'\'' => out.extend_from_slice(b"&#39;"), // "&#39;" is shorter than "&apos;" and apos was not in HTML until HTML5.
            b'<' => out.extend_from_slice(b"&lt;"),
            b'>' => out.extend_from_slice(b"&gt;"),
            b'"' => out.extend_from_slice(b"&#34;"), // "&#34;" is shorter than "&quot;".
            _ => out.push(c),
        }
    }
    out
}

/// `&str` form of [`escape_string_bytes`].
pub fn escape_string(s: &str) -> String {
    String::from_utf8(escape_string_bytes(s.as_bytes())).expect("escaping only replaces ASCII")
}

fn index_byte(s: &[u8], c: u8) -> isize {
    match s.iter().position(|&b| b == c) {
        Some(i) => i as isize,
        None => -1,
    }
}

// Go: html/escape.go:UnescapeString
/// Unescapes entities like "&lt;" to become "<" (Go `html.UnescapeString`).
pub fn unescape_string_bytes(s: &[u8]) -> Vec<u8> {
    let i = index_byte(s, b'&');

    if i < 0 {
        return s.to_vec();
    }

    let mut b = s.to_vec();
    let (mut dst, mut src) = unescape_entity(&mut b, i as usize, i as usize);
    while s.len() > src {
        let i = if s[src] == b'&' {
            0
        } else {
            index_byte(&s[src..], b'&')
        };
        if i < 0 {
            let n = s.len() - src;
            b[dst..dst + n].copy_from_slice(&s[src..]);
            dst += n;
            break;
        }
        let i = i as usize;

        if i > 0 {
            b[dst..dst + i].copy_from_slice(&s[src..src + i]);
        }
        (dst, src) = unescape_entity(&mut b, dst + i, src + i);
    }
    b.truncate(dst);
    b
}

/// `&str` form of [`unescape_string_bytes`]. Valid UTF-8 input always yields
/// valid UTF-8 output.
pub fn unescape_string(s: &str) -> String {
    String::from_utf8(unescape_string_bytes(s.as_bytes()))
        .expect("unescaping valid UTF-8 yields valid UTF-8")
}
