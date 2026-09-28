//! Port of `publisher/htmlElementsCollector.go`.
//!
//! Owner: Wave B task T07 (transform-publisher).

//! Go `publisher/htmlElementsCollector.go` — port the state machine VERBATIM (it decides the CSS
//! purged into every page). Stops at the first `utf8.RuneError` of a Write; `pre|textarea|script|
//! style` prefix match; per element string `x/net/html.Parse` (see `xnethtml`) with the in-body
//! quirks; classes from `(?i)^class$|transition` attributes; `sort.Strings` + dedupe on merge.
//!
//! The Go regexps are matched by hand with RE2's semantics: `(?i)` is Unicode simple case
//! folding (`s` also matches U+017F `ſ`), `\s` is `[\t\n\f\r ]`, `.` is any rune but `\n`.

use std::collections::BTreeSet;

use go_unicode::{Rune, strings as gostrings, utf8};
use nh_config::common_config::BuildStats;

use crate::xnethtml;

/// Go: `eof`.
const EOF: Rune = -1;

/// Go: `publisher.HTMLElements` (JSON keys `tags`, `classes`, `ids`; nil slices -> `null`).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct HtmlElements {
    pub tags: Option<Vec<String>>,
    pub classes: Option<Vec<String>>,
    pub ids: Option<Vec<String>>,
}

/// Go `h.X = append(h.X, other.X...)` then `helpers.UniqueStringsReuse(h.X)`: a nil result only
/// when both are nil or empty (Go's append of nothing to a nil slice stays nil, and
/// `UniqueStringsReuse(nil)` is `nil[:0]` = nil).
fn merge_list(h: &mut Option<Vec<String>>, other: &Option<Vec<String>>) {
    let other = other.as_deref().unwrap_or(&[]);
    let merged = match h.take() {
        None if other.is_empty() => None,
        None => Some(other.to_vec()),
        Some(mut v) => {
            v.extend_from_slice(other);
            Some(v)
        }
    };
    *h = merged.map(nh_helpers::general::unique_strings_reuse);
}

impl HtmlElements {
    /// Go: `Merge(other)` (append + UniqueStringsReuse).
    // Go: publisher/htmlElementsCollector.go:Merge
    pub fn merge(&mut self, other: &HtmlElements) {
        merge_list(&mut self.tags, &other.tags);
        merge_list(&mut self.classes, &other.classes);
        merge_list(&mut self.ids, &other.ids);
    }

    /// Go: `Sort()` (`sort.Strings` each; equal strings are indistinguishable, so any sort gives
    /// Go's order).
    // Go: publisher/htmlElementsCollector.go:Sort
    pub fn sort(&mut self) {
        for v in [&mut self.tags, &mut self.classes, &mut self.ids]
            .into_iter()
            .flatten()
        {
            v.sort();
        }
    }
}

/// Go: `htmlElement`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct HtmlElement {
    pub tag: String,
    pub classes: Vec<String>,
    pub ids: Vec<String>,
}

/// Go: `htmlElementsCollector`.
pub struct HtmlElementsCollector {
    pub conf: BuildStats,
    /// Contains the raw HTML string. We will get the same element several times, and want to
    /// avoid costly reparsing when this is used for aggregated data only.
    pub(crate) element_set: BTreeSet<String>,
    pub(crate) elements: Vec<HtmlElement>,
}

impl HtmlElementsCollector {
    // Go: publisher/htmlElementsCollector.go:newHTMLElementsCollector
    pub fn new(conf: BuildStats) -> Self {
        HtmlElementsCollector {
            conf,
            element_set: BTreeSet::new(),
            elements: Vec::new(),
        }
    }

    /// Feeds one published document (Go: a single `Write` of the whole buffer through a new
    /// `htmlElementsCollectorWriter`, as `Publish` does). Returns the last parse error the
    /// writer recorded (Go keeps it in the writer and never reads it).
    pub fn write(&mut self, p: &[u8]) -> Option<String> {
        let mut w = HtmlElementsCollectorWriter::new(self);
        w.write(p);
        w.err
    }

