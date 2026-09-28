//! Go `net/http` `ReadResponse` + `net/textproto` header reader subset (go1.27.1): enough to
//! parse the `httputil.DumpResponse` files of the getresource file cache exactly like Go,
//! including Go's error texts for malformed entries.
//!
//! Owner: Wave B task T08 (helpers-source-cache).
//!
//! Go reads the dump through a `bufio.Reader` (default size 4096) and reads the body lazily;
//! [`read_response`] reads the whole body right away (`io.ReadAll(resp.Body)`) and keeps its error
//! in [`Response::body_err`]. Go's buffering is modelled where it changes a result: the chunk
//! size line must end within 4096 buffered bytes (`header line too long`), and a trailer's blank
//! line must come within 4096 bytes (`suspiciously long trailer`). Everything else in Go's
//! buffered reading (the `readContinuedLineSlice` fast path, `upcomingHeaderKeys`, early returns
//! of the chunked reader) only changes how the bytes are split between calls.

use std::collections::BTreeMap;

/// Go `bufio.defaultBufSize` (httpcache uses `bufio.NewReader`).
const BUF_SIZE: usize = 4096;

/// Go `net/http/internal.maxLineLength`.
const MAX_LINE_LENGTH: usize = 4096;

/// Go: `http.Header` / `textproto.MIMEHeader` — canonical keys to values (Go strings, i.e.
/// bytes). Keys are ASCII: the parser only accepts token bytes and spaces.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Header(pub BTreeMap<String, Vec<Vec<u8>>>);

impl Header {
    /// Go: `Header.Get(key)` — the first value, `""` if none.
    pub fn get(&self, key: &str) -> &[u8] {
        match self.0.get(&canonical_mime_header_key(key)) {
            Some(v) if !v.is_empty() => &v[0],
            _ => b"",
        }
    }

    /// Go: `Header.Values(key)`.
    pub fn values(&self, key: &str) -> &[Vec<u8>] {
        match self.0.get(&canonical_mime_header_key(key)) {
            Some(v) => v,
            None => &[],
        }
    }

    /// Go: `Header.Set(key, value)`.
    pub fn set(&mut self, key: &str, value: &[u8]) {
        self.0
            .insert(canonical_mime_header_key(key), vec![value.to_vec()]);
    }

    /// Go: `Header.Add(key, value)`.
    pub fn add(&mut self, key: &str, value: &[u8]) {
        self.0
            .entry(canonical_mime_header_key(key))
            .or_default()
            .push(value.to_vec());
    }

    /// Go: `Header.Del(key)`.
    pub fn del(&mut self, key: &str) {
        self.0.remove(&canonical_mime_header_key(key));
    }

    /// Go: `h[key]` (no canonicalisation), `None` when absent.
    pub fn raw(&self, key: &str) -> Option<&Vec<Vec<u8>>> {
        self.0.get(key)
    }
}

/// A parsed HTTP response (Go `*http.Response` subset).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Response {
    /// e.g. `200 OK` (Go `Status`, bytes).
    pub status: Vec<u8>,
    pub status_code: i64,
    /// e.g. `HTTP/2.0`.
    pub proto: String,
    pub proto_major: i64,
    pub proto_minor: i64,
    /// Canonicalised header keys -> values (Go `http.Header`).
    pub header: Header,
    /// Go `ContentLength` (-1 = unknown).
    pub content_length: i64,
    /// Go `TransferEncoding` (`["chunked"]` or empty = nil).
    pub transfer_encoding: Vec<String>,
    pub close: bool,
    /// Go `Trailer` (`None` = nil).
    pub trailer: Option<Header>,
    /// The body as `io.ReadAll(resp.Body)` returns it (what was read before an error).
    pub body: Vec<u8>,
    /// The error of `io.ReadAll(resp.Body)` (Go reads the body lazily).
    pub body_err: Option<String>,
}

impl Response {
    /// Go: `resp.Header.Get(key)`.
    pub fn header_get(&self, key: &str) -> &[u8] {
        self.header.get(key)
    }

    /// Go: `ProtoAtLeast(major, minor)`.
    // Go: net/http/response.go:ProtoAtLeast
    pub fn proto_at_least(&self, major: i64, minor: i64) -> bool {
        self.proto_major > major || self.proto_major == major && self.proto_minor >= minor
    }
}

/// Go `%q` of a Go string (`strconv.Quote`).
fn q(b: &[u8]) -> String {
    go_strconv::quote(b)
}

