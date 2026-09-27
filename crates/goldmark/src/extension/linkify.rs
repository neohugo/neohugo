// Go: github.com/yuin/goldmark@v1.7.12/extension/linkify.go

use std::sync::Arc;

use crate::ast::{self, Ast, AutoLinkType, NodeId};
use crate::parser::{self, Context, InlineParser, OptionValue, ParserOption};
use crate::text::{Reader, new_segment};
use crate::util;
use crate::{Extender, Markdown};

/// A regular expression as the linkify extension uses it (Go:
/// `*regexp.Regexp`). Only `FindSubmatchIndex(b)[0:2]` is used: the
/// leftmost-first match `(start, end)` in `b`, or `None`.
///
/// The default URL/WWW patterns are hand-written matchers (see
/// [`URL_REGEXP`], [`WWW_URL_REGEXP`]); custom patterns are any
/// implementation of this trait (a closure works).
pub trait LinkifyRegexp: Send + Sync {
    /// Go: `re.FindSubmatchIndex(b)` → `m[0], m[1]`.
    fn find_submatch_index(&self, b: &[u8]) -> Option<(usize, usize)>;
}

impl<F: Fn(&[u8]) -> Option<(usize, usize)> + Send + Sync> LinkifyRegexp for F {
    fn find_submatch_index(&self, b: &[u8]) -> Option<(usize, usize)> {
        self(b)
    }
}

/// `[-a-zA-Z0-9@:%._\+~#=]` (the host part of both URL patterns).
fn is_host_char(c: u8) -> bool {
    c.is_ascii_alphanumeric()
        || matches!(
            c,
            b'-' | b'@' | b':' | b'%' | b'.' | b'_' | b'+' | b'~' | b'#' | b'='
        )
}

/// The path class of wwwURLRegxp:
/// ``[-a-zA-Z0-9@:%_\+.~#!?&/=\(\);,'">\^{}\[\]`]``.
fn is_www_path_char(c: u8) -> bool {
    c.is_ascii_alphanumeric()
        || matches!(
            c,
            b'-' | b'@'
                | b':'
                | b'%'
                | b'_'
                | b'+'
                | b'.'
                | b'~'
                | b'#'
                | b'!'
                | b'?'
                | b'&'
                | b'/'
                | b'='
                | b'('
                | b')'
                | b';'
                | b','
                | b'\''
                | b'"'
                | b'>'
                | b'^'
                | b'{'
                | b'}'
                | b'['
                | b']'
                | b'`'
        )
}

/// The path class of urlRegexp: the www class plus `$`.
fn is_url_path_char(c: u8) -> bool {
    is_www_path_char(c) || c == b'$'
}

/// Matches `[-a-zA-Z0-9@:%._\+~#=]{1,256}\.[a-z]+` at `b[start..]` with
/// leftmost-first (backtracking) semantics and returns the end of the
/// `[a-z]+` run.
///
/// The host run is greedy: the first successful count is the largest
/// n ≤ 256 (within the run of host characters) such that `b[start+n]` is
/// `.` and `b[start+n+1]` is `[a-z]`; everything after the `[a-z]+` is
/// optional in both patterns, so the first success is final and `[a-z]+`
/// takes its maximal run. All classes are ASCII, so matching bytes is
/// matching runes (a non-ASCII rune, valid or not, matches no class).
fn match_host_tld(b: &[u8], start: usize) -> Option<usize> {
    let mut run = 0;
    while start + run < b.len() && is_host_char(b[start + run]) {
        run += 1;
    }
    let max = run.min(256);
    let mut n = max;
    while n >= 1 {
        let p = start + n;
        if p + 1 < b.len() && b[p] == b'.' && b[p + 1].is_ascii_lowercase() {
            let mut e = p + 2;
            while e < b.len() && b[e].is_ascii_lowercase() {
                e += 1;
            }
            return Some(e);
        }
        n -= 1;
    }
    None
}

