//! Go: tpl/internal/go_templates/htmltemplate/url.go

use go_unicode::strings;
use go_value::Value;

use super::FILTER_FAILSAFE;
use super::content::{ContentType, stringify};
use super::css::is_hex;

// Go: url.go:urlFilter
/// urlFilter returns its input unless it contains an unsafe scheme in which
/// case it defangs the entire URL.
///
/// Schemes that cause unintended side effects that are irreversible without
/// user interaction are considered unsafe. For example, clicking on a
/// "javascript:" link can immediately trigger JavaScript code execution.
///
/// This filter conservatively assumes that all schemes other than the
/// following are unsafe:
///   - http:   Navigates to a new website, and may open a new window or tab.
///   - https:  Same as http.
///   - mailto: Opens an email program and starts a new draft.
///
/// To allow URLs containing other schemes to bypass this filter, developers
/// must explicitly indicate that such a URL is expected and safe by
/// encapsulating it in a template.URL value.
pub(crate) fn url_filter(args: &[Value]) -> Vec<u8> {
    let (s, t) = stringify(args);
    if t == ContentType::Url {
        return s;
    }
    if !is_safe_url(&s) {
        let mut out = b"#".to_vec();
        out.extend_from_slice(FILTER_FAILSAFE.as_bytes());
        return out;
    }
    s
}

// Go: url.go:isSafeURL
/// isSafeURL is true if s is a relative URL or if URL has a protocol in
/// (http, https, mailto).
pub(crate) fn is_safe_url(s: &[u8]) -> bool {
    let (protocol, _, ok) = strings::cut(s, b":");
    if ok && !strings::contains(protocol, b"/") {
        if !strings::equal_fold(protocol, b"http")
            && !strings::equal_fold(protocol, b"https")
            && !strings::equal_fold(protocol, b"mailto")
        {
            return false;
        }
    }
    true
}

// Go: url.go:urlEscaper
/// urlEscaper produces an output that can be embedded in a URL query.
/// The output can be embedded in an HTML attribute without further escaping.
pub(crate) fn url_escaper(args: &[Value]) -> Vec<u8> {
    url_processor(false, args)
}

// Go: url.go:urlNormalizer
/// urlNormalizer normalizes URL content so it can be embedded in a
/// quote-delimited string or parenthesis delimited url(...).
/// The normalizer does not encode all HTML specials. Specifically, it does
/// not encode '&' so correct embedding in an HTML attribute requires
/// escaping of '&' to '&amp;'.
pub(crate) fn url_normalizer(args: &[Value]) -> Vec<u8> {
    url_processor(true, args)
}

// Go: url.go:urlProcessor
/// urlProcessor normalizes (when norm is true) or escapes its input to
/// produce a valid hierarchical or opaque URL part.
pub(crate) fn url_processor(mut norm: bool, args: &[Value]) -> Vec<u8> {
    let (s, t) = stringify(args);
    if t == ContentType::Url {
        norm = true;
    }
    let mut b: Vec<u8> = Vec::new();
    if process_url_onto(&s, norm, &mut b) {
        return b;
    }
    s
}

// Go: url.go:processURLOnto
/// processURLOnto appends a normalized URL corresponding to its input to b
/// and reports whether the appended content differs from s.
pub(crate) fn process_url_onto(s: &[u8], norm: bool, b: &mut Vec<u8>) -> bool {
    b.reserve(s.len() + 16);
    let mut written = 0usize;
    // The byte loop below assumes that all URLs use UTF-8 as the
    // content-encoding. This is similar to the URI to IRI encoding scheme
    // defined in section 3.1 of  RFC 3987, and behaves the same as the
    // EcmaScript builtin encodeURIComponent.
    // It should not cause any misencoding of URLs in pages with
    // Content-type: text/html;charset=UTF-8.
    let n = s.len();
    for i in 0..n {
        let c = s[i];
        match c {
            // Single quote and parens are sub-delims in RFC 3986, but we
            // escape them so the output can be embedded in single
            // quoted attributes and unquoted CSS url(...) constructs.
            // Single quotes are reserved in URLs, but are only used in
            // the obsolete "mark" rule in an appendix in RFC 3986
            // so can be safely encoded.
            b'!' | b'#' | b'$' | b'&' | b'*' | b'+' | b',' | b'/' | b':' | b';' | b'=' | b'?'
            | b'@' | b'[' | b']' => {
                if norm {
                    continue;
                }
            }
            // Unreserved according to RFC 3986 sec 2.3
            // "For consistency, percent-encoded octets in the ranges of
            // ALPHA (%41-%5A and %61-%7A), DIGIT (%30-%39), hyphen (%2D),
            // period (%2E), underscore (%5F), or tilde (%7E) should not be
            // created by URI producers
            b'-' | b'.' | b'_' | b'~' => continue,
            b'%' => {
                // When normalizing do not re-encode valid escapes.
                if norm && i + 2 < s.len() && is_hex(s[i + 1]) && is_hex(s[i + 2]) {
                    continue;
                }
            }
            _ => {
                // Unreserved according to RFC 3986 sec 2.3
                if c.is_ascii_lowercase() || c.is_ascii_uppercase() || c.is_ascii_digit() {
                    continue;
                }
            }
        }
        b.extend_from_slice(&s[written..i]);
        // fmt.Fprintf(b, "%%%02x", c)
        b.extend_from_slice(format!("%{c:02x}").as_bytes());
        written = i + 1;
    }
    b.extend_from_slice(&s[written..]);
    written != 0
}