/// Go `%q` of a `[]string`: `["a" "b"]`.
fn q_list(v: &[Vec<u8>]) -> String {
    let items: Vec<String> = v.iter().map(|s| q(s)).collect();
    format!("[{}]", items.join(" "))
}

// Go: net/http/request.go:badStringError
fn bad_string_error(what: &str, val: &[u8]) -> String {
    format!("{what} {}", q(val))
}

const ERR_UNEXPECTED_EOF: &str = "unexpected EOF";

/// Read errors of the model reader: Go's `io.EOF` or another error text.
#[derive(Debug, PartialEq, Eq)]
enum RErr {
    Eof,
    Msg(String),
}

impl RErr {
    fn msg(s: impl Into<String>) -> RErr {
        RErr::Msg(s.into())
    }

    /// `if err == io.EOF { err = io.ErrUnexpectedEOF }`.
    fn eof_to_unexpected(self) -> String {
        match self {
            RErr::Eof => ERR_UNEXPECTED_EOF.to_string(),
            RErr::Msg(m) => m,
        }
    }
}

/// The dump behind Go's `bufio.Reader` (all bytes are in memory).
struct Reader<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> Reader<'a> {
    fn rest(&self) -> &'a [u8] {
        &self.data[self.pos..]
    }

    /// Go `bufio.Reader.ReadLine` called until the line is complete (textproto `readLineSlice`
    /// without a limit): the bytes up to `\n`, without `\n` or `\r\n`; `EOF` when nothing is left.
    fn read_line(&mut self) -> Result<&'a [u8], RErr> {
        let rest = self.rest();
        if rest.is_empty() {
            return Err(RErr::Eof);
        }
        match rest.iter().position(|&c| c == b'\n') {
            Some(i) => {
                self.pos += i + 1;
                let mut line = &rest[..i];
                if let Some(l) = line.strip_suffix(b"\r") {
                    line = l;
                }
                Ok(line)
            }
            None => {
                self.pos = self.data.len();
                Ok(rest)
            }
        }
    }

    /// Go textproto `readLineSlice(lim)`: `message too large` once the line exceeds `lim`.
    fn read_line_slice(&mut self, lim: i64) -> Result<&'a [u8], RErr> {
        let line = self.read_line()?;
        if lim >= 0 && line.len() as i64 > lim {
            return Err(RErr::msg("message too large"));
        }
        Ok(line)
    }

    /// Go `bufio.Reader.Peek(n)` (n <= BUF_SIZE): at most `n` bytes.
    fn peek(&self, n: usize) -> &'a [u8] {
        let rest = self.rest();
        &rest[..n.min(rest.len())]
    }

    /// Go textproto `skipSpace`.
    fn skip_space(&mut self) -> usize {
        let mut n = 0;
        while let Some(&c) = self.rest().first() {
            if c != b' ' && c != b'\t' {
                break;
            }
            self.pos += 1;
            n += 1;
        }
        n
    }

    /// Go `bufio.Reader.ReadSlice('\n')` for the chunk reader: `Err(true)` = EOF before a
    /// newline, `Err(false)` = `ErrBufferFull` (no newline in 4096 buffered bytes).
    fn read_slice_nl(&mut self) -> Result<&'a [u8], bool> {
        let rest = self.rest();
        let window = &rest[..rest.len().min(BUF_SIZE)];
        match window.iter().position(|&c| c == b'\n') {
            Some(i) => {
                self.pos += i + 1;
                Ok(&rest[..=i])
            }
            None => {
                if rest.len() >= BUF_SIZE {
                    self.pos += BUF_SIZE;
                    Err(false)
                } else {
                    self.pos = self.data.len();
                    Err(true)
                }
            }
        }
    }
}

/// Go textproto `trim` (spaces and tabs).
fn trim(s: &[u8]) -> &[u8] {
    let mut i = 0;
    while i < s.len() && (s[i] == b' ' || s[i] == b'\t') {
        i += 1;
    }
    let s = &s[i..];
    let mut n = s.len();
    while n > 0 && (s[n - 1] == b' ' || s[n - 1] == b'\t') {
        n -= 1;
    }
    &s[..n]
}

