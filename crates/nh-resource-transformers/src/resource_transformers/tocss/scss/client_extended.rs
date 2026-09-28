//! Port of `resources/resource_transformers/tocss/scss/client_extended.go`.
//!
//! Owner: Wave B task T16 (js-css-pipeline).

use std::borrow::Cow;
use std::sync::{Arc, Once};

use go_hashstructure::{GoMap, GoStruct, HashValue};
use go_value::{HostCtx, Object, Value};
use nh_common::Result;
use nh_common::herrors::Error;
use nh_resource::internal::key::ResourceTransformationKey;
use nh_resource::resourcetypes::Resource;

use super::client::TRANSFORMATION_NAME;
use super::tocss::{Client, Options};

/// Go: `scss.options`.
#[derive(Clone)]
pub(crate) struct InternalOptions {
    /// The options we receive from the end user.
    pub(crate) from: Options,
    /// The options we send to the SCSS library.
    pub(crate) to: libsass_sys::transpiler::Options,
}

/// Go: `toCSSTransformation`.
pub(crate) struct ToCssTransformation {
    pub(crate) c: Arc<Client>,
    pub(crate) options: InternalOptions,
}

impl Client {
    /// Go: `Client.ToCSS(res, opts)`. `r` must be a resource adapter (Go's
    /// `resources.ResourceTransformer`); Go calls `res.Transform` (no context).
    // Go: resources/resource_transformers/tocss/scss/client_extended.go:ToCSS
    pub fn to_css(
        &self,
        _ctx: &nh_tpl::template::TplContext,
        r: Arc<dyn Resource>,
        opts: Options,
    ) -> Result<Arc<dyn Resource>> {
        let res = nh_resources::transform::resource_adapter(&r)
            .ok_or_else(|| Error::new(format!("{} can not be transformed", r.tpl_type_name())))?;

        let mut internal_options = InternalOptions {
            from: opts.clone(),
            to: libsass_sys::transpiler::Options::default(),
        };

        // Transfer values from client.
        internal_options.to.precision = opts.precision;
        internal_options.to.output_style =
            libsass_sys::transpiler::parse_output_style(opts.output_style.as_bytes());

        if internal_options.to.precision == 0 {
            // bootstrap-sass requires 8 digits precision. The libsass default is 5.
            // https://github.com/twbs/bootstrap-sass/blob/master/README.md#sass-number-precision
            internal_options.to.precision = 8;
        }

        Ok(res.transform(vec![Arc::new(ToCssTransformation {
            c: Arc::new(self.clone()),
            options: internal_options,
        })])?)
    }
}

impl ToCssTransformation {
    // Go: resources/resource_transformers/tocss/scss/client_extended.go:(*toCSSTransformation).Key
    pub(crate) fn transformation_key(&self) -> ResourceTransformationKey {
        ResourceTransformationKey::new(
            TRANSFORMATION_NAME,
            vec![Value::object(OptionsKey(self.options.from.clone()))],
        )
    }
}

/// The key value (`tocss_<hash>`) of a toCSS transformation with these options (Go:
/// `NewResourceTransformationKey("tocss", opts).Value()`).
pub fn key_value(opts: &Options) -> String {
    register_options_key();
    ResourceTransformationKey::new(
        TRANSFORMATION_NAME,
        vec![Value::object(OptionsKey(opts.clone()))],
    )
    .value()
}

/// `scss.Options` as the key element: hashed by go-hashstructure like Go's struct (fields in
/// declaration order: TargetPath, IncludePaths, OutputStyle, Precision, EnableSourceMap, Vars).
struct OptionsKey(Options);

impl Object for OptionsKey {
    fn type_name(&self) -> Cow<'_, str> {
        Cow::Borrowed("scss.Options")
    }
    fn kind(&self) -> go_value::Kind {
        go_value::Kind::Struct
    }
    fn has_method(&self, _name: &str) -> bool {
        false
    }
    fn call_method(&self, _: HostCtx<'_>, _: &str, _: &[Value]) -> Option<go_value::Result<Value>> {
        None
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

/// Registers [`OptionsKey`] with go-hashstructure (once).
pub(crate) fn register_options_key() {
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        go_hashstructure::register_object::<OptionsKey>(|o| {
            let o = &o.0;
            let include = if o.include_paths.is_empty() {
                HashValue::Slice(None)
            } else {
                HashValue::string_slice(o.include_paths.iter().map(String::as_str))
            };
            let vars = match &o.vars {
                None => HashValue::Map(GoMap::nil()),
                Some(v) => HashValue::string_any_map(v.entries.iter().map(|(k, x)| {
                    (
                        std::str::from_utf8(k.as_bytes()).unwrap_or_default(),
                        HashValue::Value(x.clone()),
                    )
                })),
            };
            HashValue::Struct(
                GoStruct::new("Options")
                    .field("TargetPath", o.target_path.as_str())
                    .field("IncludePaths", include)
                    .field("OutputStyle", o.output_style.as_str())
                    .field("Precision", HashValue::int(o.precision))
                    .field("EnableSourceMap", o.enable_source_map)
                    .field("Vars", vars),
            )
        });
    });
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: resources/resource_transformers/tocss/scss/client_extended.go (56 lines; 2/2 funcs executed)
//   types: options, toCSSTransformation
// OK L31-47: (c *Client) ToCSS(res resources.ResourceTransformer, opts Options) (resource.Resource, error)
// OK L54-56: (t *toCSSTransformation) Key() internal.ResourceTransformationKey
// ---------------------------------------------------------------------------
