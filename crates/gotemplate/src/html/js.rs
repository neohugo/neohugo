//! Go: tpl/internal/go_templates/htmltemplate/js.go (+ the
//! `specialScriptTagRE` helpers of escape.go).

use go_unicode::{Rune, bytes, strings, utf8};
use go_value::{SafeKind, Value};

use super::content::{
    ContentType, indirect_to_json_marshaler, is_json_marshaler, stringer_string, stringify,
};
use super::context::JsCtx;

// Go: js.go:jsWhitespace
/// jsWhitespace contains all of the JS whitespace characters, as defined
/// by the \s character class.
/// See <https://developer.mozilla.org/en-US/docs/Web/JavaScript/Guide/Regular_expressions/Character_classes>.
const JS_WHITESPACE: &str = "\x0c\n\r\t\x0b\u{0020}\u{00a0}\u{1680}\u{2000}\u{2001}\u{2002}\u{2003}\u{2004}\u{2005}\u{2006}\u{2007}\u{2008}\u{2009}\u{200a}\u{2028}\u{2029}\u{202f}\u{205f}\u{3000}\u{feff}";

// Go: js.go:nextJSCtx
/// nextJSCtx returns the context that determines whether a slash after the
/// given run of tokens starts a regular expression instead of a division
/// operator: / or /=.
///
/// This assumes that the token run does not include any string tokens,
/// comment tokens, regular expression literal tokens, or division operators.
///
/// This fails on some valid but nonsensical JavaScript programs like
/// "x = ++/foo/i" which is quite different than "x++/foo/i", but is not
/// known to fail on any known useful programs. It is based on the draft
/// JavaScript 2.0 lexical grammar and requires one token of lookbehind:
/// <https://www.mozilla.org/js/language/js20-2000-07/rationale/syntax.html>
pub(crate) fn next_js_ctx(s: &[u8], preceding: JsCtx) -> JsCtx {
    // Trim all JS whitespace characters
    let s = bytes::trim_right(s, JS_WHITESPACE.as_bytes());
    if s.is_empty() {
        return preceding;
    }

    // All cases below are in the single-byte UTF-8 group.
    let n = s.len();
    let c = s[n - 1];
    match c {
        b'+' | b'-' => {
            // ++ and -- are not regexp preceders, but + and - are whether
            // they are used as infix or prefix operators.
            let mut start = n - 1;
            // Count the number of adjacent dashes or pluses.
            while start > 0 && s[start - 1] == c {
                start -= 1;
            }
            if (n - start) & 1 == 1 {
                // Reached for trailing minus signs since "---" is the
                // same as "-- -".
                return JsCtx::Regexp;
            }
            return JsCtx::DivOp;
        }
        b'.' => {
            // Handle "42."
            if n != 1 && b'0' <= s[n - 2] && s[n - 2] <= b'9' {
                return JsCtx::DivOp;
            }
            return JsCtx::Regexp;
        }
        // Suffixes for all punctuators from section 7.7 of the language spec
        // that only end binary operators not handled above.
        b',' | b'<' | b'>' | b'=' | b'*' | b'%' | b'&' | b'|' | b'^' | b'?' => {
            return JsCtx::Regexp;
        }
        // Suffixes for all punctuators from section 7.7 of the language spec
        // that are prefix operators not handled above.
        b'!' | b'~' => return JsCtx::Regexp,
        // Matches all the punctuators from section 7.7 of the language spec
        // that are open brackets not handled above.
        b'(' | b'[' => return JsCtx::Regexp,
        // Matches all the punctuators from section 7.7 of the language spec
        // that precede expression starts.
        b':' | b';' | b'{' => return JsCtx::Regexp,
        // CAVEAT: the close punctuators ('}', ']', ')') precede div ops and
        // are handled in the default except for '}' which can precede a
        // division op as in
        //    ({ valueOf: function () { return 42 } } / 2
        // which is valid, but, in practice, developers don't divide object
        // literals, so our heuristic works well for code like
        //    function () { ... }  /foo/.test(x) && sideEffect();
        // The ')' punctuator can precede a regular expression as in
        //     if (b) /foo/.test(x) && ...
        // but this is much less likely than
        //     (a + b) / c
        b'}' => return JsCtx::Regexp,
        _ => {
            // Look for an IdentifierName and see if it is a keyword that
            // can precede a regular expression.
            let mut j = n;
            while j > 0 && is_js_ident_part(s[j - 1] as Rune) {
                j -= 1;
            }
            if regexp_preceder_keywords(&s[j..]) {
                return JsCtx::Regexp;
            }
        }
    }
    // Otherwise is a punctuator not listed above, or
    // a string which precedes a div op, or an identifier
    // which precedes a div op.
    JsCtx::DivOp
}

