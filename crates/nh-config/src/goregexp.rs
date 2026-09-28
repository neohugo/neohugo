//! Go `regexp` (RE2) over the `regex` crate, for the few patterns Hugo's config compiles:
//! the security whitelists (`security.exec.allow`, `osEnv`, `funcs.getenv`, `http.*`), the
//! build cache busters and the server redirects (`fromRe`).
//!
//! RE2 and Rust `regex` share the syntax and the leftmost-first match semantics. The
//! differences are rewritten before compiling:
//! * Perl classes and word boundaries are ASCII in RE2 and Unicode in Rust: `\d \D \s \S \w \W
//!   \b \B` get Go's ASCII definitions (`\s` is `[\t\n\f\r ]`, without `\v`);
//! * RE2's `\Q...\E` literals, octal escapes (`\0`, `\12`, `\101`) and negated Unicode classes
//!   (`\p{^Greek}`) become their Rust spellings;
//! * syntax errors are reported with Go's `regexp/syntax` texts for the common kinds, and the
//!   patterns Go rejects but Rust accepts (stacked repetitions `a**`, repeat counts over 1000,
//!   `\1` backreferences) are rejected like Go.

use std::sync::Arc;

/// A compiled Go regexp.
#[derive(Clone, Debug)]
pub struct Regexp {
    source: String,
    re: Arc<regex::Regex>,
}

impl PartialEq for Regexp {
    fn eq(&self, other: &Self) -> bool {
        self.source == other.source
    }
}

impl Regexp {
    /// Go: `regexp.Compile(expr)`.
    pub fn compile(expr: &str) -> Result<Regexp, String> {
        let expanded = expand_quotes(expr);
        if let Some(e) = go_syntax_error(&expanded) {
            return Err(e);
        }
        let translated = translate(&expanded);
        match regex::Regex::new(&translated) {
            Ok(re) => Ok(Regexp {
                source: expr.to_string(),
                re: Arc::new(re),
            }),
            Err(e) => Err(format!("error parsing regexp: {e}")),
        }
    }

    /// Go: `re.String()`.
    pub fn string(&self) -> &str {
        &self.source
    }

    /// Go: `re.MatchString(s)`.
    pub fn match_string(&self, s: &str) -> bool {
        self.re.is_match(s)
    }

    /// Go: `re.FindStringSubmatch(s)`: the match and its groups ("" for groups that did not
    /// participate), or `None`.
    pub fn find_string_submatch(&self, s: &str) -> Option<Vec<String>> {
        let caps = self.re.captures(s)?;
        Some(
            (0..caps.len())
                .map(|i| {
                    caps.get(i)
                        .map(|m| m.as_str().to_string())
                        .unwrap_or_default()
                })
                .collect(),
        )
    }
}

/// RE2's `\Q...\E` (outside classes): the text in between is literal.
fn expand_quotes(expr: &str) -> String {
    if !expr.contains("\\Q") {
        return expr.to_string();
    }
    let b = expr.as_bytes();
    let mut out = String::with_capacity(expr.len());
    let mut i = 0;
    let mut in_class = false;
    while i < b.len() {
        let c = b[i];
        if c == b'\\' && i + 1 < b.len() {
            if !in_class && b[i + 1] == b'Q' {
                let rest = &expr[i + 2..];
                let (lit, next) = match rest.find("\\E") {
                    Some(e) => (&rest[..e], i + 2 + e + 2),
                    None => (rest, b.len()),
                };
                out.push_str(&regex_syntax::escape(lit));
                i = next;
                continue;
            }
            let ch = expr[i + 1..].chars().next().unwrap_or('\\');
            out.push('\\');
            out.push(ch);
            i += 1 + ch.len_utf8();
            continue;
        }
        if c == b'[' && !in_class {
            in_class = true;
        } else if c == b']' && in_class {
            in_class = false;
        }
        let ch = expr[i..].chars().next().unwrap_or('\u{fffd}');
        out.push(ch);
        i += ch.len_utf8();
    }
    out
}

