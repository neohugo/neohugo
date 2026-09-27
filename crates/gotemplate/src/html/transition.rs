//! Go: tpl/internal/go_templates/htmltemplate/transition.go (+ `delimEnds`
//! from escape.go).

use go_unicode::{bytes, strings};
use go_value::Value;

use super::attr::attr_type;
use super::content::ContentType;
use super::context::{
    Attr, Context, Delim, Element, JsCtx, State, UrlPart, is_comment, is_in_script_literal,
};
use super::css::{decode_css, ends_with_css_keyword};
use super::error::{Error, ErrorCode, errorf};
use super::js::next_js_ctx;

/// `fmt.Sprintf("%q", b)` (also `%.Nq` with `prec`) for error texts.
pub(crate) fn fmt_q(b: &[u8], prec: Option<usize>) -> String {
    let format = match prec {
        Some(p) => format!("%.{p}q"),
        None => "%q".to_string(),
    };
    let out = go_fmt::sprintf(format, &[Value::String(b.into())]);
    // strconv.Quote output is always valid UTF-8.
    String::from_utf8_lossy(&out).into_owned()
}

// Go: transition.go:transitionFunc
/// transitionFunc is the array of context transition functions for text
/// nodes. A transition function takes a context and template text input,
/// and returns the updated context and the number of bytes consumed from
/// the front of the input.
///
/// Go's array has no entry for `stateDead` (indexing it would panic); the
/// escaper never transitions a dead context (`escapeList` stops at
/// `stateDead`), so that case consumes the input like `tError`.
pub(crate) fn transition(c: Context, s: &[u8]) -> (Context, usize) {
    match c.state {
        State::Text => t_text(c, s),
        State::Tag => t_tag(c, s),
        State::AttrName => t_attr_name(c, s),
        State::AfterName => t_after_name(c, s),
        State::BeforeValue => t_before_value(c, s),
        State::HtmlCmt => t_html_cmt(c, s),
        State::Rcdata => t_special_tag_end(c, s),
        State::Attr => t_attr(c, s),
        State::Url => t_url(c, s),
        State::Srcset => t_url(c, s),
        State::Js => t_js(c, s),
        State::JsDqStr => t_js_delimited(c, s),
        State::JsSqStr => t_js_delimited(c, s),
        State::JsRegexp => t_js_delimited(c, s),
        State::JsTmplLit => t_js_tmpl(c, s),
        State::JsBlockCmt => t_block_cmt(c, s),
        State::JsLineCmt => t_line_cmt(c, s),
        State::JsHtmlOpenCmt => t_line_cmt(c, s),
        State::JsHtmlCloseCmt => t_line_cmt(c, s),
        State::Css => t_css(c, s),
        State::CssDqStr => t_css_str(c, s),
        State::CssSqStr => t_css_str(c, s),
        State::CssDqUrl => t_css_str(c, s),
        State::CssSqUrl => t_css_str(c, s),
        State::CssUrl => t_css_str(c, s),
        State::CssBlockCmt => t_block_cmt(c, s),
        State::CssLineCmt => t_line_cmt(c, s),
        State::Error => t_error(c, s),
        State::Dead => t_error(c, s),
    }
}

// Go: transition.go:commentStart, commentEnd
const COMMENT_START: &[u8] = b"<!--";
const COMMENT_END: &[u8] = b"-->";

// Go: escape.go:delimEnds
/// delimEnds maps each delim to a string of characters that terminate it.
pub(crate) fn delim_ends(d: Delim) -> &'static [u8] {
    match d {
        Delim::None => b"",
        Delim::DoubleQuote => b"\"",
        Delim::SingleQuote => b"'",
        // Determined empirically by running the below in various browsers.
        // var div = document.createElement("DIV");
        // for (var i = 0; i < 0x10000; ++i) {
        //   div.innerHTML = "<span title=x" + String.fromCharCode(i) + "-bar>";
        //   if (div.getElementsByTagName("SPAN")[0].title.indexOf("bar") < 0)
        //     document.write("<p>U+" + i.toString(16));
        // }
        Delim::SpaceOrTagEnd => b" \t\n\x0c\r>",
    }
}

fn error_ctx(e: Error) -> Context {
    Context::error(e)
}

