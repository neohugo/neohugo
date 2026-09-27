//! Go: tpl/internal/go_templates/htmltemplate/html.go

use go_unicode::{Rune, bytes, strings, utf8};
use go_value::Value;

use super::FILTER_FAILSAFE;
use super::attr::attr_type;
use super::content::{ContentType, stringify};
use super::context::{Context, Delim, Element, State, is_in_tag};
use super::transition::{delim_ends, t_special_tag_end, transition};

// Go: html.go:htmlNospaceEscaper
/// htmlNospaceEscaper escapes for inclusion in unquoted attribute values.
pub(crate) fn html_nospace_escaper(args: &[Value]) -> Vec<u8> {
    let (s, t) = stringify(args);
    if s.is_empty() {
        return FILTER_FAILSAFE.as_bytes().to_vec();
    }
    if t == ContentType::Html {
        return html_replacer(&strip_tags(&s), html_nospace_norm_replacement_table, false);
    }
    html_replacer(&s, html_nospace_replacement_table, false)
}

// Go: html.go:attrEscaper
/// attrEscaper escapes for inclusion in quoted attribute values.
pub(crate) fn attr_escaper(args: &[Value]) -> Vec<u8> {
    let (s, t) = stringify(args);
    if t == ContentType::Html {
        return html_replacer(&strip_tags(&s), html_norm_replacement_table, true);
    }
    html_replacer(&s, html_replacement_table, true)
}

// Go: html.go:rcdataEscaper
/// rcdataEscaper escapes for inclusion in an RCDATA element body.
pub(crate) fn rcdata_escaper(args: &[Value]) -> Vec<u8> {
    let (s, t) = stringify(args);
    if t == ContentType::Html {
        return html_replacer(&s, html_norm_replacement_table, true);
    }
    html_replacer(&s, html_replacement_table, true)
}

// Go: html.go:htmlEscaper
/// htmlEscaper escapes for inclusion in HTML text.
pub(crate) fn html_escaper(args: &[Value]) -> Vec<u8> {
    let (s, t) = stringify(args);
    if t == ContentType::Html {
        return s;
    }
    html_replacer(&s, html_replacement_table, true)
}

/// A Go replacement table (`[]string` indexed by rune): the replacement for
/// `r` when `r < len(table)` and the entry is non-empty.
pub(crate) type ReplacementTable = fn(Rune) -> Option<&'static [u8]>;

// Go: html.go:htmlReplacementTable
/// htmlReplacementTable contains the runes that need to be escaped
/// inside a quoted attribute value or in a text node.
pub(crate) fn html_replacement_table(r: Rune) -> Option<&'static [u8]> {
    Some(match r {
        // https://www.w3.org/TR/html5/syntax.html#attribute-value-(unquoted)-state
        // U+0000 NULL Parse error. Append a U+FFFD REPLACEMENT
        // CHARACTER character to the current attribute's value.
        // "
        // and similarly
        // https://www.w3.org/TR/html5/syntax.html#before-attribute-value-state
        0 => "\u{FFFD}".as_bytes(),
        0x22 => b"&#34;", // '"'
        0x26 => b"&amp;", // '&'
        0x27 => b"&#39;", // '\''
        0x2b => b"&#43;", // '+'
        0x3c => b"&lt;",  // '<'
        0x3e => b"&gt;",  // '>'
        _ => return None,
    })
}

// Go: html.go:htmlNormReplacementTable
/// htmlNormReplacementTable is like htmlReplacementTable but without '&' to
/// avoid over-encoding existing entities.
pub(crate) fn html_norm_replacement_table(r: Rune) -> Option<&'static [u8]> {
    Some(match r {
        0 => "\u{FFFD}".as_bytes(),
        0x22 => b"&#34;", // '"'
        0x27 => b"&#39;", // '\''
        0x2b => b"&#43;", // '+'
        0x3c => b"&lt;",  // '<'
        0x3e => b"&gt;",  // '>'
        _ => return None,
    })
}

