//! Port of `resources/images/filters.go`.
//!
//! Owner: Wave B task T10 (images).

//! Go `resources/images/filters.go`: the `images.*` filter constructors (template namespace
//! `images` embeds `*images.Filters`). Each returns an `images.filter{Options, gift.Filter}`
//! whose hashstructure hash is the Filter key (`HashString(gfilters)`): struct `filter` with fields
//! `Options` (struct `filterOpts{Version int, Vals any}`) and `Filter` (the embedded interface,
//! e.g. `overlayFilter` with only unexported fields -> hash of its type name). Template int
//! literals arrive as Go `int` (8-byte hash); a float literal would change the key.
//!
//! Go's constructors panic on invalid arguments (`images.Padding`, ...); the port returns the
//! panic message as an error.

use std::any::Any;
use std::borrow::Cow;
use std::sync::{Arc, Once};

use go_hashstructure::{GoStruct, HashValue, Receiver};
use go_image::color::Color as GoColor;
use go_value::{HostCtx, IntKind, Object, SliceType, Value};
use nh_common::cast::caste;
use nh_common::{Error, Result};

use crate::auto_orient::AutoOrientFilter;
use crate::color::{Color, to_color_go};
use crate::image::{GiftFilter, ImageSource};
use crate::mask::MaskFilter;
use crate::opacity::OpacityFilter;
use crate::overlay::OverlayFilter;
use crate::padding::PaddingFilter;
use crate::process::ProcessFilter;

/// Go: `filterAPIVersion`. Increment for re-generation of images using these filters.
pub const FILTER_API_VERSION: i64 = 0;

/// Go: `images.filter` as a template value.
#[derive(Clone)]
pub struct FilterObject {
    /// Go `filterOpts.Version`.
    pub version: i64,
    /// Go `filterOpts.Vals` (e.g. `[]any{wm.Key(), 0, 0}` for Overlay); `Value::Invalid` is a
    /// nil `Vals` (filters built without options: Grayscale, Invert, AutoOrient).
    pub vals: Value,
    /// Go type name of the embedded gift filter for hashing (e.g. "overlayFilter").
    pub filter_type_name: &'static str,
    pub filter: GiftFilter,
}

impl FilterObject {
    /// Go: `filter{Options: opts, Filter: f}`.
    fn new(opts: FilterOpts, filter_type_name: &'static str, filter: GiftFilter) -> FilterObject {
        register_hash_types();
        FilterObject {
            version: opts.version,
            vals: opts.vals,
            filter_type_name,
            filter,
        }
    }

    /// How hashstructure sees the value: `filter{Options filterOpts; gift.Filter}`.
    pub fn hash_value(&self) -> HashValue {
        let vals = match &self.vals {
            Value::Invalid => HashValue::Nil,
            v => HashValue::Interface(Box::new(HashValue::Value(v.clone()))),
        };
        let opts = GoStruct::new("filterOpts")
            .field("Version", HashValue::Int(self.version, IntKind::Int))
            .field("Vals", vals);
        // The embedded gift.Filter: a (pointer to a) struct whose fields are all unexported.
        let f = HashValue::Interface(Box::new(HashValue::Struct(GoStruct::new(
            self.filter_type_name,
        ))));
        HashValue::Struct(
            GoStruct::new("filter")
                .field("Options", HashValue::Struct(opts))
                .field("Filter", f),
        )
    }
}

