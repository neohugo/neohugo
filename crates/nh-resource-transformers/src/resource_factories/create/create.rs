//! Port of `resources/resource_factories/create/create.go`.
//!
//! Owner: Wave B task T15 (resource-factories).


//! Go `resources/resource_factories/create`: `resources.Get` (assets fs, LazyPublish, cache key
//! `cleanKey(path)+"__get"`, missing -> nil), `resources.FromString`, `resources.GetMatch/Match`,
//! `resources.Copy`, and the GetRemote client (remote.go).

use std::sync::Arc;

use go_value::{Map, Value};
use nh_common::Result;
use nh_helpers::cache::httpcache::transport::Transport;
use nh_resource::resourcetypes::{Resource, Resources};
use nh_resources::resource_spec::Spec;

/// Go: `create.Client`.
pub struct Client {
    pub rs: Arc<Spec>,
    /// GetRemote HTTP client over the getresource file cache.
    pub http_client: Arc<Transport>,
}

impl Client {
    // Go: resources/resource_factories/create/create.go:New
    pub fn new(rs: Arc<Spec>) -> Result<Arc<Client>> {
        todo!()
    }

    /// Go: `Client.Get(pathname)` — `None` when the file does not exist.
    // Go: resources/resource_factories/create/create.go:Get
    pub fn get(&self, pathname: &str) -> Result<Option<Arc<dyn Resource>>> {
        todo!()
    }

    // Go: resources/resource_factories/create/create.go:FromString
    pub fn from_string(&self, target_path: &str, content: &[u8]) -> Result<Arc<dyn Resource>> {
        todo!()
    }

    // Go: resources/resource_factories/create/create.go:GetMatch
    pub fn get_match(&self, pattern: &str) -> Result<Option<Arc<dyn Resource>>> {
        todo!()
    }

    // Go: resources/resource_factories/create/create.go:Match
    pub fn match_(&self, pattern: &str) -> Result<Resources> {
        todo!()
    }

    // Go: resources/resource_factories/create/create.go:Copy
    pub fn copy(&self, r: &Arc<dyn Resource>, target_path: &str) -> Result<Arc<dyn Resource>> {
        todo!()
    }
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: resources/resource_factories/create/create.go (304 lines; 3/10 funcs executed)
//   types: Client, contextKey, Options
// EX L65-122: New(rs *resources.Spec) *Client
//    L125-130: (c *Client) Copy(r resource.Resource, targetPath string) (resource.Resource, error)
// EX L133-152: (c *Client) Get(pathname string) (resource.Resource, error)
//    L155-157: (c *Client) Match(pattern string) (resource.Resources, error)
//    L159-165: (c *Client) ByType(tp string) resource.Resources
//    L168-174: (c *Client) GetMatch(pattern string) (resource.Resource, error)
// EX L176-191: (c *Client) getOrCreateFileResource(info hugofs.FileMetaInfo) (resource.Resource, error)
//    L193-223: (c *Client) match(name, pattern string, matchFunc func(r resource.Resource) bool, firstOnly bool) (resource.Resources, error)
//    L244-292: (c *Client) FromOpts(opts Options) (resource.Resource, error)
//    L295-304: (c *Client) FromString(targetPath, content string) (resource.Resource, error)
// ---------------------------------------------------------------------------