    // Go: publisher/htmlElementsCollector.go:getHTMLElements
    pub fn get_html_elements(&self) -> HtmlElements {
        let mut classes = Vec::new();
        let mut ids = Vec::new();
        let mut tags = Vec::new();

        for el in &self.elements {
            classes.extend(el.classes.iter().cloned());
            ids.extend(el.ids.iter().cloned());
            if !self.conf.disable_tags {
                tags.push(el.tag.clone());
            }
        }

        HtmlElements {
            classes: nh_helpers::general::unique_strings_sorted(classes),
            ids: nh_helpers::general::unique_strings_sorted(ids),
            tags: nh_helpers::general::unique_strings_sorted(tags),
        }
    }
}

/// What a `lexElementInside` state resolves to when its element ends.
#[derive(Clone, Debug, PartialEq, Eq)]
enum Resolve {
    /// `htmlLexStart`.
    LexStart,
    /// `consumeBuffUntil(isClosedByTag(buff, tag), htmlLexStart)` (pre, textarea, script, style).
    ConsumeBuffUntilClosed(Vec<u8>),
}

/// Go: `htmlCollectorStateFunc` values.
#[derive(Clone, Debug, PartialEq, Eq)]
enum State {
    /// `htmlLexStart`.
    LexStart,
    /// `htmlLexElementStart`.
    LexElementStart,
    /// `htmlLexToEndOfComment`.
    LexToEndOfComment,
    /// `lexElementInside(resolve)`.
    ElementInside(Resolve),
    /// `consumeBuffUntil(...)` (see [`Resolve::ConsumeBuffUntilClosed`]).
    ConsumeBuffUntilClosed(Vec<u8>),
    /// `consumeRuneUntil(r == '>', htmlLexStart)`.
    ConsumeRuneUntilGt,
}

impl Resolve {
    fn state(self) -> State {
        match self {
            Resolve::LexStart => State::LexStart,
            Resolve::ConsumeBuffUntilClosed(t) => State::ConsumeBuffUntilClosed(t),
        }
    }
}

/// Go: `htmlElementsCollectorWriter`.
pub struct HtmlElementsCollectorWriter<'a> {
    collector: &'a mut HtmlElementsCollector,

    /// Current rune
    r: Rune,
    /// The width in bytes of r
    width: usize,
    /// The current position in input
    pos: usize,

    /// The last parse error (Go's `w.err`).
    pub err: Option<String>,

    in_quote: Rune,

    buff: Vec<u8>,

    /// Current state
    state: State,
}

impl<'a> HtmlElementsCollectorWriter<'a> {
    // Go: publisher/htmlElementsCollector.go:newHTMLElementsCollectorWriter
    pub fn new(collector: &'a mut HtmlElementsCollector) -> Self {
        HtmlElementsCollectorWriter {
            collector,
            r: 0,
            width: 0,
            pos: 0,
            err: None,
            in_quote: 0,
            buff: Vec::new(),
            state: State::LexStart,
        }
    }

    /// Write collects HTML elements from p, which must contain complete runes (the state carries
    /// over to the next Write). Returns `len(p)`.
    // Go: publisher/htmlElementsCollector.go:Write
    pub fn write(&mut self, p: &[u8]) -> usize {
        loop {
            self.r = self.next(p);
            if self.r == EOF || self.r == utf8::RUNE_ERROR {
                break;
            }
            let st = std::mem::replace(&mut self.state, State::LexStart);
            self.state = self.step(st, p);
        }

        self.pos = 0;

        p.len()
    }

    /// Runs the current state function.
    fn step(&mut self, st: State, input: &[u8]) -> State {
        match st {
            State::LexStart => self.html_lex_start(input),
            State::LexElementStart => self.html_lex_element_start(input),
            State::LexToEndOfComment => self.html_lex_to_end_of_comment(),
            State::ElementInside(resolve) => self.lex_element_inside(resolve),
            State::ConsumeBuffUntilClosed(tag) => self.consume_buff_until(tag),
            State::ConsumeRuneUntilGt => self.consume_rune_until(),
        }
    }

    // Go: publisher/htmlElementsCollector.go:backup
    fn backup(&mut self, input: &[u8]) {
        self.pos -= self.width;
        self.r = utf8::decode_rune(&input[self.pos..]).0;
    }

    fn write_rune(&mut self, r: Rune) {
        utf8::append_rune(&mut self.buff, r);
    }

