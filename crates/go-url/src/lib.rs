//! Port of Go's `net/url` package (go1.27.1 `src/net/url/url.go`).
//!
//! Go strings are bytes: every URL field and every result is a `Vec<u8>`
//! (a `%ff` escape unescapes to a raw 0xFF byte, and `RawQuery`/`Opaque` are
//! kept verbatim), and every input is `impl AsRef<[u8]>` so `&str` works.
//! Error values implement `Display` with Go's exact `Error()` strings.
//!
//! GODEBUG: the golden neohugo binary is built from a module declaring
//! `go 1.23.0`, so its `DefaultGODEBUG` contains `urlstrictcolons=0` and
//! `urlmaxqueryparams=0`. Both are reproduced here as constants
//! ([`GODEBUG_URLSTRICTCOLONS`], [`GODEBUG_URLMAXQUERYPARAMS`]).

// Lints that fight a faithful line-by-line port of the Go control flow.
#![allow(
    clippy::needless_range_loop,
    clippy::collapsible_if,
    clippy::collapsible_match,
    clippy::needless_late_init
)]

use std::collections::BTreeMap;
use std::fmt;

mod encoding;
pub mod netip;

pub use encoding::encoding_table;
use encoding::{
    ENCODE_FRAGMENT, ENCODE_HOST, ENCODE_PATH, ENCODE_PATH_SEGMENT, ENCODE_QUERY_COMPONENT,
    ENCODE_USER_PASSWORD, ENCODE_ZONE, Encoding, HEX_CHAR, TABLE,
};

/// Value of GODEBUG `urlstrictcolons` in the golden build (`go 1.23.0` module).
pub const GODEBUG_URLSTRICTCOLONS: &str = "0";
/// Value of GODEBUG `urlmaxqueryparams` in the golden build (`go 1.23.0` module).
pub const GODEBUG_URLMAXQUERYPARAMS: &str = "0";

/// A Go `error` as produced by this package.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// Go `*url.Error`: "Op \"URL\": Err".
    Url {
        op: &'static str,
        url: Vec<u8>,
        err: Box<Error>,
    },
    /// Go `url.EscapeError`: "invalid URL escape \"...\"".
    Escape(Vec<u8>),
    /// Go `url.InvalidHostError`: "invalid character \"...\" in host name".
    InvalidHost(Vec<u8>),
    /// Any other error (`errors.New`/`fmt.Errorf` in url.go), already formatted.
    Other(String),
}

impl Error {
    // Go: net/url/url.go:Error.Unwrap
    /// For [`Error::Url`], the wrapped error; `None` otherwise.
    pub fn unwrap_err(&self) -> Option<&Error> {
        match self {
            Error::Url { err, .. } => Some(err),
            _ => None,
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            // Go: net/url/url.go:Error.Error
            Error::Url { op, url, err } => {
                write!(f, "{} {}: {}", op, go_strconv::quote(url), err)
            }
            // Go: net/url/url.go:EscapeError.Error
            Error::Escape(s) => write!(f, "invalid URL escape {}", go_strconv::quote(s)),
            // Go: net/url/url.go:InvalidHostError.Error
            Error::InvalidHost(s) => {
                write!(f, "invalid character {} in host name", go_strconv::quote(s))
            }
            Error::Other(s) => f.write_str(s),
        }
    }
}

impl std::error::Error for Error {}

fn errors_new(s: &str) -> Error {
    Error::Other(s.to_string())
}

const UPPERHEX: &[u8; 16] = b"0123456789ABCDEF";

// Go: net/url/url.go:ishex
fn ishex(c: u8) -> bool {
    TABLE[c as usize] & HEX_CHAR != 0
}

// Go: net/url/url.go:unhex
/// Precondition: ishex(c) is true.
fn unhex(c: u8) -> u8 {
    9u8.wrapping_mul(c >> 6).wrapping_add(c & 15)
}

// Go: net/url/url.go:shouldEscape
fn should_escape(c: u8, mode: Encoding) -> bool {
    TABLE[c as usize] & mode == 0
}

// Go: net/url/url.go:QueryUnescape
/// Converts each 3-byte encoded substring of the form "%AB" into the
/// hex-decoded byte 0xAB, and '+' into ' '.
pub fn query_unescape(s: impl AsRef<[u8]>) -> Result<Vec<u8>, Error> {
    unescape(s.as_ref(), ENCODE_QUERY_COMPONENT)
}

// Go: net/url/url.go:PathUnescape
/// Like [`query_unescape`] but does not unescape '+' to ' '.
pub fn path_unescape(s: impl AsRef<[u8]>) -> Result<Vec<u8>, Error> {
    unescape(s.as_ref(), ENCODE_PATH_SEGMENT)
}

// Go: net/url/url.go:unescape
/// Unescapes a string; the mode specifies which section of the URL string
/// is being unescaped.
fn unescape(s: &[u8], mode: Encoding) -> Result<Vec<u8>, Error> {
    // Count %, check that they're well-formed.
    let mut n = 0usize;
    let mut has_plus = false;
    let mut i = 0usize;
    while i < s.len() {
        match s[i] {
            b'%' => {
                n += 1;
                if i + 2 >= s.len() || !ishex(s[i + 1]) || !ishex(s[i + 2]) {
                    let mut t = &s[i..];
                    if t.len() > 3 {
                        t = &t[..3];
                    }
                    return Err(Error::Escape(t.to_vec()));
                }
                // Per https://tools.ietf.org/html/rfc3986#page-21
                // in the host component %-encoding can only be used
                // for non-ASCII bytes.
                // But https://tools.ietf.org/html/rfc6874#section-2
                // introduces %25 being allowed to escape a percent sign
                // in IPv6 scoped-address literals. Yay.
                if mode == ENCODE_HOST && unhex(s[i + 1]) < 8 && &s[i..i + 3] != b"%25" {
                    return Err(Error::Escape(s[i..i + 3].to_vec()));
                }
                if mode == ENCODE_ZONE {
                    // RFC 6874 says basically "anything goes" for zone identifiers
                    // and that even non-ASCII can be redundantly escaped,
                    // but it seems prudent to restrict %-escaped bytes here to those
                    // that are valid host name bytes in their unescaped form.
                    // That is, you can use escaping in the zone identifier but not
                    // to introduce bytes you couldn't just write directly.
                    // But Windows puts spaces here! Yay.
                    let v = (unhex(s[i + 1]) << 4) | unhex(s[i + 2]);
                    if &s[i..i + 3] != b"%25" && v != b' ' && should_escape(v, ENCODE_HOST) {
                        return Err(Error::Escape(s[i..i + 3].to_vec()));
                    }
                }
                i += 3;
            }
            b'+' => {
                has_plus = mode == ENCODE_QUERY_COMPONENT;
                i += 1;
            }
            _ => {
                if (mode == ENCODE_HOST || mode == ENCODE_ZONE)
                    && s[i] < 0x80
                    && should_escape(s[i], mode)
                {
                    return Err(Error::InvalidHost(s[i..i + 1].to_vec()));
                }
                i += 1;
            }
        }
    }

    if n == 0 && !has_plus {
        return Ok(s.to_vec());
    }

    let unescaped_plus_sign = if mode == ENCODE_QUERY_COMPONENT {
        b' '
    } else {
        b'+'
    };
    let mut t: Vec<u8> = Vec::with_capacity(s.len() - 2 * n);
    let mut i = 0usize;
    while i < s.len() {
        match s[i] {
            b'%' => {
                // In the loop above, we established that unhex's precondition is
                // fulfilled for both s[i+1] and s[i+2].
                t.push((unhex(s[i + 1]) << 4) | unhex(s[i + 2]));
                i += 2;
            }
            b'+' => t.push(unescaped_plus_sign),
            c => t.push(c),
        }
        i += 1;
    }
    Ok(t)
}