// Go: html.go:htmlNospaceReplacementTable
/// htmlNospaceReplacementTable contains the runes that need to be escaped
/// inside an unquoted attribute value.
/// The set of runes escaped is the union of the HTML specials and
/// those determined by running the JS below in browsers:
/// (see html.go).
pub(crate) fn html_nospace_replacement_table(r: Rune) -> Option<&'static [u8]> {
    Some(match r {
        0 => b"&#xfffd;",
        0x09 => b"&#9;",
        0x0a => b"&#10;",
        0x0b => b"&#11;",
        0x0c => b"&#12;",
        0x0d => b"&#13;",
        0x20 => b"&#32;", // ' '
        0x22 => b"&#34;", // '"'
        0x26 => b"&amp;", // '&'
        0x27 => b"&#39;", // '\''
        0x2b => b"&#43;", // '+'
        0x3c => b"&lt;",  // '<'
        0x3d => b"&#61;", // '='
        0x3e => b"&gt;",  // '>'
        // A parse error in the attribute value (unquoted) and
        // before attribute value states.
        // Treated as a quoting character by IE.
        0x60 => b"&#96;", // '`'
        _ => return None,
    })
}

// Go: html.go:htmlNospaceNormReplacementTable
/// htmlNospaceNormReplacementTable is like htmlNospaceReplacementTable but
/// without '&' to avoid over-encoding existing entities.
pub(crate) fn html_nospace_norm_replacement_table(r: Rune) -> Option<&'static [u8]> {
    Some(match r {
        0 => b"&#xfffd;",
        0x09 => b"&#9;",
        0x0a => b"&#10;",
        0x0b => b"&#11;",
        0x0c => b"&#12;",
        0x0d => b"&#13;",
        0x20 => b"&#32;", // ' '
        0x22 => b"&#34;", // '"'
        0x27 => b"&#39;", // '\''
        0x2b => b"&#43;", // '+'
        0x3c => b"&lt;",  // '<'
        0x3d => b"&#61;", // '='
        0x3e => b"&gt;",  // '>'
        // A parse error in the attribute value (unquoted) and
        // before attribute value states.
        // Treated as a quoting character by IE.
        0x60 => b"&#96;", // '`'
        _ => return None,
    })
}

// Go: html.go:htmlReplacer
/// htmlReplacer returns s with runes replaced according to replacementTable
/// and when badRunes is true, certain bad runes are allowed through
/// unescaped.
pub(crate) fn html_replacer(
    s: &[u8],
    replacement_table: ReplacementTable,
    bad_runes: bool,
) -> Vec<u8> {
    let mut written = 0usize;
    let mut b: Vec<u8> = Vec::new();
    let mut i = 0usize;
    while i < s.len() {
        // Cannot use 'for range s' because we need to preserve the width
        // of the runes in the input. If we see a decoding error, the input
        // width will not be utf8.Runelen(r) and we will overrun the buffer.
        let (r, w) = utf8::decode_rune(&s[i..]);
        // Go: `if int(r) < len(replacementTable) { if repl := ...; len(repl) != 0 {...} }`.
        // Every rune with an entry is below len(table), and the runes below
        // len(table) without an entry (all < 0x61) fail the range test of
        // the last branch, so testing the entry first is equivalent.
        if let Some(repl) = replacement_table(r) {
            if written == 0 {
                b.reserve(s.len());
            }
            b.extend_from_slice(&s[written..i]);
            b.extend_from_slice(repl);
            written = i + w;
        } else if bad_runes {
            // No-op.
            // IE does not allow these ranges in unquoted attrs.
        } else if 0xfdd0 <= r && r <= 0xfdef || 0xfff0 <= r && r <= 0xffff {
            if written == 0 {
                b.reserve(s.len());
            }
            // fmt.Fprintf(b, "%s&#x%x;", s[written:i], r)
            b.extend_from_slice(&s[written..i]);
            b.extend_from_slice(format!("&#x{r:x};").as_bytes());
            written = i + w;
        }
        i += w;
    }
    if written == 0 {
        return s.to_vec();
    }
    b.extend_from_slice(&s[written..]);
    b
}

