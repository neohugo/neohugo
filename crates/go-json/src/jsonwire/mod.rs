//! Port of `encoding/json/internal/jsonwire` (go1.27.1): low-level
//! consumption and formatting of JSON grammar (`wire.go` here, `decode.go`
//! and `encode.go` in submodules).

mod decode;
mod encode;
#[cfg(test)]
mod tests;

pub(crate) use decode::*;
pub(crate) use encode::*;

use go_unicode::Rune;
use go_unicode::utf8;

// Go: wire.go:QuoteRune
/// QuoteRune quotes the first rune in the input.
pub(crate) fn quote_rune(b: &[u8]) -> String {
    let (r, n) = utf8::decode_rune(b);
    if r == utf8::RUNE_ERROR && n == 1 {
        return format!("'\\x{}'", go_strconv::format_uint(b[0] as u64, 16));
    }
    go_strconv::quote_rune(r)
}

// Go: wire.go:NewInvalidCharacterError
/// NewInvalidCharacterError returns an [InvalidTextError] for the first
/// rune of `prefix`.
pub(crate) fn new_invalid_character_error(prefix: &[u8], where_: &str) -> crate::goerr::Err {
    let (_, n) = utf8::decode_rune(prefix);
    crate::goerr::Err::InvalidText(InvalidTextError {
        label: "character",
        what: prefix[..n].to_vec(),
        where_: where_.to_string(),
    })
}

// Go: wire.go:NewInvalidEscapeSequenceError
pub(crate) fn new_invalid_escape_sequence_error(what: &[u8]) -> crate::goerr::Err {
    if what.len() > 6 {
        return crate::goerr::Err::InvalidText(InvalidTextError {
            label: "surrogate pair",
            what: what.to_vec(),
            where_: "in string".to_string(),
        });
    }
    crate::goerr::Err::InvalidText(InvalidTextError {
        label: "escape sequence",
        what: what.to_vec(),
        where_: "in string".to_string(),
    })
}

// Go: wire.go:InvalidTextError
/// InvalidTextError is an error for invalid text in JSON.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct InvalidTextError {
    /// e.g., "character" | "escape sequence" | "surrogate pair"
    pub(crate) label: &'static str,
    /// raw invalid text
    pub(crate) what: Vec<u8>,
    /// e.g., "in string" | "at start of value"
    pub(crate) where_: String,
}

impl InvalidTextError {
    // Go: wire.go:InvalidTextError.Error
    pub(crate) fn error(&self) -> String {
        let what;
        let need_escape = go_unicode::strings::contains_func(&self.what, |r: Rune| {
            r == '`' as Rune
                || r == utf8::RUNE_ERROR
                || go_unicode::is_space(r)
                || !go_unicode::is_print(r)
        });
        if utf8::rune_count(&self.what) == 1 {
            what = quote_rune(&self.what);
        } else if need_escape {
            what = go_strconv::quote(&self.what);
        } else {
            // Valid UTF-8: need_escape would be set otherwise.
            what = format!("`{}`", String::from_utf8_lossy(&self.what));
        }
        let s = format!("invalid {} {} {}", self.label, what, self.where_);
        match s.strip_suffix(' ') {
            Some(t) => t.to_string(),
            None => s,
        }
    }
}

// Go: wire.go:TruncatePointer
/// TruncatePointer optionally truncates the JSON pointer,
/// enforcing that the length roughly does not exceed n.
pub(crate) fn truncate_pointer(s: &[u8], n: usize) -> Vec<u8> {
    if s.len() <= n {
        return s.to_vec();
    }
    let mut i = n / 2;
    let mut j = s.len() - n / 2;

    // Avoid truncating a name if there are multiple names present.
    if let Some(k) = s[..i].iter().rposition(|&c| c == b'/') {
        if k > 0 {
            i = k;
        }
    }
    if let Some(k) = s[j..].iter().position(|&c| c == b'/') {
        j += k + "/".len();
    }

    // Avoid truncation in the middle of a UTF-8 rune.
    while i > 0 && {
        let (r, rn) = utf8::decode_last_rune(&s[..i]);
        is_invalid_utf8(r, rn)
    } {
        i -= 1;
    }
    while j < s.len() && {
        let (r, rn) = utf8::decode_rune(&s[j..]);
        is_invalid_utf8(r, rn)
    } {
        j += 1;
    }

    // Determine the right middle fragment to use.
    let mid = &s[i..j];
    let mut middle: &str = match mid.iter().filter(|&&c| c == b'/').count() {
        0 => "…",
        1 => "…/…",
        _ => "…/…/…",
    };
    if mid.starts_with(b"/") && middle != "…" {
        middle = middle.strip_prefix('…').unwrap_or(middle);
    }
    if mid.ends_with(b"/") && middle != "…" {
        middle = middle.strip_suffix('…').unwrap_or(middle);
    }
    let mut out = s[..i].to_vec();
    out.extend_from_slice(middle.as_bytes());
    out.extend_from_slice(&s[j..]);
    out
}

// Go: wire.go:isInvalidUTF8
pub(crate) fn is_invalid_utf8(r: Rune, rn: usize) -> bool {
    r == utf8::RUNE_ERROR && rn == 1
}