// Go: net/url/url.go:QueryEscape
/// Escapes the string so it can be safely placed inside a URL query.
pub fn query_escape(s: impl AsRef<[u8]>) -> Vec<u8> {
    escape(s.as_ref(), ENCODE_QUERY_COMPONENT)
}

// Go: net/url/url.go:PathEscape
/// Escapes the string so it can be safely placed inside a URL path segment,
/// replacing special characters (including /) with %XX sequences as needed.
pub fn path_escape(s: impl AsRef<[u8]>) -> Vec<u8> {
    escape(s.as_ref(), ENCODE_PATH_SEGMENT)
}

// Go: net/url/url.go:escape
fn escape(s: &[u8], mode: Encoding) -> Vec<u8> {
    let (mut space_count, mut hex_count) = (0usize, 0usize);
    for &c in s {
        if should_escape(c, mode) {
            if c == b' ' && mode == ENCODE_QUERY_COMPONENT {
                space_count += 1;
            } else {
                hex_count += 1;
            }
        }
    }

    if space_count == 0 && hex_count == 0 {
        return s.to_vec();
    }

    let required = s.len() + 2 * hex_count;
    let mut t = vec![0u8; required];

    if hex_count == 0 {
        t[..s.len()].copy_from_slice(s);
        for i in 0..s.len() {
            if s[i] == b' ' {
                t[i] = b'+';
            }
        }
        return t;
    }

    let mut j = 0usize;
    for &c in s {
        if c == b' ' && mode == ENCODE_QUERY_COMPONENT {
            t[j] = b'+';
            j += 1;
        } else if should_escape(c, mode) {
            t[j] = b'%';
            t[j + 1] = UPPERHEX[(c >> 4) as usize];
            t[j + 2] = UPPERHEX[(c & 15) as usize];
            j += 3;
        } else {
            t[j] = c;
            j += 1;
        }
    }
    t
}

// Go: net/url/url.go:Userinfo
/// An immutable encapsulation of username and password details for a URL.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Userinfo {
    username: Vec<u8>,
    password: Vec<u8>,
    password_set: bool,
}

// Go: net/url/url.go:User
/// Returns a [`Userinfo`] containing the provided username and no password set.
pub fn user(username: impl AsRef<[u8]>) -> Userinfo {
    Userinfo {
        username: username.as_ref().to_vec(),
        password: Vec::new(),
        password_set: false,
    }
}

// Go: net/url/url.go:UserPassword
/// Returns a [`Userinfo`] containing the provided username and password.
pub fn user_password(username: impl AsRef<[u8]>, password: impl AsRef<[u8]>) -> Userinfo {
    Userinfo {
        username: username.as_ref().to_vec(),
        password: password.as_ref().to_vec(),
        password_set: true,
    }
}

impl Userinfo {
    // Go: net/url/url.go:Userinfo.Username
    pub fn username(&self) -> &[u8] {
        &self.username
    }

    // Go: net/url/url.go:Userinfo.Password
    /// Returns the password in case it is set, and whether it is set.
    pub fn password(&self) -> (&[u8], bool) {
        (&self.password, self.password_set)
    }

    // Go: net/url/url.go:Userinfo.String
    /// Returns the encoded userinfo information in the standard form of
    /// "username[:password]".
    pub fn string(&self) -> Vec<u8> {
        let mut s = escape(&self.username, ENCODE_USER_PASSWORD);
        if self.password_set {
            s.push(b':');
            s.extend_from_slice(&escape(&self.password, ENCODE_USER_PASSWORD));
        }
        s
    }
}

/// Nil-safe helpers mirroring Go methods on a nil `*Userinfo`.
fn ui_username(u: &Option<Userinfo>) -> &[u8] {
    match u {
        None => b"",
        Some(u) => &u.username,
    }
}

fn ui_password(u: &Option<Userinfo>) -> (&[u8], bool) {
    match u {
        None => (b"", false),
        Some(u) => (&u.password, u.password_set),
    }
}

/// A parsed URL (technically, a URI reference): Go `net/url.URL`.
///
/// `[scheme:][//[userinfo@]host][/]path[?query][#fragment]`
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Url {
    pub scheme: Vec<u8>,
    /// encoded opaque data
    pub opaque: Vec<u8>,
    /// username and password information (Go `*Userinfo`, `None` = nil)
    pub user: Option<Userinfo>,
    /// "host" or "host:port" (see hostname and port methods)
    pub host: Vec<u8>,
    /// path (relative paths may omit leading slash)
    pub path: Vec<u8>,
    /// fragment for references (without '#')
    pub fragment: Vec<u8>,
    /// encoded query values, without '?'
    pub raw_query: Vec<u8>,
    /// encoded path hint (see escaped_path method)
    pub raw_path: Vec<u8>,
    /// encoded fragment hint (see escaped_fragment method)
    pub raw_fragment: Vec<u8>,
    /// append a query ('?') even if RawQuery is empty
    pub force_query: bool,
    /// do not emit empty host (authority)
    pub omit_host: bool,
}

