//! Module `cache::httpcache::transport`.
//!
//! NEW: gohugoio/httpcache subset (cached-response read path, AlwaysUseCachedResponse) + Go http.ReadResponse / DumpResponse compatible parser
//!
//! Owner: Wave B task T08 (helpers-source-cache).


//! `github.com/gohugoio/httpcache` subset + Go `net/http` response dump parsing. The getresource
//! file cache stores `httputil.DumpResponse` bytes (`HTTP/2.0 200 OK\r\n` + headers + `\r\n` +
//! body). With `AlwaysUseCachedResponse` (default config) a cache hit is returned without any
//! network access. A miss would need a real HTTP client (not required for the golden build; make
//! it an explicit error unless network is enabled).

use std::collections::BTreeMap;
use std::sync::Arc;

use nh_common::Result;

/// Go: `httpcache.Cache`.
pub trait HttpCache: Send + Sync {
    fn get(&self, key: &str) -> Option<Vec<u8>>;
    fn set(&self, key: &str, resp: &[u8]);
    fn delete(&self, key: &str);
}

/// A parsed HTTP response (Go `*http.Response` subset).
#[derive(Clone, Debug, Default)]
pub struct Response {
    pub status_code: i64,
    pub status: String,
    pub proto: String,
    /// Canonicalised header keys -> values (Go `http.Header`).
    pub header: BTreeMap<String, Vec<String>>,
    pub body: Vec<u8>,
}

impl Response {
    /// Go: `resp.Header.Get(key)`.
    pub fn header_get(&self, key: &str) -> Option<&str> {
        todo!()
    }
}

/// Go: `http.ReadResponse(bufio.NewReader(bytes.NewReader(dump)), req)` (handles chunked bodies,
/// Content-Length).
pub fn read_response(dump: &[u8]) -> Result<Response> {
    todo!()
}

/// Go: `httpcache.Transport` configured like `create.New`: `AlwaysUseCachedResponse`,
/// `MarkCachedResponses`, `EnableETagPair`, `ShouldCache`.
pub struct Transport {
    pub cache: Arc<dyn HttpCache>,
    pub always_use_cached_response: Arc<dyn Fn(&str) -> bool + Send + Sync>,
    pub allow_network: bool,
}

impl Transport {
    /// Performs (or serves from cache) a GET/POST with the given cache key.
    pub fn round_trip(&self, method: &str, url: &str, cache_key: &str, headers: &BTreeMap<String, Vec<String>>, body: &[u8]) -> Result<Response> {
        todo!()
    }
}