/// `(?:[/#?][class]*)?` at `b[i..]`, greedy.
fn match_path(b: &[u8], mut i: usize, class: fn(u8) -> bool) -> usize {
    if i < b.len() && matches!(b[i], b'/' | b'#' | b'?') {
        i += 1;
        while i < b.len() && class(b[i]) {
            i += 1;
        }
    }
    i
}

/// Go: `wwwURLRegxp`
/// (`^www\.[-a-zA-Z0-9@:%._\+~#=]{1,256}\.[a-z]+(?:[/#?][-a-zA-Z0-9@:%_\+.~#!?&/=\(\);,'">\^{}\[\]` + "`" + `]*)?`),
/// hand-written with Go's leftmost-first semantics.
pub struct WwwUrlRegexp;

impl LinkifyRegexp for WwwUrlRegexp {
    fn find_submatch_index(&self, b: &[u8]) -> Option<(usize, usize)> {
        if !b.starts_with(b"www.") {
            return None;
        }
        let e = match_host_tld(b, 4)?;
        Some((0, match_path(b, e, is_www_path_char)))
    }
}

/// Go: `urlRegexp`
/// (`^(?:http|https|ftp)://[-a-zA-Z0-9@:%._\+~#=]{1,256}\.[a-z]+(?::\d+)?(?:[/#?][-a-zA-Z0-9@:%_+.~#$!?&/=\(\);,'">\^{}\[\]` + "`" + `]*)?`),
/// hand-written with Go's leftmost-first semantics.
pub struct UrlRegexp;

impl LinkifyRegexp for UrlRegexp {
    fn find_submatch_index(&self, b: &[u8]) -> Option<(usize, usize)> {
        // (?:http|https|ftp): the alternatives are tried in order, but only
        // one of them can be followed by "://".
        let start = if b.starts_with(b"http://") {
            7
        } else if b.starts_with(b"https://") {
            8
        } else if b.starts_with(b"ftp://") {
            6
        } else {
            return None;
        };
        let mut e = match_host_tld(b, start)?;
        // (?::\d+)? greedy (\d is ASCII [0-9] in RE2)
        if e + 1 < b.len() && b[e] == b':' && b[e + 1].is_ascii_digit() {
            e += 2;
            while e < b.len() && b[e].is_ascii_digit() {
                e += 1;
            }
        }
        Some((0, match_path(b, e, is_url_path_char)))
    }
}

/// The default `urlRegexp`.
pub static URL_REGEXP: UrlRegexp = UrlRegexp;
/// The default `wwwURLRegxp`.
pub static WWW_URL_REGEXP: WwwUrlRegexp = WwwUrlRegexp;

/// An LinkifyConfig struct is a data structure that holds configuration of the
/// Linkify extension.
#[derive(Clone)]
pub struct LinkifyConfig {
    /// `None` is Go's nil (the default http:/https:/ftp: check).
    pub allowed_protocols: Option<Vec<Vec<u8>>>,
    pub url_regexp: Arc<dyn LinkifyRegexp>,
    pub www_regexp: Arc<dyn LinkifyRegexp>,
    /// `None` is Go's nil (util.FindEmailIndex is used).
    pub email_regexp: Option<Arc<dyn LinkifyRegexp>>,
}

const OPT_LINKIFY_ALLOWED_PROTOCOLS: &str = "LinkifyAllowedProtocols";
const OPT_LINKIFY_URL_REGEXP: &str = "LinkifyURLRegexp";
const OPT_LINKIFY_WWW_REGEXP: &str = "LinkifyWWWRegexp";
const OPT_LINKIFY_EMAIL_REGEXP: &str = "LinkifyEmailRegexp";