// Go: net/url/url.go:getScheme
/// Maybe rawURL is of the form scheme:path.
/// (Scheme must be [a-zA-Z][a-zA-Z0-9+.-]*)
/// If so, return scheme, path; else return "", rawURL.
fn get_scheme(raw_url: &[u8]) -> Result<(&[u8], &[u8]), Error> {
    for i in 0..raw_url.len() {
        let c = raw_url[i];
        if c.is_ascii_lowercase() || c.is_ascii_uppercase() {
            // do nothing
        } else if c.is_ascii_digit() || c == b'+' || c == b'-' || c == b'.' {
            if i == 0 {
                return Ok((b"", raw_url));
            }
        } else if c == b':' {
            if i == 0 {
                return Err(errors_new("missing protocol scheme"));
            }
            return Ok((&raw_url[..i], &raw_url[i + 1..]));
        } else {
            // we have encountered an invalid character,
            // so there is no valid scheme
            return Ok((b"", raw_url));
        }
    }
    Ok((b"", raw_url))
}

fn index_of(s: &[u8], sep: &[u8]) -> Option<usize> {
    if sep.is_empty() {
        return Some(0);
    }
    s.windows(sep.len()).position(|w| w == sep)
}

fn last_index_of(s: &[u8], sep: &[u8]) -> Option<usize> {
    if sep.is_empty() {
        return Some(s.len());
    }
    s.windows(sep.len()).rposition(|w| w == sep)
}

// Go: strings.Cut
fn cut<'a>(s: &'a [u8], sep: &[u8]) -> (&'a [u8], &'a [u8], bool) {
    match index_of(s, sep) {
        Some(i) => (&s[..i], &s[i + sep.len()..], true),
        None => (s, b"", false),
    }
}

// Go: net/url/url.go:Parse
/// Parses a raw url into a [`Url`] structure. The url may be relative (a
/// path, without a host) or absolute (starting with a scheme).
pub fn parse(raw_url: impl AsRef<[u8]>) -> Result<Url, Error> {
    let raw_url = raw_url.as_ref();
    // Cut off #frag
    let (u, frag, _) = cut(raw_url, b"#");
    let mut url = match parse_internal(u, false) {
        Ok(url) => url,
        Err(err) => {
            return Err(Error::Url {
                op: "parse",
                url: u.to_vec(),
                err: Box::new(err),
            });
        }
    };
    if frag.is_empty() {
        return Ok(url);
    }
    if let Err(err) = url.set_fragment(frag) {
        return Err(Error::Url {
            op: "parse",
            url: raw_url.to_vec(),
            err: Box::new(err),
        });
    }
    Ok(url)
}

// Go: net/url/url.go:ParseRequestURI
/// Parses a raw url into a [`Url`] structure, assuming it was received in an
/// HTTP request (absolute URI or absolute path, no #fragment).
pub fn parse_request_uri(raw_url: impl AsRef<[u8]>) -> Result<Url, Error> {
    let raw_url = raw_url.as_ref();
    parse_internal(raw_url, true).map_err(|err| Error::Url {
        op: "parse",
        url: raw_url.to_vec(),
        err: Box::new(err),
    })
}

// Go: net/url/url.go:parse
/// Parses a URL from a string in one of two contexts. If viaRequest is true,
/// the URL is assumed to have arrived via an HTTP request, in which case only
/// absolute URLs or path-absolute relative URLs are allowed. If viaRequest is
/// false, all forms of relative URLs are allowed.
fn parse_internal(raw_url: &[u8], via_request: bool) -> Result<Url, Error> {
    if string_contains_ctl_byte(raw_url) {
        return Err(errors_new("net/url: invalid control character in URL"));
    }

    if raw_url.is_empty() && via_request {
        return Err(errors_new("empty url"));
    }
    let mut url = Url::default();

    if raw_url == b"*" {
        url.path = b"*".to_vec();
        return Ok(url);
    }

    // Split off possible leading "http:", "mailto:", etc.
    // Cannot contain escaped characters.
    let (scheme, mut rest) = get_scheme(raw_url)?;
    // strings.ToLower: the scheme is ASCII-only here (validated by getScheme).
    url.scheme = scheme.to_ascii_lowercase();

    if rest.ends_with(b"?") && rest.iter().filter(|&&c| c == b'?').count() == 1 {
        url.force_query = true;
        rest = &rest[..rest.len() - 1];
    } else {
        let (r, q, _) = cut(rest, b"?");
        rest = r;
        url.raw_query = q.to_vec();
    }

    if !rest.starts_with(b"/") {
        if !url.scheme.is_empty() {
            // We consider rootless paths per RFC 3986 as opaque.
            url.opaque = rest.to_vec();
            return Ok(url);
        }
        if via_request {
            return Err(errors_new("invalid URI for request"));
        }

        // Avoid confusion with malformed schemes, like cache_object:foo/bar.
        // See golang.org/issue/16822.
        //
        // RFC 3986, §3.3:
        // In addition, a URI reference (Section 4.1) may be a relative-path reference,
        // in which case the first path segment cannot contain a colon (":") character.
        let (segment, _, _) = cut(rest, b"/");
        if segment.contains(&b':') {
            // First path segment has colon. Not allowed in relative URL.
            return Err(errors_new("first path segment in URL cannot contain colon"));
        }
    }

    if (!url.scheme.is_empty() || !via_request && !rest.starts_with(b"///"))
        && rest.starts_with(b"//")
    {
        let mut authority = &rest[2..];
        rest = b"";
        if let Some(i) = index_of(authority, b"/") {
            rest = &authority[i..];
            authority = &authority[..i];
        }
        let (user, host) = parse_authority(&url.scheme, authority)?;
        url.user = user;
        url.host = host;
    } else if !url.scheme.is_empty() && rest.starts_with(b"/") {
        // OmitHost is set to true when rawURL has an empty host (authority).
        // See golang.org/issue/46059.
        url.omit_host = true;
    }

    // Set Path and, optionally, RawPath.
    // RawPath is a hint of the encoding of Path. We don't want to set it if
    // the default escaping of Path is equivalent, to help make sure that people
    // don't rely on it in general.
    url.set_path(rest)?;
    Ok(url)
}

