//! Port of `resources/resource_factories/create/remote.go`.
//!
//! Owner: Wave B task T15 (resource-factories).

//! Go `create/remote.go`: `resources.GetRemote(uri, options)`. Cache key (`remoteResourceKeys`):
//! with no options, `optionsKey = userKey = hashing.HashString(uri, map[string]any(nil))` (decimal).
//! The response comes from the getresource file cache (a raw HTTP dump) via the httpcache transport;
//! media type from Content-Type (`application/json; charset=UTF-8` -> `application/json`), else
//! `http.DetectContentType` + extension hints. The result is a resource (FromRemote) whose
//! `.Content` is the body.
//!
//! The Rust port has no network code: the transport's inner round tripper is
//! [`NoNetwork`](nh_helpers::cache::httpcache::transport::NoNetwork), so a request whose response
//! is not in the getresource file cache fails (Go would fetch it, with retries on temporary
//! errors: `transport.RoundTrip`).

use std::sync::Arc;

use go_value::{GoString, Map, MapType, SliceType, Value};
use nh_common::Result;
use nh_common::herrors::Error;
use nh_config::decode::FieldRef;
use nh_helpers::cache::httpcache::transport::{Header, Request, Response};
use nh_media::media::media_type::MediaType;
use nh_resource::resourcetypes::Resource;
use nh_resources::resource::ResourceSourceDescriptor;

use super::create::{Client, nil_resource, nil_to_none};
use super::mime;

/// Go: `create.HTTPError` — an error with the response data (`Data`: `StatusCode`, `Status`,
/// `Body`, ...). The tpl namespace turns it into a `resource.ResourceError`.
#[derive(Clone, Debug)]
pub struct HttpError {
    pub err: Error,
    pub data: Map,
    pub status_code: i64,
    pub body: String,
}

/// Go: `responseToData(res, readBody, includeHeaders)`.
// Go: resources/resource_factories/create/remote.go:responseToData
fn response_to_data(res: &Response, read_body: bool, include_headers: &[String]) -> Map {
    let mut body: Vec<u8> = Vec::new();
    if read_body {
        body = res.body.clone();
    }

    let mut response_headers = Map::new(MapType::Named(Arc::from("map[string][]string")));
    if !include_headers.is_empty() {
        for (k, v) in &res.header.0 {
            let arr: Vec<&str> = include_headers.iter().map(String::as_str).collect();
            if nh_common::hstrings::in_slice_equal_fold(&arr, k) {
                response_headers.insert(
                    k.as_str(),
                    Value::list(
                        SliceType::String,
                        v.iter()
                            .map(|s| Value::String(GoString::from(s.clone())))
                            .collect(),
                    ),
                );
            }
        }
    }

    let mut m = Map::new(MapType::StringAny);
    m.insert("StatusCode", Value::int(res.status_code));
    m.insert("Status", Value::String(GoString::from(res.status.clone())));
    m.insert(
        "TransferEncoding",
        if res.transfer_encoding.is_empty() {
            Value::TypedNil(Arc::from("[]string"))
        } else {
            Value::string_list(res.transfer_encoding.iter().map(String::as_str))
        },
    );
    m.insert("ContentLength", Value::int64(res.content_length));
    m.insert(
        "ContentType",
        Value::String(GoString::from(res.header.get("Content-Type").to_vec())),
    );
    m.insert("Headers", Value::map(response_headers));

    if read_body {
        m.insert("Body", Value::String(GoString::from(body)));
    }

    m
}

/// Go: `toHTTPError(err, res, readBody, responseHeaders)`.
// Go: resources/resource_factories/create/remote.go:toHTTPError
fn to_http_error(
    err: Error,
    res: Option<&Response>,
    read_body: bool,
    response_headers: &[String],
) -> HttpError {
    let Some(res) = res else {
        return HttpError {
            err,
            data: Map::new(MapType::StringAny),
            status_code: 0,
            body: String::new(),
        };
    };

    HttpError {
        err,
        data: response_to_data(res, read_body, response_headers),
        status_code: 0,
        body: String::new(),
    }
}