/// Go textproto `readContinuedLineSlice(lim, validateFirstLine)` (no limit is ever reached).
fn read_continued_line_slice(
    r: &mut Reader<'_>,
    validate_first_line: fn(&[u8]) -> Result<(), String>,
) -> Result<Vec<u8>, RErr> {
    // Read the first line.
    let line = r.read_line_slice(-1)?;
    if line.is_empty() {
        // blank line - no continuation
        return Ok(Vec::new());
    }

    validate_first_line(line).map_err(RErr::Msg)?;

    let mut buf = trim(line).to_vec();

    // Read continuation lines.
    while r.skip_space() > 0 {
        buf.push(b' ');
        let Ok(line) = r.read_line_slice(-1) else {
            break;
        };
        buf.extend_from_slice(trim(line));
    }
    Ok(buf)
}

// Go: net/textproto/reader.go:mustHaveFieldNameColon
fn must_have_field_name_colon(line: &[u8]) -> Result<(), String> {
    if !line.contains(&b':') {
        return Err(format!("malformed MIME header: missing colon: {}", q(line)));
    }
    Ok(())
}

/// Go: `textproto.readMIMEHeader(r, math.MaxInt64, maxHeaders)`.
// Go: net/textproto/reader.go:readMIMEHeader
fn read_mime_header(r: &mut Reader<'_>) -> Result<Header, RErr> {
    let mut m = Header::default();

    // The first line cannot start with a leading space.
    let buf = r.peek(1);
    if !buf.is_empty() && (buf[0] == b' ' || buf[0] == b'\t') {
        const ERROR_LIMIT: i64 = 80; // arbitrary limit on how much of the line we'll quote
        let line = r.read_line_slice(ERROR_LIMIT)?;
        return Err(RErr::msg(format!(
            "malformed MIME header initial line: {}",
            q(line)
        )));
    }

    loop {
        let kv = match read_continued_line_slice(r, must_have_field_name_colon) {
            Ok(kv) if kv.is_empty() => return Ok(m),
            Ok(kv) => kv,
            Err(e) => return Err(e),
        };

        // Key ends at first colon.
        let Some(colon) = kv.iter().position(|&c| c == b':') else {
            return Err(RErr::msg(format!("malformed MIME header line: {}", q(&kv))));
        };
        let (k, v) = (&kv[..colon], &kv[colon + 1..]);
        let Some(key) = canonical_mime_header_key_bytes(k) else {
            return Err(RErr::msg(format!("malformed MIME header line: {}", q(&kv))));
        };
        if !v.iter().all(|&c| valid_header_value_byte(c)) {
            // Go's canonicalMIMEHeaderKey canonicalises the key bytes of kv in place (unless the
            // key holds a space), so the error quotes the canonical key.
            let mut kv2 = kv.clone();
            if !k.contains(&b' ') {
                kv2[..colon].copy_from_slice(key.as_bytes());
            }
            return Err(RErr::msg(format!(
                "malformed MIME header line: {}",
                q(&kv2)
            )));
        }

        // Skip initial spaces in value.
        let mut i = 0;
        while i < v.len() && (v[i] == b' ' || v[i] == b'\t') {
            i += 1;
        }
        let value = v[i..].to_vec();

        m.0.entry(key).or_default().push(value);
    }
}

/// Go textproto `validHeaderFieldByte` (RFC 7230 token bytes).
fn valid_header_field_byte(c: u8) -> bool {
    c.is_ascii_alphanumeric()
        || matches!(
            c,
            b'!' | b'#'
                | b'$'
                | b'%'
                | b'&'
                | b'\''
                | b'*'
                | b'+'
                | b'-'
                | b'.'
                | b'^'
                | b'_'
                | b'`'
                | b'|'
                | b'~'
        )
}

/// Go textproto `validHeaderValueByte` (VCHAR, SP, HTAB, obs-text).
fn valid_header_value_byte(c: u8) -> bool {
    c >= 0x80 || (0x21..=0x7e).contains(&c) || c == b' ' || c == b'\t'
}

/// Go textproto `canonicalMIMEHeaderKey(a)` — `None` = Go's `ok == false`.
fn canonical_mime_header_key_bytes(a: &[u8]) -> Option<String> {
    if a.is_empty() {
        return None;
    }

    // See if a looks like a header key. If not, return it unchanged.
    let mut no_canon = false;
    for &c in a {
        if valid_header_field_byte(c) {
            continue;
        }
        // Don't canonicalize.
        if c == b' ' {
            // We accept invalid headers with a space before the colon, but must not
            // canonicalize them. See https://go.dev/issue/34540.
            no_canon = true;
            continue;
        }
        return None;
    }
    // Only token bytes and spaces: ASCII.
    let s = String::from_utf8(a.to_vec()).expect("ASCII");
    if no_canon {
        return Some(s);
    }
    Some(canonicalize(&s))
}