// Go: transition.go:tText
/// tText is the context transition function for the text state.
pub(crate) fn t_text(c: Context, s: &[u8]) -> (Context, usize) {
    let mut k = 0usize;
    loop {
        let idx = bytes::index_byte(&s[k..], b'<');
        if idx < 0 || k + idx as usize + 1 == s.len() {
            return (c, s.len());
        }
        let mut i = k + idx as usize;
        if i + 4 <= s.len() && &s[i..i + 4] == COMMENT_START {
            return (Context::with_state(State::HtmlCmt), i + 4);
        }
        i += 1;
        let mut end = false;
        if s[i] == b'/' {
            if i + 1 == s.len() {
                return (c, s.len());
            }
            end = true;
            i += 1;
        }
        let (j, mut e) = eat_tag_name(s, i);
        if j != i {
            if end {
                e = Element::None;
            }
            // We've found an HTML tag.
            return (
                Context {
                    state: State::Tag,
                    element: e,
                    ..Context::default()
                },
                j,
            );
        }
        k = j;
    }
}

// Go: transition.go:elementContentType
pub(crate) fn element_content_type(e: Element) -> State {
    match e {
        Element::None => State::Text,
        Element::Script => State::Js,
        Element::Style => State::Css,
        Element::Textarea => State::Rcdata,
        Element::Title => State::Rcdata,
    }
}

// Go: transition.go:tTag
/// tTag is the context transition function for the tag state.
pub(crate) fn t_tag(c: Context, s: &[u8]) -> (Context, usize) {
    // Find the attribute name.
    let i = eat_white_space(s, 0);
    if i == s.len() {
        return (c, s.len());
    }
    if s[i] == b'>' {
        return (
            Context {
                state: element_content_type(c.element),
                element: c.element,
                ..Context::default()
            },
            i + 1,
        );
    }
    let j = match eat_attr_name(s, i) {
        Ok(j) => j,
        Err(err) => return (error_ctx(err), s.len()),
    };
    let mut attr = Attr::None;
    if i == j {
        return (
            error_ctx(errorf(
                ErrorCode::ErrBadHTML,
                None,
                0,
                format!(
                    "expected space, attr name, or end of tag, but got {}",
                    fmt_q(&s[i..], None)
                ),
            )),
            s.len(),
        );
    }

    let attr_name = strings::to_lower(&s[i..j]);
    if c.element == Element::Script && &*attr_name == b"type" {
        attr = Attr::ScriptType;
    } else {
        match attr_type(&attr_name) {
            ContentType::Url => attr = Attr::Url,
            ContentType::Css => attr = Attr::Style,
            ContentType::Js => attr = Attr::Script,
            ContentType::Srcset => attr = Attr::Srcset,
            _ => {}
        }
    }

    let state = if j == s.len() {
        State::AttrName
    } else {
        State::AfterName
    };
    (
        Context {
            state,
            element: c.element,
            attr,
            ..Context::default()
        },
        j,
    )
}

// Go: transition.go:tAttrName
/// tAttrName is the context transition function for stateAttrName.
pub(crate) fn t_attr_name(mut c: Context, s: &[u8]) -> (Context, usize) {
    match eat_attr_name(s, 0) {
        Err(err) => (error_ctx(err), s.len()),
        Ok(i) => {
            if i != s.len() {
                c.state = State::AfterName;
            }
            (c, i)
        }
    }
}

// Go: transition.go:tAfterName
/// tAfterName is the context transition function for stateAfterName.
pub(crate) fn t_after_name(mut c: Context, s: &[u8]) -> (Context, usize) {
    // Look for the start of the value.
    let i = eat_white_space(s, 0);
    if i == s.len() {
        return (c, s.len());
    } else if s[i] != b'=' {
        // Occurs due to tag ending '>', and valueless attribute.
        c.state = State::Tag;
        return (c, i);
    }
    c.state = State::BeforeValue;
    // Consume the "=".
    (c, i + 1)
}

// Go: transition.go:attrStartStates
pub(crate) fn attr_start_state(a: Attr) -> State {
    match a {
        Attr::None => State::Attr,
        Attr::Script => State::Js,
        Attr::ScriptType => State::Attr,
        Attr::Style => State::Css,
        Attr::Url => State::Url,
        Attr::Srcset => State::Srcset,
    }
}