// Go: js.go:regexpPrecederKeywords
/// regexpPrecederKeywords is a set of reserved JS keywords that can precede
/// a regular expression in JS source.
pub(crate) fn regexp_preceder_keywords(s: &[u8]) -> bool {
    matches!(
        s,
        b"break"
            | b"case"
            | b"continue"
            | b"delete"
            | b"do"
            | b"else"
            | b"finally"
            | b"in"
            | b"instanceof"
            | b"return"
            | b"throw"
            | b"try"
            | b"typeof"
            | b"void"
    )
}

// ---------------------------------------------------------------------------
// Case-insensitive regexps, hand-coded.
//
// Go compiles `(?i)x` for a literal rune x to "x or any rune in its
// unicode.SimpleFold orbit" (regexp/syntax Inst.MatchRune with FoldCase).
// For the ASCII letters of "script" the orbits are {c,C}, {r,R}, {i,I},
// {p,P}, {t,T} and {s,S,U+017F LATIN SMALL LETTER LONG S}: `<ſcript` matches.
// `<`, `/`, `!` and `-` have no case folding. The input is decoded like the
// regexp engine does (invalid UTF-8 → U+FFFD of width 1, which matches none
// of these literals).

/// Whether rune `r` matches the literal `lit` under Go's `(?i)`.
fn fold_match(r: Rune, lit: Rune) -> bool {
    if r == lit {
        return true;
    }
    let mut r1 = go_unicode::simple_fold(lit);
    while r1 != lit {
        if r == r1 {
            return true;
        }
        r1 = go_unicode::simple_fold(r1);
    }
    false
}

/// Matches the case-insensitive literal `lit` at `s[pos..]`, returning the
/// end offset.
fn match_fold(s: &[u8], mut pos: usize, lit: &[u8]) -> Option<usize> {
    for &l in lit {
        if pos >= s.len() {
            return None;
        }
        let (r, w) = utf8::decode_rune(&s[pos..]);
        if !fold_match(r, l as Rune) {
            return None;
        }
        pos += w;
    }
    Some(pos)
}

/// Go: `regexp.MustCompile("(?i)<(script|/script|!--)")` matched at `s[i]`
/// (which must be `<`): the end of the match.
fn match_special_script_tag(s: &[u8], i: usize) -> Option<usize> {
    match_fold(s, i + 1, b"script")
        .or_else(|| match_fold(s, i + 1, b"/script"))
        .or_else(|| match_fold(s, i + 1, b"!--"))
}

// Go: escape.go:containsSpecialScriptTag
/// `specialScriptTagRE.Match(s)`.
pub(crate) fn contains_special_script_tag(s: &[u8]) -> bool {
    let mut i = 0;
    while i < s.len() {
        let idx = bytes::index_byte(&s[i..], b'<');
        if idx < 0 {
            return false;
        }
        let at = i + idx as usize;
        if match_special_script_tag(s, at).is_some() {
            return true;
        }
        i = at + 1;
    }
    false
}

// Go: escape.go:escapeSpecialScriptTags
/// `specialScriptTagRE.ReplaceAll(s, []byte("\\x3C$1"))`.
pub(crate) fn escape_special_script_tags(s: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(s.len());
    let mut last = 0;
    let mut i = 0;
    while i < s.len() {
        let idx = bytes::index_byte(&s[i..], b'<');
        if idx < 0 {
            break;
        }
        let at = i + idx as usize;
        if let Some(end) = match_special_script_tag(s, at) {
            out.extend_from_slice(&s[last..at]);
            out.extend_from_slice(b"\\x3C");
            out.extend_from_slice(&s[at + 1..end]);
            last = end;
            i = end;
        } else {
            i = at + 1;
        }
    }
    out.extend_from_slice(&s[last..]);
    out
}