fn canonicalize(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut upper = true;
    for c in s.bytes() {
        let c = if upper && c.is_ascii_lowercase() {
            c.to_ascii_uppercase()
        } else if !upper && c.is_ascii_uppercase() {
            c.to_ascii_lowercase()
        } else {
            c
        };
        out.push(c as char);
        upper = c == b'-';
    }
    out
}

/// Go: `textproto.CanonicalMIMEHeaderKey(s)` = `http.CanonicalHeaderKey(s)` — returned unchanged
/// when it holds a byte that is not a token byte.
// Go: net/textproto/reader.go:CanonicalMIMEHeaderKey
pub fn canonical_mime_header_key(s: &str) -> String {
    if !s.bytes().all(valid_header_field_byte) {
        return s.to_string();
    }
    canonicalize(s)
}

/// Go: `http.ParseHTTPVersion(vers)`.
// Go: net/http/request.go:ParseHTTPVersion
pub fn parse_http_version(vers: &[u8]) -> Option<(i64, i64)> {
    match vers {
        b"HTTP/1.1" => return Some((1, 1)),
        b"HTTP/1.0" => return Some((1, 0)),
        _ => {}
    }
    if !vers.starts_with(b"HTTP/") {
        return None;
    }
    if vers.len() != b"HTTP/X.Y".len() {
        return None;
    }
    if vers[6] != b'.' {
        return None;
    }
    let maj = go_strconv::parse_uint(&vers[5..6], 10, 0).ok()?;
    let min = go_strconv::parse_uint(&vers[7..8], 10, 0).ok()?;
    Some((maj as i64, min as i64))
}

/// Go textproto `TrimString` (ASCII space: SP, HT, LF, CR).
fn trim_string(s: &[u8]) -> &[u8] {
    let sp = |c: u8| matches!(c, b' ' | b'\t' | b'\n' | b'\r');
    let mut s = s;
    while let Some((&c, rest)) = s.split_first() {
        if !sp(c) {
            break;
        }
        s = rest;
    }
    while let Some((&c, rest)) = s.split_last() {
        if !sp(c) {
            break;
        }
        s = rest;
    }
    s
}

/// httpguts `headerValueContainsToken` / `HeaderValuesContainsToken`.
fn header_values_contain_token(values: Option<&Vec<Vec<u8>>>, token: &str) -> bool {
    let Some(values) = values else {
        return false;
    };
    fn trim_ows(mut x: &[u8]) -> &[u8] {
        while let Some((&c, r)) = x.split_first() {
            if c != b' ' && c != b'\t' {
                break;
            }
            x = r;
        }
        while let Some((&c, r)) = x.split_last() {
            if c != b' ' && c != b'\t' {
                break;
            }
            x = r;
        }
        x
    }
    let token_equal = |t1: &[u8]| {
        t1.len() == token.len() && t1.is_ascii() && t1.eq_ignore_ascii_case(token.as_bytes())
    };
    values
        .iter()
        .any(|v| v.split(|&c| c == b',').any(|f| token_equal(trim_ows(f))))
}

// Go: net/http/transfer.go:shouldClose
fn should_close(major: i64, minor: i64, header: &mut Header, remove_close_header: bool) -> bool {
    if major < 1 {
        return true;
    }

    let conv = header.raw("Connection");
    let has_close = header_values_contain_token(conv, "close");
    if major == 1 && minor == 0 {
        return has_close || !header_values_contain_token(conv, "keep-alive");
    }

    if has_close && remove_close_header {
        header.del("Connection");
    }

    has_close
}

// Go: net/http/transfer.go:bodyAllowedForStatus
fn body_allowed_for_status(status: i64) -> bool {
    !((100..=199).contains(&status) || status == 204 || status == 304)
}

// Go: net/http/transfer.go:parseContentLength
fn parse_content_length(cl_headers: Option<&Vec<Vec<u8>>>) -> Result<i64, String> {
    let Some(cl_headers) = cl_headers.filter(|v| !v.is_empty()) else {
        return Ok(-1);
    };
    let cl = trim_string(&cl_headers[0]);

    // The Content-Length must be a valid numeric value. (httplaxcontentlength=0: the
    // default for go.mod's go 1.23.)
    if cl.is_empty() {
        return Err(bad_string_error("invalid empty Content-Length", cl));
    }
    match go_strconv::parse_uint(cl, 10, 63) {
        Ok(n) => Ok(n as i64),
        Err(_) => Err(bad_string_error("bad Content-Length", cl)),
    }
}

