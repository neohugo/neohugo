//! Port of `tpl/images/images.go`.
//!
//! Owner: Wave B task T19 (tplfuncs-host).
//!
//! The namespace embeds `*images.Filters` (nh-images `filters::Filters`): its methods are the
//! filter constructors (`images.Overlay src x y` returns an `images.filter`). Resource arguments
//! (`Overlay`'s source, `Mask`'s mask) become `Arc<dyn ImageSource>` through nh-resources'
//! `image_source_from_value`; a constructor's error is the call's error (Go's constructors do
//! not fail except by panicking, which text/template reports the same way).
//!
//! STUB: `QR` (`rsc.io/qr` is not ported). `Text` and `Dither` are nh-images stubs.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use go_value::{HostCtx, Kind, Object, Value};
use nh_common::cast::caste;
use nh_common::object::{GoResult, args};
use nh_deps::deps::Deps;
use nh_hugofs::afero::Fs;
use nh_hugofs::overlayfs::{Options, OverlayFs};
use nh_images::filters::{FilterObject, Filters};
use nh_images::image::ImageSource;

// Parity notes: Embeds `images.Filters` (nh-images): `images.Overlay src x y` returns an `images.filter` object; `images.Filter FILTERS... IMAGE` applies them via the resource's `.Filter`.

/// Go: `images.Namespace` (template value `*images.Namespace`).
pub struct Namespace {
    pub d: Arc<Deps>,
    filters: Filters,
    read_file_fs: Option<Arc<dyn Fs>>,
    cache: Mutex<HashMap<String, ImageConfig>>,
}

fn gerr(msg: impl Into<String>) -> go_value::Error {
    go_value::Error::new(msg)
}

fn nil_deref() -> go_value::Error {
    gerr("runtime error: invalid memory address or nil pointer dereference")
}

/// Go: `image.Config` as a template value (fields `ColorModel`, `Width`, `Height`).
#[derive(Clone, Copy, Debug, Default)]
pub struct ImageConfig {
    pub width: i64,
    pub height: i64,
}

nh_common::go_methods!(ImageConfig {});

impl Object for ImageConfig {
    nh_common::object_basics!("image.Config");
    fn kind(&self) -> Kind {
        Kind::Struct
    }
    fn field(&self, name: &str) -> Option<Value> {
        match name {
            "Width" => Some(Value::int(self.width)),
            "Height" => Some(Value::int(self.height)),
            // The decoder's `color.Model` (a `*color.modelFunc` or a palette): not modelled.
            "ColorModel" => Some(Value::TypedNil(std::sync::Arc::from("color.Model"))),
            _ => None,
        }
    }
    fn is_zero(&self) -> Option<bool> {
        Some(self.width == 0 && self.height == 0)
    }
}

/// text/template's check of an `images.ImageSource` parameter: a nil value is accepted (Go
/// then dereferences it in the constructor).
fn image_source_arg(a: &[Value], i: usize) -> GoResult<Arc<dyn ImageSource>> {
    match a.get(i) {
        Some(Value::Invalid) => Err(nil_deref()),
        Some(v) => nh_resources::transform::image_source_from_value(v)
            .ok_or_else(|| args::wrong_type("images.ImageSource", v)),
        None => Err(gerr(format!("missing argument {i}"))),
    }
}

fn filter_value(f: nh_common::Result<FilterObject>) -> GoResult<Value> {
    Ok(Value::object(f?))
}

impl Namespace {
    /// New returns a new instance of the images-namespaced template functions.
    // Go: tpl/images/images.go:New
    pub fn new(d: Arc<Deps>) -> Namespace {
        let mut read_file_fs: Option<Arc<dyn Fs>> = None;

        // The docshelper script does not have or need all the dependencies set up.
        if let Some(ps) = &d.path_spec {
            read_file_fs = Some(Arc::new(OverlayFs::new(Options {
                fss: vec![ps.base_fs.work.clone(), ps.base_fs.content.fs.clone()],
                ..Default::default()
            })));
        }

        Namespace {
            d,
            filters: Filters,
            read_file_fs,
            cache: Mutex::new(HashMap::new()),
        }
    }