/// Go: `fromRemoteOptions` (decoded with `mapstructure.WeakDecode`).
#[derive(Clone, Debug)]
pub struct FromRemoteOptions {
    pub method: String,
    /// Go `Headers map[string]any` (an empty map when not set: Go's nil map adds no headers
    /// either).
    pub headers: Map,
    /// Go `Body []byte` (empty = nil: no body).
    pub body: Vec<u8>,
    pub response_headers: Vec<String>,
}

impl Default for FromRemoteOptions {
    fn default() -> Self {
        FromRemoteOptions {
            method: String::new(),
            headers: Map::new(MapType::StringAny),
            body: Vec::new(),
            response_headers: Vec::new(),
        }
    }
}

nh_config::decode_struct!(FromRemoteOptions, "create.fromRemoteOptions", |s| vec![
    FieldRef::new("Method", &mut s.method),
    FieldRef::new("Headers", &mut s.headers),
    FieldRef::new("Body", &mut s.body),
    FieldRef::new("ResponseHeaders", &mut s.response_headers),
]);

impl FromRemoteOptions {
    /// Go: `BodyReader()` — nil without a body (the request is never sent by the port; the
    /// body only matters to the network).
    // Go: resources/resource_factories/create/remote.go:BodyReader
    pub fn body_reader(&self) -> Option<&[u8]> {
        if self.body.is_empty() {
            return None;
        }
        Some(&self.body)
    }

    /// Go: `NewRequest(url)` — the request with the user provided headers, then the default
    /// headers not provided by the user.
    // Go: resources/resource_factories/create/remote.go:NewRequest
    pub fn new_request(&self, url: &str) -> Result<Request> {
        let req = new_http_request(&self.method, url)?;
        let mut req = req;

        // First add any user provided headers.
        if !self.headers.entries.is_empty() {
            add_user_provided_headers(&self.headers, &mut req);
        }

        // Then add default headers not provided by the user.
        add_default_headers(&mut req);

        Ok(req)
    }
}

/// Go `http.NewRequest(method, url, body)`: the method must be a token, the URL must parse.
// Go: net/http/request.go:NewRequestWithContext
fn new_http_request(method: &str, url: &str) -> Result<Request> {
    let method = if method.is_empty() {
        // We document that "" means "GET" for Request.Method, and people have relied on that
        // from NewRequest, so keep that working.
        "GET"
    } else {
        method
    };
    if !valid_method(method) {
        return Err(Error::new(format!(
            "net/http: invalid method {}",
            go_strconv::quote(method)
        )));
    }
    let u = go_url::parse(url).map_err(|e| Error::new(e.to_string()))?;
    Ok(Request {
        method: method.to_string(),
        url: String::from_utf8_lossy(&u.string()).into_owned(),
        header: Header::default(),
    })
}

// Go: net/http/http.go:validMethod (isNotToken over runes)
fn valid_method(method: &str) -> bool {
    !method.is_empty()
        && method.bytes().all(|c| {
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
        })
}

/// Go: `remoteResourceKeys(uri, optionsm)` -> (userKey, optionsKey). A `key` option (looked up
/// case-insensitively) is removed from the options before they are hashed.
// Go: resources/resource_factories/create/remote.go:remoteResourceKeys
pub fn remote_resource_keys(uri: &str, options: Option<&Map>) -> (String, String) {
    let mut optionsm = options.cloned();
    let (user_key, options_key) = remote_resource_keys_mut(uri, optionsm.as_mut());
    (user_key, options_key)
}

