//! Port of `resources/resource_transformers/cssjs/tailwindcss.go`.
//!
//! STUB
//!
//! Owner: Wave B task T16 (js-css-pipeline).

//! Go `cssjs/tailwindcss.go`: `css.TailwindCSS`. Seeksnack never calls it (only the client is
//! created), so the transformation is an explicit unsupported error (HUGO_LAYER.md §1 rule 5);
//! the options, their arguments and the import exclusion (shared with the `@import` inliner) are
//! ported.

use std::sync::{Arc, LazyLock};

use go_value::{Map, Value};
use nh_common::Result;
use nh_common::goregexp::Regexp;
use nh_common::herrors::Error;
use nh_config::decode::FieldRef;
use nh_resource::resourcetypes::Resource;
use nh_resources::resource_spec::Spec;

use super::inline_imports::InlineImports;

static TAILWINDCSS_IMPORT_RE: LazyLock<Regexp> =
    LazyLock::new(|| Regexp::must_compile(r"^tailwindcss/?"));

/// Go: `tailwindImportExclude`.
pub(crate) fn tailwind_import_exclude(s: &[u8]) -> bool {
    TAILWINDCSS_IMPORT_RE.is_match(s) && !s.contains(&b'.')
}

/// Go: `cssjs.TailwindCSSClient`.
pub struct TailwindCssClient {
    pub rs: Arc<Spec>,
}

/// NewTailwindCSSClient creates a new TailwindCSSClient with the given specification.
// Go: resources/resource_transformers/cssjs/tailwindcss.go:NewTailwindCSSClient
pub fn new_tailwind_css_client(rs: Arc<Spec>) -> TailwindCssClient {
    TailwindCssClient { rs }
}

impl TailwindCssClient {
    /// Go: `Process(res, options)` — STUB: explicit unsupported error.
    // Go: resources/resource_transformers/cssjs/tailwindcss.go:(*TailwindCSSClient).Process
    pub fn process(
        &self,
        _ctx: &nh_tpl::template::TplContext,
        _r: Arc<dyn Resource>,
        options: Option<&Map>,
    ) -> Result<Arc<dyn Resource>> {
        decode_tailwind_css_options(options)?;
        Err(Error::new("neohugo-rs: css.TailwindCSS is not supported"))
    }
}

/// Go: `cssjs.TailwindCSSOptions`.
#[derive(Clone, Debug, Default)]
pub struct TailwindCssOptions {
    /// Optimize and minify the output
    pub minify: bool,
    /// Optimize the output without minifying
    pub optimize: bool,
    pub inline_imports: InlineImports,
}

nh_config::decode_struct!(TailwindCssOptions, "cssjs.TailwindCSSOptions", |s| vec![
    FieldRef::new("Minify", &mut s.minify),
    FieldRef::new("Optimize", &mut s.optimize),
    FieldRef::squash("InlineImports", &mut s.inline_imports),
]);

impl TailwindCssOptions {
    // Go: resources/resource_transformers/cssjs/tailwindcss.go:(TailwindCSSOptions).toArgs
    pub fn to_args(&self) -> Vec<String> {
        let mut args = Vec::new();
        if self.minify {
            args.push("--minify".to_string());
        }
        if self.optimize {
            args.push("--optimize".to_string());
        }
        args
    }
}

// Go: resources/resource_transformers/cssjs/tailwindcss.go:decodeTailwindCSSOptions
pub fn decode_tailwind_css_options(m: Option<&Map>) -> Result<TailwindCssOptions> {
    let mut opts = TailwindCssOptions::default();
    let Some(m) = m else {
        return Ok(opts);
    };
    nh_config::decode::weak_decode_into(&Value::map(m.clone()), &mut opts)?;
    Ok(opts)
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: resources/resource_transformers/cssjs/tailwindcss.go (167 lines; 1/6 funcs executed)
//   types: TailwindCSSClient, tailwindcssTransformation, TailwindCSSOptions
// OK L40-42: NewTailwindCSSClient(rs *resources.Spec) *TailwindCSSClient
//    L50-52: (c *TailwindCSSClient) Process(res resources.ResourceTransformer, options map[string]any) (resource.Resource, error) (STUB: unsupported error)
//    L59-61: (t *tailwindcssTransformation) Key() internal.ResourceTransformationKey (STUB)
// OK L69-78: (opts TailwindCSSOptions) toArgs() []any
//    L80-159: (t *tailwindcssTransformation) Transform(ctx *resources.ResourceTransformationCtx) error (STUB)
// OK L161-167: decodeTailwindCSSOptions(m map[string]any) (opts TailwindCSSOptions, err error)
// ---------------------------------------------------------------------------