impl Object for FilterObject {
    fn type_name(&self) -> Cow<'_, str> {
        Cow::Borrowed("images.filter")
    }
    fn kind(&self) -> go_value::Kind {
        go_value::Kind::Struct
    }
    fn has_method(&self, name: &str) -> bool {
        // The embedded gift.Filter's methods.
        matches!(name, "Draw" | "Bounds")
    }
    fn call_method(
        &self,
        _ctx: HostCtx<'_>,
        name: &str,
        _args: &[Value],
    ) -> Option<go_value::Result<Value>> {
        if !self.has_method(name) {
            return None;
        }
        Some(Err(go_value::Error::new(format!(
            "neohugo-rs: calling images.filter.{name} from a template is not supported"
        ))))
    }
    /// For hashstructure: `Options` (filterOpts{Version, Vals}), `Filter` (the gift filter struct).
    /// (Hashing uses the registered [`FilterObject::hash_value`]; these are the fields as a
    /// template sees them.)
    fn struct_fields(&self) -> Option<Vec<(Cow<'_, str>, Value)>> {
        Some(vec![
            (
                Cow::Borrowed("Options"),
                Value::object(FilterOptsObject {
                    version: self.version,
                    vals: self.vals.clone(),
                }),
            ),
            (
                Cow::Borrowed("Filter"),
                Value::string(self.filter_type_name),
            ),
        ])
    }
    fn field(&self, name: &str) -> Option<Value> {
        match name {
            "Options" => Some(Value::object(FilterOptsObject {
                version: self.version,
                vals: self.vals.clone(),
            })),
            _ => None,
        }
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// `images.filterOpts` as a template value (only reachable through `.Options`).
struct FilterOptsObject {
    version: i64,
    vals: Value,
}

impl Object for FilterOptsObject {
    fn type_name(&self) -> Cow<'_, str> {
        Cow::Borrowed("images.filterOpts")
    }
    fn kind(&self) -> go_value::Kind {
        go_value::Kind::Struct
    }
    fn has_method(&self, _name: &str) -> bool {
        false
    }
    fn call_method(
        &self,
        _ctx: HostCtx<'_>,
        _name: &str,
        _args: &[Value],
    ) -> Option<go_value::Result<Value>> {
        None
    }
    fn field(&self, name: &str) -> Option<Value> {
        match name {
            "Version" => Some(Value::int(self.version)),
            "Vals" => Some(self.vals.clone()),
            _ => None,
        }
    }
    fn struct_fields(&self) -> Option<Vec<(Cow<'_, str>, Value)>> {
        Some(vec![
            (Cow::Borrowed("Version"), Value::int(self.version)),
            (Cow::Borrowed("Vals"), self.vals.clone()),
        ])
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// Registers how the filter objects and `images.Color` look to hashstructure. Idempotent.
pub fn register_hash_types() {
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        go_hashstructure::register_object::<FilterObject>(|f| f.hash_value());
        go_hashstructure::register_object::<Color>(|c| {
            struct H(u64);
            impl go_hashstructure::Hashable for H {
                fn hash(&self) -> std::result::Result<u64, go_hashstructure::Error> {
                    Ok(self.0)
                }
            }
            HashValue::Struct(
                GoStruct::new("Color").with_hashable(Receiver::Value, Arc::new(H(c.hash()))),
            )
        });
    });
}

/// `hashing.HashString(gfilters)` of a `[]gift.Filter` (the Filter key of
/// `resources/image.go:Filter`).
pub fn filters_key(filters: &[FilterObject]) -> String {
    register_hash_types();
    let items: Vec<Value> = filters.iter().map(|f| Value::object(f.clone())).collect();
    nh_common::hashing::hash_string(&[Value::list(SliceType::Named("[]gift.Filter".into()), items)])
}

/// Go: `filterOpts{Version, Vals}`.
struct FilterOpts {
    version: i64,
    vals: Value,
}

/// For cache-busting.
// Go: resources/images/filters.go:newFilterOpts
fn new_filter_opts(vals: Vec<Value>) -> FilterOpts {
    FilterOpts {
        version: FILTER_API_VERSION,
        vals: Value::list(SliceType::Any, vals),
    }
}

/// The zero `filterOpts` (`filter{Filter: ...}` without options).
fn no_filter_opts() -> FilterOpts {
    FilterOpts {
        version: 0,
        vals: Value::Invalid,
    }
}

/// Go's type name of the concrete filter a gift constructor returned.
fn gift_type_name(f: &Arc<dyn gift::Filter>) -> &'static str {
    let a: &dyn Any = &**f;
    if a.is::<gift::ColorchanFilter>() {
        "colorchanFilter"
    } else if a.is::<gift::ColorFilter>() {
        "colorFilter"
    } else if a.is::<gift::CopyimageFilter>() {
        "copyimageFilter"
    } else if a.is::<gift::GaussianBlurFilter>() {
        // Sic (gift v1.2.1).
        "gausssianBlurFilter"
    } else if a.is::<gift::UnsharpMaskFilter>() {
        "unsharpMaskFilter"
    } else if a.is::<gift::PixelateFilter>() {
        "pixelateFilter"
    } else if a.is::<gift::TransformFilter>() {
        "transformFilter"
    } else if a.is::<gift::ResizeFilter>() {
        "resizeFilter"
    } else if a.is::<gift::ResizeToFitFilter>() {
        "resizeToFitFilter"
    } else if a.is::<gift::ResizeToFillFilter>() {
        "resizeToFillFilter"
    } else if a.is::<gift::RotateFilter>() {
        "rotateFilter"
    } else if a.is::<gift::CropFilter>() {
        "cropFilter"
    } else if a.is::<gift::CropToSizeFilter>() {
        "cropToSizeFilter"
    } else if a.is::<gift::ConvolutionFilter>() {
        "convolutionFilter"
    } else if a.is::<gift::MeanFilter>() {
        "meanFilter"
    } else if a.is::<gift::HvConvolutionFilter>() {
        "hvConvolutionFilter"
    } else if a.is::<gift::RankFilter>() {
        "rankFilter"
    } else {
        "Filter"
    }
}