/// [`remote_resource_keys`] over the caller's (cloned) options, from which the `key` option is
/// deleted (Go deletes it from the map `FromRemote` goes on to decode).
fn remote_resource_keys_mut(uri: &str, optionsm: Option<&mut Map>) -> (String, String) {
    let mut user_key = String::new();
    let mut optionsm = optionsm;
    if let Some(m) = optionsm.as_deref_mut()
        && let Some((key, k)) = nh_common::maps::maps::lookup_equal_fold(m, b"key")
    {
        let (key, k) = (key.clone(), k.clone());
        user_key = nh_common::hashing::hash_string(&[key]);
        m.entries.remove(&k);
    }
    let options_value = match optionsm {
        Some(m) => Value::map(m.clone()),
        None => Value::TypedNil(Arc::from("map[string]interface {}")),
    };
    let options_key = nh_common::hashing::hash_string(&[Value::string(uri), options_value]);
    if user_key.is_empty() {
        user_key = options_key.clone();
    }
    (user_key, options_key)
}

/// Go: `addDefaultHeaders(req)`.
// Go: resources/resource_factories/create/remote.go:addDefaultHeaders
fn add_default_headers(req: &mut Request) {
    if !has_header_key(&req.header, "User-Agent") {
        req.header.add("User-Agent", b"Hugo Static Site Generator");
    }
}

/// Go: `addUserProvidedHeaders(headers, req)`. Go ranges over the map (random order); only keys
/// that canonicalise to the same header can see it, the port adds them in sorted key order.
// Go: resources/resource_factories/create/remote.go:addUserProvidedHeaders
fn add_user_provided_headers(headers: &Map, req: &mut Request) {
    let mut keys: Vec<&GoString> = headers.entries.keys().collect();
    keys.sort_by(|a, b| a.as_bytes().cmp(b.as_bytes()));
    for key in keys {
        let val = &headers.entries[key];
        let vals = nh_common::types::convert::to_string_slice_preserve_string(val);
        for s in vals {
            req.header
                .add(&String::from_utf8_lossy(key.as_bytes()), s.as_bytes());
        }
    }
}

// Go: resources/resource_factories/create/remote.go:hasHeaderKey
fn has_header_key(m: &Header, key: &str) -> bool {
    m.raw(key).is_some()
}

/// Go: `decodeRemoteOptions(optionsm)`.
// Go: resources/resource_factories/create/remote.go:decodeRemoteOptions
fn decode_remote_options(optionsm: Option<&Map>) -> Result<FromRemoteOptions> {
    let mut options = FromRemoteOptions {
        method: "GET".to_string(),
        ..Default::default()
    };

    let input = match optionsm {
        Some(m) => Value::map(m.clone()),
        None => Value::TypedNil(Arc::from("map[string]interface {}")),
    };
    nh_config::decode::weak_decode_into(&input, &mut options)?;
    options.method = go_unicode::strings::to_upper_str(&options.method).into_owned();

    Ok(options)
}

/// We need to send the redirect responses back to the HTTP client from RoundTrip, but we don't
/// want to cache them.
// Go: resources/resource_factories/create/remote.go:shouldCache
pub(crate) fn should_cache(status_code: i64) -> bool {
    !matches!(status_code, 301 | 302 | 303 | 307 | 308)
}