// Go: html.go:stripTags
/// stripTags takes a snippet of HTML and returns only the text content.
/// For example, `<b>&iexcl;Hi!</b> <script>...</script>` -> `&iexcl;Hi! `.
pub(crate) fn strip_tags(html: &[u8]) -> Vec<u8> {
    let mut b: Vec<u8> = Vec::new();
    let s = html;
    let mut c = Context::default();
    let mut i = 0usize;
    let mut all_text = true;
    // Using the transition funcs helps us avoid mangling
    // `<div title="1>2">` or `I <3 Ponies!`.
    while i != s.len() {
        if c.delim == Delim::None {
            let mut st = c.state;
            // Use RCDATA instead of parsing into JS or CSS styles.
            if c.element != Element::None && !is_in_tag(st) {
                st = State::Rcdata;
            }
            let c_state = c.state;
            // Go: transitionFunc[st](c, s[i:]) — the function of state st,
            // called with the unmodified context c. st differs from c.state
            // only when it was replaced by stateRCDATA (tSpecialTagEnd).
            let (d, nread) = if st == State::Rcdata {
                t_special_tag_end(c, &s[i..])
            } else {
                transition(c, &s[i..])
            };
            let i1 = i + nread;
            if c_state == State::Text || c_state == State::Rcdata {
                // Emit text up to the start of the tag or comment.
                let mut j = i1;
                if d.state != c_state {
                    let mut j1 = j as isize - 1;
                    while j1 >= i as isize {
                        if s[j1 as usize] == b'<' {
                            j = j1 as usize;
                            break;
                        }
                        j1 -= 1;
                    }
                }
                b.extend_from_slice(&s[i..j]);
            } else {
                all_text = false;
            }
            c = d;
            i = i1;
            continue;
        }
        let idx = bytes::index_any(&s[i..], delim_ends(c.delim));
        let mut i1 = i as isize + idx;
        if i1 < i as isize {
            break;
        }
        if c.delim != Delim::SpaceOrTagEnd {
            // Consume any quote.
            i1 += 1;
        }
        c = Context {
            state: State::Tag,
            element: c.element,
            ..Context::default()
        };
        i = i1 as usize;
    }
    if all_text {
        return html.to_vec();
    } else if c.state == State::Text || c.state == State::Rcdata {
        b.extend_from_slice(&s[i..]);
    }
    b
}

// Go: html.go:htmlNameFilter
/// htmlNameFilter accepts valid parts of an HTML attribute or tag name or
/// a known-safe HTML attribute.
pub(crate) fn html_name_filter(args: &[Value]) -> Vec<u8> {
    let (s, t) = stringify(args);
    if t == ContentType::HtmlAttr {
        return s;
    }
    if s.is_empty() {
        // Avoid violation of structure preservation.
        // <input checked {{.K}}={{.V}}>.
        // Without this, if .K is empty then .V is the value of
        // checked, but otherwise .V is the value of the attribute
        // named .K.
        return FILTER_FAILSAFE.as_bytes().to_vec();
    }
    let s = strings::to_lower(&s).into_owned();
    if attr_type(&s) != ContentType::Plain {
        // TODO: Split attr and element name part filters so we can recognize known attributes.
        return FILTER_FAILSAFE.as_bytes().to_vec();
    }
    for (_, r) in utf8::runes(&s) {
        if '0' as Rune <= r && r <= '9' as Rune || 'a' as Rune <= r && r <= 'z' as Rune {
            continue;
        }
        return FILTER_FAILSAFE.as_bytes().to_vec();
    }
    s
}

// Go: html.go:commentEscaper
/// commentEscaper returns the empty string regardless of input.
/// Comment content does not correspond to any parsed structure or
/// human-readable content, so the simplest and most secure policy is to drop
/// content interpolated into comments.
/// This approach is equally valid whether or not static comment content is
/// removed from the template.
pub(crate) fn comment_escaper(_args: &[Value]) -> Vec<u8> {
    Vec::new()
}
