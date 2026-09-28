//! Port of `resources/resource_transformers/js/transform.go`.
//!
//! Owner: Wave B task T16 (js-css-pipeline).

//! Go `js/transform.go`: the `jsbuild` ResourceTransformation (OutMediaType text/javascript; OutPath =
//! TargetPath or ReplaceOutPathExtension(".js"); SourceDir = dir(SourcePath); Stdin with contents).

use std::sync::Arc;

use go_value::{Map, Value};
use nh_common::Result;
use nh_esbuild::build::BuildClient;
use nh_esbuild::options::Options;
use nh_resource::internal::key::ResourceTransformationKey;
use nh_resources::transform::{ResourceTransformation, ResourceTransformationCtx};

/// Go: `buildTransformation`.
pub(crate) struct BuildTransformation {
    /// Go `optsm map[string]any` (`None` = nil).
    pub(crate) optsm: Option<Map>,
    pub(crate) c: Arc<BuildClient>,
}

impl ResourceTransformation for BuildTransformation {
    // Go: resources/resource_transformers/js/transform.go:(*buildTransformation).Key
    fn key(&self) -> ResourceTransformationKey {
        ResourceTransformationKey::new("jsbuild", vec![opts_value(self.optsm.as_ref())])
    }

    // Go: resources/resource_transformers/js/transform.go:(*buildTransformation).Transform
    fn transform(&self, ctx: &mut ResourceTransformationCtx<'_>) -> Result<()> {
        ctx.out_media_type = nh_media::media::builtin::builtin().javascript_type.clone();

        let mut opts = Options::default();

        if let Some(m) = &self.optsm {
            let opts_ext = nh_esbuild::options::decode_external_options(m)?;
            opts.external = opts_ext;
        }

        if !opts.external.target_path.is_empty() {
            ctx.out_path = opts.external.target_path.clone();
        } else {
            ctx.replace_out_path_extension(".js");
        }

        let src = ctx.from.read_all()?;

        opts.internal.source_dir =
            go_path::filepath::from_slash(&go_path::path::dir(&ctx.source_path)).to_string();
        opts.internal.contents = src;
        opts.internal.media_type = ctx.in_media_type.clone();
        opts.internal.stdin = true;

        super::build::transform(&self.c, opts, ctx)?;

        Ok(())
    }
}

/// A Go `map[string]any` option argument as a template value (a nil map is a typed nil, which
/// hashes differently from an empty map).
pub(crate) fn opts_value(m: Option<&Map>) -> Value {
    match m {
        Some(m) => Value::map(m.clone()),
        None => Value::TypedNil(Arc::from("map[string]interface {}")),
    }
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: resources/resource_transformers/js/transform.go (68 lines; 2/2 funcs executed)
//   types: buildTransformation
// OK L32-34: (t *buildTransformation) Key() internal.ResourceTransformationKey
// OK L36-68: (t *buildTransformation) Transform(ctx *resources.ResourceTransformationCtx) error
// ---------------------------------------------------------------------------