/// Go: `fixLength(isResponse=true, status, requestMethod, header, chunked)`.
// Go: net/http/transfer.go:fixLength
fn fix_length(
    status: i64,
    request_method: &str,
    header: &mut Header,
    chunked: bool,
) -> Result<i64, String> {
    let mut content_lens = header.raw("Content-Length").cloned().unwrap_or_default();

    // Hardening against HTTP request smuggling
    if content_lens.len() > 1 {
        // Per RFC 7230 Section 3.3.2, prevent multiple Content-Length headers if they differ in
        // value. If there are dups of the value, remove the dups.
        let first = trim_string(&content_lens[0]).to_vec();
        for ct in &content_lens[1..] {
            if first != trim_string(ct) {
                return Err(format!(
                    "http: message cannot contain multiple Content-Length headers; got {}",
                    q_list(&content_lens)
                ));
            }
        }

        // deduplicate Content-Length
        header.del("Content-Length");
        header.add("Content-Length", &first);

        content_lens = header.raw("Content-Length").cloned().unwrap_or_default();
    }

    // Reject requests with invalid Content-Length headers.
    let mut n = 0;
    if !content_lens.is_empty() {
        n = parse_content_length(Some(&content_lens)).map_err(|e| e.to_string())?;
    }

    // Logic based on response type or status
    if request_method == "HEAD" {
        return Ok(0);
    }
    if status / 100 == 1 {
        return Ok(0);
    }
    if status == 204 || status == 304 {
        return Ok(0);
    }

    // Logic based on Transfer-Encoding
    if chunked {
        header.del("Content-Length");
        return Ok(-1);
    }

    // Logic based on Content-Length
    if !content_lens.is_empty() {
        return Ok(n);
    }

    header.del("Content-Length");

    // Body-EOF logic based on other methods (like closing, or chunked coding)
    Ok(-1)
}

/// Go: `fixTrailer(header, chunked)`.
// Go: net/http/transfer.go:fixTrailer
fn fix_trailer(header: &mut Header, chunked: bool) -> Result<Option<Header>, String> {
    let Some(vv) = header.raw("Trailer").cloned() else {
        return Ok(None);
    };
    if !chunked {
        // Trailer and no chunking: the Trailer header is kept in Response.Header but does not
        // populate Response.Trailer (issue #27197).
        return Ok(None);
    }
    header.del("Trailer");

    let mut trailer = Header::default();
    let mut err: Option<String> = None;
    for v in &vv {
        foreach_header_element(v, |key| {
            let key = canonical_mime_header_key_lossy(key);
            if matches!(
                key.as_str(),
                "Transfer-Encoding" | "Trailer" | "Content-Length"
            ) && err.is_none()
            {
                err = Some(bad_string_error("bad trailer key", key.as_bytes()));
                return;
            }
            trailer.0.insert(key, Vec::new());
        });
    }
    if let Some(e) = err {
        return Err(e);
    }
    if trailer.0.is_empty() {
        return Ok(None);
    }
    Ok(Some(trailer))
}

/// `CanonicalHeaderKey` of a header element (a key that is not UTF-8 is converted lossily; it
/// holds a non-token byte, so Go leaves it unchanged too).
fn canonical_mime_header_key_lossy(key: &[u8]) -> String {
    canonical_mime_header_key(&String::from_utf8_lossy(key))
}

// Go: net/http/server.go:foreachHeaderElement
fn foreach_header_element(v: &[u8], mut f: impl FnMut(&[u8])) {
    let v = trim_string(v);
    if v.is_empty() {
        return;
    }
    if !v.contains(&b',') {
        f(v);
        return;
    }
    for fld in v.split(|&c| c == b',') {
        let fld = trim_string(fld);
        if !fld.is_empty() {
            f(fld);
        }
    }
}

/// The body source chosen by `readTransfer`.
enum BodyKind {
    NoBody,
    Chunked,
    Limited(i64),
    UntilEof,
}