    /// Config returns the image.Config for the specified path relative to the working
    /// directory.
    // Go: tpl/images/images.go:Config
    pub fn config(&self, _ctx: HostCtx<'_>, a: &[Value]) -> GoResult<Value> {
        args::exactly(a, 1, "Config")?;
        let filename = caste::to_string_e(&a[0])?;
        let filename = filename.to_str_lossy().into_owned();

        if filename.is_empty() {
            return Err(gerr("config needs a filename"));
        }

        // Check cache for image config.
        if let Some(c) = self
            .cache
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .get(&filename)
        {
            return Ok(Value::object(*c));
        }

        let fs = self.read_file_fs.as_ref().ok_or_else(nil_deref)?;
        let mut f = fs.open(&filename)?;

        let dims = nh_images::image::decode_config(&mut f);
        let _ = f.close();
        let dims = dims?;
        let config = ImageConfig {
            width: dims.width,
            height: dims.height,
        };

        self.cache
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .insert(filename, config);

        Ok(Value::object(config))
    }

    /// Filter applies the given filters to the image given as the last element in args.
    // Go: tpl/images/images.go:Filter
    pub fn filter(&self, _ctx: HostCtx<'_>, a: &[Value]) -> GoResult<Value> {
        if a.len() < 2 {
            return Err(gerr("must provide an image and one or more filters"));
        }

        let last = &a[a.len() - 1];
        let img = nh_resource::resourcetypes::resource_from_value_any(last)
            .and_then(|r| nh_resources::transform::resource_adapter(&r))
            .ok_or_else(|| image_resource_assertion_error(last))?;
        let filtersv = &a[..a.len() - 1];

        Ok(img.filter(filtersv)?.to_value())
    }

    /// QR encodes the given text into a QR code using the specified options, returning an
    /// image resource. STUB: `rsc.io/qr` is not ported.
    // Go: tpl/images/images.go:QR
    pub fn qr(&self, _ctx: HostCtx<'_>, a: &[Value]) -> GoResult<Value> {
        if a.is_empty() || a.len() > 2 {
            return Err(gerr("requires 1 or 2 arguments"));
        }

        let text = caste::to_string_e(&a[0])?;

        if text.is_empty() {
            return Err(gerr("cannot encode an empty string"));
        }

        Err(gerr("neohugo-rs: images.QR is not supported"))
    }

    // Go: resources/images/filters.go:Overlay
    pub fn overlay(&self, _ctx: HostCtx<'_>, a: &[Value]) -> GoResult<Value> {
        args::exactly(a, 3, "Overlay")?;
        let src = image_source_arg(a, 0)?;
        filter_value(self.filters.overlay(src, &a[1], &a[2]))
    }

    // Go: resources/images/filters.go:Process
    pub fn process(&self, _ctx: HostCtx<'_>, a: &[Value]) -> GoResult<Value> {
        args::exactly(a, 1, "Process")?;
        filter_value(self.filters.process(&a[0]))
    }

    // Go: resources/images/filters.go:Brightness
    pub fn brightness(&self, _ctx: HostCtx<'_>, a: &[Value]) -> GoResult<Value> {
        args::exactly(a, 1, "Brightness")?;
        filter_value(self.filters.brightness(&a[0]))
    }

    // Go: resources/images/filters.go:ColorBalance
    pub fn color_balance(&self, _ctx: HostCtx<'_>, a: &[Value]) -> GoResult<Value> {
        args::exactly(a, 3, "ColorBalance")?;
        filter_value(self.filters.color_balance(&a[0], &a[1], &a[2]))
    }

    // Go: resources/images/filters.go:Colorize
    pub fn colorize(&self, _ctx: HostCtx<'_>, a: &[Value]) -> GoResult<Value> {
        args::exactly(a, 3, "Colorize")?;
        filter_value(self.filters.colorize(&a[0], &a[1], &a[2]))
    }

    // Go: resources/images/filters.go:Contrast
    pub fn contrast(&self, _ctx: HostCtx<'_>, a: &[Value]) -> GoResult<Value> {
        args::exactly(a, 1, "Contrast")?;
        filter_value(self.filters.contrast(&a[0]))
    }