    /// `consumeBuffUntil(func() bool { r == '>' && isClosedByTag(buff, tag) }, htmlLexStart)`.
    // Go: publisher/htmlElementsCollector.go:consumeBuffUntil
    fn consume_buff_until(&mut self, tag: Vec<u8>) -> State {
        self.write_rune(self.r);
        if self.r == '>' as Rune && is_closed_by_tag(&self.buff, &tag) {
            self.buff.clear();
            return State::LexStart;
        }
        State::ConsumeBuffUntilClosed(tag)
    }

    /// `consumeRuneUntil(func(r rune) bool { return r == '>' }, htmlLexStart)`.
    // Go: publisher/htmlElementsCollector.go:consumeRuneUntil
    fn consume_rune_until(&mut self) -> State {
        if self.r == '>' as Rune {
            return State::LexStart;
        }
        State::ConsumeRuneUntilGt
    }

    /// Starts with e.g. "<body " or "<div".
    // Go: publisher/htmlElementsCollector.go:lexElementInside
    fn lex_element_inside(&mut self, resolve: Resolve) -> State {
        self.write_rune(self.r);

        // Skip any text inside a quote.
        if self.r == '\'' as Rune || self.r == '"' as Rune {
            if self.in_quote == self.r {
                self.in_quote = 0;
            } else if self.in_quote == 0 {
                self.in_quote = self.r;
            }
        }

        if self.in_quote != 0 {
            return State::ElementInside(resolve);
        }

        if self.r == '>' as Rune {
            // The buffer only holds complete runes of valid UTF-8 (Write stops at RuneError).
            let s = String::from_utf8_lossy(&self.buff).into_owned();
            // Go: `defer w.buff.Reset()`.
            self.buff.clear();

            // First check if we have processed this element before.
            if self.collector.element_set.contains(&s) {
                return resolve.state();
            }

            if s.is_empty() {
                return resolve.state();
            }

            // Parse each collected element.
            match parse_html_element(&self.collector.conf, &s) {
                Err(e) => {
                    self.err = Some(e);
                    return resolve.state();
                }
                Ok(el) => {
                    // Write this tag to the element set.
                    self.collector.element_set.insert(s);
                    self.collector.elements.push(el);
                }
            }

            return resolve.state();
        }

        State::ElementInside(resolve)
    }

    // Go: publisher/htmlElementsCollector.go:next
    fn next(&mut self, input: &[u8]) -> Rune {
        if self.pos >= input.len() {
            self.width = 0;
            return EOF;
        }

        let (rune_value, rune_width) = utf8::decode_rune(&input[self.pos..]);

        self.width = rune_width;
        self.pos += self.width;
        rune_value
    }

    /// At "<", buffer empty. Potentially starting a HTML element.
    // Go: publisher/htmlElementsCollector.go:htmlLexElementStart
    fn html_lex_element_start(&mut self, input: &[u8]) -> State {
        if self.r == '>' as Rune || go_unicode::is_space(self.r) {
            if self.buff.len() < 2 || self.buff.starts_with(b"</") {
                self.buff.clear();
                return State::LexStart;
            }

            let tag_name = self.buff[1..].to_vec();
            let is_self_closing = tag_name[tag_name.len() - 1] == b'/';

            if !is_self_closing && skip_inner_element_re(&tag_name) {
                // pre, script etc. We collect classes etc. on the surrounding element, but skip
                // the inner content.
                self.backup(input);

                return State::ElementInside(Resolve::ConsumeBuffUntilClosed(tag_name));
            } else if skip_all_element_re(&tag_name) {
                // E.g. "<!DOCTYPE ..."
                self.buff.clear();
                return State::ConsumeRuneUntilGt;
            } else {
                self.backup(input);
                return State::ElementInside(Resolve::LexStart);
            }
        }

        self.write_rune(self.r);

        // If it's a comment, skip to its end.
        if self.r == '-' as Rune && self.buff == b"<!--" {
            self.buff.clear();
            return State::LexToEndOfComment;
        }

        State::LexElementStart
    }

    /// Entry state func. Looks for a opening bracket, '<'.
    // Go: publisher/htmlElementsCollector.go:htmlLexStart
    fn html_lex_start(&mut self, input: &[u8]) -> State {
        if self.r == '<' as Rune {
            self.backup(input);
            self.buff.clear();
            return State::LexElementStart;
        }

        State::LexStart
    }