/// Go `http.StatusText(code)`.
// Go: net/http/status.go:StatusText
fn status_text(code: i64) -> &'static str {
    match code {
        100 => "Continue",
        101 => "Switching Protocols",
        102 => "Processing",
        103 => "Early Hints",
        200 => "OK",
        201 => "Created",
        202 => "Accepted",
        203 => "Non-Authoritative Information",
        204 => "No Content",
        205 => "Reset Content",
        206 => "Partial Content",
        207 => "Multi-Status",
        208 => "Already Reported",
        226 => "IM Used",
        300 => "Multiple Choices",
        301 => "Moved Permanently",
        302 => "Found",
        303 => "See Other",
        304 => "Not Modified",
        305 => "Use Proxy",
        307 => "Temporary Redirect",
        308 => "Permanent Redirect",
        400 => "Bad Request",
        401 => "Unauthorized",
        402 => "Payment Required",
        403 => "Forbidden",
        404 => "Not Found",
        405 => "Method Not Allowed",
        406 => "Not Acceptable",
        407 => "Proxy Authentication Required",
        408 => "Request Timeout",
        409 => "Conflict",
        410 => "Gone",
        411 => "Length Required",
        412 => "Precondition Failed",
        413 => "Request Entity Too Large",
        414 => "Request URI Too Long",
        415 => "Unsupported Media Type",
        416 => "Requested Range Not Satisfiable",
        417 => "Expectation Failed",
        418 => "I'm a teapot",
        421 => "Misdirected Request",
        422 => "Unprocessable Entity",
        423 => "Locked",
        424 => "Failed Dependency",
        425 => "Too Early",
        426 => "Upgrade Required",
        428 => "Precondition Required",
        429 => "Too Many Requests",
        431 => "Request Header Fields Too Large",
        451 => "Unavailable For Legal Reasons",
        500 => "Internal Server Error",
        501 => "Not Implemented",
        502 => "Bad Gateway",
        503 => "Service Unavailable",
        504 => "Gateway Timeout",
        505 => "HTTP Version Not Supported",
        506 => "Variant Also Negotiates",
        507 => "Insufficient Storage",
        508 => "Loop Detected",
        510 => "Not Extended",
        511 => "Network Authentication Required",
        _ => "",
    }
}

/// Go `(*http.Client).Do` error wrapping: `*url.Error{Op, URL, Err}` (`Get "https://...": err`).
// Go: net/http/client.go:urlErrorOp
fn url_error(req: &Request, err: &Error) -> Error {
    let method = if req.method.is_empty() {
        "GET"
    } else {
        req.method.as_str()
    };
    let op = format!(
        "{}{}",
        &method[..1],
        go_unicode::strings::to_lower_str(&method[1..])
    );
    Error::new(format!(
        "{op} {}: {}",
        go_strconv::quote(&req.url),
        err.message()
    ))
}

impl Client {
    /// Go: `configurePollingIfEnabled` — the remote resource checker only exists when watching
    /// (server mode, which `New` rejects), so this never polls.
    // Go: resources/resource_factories/create/remote.go:configurePollingIfEnabled
    fn configure_polling_if_enabled(&self, _uri: &str, _options_key: &str) {}

    /// Go: `Client.FromRemote(uri, optionsm)` — `None` for a 404 (as for a missing local
    /// resource). An HTTP error status is an [`HttpError`]'s error (see
    /// [`Client::from_remote_http`] for its data).
    // Go: resources/resource_factories/create/remote.go:FromRemote
    pub fn from_remote(
        &self,
        uri: &str,
        options: Option<&Map>,
    ) -> Result<Option<Arc<dyn Resource>>> {
        self.from_remote_http(uri, options).map_err(|e| e.err)
    }