// Go: js.go:scriptTagRe
/// `scriptTagRe.ReplaceAll(s, []byte(`\x3C${1}script`))` with
/// `scriptTagRe = regexp.MustCompile("(?i)<(/?)script")`.
pub(crate) fn replace_script_tag_re(s: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(s.len());
    let mut last = 0;
    let mut i = 0;
    while i < s.len() {
        let idx = bytes::index_byte(&s[i..], b'<');
        if idx < 0 {
            break;
        }
        let at = i + idx as usize;
        // `/?` is greedy: try with the slash first.
        let m = match match_fold(s, at + 1, b"/script") {
            Some(end) => Some((end, true)),
            None => match_fold(s, at + 1, b"script").map(|end| (end, false)),
        };
        if let Some((end, slash)) = m {
            out.extend_from_slice(&s[last..at]);
            out.extend_from_slice(b"\\x3C");
            if slash {
                out.push(b'/');
            }
            out.extend_from_slice(b"script");
            last = end;
            i = end;
        } else {
            i = at + 1;
        }
    }
    out.extend_from_slice(&s[last..]);
    out
}

// Go: js.go:jsValEscaper
/// jsValEscaper escapes its inputs to a JS Expression (section 11.14) that
/// has neither side-effects nor free variables outside (NaN, Infinity).
pub(crate) fn js_val_escaper(args: &[Value]) -> Vec<u8> {
    let a: Value;
    if args.len() == 1 {
        let mut x = indirect_to_json_marshaler(&args[0]);
        match &x {
            Value::Safe(SafeKind::Js, t) => return t.to_vec(),
            Value::Safe(SafeKind::JsStr, t) => {
                // TODO: normalize quotes.
                let mut out = Vec::with_capacity(t.len() + 2);
                out.push(b'"');
                out.extend_from_slice(t);
                out.push(b'"');
                return out;
            }
            _ if is_json_marshaler(&x) => {
                // Do not treat as a Stringer.
            }
            _ => {
                if let Some(t) = stringer_string(&x) {
                    x = Value::String(t);
                }
            }
        }
        a = x;
    } else {
        let args: Vec<Value> = args.iter().map(indirect_to_json_marshaler).collect();
        a = Value::String(go_fmt::sprint(&args).into());
    }
    // TODO: detect cycles before calling Marshal which loops infinitely on
    // cyclic data. This may be an unacceptable DoS risk.
    let b = match go_json::marshal(&a) {
        Ok(b) => b,
        Err(err) => {
            // While the standard JSON marshaler does not include user controlled
            // information in the error message, if a type has a MarshalJSON method,
            // the content of the error message is not guaranteed. Since we insert
            // the error into the template, as part of a comment, we attempt to
            // prevent the error from either terminating the comment, or the script
            // block itself.
            //
            // In particular we:
            //   * replace "*/" comment end tokens with "* /", which does not
            //     terminate the comment
            //   * replace "<script" and "</script" with "\x3Cscript" and "\x3C/script"
            //     (case insensitively), and "<!--" with "\x3C!--", which prevents
            //     confusing script block termination semantics
            //
            // We also put a space before the comment so that if it is flush against
            // a division operator it is not turned into a line comment:
            //     x/{{y}}
            // turning into
            //     x//* error marshaling y:
            //          second line of error message */null
            let err_str = err.error();
            let err_str = replace_script_tag_re(err_str.as_bytes());
            let err_str = strings::replace_all(&err_str, b"*/", b"* /").into_owned();
            let err_str = strings::replace_all(&err_str, b"<!--", b"\\x3C!--").into_owned();
            let mut out = b" /* ".to_vec();
            out.extend_from_slice(&err_str);
            out.extend_from_slice(b" */null ");
            return out;
        }
    };

    // TODO: maybe post-process output to prevent it from containing
    // "<!--", "-->", "<![CDATA[", "]]>", or "</script"
    // in case custom marshalers produce output containing those.
    // Note: Do not use \x escaping to save bytes because it is not JSON compatible and this escaper
    // supports ld+json content-type.
    if b.is_empty() {
        // In, `x=y/{{.}}*z` a json.Marshaler that produces "" should
        // not cause the output `x=y/*z`.
        return b" null ".to_vec();
    }
    let (first, _) = utf8::decode_rune(&b);
    let (last, _) = utf8::decode_last_rune(&b);
    let mut buf: Vec<u8> = Vec::new();
    // Prevent IdentifierNames and NumericLiterals from running into
    // keywords: in, instanceof, typeof, void
    let pad = is_js_ident_part(first) || is_js_ident_part(last);
    if pad {
        buf.push(b' ');
    }
    let mut written = 0usize;
    // Make sure that json.Marshal escapes codepoints U+2028 & U+2029
    // so it falls within the subset of JSON which is valid JS.
    let mut i = 0usize;
    while i < b.len() {
        let (rune, n) = utf8::decode_rune(&b[i..]);
        let repl: &[u8] = if rune == 0x2028 {
            b"\\u2028"
        } else if rune == 0x2029 {
            b"\\u2029"
        } else {
            b""
        };
        if !repl.is_empty() {
            buf.extend_from_slice(&b[written..i]);
            buf.extend_from_slice(repl);
            written = i + n;
        }
        i += n;
    }
    if !buf.is_empty() {
        buf.extend_from_slice(&b[written..]);
        if pad {
            buf.push(b' ');
        }
        return buf;
    }
    b
}

