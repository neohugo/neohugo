//! Module `cache::httpcache::transport`.
//!
//! NEW: `github.com/gohugoio/httpcache@v0.7.0` subset: `Transport.RoundTrip` with its
//! cached-response path (`AlwaysUseCachedResponse`, `MarkCachedResponses`, `EnableETagPair`
//! read side, Vary matching, freshness, validators, stale-if-error).
//!
//! Owner: Wave B task T08 (helpers-source-cache).
//!
//! The getresource file cache stores `httputil.DumpResponse` bytes (`HTTP/2.0 200 OK\r\n` +
//! headers + `\r\n` + body), parsed by [`super::http::read_response`]. With the default config
//! (`AlwaysUseCachedResponse`) a cache hit is returned without any network access. The port has
//! no network code: the inner transport ([`Transport::transport`]) defaults to
//! [`NoNetwork`], an explicit error, and a response coming back from an inner transport (the
//! store path: `ShouldCache`, `httputil.DumpResponse`, the ETag pair) is an explicit unsupported
//! error too.

use std::collections::BTreeMap;
use std::sync::Arc;

use go_time::Duration;
use nh_common::Result;
use nh_common::herrors::Error;

pub use super::http::{Header, Response, canonical_mime_header_key, read_response};

/// Go: `httpcache.XFromCache` — added to responses returned from the cache.
pub const X_FROM_CACHE: &str = "X-From-Cache";
/// Go: `httpcache.XETag1` — the first eTag value.
pub const X_ETAG1: &str = "X-Etags-1";
/// Go: `httpcache.XETag2` — the second eTag value.
pub const X_ETAG2: &str = "X-Etags-2";

/// Go: `httpcache.Cache`.
pub trait HttpCache: Send + Sync {
    /// Go: `Get(key) (responseBytes []byte, ok bool)` — `ok` is false if the value is stale;
    /// `None` is Go's nil slice.
    fn get(&self, key: &str) -> (Option<Vec<u8>>, bool);
    fn set(&self, key: &str, resp: &[u8]);
    fn delete(&self, key: &str);
}

/// The request as the transport sees it (Go `*http.Request` subset).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Request {
    /// e.g. `GET` (Go's `""` means GET).
    pub method: String,
    /// `req.URL.String()`.
    pub url: String,
    pub header: Header,
}

impl Request {
    fn method(&self) -> &str {
        if self.method.is_empty() {
            "GET"
        } else {
            &self.method
        }
    }
}

/// Go `http.RoundTripper` (the inner transport).
pub trait RoundTripper: Send + Sync {
    fn round_trip(&self, req: &Request) -> Result<Response>;
}

/// The default inner transport: the Rust port has no network code (a GetRemote cache miss is an
/// error in the acceptance configuration).
pub struct NoNetwork;

impl RoundTripper for NoNetwork {
    fn round_trip(&self, req: &Request) -> Result<Response> {
        Err(Error::new(format!(
            "neohugo-rs: network access is not supported ({} {} is not in the file cache)",
            req.method(),
            req.url
        )))
    }
}

/// Go `Transport.CacheKey`.
pub type CacheKeyFunc = Arc<dyn Fn(&Request) -> String + Send + Sync>;
/// Go `Transport.AlwaysUseCachedResponse`.
pub type AlwaysUseCachedResponseFunc = Arc<dyn Fn(&Request, &str) -> bool + Send + Sync>;
/// Go `Transport.ShouldCache`.
pub type ShouldCacheFunc = Arc<dyn Fn(&Request, &Response, &str) -> bool + Send + Sync>;
/// Go `Transport.Around`.
pub type AroundFunc = Arc<dyn Fn(&Request, &str) -> AroundGuard + Send + Sync>;

/// A guard returned by [`Transport::around`] (Go: the func deferred until the end of RoundTrip).
pub type AroundGuard = Box<dyn std::any::Any>;

/// Go: `httpcache.Transport`, configured like `create.New`: `AlwaysUseCachedResponse`,
/// `MarkCachedResponses`, `EnableETagPair`, `ShouldCache`, `CacheKey`, `Around`.
#[derive(Clone)]
pub struct Transport {
    /// The inner transport (Go `Transport`; nil = `http.DefaultTransport`, here [`NoNetwork`]).
    pub transport: Option<Arc<dyn RoundTripper>>,
    /// The cache used to store and retrieve responses.
    pub cache: Arc<dyn HttpCache>,
    /// If true, responses returned from the cache get an extra header, `X-From-Cache`.
    pub mark_cached_responses: bool,
    /// If true, the pair of eTags is stored in the response header.
    pub enable_etag_pair: bool,
    /// Returns the key to store the response under; `""` = do not cache.
    pub cache_key: Option<CacheKeyFunc>,
    /// When it returns true, a successful response from the cache is returned without
    /// connecting to the server.
    pub always_use_cached_response: Option<AlwaysUseCachedResponseFunc>,
    /// When it returns false, the response is not cached.
    pub should_cache: Option<ShouldCacheFunc>,
    /// Called at the start of RoundTrip; the returned guard is dropped at its end.
    pub around: Option<AroundFunc>,
}