    /// [`Client::from_remote`] with Go's `*HTTPError` (the response data of an error status).
    pub fn from_remote_http(
        &self,
        uri: &str,
        optionsm: Option<&Map>,
    ) -> std::result::Result<Option<Arc<dyn Resource>>, Box<HttpError>> {
        let plain = |err: Error| Box::new(to_http_error(err, None, false, &[]));

        let r_url = go_url::parse(uri).map_err(|err| {
            plain(Error::new(format!(
                "failed to parse URL for resource {uri}: {err}"
            )))
        })?;

        let mut method = "GET".to_string();
        if let Some(m) = optionsm
            && let Some((s, _)) = nh_common::maps::maps::lookup_equal_fold(m, b"method")
        {
            let Value::String(s) = s else {
                // Go: `s.(string)` panics.
                return Err(plain(Error::new(format!(
                    "interface conversion: interface {{}} is {}, not string",
                    match s {
                        Value::Invalid => "nil".into(),
                        v => v.go_type_name(),
                    }
                ))));
            };
            method = go_unicode::strings::to_upper_str(&String::from_utf8_lossy(s.as_bytes()))
                .into_owned();
        }
        let is_head_method = method == "HEAD";

        let mut optionsm = optionsm.cloned();
        let (user_key, options_key) = remote_resource_keys_mut(uri, optionsm.as_mut());

        // A common pattern is to use the key in the options map as a way to control cache
        // eviction, so make sure we use any user provided key as the file cache key, but the
        // auto generated and more stable key for everything else.
        let filecache_key = user_key.clone();

        let http_error: std::cell::RefCell<Option<HttpError>> = std::cell::RefCell::new(None);
        let res = self
            .rs
            .resource_cache()
            .cache_resource_remote
            .get_or_create(options_key.clone(), |_key| {
                let fail = |e: HttpError| -> Error {
                    let err = e.err.clone();
                    *http_error.borrow_mut() = Some(e);
                    err
                };
                let options = decode_remote_options(optionsm.as_ref()).map_err(|err| {
                    Error::new(format!(
                        "failed to decode options for resource {uri}: {err}"
                    ))
                })?;

                self.validate_from_remote_args(uri, &options)?;

                let get_res = || -> Result<Response> {
                    let req = options.new_request(uri).map_err(|err| {
                        Error::new(format!(
                            "failed to create request for resource {uri}: {err}"
                        ))
                    })?;

                    let mut t = (*self.http_client).clone();
                    let key = filecache_key.clone();
                    t.cache_key = Some(Arc::new(move |_req| key.clone()));
                    t.round_trip(&req).map_err(|err| url_error(&req, &err))
                };

                let res = get_res()?;

                self.configure_polling_if_enabled(uri, &options_key);

                if res.status_code == 404 {
                    // Not found. This matches how lookups for local resources work.
                    return Err(nil_resource());
                }

                if res.status_code < 200 || res.status_code > 299 {
                    return Err(fail(to_http_error(
                        Error::new(format!(
                            "failed to fetch remote resource from '{uri}': {}",
                            status_text(res.status_code)
                        )),
                        Some(&res),
                        !is_head_method,
                        &options.response_headers,
                    )));
                }

                let mut body: Vec<u8> = Vec::new();
                // A response to a HEAD method should not have a body. If it has one anyway, that
                // body must be ignored.
                if !is_head_method {
                    if let Some(err) = &res.body_err {
                        return Err(Error::new(format!(
                            "failed to read remote resource {}: {err}",
                            go_strconv::quote(uri)
                        )));
                    }
                    body = res.body.clone();
                }

                let r_path = String::from_utf8_lossy(&r_url.path).into_owned();
                let mut filename = go_path::path::base(&r_path).to_string();
                if let Ok((_, params)) =
                    mime::parse_media_type(res.header.get("Content-Disposition"))
                    && let Some(f) = params.get(b"filename".as_slice())
                {
                    filename = String::from_utf8_lossy(f).into_owned();
                }

                let content_type =
                    String::from_utf8_lossy(res.header.get("Content-Type")).into_owned();

                let mut media_type = MediaType::default();
                // For HEAD requests we have no body to work with, so we need to use the
                // Content-Type header.
                if is_head_method
                    || self
                        .rs
                        .exec_helper
                        .sec()
                        .http
                        .media_types
                        .accept(&content_type)
                {
                    let (mt, found) = self.rs.media_types().get_by_type_found(&content_type);
                    media_type = mt;
                    if !found {
                        // A media type not configured in Hugo, just create one from the
                        // content type string.
                        media_type = MediaType::from_string(&content_type).unwrap_or_default();
                    }
                }

                if media_type.is_zero() {
                    let mut extension_hints: Option<Vec<String>> = None;

                    // mime.ExtensionsByType gives a long list of extensions for text/plain,
                    // just use ".txt".
                    if content_type.starts_with("text/plain") {
                        extension_hints = Some(vec![".txt".to_string()]);
                    } else if let Ok(Some(exts)) = mime::extensions_by_type(content_type.as_bytes())
                    {
                        extension_hints = Some(
                            exts.iter()
                                .map(|e| String::from_utf8_lossy(e).into_owned())
                                .collect(),
                        );
                    }

                    // Look for a file extension. If it's .txt, look for a more specific.
                    if extension_hints
                        .as_ref()
                        .is_none_or(|h| h.first().map(String::as_str) == Some(".txt"))
                    {
                        let ext = go_path::path::ext(&filename);
                        if !ext.is_empty() {
                            extension_hints = Some(vec![ext.to_string()]);
                        }
                    }

                    // Now resolve the media type primarily using the content.
                    media_type = nh_media::media::media_type::from_content(
                        &self.rs.media_types(),
                        extension_hints.as_deref().unwrap_or(&[]),
                        &body,
                    );
                }

                if media_type.is_zero() {
                    return Err(Error::new(format!(
                        "failed to resolve media type for remote resource {}",
                        go_strconv::quote(uri)
                    )));
                }

                let target_path = format!(
                    "{}_{}{}",
                    &filename[..filename.len() - go_path::path::ext(&filename).len()],
                    user_key,
                    media_type.first_suffix.full_suffix
                );
                let data = response_to_data(&res, false, &options.response_headers);

                let open: nh_common::hugio::OpenReadSeekCloser =
                    Arc::new(move || Ok(nh_common::hugio::read_seeker_from_bytes(body.clone())));
                self.rs.new_resource(ResourceSourceDescriptor {
                    media_type: Some(media_type),
                    data: Some(data),
                    lazy_publish: true,
                    open_read_seek_closer: Some(open),
                    target_path,
                    ..Default::default()
                })
            });

        match nil_to_none(res) {
            Ok(r) => Ok(r),
            Err(err) => match http_error.into_inner() {
                Some(he) if he.err.message() == err.message() => Err(Box::new(he)),
                _ => Err(plain(err)),
            },
        }
    }