impl LinkifyConfig {
    // Go: extension/linkify.go:LinkifyConfig.SetOption
    /// SetOption implements SetOptioner.
    pub fn set_option(&mut self, name: &str, value: &OptionValue) {
        match name {
            OPT_LINKIFY_ALLOWED_PROTOCOLS => {
                self.allowed_protocols = Some(
                    value
                        .downcast_ref::<Vec<Vec<u8>>>()
                        .expect("interface conversion: not [][]uint8")
                        .clone(),
                )
            }
            OPT_LINKIFY_URL_REGEXP => {
                self.url_regexp = value
                    .downcast_ref::<Arc<dyn LinkifyRegexp>>()
                    .expect("interface conversion: not *regexp.Regexp")
                    .clone()
            }
            OPT_LINKIFY_WWW_REGEXP => {
                self.www_regexp = value
                    .downcast_ref::<Arc<dyn LinkifyRegexp>>()
                    .expect("interface conversion: not *regexp.Regexp")
                    .clone()
            }
            OPT_LINKIFY_EMAIL_REGEXP => {
                self.email_regexp = Some(
                    value
                        .downcast_ref::<Arc<dyn LinkifyRegexp>>()
                        .expect("interface conversion: not *regexp.Regexp")
                        .clone(),
                )
            }
            _ => {}
        }
    }
}

/// A LinkifyOption sets options for the linkify parser; it is also a
/// parser option (Go: the `LinkifyOption` interface).
#[derive(Clone)]
pub enum LinkifyOption {
    /// Go: WithLinkifyAllowedProtocols.
    AllowedProtocols(Vec<Vec<u8>>),
    /// Go: WithLinkifyURLRegexp.
    UrlRegexp(Arc<dyn LinkifyRegexp>),
    /// Go: WithLinkifyWWWRegexp.
    WwwRegexp(Arc<dyn LinkifyRegexp>),
    /// Go: WithLinkifyEmailRegexp.
    EmailRegexp(Arc<dyn LinkifyRegexp>),
}

impl LinkifyOption {
    /// Go: `SetLinkifyOption`.
    pub fn set_linkify_option(&self, p: &mut LinkifyConfig) {
        match self {
            LinkifyOption::AllowedProtocols(v) => p.allowed_protocols = Some(v.clone()),
            LinkifyOption::UrlRegexp(v) => p.url_regexp = v.clone(),
            LinkifyOption::WwwRegexp(v) => p.www_regexp = v.clone(),
            LinkifyOption::EmailRegexp(v) => p.email_regexp = Some(v.clone()),
        }
    }
}

impl ParserOption for LinkifyOption {
    // Go: SetParserOption of each option type
    fn set_parser_option(self: Box<Self>, c: &mut parser::Config) {
        let (name, value): (&str, OptionValue) = match *self {
            LinkifyOption::AllowedProtocols(v) => (OPT_LINKIFY_ALLOWED_PROTOCOLS, Arc::new(v)),
            LinkifyOption::UrlRegexp(v) => (OPT_LINKIFY_URL_REGEXP, Arc::new(v)),
            LinkifyOption::WwwRegexp(v) => (OPT_LINKIFY_WWW_REGEXP, Arc::new(v)),
            LinkifyOption::EmailRegexp(v) => (OPT_LINKIFY_EMAIL_REGEXP, Arc::new(v)),
        };
        c.options.insert(name.to_string(), value);
    }
}

// Go: extension/linkify.go:WithLinkifyAllowedProtocols
/// WithLinkifyAllowedProtocols is a functional option that specify allowed
/// protocols in autolinks. Each protocol must end with ':' like
/// 'http:' .
pub fn with_linkify_allowed_protocols<T: AsRef<[u8]>>(value: &[T]) -> LinkifyOption {
    LinkifyOption::AllowedProtocols(value.iter().map(|v| v.as_ref().to_vec()).collect())
}

// Go: extension/linkify.go:WithLinkifyURLRegexp
/// WithLinkifyURLRegexp is a functional option that specify
/// a pattern of the URL including a protocol.
pub fn with_linkify_url_regexp(value: Arc<dyn LinkifyRegexp>) -> LinkifyOption {
    LinkifyOption::UrlRegexp(value)
}

// Go: extension/linkify.go:WithLinkifyWWWRegexp
/// WithLinkifyWWWRegexp is a functional option that specify
/// a pattern of the URL without a protocol.
/// This pattern must start with 'www.' .
pub fn with_linkify_www_regexp(value: Arc<dyn LinkifyRegexp>) -> LinkifyOption {
    LinkifyOption::WwwRegexp(value)
}