/// Go's `stale`/`fresh`/`transparent`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Freshness {
    Stale,
    Fresh,
    Transparent,
}

impl Transport {
    /// A transport with Go's zero values and the given cache.
    pub fn new(cache: Arc<dyn HttpCache>) -> Transport {
        Transport {
            transport: None,
            cache,
            mark_cached_responses: false,
            enable_etag_pair: false,
            cache_key: None,
            always_use_cached_response: None,
            should_cache: None,
            around: None,
        }
    }

    /// Go: `cacheKey(req)`.
    // Go: gohugoio/httpcache httpcache.go:(*Transport).cacheKey
    pub fn cache_key_for(&self, req: &Request) -> String {
        if let Some(f) = &self.cache_key {
            return f(req);
        }

        let cacheable = req.header.get("range").is_empty();
        if !cacheable {
            return String::new();
        }

        if req.method() == "GET" {
            req.url.clone()
        } else {
            format!("{} {}", req.method(), req.url)
        }
    }

    /// Go: `cachedResponse(req)` — the cached response and whether it is fresh (`ok`), or the
    /// parse error.
    // Go: gohugoio/httpcache httpcache.go:(*Transport).cachedResponse
    fn cached_response(&self, req: &Request) -> (Option<Response>, bool, Option<String>) {
        let (cached_val, ok) = self.cache.get(&self.cache_key_for(req));
        let cached_val = cached_val.unwrap_or_default();
        if !ok && cached_val.is_empty() {
            return (None, false, None);
        }
        match read_response(&cached_val, req.method()) {
            Ok(resp) => (Some(resp), ok, None),
            Err(e) => (None, false, Some(e)),
        }
    }

