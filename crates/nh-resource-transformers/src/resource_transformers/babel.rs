//! Port of `resources/resource_transformers/babel/babel.go`.
//!
//! STUB
//!
//! Owner: Wave B task T16 (js-css-pipeline).

//! Go `babel`: `js.Babel`. Seeksnack never calls it (only the client is created), so the
//! transformation is an explicit unsupported error (HUGO_LAYER.md §1 rule 5); the options and
//! their command-line arguments are ported.

use std::sync::Arc;

use go_value::{Map, Value};
use nh_common::Result;
use nh_common::herrors::Error;
use nh_config::decode::FieldRef;
use nh_resource::resourcetypes::Resource;
use nh_resources::resource_spec::Spec;

/// Go: `babel.Options` — options from https://babeljs.io/docs/en/options.
#[derive(Clone, Debug, Default)]
pub struct Options {
    /// Custom path to config file
    pub config: String,
    pub minified: bool,
    pub no_comments: bool,
    /// Go `*bool` (`None` = nil).
    pub compact: Option<Box<bool>>,
    pub verbose: bool,
    pub no_babelrc: bool,
    pub source_map: String,
}

nh_config::decode_struct!(Options, "babel.Options", |s| vec![
    FieldRef::new("Config", &mut s.config),
    FieldRef::new("Minified", &mut s.minified),
    FieldRef::new("NoComments", &mut s.no_comments),
    FieldRef::new("Compact", &mut s.compact),
    FieldRef::new("Verbose", &mut s.verbose),
    FieldRef::new("NoBabelrc", &mut s.no_babelrc),
    FieldRef::new("SourceMap", &mut s.source_map),
]);

/// DecodeOptions decodes options to and generates command flags
// Go: resources/resource_transformers/babel/babel.go:DecodeOptions
pub fn decode_options(m: Option<&Map>) -> Result<Options> {
    let mut opts = Options::default();
    let Some(m) = m else {
        return Ok(opts);
    };
    nh_config::decode::weak_decode_into(&Value::map(m.clone()), &mut opts)?;
    Ok(opts)
}

impl Options {
    // Go: resources/resource_transformers/babel/babel.go:(Options).toArgs
    pub fn to_args(&self) -> Vec<String> {
        let mut args = Vec::new();

        // external is not a known constant on the babel command line
        // .sourceMaps must be a boolean, "inline", "both", or undefined
        match self.source_map.as_str() {
            "external" => args.push("--source-maps".to_string()),
            "inline" => args.push("--source-maps=inline".to_string()),
            _ => {}
        }
        if self.minified {
            args.push("--minified".to_string());
        }
        if self.no_comments {
            args.push("--no-comments".to_string());
        }
        if let Some(c) = &self.compact {
            args.push(format!("--compact={}", **c));
        }
        if self.verbose {
            args.push("--verbose".to_string());
        }
        if self.no_babelrc {
            args.push("--no-babelrc".to_string());
        }
        args
    }
}

/// Go: `babel.Client`.
pub struct Client {
    pub rs: Arc<Spec>,
}

impl Client {
    /// New creates a new Client with the given specification.
    // Go: resources/resource_transformers/babel/babel.go:New
    pub fn new(rs: Arc<Spec>) -> Client {
        Client { rs }
    }

    /// Go: `Process(res, options)` — STUB: explicit unsupported error.
    // Go: resources/resource_transformers/babel/babel.go:(*Client).Process
    pub fn process(
        &self,
        _ctx: &nh_tpl::template::TplContext,
        _r: Arc<dyn Resource>,
        _options: Options,
    ) -> Result<Arc<dyn Resource>> {
        Err(Error::new("neohugo-rs: js.Babel is not supported"))
    }
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: resources/resource_transformers/babel/babel.go (242 lines; 1/6 funcs executed)
//   types: Options, Client, babelTransformation
// OK L52-58: DecodeOptions(m map[string]any) (opts Options, err error)
// OK L60-87: (opts Options) toArgs() []any
// OK L95-97: New(rs *resources.Spec) *Client
//    L104-106: (t *babelTransformation) Key() internal.ResourceTransformationKey (STUB)
//    L115-235: (t *babelTransformation) Transform(ctx *resources.ResourceTransformationCtx) error (STUB)
//    L238-242: (c *Client) Process(res resources.ResourceTransformer, options Options) (resource.Resource, error) (STUB: unsupported error)
// ---------------------------------------------------------------------------