// Go: transition.go:tBeforeValue
/// tBeforeValue is the context transition function for stateBeforeValue.
pub(crate) fn t_before_value(mut c: Context, s: &[u8]) -> (Context, usize) {
    let mut i = eat_white_space(s, 0);
    if i == s.len() {
        return (c, s.len());
    }
    // Find the attribute delimiter.
    let mut delim = Delim::SpaceOrTagEnd;
    match s[i] {
        b'\'' => {
            delim = Delim::SingleQuote;
            i += 1;
        }
        b'"' => {
            delim = Delim::DoubleQuote;
            i += 1;
        }
        _ => {}
    }
    c.state = attr_start_state(c.attr);
    c.delim = delim;
    (c, i)
}

// Go: transition.go:tHTMLCmt
/// tHTMLCmt is the context transition function for stateHTMLCmt.
pub(crate) fn t_html_cmt(c: Context, s: &[u8]) -> (Context, usize) {
    let i = bytes::index(s, COMMENT_END);
    if i != -1 {
        return (Context::default(), i as usize + 3);
    }
    (c, s.len())
}

// Go: transition.go:specialTagEndMarkers
/// specialTagEndMarkers maps element types to the character sequence that
/// case-insensitively signals the end of the special tag body.
pub(crate) fn special_tag_end_marker(e: Element) -> &'static [u8] {
    match e {
        Element::None => b"",
        Element::Script => b"script",
        Element::Style => b"style",
        Element::Textarea => b"textarea",
        Element::Title => b"title",
    }
}

// Go: transition.go:specialTagEndPrefix, tagEndSeparators
const SPECIAL_TAG_END_PREFIX: &[u8] = b"</";
const TAG_END_SEPARATORS: &[u8] = b"> \t\n\x0c/";

// Go: transition.go:tSpecialTagEnd
/// tSpecialTagEnd is the context transition function for raw text and
/// RCDATA element states.
pub(crate) fn t_special_tag_end(c: Context, s: &[u8]) -> (Context, usize) {
    if c.element != Element::None {
        // script end tags ("</script") within script literals are ignored, so that
        // we can properly escape them.
        if c.element == Element::Script && (is_in_script_literal(c.state) || is_comment(c.state)) {
            return (c, s.len());
        }
        let i = index_tag_end(s, special_tag_end_marker(c.element));
        if i != -1 {
            return (Context::default(), i as usize);
        }
    }
    (c, s.len())
}

// Go: transition.go:indexTagEnd
/// indexTagEnd finds the index of a special tag end in a case insensitive
/// way, or returns -1.
pub(crate) fn index_tag_end(s: &[u8], tag: &[u8]) -> isize {
    let mut s = s;
    let mut res: isize = 0;
    let plen = SPECIAL_TAG_END_PREFIX.len() as isize;
    while !s.is_empty() {
        // Try to find the tag end prefix first
        let i = bytes::index(s, SPECIAL_TAG_END_PREFIX);
        if i == -1 {
            return i;
        }
        s = &s[(i + plen) as usize..];
        // Try to match the actual tag if there is still space for it
        if tag.len() <= s.len() && bytes::equal_fold(tag, &s[..tag.len()]) {
            s = &s[tag.len()..];
            // Check the tag is followed by a proper separator
            if !s.is_empty() && bytes::index_byte(TAG_END_SEPARATORS, s[0]) != -1 {
                return res + i;
            }
            res += tag.len() as isize;
        }
        res += i + plen;
    }
    -1
}

// Go: transition.go:tAttr
/// tAttr is the context transition function for the attribute state.
pub(crate) fn t_attr(c: Context, s: &[u8]) -> (Context, usize) {
    (c, s.len())
}

// Go: transition.go:tURL
/// tURL is the context transition function for the URL state.
pub(crate) fn t_url(mut c: Context, s: &[u8]) -> (Context, usize) {
    if bytes::contains_any(s, b"#?") {
        c.url_part = UrlPart::QueryOrFrag;
    } else if s.len() != eat_white_space(s, 0) && c.url_part == UrlPart::None {
        // HTML5 uses "Valid URL potentially surrounded by spaces" for
        // attrs: https://www.w3.org/TR/html5/index.html#attributes-1
        c.url_part = UrlPart::PreQuery;
    }
    (c, s.len())
}