/// Go: `http.ReadResponse(bufio.NewReader(bytes.NewReader(dump)), req)` followed by
/// `io.ReadAll(resp.Body)`: the status line, the MIME header, the transfer semantics and the body
/// (Content-Length, chunked with trailer, or until EOF). `method` is the request's (`GET`,
/// `HEAD`, ...). The error is Go's `ReadResponse` error text.
// Go: net/http/response.go:ReadResponse
pub fn read_response(dump: &[u8], method: &str) -> Result<Response, String> {
    let mut r = Reader { data: dump, pos: 0 };
    let mut resp = Response::default();

    // Parse the first line of the response.
    let line = r.read_line_slice(-1).map_err(RErr::eof_to_unexpected)?;
    let Some(sp) = line.iter().position(|&c| c == b' ') else {
        return Err(bad_string_error("malformed HTTP response", line));
    };
    let (proto, status) = (&line[..sp], &line[sp + 1..]);
    let mut i = 0;
    while i < status.len() && status[i] == b' ' {
        i += 1;
    }
    resp.status = status[i..].to_vec();

    let status_code = match resp.status.iter().position(|&c| c == b' ') {
        Some(j) => &resp.status[..j],
        None => &resp.status[..],
    };
    if status_code.len() != 3 {
        return Err(bad_string_error("malformed HTTP status code", status_code));
    }
    match go_strconv::atoi(status_code) {
        Ok(n) if n >= 0 => resp.status_code = n,
        _ => return Err(bad_string_error("malformed HTTP status code", status_code)),
    }
    let Some((major, minor)) = parse_http_version(proto) else {
        return Err(bad_string_error("malformed HTTP version", proto));
    };
    resp.proto = String::from_utf8(proto.to_vec()).expect("HTTP/X.Y is ASCII");
    resp.proto_major = major;
    resp.proto_minor = minor;

    // Parse the response headers.
    resp.header = read_mime_header(&mut r).map_err(RErr::eof_to_unexpected)?;

    fix_pragma_cache_control(&mut resp.header);

    let kind = read_transfer(&mut resp, method)?;

    read_body(&mut r, &mut resp, kind);

    Ok(resp)
}

/// RFC 7234, section 5.4: treat `Pragma: no-cache` like `Cache-Control: no-cache`.
// Go: net/http/response.go:fixPragmaCacheControl
fn fix_pragma_cache_control(header: &mut Header) {
    if let Some(hp) = header.raw("Pragma")
        && !hp.is_empty()
        && hp[0] == b"no-cache"
        && header.raw("Cache-Control").is_none()
    {
        header
            .0
            .insert("Cache-Control".to_string(), vec![b"no-cache".to_vec()]);
    }
}

/// Go: `readTransfer(resp, r, math.MaxInt64)` for a `*Response`.
// Go: net/http/transfer.go:readTransfer
fn read_transfer(resp: &mut Response, request_method: &str) -> Result<BodyKind, String> {
    let status_code = resp.status_code;
    let (mut proto_major, mut proto_minor) = (resp.proto_major, resp.proto_minor);
    let mut close = should_close(proto_major, proto_minor, &mut resp.header, true);

    // Default to HTTP/1.1
    if proto_major == 0 && proto_minor == 0 {
        proto_major = 1;
        proto_minor = 1;
    }

    // Transfer-Encoding: chunked, and overriding Content-Length.
    let chunked = parse_transfer_encoding(&mut resp.header, proto_major, proto_minor)?;

    let real_length = fix_length(status_code, request_method, &mut resp.header, chunked)?;
    let content_length = if request_method == "HEAD" {
        parse_content_length(resp.header.raw("Content-Length"))?
    } else {
        real_length
    };

    // Trailer
    let trailer = fix_trailer(&mut resp.header, chunked)?;

    // If there is no Content-Length or chunked Transfer-Encoding on a *Response and the status
    // is not 1xx, 204 or 304, then the body is unbounded. See RFC 7230, section 3.3.
    if real_length == -1 && !chunked && body_allowed_for_status(status_code) {
        // Unbounded body.
        close = true;
    }

    // Prepare body reader.
    let kind = if chunked {
        if request_method == "HEAD" || !body_allowed_for_status(status_code) {
            BodyKind::NoBody
        } else {
            BodyKind::Chunked
        }
    } else if real_length == 0 {
        BodyKind::NoBody
    } else if real_length > 0 {
        BodyKind::Limited(real_length)
    } else if close {
        // Close semantics (i.e. HTTP/1.0)
        BodyKind::UntilEof
    } else {
        // Persistent connection (i.e. HTTP/1.1)
        BodyKind::NoBody
    };

    resp.content_length = content_length;
    if chunked {
        resp.transfer_encoding = vec!["chunked".to_string()];
    }
    resp.close = close;
    resp.trailer = trailer;

    Ok(kind)
}