// Go: net/url/url.go:parseAuthority
fn parse_authority(scheme: &[u8], authority: &[u8]) -> Result<(Option<Userinfo>, Vec<u8>), Error> {
    let i = last_index_of(authority, b"@");
    let host = match i {
        None => parse_host(scheme, authority)?,
        Some(i) => parse_host(scheme, &authority[i + 1..])?,
    };
    let Some(i) = i else {
        return Ok((None, host));
    };
    let userinfo = &authority[..i];
    if !valid_userinfo(userinfo) {
        return Err(errors_new("net/url: invalid userinfo"));
    }
    let user_v;
    if !userinfo.contains(&b':') {
        let u = unescape(userinfo, ENCODE_USER_PASSWORD)?;
        user_v = user(u);
    } else {
        let (username, password, _) = cut(userinfo, b":");
        let username = unescape(username, ENCODE_USER_PASSWORD)?;
        let password = unescape(password, ENCODE_USER_PASSWORD)?;
        user_v = user_password(username, password);
    }
    Ok((Some(user_v), host))
}

// Go: net/url/url.go:parseHost
/// Parses host as an authority without user information. That is, as
/// host[:port].
fn parse_host(scheme: &[u8], host: &[u8]) -> Result<Vec<u8>, Error> {
    let open_bracket_idx = last_index_of(host, b"[");
    if matches!(open_bracket_idx, Some(i) if i > 0) {
        return Err(errors_new("invalid IP-literal"));
    } else if open_bracket_idx == Some(0) {
        // Parse an IP-Literal in RFC 3986 and RFC 6874.
        // E.g., "[fe80::1]", "[fe80::1%25en0]", "[fe80::1]:80".
        let Some(close_bracket_idx) = last_index_of(host, b"]") else {
            return Err(errors_new("missing ']' in host"));
        };

        let colon_port = &host[close_bracket_idx + 1..];
        if !valid_optional_port(colon_port) {
            return Err(Error::Other(format!(
                "invalid port {} after host",
                go_strconv::quote(colon_port)
            )));
        }
        let unescaped_colon_port = unescape(colon_port, ENCODE_HOST)?;

        // host[0] == '[', so closeBracketIdx >= 1 and the range is valid.
        let hostname = &host[1..close_bracket_idx];
        let mut unescaped_hostname;
        // RFC 6874 defines that %25 (%-encoded percent) introduces
        // the zone identifier, and the zone identifier can use basically
        // any %-encoding it likes. That's different from the host, which
        // can only %-encode non-ASCII bytes.
        // We do impose some restrictions on the zone, to avoid stupidity
        // like newlines.
        if let Some(zone_idx) = index_of(hostname, b"%25") {
            let host_part = unescape(&hostname[..zone_idx], ENCODE_HOST)?;
            let zone_part = unescape(&hostname[zone_idx..], ENCODE_ZONE)?;
            unescaped_hostname = host_part;
            unescaped_hostname.extend_from_slice(&zone_part);
        } else {
            unescaped_hostname = unescape(hostname, ENCODE_HOST)?;
        }

        // Per RFC 3986, only a host identified by a valid
        // IPv6 address can be enclosed by square brackets.
        // This excludes any IPv4, but notably not IPv4-mapped addresses.
        let addr = match netip::parse_addr(&unescaped_hostname) {
            Ok(a) => a,
            Err(e) => return Err(Error::Other(format!("invalid host: {e}"))),
        };
        if addr.is4() {
            return Err(errors_new("invalid IP-literal"));
        }
        let mut out = Vec::with_capacity(unescaped_hostname.len() + 2 + unescaped_colon_port.len());
        out.push(b'[');
        out.extend_from_slice(&unescaped_hostname);
        out.push(b']');
        out.extend_from_slice(&unescaped_colon_port);
        return Ok(out);
    } else if let Some(mut i) = index_of(host, b":") {
        let last_colon = last_index_of(host, b":").expect("contains ':'");
        if last_colon != i {
            // RFC 3986 does not allow colons to appear in the host subcomponent.
            //
            // However, a number of databases including PostgreSQL and MongoDB
            // permit a comma-separated list of hosts (with optional ports) in the
            // host subcomponent.
            //
            // Since we historically permitted colons to appear in the host,
            // enforce strict colons only for http and https URLs.
            //
            // See https://go.dev/issue/75223 and https://go.dev/issue/78077.
            if scheme == b"http" || scheme == b"https" {
                if GODEBUG_URLSTRICTCOLONS == "0" {
                    i = last_colon;
                }
            } else {
                i = last_colon;
            }
        }
        let colon_port = &host[i..];
        if !valid_optional_port(colon_port) {
            return Err(Error::Other(format!(
                "invalid port {} after host",
                go_strconv::quote(colon_port)
            )));
        }
    }

    unescape(host, ENCODE_HOST)
}

impl Url {
    // Go: net/url/url.go:URL.setPath
    /// Sets the Path and RawPath fields of the URL based on the provided
    /// escaped path p. It maintains the invariant that RawPath is only
    /// specified when it differs from the default encoding of the path.
    /// Returns an error only if p contains an invalid escaping (then the URL
    /// is left unchanged). Unexported in Go; public here for Hugo ports.
    pub fn set_path(&mut self, p: impl AsRef<[u8]>) -> Result<(), Error> {
        let p = p.as_ref();
        let path = unescape(p, ENCODE_PATH)?;
        let escp = escape(&path, ENCODE_PATH);
        self.path = path;
        if p == escp.as_slice() {
            // Default encoding is fine.
            self.raw_path = Vec::new();
        } else {
            self.raw_path = p.to_vec();
        }
        Ok(())
    }

    // Go: net/url/url.go:URL.EscapedPath
    /// Returns the escaped form of u.Path: u.RawPath when it is a valid
    /// escaping of u.Path, otherwise a computed escaping.
    pub fn escaped_path(&self) -> Vec<u8> {
        if !self.raw_path.is_empty() && valid_encoded(&self.raw_path, ENCODE_PATH) {
            if let Ok(p) = unescape(&self.raw_path, ENCODE_PATH) {
                if p == self.path {
                    return self.raw_path.clone();
                }
            }
        }
        if self.path == b"*" {
            return b"*".to_vec(); // don't escape (Issue 11202)
        }
        escape(&self.path, ENCODE_PATH)
    }