    /// Go: `RoundTrip(req)`. If there is a fresh response in the cache (or
    /// `AlwaysUseCachedResponse`), it is returned without connecting to the server; everything
    /// else goes to the inner transport.
    // Go: gohugoio/httpcache httpcache.go:(*Transport).RoundTrip
    pub fn round_trip(&self, req: &Request) -> Result<Response> {
        let cache_key = self.cache_key_for(req);
        let _around = self.around.as_ref().map(|f| f(req, &cache_key));

        let cacheable = !cache_key.is_empty();

        let mut cached_resp: Option<Response> = None;
        let mut has_cached_resp = false;
        let mut err: Option<String> = None;
        if cacheable {
            let (cr, ok, e) = self.cached_response(req);
            cached_resp = cr;
            has_cached_resp = ok;
            err = e;
            if err.is_none()
                && has_cached_resp
                && let Some(f) = &self.always_use_cached_response
                && f(req, &cache_key)
            {
                return Ok(cached_resp.expect("a cached response"));
            }
        } else {
            // Need to invalidate an existing value
            self.cache.delete(&cache_key);
        }

        let no_network: Arc<dyn RoundTripper> = Arc::new(NoNetwork);
        let transport = self.transport.clone().unwrap_or(no_network);

        if let Some(cr) = &cached_resp
            && self.enable_etag_pair
        {
            // cachedXEtag is only used by the store path.
            let _ = get_x_etags(&cr.header);
        }

        let mut req = req.clone();
        let mut resp: Result<Response>;
        if cacheable && has_cached_resp && err.is_none() {
            let cr = cached_resp.as_mut().expect("a cached response");
            if self.mark_cached_responses {
                cr.header.set(X_FROM_CACHE, b"1");
            }

            if vary_matches(cr, &req) {
                // Can only use cached value if the new request doesn't Vary significantly
                let freshness = get_freshness(&cr.header, &req.header);
                if freshness == Freshness::Fresh {
                    return Ok(cached_resp.expect("a cached response"));
                }

                if freshness == Freshness::Stale {
                    let mut req2: Option<Request> = None;
                    // Add validators if caller hasn't already done so
                    let etag = cr.header.get("etag").to_vec();
                    if !etag.is_empty() && req.header.get("etag").is_empty() {
                        let mut r2 = req.clone();
                        r2.header.set("if-none-match", &etag);
                        req2 = Some(r2);
                    }
                    let last_modified = cr.header.get("last-modified").to_vec();
                    if !last_modified.is_empty() && req.header.get("last-modified").is_empty() {
                        let mut r2 = req2.take().unwrap_or_else(|| req.clone());
                        r2.header.set("if-modified-since", &last_modified);
                        req2 = Some(r2);
                    }
                    if let Some(r2) = req2 {
                        req = r2;
                    }
                }
            }

            resp = transport.round_trip(&req);
            let cr_header = cached_resp
                .as_ref()
                .expect("a cached response")
                .header
                .clone();

            match resp {
                Ok(r) if req.method() != "HEAD" && r.status_code == 304 => {
                    // Replace the 304 response with the one from cache, but update with some new
                    // headers
                    let end_to_end_headers = get_end_to_end_headers(&r.header);
                    let mut cr = cached_resp.expect("a cached response");
                    for header in end_to_end_headers {
                        if let Some(v) = r.header.0.get(&header) {
                            cr.header.0.insert(header, v.clone());
                        }
                    }
                    resp = Ok(cr);
                }
                Ok(ref r)
                    if r.status_code >= 500
                        && req.method() != "HEAD"
                        && can_stale_on_error(&cr_header, &req.header) =>
                {
                    return Ok(cached_resp.expect("a cached response"));
                }
                Err(_) if req.method() != "HEAD" && can_stale_on_error(&cr_header, &req.header) => {
                    // In case of transport failure and stale-if-error activated, returns
                    // cached content when available
                    return Ok(cached_resp.expect("a cached response"));
                }
                other => {
                    let bad = match &other {
                        Err(_) => true,
                        Ok(r) => r.status_code != 200,
                    };
                    if bad {
                        self.cache.delete(&cache_key);
                    }
                    resp = Ok(other?);
                }
            }
        } else {
            let req_cache_control = parse_cache_control(&req.header);
            if req_cache_control.contains_key("only-if-cached".as_bytes()) {
                resp = Ok(new_gateway_timeout_response(&req));
            } else {
                resp = Ok(transport.round_trip(&req)?);
            }
        }
        let resp = resp?;

        if cacheable
            && self
                .should_cache
                .as_ref()
                .is_none_or(|f| f(&req, &resp, &cache_key))
            && can_store(
                &parse_cache_control(&req.header),
                &parse_cache_control(&resp.header),
            )
        {
            // The store path (X-Varied headers, httputil.DumpResponse, the ETag pair, Set) is
            // only reached with a response from the network or a 304 merge.
            return Err(unsupported_store());
        }
        self.cache.delete(&cache_key);
        Ok(resp)
    }
}

fn unsupported_store() -> Error {
    Error::new("neohugo-rs: storing HTTP responses in the file cache is not supported")
}

/// Go: `varyMatches(cachedResp, req)` — false unless all of the cached values for the headers
/// listed in Vary match the new request.
// Go: gohugoio/httpcache httpcache.go:varyMatches
fn vary_matches(cached_resp: &Response, req: &Request) -> bool {
    for header in header_all_comma_sep_values(&cached_resp.header, "vary") {
        let header = canonical_mime_header_key(&String::from_utf8_lossy(&header));
        if !header.is_empty()
            && req.header.get(&header) != cached_resp.header.get(&format!("X-Varied-{header}"))
        {
            return false;
        }
    }
    true
}

/// Go: `date(respHeaders)` — the parsed Date header.
// Go: gohugoio/httpcache httpcache.go:date
fn date(resp_headers: &Header) -> std::result::Result<go_value::Time, ()> {
    let date_header = resp_headers.get("date");
    if date_header.is_empty() {
        return Err(());
    }
    go_time::parse(go_time::RFC1123, date_header).map_err(|_| ())
}

/// Go: `getXETags(h)`.
// Go: gohugoio/httpcache httpcache.go:getXETags
pub fn get_x_etags(h: &Header) -> (Vec<u8>, Vec<u8>) {
    (h.get(X_ETAG1).to_vec(), h.get(X_ETAG2).to_vec())
}

/// Go `time.ParseDuration(s + "s")`.
fn parse_seconds(s: &[u8]) -> std::result::Result<Duration, ()> {
    let mut v = s.to_vec();
    v.push(b's');
    go_time::parse_duration(&v).map_err(|_| ())
}