// Go: transition.go:tJS
/// tJS is the context transition function for the JS state.
pub(crate) fn t_js(mut c: Context, s: &[u8]) -> (Context, usize) {
    let i = bytes::index_any(s, b"\"`'/{}<-#");
    if i == -1 {
        // Entire input is non string, comment, regexp tokens.
        c.js_ctx = next_js_ctx(s, c.js_ctx);
        return (c, s.len());
    }
    let mut i = i as usize;
    c.js_ctx = next_js_ctx(&s[..i], c.js_ctx);
    match s[i] {
        b'"' => {
            c.state = State::JsDqStr;
            c.js_ctx = JsCtx::Regexp;
        }
        b'\'' => {
            c.state = State::JsSqStr;
            c.js_ctx = JsCtx::Regexp;
        }
        b'`' => {
            c.state = State::JsTmplLit;
            c.js_ctx = JsCtx::Regexp;
        }
        b'/' => {
            if i + 1 < s.len() && s[i + 1] == b'/' {
                c.state = State::JsLineCmt;
                i += 1;
            } else if i + 1 < s.len() && s[i + 1] == b'*' {
                c.state = State::JsBlockCmt;
                i += 1;
            } else if c.js_ctx == JsCtx::Regexp {
                c.state = State::JsRegexp;
            } else if c.js_ctx == JsCtx::DivOp {
                c.js_ctx = JsCtx::Regexp;
            } else {
                return (
                    error_ctx(errorf(
                        ErrorCode::ErrSlashAmbig,
                        None,
                        0,
                        format!(
                            "'/' could start a division or regexp: {}",
                            fmt_q(&s[i..], Some(32))
                        ),
                    )),
                    s.len(),
                );
            }
        }
        // ECMAScript supports HTML style comments for legacy reasons, see Appendix
        // B.1.1 "HTML-like Comments". The handling of these comments is somewhat
        // confusing. Multi-line comments are not supported, i.e. anything on lines
        // between the opening and closing tokens is not considered a comment, but
        // anything following the opening or closing token, on the same line, is
        // ignored. As such we simply treat any line prefixed with "<!--" or "-->"
        // as if it were actually prefixed with "//" and move on.
        b'<' => {
            if i + 3 < s.len() && &s[i..i + 4] == COMMENT_START {
                c.state = State::JsHtmlOpenCmt;
                i += 3;
            }
        }
        b'-' => {
            if i + 2 < s.len() && &s[i..i + 3] == COMMENT_END {
                c.state = State::JsHtmlCloseCmt;
                i += 2;
            }
        }
        // ECMAScript also supports "hashbang" comment lines, see Section 12.5.
        b'#' => {
            if i + 1 < s.len() && s[i + 1] == b'!' {
                c.state = State::JsLineCmt;
                i += 1;
            }
        }
        b'{' => {
            // We only care about tracking brace depth if we are inside of a
            // template literal.
            if c.js_brace_depth.is_empty() {
                return (c, i + 1);
            }
            let last = c.js_brace_depth.len() - 1;
            c.js_brace_depth
                .set(last, c.js_brace_depth.get(last).wrapping_add(1));
        }
        b'}' => {
            if c.js_brace_depth.is_empty() {
                return (c, i + 1);
            }
            // There are no cases where a brace can be escaped in the JS context
            // that are not syntax errors, it seems. Because of this we can just
            // count "\}" as "}" and move on, the script is already broken as
            // fully fledged parsers will just fail anyway.
            let last = c.js_brace_depth.len() - 1;
            c.js_brace_depth
                .set(last, c.js_brace_depth.get(last).wrapping_sub(1));
            if c.js_brace_depth.get(last) >= 0 {
                return (c, i + 1);
            }
            c.js_brace_depth = c.js_brace_depth.reslice(last);
            c.state = State::JsTmplLit;
        }
        _ => unreachable!("index_any only matches the listed bytes"),
    }
    (c, i + 1)
}