    // Go: net/url/url.go:URL.setFragment
    /// Like set_path but for Fragment/RawFragment.
    pub fn set_fragment(&mut self, f: impl AsRef<[u8]>) -> Result<(), Error> {
        let f = f.as_ref();
        let frag = unescape(f, ENCODE_FRAGMENT)?;
        let escf = escape(&frag, ENCODE_FRAGMENT);
        self.fragment = frag;
        if f == escf.as_slice() {
            // Default encoding is fine.
            self.raw_fragment = Vec::new();
        } else {
            self.raw_fragment = f.to_vec();
        }
        Ok(())
    }

    // Go: net/url/url.go:URL.EscapedFragment
    /// Returns the escaped form of u.Fragment.
    pub fn escaped_fragment(&self) -> Vec<u8> {
        if !self.raw_fragment.is_empty() && valid_encoded(&self.raw_fragment, ENCODE_FRAGMENT) {
            if let Ok(f) = unescape(&self.raw_fragment, ENCODE_FRAGMENT) {
                if f == self.fragment {
                    return self.raw_fragment.clone();
                }
            }
        }
        escape(&self.fragment, ENCODE_FRAGMENT)
    }

    // Go: net/url/url.go:URL.String
    /// Reassembles the URL into a valid URL string (Go `URL.String()`).
    pub fn string(&self) -> Vec<u8> {
        let mut buf: Vec<u8> = Vec::new();

        if !self.scheme.is_empty() {
            buf.extend_from_slice(&self.scheme);
            buf.push(b':');
        }
        if !self.opaque.is_empty() {
            buf.extend_from_slice(&self.opaque);
        } else {
            if !self.scheme.is_empty() || !self.host.is_empty() || self.user.is_some() {
                if self.omit_host && self.host.is_empty() && self.user.is_none() {
                    // omit empty host
                } else {
                    if !self.host.is_empty() || !self.path.is_empty() || self.user.is_some() {
                        buf.extend_from_slice(b"//");
                    }
                    if let Some(ui) = &self.user {
                        buf.extend_from_slice(&ui.string());
                        buf.push(b'@');
                    }
                    if !self.host.is_empty() {
                        buf.extend_from_slice(&escape(&self.host, ENCODE_HOST));
                    }
                }
            }
            let path_v = self.escaped_path();
            let mut path: &[u8] = &path_v;
            if self.omit_host
                && self.host.is_empty()
                && self.user.is_none()
                && path.starts_with(b"//")
            {
                // Escape the first / in a path starting with "//" and no authority
                // so that re-parsing the URL doesn't turn the path into an authority
                // (e.g., Path="//host/p" producing "http://host/p").
                buf.extend_from_slice(b"%2F");
                path = &path[1..];
            }
            if !path.is_empty() && path[0] != b'/' && !self.host.is_empty() {
                buf.push(b'/');
            }
            if buf.is_empty() {
                // RFC 3986 §4.2
                // A path segment that contains a colon character (e.g., "this:that")
                // cannot be used as the first segment of a relative-path reference, as
                // it would be mistaken for a scheme name. Such a segment must be
                // preceded by a dot-segment (e.g., "./this:that") to make a relative-
                // path reference.
                let (segment, _, _) = cut(path, b"/");
                if segment.contains(&b':') {
                    buf.extend_from_slice(b"./");
                }
            }
            buf.extend_from_slice(path);
        }
        if self.force_query || !self.raw_query.is_empty() {
            buf.push(b'?');
            buf.extend_from_slice(&self.raw_query);
        }
        if !self.fragment.is_empty() {
            buf.push(b'#');
            buf.extend_from_slice(&self.escaped_fragment());
        }
        buf
    }

    // Go: net/url/url.go:URL.Redacted
    /// Like [`Url::string`] but replaces any password with "xxxxx".
    pub fn redacted(&self) -> Vec<u8> {
        let mut ru = self.clone();
        if ui_password(&ru.user).1 {
            ru.user = Some(user_password(ui_username(&ru.user), b"xxxxx"));
        }
        ru.string()
    }

    // Go: net/url/url.go:URL.IsAbs
    /// Reports whether the URL is absolute (has a non-empty scheme).
    pub fn is_abs(&self) -> bool {
        !self.scheme.is_empty()
    }