fn gift_filter(opts: FilterOpts, f: Arc<dyn gift::Filter>) -> FilterObject {
    let name = gift_type_name(&f);
    FilterObject::new(opts, name, GiftFilter::Gift(f))
}

fn f32v(v: &Value) -> f32 {
    caste::to_float32(v)
}

/// Go: `images.Filters` (namespace methods).
pub struct Filters;

impl Filters {
    /// Process creates a filter that processes an image using the given specification.
    // Go: resources/images/filters.go:Process
    pub fn process(&self, spec: &Value) -> Result<FilterObject> {
        let specs = go_unicode::strings::to_lower(&caste::to_string(spec)).into_owned();
        let specs_s = String::from_utf8_lossy(&specs).into_owned();
        Ok(FilterObject::new(
            new_filter_opts(vec![Value::string(specs)]),
            "processFilter",
            GiftFilter::Process(Arc::new(ProcessFilter { spec: specs_s })),
        ))
    }

    /// Overlay creates a filter that overlays src at position x y.
    // Go: resources/images/filters.go:Overlay
    pub fn overlay(&self, src: Arc<dyn ImageSource>, x: &Value, y: &Value) -> Result<FilterObject> {
        let key = src.key();
        Ok(FilterObject::new(
            new_filter_opts(vec![Value::string(key), x.clone(), y.clone()]),
            "overlayFilter",
            GiftFilter::Overlay(Arc::new(OverlayFilter {
                src,
                x: caste::to_int(x),
                y: caste::to_int(y),
            })),
        ))
    }

    /// Mask creates a filter that applies a mask image to the source image.
    // Go: resources/images/filters.go:Mask
    pub fn mask(&self, mask: Arc<dyn ImageSource>) -> Result<FilterObject> {
        let key = mask.key();
        Ok(FilterObject::new(
            new_filter_opts(vec![Value::string(key)]),
            "maskFilter",
            GiftFilter::Mask(Arc::new(MaskFilter { mask })),
        ))
    }

    /// Opacity creates a filter that changes the opacity of an image. The opacity parameter
    /// must be in range (0, 1).
    // Go: resources/images/filters.go:Opacity
    pub fn opacity(&self, opacity: &Value) -> Result<FilterObject> {
        Ok(FilterObject::new(
            new_filter_opts(vec![opacity.clone()]),
            "opacityFilter",
            GiftFilter::Gift(Arc::new(OpacityFilter {
                opacity: f32v(opacity),
            })),
        ))
    }

    /// Text creates a filter that draws text with the given options. Not supported: drawing
    /// needs `golang.org/x/image/font/opentype` and the Go fonts (see PORTING.md).
    // Go: resources/images/filters.go:Text
    pub fn text(&self, _text: &Value, _options: &[Value]) -> Result<FilterObject> {
        Err(crate::text::unsupported())
    }

    /// Padding creates a filter that resizes the image canvas without resizing the image. The
    /// last argument is the canvas color, expressed as an RGB or RGBA hexadecimal color. The
    /// default value is `ffffffff` (opaque white). The preceding arguments are the padding
    /// values, in pixels, using the CSS shorthand property syntax. Negative padding values will
    /// crop the image. The signature is images.Padding V1 [V2] [V3] [V4] [COLOR].
    // Go: resources/images/filters.go:Padding
    pub fn padding(&self, args: &[Value]) -> Result<FilterObject> {
        if args.is_empty() || args.len() > 5 {
            return Err(Error::new(
                "the padding filter requires between 1 and 5 arguments",
            ));
        }

        let (top, right, bottom, left);
        let mut ccolor: GoColor = GoColor::Gray16(go_image::color::WHITE); // canvas color

        let orig_args = args; // preserve original args for most stable hash
        let mut args = args;

        match to_color_go(&args[args.len() - 1]) {
            Err(_) => {
                return Err(Error::new(
                    "invalid canvas color: specify RGB or RGBA using hex notation",
                ));
            }
            Ok(Some(vcs)) => {
                ccolor = vcs;
                args = &args[..args.len() - 1];
                if args.is_empty() {
                    return Err(Error::new(
                        "not enough arguments: provide one or more padding values using the CSS shorthand property syntax",
                    ));
                }
            }
            Ok(None) => {}
        }

        let mut vals: Vec<i64> = Vec::new();
        for v in args {
            let vi = caste::to_int(v);
            if vi > 5000 {
                return Err(Error::new("padding values must not exceed 5000 pixels"));
            }
            vals.push(vi);
        }

        match args.len() {
            1 => {
                (top, right, bottom, left) = (vals[0], vals[0], vals[0], vals[0]);
            }
            2 => {
                (top, right, bottom, left) = (vals[0], vals[1], vals[0], vals[1]);
            }
            3 => {
                (top, right, bottom, left) = (vals[0], vals[1], vals[2], vals[1]);
            }
            4 => {
                (top, right, bottom, left) = (vals[0], vals[1], vals[2], vals[3]);
            }
            n => {
                return Err(Error::new(format!(
                    "too many padding values: received {n}, expected maximum of 4"
                )));
            }
        }

        Ok(FilterObject::new(
            new_filter_opts(orig_args.to_vec()),
            "paddingFilter",
            GiftFilter::Padding(Arc::new(PaddingFilter {
                top,
                right,
                bottom,
                left,
                ccolor,
            })),
        ))
    }