// Go: url.go:srcsetFilterAndEscaper
/// Filters and normalizes srcset values which are comma separated
/// URLs followed by metadata.
pub(crate) fn srcset_filter_and_escaper(args: &[Value]) -> Vec<u8> {
    let (s, t) = stringify(args);
    match t {
        ContentType::Srcset => return s,
        ContentType::Url => {
            // Normalizing gets rid of all HTML whitespace
            // which separate the image URL from its metadata.
            let mut b: Vec<u8> = Vec::new();
            let s = if process_url_onto(&s, true, &mut b) {
                b
            } else {
                s
            };
            // Additionally, commas separate one source from another.
            return strings::replace_all(&s, b",", b"%2c").into_owned();
        }
        _ => {}
    }

    let mut b: Vec<u8> = Vec::new();
    let mut written = 0usize;
    for i in 0..s.len() {
        if s[i] == b',' {
            filter_srcset_element(&s, written, i, &mut b);
            b.push(b',');
            written = i + 1;
        }
    }
    filter_srcset_element(&s, written, s.len(), &mut b);
    b
}

// Go: url.go:htmlSpaceAndASCIIAlnumBytes
/// Derived from <https://play.golang.org/p/Dhmj7FORT5>
const HTML_SPACE_AND_ASCII_ALNUM_BYTES: &[u8; 16] =
    b"\x00\x36\x00\x00\x01\x00\xff\x03\xfe\xff\xff\x07\xfe\xff\xff\x07";

// Go: url.go:isHTMLSpace
/// isHTMLSpace is true iff c is a whitespace character per
/// <https://infra.spec.whatwg.org/#ascii-whitespace>
pub(crate) fn is_html_space(c: u8) -> bool {
    (c <= 0x20) && 0 != (HTML_SPACE_AND_ASCII_ALNUM_BYTES[(c >> 3) as usize] & (1 << (c & 0x7)))
}

// Go: url.go:isHTMLSpaceOrASCIIAlnum
pub(crate) fn is_html_space_or_ascii_alnum(c: u8) -> bool {
    (c < 0x80) && 0 != (HTML_SPACE_AND_ASCII_ALNUM_BYTES[(c >> 3) as usize] & (1 << (c & 0x7)))
}

// Go: url.go:filterSrcsetElement
pub(crate) fn filter_srcset_element(s: &[u8], left: usize, right: usize, b: &mut Vec<u8>) {
    let mut start = left;
    while start < right && is_html_space(s[start]) {
        start += 1;
    }
    let mut end = right;
    for i in start..right {
        if is_html_space(s[i]) {
            end = i;
            break;
        }
    }
    let url = &s[start..end];
    if is_safe_url(url) {
        // If image metadata is only spaces or alnums then
        // we don't need to URL normalize it.
        let mut metadata_ok = true;
        for i in end..right {
            if !is_html_space_or_ascii_alnum(s[i]) {
                metadata_ok = false;
                break;
            }
        }
        if metadata_ok {
            b.extend_from_slice(&s[left..start]);
            process_url_onto(url, true, b);
            b.extend_from_slice(&s[end..right]);
            return;
        }
    }
    b.push(b'#');
    b.extend_from_slice(FILTER_FAILSAFE.as_bytes());
}
