//! Port of `resources/page/page_markup.go`.
//!
//! Owner: Wave B task T11 (page-api-paths).
//!
//! Go `resources/page/page_markup.go`: summary extraction (`.Summary` is computed eagerly by
//! page__content.go but unused by seeksnack) and the `page.Summary` value type.
//!
//! The rendered content is a Go string that may hold any bytes, so [`HtmlSummary`] works on
//! bytes (`source: Vec<u8>`); the Go regexps are hand-written matchers with Go's semantics.

use std::any::Any;
use std::borrow::Cow;

use go_unicode::utf8;
use go_value::{GoString, HostCtx, Object, SafeKind, Value};
use nh_common::types::types::LowHigh;
use nh_media::media::media_type::MediaType;

pub const SUMMARY_TYPE_AUTO: &str = "auto";
pub const SUMMARY_TYPE_MANUAL: &str = "manual";
pub const SUMMARY_TYPE_FRONT_MATTER: &str = "frontmatter";

/// Go: `page.Summary` (fields `Text` (template.HTML), `Type`, `Truncated`; `IsZero`;
/// `PrintableValue()` = Text).
#[derive(Clone, Debug, Default)]
pub struct Summary {
    pub text: GoString,
    pub type_: String,
    pub truncated: bool,
}

impl Summary {
    // Go: resources/page/page_markup.go:IsZero
    pub fn is_zero(&self) -> bool {
        self.text.is_empty()
    }

    // Go: resources/page/page_markup.go:PrintableValue
    pub fn printable_value(&self) -> Value {
        Value::Safe(SafeKind::Html, self.text.clone())
    }
}