/// A repeat `{n}`, `{n,}` or `{n,m}` at the start of `s` (Go `parseRepeat`: no leading zeros):
/// (min, max (-1 = none), length).
fn parse_repeat(s: &[u8]) -> Option<(i64, i64, usize)> {
    if s.first() != Some(&b'{') {
        return None;
    }
    let mut i = 1;
    let parse_int = |i: &mut usize| -> Option<i64> {
        let start = *i;
        while *i < s.len() && s[*i].is_ascii_digit() {
            *i += 1;
        }
        if *i == start {
            return None;
        }
        // Disallow leading zeros.
        if *i - start >= 2 && s[start] == b'0' {
            return None;
        }
        let mut n: i64 = 0;
        for &d in &s[start..*i] {
            if n >= 100_000_000 {
                // Avoid overflow (Go: -1 for too-large numbers).
                return Some(-1);
            }
            n = n * 10 + i64::from(d - b'0');
        }
        Some(n)
    };
    let min = parse_int(&mut i)?;
    let max;
    if s.get(i) == Some(&b',') {
        i += 1;
        if s.get(i) == Some(&b'}') {
            max = -1;
        } else {
            max = parse_int(&mut i)?;
            if max < 0 {
                return Some((min, -2, i + 1));
            }
        }
    } else {
        max = min;
    }
    if s.get(i) != Some(&b'}') {
        return None;
    }
    Some((min, max, i + 1))
}

/// Go's errors for the patterns Rust's parser accepts: a repetition operator right after
/// another (`invalid nested repetition operator`), a repeat count over 1000 or with min > max
/// (`invalid repeat count`), and `\1`..`\7` without a following octal digit (a backreference:
/// `invalid escape sequence`). Returns the error's position and text.
fn go_prescan(expr: &str) -> Option<(usize, String)> {
    let b = expr.as_bytes();
    let mut i = 0;
    let mut in_class = false;
    let mut class_start = false;
    // Start of the previous token when it was a repetition operator.
    let mut last_repeat: Option<usize> = None;
    let err = |pos: usize, code: &str, e: &str| {
        Some((pos, format!("error parsing regexp: {code}: `{e}`")))
    };
    while i < b.len() {
        let c = b[i];
        if in_class {
            match c {
                b'\\' if i + 1 < b.len() => {
                    let n = b[i + 1];
                    if (b'1'..=b'7').contains(&n)
                        && !b.get(i + 2).is_some_and(|d| (b'0'..=b'7').contains(d))
                    {
                        return err(i, "invalid escape sequence", &expr[i..i + 2]);
                    }
                    i += 2;
                }
                b'[' if b.get(i + 1) == Some(&b':') => match expr[i + 2..].find(":]") {
                    Some(end) => i += 2 + end + 2,
                    None => i += 1,
                },
                b']' if !class_start => {
                    in_class = false;
                    i += 1;
                }
                _ => i += 1,
            }
            class_start = false;
            continue;
        }
        match c {
            b'\\' if i + 1 < b.len() => {
                let n = b[i + 1];
                if (b'1'..=b'7').contains(&n)
                    && !b.get(i + 2).is_some_and(|d| (b'0'..=b'7').contains(d))
                {
                    return err(i, "invalid escape sequence", &expr[i..i + 2]);
                }
                let ch = expr[i + 1..].chars().next().unwrap_or('\\');
                i += 1 + ch.len_utf8();
                last_repeat = None;
            }
            b'[' => {
                in_class = true;
                class_start = true;
                i += 1;
                if b.get(i) == Some(&b'^') {
                    i += 1;
                }
                last_repeat = None;
            }
            b'*' | b'+' | b'?' | b'{' => {
                let mut end;
                if c == b'{' {
                    match parse_repeat(&b[i..]) {
                        None => {
                            // A literal `{`.
                            i += 1;
                            last_repeat = None;
                            continue;
                        }
                        Some((min, max, len)) => {
                            end = i + len;
                            if !(0..=1000).contains(&min)
                                || max > 1000
                                || max == -2
                                || (max >= 0 && min > max)
                            {
                                return err(i, "invalid repeat count", &expr[i..end]);
                            }
                        }
                    }
                } else {
                    end = i + 1;
                }
                if b.get(end) == Some(&b'?') {
                    end += 1;
                }
                if let Some(prev) = last_repeat {
                    return err(prev, "invalid nested repetition operator", &expr[prev..end]);
                }
                last_repeat = Some(i);
                i = end;
            }
            _ => {
                let ch = expr[i..].chars().next().unwrap_or('\u{fffd}');
                i += ch.len_utf8();
                last_repeat = None;
            }
        }
    }
    None
}

