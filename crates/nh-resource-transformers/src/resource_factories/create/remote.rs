//! Port of `resources/resource_factories/create/remote.go`.
//!
//! Owner: Wave B task T15 (resource-factories).


//! Go `create/remote.go`: `resources.GetRemote(uri, options)`. Cache key (`remoteResourceKeys`):
//! with no options, `optionsKey = userKey = hashing.HashString(uri, map[string]any(nil))` (decimal).
//! The response comes from the getresource file cache (a raw HTTP dump) via the httpcache transport;
//! media type from Content-Type (`application/json; charset=UTF-8` -> `application/json`), else
//! `http.DetectContentType` + extension hints. The result is a resource (FromRemote) whose
//! `.Content` is the body.

use std::sync::Arc;

use go_value::{Map, Value};
use nh_common::Result;
use nh_resource::resourcetypes::Resource;

use super::create::Client;

/// Go: `fromRemoteOptions` (method, headers, body, key).
#[derive(Clone, Debug, Default)]
pub struct FromRemoteOptions {
    pub method: String,
    pub headers: std::collections::BTreeMap<String, Vec<String>>,
    pub body: Vec<u8>,
    pub key: Option<String>,
}

/// Go: `remoteResourceKeys(uri, optionsm)` -> (userKey, optionsKey).
// Go: resources/resource_factories/create/remote.go:remoteResourceKeys
pub fn remote_resource_keys(uri: &str, options: Option<&Map>) -> (String, String) {
    todo!()
}

impl Client {
    /// Go: `Client.FromRemote(uri, optionsm)`.
    // Go: resources/resource_factories/create/remote.go:FromRemote
    pub fn from_remote(&self, uri: &str, options: Option<&Map>) -> Result<Option<Arc<dyn Resource>>> {
        todo!()
    }
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: resources/resource_factories/create/remote.go (469 lines; 10/14 funcs executed)
//   types: HTTPError, fromRemoteOptions, transport
// EX L55-84: responseToData(res *http.Response, readBody bool, includeHeaders []string) map[string]any
//    L86-101: toHTTPError(err error, res *http.Response, readBody bool, responseHeaders []string) *HTTPError
// EX L112-164: (c *Client) configurePollingIfEnabled(uri, optionsKey string, getRes func() (*http.Response, error))
// EX L168-308: (c *Client) FromRemote(uri string, optionsm map[string]any) (resource.Resource, error)
// EX L310-320: (c *Client) validateFromRemoteArgs(uri string, options fromRemoteOptions) error
// EX L322-333: remoteResourceKeys(uri string, optionsm map[string]any) (string, string)
// EX L335-339: addDefaultHeaders(req *http.Request)
//    L341-351: addUserProvidedHeaders(headers map[string]any, req *http.Request)
// EX L353-356: hasHeaderKey(m http.Header, key string) bool
// EX L365-370: (o fromRemoteOptions) BodyReader() io.Reader
// EX L372-387: (o fromRemoteOptions) NewRequest(url string) (*http.Request, error)
// EX L389-401: decodeRemoteOptions(optionsm map[string]any) (fromRemoteOptions, error)
//    L410-459: (t *transport) RoundTrip(req *http.Request) (resp *http.Response, err error)
//    L463-469: shouldCache(statusCode int) bool
// ---------------------------------------------------------------------------