// Go: extension/linkify.go:WithLinkifyEmailRegexp
/// WithLinkifyEmailRegexp is a functional otpion that specify
/// a pattern of the email address.
pub fn with_linkify_email_regexp(value: Arc<dyn LinkifyRegexp>) -> LinkifyOption {
    LinkifyOption::EmailRegexp(value)
}

struct LinkifyParser {
    config: LinkifyConfig,
}

// Go: extension/linkify.go:NewLinkifyParser
/// NewLinkifyParser return a new InlineParser can parse
/// text that seems like a URL.
pub fn new_linkify_parser(opts: &[LinkifyOption]) -> Box<dyn InlineParser> {
    let mut p = LinkifyParser {
        config: LinkifyConfig {
            allowed_protocols: None,
            url_regexp: Arc::new(UrlRegexp),
            www_regexp: Arc::new(WwwUrlRegexp),
            email_regexp: None,
        },
    };
    for o in opts {
        o.set_linkify_option(&mut p.config);
    }
    Box::new(p)
}

const PROTO_HTTP: &[u8] = b"http:";
const PROTO_HTTPS: &[u8] = b"https:";
const PROTO_FTP: &[u8] = b"ftp:";
const DOMAIN_WWW: &[u8] = b"www.";

impl InlineParser for LinkifyParser {
    // Go: extension/linkify.go:linkifyParser.Trigger
    fn trigger(&self) -> &[u8] {
        // ' ' indicates any white spaces and a line head
        b" *_~("
    }