    /// Dither creates a filter that dithers an image. Not supported (see PORTING.md).
    // Go: resources/images/filters.go:Dither
    pub fn dither(&self, _options: &[Value]) -> Result<FilterObject> {
        Err(crate::dither::unsupported())
    }

    /// AutoOrient creates a filter that rotates and flips an image as needed per its EXIF
    /// orientation tag.
    // Go: resources/images/filters.go:AutoOrient
    pub fn auto_orient(&self) -> Result<FilterObject> {
        Ok(FilterObject::new(
            no_filter_opts(),
            "autoOrientFilter",
            GiftFilter::AutoOrient(Arc::new(AutoOrientFilter)),
        ))
    }

    /// Brightness creates a filter that changes the brightness of an image. The percentage
    /// parameter must be in range (-100, 100).
    // Go: resources/images/filters.go:Brightness
    pub fn brightness(&self, percentage: &Value) -> Result<FilterObject> {
        Ok(gift_filter(
            new_filter_opts(vec![percentage.clone()]),
            gift::brightness(f32v(percentage)),
        ))
    }

    /// ColorBalance creates a filter that changes the color balance of an image. The
    /// percentage parameters for each color channel (red, green, blue) must be in range
    /// (-100, 500).
    // Go: resources/images/filters.go:ColorBalance
    pub fn color_balance(
        &self,
        percentage_red: &Value,
        percentage_green: &Value,
        percentage_blue: &Value,
    ) -> Result<FilterObject> {
        Ok(gift_filter(
            new_filter_opts(vec![
                percentage_red.clone(),
                percentage_green.clone(),
                percentage_blue.clone(),
            ]),
            gift::color_balance(
                f32v(percentage_red),
                f32v(percentage_green),
                f32v(percentage_blue),
            ),
        ))
    }

    /// Colorize creates a filter that produces a colorized version of an image.
    // Go: resources/images/filters.go:Colorize
    pub fn colorize(
        &self,
        hue: &Value,
        saturation: &Value,
        percentage: &Value,
    ) -> Result<FilterObject> {
        Ok(gift_filter(
            new_filter_opts(vec![hue.clone(), saturation.clone(), percentage.clone()]),
            gift::colorize(f32v(hue), f32v(saturation), f32v(percentage)),
        ))
    }

    /// Contrast creates a filter that changes the contrast of an image.
    // Go: resources/images/filters.go:Contrast
    pub fn contrast(&self, percentage: &Value) -> Result<FilterObject> {
        Ok(gift_filter(
            new_filter_opts(vec![percentage.clone()]),
            gift::contrast(f32v(percentage)),
        ))
    }

    /// Gamma creates a filter that performs a gamma correction on an image.
    // Go: resources/images/filters.go:Gamma
    pub fn gamma(&self, gamma: &Value) -> Result<FilterObject> {
        Ok(gift_filter(
            new_filter_opts(vec![gamma.clone()]),
            gift::gamma(f32v(gamma)),
        ))
    }

    /// GaussianBlur creates a filter that applies a gaussian blur to an image.
    // Go: resources/images/filters.go:GaussianBlur
    pub fn gaussian_blur(&self, sigma: &Value) -> Result<FilterObject> {
        Ok(gift_filter(
            new_filter_opts(vec![sigma.clone()]),
            gift::gaussian_blur(f32v(sigma)),
        ))
    }

    /// Grayscale creates a filter that produces a grayscale version of an image.
    // Go: resources/images/filters.go:Grayscale
    pub fn grayscale(&self) -> Result<FilterObject> {
        Ok(gift_filter(no_filter_opts(), gift::grayscale()))
    }