/// Go: `transferReader.parseTransferEncoding()` — whether the body is chunked.
// Go: net/http/transfer.go:parseTransferEncoding
fn parse_transfer_encoding(
    header: &mut Header,
    proto_major: i64,
    proto_minor: i64,
) -> Result<bool, String> {
    let Some(raw) = header.0.remove("Transfer-Encoding") else {
        return Ok(false);
    };

    // Issue 12785; ignore Transfer-Encoding on HTTP/1.0 requests.
    if !(proto_major > 1 || proto_major == 1 && proto_minor >= 1) {
        return Ok(false);
    }

    if raw.len() != 1 {
        return Err(format!("too many transfer encodings: {}", q_list(&raw)));
    }
    if !raw[0].eq_ignore_ascii_case(b"chunked") {
        return Err(format!("unsupported transfer encoding: {}", q(&raw[0])));
    }

    Ok(true)
}

/// `io.ReadAll(resp.Body)` for the chosen body source.
fn read_body(r: &mut Reader<'_>, resp: &mut Response, kind: BodyKind) {
    match kind {
        BodyKind::NoBody => {}
        BodyKind::UntilEof => {
            resp.body = r.rest().to_vec();
            r.pos = r.data.len();
        }
        BodyKind::Limited(n) => {
            // io.LimitReader; an early EOF is io.ErrUnexpectedEOF (body.readLocked).
            let rest = r.rest();
            let take = (n as u64).min(rest.len() as u64) as usize;
            resp.body = rest[..take].to_vec();
            r.pos += take;
            if (take as i64) < n {
                resp.body_err = Some(ERR_UNEXPECTED_EOF.to_string());
            }
        }
        BodyKind::Chunked => {
            let (body, err) = read_chunked(r);
            resp.body = body;
            match err {
                // The chunked reader reached its end: read the trailer.
                None => {
                    if let Err(e) = read_trailer(r, resp) {
                        resp.body_err = Some(e);
                    }
                }
                Some(e) => resp.body_err = Some(e),
            }
        }
    }
}

/// Go's `internal.chunkedReader` read to the end: the data and the error (`None` = `io.EOF`).
// Go: net/http/internal/chunked.go:(*chunkedReader).Read
fn read_chunked(r: &mut Reader<'_>) -> (Vec<u8>, Option<String>) {
    let mut out = Vec::new();
    let mut excess: i64 = 0;
    let mut check_end = false;
    loop {
        if check_end {
            let rest = r.rest();
            if rest.len() < 2 {
                // io.ReadFull: EOF (0 bytes) or ErrUnexpectedEOF; both become ErrUnexpectedEOF.
                r.pos = r.data.len();
                return (out, Some(ERR_UNEXPECTED_EOF.to_string()));
            }
            r.pos += 2;
            if &rest[..2] != b"\r\n" {
                return (out, Some("malformed chunked encoding".to_string()));
            }
        }
        // beginChunk: chunk-size CRLF
        let line = match read_chunk_line(r) {
            Ok(l) => l,
            Err(e) => return (out, Some(e)),
        };
        excess = excess.wrapping_add(line.len() as i64 + 2); // header, plus \r\n after the data
        let line = trim_trailing_whitespace(line);
        let line = remove_chunk_extension(line);
        let n = match parse_hex_uint(line) {
            Ok(n) => n,
            Err(e) => return (out, Some(e)),
        };
        // We are willing to accept 16 bytes of overhead per chunk, plus twice the amount of
        // real data in the chunk.
        excess = excess.wrapping_sub(16i64.wrapping_add(2i64.wrapping_mul(n as i64)));
        excess = excess.max(0);
        if n == 0 {
            return (out, None);
        }
        if excess > 16 * 1024 {
            return (
                out,
                Some("chunked encoding contains too much non-data".to_string()),
            );
        }
        let rest = r.rest();
        if (rest.len() as u64) < n {
            out.extend_from_slice(rest);
            r.pos = r.data.len();
            return (out, Some(ERR_UNEXPECTED_EOF.to_string()));
        }
        out.extend_from_slice(&rest[..n as usize]);
        r.pos += n as usize;
        check_end = true;
    }
}

// Go: net/http/internal/chunked.go:readChunkLine
fn read_chunk_line<'a>(r: &mut Reader<'a>) -> Result<&'a [u8], String> {
    let p = match r.read_slice_nl() {
        Ok(p) => p,
        // We always know when EOF is coming. If the caller asked for a line, there should be a
        // line.
        Err(true) => return Err(ERR_UNEXPECTED_EOF.to_string()),
        Err(false) => return Err("header line too long".to_string()),
    };

    // Verify that the line ends in a CRLF, and that no CRs appear before the end.
    match p.iter().position(|&c| c == b'\r') {
        None => return Err("chunked line ends with bare LF".to_string()),
        Some(idx) if idx != p.len() - 2 => {
            return Err("invalid CR in chunked line".to_string());
        }
        _ => {}
    }
    let p = &p[..p.len() - 2]; // trim CRLF

    if p.len() >= MAX_LINE_LENGTH {
        return Err("header line too long".to_string());
    }
    Ok(p)
}