// Go: transition.go:tJSTmpl
pub(crate) fn t_js_tmpl(mut c: Context, s: &[u8]) -> (Context, usize) {
    let mut k = 0usize;
    loop {
        let idx = bytes::index_any(&s[k..], b"`\\$");
        if idx < 0 {
            break;
        }
        let mut i = k + idx as usize;
        match s[i] {
            b'\\' => {
                i += 1;
                if i == s.len() {
                    return (
                        error_ctx(errorf(
                            ErrorCode::ErrPartialEscape,
                            None,
                            0,
                            format!(
                                "unfinished escape sequence in JS string: {}",
                                fmt_q(s, None)
                            ),
                        )),
                        s.len(),
                    );
                }
            }
            b'$' => {
                if s.len() >= i + 2 && s[i + 1] == b'{' {
                    c.js_brace_depth = c.js_brace_depth.append(0);
                    c.state = State::Js;
                    return (c, i + 2);
                }
            }
            b'`' => {
                // end
                c.state = State::Js;
                return (c, i + 1);
            }
            _ => {}
        }
        k = i + 1;
    }

    (c, s.len())
}

// Go: transition.go:tJSDelimited
/// tJSDelimited is the context transition function for the JS string and
/// regexp states.
pub(crate) fn t_js_delimited(mut c: Context, s: &[u8]) -> (Context, usize) {
    let specials: &[u8] = match c.state {
        State::JsSqStr => b"\\'",
        State::JsRegexp => b"\\/[]",
        _ => b"\\\"",
    };

    let mut k = 0usize;
    let mut in_charset = false;
    loop {
        let idx = bytes::index_any(&s[k..], specials);
        if idx < 0 {
            break;
        }
        let mut i = k + idx as usize;
        match s[i] {
            b'\\' => {
                i += 1;
                if i == s.len() {
                    return (
                        error_ctx(errorf(
                            ErrorCode::ErrPartialEscape,
                            None,
                            0,
                            format!(
                                "unfinished escape sequence in JS string: {}",
                                fmt_q(s, None)
                            ),
                        )),
                        s.len(),
                    );
                }
            }
            b'[' => in_charset = true,
            b']' => in_charset = false,
            b'/' => {
                // If "</script" appears in a regex literal, the '/' should not
                // close the regex literal, and it will later be escaped to
                // "\x3C/script" in escapeText.
                if i > 0 && i + 7 <= s.len() && bytes::to_lower(&s[i - 1..i + 7]) == b"</script" {
                    i += 1;
                } else if !in_charset {
                    c.state = State::Js;
                    c.js_ctx = JsCtx::DivOp;
                    return (c, i + 1);
                }
            }
            _ => {
                // end delimiter
                if !in_charset {
                    c.state = State::Js;
                    c.js_ctx = JsCtx::DivOp;
                    return (c, i + 1);
                }
            }
        }
        k = i + 1;
    }

    if in_charset {
        // This can be fixed by making context richer if interpolation
        // into charsets is desired.
        return (
            error_ctx(errorf(
                ErrorCode::ErrPartialCharset,
                None,
                0,
                format!("unfinished JS regexp charset: {}", fmt_q(s, None)),
            )),
            s.len(),
        );
    }

    (c, s.len())
}

// Go: transition.go:blockCommentEnd
const BLOCK_COMMENT_END: &[u8] = b"*/";

// Go: transition.go:tBlockCmt
/// tBlockCmt is the context transition function for /*comment*/ states.
pub(crate) fn t_block_cmt(mut c: Context, s: &[u8]) -> (Context, usize) {
    let i = bytes::index(s, BLOCK_COMMENT_END);
    if i == -1 {
        return (c, s.len());
    }
    match c.state {
        State::JsBlockCmt => c.state = State::Js,
        State::CssBlockCmt => c.state = State::Css,
        // Go: panic(c.state.String()); only reachable through transition().
        st => panic!("{}", st.string()),
    }
    (c, i as usize + 2)
}

