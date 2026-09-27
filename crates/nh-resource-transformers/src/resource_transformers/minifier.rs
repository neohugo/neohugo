//! Port of `resources/resource_transformers/minifier/minify.go`.
//!
//! Owner: Wave B task T15 (resource-factories).


//! Go `minifier`: `minify` template func on resources — `AddOutPathIdentifier(".min")` then the
//! tdewolff minifier for the input media type (runs even without `--minify`).

use std::sync::Arc;

use nh_common::Result;
use nh_resource::resourcetypes::Resource;
use nh_resources::resource_spec::Spec;
use nh_transform::minifiers::minifiers::Client as MinifyClient;

/// Go: `minifier.Client`.
pub struct Client {
    pub rs: Arc<Spec>,
    pub m: MinifyClient,
}

impl Client {
    // Go: resources/resource_transformers/minifier/minify.go:New
    pub fn new(rs: Arc<Spec>) -> Result<Client> {
        todo!()
    }

    // Go: resources/resource_transformers/minifier/minify.go:Minify
    pub fn minify(&self, ctx: &nh_tpl::template::TplContext, r: Arc<dyn Resource>) -> Result<Arc<dyn Resource>> {
        todo!()
    }
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: resources/resource_transformers/minifier/minify.go (59 lines; 4/4 funcs executed)
//   types: Client, minifyTransformation
// EX L32-38: New(rs *resources.Spec) (*Client, error)
// EX L45-47: (t *minifyTransformation) Key() internal.ResourceTransformationKey
// EX L49-52: (t *minifyTransformation) Transform(ctx *resources.ResourceTransformationCtx) error
// EX L54-59: (c *Client) Minify(res resources.ResourceTransformer) (resource.Resource, error)
// ---------------------------------------------------------------------------