// Go: js.go:jsStrEscaper
/// jsStrEscaper produces a string that can be included between quotes in
/// JavaScript source, in JavaScript embedded in an HTML5 `<script>` element,
/// or in an HTML5 event handler attribute such as onclick.
pub(crate) fn js_str_escaper(args: &[Value]) -> Vec<u8> {
    let (s, t) = stringify(args);
    if t == ContentType::JsStr {
        return replace(&s, js_str_norm_replacement_table);
    }
    replace(&s, js_str_replacement_table)
}

// Go: js.go:jsTmplLitEscaper
pub(crate) fn js_tmpl_lit_escaper(args: &[Value]) -> Vec<u8> {
    let (s, _) = stringify(args);
    replace(&s, js_bq_str_replacement_table)
}

// Go: js.go:jsRegexpEscaper
/// jsRegexpEscaper behaves like jsStrEscaper but escapes regular expression
/// specials so the result is treated literally when included in a regular
/// expression literal. /foo{{.X}}bar/ matches the string "foo" followed by
/// the literal text of {{.X}} followed by the string "bar".
pub(crate) fn js_regexp_escaper(args: &[Value]) -> Vec<u8> {
    let (s, _) = stringify(args);
    let s = replace(&s, js_regexp_replacement_table);
    if s.is_empty() {
        // /{{.X}}/ should not produce a line comment when .X == "".
        return b"(?:)".to_vec();
    }
    s
}

/// A Go replacement table (`[]string` indexed by rune).
pub(crate) type JsTable = fn(Rune) -> Option<&'static [u8]>;

// Go: js.go:replace
/// replace replaces each rune r of s with replacementTable[r], provided that
/// r < len(replacementTable). If replacementTable[r] is the empty string then
/// no replacement is made.
/// It also replaces runes U+2028 and U+2029 with the raw strings `\u2028` and
/// `\u2029`.
pub(crate) fn replace(s: &[u8], replacement_table: JsTable) -> Vec<u8> {
    let mut b: Vec<u8> = Vec::new();
    let mut written = 0usize;
    let mut i = 0usize;
    while i < s.len() {
        // See comment in htmlEscaper.
        let (r, w) = utf8::decode_rune(&s[i..]);
        let repl: &[u8] = if let Some(repl) = low_unicode_replacement_table(r) {
            repl
        } else if let Some(repl) = replacement_table(r) {
            repl
        } else if r == 0x2028 {
            b"\\u2028"
        } else if r == 0x2029 {
            b"\\u2029"
        } else {
            i += w;
            continue;
        };
        if written == 0 {
            b.reserve(s.len());
        }
        b.extend_from_slice(&s[written..i]);
        b.extend_from_slice(repl);
        written = i + w;
        i += w;
    }
    if written == 0 {
        return s.to_vec();
    }
    b.extend_from_slice(&s[written..]);
    b
}

// Go: js.go:lowUnicodeReplacementTable
pub(crate) fn low_unicode_replacement_table(r: Rune) -> Option<&'static [u8]> {
    const T: [&[u8]; 0x20] = [
        b"\\u0000", b"\\u0001", b"\\u0002", b"\\u0003", b"\\u0004", b"\\u0005", b"\\u0006",
        b"\\u0007", // '\a'
        b"\\u0008", // '\b'
        b"\\t",     // '\t'
        b"\\n",     // '\n'
        b"\\u000b", // '\v' — "\v" == "v" on IE 6.
        b"\\f",     // '\f'
        b"\\r",     // '\r'
        b"\\u000e", b"\\u000f", b"\\u0010", b"\\u0011", b"\\u0012", b"\\u0013", b"\\u0014",
        b"\\u0015", b"\\u0016", b"\\u0017", b"\\u0018", b"\\u0019", b"\\u001a", b"\\u001b",
        b"\\u001c", b"\\u001d", b"\\u001e", b"\\u001f",
    ];
    if (0..0x20).contains(&r) {
        Some(T[r as usize])
    } else {
        None
    }
}