    /// After "<!--", buff empty.
    // Go: publisher/htmlElementsCollector.go:htmlLexToEndOfComment
    fn html_lex_to_end_of_comment(&mut self) -> State {
        self.write_rune(self.r);

        if self.r == '>' as Rune && self.buff.ends_with(b"-->") {
            // Done, start looking for HTML elements again.
            return State::LexStart;
        }

        State::LexToEndOfComment
    }
}

/// Whether runes `a` and `b` are equal under Unicode simple case folding (RE2 `(?i)` literal
/// matching, `strings.EqualFold` per rune).
fn rune_fold_eq(a: Rune, b: Rune) -> bool {
    if a == b {
        return true;
    }
    let mut r = go_unicode::simple_fold(a);
    while r != a {
        if r == b {
            return true;
        }
        r = go_unicode::simple_fold(r);
    }
    false
}

/// The length of the prefix of `s` that matches the ASCII literal `lit` under `(?i)`, if any.
fn fold_prefix_len(s: &[u8], lit: &[u8]) -> Option<usize> {
    let mut pos = 0;
    for &c in lit {
        if pos >= s.len() {
            return None;
        }
        let (r, w) = utf8::decode_rune(&s[pos..]);
        if !rune_fold_eq(r, Rune::from(c)) {
            return None;
        }
        pos += w;
    }
    Some(pos)
}

/// Go: `skipInnerElementRe.Match(b)` = `(?i)^(pre|textarea|script|style)`.
fn skip_inner_element_re(b: &[u8]) -> bool {
    [&b"pre"[..], b"textarea", b"script", b"style"]
        .iter()
        .any(|lit| fold_prefix_len(b, lit).is_some())
}

/// Go: `skipAllElementRe.Match(b)` = `(?i)^!DOCTYPE`.
fn skip_all_element_re(b: &[u8]) -> bool {
    fold_prefix_len(b, b"!DOCTYPE").is_some()
}

/// Go: `classAttrRe.MatchString(s)` = `(?i)^class$|transition`.
fn class_attr_re(s: &[u8]) -> bool {
    if fold_prefix_len(s, b"class") == Some(s.len()) {
        return true;
    }
    let mut i = 0;
    while i < s.len() {
        if fold_prefix_len(&s[i..], b"transition").is_some() {
            return true;
        }
        i += utf8::decode_rune(&s[i..]).1;
    }
    false
}

/// RE2 `\s`.
fn is_re_space(c: u8) -> bool {
    matches!(c, b'\t' | b'\n' | b'\x0c' | b'\r' | b' ')
}

/// Tries `'?(.*?)'?:\s.*` at `start` (leftmost-first priorities); returns the group and the
/// match end.
fn json_attr_match_at(s: &[u8], start: usize) -> Option<((usize, usize), usize)> {
    let mut group_starts = Vec::with_capacity(2);
    if start < s.len() && s[start] == b'\'' {
        // `'?` is greedy: taking the quote is tried first.
        group_starts.push(start + 1);
    }
    group_starts.push(start);
    for g in group_starts {
        // `(.*?)` is lazy: the shortest group first; `.` does not match '\n'.
        let mut k = g;
        loop {
            // `'?` then `:` then `\s`.
            let mut after: Option<usize> = None;
            if k < s.len() && s[k] == b'\'' {
                if k + 2 < s.len() && s[k + 1] == b':' && is_re_space(s[k + 2]) {
                    after = Some(k + 3);
                }
            } else if k + 1 < s.len() && s[k] == b':' && is_re_space(s[k + 1]) {
                after = Some(k + 2);
            }
            if let Some(mut e) = after {
                // `.*` is greedy: to the end of the line.
                while e < s.len() && s[e] != b'\n' {
                    e += 1;
                }
                return Some(((g, k), e));
            }
            if k >= s.len() || s[k] == b'\n' {
                break;
            }
            k += 1;
        }
    }
    None
}

/// Go: `jsonAttrRe.ReplaceAllString(s, "$1")` with `jsonAttrRe = '?(.*?)'?:\s.*`.
///
/// Every delimiter of the pattern is ASCII, so matching over the bytes of a valid UTF-8 string
/// gives the rune matcher's boundaries.
fn json_attr_replace_all(s: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(s.len());
    let mut last = 0;
    let mut i = 0;
    while i <= s.len() {
        match json_attr_match_at(s, i) {
            Some(((g0, g1), end)) => {
                out.extend_from_slice(&s[last..i]);
                out.extend_from_slice(&s[g0..g1]);
                last = end;
                // The match is never empty (it holds ':' and a space).
                i = end;
            }
            None => {
                if i >= s.len() {
                    break;
                }
                i += utf8::decode_rune(&s[i..]).1;
            }
        }
    }
    out.extend_from_slice(&s[last..]);
    out
}