/// Go's `regexp.Compile` error text (`error parsing regexp: <code>: `<expr>``) for the syntax
/// errors whose Rust counterpart is unambiguous: a repetition operator without argument, an
/// unclosed or unopened group, an unclosed class, an invalid escape, a trailing backslash, an
/// invalid class range or repeat count, and the errors of [`go_prescan`]. `None` when the
/// expression parses (or for other errors, which keep the `regex` crate's text).
fn go_syntax_error(expr: &str) -> Option<String> {
    use regex_syntax::ast::ErrorKind as K;
    let pre = go_prescan(expr);
    let ast_err = regex_syntax::ast::parse::ParserBuilder::new()
        .octal(true)
        .build()
        .parse(expr)
        .err();
    let Some(err) = ast_err else {
        return pre.map(|(_, e)| e);
    };
    let span = err.span();
    if let Some((pos, e)) = &pre
        && *pos <= span.start.offset
    {
        // Go reports the first error.
        return Some(e.clone());
    }
    let at = |start: usize, end: usize| expr.get(start..end).unwrap_or("").to_string();
    let (code, e) = match err.kind() {
        K::RepetitionMissing => {
            // The operator, with its non-greedy `?`.
            let start = span.start.offset;
            let rest = &expr[start..];
            let mut len = rest.chars().next().map_or(0, char::len_utf8);
            if rest.starts_with('{')
                && let Some(end) = rest.find('}')
            {
                len = end + 1;
            }
            if rest[len..].starts_with('?') {
                len += 1;
            }
            (
                "missing argument to repetition operator",
                at(start, start + len),
            )
        }
        K::RepetitionCountInvalid | K::RepetitionCountDecimalEmpty => {
            let start = expr[..span.end.offset.min(expr.len())]
                .rfind('{')
                .unwrap_or(span.start.offset);
            let end = expr[start..]
                .find('}')
                .map_or(expr.len(), |e| start + e + 1);
            ("invalid repeat count", at(start, end))
        }
        K::GroupUnclosed => ("missing closing )", expr.to_string()),
        K::GroupUnopened => ("unexpected )", expr.to_string()),
        K::ClassUnclosed => ("missing closing ]", at(span.start.offset, expr.len())),
        K::EscapeUnrecognized | K::UnsupportedBackreference => (
            "invalid escape sequence",
            at(span.start.offset, span.end.offset),
        ),
        K::EscapeUnexpectedEof => ("trailing backslash at end of expression", String::new()),
        K::ClassRangeInvalid => (
            "invalid character class range",
            at(span.start.offset, span.end.offset),
        ),
        _ => return pre.map(|(_, e)| e),
    };
    Some(format!("error parsing regexp: {code}: `{e}`"))
}

/// An octal escape at `b[i..]` (after the backslash): Go's `\0`.. and `\1`..`\7` followed by an
/// octal digit, up to 3 digits: (code point, digits).
fn octal_escape(b: &[u8]) -> Option<(u32, usize)> {
    let first = *b.first()?;
    if !(b'0'..=b'7').contains(&first) {
        return None;
    }
    if first != b'0' && !b.get(1).is_some_and(|d| (b'0'..=b'7').contains(d)) {
        return None;
    }
    let mut r = u32::from(first - b'0');
    let mut n = 1;
    while n < 3 && b.get(n).is_some_and(|d| (b'0'..=b'7').contains(d)) {
        r = r * 8 + u32::from(b[n] - b'0');
        n += 1;
    }
    Some((r, n))
}

