//! Port of `github.com/mattn/go-runewidth@v0.0.16` (`runewidth.go`, `runewidth_posix.go`): the
//! display width of runes and strings, as tablewriter's `twwidth` uses it.

use std::sync::OnceLock;

use super::runewidth_tables::*;
use super::uniseg::Graphemes;

/// Go: `nonprint`.
static NONPRINT: &[(i32, i32)] = &[
    (0x0000, 0x001F),
    (0x007F, 0x009F),
    (0x00AD, 0x00AD),
    (0x070F, 0x070F),
    (0x180B, 0x180E),
    (0x200B, 0x200F),
    (0x2028, 0x202E),
    (0x206A, 0x206F),
    (0xD800, 0xDFFF),
    (0xFEFF, 0xFEFF),
    (0xFFF9, 0xFFFB),
    (0xFFFE, 0xFFFF),
];

// Go: runewidth.go:inTable
fn in_table(r: i32, t: &[(i32, i32)]) -> bool {
    if r < t[0].0 {
        return false;
    }

    let mut bot: i64 = 0;
    let mut top: i64 = t.len() as i64 - 1;
    while top >= bot {
        let mid = ((bot + top) >> 1) as usize;
        if t[mid].1 < r {
            bot = mid as i64 + 1;
        } else if t[mid].0 > r {
            top = mid as i64 - 1;
        } else {
            return true;
        }
    }

    false
}

// Go: runewidth.go:inTables
fn in_tables(r: i32, ts: &[&[(i32, i32)]]) -> bool {
    ts.iter().any(|t| in_table(r, t))
}

/// Go: `runewidth.EastAsianWidth` as `handleEnv` sets it at init: `RUNEWIDTH_EASTASIAN`
/// ("1" = true), else `IsEastAsian()` from the locale.
// Go: runewidth.go:handleEnv
pub(crate) fn east_asian_width() -> bool {
    static EA: OnceLock<bool> = OnceLock::new();
    *EA.get_or_init(|| {
        let getenv = |k: &str| std::env::var(k).unwrap_or_default();
        let env = getenv("RUNEWIDTH_EASTASIAN");
        if env.is_empty() {
            is_east_asian_env(&getenv)
        } else {
            env == "1"
        }
    })
}

/// Go: `IsEastAsian()` (posix) over the given environment.
// Go: runewidth_posix.go:IsEastAsian
fn is_east_asian_env(getenv: &dyn Fn(&str) -> String) -> bool {
    let mut locale = getenv("LC_ALL");
    if locale.is_empty() {
        locale = getenv("LC_CTYPE");
    }
    if locale.is_empty() {
        locale = getenv("LANG");
    }

    // ignore C locale
    if locale == "POSIX" || locale == "C" {
        return false;
    }
    let b = locale.as_bytes();
    if b.len() > 1 && b[0] == b'C' && (b[1] == b'.' || b[1] == b'-') {
        return false;
    }

    is_east_asian(&locale)
}

/// Go `reLoc = ^[a-z][a-z][a-z]?(?:_[A-Z][A-Z])?\.(.+)`: the charset after the dot.
fn re_loc(locale: &str) -> Option<&str> {
    let b = locale.as_bytes();
    let lower = |i: usize| b.get(i).is_some_and(|c| c.is_ascii_lowercase());
    let upper = |i: usize| b.get(i).is_some_and(|c| c.is_ascii_uppercase());
    if !(lower(0) && lower(1)) {
        return None;
    }
    // The regexp backtracks over the optional parts; try every combination in order.
    for n in [3usize, 2] {
        if n == 3 && !lower(2) {
            continue;
        }
        for with_region in [true, false] {
            let mut i = n;
            if with_region {
                if b.get(i) == Some(&b'_') && upper(i + 1) && upper(i + 2) {
                    i += 3;
                } else {
                    continue;
                }
            }
            // `.(.+)`: a dot then at least one character that is not a newline.
            if b.get(i) == Some(&b'.') {
                let rest = &locale[i + 1..];
                if !rest.is_empty() && !rest.starts_with('\n') {
                    let end = rest.find('\n').unwrap_or(rest.len());
                    return Some(&rest[..end]);
                }
            }
        }
    }
    None
}

// Go: runewidth_posix.go:isEastAsian
fn is_east_asian(locale: &str) -> bool {
    let mut charset = go_unicode::strings::to_lower_str(locale).into_owned();
    if let Some(r) = re_loc(locale) {
        charset = go_unicode::strings::to_lower_str(r).into_owned();
    }

    if charset.ends_with("@cjk_narrow") {
        return false;
    }

    if let Some(pos) = charset.find('@') {
        charset.truncate(pos);
    }
    let max = match charset.as_str() {
        "utf-8" | "utf8" => 6,
        "jis" => 8,
        "eucjp" => 3,
        "euckr" | "euccn" | "sjis" | "cp932" | "cp51932" | "cp936" | "cp949" | "cp950" | "big5"
        | "gbk" | "gb2312" => 2,
        _ => 1,
    };
    max > 1
        && (!charset.starts_with('u')
            || locale.starts_with("ja")
            || locale.starts_with("ko")
            || locale.starts_with("zh"))
}

/// Go: `runewidth.Condition` (`StrictEmojiNeutral` is always true here).
#[derive(Clone, Copy)]
pub(crate) struct Condition {
    pub(crate) east_asian_width: bool,
}

impl Condition {
    /// Go: `(*Condition).RuneWidth(r)` (no lookup table is created by tablewriter).
    // Go: runewidth.go:RuneWidth
    pub(crate) fn rune_width(&self, r: i32) -> i64 {
        if !(0..=0x10FFFF).contains(&r) {
            return 0;
        }
        if !self.east_asian_width {
            if r < 0x20 {
                return 0;
            }
            if (0x7F..=0x9F).contains(&r) || r == 0xAD {
                // nonprint
                return 0;
            }
            if r < 0x300 {
                return 1;
            }
            if in_table(r, NARROW) {
                return 1;
            }
            if in_tables(r, &[NONPRINT, COMBINING]) {
                return 0;
            }
            if in_table(r, DOUBLEWIDTH) {
                return 2;
            }
            1
        } else {
            if in_tables(r, &[NONPRINT, COMBINING]) {
                return 0;
            }
            if in_table(r, NARROW) {
                return 1;
            }
            if in_tables(r, &[AMBIGUOUS, DOUBLEWIDTH]) {
                return 2;
            }
            // !StrictEmojiNeutral && inTables(r, ambiguous, emoji, narrow): never (strict).
            1
        }
    }

    /// Go: `(*Condition).StringWidth(s)` — per grapheme cluster, the width of its first rune
    /// with a non-zero width.
    // Go: runewidth.go:StringWidth
    pub(crate) fn string_width(&self, s: &str) -> i64 {
        let mut width = 0;
        let mut g = Graphemes::new(s);
        while g.next() {
            let mut ch_width = 0;
            for &r in g.runes() {
                ch_width = self.rune_width(r);
                if ch_width > 0 {
                    break; // Our best guess at this point is to use the width of the first non-zero-width rune.
                }
            }
            width += ch_width;
        }
        width
    }
}