/// Go: `getFreshness(respHeaders, reqHeaders)` — fresh (return it), stale (validate it first)
/// or transparent (do not use it). Uses the real clock (`time.Since`), like Go.
// Go: gohugoio/httpcache httpcache.go:getFreshness
pub fn get_freshness(resp_headers: &Header, req_headers: &Header) -> Freshness {
    let resp_cache_control = parse_cache_control(resp_headers);
    let req_cache_control = parse_cache_control(req_headers);
    if req_cache_control.contains_key("no-cache".as_bytes()) {
        return Freshness::Transparent;
    }
    if resp_cache_control.contains_key("no-cache".as_bytes()) {
        return Freshness::Stale;
    }
    if req_cache_control.contains_key("only-if-cached".as_bytes()) {
        return Freshness::Fresh;
    }

    let Ok(date) = date(resp_headers) else {
        return Freshness::Stale;
    };
    let mut current_age = go_time::since(&date);

    let mut lifetime = Duration(0);

    // If a response includes both an Expires header and a max-age directive, the max-age
    // directive overrides the Expires header, even if the Expires header is more restrictive.
    if let Some(max_age) = resp_cache_control.get("max-age".as_bytes()) {
        lifetime = parse_seconds(max_age).unwrap_or(Duration(0));
    } else {
        let expires_header = resp_headers.get("Expires");
        if !expires_header.is_empty() {
            lifetime = match go_time::parse(go_time::RFC1123, expires_header) {
                Err(_) => Duration(0),
                Ok(expires) => go_time::GoTimeExt::sub(&expires, &date),
            };
        }
    }

    if let Some(max_age) = req_cache_control.get("max-age".as_bytes()) {
        // the client is willing to accept a response whose age is no greater than the
        // specified time in seconds
        lifetime = parse_seconds(max_age).unwrap_or(Duration(0));
    }
    if let Some(minfresh) = req_cache_control.get("min-fresh".as_bytes()) {
        // the client wants a response that will still be fresh for at least the specified
        // number of seconds.
        if let Ok(d) = parse_seconds(minfresh) {
            current_age = Duration(current_age.0.wrapping_add(d.0));
        }
    }

    if let Some(maxstale) = req_cache_control.get("max-stale".as_bytes()) {
        // The client is willing to accept a response that has exceeded its expiration time.
        if maxstale.is_empty() {
            return Freshness::Fresh;
        }
        if let Ok(d) = parse_seconds(maxstale) {
            current_age = Duration(current_age.0.wrapping_sub(d.0));
        }
    }

    if lifetime > current_age {
        return Freshness::Fresh;
    }

    Freshness::Stale
}

/// Go: `canStaleOnError(respHeaders, reqHeaders)` — the stale-if-error cache control extension
/// (RFC 5861).
// Go: gohugoio/httpcache httpcache.go:canStaleOnError
pub fn can_stale_on_error(resp_headers: &Header, req_headers: &Header) -> bool {
    let resp_cache_control = parse_cache_control(resp_headers);
    let req_cache_control = parse_cache_control(req_headers);

    let mut lifetime = Duration(-1);

    if let Some(stale_max_age) = resp_cache_control.get("stale-if-error".as_bytes()) {
        if !stale_max_age.is_empty() {
            match parse_seconds(stale_max_age) {
                Ok(d) => lifetime = d,
                Err(_) => return false,
            }
        } else {
            return true;
        }
    }
    if let Some(stale_max_age) = req_cache_control.get("stale-if-error".as_bytes()) {
        if !stale_max_age.is_empty() {
            match parse_seconds(stale_max_age) {
                Ok(d) => lifetime = d,
                Err(_) => return false,
            }
        } else {
            return true;
        }
    }

    if lifetime.0 >= 0 {
        let Ok(date) = date(resp_headers) else {
            return false;
        };
        let current_age = go_time::since(&date);
        if lifetime > current_age {
            return true;
        }
    }

    false
}

