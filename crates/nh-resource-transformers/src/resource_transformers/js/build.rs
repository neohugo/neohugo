//! Port of `resources/resource_transformers/js/build.go`.
//!
//! Owner: Wave B task T16 (js-css-pipeline).

use std::sync::Arc;

use go_value::Map;
use nh_common::Result;
use nh_common::herrors::Error;
use nh_esbuild::build::{BuildClient, BuildResult};
use nh_esbuild::options::Options;
use nh_resource::resourcetypes::Resource;
use nh_resources::resource_spec::Spec;
use nh_resources::transform::ResourceTransformationCtx;

use super::transform::BuildTransformation;

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

    /// Go's `New(fs, rs)`: the build client over the assets filesystem, with an esbuild service
    /// started on the first build.
    pub fn new_default(rs: Arc<Spec>) -> Client {
        let sfs = rs.path_spec.base_fs.assets.clone();
        let c = BuildClient::new_default(sfs, rs.clone());
        Client { rs, c }
    }

    /// Process processes a resource with the user provided options. `r` must be a resource
    /// adapter (Go's `resources.ResourceTransformer`); Go's `Process` calls `res.Transform`
    /// (no context).
    // Go: resources/resource_transformers/js/build.go:Process
    pub fn process(
        &self,
        _ctx: &nh_tpl::template::TplContext,
        r: Arc<dyn Resource>,
        opts: Option<&Map>,
    ) -> Result<Arc<dyn Resource>> {
        let res = nh_resources::transform::resource_adapter(&r)
            .ok_or_else(|| Error::new(format!("{} can not be transformed", r.tpl_type_name())))?;
        Ok(res.transform(vec![Arc::new(BuildTransformation {
            optsm: opts.cloned(),
            c: self.c.clone(),
        })])?)
    }
}

// Go: resources/resource_transformers/js/build.go:transform
pub(super) fn transform(
    c: &BuildClient,
    mut opts: Options,
    transform_ctx: &mut ResourceTransformationCtx<'_>,
) -> Result<BuildResult> {
    opts.internal.stdin_source_path = transform_ctx.source_path.clone();

    let result = c.build(opts.clone())?;

    let source_map = opts.external.source_map.as_str();
    if source_map == "linked" || source_map == "external" {
        let (Some(map), Some(js)) = (result.output_files.first(), result.output_files.get(1))
        else {
            return Err(Error::new(
                "runtime error: index out of range [1] with length 1",
            ));
        };
        let mut content = js.contents.clone();
        if source_map == "linked" {
            let sym_path = format!("{}.map", go_path::path::base(&transform_ctx.out_path));
            let re = nh_common::goregexp::Regexp::must_compile(r"//# sourceMappingURL=.*\n?");
            content = re.replace_all(
                &content,
                format!("//# sourceMappingURL={sym_path}\n").as_bytes(),
            );
        }

        transform_ctx.publish_source_map(&String::from_utf8_lossy(&map.contents))?;
        transform_ctx.to.extend_from_slice(&content);
    } else {
        let Some(js) = result.output_files.first() else {
            return Err(Error::new(
                "runtime error: index out of range [0] with length 0",
            ));
        };
        transform_ctx.to.extend_from_slice(&js.contents);
    }
    Ok(result)
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: resources/resource_transformers/js/build.go (82 lines; 3/3 funcs executed)
//   types: Client
// OK L34-38: New(fs *filesystems.SourceFilesystem, rs *resources.Spec) *Client
// OK L41-45: (c *Client) Process(res resources.ResourceTransformer, opts map[string]any) (resource.Resource, error)
// OK L47-82: (c *Client) transform(opts esbuild.Options, transformCtx *resources.ResourceTransformationCtx) (api.BuildResult, error)
// ---------------------------------------------------------------------------