    // Go: net/url/url.go:URL.Parse
    /// Parses a URL in the context of the receiver.
    pub fn parse(&self, r#ref: impl AsRef<[u8]>) -> Result<Url, Error> {
        let ref_url = parse(r#ref)?;
        Ok(self.resolve_reference(&ref_url))
    }

    // Go: net/url/url.go:URL.ResolveReference
    /// Resolves a URI reference to an absolute URI from an absolute base URI
    /// u, per RFC 3986 Section 5.2.
    pub fn resolve_reference(&self, r#ref: &Url) -> Url {
        let u = self;
        let mut url = r#ref.clone();
        if r#ref.scheme.is_empty() {
            url.scheme = u.scheme.clone();
        }
        if !r#ref.scheme.is_empty() || !r#ref.host.is_empty() || r#ref.user.is_some() {
            // The "absoluteURI" or "net_path" cases.
            // We can ignore the error from setPath since we know we provided a
            // validly-escaped path.
            let _ = url.set_path(resolve_path(&r#ref.escaped_path(), b""));
            return url;
        }
        if !r#ref.opaque.is_empty() {
            url.user = None;
            url.host = Vec::new();
            url.path = Vec::new();
            return url;
        }
        if r#ref.path.is_empty() && !r#ref.force_query && r#ref.raw_query.is_empty() {
            url.raw_query = u.raw_query.clone();
            if r#ref.fragment.is_empty() {
                url.fragment = u.fragment.clone();
                url.raw_fragment = u.raw_fragment.clone();
            }
        }
        if r#ref.path.is_empty() && !u.opaque.is_empty() {
            url.opaque = u.opaque.clone();
            url.user = None;
            url.host = Vec::new();
            url.path = Vec::new();
            return url;
        }
        // The "abs_path" or "rel_path" cases.
        url.host = u.host.clone();
        url.user = u.user.clone();
        let _ = url.set_path(resolve_path(&u.escaped_path(), &r#ref.escaped_path()));
        url
    }

    // Go: net/url/url.go:URL.Query
    /// Parses RawQuery and returns the corresponding values, silently
    /// discarding malformed value pairs.
    pub fn query(&self) -> Values {
        let (v, _) = parse_query(&self.raw_query);
        v
    }

    // Go: net/url/url.go:URL.RequestURI
    /// Returns the encoded path?query or opaque?query string that would be
    /// used in an HTTP request for u.
    pub fn request_uri(&self) -> Vec<u8> {
        let mut result = self.opaque.clone();
        if result.is_empty() {
            result = self.escaped_path();
            if result.is_empty() {
                result = b"/".to_vec();
            }
        } else if result.starts_with(b"//") {
            let mut r = self.scheme.clone();
            r.push(b':');
            r.extend_from_slice(&result);
            result = r;
        }
        if self.force_query || !self.raw_query.is_empty() {
            result.push(b'?');
            result.extend_from_slice(&self.raw_query);
        }
        result
    }

    // Go: net/url/url.go:URL.Hostname
    /// Returns u.Host, stripping any valid port number if present, and the
    /// square brackets of an IPv6 literal.
    pub fn hostname(&self) -> &[u8] {
        split_host_port(&self.host).0
    }

    // Go: net/url/url.go:URL.Port
    /// Returns the port part of u.Host, without the leading colon.
    pub fn port(&self) -> &[u8] {
        split_host_port(&self.host).1
    }

    // Go: net/url/url.go:URL.MarshalBinary
    pub fn marshal_binary(&self) -> Vec<u8> {
        self.append_binary(Vec::new())
    }

    // Go: net/url/url.go:URL.AppendBinary
    pub fn append_binary(&self, mut b: Vec<u8>) -> Vec<u8> {
        b.extend_from_slice(&self.string());
        b
    }

    // Go: net/url/url.go:URL.UnmarshalBinary
    pub fn unmarshal_binary(&mut self, text: &[u8]) -> Result<(), Error> {
        let u1 = parse(text)?;
        *self = u1;
        Ok(())
    }

    // Go: net/url/url.go:URL.JoinPath
    /// Returns a new URL with the provided path elements joined to any
    /// existing path and the resulting path cleaned of any ./ or ../
    /// elements. Path elements must already be in escaped form.
    pub fn join_path<S: AsRef<[u8]>>(&self, elem: &[S]) -> Url {
        self.join_path_internal(elem).0
    }

    // Go: net/url/url.go:URL.joinPath
    fn join_path_internal<S: AsRef<[u8]>>(&self, elem: &[S]) -> (Url, Result<(), Error>) {
        let mut elems: Vec<Vec<u8>> = Vec::with_capacity(elem.len() + 1);
        elems.push(self.escaped_path());
        for e in elem {
            elems.push(e.as_ref().to_vec());
        }
        let mut p: Vec<u8>;
        if !elems[0].starts_with(b"/") {
            // Return a relative path if u is relative,
            // but ensure that it contains no ../ elements.
            elems[0].insert(0, b'/');
            p = go_path::path::join_bytes(&elems)[1..].to_vec();
        } else {
            p = go_path::path::join_bytes(&elems);
        }
        // path.Join will remove any trailing slashes.
        // Preserve at least one.
        if elems[elems.len() - 1].ends_with(b"/") && !p.ends_with(b"/") {
            p.push(b'/');
        }
        let mut url = self.clone();
        let err = url.set_path(&p);
        (url, err)
    }
}

// Go: net/url/url.go:validEncoded
/// Reports whether s is a valid encoded path or fragment, according to mode.
/// It must not contain any bytes that require escaping during encoding.
fn valid_encoded(s: &[u8], mode: Encoding) -> bool {
    for &c in s {
        // RFC 3986, Appendix A.
        // pchar = unreserved / pct-encoded / sub-delims / ":" / "@".
        // shouldEscape is not quite compliant with the RFC,
        // so we check the sub-delims ourselves and let
        // shouldEscape handle the others.
        match c {
            b'!' | b'$' | b'&' | b'\'' | b'(' | b')' | b'*' | b'+' | b',' | b';' | b'=' | b':'
            | b'@' => {
                // ok
            }
            b'[' | b']' => {
                // ok - not specified in RFC 3986 but left alone by modern browsers
            }
            b'%' => {
                // ok - percent encoded, will decode
            }
            _ => {
                if should_escape(c, mode) {
                    return false;
                }
            }
        }
    }
    true
}

// Go: net/url/url.go:validOptionalPort
/// Reports whether port is either an empty string or matches /^:\d*$/.
fn valid_optional_port(port: &[u8]) -> bool {
    if port.is_empty() {
        return true;
    }
    if port[0] != b':' {
        return false;
    }
    for &b in &port[1..] {
        if !b.is_ascii_digit() {
            return false;
        }
    }
    true
}

/// Go `url.Values`: maps a string key to a list of values. Keys are kept in
/// byte order (Go sorts keys where order is observable, e.g. `Encode`).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Values(pub BTreeMap<Vec<u8>, Vec<Vec<u8>>>);

impl Values {
    pub fn new() -> Self {
        Values(BTreeMap::new())
    }

    // Go: net/url/url.go:Values.Get
    /// Gets the first value associated with the given key, or "".
    pub fn get(&self, key: impl AsRef<[u8]>) -> &[u8] {
        match self.0.get(key.as_ref()) {
            Some(vs) if !vs.is_empty() => &vs[0],
            _ => b"",
        }
    }

    // Go: net/url/url.go:Values.Set
    pub fn set(&mut self, key: impl AsRef<[u8]>, value: impl AsRef<[u8]>) {
        self.0
            .insert(key.as_ref().to_vec(), vec![value.as_ref().to_vec()]);
    }

    // Go: net/url/url.go:Values.Add
    pub fn add(&mut self, key: impl AsRef<[u8]>, value: impl AsRef<[u8]>) {
        self.0
            .entry(key.as_ref().to_vec())
            .or_default()
            .push(value.as_ref().to_vec());
    }

    // Go: net/url/url.go:Values.Del
    pub fn del(&mut self, key: impl AsRef<[u8]>) {
        self.0.remove(key.as_ref());
    }

    // Go: net/url/url.go:Values.Has
    pub fn has(&self, key: impl AsRef<[u8]>) -> bool {
        self.0.contains_key(key.as_ref())
    }