    // Go: resources/images/filters.go:Gamma
    pub fn gamma(&self, _ctx: HostCtx<'_>, a: &[Value]) -> GoResult<Value> {
        args::exactly(a, 1, "Gamma")?;
        filter_value(self.filters.gamma(&a[0]))
    }

    // Go: resources/images/filters.go:GaussianBlur
    pub fn gaussian_blur(&self, _ctx: HostCtx<'_>, a: &[Value]) -> GoResult<Value> {
        args::exactly(a, 1, "GaussianBlur")?;
        filter_value(self.filters.gaussian_blur(&a[0]))
    }

    // Go: resources/images/filters.go:Grayscale
    pub fn grayscale(&self, _ctx: HostCtx<'_>, a: &[Value]) -> GoResult<Value> {
        args::exactly(a, 0, "Grayscale")?;
        filter_value(self.filters.grayscale())
    }

    // Go: resources/images/filters.go:Hue
    pub fn hue(&self, _ctx: HostCtx<'_>, a: &[Value]) -> GoResult<Value> {
        args::exactly(a, 1, "Hue")?;
        filter_value(self.filters.hue(&a[0]))
    }

    // Go: resources/images/filters.go:Invert
    pub fn invert(&self, _ctx: HostCtx<'_>, a: &[Value]) -> GoResult<Value> {
        args::exactly(a, 0, "Invert")?;
        filter_value(self.filters.invert())
    }

    // Go: resources/images/filters.go:Pixelate
    pub fn pixelate(&self, _ctx: HostCtx<'_>, a: &[Value]) -> GoResult<Value> {
        args::exactly(a, 1, "Pixelate")?;
        filter_value(self.filters.pixelate(&a[0]))
    }

    // Go: resources/images/filters.go:Saturation
    pub fn saturation(&self, _ctx: HostCtx<'_>, a: &[Value]) -> GoResult<Value> {
        args::exactly(a, 1, "Saturation")?;
        filter_value(self.filters.saturation(&a[0]))
    }

    // Go: resources/images/filters.go:Sepia
    pub fn sepia(&self, _ctx: HostCtx<'_>, a: &[Value]) -> GoResult<Value> {
        args::exactly(a, 1, "Sepia")?;
        filter_value(self.filters.sepia(&a[0]))
    }

    // Go: resources/images/filters.go:Sigmoid
    pub fn sigmoid(&self, _ctx: HostCtx<'_>, a: &[Value]) -> GoResult<Value> {
        args::exactly(a, 2, "Sigmoid")?;
        filter_value(self.filters.sigmoid(&a[0], &a[1]))
    }

    // Go: resources/images/filters.go:UnsharpMask
    pub fn unsharp_mask(&self, _ctx: HostCtx<'_>, a: &[Value]) -> GoResult<Value> {
        args::exactly(a, 3, "UnsharpMask")?;
        filter_value(self.filters.unsharp_mask(&a[0], &a[1], &a[2]))
    }

    // Go: resources/images/filters.go:Text
    pub fn text(&self, _ctx: HostCtx<'_>, a: &[Value]) -> GoResult<Value> {
        args::at_least(a, 1, "Text")?;
        args::string(a, 0)?;
        filter_value(self.filters.text(&a[0], &a[1..]))
    }

    // Go: resources/images/filters.go:Padding
    pub fn padding(&self, _ctx: HostCtx<'_>, a: &[Value]) -> GoResult<Value> {
        filter_value(self.filters.padding(a))
    }

    // Go: resources/images/filters.go:Dither
    pub fn dither(&self, _ctx: HostCtx<'_>, a: &[Value]) -> GoResult<Value> {
        filter_value(self.filters.dither(a))
    }

    // Go: resources/images/filters.go:AutoOrient
    pub fn auto_orient(&self, _ctx: HostCtx<'_>, a: &[Value]) -> GoResult<Value> {
        args::exactly(a, 0, "AutoOrient")?;
        filter_value(self.filters.auto_orient())
    }

    // Go: resources/images/filters.go:Mask
    pub fn mask(&self, _ctx: HostCtx<'_>, a: &[Value]) -> GoResult<Value> {
        args::exactly(a, 1, "Mask")?;
        let mask = image_source_arg(a, 0)?;
        filter_value(self.filters.mask(mask))
    }