/// Go `strings.Fields(s)` as owned strings.
fn fields(s: &[u8]) -> Vec<String> {
    gostrings::fields(s)
        .into_iter()
        .map(|f| String::from_utf8_lossy(f).into_owned())
        .collect()
}

// Go: publisher/htmlElementsCollector.go:parseHTMLElement
pub fn parse_html_element(conf: &BuildStats, el_str: &str) -> Result<HtmlElement, String> {
    let mut el = HtmlElement::default();

    let tag_name = parse_start_tag(el_str.as_bytes())?;

    el.tag = String::from_utf8_lossy(&gostrings::to_lower(tag_name)).into_owned();
    let mut tag_name_to_parse = el.tag.clone();

    // The net/html parser does not handle single table elements as input, e.g. tbody. We only
    // care about the element/class/ids, so just store away the original tag name and pretend
    // it's a <div>.
    let mut el_bytes = el_str.as_bytes().to_vec();
    if matches!(el.tag.as_str(), "thead" | "tbody" | "tfoot" | "td" | "tr") {
        el_bytes = gostrings::replace(&el_bytes, tag_name, b"div", 1).into_owned();
        tag_name_to_parse = "div".to_string();
    }

    // Go's html.Parse never returns an error for a strings.Reader; where it panics, the port
    // returns the panic text.
    let (d, root) = xnethtml::parse(&el_bytes)?;

    xnethtml::walk_elements(&d, root, &mut |n| {
        if n.data != tag_name_to_parse.as_bytes() {
            return;
        }
        for a in &n.attr {
            if gostrings::equal_fold(&a.key, b"id") {
                // There should be only one, but one never knows...
                if !conf.disable_ids {
                    el.ids.push(String::from_utf8_lossy(&a.val).into_owned());
                }
                continue;
            }
            if conf.disable_classes {
                continue;
            }

            if class_attr_re(&a.key) {
                el.classes.extend(fields(&a.val));
            } else {
                let key = gostrings::to_lower(&a.key).into_owned();
                let mut val = gostrings::trim_space(&a.val).to_vec();

                if gostrings::contains(&key, b":class") {
                    if val.starts_with(b"{") {
                        // This looks like a Vue or AlpineJS class binding.
                        val = go_unicode::bytes::replace_all(
                            gostrings::trim(&val, b"{}"),
                            b", ",
                            b"\n",
                        );
                        let lines: Vec<&[u8]> = gostrings::split(&val, b"\n")
                            .into_iter()
                            .map(gostrings::trim_space)
                            .collect();
                        val = lines.join(&b'\n');

                        val = json_attr_replace_all(&val);

                        el.classes.extend(fields(&val));
                    }
                    // Also add single quoted strings. This may introduce some false positives,
                    // but it covers some missing cases in the above. E.g. AlpinesJS'
                    // :class="isTrue 'class1' : 'class2'"
                    el.classes.extend(extract_single_quoted_strings(&val));
                }
            }
        }
    });

    Ok(el)
}

/// Variants of s
///
/// ```text
/// <body class="b a">
/// <div>
/// ```
// Go: publisher/htmlElementsCollector.go:parseStartTag
pub fn parse_start_tag(s: &[u8]) -> Result<&[u8], String> {
    let space_index = gostrings::index_func(s, go_unicode::is_space);

    let slice_err =
        |lo: usize, hi: usize| format!("runtime error: slice bounds out of range [{lo}:{hi}]");
    let mut s = if space_index == -1 {
        if s.is_empty() {
            return Err(slice_err(1, 0));
        }
        if s.len() < 2 {
            return Err(slice_err(1, s.len() - 1));
        }
        &s[1..s.len() - 1]
    } else {
        let si = space_index as usize;
        if si < 1 {
            return Err(slice_err(1, si));
        }
        &s[1..si]
    };

    if s.is_empty() {
        return Err("runtime error: index out of range [-1]".into());
    }
    if s[s.len() - 1] == b'/' {
        // Self closing.
        s = &s[..s.len() - 1];
    }

    Ok(s)
}