    // Go: net/url/url.go:Values.Encode
    /// Encodes the values into "URL encoded" form ("bar=baz&foo=quux")
    /// sorted by key.
    pub fn encode(&self) -> Vec<u8> {
        if self.0.is_empty() {
            return Vec::new();
        }
        let mut buf: Vec<u8> = Vec::new();
        // BTreeMap iteration order == slices.Sort(keys) (bytewise).
        for (k, vs) in &self.0 {
            let key_escaped = query_escape(k);
            for v in vs {
                if !buf.is_empty() {
                    buf.push(b'&');
                }
                buf.extend_from_slice(&key_escaped);
                buf.push(b'=');
                buf.extend_from_slice(&query_escape(v));
            }
        }
        buf
    }
}

// Go: net/url/url.go:ParseQuery
/// Parses the URL-encoded query string and returns a map listing the values
/// specified for each key, plus the first decoding error encountered, if any.
pub fn parse_query(query: impl AsRef<[u8]>) -> (Values, Option<Error>) {
    let mut m = Values::new();
    let err = parse_query_into(&mut m, query.as_ref());
    (m, err)
}

// Go: net/url/url.go:defaultMaxParams
const DEFAULT_MAX_PARAMS: usize = 10000;

// Go: net/url/url.go:urlParamsWithinMax
fn url_params_within_max(params: usize) -> bool {
    let within_default_max = params <= DEFAULT_MAX_PARAMS;
    if GODEBUG_URLMAXQUERYPARAMS.is_empty() {
        return within_default_max;
    }
    let custom_max: usize = match GODEBUG_URLMAXQUERYPARAMS.parse() {
        Ok(v) => v,
        Err(_) => return within_default_max,
    };
    custom_max == 0 || params < custom_max
}

// Go: net/url/url.go:parseQuery
fn parse_query_into(m: &mut Values, query: &[u8]) -> Option<Error> {
    let mut err: Option<Error> = None;
    if !url_params_within_max(query.iter().filter(|&&c| c == b'&').count() + 1) {
        return Some(errors_new("number of URL query parameters exceeded limit"));
    }
    let mut query = query;
    while !query.is_empty() {
        let (key, rest, _) = cut(query, b"&");
        query = rest;
        if key.contains(&b';') {
            err = Some(errors_new("invalid semicolon separator in query"));
            continue;
        }
        if key.is_empty() {
            continue;
        }
        let (key, value, _) = cut(key, b"=");
        let key = match query_unescape(key) {
            Ok(k) => k,
            Err(err1) => {
                if err.is_none() {
                    err = Some(err1);
                }
                continue;
            }
        };
        let value = match query_unescape(value) {
            Ok(v) => v,
            Err(err1) => {
                if err.is_none() {
                    err = Some(err1);
                }
                continue;
            }
        };
        m.0.entry(key).or_default().push(value);
    }
    err
}

// Go: net/url/url.go:resolvePath
/// Applies special path segments from refs and applies them to base, per
/// RFC 3986.
fn resolve_path(base: &[u8], r#ref: &[u8]) -> Vec<u8> {
    let full: Vec<u8>;
    if r#ref.is_empty() {
        full = base.to_vec();
    } else if r#ref[0] != b'/' {
        let i = match last_index_of(base, b"/") {
            Some(i) => i as isize,
            None => -1,
        };
        let mut f = base[..(i + 1) as usize].to_vec();
        f.extend_from_slice(r#ref);
        full = f;
    } else {
        full = r#ref.to_vec();
    }
    if full.is_empty() {
        return Vec::new();
    }

    let mut dst: Vec<u8> = Vec::with_capacity(full.len() + 1);
    dst.push(b'/');
    let mut elem: &[u8] = b"";
    let mut remaining: &[u8] = &full;
    let mut found = true;
    let mut first = true;
    while found {
        let (e, r, f) = cut(remaining, b"/");
        elem = e;
        remaining = r;
        found = f;
        match elem {
            b"." => {
                first = false;
                continue;
            }
            b".." => {
                match dst[1..].iter().rposition(|&c| c == b'/') {
                    Some(i) => dst.truncate(i + 1),
                    None => dst.truncate(1),
                }
                first = dst.len() == 1;
            }
            _ => {
                if !first {
                    dst.push(b'/');
                }
                dst.extend_from_slice(elem);
                first = false;
            }
        }
    }

    if elem == b"." || elem == b".." {
        dst.push(b'/');
    }

    // We wrote an initial '/', but we don't want two.
    if dst.len() > 1 && dst[1] == b'/' {
        return dst[1..].to_vec();
    }
    dst
}

// Go: net/url/url.go:splitHostPort
/// Separates host and port. If the port is not valid, it returns the entire
/// input as host, and it doesn't check the validity of the host.
fn split_host_port(host_port: &[u8]) -> (&[u8], &[u8]) {
    let mut host = host_port;
    let mut port: &[u8] = b"";

    if let Some(colon) = host.iter().rposition(|&c| c == b':') {
        if valid_optional_port(&host[colon..]) {
            port = &host[colon + 1..];
            host = &host[..colon];
        }
    }

    if host.starts_with(b"[") && host.ends_with(b"]") {
        // Both hold only if len(host) >= 2.
        host = &host[1..host.len() - 1];
    }

    (host, port)
}

// Go: net/url/url.go:validUserinfo
/// Reports whether s is a valid userinfo string per RFC 3986 Section 3.2.1.
/// Go ranges over runes; every non-ASCII rune (and RuneError for invalid
/// bytes) hits the `default` case, so checking bytes is equivalent.
fn valid_userinfo(s: &[u8]) -> bool {
    for &r in s {
        if r.is_ascii_uppercase() || r.is_ascii_lowercase() || r.is_ascii_digit() {
            continue;
        }
        match r {
            b'-' | b'.' | b'_' | b':' | b'~' | b'!' | b'$' | b'&' | b'\'' | b'(' | b')' | b'*'
            | b'+' | b',' | b';' | b'=' | b'%' => continue,
            b'@' => {
                // `RFC 3986 section 3.2.1` does not allow '@' in userinfo.
                // It is a delimiter between userinfo and host.
                // However, URLs are diverse, and in some cases,
                // the userinfo may contain an '@' character,
                // for example, in "http://username:p@ssword@google.com",
                // the string "username:p@ssword" should be treated as valid userinfo.
                // Ref:
                //   https://go.dev/issue/3439
                //   https://go.dev/issue/22655
                continue;
            }
            _ => return false,
        }
    }
    true
}

