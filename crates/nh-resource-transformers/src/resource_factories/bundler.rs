//! Port of `resources/resource_factories/bundler/bundler.go`.
//!
//! Owner: Wave B task T15 (resource-factories).


//! Go `resource_factories/bundler`: `resources.Concat(targetPath, resources)`. Cached by TARGET PATH
//! ONLY (first caller wins for the whole build, across languages). All parts must share the media
//! type; for JavaScript parts are joined with `"\n;\n"`; the composite's media type comes from the
//! target path extension; LazyPublish.

use std::sync::Arc;

use nh_common::Result;
use nh_resource::resourcetypes::{Resource, Resources};
use nh_resources::resource_spec::Spec;

/// Go: `bundler.Client`.
pub struct Client {
    pub rs: Arc<Spec>,
}

impl Client {
    // Go: resources/resource_factories/bundler/bundler.go:New
    pub fn new(rs: Arc<Spec>) -> Client {
        Client { rs }
    }

    // Go: resources/resource_factories/bundler/bundler.go:Concat
    pub fn concat(&self, target_path: &str, r: &Resources) -> Result<Arc<dyn Resource>> {
        todo!()
    }
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: resources/resource_factories/bundler/bundler.go (171 lines; 6/7 funcs executed)
//   types: Client, multiReadSeekCloser
// EX L36-38: New(rs *resources.Spec) *Client
// EX L45-51: toReaders(sources []hugio.ReadSeekCloser) []io.Reader
// EX L53-56: newMultiReadSeekCloser(sources ...hugio.ReadSeekCloser) *multiReadSeekCloser
// EX L58-60: (r *multiReadSeekCloser) Read(p []byte) (n int, err error)
//    L62-73: (r *multiReadSeekCloser) Seek(offset int64, whence int) (newOffset int64, err error)
// EX L75-80: (r *multiReadSeekCloser) Close() error
// EX L83-171: (c *Client) Concat(targetPath string, r resource.Resources) (resource.Resource, error)
// ---------------------------------------------------------------------------