    // Go: resources/resource_factories/create/remote.go:validateFromRemoteArgs
    fn validate_from_remote_args(&self, uri: &str, options: &FromRemoteOptions) -> Result<()> {
        let sec = self.rs.exec_helper.sec();
        sec.check_allowed_http_url(uri)?;

        sec.check_allowed_http_method(&options.method)?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn go_tables() {
        assert!(should_cache(200));
        assert!(should_cache(404));
        assert!(!should_cache(302));
        assert_eq!(status_text(418), "I'm a teapot");
        assert_eq!(status_text(299), "");
        assert!(valid_method("GET"));
        assert!(!valid_method("GE T"));
        assert!(!valid_method(""));
    }
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: resources/resource_factories/create/remote.go (469 lines; 10/14 funcs executed)
//   types: HTTPError, fromRemoteOptions, transport
// OK L55-84: responseToData(res *http.Response, readBody bool, includeHeaders []string) map[string]any
// OK L86-101: toHTTPError(err error, res *http.Response, readBody bool, responseHeaders []string) *HTTPError
// OK L112-164: (c *Client) configurePollingIfEnabled(uri, optionsKey string, getRes func() (*http.Response, error))
// OK L168-308: (c *Client) FromRemote(uri string, optionsm map[string]any) (resource.Resource, error)
// OK L310-320: (c *Client) validateFromRemoteArgs(uri string, options fromRemoteOptions) error
// OK L322-333: remoteResourceKeys(uri string, optionsm map[string]any) (string, string)
// OK L335-339: addDefaultHeaders(req *http.Request)
// OK L341-351: addUserProvidedHeaders(headers map[string]any, req *http.Request)
// OK L353-356: hasHeaderKey(m http.Header, key string) bool
// OK L365-370: (o fromRemoteOptions) BodyReader() io.Reader
// OK L372-387: (o fromRemoteOptions) NewRequest(url string) (*http.Request, error)
// OK L389-401: decodeRemoteOptions(optionsm map[string]any) (fromRemoteOptions, error)
// STUB L410-459: (t *transport) RoundTrip(req *http.Request) (resp *http.Response, err error) — the network (retries on temporary statuses): NoNetwork
// OK L463-469: shouldCache(statusCode int) bool
// ---------------------------------------------------------------------------
