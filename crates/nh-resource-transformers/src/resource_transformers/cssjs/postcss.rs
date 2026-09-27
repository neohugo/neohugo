//! Port of `resources/resource_transformers/cssjs/postcss.go`.
//!
//! Owner: Wave B task T16 (js-css-pipeline).


//! Go `cssjs/postcss.go`: spawn `node_modules/.bin/postcss --config <abs postcss.config.js>` via
//! `hexec.Npx` with stdin = CSS, stdout = result, env = `GetExecEnviron` (NODE_PATH, PWD,
//! HUGO_ENVIRONMENT=production, HUGO_FILE_*...), cwd INHERITED (purgecss reads `./hugo_stats.json`).

use std::sync::Arc;

use go_value::Map;
use nh_common::Result;
use nh_resource::resourcetypes::Resource;
use nh_resources::resource_spec::Spec;

/// Go: `cssjs.PostCSSOptions`.
#[derive(Clone, Debug, Default)]
pub struct PostCssOptions {
    pub config: String,
    pub no_map: bool,
    pub use_: String,
    pub parser: String,
    pub stringifier: String,
    pub syntax: String,
    pub inline_imports: bool,
    pub skip_inline_imports_not_found: bool,
}

/// Go: `cssjs.PostCSSClient`.
pub struct PostCssClient {
    pub rs: Arc<Spec>,
}

impl PostCssClient {
    // Go: resources/resource_transformers/cssjs/postcss.go:Process
    pub fn process(&self, ctx: &nh_tpl::template::TplContext, r: Arc<dyn Resource>, options: Option<&Map>) -> Result<Arc<dyn Resource>> {
        todo!()
    }
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: resources/resource_transformers/cssjs/postcss.go (239 lines; 6/6 funcs executed)
//   types: PostCSSClient, InlineImports, PostCSSOptions, postcssTransformation
// EX L41-43: NewPostCSSClient(rs *resources.Spec) *PostCSSClient
// EX L45-59: decodePostCSSOptions(m map[string]any) (opts PostCSSOptions, err error)
// EX L67-69: (c *PostCSSClient) Process(res resources.ResourceTransformer, options map[string]any) (resource.Resource, error)
// EX L108-127: (opts PostCSSOptions) toArgs() []string
// EX L134-136: (t *postcssTransformation) Key() internal.ResourceTransformationKey
// EX L142-239: (t *postcssTransformation) Transform(ctx *resources.ResourceTransformationCtx) error
// ---------------------------------------------------------------------------
