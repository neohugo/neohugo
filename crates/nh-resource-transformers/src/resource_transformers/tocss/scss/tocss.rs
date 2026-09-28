//! Port of `resources/resource_transformers/tocss/scss/tocss.go`.
//!
//! Owner: Wave B task T16 (js-css-pipeline).

//! Go `tocss/scss`: LibSass 3.6.6 via libsass-sys (golibsass wrapper): precision 0 -> 8,
//! outputStyle "compressed" = 3, include paths `[<assets real dirs>/scss, node_modules, assets/scss]`,
//! the Hugo importer (resolves `@import` in the assets fs; `prev == "stdin"` -> baseDir), entry
//! `@import "x.css"` protection regexes. OutPath: ReplaceOutPathExtension(".css").

use std::sync::Arc;

use go_value::Map;
use nh_common::Result;
use nh_resource::resourcetypes::Resource;
use nh_resources::resource_spec::Spec;

/// Go: `scss.Options`.
#[derive(Clone, Debug, Default)]
pub struct Options {
    pub target_path: String,
    pub include_paths: Vec<String>,
    /// "nested", "expanded", "compact", "compressed".
    pub output_style: String,
    pub precision: i64,
    pub enable_source_map: bool,
    pub source_map_include_sources: bool,
    pub vars: Option<Map>,
}

/// Go: `scss.Client`.
pub struct Client {
    pub rs: Arc<Spec>,
}

impl Client {
    /// Go: `Client.ToCSS(res, opts)`.
    // Go: resources/resource_transformers/tocss/scss/client_extended.go:ToCSS
    pub fn to_css(
        &self,
        ctx: &nh_tpl::template::TplContext,
        r: Arc<dyn Resource>,
        opts: Options,
    ) -> Result<Arc<dyn Resource>> {
        todo!()
    }
}

/// Go: `scss.DecodeOptions(m)`.
// Go: resources/resource_transformers/tocss/scss/client.go:DecodeOptions
pub fn decode_options(m: Option<&Map>) -> Result<Options> {
    todo!()
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: resources/resource_transformers/tocss/scss/tocss.go (214 lines; 2/3 funcs executed)
//    L35-37: Supports() bool
// EX L39-181: (t *toCSSTransformation) Transform(ctx *resources.ResourceTransformationCtx) error
// EX L183-214: (c *Client) toCSS(options libsass.Options, dst io.Writer, src io.Reader) (libsass.Result, error)
// ---------------------------------------------------------------------------
