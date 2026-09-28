//! Port of `resources/resource_transformers/minifier/minify.go`.
//!
//! Owner: Wave B task T15 (resource-factories).

//! Go `minifier`: `minify` template func on resources — `AddOutPathIdentifier(".min")` then the
//! tdewolff minifier for the input media type (runs even without `--minify`).

use std::sync::Arc;

use nh_common::Result;
use nh_resource::internal::key::ResourceTransformationKey;
use nh_resource::resourcetypes::Resource;
use nh_resources::resource_spec::Spec;
use nh_resources::transform::{ResourceTransformation, ResourceTransformationCtx};
use nh_transform::minifiers::minifiers::Client as MinifyClient;

/// Go: `minifier.Client` — minification of Resource objects. Supported minifiers are: css,
/// html, js, json, svg and xml.
pub struct Client {
    pub rs: Arc<Spec>,
    pub m: MinifyClient,
}

/// Go: `minifyTransformation`.
struct MinifyTransformation {
    m: MinifyClient,
}

impl ResourceTransformation for MinifyTransformation {
    // Go: resources/resource_transformers/minifier/minify.go:(*minifyTransformation).Key
    fn key(&self) -> ResourceTransformationKey {
        ResourceTransformationKey::new("minify", vec![])
    }

    // Go: resources/resource_transformers/minifier/minify.go:(*minifyTransformation).Transform
    fn transform(&self, ctx: &mut ResourceTransformationCtx<'_>) -> Result<()> {
        ctx.add_out_path_identifier(".min");
        let src = ctx.from.read_all()?;
        let out = self.m.minify(&ctx.in_media_type, &src)?;
        ctx.to.extend_from_slice(&out);
        Ok(())
    }
}

impl Client {
    /// New creates a new Client given a specification. Note that it is the media types
    /// configured for the site that is used to match files to the correct minifier.
    // Go: resources/resource_transformers/minifier/minify.go:New
    pub fn new(rs: Arc<Spec>) -> Result<Client> {
        let m = MinifyClient::new(
            &rs.media_types(),
            &rs.output_formats(),
            rs.path_spec.cfg.as_ref(),
        )?;
        Ok(Client { rs, m })
    }

    /// `r` must be a `resources.ResourceTransformer` (a resource adapter). Go's `Minify` calls
    /// `res.Transform` (no context), so `_ctx` is not used.
    // Go: resources/resource_transformers/minifier/minify.go:Minify
    pub fn minify(
        &self,
        _ctx: &nh_tpl::template::TplContext,
        r: Arc<dyn Resource>,
    ) -> Result<Arc<dyn Resource>> {
        let res = super::integrity::transformer(&r)?;
        Ok(res.transform(vec![Arc::new(MinifyTransformation { m: self.m.clone() })])?)
    }
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: resources/resource_transformers/minifier/minify.go (59 lines; 4/4 funcs executed)
//   types: Client, minifyTransformation
// OK L32-38: New(rs *resources.Spec) (*Client, error)
// OK L45-47: (t *minifyTransformation) Key() internal.ResourceTransformationKey
// OK L49-52: (t *minifyTransformation) Transform(ctx *resources.ResourceTransformationCtx) error
// OK L54-59: (c *Client) Minify(res resources.ResourceTransformer) (resource.Resource, error)
// ---------------------------------------------------------------------------