impl Object for Summary {
    fn type_name(&self) -> Cow<'_, str> {
        Cow::Borrowed("page.Summary")
    }
    fn kind(&self) -> go_value::Kind {
        go_value::Kind::Struct
    }
    fn has_method(&self, name: &str) -> bool {
        matches!(name, "IsZero" | "PrintableValue")
    }
    fn call_method(
        &self,
        _ctx: HostCtx<'_>,
        name: &str,
        args: &[Value],
    ) -> Option<go_value::Result<Value>> {
        match name {
            "IsZero" => Some(
                nh_common::object::args::exactly(args, 0, name)
                    .map(|_| Value::Bool(Summary::is_zero(self))),
            ),
            "PrintableValue" => Some(
                nh_common::object::args::exactly(args, 0, name)
                    .map(|_| Summary::printable_value(self)),
            ),
            _ => None,
        }
    }
    fn field(&self, name: &str) -> Option<Value> {
        match name {
            "Text" => Some(Value::Safe(SafeKind::Html, self.text.clone())),
            "Type" => Some(Value::string(self.type_.as_str())),
            "Truncated" => Some(Value::Bool(self.truncated)),
            _ => None,
        }
    }
    fn is_zero(&self) -> Option<bool> {
        Some(self.text.is_empty())
    }
    fn printable_value(&self) -> Option<Value> {
        Some(Value::Safe(SafeKind::Html, self.text.clone()))
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// Go: `page.HtmlSummary`. Go stores a manual divider that was not found as `Low: -1`; the port
/// stores the zero range (both are `IsZero`).
#[derive(Clone, Debug, Default)]
pub struct HtmlSummary {
    pub source: Vec<u8>,
    pub summary_low_high: LowHigh,
    pub summary_end_tag: LowHigh,
    pub wrapper_start: LowHigh,
    pub wrapper_end: LowHigh,
    pub divider: LowHigh,
}

/// Go: `strings.TrimSpace`.
fn trim_space(s: &[u8]) -> Vec<u8> {
    go_unicode::strings::trim_space(s).to_vec()
}

impl HtmlSummary {
    // Go: resources/page/page_markup.go:wrap
    fn wrap(&self, ss: &[u8]) -> Vec<u8> {
        if self.wrapper_start.is_zero() {
            return ss.to_vec();
        }
        let mut out = self.source[self.wrapper_start.low..self.wrapper_start.high].to_vec();
        out.extend_from_slice(ss);
        out.extend_from_slice(&self.source[self.wrapper_end.low..self.wrapper_end.high]);
        out
    }

    // Go: resources/page/page_markup.go:wrapLeft
    fn wrap_left(&self, ss: &[u8]) -> Vec<u8> {
        if self.wrapper_start.is_zero() {
            return ss.to_vec();
        }

        let mut out = self.source[self.wrapper_start.low..self.wrapper_start.high].to_vec();
        out.extend_from_slice(ss);
        out
    }

    /// Go: `Value(l)` — the source bytes of `l`.
    // Go: resources/page/page_markup.go:Value
    pub fn value(&self, l: LowHigh) -> &[u8] {
        &self.source[l.low..l.high]
    }

    // Go: resources/page/page_markup.go:trimSpace
    fn trim_space(&self, ss: &[u8]) -> Vec<u8> {
        trim_space(ss)
    }

    // Go: resources/page/page_markup.go:Content
    pub fn content(&self) -> Vec<u8> {
        if self.divider.is_zero() {
            return self.source.clone();
        }
        let mut ss = self.source[..self.divider.low].to_vec();
        ss.extend_from_slice(&self.source[self.divider.high..]);
        self.trim_space(&ss)
    }

    // Go: resources/page/page_markup.go:Summary
    pub fn summary(&self) -> Vec<u8> {
        if self.divider.is_zero() {
            return self.trim_space(&self.wrap(self.value(self.summary_low_high)));
        }
        let mut ss = self.source[self.summary_low_high.low..self.divider.low].to_vec();
        if self.summary_low_high.high > self.divider.high {
            ss.extend_from_slice(&self.source[self.divider.high..self.summary_low_high.high]);
        }
        if !self.summary_end_tag.is_zero() {
            ss.extend_from_slice(self.value(self.summary_end_tag));
        }
        self.trim_space(&self.wrap(&ss))
    }

    // Go: resources/page/page_markup.go:ContentWithoutSummary
    pub fn content_without_summary(&self) -> Vec<u8> {
        if self.divider.is_zero() {
            if self.summary_low_high.low == self.wrapper_start.high
                && self.summary_low_high.high == self.wrapper_end.low
            {
                return Vec::new();
            }
            return self.trim_space(&self.wrap_left(&self.source[self.summary_low_high.high..]));
        }
        if self.summary_end_tag.is_zero() {
            return self.trim_space(&self.wrap_left(&self.source[self.divider.high..]));
        }
        self.trim_space(&self.wrap_left(&self.source[self.summary_end_tag.high..]))
    }

    // Go: resources/page/page_markup.go:Truncated
    pub fn truncated(&self) -> bool {
        self.summary_low_high.high < self.source.len()
    }

    // Go: resources/page/page_markup.go:resolveParagraphTagAndSetWrapper
    fn resolve_paragraph_tag_and_set_wrapper(&mut self, mt: &MediaType) -> TagReStartEnd {
        let mut ptag = START_END_P;

        let ct = nh_media::media::config::default_content_types();
        if mt.sub_type == ct.ascii_doc.sub_type {
            ptag = START_END_DIV;
        } else if mt.sub_type == ct.re_structured_text.sub_type {
            const MARKER_START: &[u8] = b"<div class=\"document\">";
            const MARKER_END: &[u8] = b"</div>";
            let i1 = go_unicode::strings::index(&self.source, MARKER_START);
            let i2 = go_unicode::strings::last_index(&self.source, MARKER_END);
            if i1 > -1 && i2 > -1 {
                self.wrapper_start = LowHigh {
                    low: 0,
                    high: i1 as usize + MARKER_START.len(),
                };
                self.wrapper_end = LowHigh {
                    low: i2 as usize,
                    high: self.source.len(),
                };
            }
        }
        ptag
    }
}

fn is_ascii_letter(b: u8) -> bool {
    b.is_ascii_alphabetic()
}

/// Go: `isProbablyHTMLTag = regexp.MustCompile(`^<\/?[A-Za-z]+>?$`)`.
fn is_probably_html_tag(s: &[u8]) -> bool {
    let mut i = 0;
    if s.first() != Some(&b'<') {
        return false;
    }
    i += 1;
    if s.get(i) == Some(&b'/') {
        i += 1;
    }
    let start = i;
    while i < s.len() && is_ascii_letter(s[i]) {
        i += 1;
    }
    if i == start {
        return false;
    }
    if s.get(i) == Some(&b'>') {
        i += 1;
    }
    i == s.len()
}

/// Go: `isProablyHTMLAttribute = regexp.MustCompile(`^[A-Za-z]+=["']`)`.
fn is_probably_html_attribute(s: &[u8]) -> bool {
    let mut i = 0;
    while i < s.len() && is_ascii_letter(s[i]) {
        i += 1;
    }
    i > 0 && s.get(i) == Some(&b'=') && matches!(s.get(i + 1), Some(b'"') | Some(b'\''))
}

/// Avoid counting words that are most likely HTML tokens.
// Go: resources/page/page_markup.go:isProbablyHTMLToken
fn is_probably_html_token(s: &[u8]) -> bool {
    s == b">" || is_probably_html_tag(s) || is_probably_html_attribute(s)
}

/// Go: `tpl.StripHTML(s)` (tpl/template.go). nh-tpl owns it (T13) and it is not ported there
/// yet, so it is ported here privately, like nh-markup does (switch to
/// `nh_tpl::template::strip_html` once it lands).
// Go: tpl/template.go:StripHTML
fn strip_html(s: &[u8]) -> Vec<u8> {
    const HUGO_NEW_LINE_PLACEHOLDER: &[u8] = b"___hugonl_";

    // Shortcut strings with no tags in them
    if !s.iter().any(|&c| c == b'<' || c == b'>') {
        return s.to_vec();
    }

    // strings.NewReplacer("\n", " ", "</p>", placeholder, "<br>", placeholder,
    // "<br />", placeholder).Replace(s)
    let mut pre = Vec::with_capacity(s.len());
    let mut i = 0;
    while i < s.len() {
        let rest = &s[i..];
        if rest[0] == b'\n' {
            pre.push(b' ');
            i += 1;
        } else if rest.starts_with(b"</p>") || rest.starts_with(b"<br>") {
            pre.extend_from_slice(HUGO_NEW_LINE_PLACEHOLDER);
            i += 4;
        } else if rest.starts_with(b"<br />") {
            pre.extend_from_slice(HUGO_NEW_LINE_PLACEHOLDER);
            i += 6;
        } else {
            pre.push(rest[0]);
            i += 1;
        }
    }
    let pre_replaced = pre != s;

    let mut s = gotemplate::html::strip_tags(&pre);

    if pre_replaced {
        s = go_unicode::strings::replace_all(&s, HUGO_NEW_LINE_PLACEHOLDER, b"\n").into_owned();
    }

    let mut was_space = false;
    let mut b = Vec::with_capacity(s.len());
    let mut i = 0;
    while i < s.len() {
        let (r, size) = utf8::decode_rune(&s[i..]);
        let is_space = go_unicode::is_space(r);
        if !is_space || !was_space {
            utf8::append_rune(&mut b, r);
        }
        was_space = is_space;
        i += size;
    }

    if !b.is_empty() {
        s = b;
    }

    s
}

/// Go: `ExtractSummaryFromHTML(mt, input, numWords, isCJK)`.
// Go: resources/page/page_markup.go:ExtractSummaryFromHTML
pub fn extract_summary_from_html(
    mt: &MediaType,
    input: &[u8],
    num_words: i64,
    is_cjk: bool,
) -> HtmlSummary {
    let mut result = HtmlSummary {
        source: input.to_vec(),
        ..Default::default()
    };
    let ptag = result.resolve_paragraph_tag_and_set_wrapper(mt);

    if num_words <= 0 {
        return result;
    }

    let mut count: i64 = 0;

    let count_word = |word: &[u8]| -> i64 {
        let word = go_unicode::strings::trim_space(word);
        if word.is_empty() {
            return 0;
        }
        if is_probably_html_token(word) {
            return 0;
        }

        if is_cjk {
            let word = strip_html(word);
            let rune_count = utf8::rune_count_in_string(&word);
            if word.len() == rune_count {
                return 1;
            } else {
                return rune_count as i64;
            }
        }

        1
    };

    let mut high = input.len();
    if result.wrapper_end.low > 0 {
        high = result.wrapper_end.low;
    }

    let closing: Vec<u8> = [b"</".as_slice(), ptag.tag_name.as_bytes(), b">"].concat();

    let mut j = result.wrapper_start.high;
    while j < high {
        let s = &input[j..];
        let closing_index = go_unicode::strings::index(s, &closing);

        if closing_index == -1 {
            break;
        }
        let closing_index = closing_index as usize;

        let s = &s[..closing_index];

        // Count the words in the current paragraph.
        let mut wi = 0;

        let mut i = 0;
        while i < s.len() {
            // Go `for i, r := range s`: an invalid byte is U+FFFD with width 1.
            let (r, size) = utf8::decode_rune(&s[i..]);
            if go_unicode::is_space(r) || (i as isize + utf8::rune_len(r) == s.len() as isize) {
                let word = &s[wi..i];
                count += count_word(word);
                wi = i;
                if count >= num_words {
                    break;
                }
            }
            i += size;
        }

        if count >= num_words {
            result.summary_low_high = LowHigh {
                low: result.wrapper_start.high,
                high: j + closing_index + ptag.tag_name.len() + 3,
            };
            return result;
        }

        j += closing_index + ptag.tag_name.len() + 2;
    }

    result.summary_low_high = LowHigh {
        low: result.wrapper_start.high,
        high,
    };

    result
}

/// Go: `ExtractSummaryFromHTMLWithDivider` — a manual summary divider.
// Go: resources/page/page_markup.go:ExtractSummaryFromHTMLWithDivider
pub fn extract_summary_from_html_with_divider(
    mt: &MediaType,
    input: &[u8],
    divider: &[u8],
) -> HtmlSummary {
    let mut result = HtmlSummary {
        source: input.to_vec(),
        ..Default::default()
    };
    let low = go_unicode::strings::index(input, divider);

    if low == -1 {
        // No summary.
        return result;
    }
    result.divider = LowHigh {
        low: low as usize,
        high: low as usize + divider.len(),
    };

    let ptag = result.resolve_paragraph_tag_and_set_wrapper(mt);

    if !mt.is_html() {
        let (d, e) = expand_summary_divider(&result.source, &ptag, result.divider);
        result.divider = d;
        result.summary_end_tag = e;
    }

    result.summary_low_high = LowHigh {
        low: result.wrapper_start.high,
        high: result.divider.low,
    };

    result
}

/// Go: `tagReStartEnd` — the two regexps are `<tag[^>]*?>$` and `</tag>$`.
struct TagReStartEnd {
    tag_name: &'static str,
}

const START_END_DIV: TagReStartEnd = TagReStartEnd { tag_name: "div" };
const START_END_P: TagReStartEnd = TagReStartEnd { tag_name: "p" };

impl TagReStartEnd {
    /// `regexp.MustCompile(`<tag[^>]*?>$`).FindString(s)`: the leftmost start of `<tag` from
    /// which no `>` occurs before the final byte, which must be `>`.
    fn start_end_of_string(&self, s: &[u8]) -> Option<usize> {
        let n = s.len();
        if n == 0 || s[n - 1] != b'>' {
            return None;
        }
        let open = [b"<".as_slice(), self.tag_name.as_bytes()].concat();
        // The last '>' before the final byte bounds the candidates.
        let first = match s[..n - 1].iter().rposition(|&c| c == b'>') {
            Some(m) => m + 1,
            None => 0,
        };
        let mut k = first;
        while k + open.len() < n {
            if s[k..].starts_with(&open) {
                return Some(n - k);
            }
            k += 1;
        }
        None
    }

    /// `regexp.MustCompile(`</tag>$`).FindString(s)`.
    fn end_end_of_string(&self, s: &[u8]) -> bool {
        let close = [b"</".as_slice(), self.tag_name.as_bytes(), b">"].concat();
        s.ends_with(&close)
    }
}

/// `pOrDiv = regexp.MustCompile(`<p[^>]?>|<div[^>]?>$`).FindString(s)`: the length of the
/// leftmost match (the first alternative is not anchored; `[^>]` is one rune, an invalid byte
/// counting as one).
fn p_or_div(s: &[u8]) -> Option<usize> {
    let n = s.len();
    // `[^>]?>` at position `at`: Some(len) of the matched part, anchored at the end if `anchored`.
    let opt_then_gt = |at: usize, anchored: bool| -> Option<usize> {
        if at < n && s[at] != b'>' {
            let (_, size) = utf8::decode_rune(&s[at..]);
            let gt = at + size;
            if gt < n && s[gt] == b'>' && (!anchored || gt == n - 1) {
                return Some(size + 1);
            }
        }
        if at < n && s[at] == b'>' && (!anchored || at == n - 1) {
            return Some(1);
        }
        None
    };
    for k in 0..n {
        if s[k..].starts_with(b"<p")
            && let Some(l) = opt_then_gt(k + 2, false)
        {
            return Some(2 + l);
        }
        if s[k..].starts_with(b"<div")
            && let Some(l) = opt_then_gt(k + 4, true)
        {
            return Some(4 + l);
        }
    }
    None
}

// Go: resources/page/page_markup.go:expandSummaryDivider
fn expand_summary_divider(
    s: &[u8],
    re: &TagReStartEnd,
    mut divider: LowHigh,
) -> (LowHigh, LowHigh) {
    let mut end_markup = LowHigh::default();

    if divider.is_zero() {
        return (divider, end_markup);
    }

    let (mut lo, mut hi) = (divider.low as isize, divider.high);

    let mut preserve_end_markup = false;

    // Find the start of the paragraph.

    let mut i: isize = lo - 1;
    while i >= 0 {
        let iu = i as usize;
        if s[iu] == b'>' {
            if let Some(m) = re.start_end_of_string(&s[..iu + 1]) {
                lo = i - m as isize + 1;
                break;
            }
            if let Some(m) = p_or_div(&s[..iu + 1]) {
                i -= m as isize - 1;
                i -= 1;
                continue;
            }
        }

        let (r, _) = utf8::decode_rune(&s[iu..]);
        if !go_unicode::is_space(r) {
            preserve_end_markup = true;
            break;
        }
        i -= 1;
    }

    divider.low = lo as usize;

    // Now walk forward to the end of the paragraph.
    while hi < s.len() {
        if s[hi] != b'>' {
            hi += 1;
            continue;
        }
        if re.end_end_of_string(&s[..hi + 1]) {
            hi += 1;
            break;
        }
        hi += 1;
    }

    if preserve_end_markup {
        end_markup.low = divider.high;
        end_markup.high = hi;
    } else {
        divider.high = hi;
    }

    // Consume trailing newline if any.
    if divider.high < s.len() && s[divider.high] == b'\n' {
        divider.high += 1;
    }

    (divider, end_markup)
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: resources/page/page_markup.go (362 lines; 11/15 funcs executed)
//   types: Content, Markup, Summary, HtmlSummary, tagReStartEnd
// OK L63-65: (s Summary) IsZero() bool
// OK L67-69: (s Summary) PrintableValue() any
// OK L82-87: (s HtmlSummary) wrap(ss string) string
// OK L89-95: (s HtmlSummary) wrapLeft(ss string) string
// OK L97-99: (s HtmlSummary) Value(l types.LowHigh[string]) string
// OK L101-103: (s HtmlSummary) trimSpace(ss string) string
// OK L105-112: (s HtmlSummary) Content() string
// OK L114-126: (s HtmlSummary) Summary() string
// OK L128-139: (s HtmlSummary) ContentWithoutSummary() string
// OK L141-143: (s HtmlSummary) Truncated() bool
// OK L145-162: (s *HtmlSummary) resolveParagraphTagAndSetWrapper(mt media.Type) tagReStartEnd
// OK L170-172: isProbablyHTMLToken(s string) bool
// OK L175-254: ExtractSummaryFromHTML(mt media.Type, input string, numWords int, isCJK bool) (result HtmlSummary)
// OK L258-280: ExtractSummaryFromHTMLWithDivider(mt media.Type, input, divider string) (result HtmlSummary)
// OK L304-362: expandSummaryDivider(s string, re tagReStartEnd, divider types.LowHigh[string]) (types.LowHigh[string], types.LowHigh[string])
// ---------------------------------------------------------------------------