// Go: transition.go:tLineCmt
/// tLineCmt is the context transition function for //comment states, and
/// the JS HTML-like comment state.
pub(crate) fn t_line_cmt(mut c: Context, s: &[u8]) -> (Context, usize) {
    let line_terminators: &[u8];
    let end_state: State;
    match c.state {
        State::JsLineCmt | State::JsHtmlOpenCmt | State::JsHtmlCloseCmt => {
            line_terminators = "\n\r\u{2028}\u{2029}".as_bytes();
            end_state = State::Js;
        }
        State::CssLineCmt => {
            line_terminators = b"\n\x0c\r";
            end_state = State::Css;
            // Line comments are not part of any published CSS standard but
            // are supported by the 4 major browsers.
            // This defines line comments as
            //     LINECOMMENT ::= "//" [^\n\f\d]*
            // since https://www.w3.org/TR/css3-syntax/#SUBTOK-nl defines
            // newlines:
            //     nl ::= #xA | #xD #xA | #xD | #xC
        }
        // Go: panic(c.state.String()); only reachable through transition().
        st => panic!("{}", st.string()),
    }

    let i = bytes::index_any(s, line_terminators);
    if i == -1 {
        return (c, s.len());
    }
    c.state = end_state;
    // Per section 7.4 of EcmaScript 5 : https://es5.github.io/#x7.4
    // "However, the LineTerminator at the end of the line is not
    // considered to be part of the single-line comment; it is
    // recognized separately by the lexical grammar and becomes part
    // of the stream of input elements for the syntactic grammar."
    (c, i as usize)
}

// Go: transition.go:tCSS
/// tCSS is the context transition function for the CSS state.
pub(crate) fn t_css(mut c: Context, s: &[u8]) -> (Context, usize) {
    // CSS quoted strings are almost never used except for:
    // (1) URLs as in background: "/foo.png"
    // (2) Multiword font-names as in font-family: "Times New Roman"
    // (3) List separators in content values as in inline-lists:
    //    <style>
    //    ul.inlineList { list-style: none; padding:0 }
    //    ul.inlineList > li { display: inline }
    //    ul.inlineList > li:before { content: ", " }
    //    ul.inlineList > li:first-child:before { content: "" }
    //    </style>
    //    <ul class=inlineList><li>One<li>Two<li>Three</ul>
    // (4) Attribute value selectors as in a[href="http://example.com/"]
    //
    // We conservatively treat all strings as URLs, but make some
    // allowances to avoid confusion.
    //
    // In (1), our conservative assumption is justified.
    // In (2), valid font names do not contain ':', '?', or '#', so our
    // conservative assumption is fine since we will never transition past
    // urlPartPreQuery.
    // In (3), our protocol heuristic should not be tripped, and there
    // should not be non-space content after a '?' or '#', so as long as
    // we only %-encode RFC 3986 reserved characters we are ok.
    // In (4), we should URL escape for URL attributes, and for others we
    // have the attribute name available if our conservative assumption
    // proves problematic for real code.

    let mut k = 0usize;
    loop {
        let idx = bytes::index_any(&s[k..], b"(\"'/");
        if idx < 0 {
            return (c, s.len());
        }
        let i = k + idx as usize;
        match s[i] {
            b'(' => {
                // Look for url to the left.
                let p = bytes::trim_right(&s[..i], b"\t\n\x0c\r ");
                if ends_with_css_keyword(p, b"url") {
                    let mut j = s.len() - bytes::trim_left(&s[i + 1..], b"\t\n\x0c\r ").len();
                    if j != s.len() && s[j] == b'"' {
                        c.state = State::CssDqUrl;
                        j += 1;
                    } else if j != s.len() && s[j] == b'\'' {
                        c.state = State::CssSqUrl;
                        j += 1;
                    } else {
                        c.state = State::CssUrl;
                    }
                    return (c, j);
                }
            }
            b'/' => {
                if i + 1 < s.len() {
                    match s[i + 1] {
                        b'/' => {
                            c.state = State::CssLineCmt;
                            return (c, i + 2);
                        }
                        b'*' => {
                            c.state = State::CssBlockCmt;
                            return (c, i + 2);
                        }
                        _ => {}
                    }
                }
            }
            b'"' => {
                c.state = State::CssDqStr;
                return (c, i + 1);
            }
            b'\'' => {
                c.state = State::CssSqStr;
                return (c, i + 1);
            }
            _ => {}
        }
        k = i + 1;
    }
}