// Go: net/http/internal/chunked.go:trimTrailingWhitespace
fn trim_trailing_whitespace(mut b: &[u8]) -> &[u8] {
    while let Some((&c, rest)) = b.split_last() {
        if c != b' ' && c != b'\t' {
            break;
        }
        b = rest;
    }
    b
}

// Go: net/http/internal/chunked.go:removeChunkExtension
fn remove_chunk_extension(p: &[u8]) -> &[u8] {
    match p.iter().position(|&c| c == b';') {
        Some(i) => &p[..i],
        None => p,
    }
}

// Go: net/http/internal/chunked.go:parseHexUint
fn parse_hex_uint(v: &[u8]) -> Result<u64, String> {
    if v.is_empty() {
        return Err("empty hex number for chunk length".to_string());
    }
    let mut n: u64 = 0;
    for (i, &b) in v.iter().enumerate() {
        let d = match b {
            b'0'..=b'9' => b - b'0',
            b'a'..=b'f' => b - b'a' + 10,
            b'A'..=b'F' => b - b'A' + 10,
            _ => return Err("invalid byte in chunk length".to_string()),
        };
        if i == 16 {
            return Err("http chunk length too large".to_string());
        }
        n <<= 4;
        n |= d as u64;
    }
    Ok(n)
}

/// Go: `(*body).readTrailer()` after the last chunk.
// Go: net/http/transfer.go:readTrailer
fn read_trailer(r: &mut Reader<'_>, resp: &mut Response) -> Result<(), String> {
    const ERR_TRAILER_EOF: &str = "http: unexpected EOF reading trailer";
    // The common case, since nobody uses trailers.
    let buf = r.peek(2);
    if buf == b"\r\n" {
        r.pos += 2;
        return Ok(());
    }
    if buf.len() < 2 {
        return Err(ERR_TRAILER_EOF.to_string());
    }

    // Make sure there's a header terminator coming up (within bufio's 4096-byte buffer).
    if !see_upcoming_double_crlf(r) {
        return Err("http: suspiciously long trailer after chunked body".to_string());
    }

    let hdr = match read_mime_header(r) {
        Ok(h) => h,
        Err(RErr::Eof) => return Err(ERR_TRAILER_EOF.to_string()),
        Err(RErr::Msg(m)) => return Err(m),
    };
    // mergeSetHeader
    match &mut resp.trailer {
        None => resp.trailer = Some(hdr),
        Some(t) => {
            for (k, v) in hdr.0 {
                t.0.insert(k, v);
            }
        }
    }
    Ok(())
}

// Go: net/http/transfer.go:seeUpcomingDoubleCRLF
fn see_upcoming_double_crlf(r: &Reader<'_>) -> bool {
    let rest = r.rest();
    let window = &rest[..rest.len().min(BUF_SIZE)];
    window.windows(4).any(|w| w == b"\r\n\r\n")
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (Go net/http + net/textproto subset; not generated)
// OK net/http/response.go: ReadResponse, fixPragmaCacheControl, ProtoAtLeast
// OK net/http/request.go: ParseHTTPVersion, badStringError
// OK net/http/transfer.go: readTransfer (Response), parseTransferEncoding, fixLength,
//    shouldClose, fixTrailer, parseContentLength, bodyAllowedForStatus, body.readLocked
//    (LimitReader EOF check), readTrailer, seeUpcomingDoubleCRLF, mergeSetHeader
// OK net/http/internal/chunked.go: chunkedReader.Read/beginChunk, readChunkLine,
//    trimTrailingWhitespace, removeChunkExtension, parseHexUint
// OK net/http/server.go: foreachHeaderElement
// OK net/textproto/reader.go: readLineSlice, readContinuedLineSlice, trim, skipSpace,
//    readMIMEHeader, mustHaveFieldNameColon, CanonicalMIMEHeaderKey, canonicalMIMEHeaderKey,
//    validHeaderFieldByte, validHeaderValueByte; TrimString
// OK x/net/http/httpguts: HeaderValuesContainsToken, headerValueContainsToken, tokenEqual
// ---------------------------------------------------------------------------