/// Rewrites RE2's ASCII Perl classes, octal escapes and negated Unicode classes for the `regex`
/// crate.
fn translate(expr: &str) -> String {
    let b = expr.as_bytes();
    let mut out = String::with_capacity(expr.len() + 16);
    let mut in_class = false;
    let mut class_start = false;
    let mut i = 0;
    while i < b.len() {
        let c = b[i];
        if c == b'\\' && i + 1 < b.len() {
            let n = b[i + 1];
            if let Some((r, len)) = octal_escape(&b[i + 1..]) {
                out.push_str(&format!("\\x{{{r:X}}}"));
                i += 1 + len;
                class_start = false;
                continue;
            }
            if (n == b'p' || n == b'P')
                && b.get(i + 2) == Some(&b'{')
                && b.get(i + 3) == Some(&b'^')
            {
                // Go's negated class `\p{^X}` = `\P{X}`.
                out.push('\\');
                out.push(if n == b'p' { 'P' } else { 'p' });
                out.push('{');
                i += 4;
                class_start = false;
                continue;
            }
            let rep: Option<(&str, &str)> = match n {
                b'd' => Some(("[0-9]", "0-9")),
                b'D' => Some(("[^0-9]", "[^0-9]")),
                b's' => Some(("[\\t\\n\\f\\r ]", "\\t\\n\\f\\r ")),
                b'S' => Some(("[^\\t\\n\\f\\r ]", "[^\\t\\n\\f\\r ]")),
                b'w' => Some(("[0-9A-Za-z_]", "0-9A-Za-z_")),
                b'W' => Some(("[^0-9A-Za-z_]", "[^0-9A-Za-z_]")),
                b'b' if !in_class => Some(("(?-u:\\b)", "")),
                b'B' if !in_class => Some(("(?-u:\\B)", "")),
                _ => None,
            };
            match rep {
                Some((outside, inside)) => {
                    out.push_str(if in_class { inside } else { outside });
                }
                None => {
                    out.push('\\');
                    // Copy the escaped character (which may be multi-byte).
                    let ch = expr[i + 1..].chars().next().unwrap_or('\\');
                    out.push(ch);
                    i += 1 + ch.len_utf8();
                    class_start = false;
                    continue;
                }
            }
            i += 2;
            class_start = false;
            continue;
        }
        if in_class {
            if c == b']' && !class_start {
                in_class = false;
            } else if c == b'[' && b.get(i + 1) == Some(&b':') {
                // A POSIX class like [:alpha:]: copy it whole.
                if let Some(end) = expr[i + 2..].find(":]") {
                    out.push_str(&expr[i..i + 2 + end + 2]);
                    i += 2 + end + 2;
                    class_start = false;
                    continue;
                }
            }
            class_start = false;
        } else if c == b'[' {
            in_class = true;
            class_start = true;
            out.push('[');
            i += 1;
            if b.get(i) == Some(&b'^') {
                out.push('^');
                i += 1;
            }
            continue;
        }
        let ch = expr[i..].chars().next().unwrap_or('\u{fffd}');
        out.push(ch);
        i += ch.len_utf8();
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn perl_classes_are_ascii() {
        let re = Regexp::compile(r"^\w+$").unwrap();
        assert!(re.match_string("GOPATH_1"));
        assert!(!re.match_string("é"));
        let re = Regexp::compile(r"^[\w.-]+\s$").unwrap();
        assert!(re.match_string("a.b- "));
        assert!(!re.match_string("a\u{b}"));
        let re = Regexp::compile(r"[]a]").unwrap();
        assert!(re.match_string("]"));
        let re = Regexp::compile(r"(?i)^((HTTPS?|NO)_PROXY|GO\w+)$").unwrap();
        assert!(re.match_string("GOROOT"));
        assert!(re.match_string("https_proxy"));
    }

    #[test]
    fn go_only_syntax() {
        assert!(Regexp::compile(r"\101").unwrap().match_string("A"));
        assert!(Regexp::compile(r"\Q.*\E").unwrap().match_string("x.*"));
        assert!(Regexp::compile(r"^\p{^L}$").unwrap().match_string("1"));
        assert_eq!(
            Regexp::compile("a**").unwrap_err(),
            "error parsing regexp: invalid nested repetition operator: `**`"
        );
        assert_eq!(
            Regexp::compile("a{1001}").unwrap_err(),
            "error parsing regexp: invalid repeat count: `{1001}`"
        );
        assert_eq!(
            Regexp::compile(r"\1").unwrap_err(),
            "error parsing regexp: invalid escape sequence: `\\1`"
        );
    }
}