// Go: transition.go:tCSSStr
/// tCSSStr is the context transition function for the CSS string and URL
/// states.
pub(crate) fn t_css_str(mut c: Context, s: &[u8]) -> (Context, usize) {
    let end_and_esc: &[u8] = match c.state {
        State::CssDqStr | State::CssDqUrl => b"\\\"",
        State::CssSqStr | State::CssSqUrl => b"\\'",
        // Unquoted URLs end with a newline or close parenthesis.
        // The below includes the wc (whitespace character) and nl.
        State::CssUrl => b"\\\t\n\x0c\r )",
        // Go: panic(c.state.String()); only reachable through transition().
        st => panic!("{}", st.string()),
    };

    let mut k = 0usize;
    loop {
        let idx = bytes::index_any(&s[k..], end_and_esc);
        if idx < 0 {
            let (c, nread) = t_url(c, &decode_css(&s[k..]));
            return (c, k + nread);
        }
        let mut i = k + idx as usize;
        if s[i] == b'\\' {
            i += 1;
            if i == s.len() {
                return (
                    error_ctx(errorf(
                        ErrorCode::ErrPartialEscape,
                        None,
                        0,
                        format!(
                            "unfinished escape sequence in CSS string: {}",
                            fmt_q(s, None)
                        ),
                    )),
                    s.len(),
                );
            }
        } else {
            c.state = State::Css;
            return (c, i + 1);
        }
        c = t_url(c, &decode_css(&s[..i + 1])).0;
        k = i + 1;
    }
}

// Go: transition.go:tError
/// tError is the context transition function for the error state.
pub(crate) fn t_error(c: Context, s: &[u8]) -> (Context, usize) {
    (c, s.len())
}

// Go: transition.go:eatAttrName
/// eatAttrName returns the largest j such that s[i:j] is an attribute name.
/// It returns an error if s[i:] does not look like it begins with an
/// attribute name, such as encountering a quote mark without a preceding
/// equals sign.
pub(crate) fn eat_attr_name(s: &[u8], i: usize) -> Result<usize, Error> {
    for j in i..s.len() {
        match s[j] {
            b' ' | b'\t' | b'\n' | b'\x0c' | b'\r' | b'=' | b'>' => return Ok(j),
            b'\'' | b'"' | b'<' => {
                // These result in a parse warning in HTML5 and are
                // indicative of serious problems if seen in an attr
                // name in a template.
                return Err(errorf(
                    ErrorCode::ErrBadHTML,
                    None,
                    0,
                    format!(
                        "{} in attribute name: {}",
                        fmt_q(&s[j..j + 1], None),
                        fmt_q(s, Some(32))
                    ),
                ));
            }
            _ => {
                // No-op.
            }
        }
    }
    Ok(s.len())
}

// Go: transition.go:elementNameMap
fn element_name_map(name: &[u8]) -> Element {
    match name {
        b"script" => Element::Script,
        b"style" => Element::Style,
        b"textarea" => Element::Textarea,
        b"title" => Element::Title,
        _ => Element::None,
    }
}

// Go: transition.go:asciiAlpha
/// asciiAlpha reports whether c is an ASCII letter.
fn ascii_alpha(c: u8) -> bool {
    b'A' <= c && c <= b'Z' || b'a' <= c && c <= b'z'
}

// Go: transition.go:asciiAlphaNum
/// asciiAlphaNum reports whether c is an ASCII letter or digit.
fn ascii_alpha_num(c: u8) -> bool {
    ascii_alpha(c) || b'0' <= c && c <= b'9'
}

// Go: transition.go:eatTagName
/// eatTagName returns the largest j such that s[i:j] is a tag name and the
/// tag type.
pub(crate) fn eat_tag_name(s: &[u8], i: usize) -> (usize, Element) {
    if i == s.len() || !ascii_alpha(s[i]) {
        return (i, Element::None);
    }
    let mut j = i + 1;
    while j < s.len() {
        let x = s[j];
        if ascii_alpha_num(x) {
            j += 1;
            continue;
        }
        // Allow "x-y" or "x:y" but not "x-", "-y", or "x--y".
        if (x == b':' || x == b'-') && j + 1 < s.len() && ascii_alpha_num(s[j + 1]) {
            j += 2;
            continue;
        }
        break;
    }
    (j, element_name_map(&strings::to_lower(&s[i..j])))
}

// Go: transition.go:eatWhiteSpace
/// eatWhiteSpace returns the largest j such that s[i:j] is white space.
pub(crate) fn eat_white_space(s: &[u8], i: usize) -> usize {
    for j in i..s.len() {
        match s[j] {
            b' ' | b'\t' | b'\n' | b'\x0c' | b'\r' => {
                // No-op.
            }
            _ => return j,
        }
    }
    s.len()
}