// Go: js.go:jsStrReplacementTable
pub(crate) fn js_str_replacement_table(r: Rune) -> Option<&'static [u8]> {
    Some(match r {
        0 => b"\\u0000",
        0x09 => b"\\t",
        0x0a => b"\\n",
        0x0b => b"\\u000b", // "\v" == "v" on IE 6.
        0x0c => b"\\f",
        0x0d => b"\\r",
        // Encode HTML specials as hex so the output can be embedded
        // in HTML attributes without further encoding.
        0x22 => b"\\u0022", // '"'
        0x60 => b"\\u0060", // '`'
        0x26 => b"\\u0026", // '&'
        0x27 => b"\\u0027", // '\''
        0x2b => b"\\u002b", // '+'
        0x2f => b"\\/",     // '/'
        0x3c => b"\\u003c", // '<'
        0x3e => b"\\u003e", // '>'
        0x5c => b"\\\\",    // '\\'
        _ => return None,
    })
}

// Go: js.go:jsBqStrReplacementTable
/// jsBqStrReplacementTable is like jsStrReplacementTable except it also
/// contains the special characters for JS template literals: $, {, and }.
pub(crate) fn js_bq_str_replacement_table(r: Rune) -> Option<&'static [u8]> {
    Some(match r {
        0 => b"\\u0000",
        0x09 => b"\\t",
        0x0a => b"\\n",
        0x0b => b"\\u000b", // "\v" == "v" on IE 6.
        0x0c => b"\\f",
        0x0d => b"\\r",
        // Encode HTML specials as hex so the output can be embedded
        // in HTML attributes without further encoding.
        0x22 => b"\\u0022", // '"'
        0x60 => b"\\u0060", // '`'
        0x26 => b"\\u0026", // '&'
        0x27 => b"\\u0027", // '\''
        0x2b => b"\\u002b", // '+'
        0x2f => b"\\/",     // '/'
        0x3c => b"\\u003c", // '<'
        0x3e => b"\\u003e", // '>'
        0x5c => b"\\\\",    // '\\'
        0x24 => b"\\u0024", // '$'
        0x7b => b"\\u007b", // '{'
        0x7d => b"\\u007d", // '}'
        _ => return None,
    })
}

// Go: js.go:jsStrNormReplacementTable
/// jsStrNormReplacementTable is like jsStrReplacementTable but does not
/// overencode existing escapes since this table has no entry for `\`.
pub(crate) fn js_str_norm_replacement_table(r: Rune) -> Option<&'static [u8]> {
    Some(match r {
        0 => b"\\u0000",
        0x09 => b"\\t",
        0x0a => b"\\n",
        0x0b => b"\\u000b", // "\v" == "v" on IE 6.
        0x0c => b"\\f",
        0x0d => b"\\r",
        // Encode HTML specials as hex so the output can be embedded
        // in HTML attributes without further encoding.
        0x22 => b"\\u0022", // '"'
        0x26 => b"\\u0026", // '&'
        0x27 => b"\\u0027", // '\''
        0x60 => b"\\u0060", // '`'
        0x2b => b"\\u002b", // '+'
        0x2f => b"\\/",     // '/'
        0x3c => b"\\u003c", // '<'
        0x3e => b"\\u003e", // '>'
        _ => return None,
    })
}

// Go: js.go:jsRegexpReplacementTable
pub(crate) fn js_regexp_replacement_table(r: Rune) -> Option<&'static [u8]> {
    Some(match r {
        0 => b"\\u0000",
        0x09 => b"\\t",
        0x0a => b"\\n",
        0x0b => b"\\u000b", // "\v" == "v" on IE 6.
        0x0c => b"\\f",
        0x0d => b"\\r",
        // Encode HTML specials as hex so the output can be embedded
        // in HTML attributes without further encoding.
        0x22 => b"\\u0022", // '"'
        0x24 => b"\\$",     // '$'
        0x26 => b"\\u0026", // '&'
        0x27 => b"\\u0027", // '\''
        0x28 => b"\\(",     // '('
        0x29 => b"\\)",     // ')'
        0x2a => b"\\*",     // '*'
        0x2b => b"\\u002b", // '+'
        0x2d => b"\\-",     // '-'
        0x2e => b"\\.",     // '.'
        0x2f => b"\\/",     // '/'
        0x3c => b"\\u003c", // '<'
        0x3e => b"\\u003e", // '>'
        0x3f => b"\\?",     // '?'
        0x5b => b"\\[",     // '['
        0x5c => b"\\\\",    // '\\'
        0x5d => b"\\]",     // ']'
        0x5e => b"\\^",     // '^'
        0x7b => b"\\{",     // '{'
        0x7c => b"\\|",     // '|'
        0x7d => b"\\}",     // '}'
        _ => return None,
    })
}