    /// Hue creates a filter that rotates the hue of an image.
    // Go: resources/images/filters.go:Hue
    pub fn hue(&self, shift: &Value) -> Result<FilterObject> {
        Ok(gift_filter(
            new_filter_opts(vec![shift.clone()]),
            gift::hue(f32v(shift)),
        ))
    }

    /// Invert creates a filter that negates the colors of an image.
    // Go: resources/images/filters.go:Invert
    pub fn invert(&self) -> Result<FilterObject> {
        Ok(gift_filter(no_filter_opts(), gift::invert()))
    }

    /// Pixelate creates a filter that applies a pixelation effect to an image.
    // Go: resources/images/filters.go:Pixelate
    pub fn pixelate(&self, size: &Value) -> Result<FilterObject> {
        Ok(gift_filter(
            new_filter_opts(vec![size.clone()]),
            gift::pixelate(caste::to_int(size)),
        ))
    }

    /// Saturation creates a filter that changes the saturation of an image.
    // Go: resources/images/filters.go:Saturation
    pub fn saturation(&self, percentage: &Value) -> Result<FilterObject> {
        Ok(gift_filter(
            new_filter_opts(vec![percentage.clone()]),
            gift::saturation(f32v(percentage)),
        ))
    }

    /// Sepia creates a filter that produces a sepia-toned version of an image.
    // Go: resources/images/filters.go:Sepia
    pub fn sepia(&self, percentage: &Value) -> Result<FilterObject> {
        Ok(gift_filter(
            new_filter_opts(vec![percentage.clone()]),
            gift::sepia(f32v(percentage)),
        ))
    }

    /// Sigmoid creates a filter that changes the contrast of an image using a sigmoidal
    /// function and returns the adjusted image.
    // Go: resources/images/filters.go:Sigmoid
    pub fn sigmoid(&self, midpoint: &Value, factor: &Value) -> Result<FilterObject> {
        Ok(gift_filter(
            new_filter_opts(vec![midpoint.clone(), factor.clone()]),
            gift::sigmoid_filter(f32v(midpoint), f32v(factor)),
        ))
    }

    /// UnsharpMask creates a filter that sharpens an image.
    // Go: resources/images/filters.go:UnsharpMask
    pub fn unsharp_mask(
        &self,
        sigma: &Value,
        amount: &Value,
        threshold: &Value,
    ) -> Result<FilterObject> {
        Ok(gift_filter(
            new_filter_opts(vec![sigma.clone(), amount.clone(), threshold.clone()]),
            gift::unsharp_mask(f32v(sigma), f32v(amount), f32v(threshold)),
        ))
    }
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: resources/images/filters.go (404 lines; 2/23 funcs executed)
//   types: Filters, filter, filterOpts
// OK L38-46: (*Filters) Process(spec any) gift.Filter
// OK L49-54: (*Filters) Overlay(src ImageSource, x, y any) gift.Filter
// OK L57-62: (*Filters) Mask(mask ImageSource) gift.Filter
// OK L66-71: (*Filters) Opacity(opacity any) gift.Filter
// STUB L74-140: (*Filters) Text(text string, options ...any) gift.Filter
// OK L148-201: (*Filters) Padding(args ...any) gift.Filter
// STUB L204-254: (*Filters) Dither(options ...any) gift.Filter
// OK L258-262: (*Filters) AutoOrient() gift.Filter
// OK L266-271: (*Filters) Brightness(percentage any) gift.Filter
// OK L275-280: (*Filters) ColorBalance(percentageRed, percentageGreen, percentageBlue any) gift.Filter
// OK L286-291: (*Filters) Colorize(hue, saturation, percentage any) gift.Filter
// OK L295-300: (*Filters) Contrast(percentage any) gift.Filter
// OK L305-310: (*Filters) Gamma(gamma any) gift.Filter
// OK L313-318: (*Filters) GaussianBlur(sigma any) gift.Filter
// OK L321-325: (*Filters) Grayscale() gift.Filter
// OK L329-334: (*Filters) Hue(shift any) gift.Filter
// OK L337-341: (*Filters) Invert() gift.Filter
// OK L344-349: (*Filters) Pixelate(size any) gift.Filter
// OK L352-357: (*Filters) Saturation(percentage any) gift.Filter
// OK L360-365: (*Filters) Sepia(percentage any) gift.Filter
// OK L369-374: (*Filters) Sigmoid(midpoint, factor any) gift.Filter
// OK L381-386: (*Filters) UnsharpMask(sigma, amount, threshold any) gift.Filter
// OK L399-404: newFilterOpts(vals ...any) filterOpts
// ---------------------------------------------------------------------------
