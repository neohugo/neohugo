//! Port of `resources/resource_transformers/js/build.go`.
//!
//! Owner: Wave B task T16 (js-css-pipeline).


use std::sync::Arc;

use go_value::Map;
use nh_common::Result;
use nh_esbuild::build::BuildClient;
use nh_resource::resourcetypes::Resource;
use nh_resources::resource_spec::Spec;

/// Go: `js.Client` (`js.Build`).
pub struct Client {
    pub rs: Arc<Spec>,
    pub c: Arc<BuildClient>,
}

impl Client {
    // Go: resources/resource_transformers/js/build.go:New
    pub fn new(rs: Arc<Spec>, c: Arc<BuildClient>) -> Client {
        Client { rs, c }
    }

    // Go: resources/resource_transformers/js/build.go:Process
    pub fn process(&self, ctx: &nh_tpl::template::TplContext, r: Arc<dyn Resource>, opts: Option<&Map>) -> Result<Arc<dyn Resource>> {
        todo!()
    }
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: resources/resource_transformers/js/build.go (82 lines; 3/3 funcs executed)
//   types: Client
// EX L34-38: New(fs *filesystems.SourceFilesystem, rs *resources.Spec) *Client
// EX L41-45: (c *Client) Process(res resources.ResourceTransformer, opts map[string]any) (resource.Resource, error)
// EX L47-82: (c *Client) transform(opts esbuild.Options, transformCtx *resources.ResourceTransformationCtx) (api.BuildResult, error)
// ---------------------------------------------------------------------------