/// Go: `getEndToEndHeaders(respHeaders)` (sorted; Go returns map order).
// Go: gohugoio/httpcache httpcache.go:getEndToEndHeaders
pub fn get_end_to_end_headers(resp_headers: &Header) -> Vec<String> {
    // These headers are always hop-by-hop
    let mut hop_by_hop: Vec<String> = [
        "Connection",
        "Keep-Alive",
        "Proxy-Authenticate",
        "Proxy-Authorization",
        "Te",
        "Trailers",
        "Transfer-Encoding",
        "Upgrade",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect();

    for extra in resp_headers.get("connection").split(|&c| c == b',') {
        // any header listed in connection, if present, is also considered hop-by-hop
        let t = trim_spaces(extra);
        if !t.is_empty() {
            hop_by_hop.push(canonical_mime_header_key(&String::from_utf8_lossy(extra)));
        }
    }
    resp_headers
        .0
        .keys()
        .filter(|k| !hop_by_hop.contains(k))
        .cloned()
        .collect()
}

/// Go `strings.Trim(s, " ")`.
fn trim_spaces(mut s: &[u8]) -> &[u8] {
    while let Some((&b' ', r)) = s.split_first() {
        s = r;
    }
    while let Some((&b' ', r)) = s.split_last() {
        s = r;
    }
    s
}

/// Go: `canStore(reqCacheControl, respCacheControl)`.
// Go: gohugoio/httpcache httpcache.go:canStore
pub fn can_store(req_cache_control: &CacheControl, resp_cache_control: &CacheControl) -> bool {
    !(resp_cache_control.contains_key("no-store".as_bytes())
        || req_cache_control.contains_key("no-store".as_bytes()))
}

/// Go: `newGatewayTimeoutResponse(req)`.
// Go: gohugoio/httpcache httpcache.go:newGatewayTimeoutResponse
fn new_gateway_timeout_response(req: &Request) -> Response {
    match read_response(b"HTTP/1.1 504 Gateway Timeout\r\n\r\n", req.method()) {
        Ok(r) => r,
        Err(e) => panic!("{e}"),
    }
}

/// Go: `httpcache.cacheControl`.
pub type CacheControl = BTreeMap<Vec<u8>, Vec<u8>>;

/// Go: `parseCacheControl(headers)`.
// Go: gohugoio/httpcache httpcache.go:parseCacheControl
pub fn parse_cache_control(headers: &Header) -> CacheControl {
    let mut cc = CacheControl::new();
    let cc_header = headers.get("Cache-Control");
    for part in cc_header.split(|&c| c == b',') {
        let part = trim_spaces(part);
        if part.is_empty() {
            continue;
        }
        if part.contains(&b'=') {
            let keyval: Vec<&[u8]> = part.split(|&c| c == b'=').collect();
            // strings.Trim(keyval[1], ",") — no commas can be left after the split on ",".
            cc.insert(trim_spaces(keyval[0]).to_vec(), keyval[1].to_vec());
        } else {
            cc.insert(part.to_vec(), Vec::new());
        }
    }
    cc
}

/// Go: `headerAllCommaSepValues(headers, name)` — every comma-separated value (trimmed) of
/// every occurrence of the header.
// Go: gohugoio/httpcache httpcache.go:headerAllCommaSepValues
pub fn header_all_comma_sep_values(headers: &Header, name: &str) -> Vec<Vec<u8>> {
    let mut vals = Vec::new();
    if let Some(vs) = headers.raw(&canonical_mime_header_key(name)) {
        for val in vs {
            for f in val.split(|&c| c == b',') {
                vals.push(go_unicode::strings::trim_space(f).to_vec());
            }
        }
    }
    vals
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (github.com/gohugoio/httpcache@v0.7.0 httpcache.go; not generated)
// OK L55-72: (t *Transport) cacheKey(req *http.Request) string
// OK L76-88: (t *Transport) cachedResponse(req *http.Request) (*http.Response, bool, error)
// OK L130-139: varyMatches(cachedResp *http.Response, req *http.Request) bool
// OK L148-310: (t *Transport) RoundTrip(req *http.Request) — the cached-response path; the
//    network path goes to the inner transport (NoNetwork by default); a response from the
//    inner transport (304 merge, store with DumpResponse + ETag pair) is an explicit
//    unsupported error
// OK L316-325: date(respHeaders http.Header) (date time.Time, err error)
// OK L339-341: getXETags(h http.Header) (string, string)
// OK L352-423: getFreshness(respHeaders, reqHeaders http.Header) (freshness int)
// OK L427-466: canStaleOnError(respHeaders, reqHeaders http.Header) bool
// OK L468-493: getEndToEndHeaders(respHeaders http.Header) []string
// OK L495-503: canStore(reqCacheControl, respCacheControl cacheControl) (canStore bool)
// OK L505-513: newGatewayTimeoutResponse(req *http.Request) *http.Response
//    L517-526: cloneRequest(r *http.Request) *http.Request — Request::clone
// OK L530-548: parseCacheControl(headers http.Header) cacheControl
// OK L557-567: headerAllCommaSepValues(headers http.Header, name string) []string
//    L571-600: cachingReadCloser — store path, not ported (explicit error above)
// ---------------------------------------------------------------------------