    // Go: extension/linkify.go:linkifyParser.Parse
    fn parse<'a>(
        &self,
        ast: &mut Ast,
        parent: NodeId,
        block: &mut dyn Reader<'a>,
        pc: &mut Context,
    ) -> Option<NodeId> {
        if pc.is_in_link_label() {
            return None;
        }
        let (line, segment) = block.peek_line();
        let full_line = line.unwrap_or_default();
        let mut line: &[u8] = &full_line;
        let mut consumes: i64 = 0;
        let mut start = segment.start;
        let c = line[0];
        // advance if current position is not a line head.
        if c == b' ' || c == b'*' || c == b'_' || c == b'~' || c == b'(' {
            consumes += 1;
            start += 1;
            line = &line[1..];
        }

        // m = [m0, m1, m2, m3] (m2/m3 only for emails)
        let mut m: Option<[i64; 4]> = None;
        let mut protocol: Option<Vec<u8>> = None;
        let mut typ = AutoLinkType::Url;
        let found = |r: Option<(usize, usize)>| r.map(|(a, b)| [a as i64, b as i64, 0, 0]);
        match &self.config.allowed_protocols {
            None => {
                if line.starts_with(PROTO_HTTP)
                    || line.starts_with(PROTO_HTTPS)
                    || line.starts_with(PROTO_FTP)
                {
                    m = found(self.config.url_regexp.find_submatch_index(line));
                }
            }
            Some(protocols) => {
                for prefix in protocols {
                    if line.starts_with(prefix) {
                        m = found(self.config.url_regexp.find_submatch_index(line));
                        break;
                    }
                }
            }
        }
        if m.is_none() && line.starts_with(DOMAIN_WWW) {
            m = found(self.config.www_regexp.find_submatch_index(line));
            protocol = Some(b"http".to_vec());
        }
        if let Some(mm) = m
            && mm[0] != 0
        {
            m = None;
        }
        if let Some(mm) = m.as_mut()
            && mm[0] == 0
        {
            let last_char = line[(mm[1] - 1) as usize];
            if last_char == b'.' {
                mm[1] -= 1;
            } else if last_char == b')' {
                let mut closing = 0;
                let mut i = mm[1] - 1;
                while i >= mm[0] {
                    if line[i as usize] == b')' {
                        closing += 1;
                    } else if line[i as usize] == b'(' {
                        closing -= 1;
                    }
                    i -= 1;
                }
                if closing > 0 {
                    mm[1] -= closing;
                }
            } else if last_char == b';' {
                let mut i = mm[1] - 2;
                while i >= mm[0] {
                    if util::is_alpha_numeric(line[i as usize]) {
                        i -= 1;
                        continue;
                    }
                    break;
                }
                if i != mm[1] - 2 {
                    // Go indexes line[i] (i may be -1 here: a panic)
                    assert!(i >= 0, "index out of range [-1]");
                    if line[i as usize] == b'&' {
                        #[allow(clippy::misrefactored_assign_op)] // Go: m[1] -= m[1] - i
                        {
                            mm[1] -= mm[1] - i;
                        }
                    }
                }
            }
        }
        if m.is_none() {
            if !line.is_empty() && util::is_punct(line[0]) {
                return None;
            }
            typ = AutoLinkType::Email;
            let mut stop: i64 = -1;
            match &self.config.email_regexp {
                None => stop = util::find_email_index(line),
                Some(re) => {
                    if let Some((m0, m1)) = re.find_submatch_index(line)
                        && m0 == 0
                    {
                        stop = m1 as i64;
                    }
                }
            }
            if stop < 0 {
                return None;
            }
            let at = go_unicode::bytes::index_byte(line, b'@') as i64;
            let mut mm = [0, stop, at, stop - 1];
            // Go: bytes.IndexByte(line[m[2]:m[3]], '.') < 0 (slicing panics
            // when at == -1 or at > stop-1)
            assert!(
                mm[2] >= 0 && mm[2] <= mm[3],
                "slice bounds out of range [{}:{}]",
                mm[2],
                mm[3]
            );
            if !line[mm[2] as usize..mm[3] as usize].contains(&b'.') {
                return None;
            }
            let last_char = line[(mm[1] - 1) as usize];
            if last_char == b'.' {
                mm[1] -= 1;
            }
            if mm[1] < line.len() as i64 {
                let next_char = line[mm[1] as usize];
                if next_char == b'-' || next_char == b'_' {
                    return None;
                }
            }
            m = Some(mm);
        }
        let mm = m?;
        if consumes != 0 {
            let s = segment.with_stop(segment.start + 1);
            ast::merge_or_append_text_segment(ast, parent, s);
        }
        let mut i = mm[1] - 1;
        while i > 0 {
            let c = line[i as usize];
            match c {
                b'?' | b'!' | b'.' | b',' | b':' | b'*' | b'_' | b'~' => {}
                _ => break,
            }
            i -= 1;
        }
        i += 1;
        consumes += i;
        block.advance(consumes);
        let n = ast::Text::new(new_segment(start, start + i));
        let link = ast.new_auto_link(typ, n);
        ast.auto_link_mut(link).unwrap().protocol = protocol;
        Some(link)
    }

    // Go: extension/linkify.go:linkifyParser.CloseBlock(parent, pc) does not match
    // parser.CloseBlocker (which takes a text.Reader too), so Go never
    // registers it as a close blocker; it is a no-op anyway.

    // Go: the embedded LinkifyConfig's SetOption
    fn set_option(&mut self, name: &str, value: &OptionValue) {
        self.config.set_option(name, value);
    }
}

/// Go: `type linkify struct{ options []LinkifyOption }`.
pub struct LinkifyExt {
    options: Vec<LinkifyOption>,
}

// Go: extension/linkify.go:Linkify
/// Linkify is an extension that allow you to parse text that seems like a URL.
pub fn linkify() -> Box<dyn Extender> {
    Box::new(LinkifyExt {
        options: Vec::new(),
    })
}

// Go: extension/linkify.go:NewLinkify
/// NewLinkify creates a new [`Extender`] that
/// allow you to parse text that seems like a URL.
pub fn new_linkify(opts: Vec<LinkifyOption>) -> Box<dyn Extender> {
    Box::new(LinkifyExt { options: opts })
}

impl Extender for LinkifyExt {
    // Go: extension/linkify.go:linkify.Extend
    fn extend(&self, m: &mut Markdown) {
        m.parser()
            .add_options(vec![parser::with_inline_parsers(vec![util::prioritized(
                new_linkify_parser(&self.options),
                999,
            )])]);
    }
}