/// isClosedByTag reports whether b ends with a closing tag for tagName.
// Go: publisher/htmlElementsCollector.go:isClosedByTag
pub fn is_closed_by_tag(b: &[u8], tag_name: &[u8]) -> bool {
    if b.is_empty() {
        return false;
    }

    if b[b.len() - 1] != b'>' {
        return false;
    }

    let mut lo = 0usize;
    let mut hi = 0usize;
    let mut state = 0;
    let mut in_word = false;

    let mut i = b.len() as isize - 2;
    while i >= 0 {
        let iu = i as usize;
        match b[iu] {
            b'<' => {
                if state != 1 {
                    return false;
                }
                state = 2;
                break;
            }
            b'/' => {
                if state != 0 {
                    return false;
                }
                state += 1;
                if in_word {
                    lo = iu + 1;
                    in_word = false;
                }
            }
            c if is_space(c) => {
                if in_word {
                    lo = iu + 1;
                    in_word = false;
                }
            }
            _ => {
                if !in_word {
                    hi = iu + 1;
                    in_word = true;
                }
            }
        }
        i -= 1;
    }

    if state != 2 || lo >= hi {
        return false;
    }

    go_unicode::bytes::equal_fold(tag_name, &b[lo..hi])
}

// Go: publisher/htmlElementsCollector.go:isSpace
fn is_space(b: u8) -> bool {
    b == b' ' || b == b'\t' || b == b'\n'
}

// Go: publisher/htmlElementsCollector.go:extractSingleQuotedStrings
fn extract_single_quoted_strings(s: &[u8]) -> Vec<String> {
    let mut in_quote = false;
    let mut lo = 0;
    let mut words = Vec::new();

    let mut i = 0;
    while i < s.len() {
        let (r, w) = utf8::decode_rune(&s[i..]);
        if r == '\'' as Rune {
            if !in_quote {
                in_quote = true;
                lo = i + 1;
            } else {
                in_quote = false;
                let hi = i;
                words.extend(fields(&s[lo..hi]));
            }
        }
        i += w;
    }

    words
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: publisher/htmlElementsCollector.go (556 lines; 17/19 funcs executed)
//   types: HTMLElements, htmlElement, htmlElementsCollector, htmlElementsCollectorWriter, htmlCollectorStateFunc
// OK L50-55: newHTMLElementsCollector(conf config.BuildStats) *htmlElementsCollector
// OK L57-66: newHTMLElementsCollectorWriter(collector *htmlElementsCollector) *htmlElementsCollectorWriter
// OK L75-83: (h *HTMLElements) Merge(other HTMLElements)
// OK L85-89: (h *HTMLElements) Sort()
// OK L110-136: (c *htmlElementsCollector) getHTMLElements() HTMLElements
// OK L160-179: (w *htmlElementsCollectorWriter) Write(p []byte) (int, error)
// OK L181-184: (l *htmlElementsCollectorWriter) backup()
// OK L186-197: (w *htmlElementsCollectorWriter) consumeBuffUntil(condition func() bool, resolve htmlCollectorStateFunc) htmlCollectorStateFunc
// OK L199-208: (w *htmlElementsCollectorWriter) consumeRuneUntil(condition func(r rune) bool, resolve htmlCollectorStateFunc) htmlCollectorStateFunc
// OK L211-276: (w *htmlElementsCollectorWriter) lexElementInside(resolve htmlCollectorStateFunc) htmlCollectorStateFunc
// OK L278-289: (l *htmlElementsCollectorWriter) next() rune
// OK L296-347: htmlLexElementStart(w *htmlElementsCollectorWriter) htmlCollectorStateFunc
// OK L351-359: htmlLexStart(w *htmlElementsCollectorWriter) htmlCollectorStateFunc
// OK L362-371: htmlLexToEndOfComment(w *htmlElementsCollectorWriter) htmlCollectorStateFunc
// OK L373-447: (w *htmlElementsCollectorWriter) parseHTMLElement(elStr string) (el htmlElement, err error)
// OK L453-470: parseStartTag(s string) string
// OK L473-526: isClosedByTag(b, tagName []byte) bool
// OK L528-530: isSpace(b byte) bool
// OK L532-556: extractSingleQuotedStrings(s string) []string
// ---------------------------------------------------------------------------