// Go: js.go:isJSIdentPart
/// isJSIdentPart reports whether the given rune is a JS identifier part.
/// It does not handle all the non-Latin letters, joiners, and combining
/// marks, but it does handle every codepoint that can occur in a numeric
/// literal or a keyword.
pub(crate) fn is_js_ident_part(r: Rune) -> bool {
    r == '$' as Rune
        || ('0' as Rune <= r && r <= '9' as Rune)
        || ('A' as Rune <= r && r <= 'Z' as Rune)
        || r == '_' as Rune
        || ('a' as Rune <= r && r <= 'z' as Rune)
}

// Go: js.go:isJSType
/// isJSType reports whether the given MIME type should be considered
/// JavaScript.
///
/// It is used to determine whether a script tag with a type attribute is a
/// javascript container. (go1.24: the empty type is not in the list.)
pub(crate) fn is_js_type(mime_type: &[u8]) -> bool {
    // per
    //   https://www.w3.org/TR/html5/scripting-1.html#attr-script-type
    //   https://tools.ietf.org/html/rfc7231#section-3.1.1
    //   https://tools.ietf.org/html/rfc4329#section-3
    //   https://www.ietf.org/rfc/rfc4627.txt
    // discard parameters
    let (mime_type, _, _) = strings::cut(mime_type, b";");
    let mime_type = strings::to_lower(mime_type);
    let mime_type = strings::trim_space(&mime_type);
    matches!(
        mime_type,
        b"application/ecmascript"
            | b"application/javascript"
            | b"application/json"
            | b"application/ld+json"
            | b"application/x-ecmascript"
            | b"application/x-javascript"
            | b"module"
            | b"text/ecmascript"
            | b"text/javascript"
            | b"text/javascript1.0"
            | b"text/javascript1.1"
            | b"text/javascript1.2"
            | b"text/javascript1.3"
            | b"text/javascript1.4"
            | b"text/javascript1.5"
            | b"text/jscript"
            | b"text/livescript"
            | b"text/x-ecmascript"
            | b"text/x-javascript"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The `(?i)` orbits hard-coded in the comment above are Go's
    /// `unicode.SimpleFold` orbits (Unicode 17 tables).
    #[test]
    fn fold_orbits() {
        let orbit = |c: char| {
            let lit = c as Rune;
            let mut v = vec![lit];
            let mut r = go_unicode::simple_fold(lit);
            while r != lit {
                v.push(r);
                r = go_unicode::simple_fold(r);
            }
            v.sort();
            v
        };
        assert_eq!(orbit('s'), vec!['S' as Rune, 's' as Rune, 0x17f]);
        for c in ['c', 'r', 'i', 'p', 't'] {
            assert_eq!(
                orbit(c),
                vec![c.to_ascii_uppercase() as Rune, c as Rune],
                "{c}"
            );
        }
        for c in ['<', '/', '!', '-'] {
            assert_eq!(orbit(c), vec![c as Rune], "{c}");
        }
    }

    #[test]
    fn special_script_tags() {
        assert!(contains_special_script_tag(b"x<ScRiPt"));
        assert!(contains_special_script_tag("<\u{17f}cript".as_bytes()));
        assert!(contains_special_script_tag(b"</SCRIPT"));
        assert!(contains_special_script_tag(b"<!--"));
        assert!(!contains_special_script_tag(b"<scrip"));
        assert!(!contains_special_script_tag(b"< script"));
        assert!(!contains_special_script_tag("<\u{212a}".as_bytes()));
        assert_eq!(
            escape_special_script_tags("a<SCRIPT b</script c<!-- d<\u{17f}cRipt <x".as_bytes()),
            "a\\x3CSCRIPT b\\x3C/script c\\x3C!-- d\\x3C\u{17f}cRipt <x".as_bytes()
        );
        assert_eq!(
            replace_script_tag_re("a <sCrIpT b </SCRIPT c <\u{17f}cript d </x".as_bytes()),
            b"a \\x3Cscript b \\x3C/script c \\x3Cscript d </x"
        );
    }
}