// Go: net/url/url.go:stringContainsCTLByte
/// Reports whether s contains any ASCII control character.
fn string_contains_ctl_byte(s: &[u8]) -> bool {
    s.iter().any(|&b| b < b' ' || b == 0x7f)
}

// Go: net/url/url.go:JoinPath
/// Returns a URL string with the provided path elements joined to the
/// existing path of base and the resulting path cleaned of any ./ or ../
/// elements. Path elements must already be in escaped form.
pub fn join_path<S: AsRef<[u8]>>(base: impl AsRef<[u8]>, elem: &[S]) -> Result<Vec<u8>, Error> {
    let url = parse(base)?;
    let (res, err) = url.join_path_internal(elem);
    err?;
    Ok(res.string())
}

#[cfg(test)]
mod go_internal_tests {
    //! Ports of the go1.27.1 `net/url/url_test.go` tables that exercise
    //! unexported functions (the oracle fixtures only reach them through the
    //! public API).
    use super::*;

    // Go: net/url/url_test.go:resolvePathTests + TestResolvePath
    #[test]
    fn resolve_path_tests() {
        let tests: &[(&str, &str, &str)] = &[
            ("a/b", ".", "/a/"),
            ("a/b", "c", "/a/c"),
            ("a/b", "..", "/"),
            ("a/", "..", "/"),
            ("a/", "../..", "/"),
            ("a/b/c", "..", "/a/"),
            ("a/b/c", "../d", "/a/d"),
            ("a/b/c", ".././d", "/a/d"),
            ("a/b", "./..", "/"),
            ("a/./b", ".", "/a/"),
            ("a/../", ".", "/"),
            ("a/.././b", "c", "/c"),
        ];
        for &(base, r, expected) in tests {
            let got = resolve_path(base.as_bytes(), r.as_bytes());
            assert_eq!(got, expected.as_bytes(), "For {base:?} + {r:?}");
        }
    }

    // Go: net/url/url_test.go:shouldEscapeTests + TestShouldEscape
    #[test]
    fn should_escape_tests() {
        let tests: &[(u8, Encoding, bool)] = &[
            // Unreserved characters (§2.3)
            (b'a', ENCODE_PATH, false),
            (b'a', ENCODE_USER_PASSWORD, false),
            (b'a', ENCODE_QUERY_COMPONENT, false),
            (b'a', ENCODE_FRAGMENT, false),
            (b'a', ENCODE_HOST, false),
            (b'z', ENCODE_PATH, false),
            (b'A', ENCODE_PATH, false),
            (b'Z', ENCODE_PATH, false),
            (b'0', ENCODE_PATH, false),
            (b'9', ENCODE_PATH, false),
            (b'-', ENCODE_PATH, false),
            (b'-', ENCODE_USER_PASSWORD, false),
            (b'-', ENCODE_QUERY_COMPONENT, false),
            (b'-', ENCODE_FRAGMENT, false),
            (b'.', ENCODE_PATH, false),
            (b'_', ENCODE_PATH, false),
            (b'~', ENCODE_PATH, false),
            // User information (§3.2.1)
            (b':', ENCODE_USER_PASSWORD, true),
            (b'/', ENCODE_USER_PASSWORD, true),
            (b'?', ENCODE_USER_PASSWORD, true),
            (b'@', ENCODE_USER_PASSWORD, true),
            (b'$', ENCODE_USER_PASSWORD, false),
            (b'&', ENCODE_USER_PASSWORD, false),
            (b'+', ENCODE_USER_PASSWORD, false),
            (b',', ENCODE_USER_PASSWORD, false),
            (b';', ENCODE_USER_PASSWORD, false),
            (b'=', ENCODE_USER_PASSWORD, false),
            // Host (IP address, IPv6 address, registered name, port suffix; §3.2.2)
            (b'!', ENCODE_HOST, false),
            (b'$', ENCODE_HOST, false),
            (b'&', ENCODE_HOST, false),
            (b'\'', ENCODE_HOST, false),
            (b'(', ENCODE_HOST, false),
            (b')', ENCODE_HOST, false),
            (b'*', ENCODE_HOST, false),
            (b'+', ENCODE_HOST, false),
            (b',', ENCODE_HOST, false),
            (b';', ENCODE_HOST, false),
            (b'=', ENCODE_HOST, false),
            (b':', ENCODE_HOST, false),
            (b'[', ENCODE_HOST, false),
            (b']', ENCODE_HOST, false),
            (b'0', ENCODE_HOST, false),
            (b'9', ENCODE_HOST, false),
            (b'A', ENCODE_HOST, false),
            (b'z', ENCODE_HOST, false),
            (b'_', ENCODE_HOST, false),
            (b'-', ENCODE_HOST, false),
            (b'.', ENCODE_HOST, false),
        ];
        for &(c, mode, want) in tests {
            assert_eq!(
                should_escape(c, mode),
                want,
                "shouldEscape({:?}, {mode})",
                c as char
            );
        }
    }

    // Go: net/url/url_test.go:TestParseStrictIpv6 (GODEBUG urlstrictcolons=0,
    // the golden build's default).
    #[test]
    fn parse_strict_ipv6_tests() {
        for u in [
            "https://1:2:3:4:5:6:7:8",
            "https://1:2:3:4:5:6:7:8:80",
            "https://example.com:80:",
        ] {
            assert!(parse(u).is_ok(), "Parse({u:?})");
        }
    }

    // Go: net/url/url_test.go:TestParseQueryLimits, rows that apply to the
    // golden build's GODEBUG urlmaxqueryparams=0 (no limit).
    #[test]
    fn parse_query_limits_tests() {
        for params in [10usize, DEFAULT_MAX_PARAMS, DEFAULT_MAX_PARAMS + 1] {
            let mut q = Vec::new();
            for i in 0..params {
                if i > 0 {
                    q.push(b'&');
                }
                q.extend_from_slice(format!("p{i}").as_bytes());
            }
            let (v, err) = parse_query(&q);
            assert!(err.is_none(), "ParseQuery({params} params)");
            assert_eq!(v.0.len(), params);
            assert!(v.0.values().all(|vs| vs == &[Vec::<u8>::new()]));
        }
    }
}