    // Go: resources/images/filters.go:Opacity
    pub fn opacity(&self, _ctx: HostCtx<'_>, a: &[Value]) -> GoResult<Value> {
        args::exactly(a, 1, "Opacity")?;
        filter_value(self.filters.opacity(&a[0]))
    }
}

/// The sorted method set of `images.ImageResource` (`resource.Resource` +
/// `images.ImageResourceOps`), for Go's type assertion error ("missing method X": the first
/// method, in sorted order, the dynamic type lacks).
const IMAGE_RESOURCE_METHODS: &[&str] = &[
    "Colors",
    "Crop",
    "Data",
    "Err",
    "Exif",
    "Fill",
    "Filter",
    "Fit",
    "Height",
    "MediaType",
    "Name",
    "Params",
    "Permalink",
    "Process",
    "RelPermalink",
    "Resize",
    "ResourceType",
    "Title",
    "Width",
];

/// Go's panic text for `args[len(args)-1].(images.ImageResource)`.
fn image_resource_assertion_error(v: &Value) -> go_value::Error {
    if v.is_invalid() {
        return gerr("interface conversion: interface is nil, not images.ImageResource");
    }
    let t = v.go_type_name();
    let missing = IMAGE_RESOURCE_METHODS
        .iter()
        .find(|m| match v {
            Value::Object(o) => !o.has_method(m),
            _ => true,
        })
        .copied()
        .unwrap_or("Colors");
    gerr(format!(
        "interface conversion: {t} is not images.ImageResource: missing method {missing}"
    ))
}

nh_common::go_methods!(Namespace {
    "Config" => |n, ctx, a| n.config(ctx, a),
    "Filter" => |n, ctx, a| n.filter(ctx, a),
    "QR" => |n, ctx, a| n.qr(ctx, a),
    "Overlay" => |n, ctx, a| n.overlay(ctx, a),
    "Process" => |n, ctx, a| n.process(ctx, a),
    "Brightness" => |n, ctx, a| n.brightness(ctx, a),
    "ColorBalance" => |n, ctx, a| n.color_balance(ctx, a),
    "Colorize" => |n, ctx, a| n.colorize(ctx, a),
    "Contrast" => |n, ctx, a| n.contrast(ctx, a),
    "Gamma" => |n, ctx, a| n.gamma(ctx, a),
    "GaussianBlur" => |n, ctx, a| n.gaussian_blur(ctx, a),
    "Grayscale" => |n, ctx, a| n.grayscale(ctx, a),
    "Hue" => |n, ctx, a| n.hue(ctx, a),
    "Invert" => |n, ctx, a| n.invert(ctx, a),
    "Pixelate" => |n, ctx, a| n.pixelate(ctx, a),
    "Saturation" => |n, ctx, a| n.saturation(ctx, a),
    "Sepia" => |n, ctx, a| n.sepia(ctx, a),
    "Sigmoid" => |n, ctx, a| n.sigmoid(ctx, a),
    "UnsharpMask" => |n, ctx, a| n.unsharp_mask(ctx, a),
    "Text" => |n, ctx, a| n.text(ctx, a),
    "Padding" => |n, ctx, a| n.padding(ctx, a),
    "Dither" => |n, ctx, a| n.dither(ctx, a),
    "AutoOrient" => |n, ctx, a| n.auto_orient(ctx, a),
    "Mask" => |n, ctx, a| n.mask(ctx, a),
    "Opacity" => |n, ctx, a| n.opacity(ctx, a),
});

impl Object for Namespace {
    nh_common::object_basics!("*images.Namespace");
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: tpl/images/images.go (211 lines; 2/4 funcs executed)
//   types: Namespace
// OK L46-66: New(d *deps.Deps) *Namespace
// OK L80-115: (ns *Namespace) Config(path any) (image.Config, error)
// OK L118-127: (ns *Namespace) Filter(args ...any) (images.ImageResource, error)
// STUB L138-211: (ns *Namespace) QR(args ...any) (images.ImageResource, error)
// ---------------------------------------------------------------------------
