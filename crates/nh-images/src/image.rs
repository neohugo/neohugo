//! Port of `resources/images/image.go`.
//!
//! Owner: Wave B task T10 (images).

//! Go `resources/images/image.go`: encoding (JPEG q75 via go-image, PNG DefaultCompression via
//! go-png + go-flate, WebP via libwebp-sys 1.3.2 preset photo + sharp YUV), the processor
//! (`doFilter` destination-type rules), image formats. See specs/images.md for exact numerics.

use std::io::{Read, Write};
use std::sync::{Arc, Once, OnceLock};

use go_image::{Gray, NRGBA, RGBA, Rectangle};
use nh_common::{Error, Result};
use nh_config::namespace::ConfigNamespace;
use nh_media::media::media_type::MediaType;

use crate::config::{ImageConfig, ImagingConfig, ImagingConfigInternal};
use crate::exif::{self, ExifInfo};
use crate::filters::FilterObject;

/// A decoded Go `image.Image`: one of go-image's concrete types (`*image.YCbCr`,
/// `*image.NRGBA`, `*image.RGBA`, `*image.Paletted`, `*image.Gray`, `*image.NRGBA64`, ...).
/// The concrete type decides the gift getters/setters, the `doFilter` destination type and the
/// encoders' paths, exactly as in Go.
#[derive(Clone)]
pub struct GoImage {
    pub(crate) inner: Arc<dyn go_image::Image>,
}

impl GoImage {
    /// Wraps a decoded image.
    pub fn new(img: Box<dyn go_image::Image>) -> GoImage {
        GoImage { inner: img.into() }
    }

    /// Wraps a shared image.
    pub fn from_arc(img: Arc<dyn go_image::Image>) -> GoImage {
        GoImage { inner: img }
    }

    /// The Go `image.Image`.
    pub fn image(&self) -> &dyn go_image::Image {
        &*self.inner
    }

    /// The shared image.
    pub fn arc(&self) -> &Arc<dyn go_image::Image> {
        &self.inner
    }

    /// Go `img.Bounds()`.
    pub fn bounds(&self) -> Rectangle {
        self.inner.bounds()
    }

    /// Go `img.(*T)`.
    pub fn downcast_ref<T: go_image::Image>(&self) -> Option<&T> {
        self.inner.as_any().downcast_ref::<T>()
    }

    /// Go's `%T` of the concrete image (`*image.NRGBA`, ...), for diagnostics and tests.
    pub fn go_type_name(&self) -> &'static str {
        go_image_type_name(&*self.inner)
    }
}

impl std::fmt::Debug for GoImage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}{}", self.go_type_name(), self.bounds())
    }
}

/// Go's `%T` of a go-image image.
pub fn go_image_type_name(img: &dyn go_image::Image) -> &'static str {
    let a = img.as_any();
    macro_rules! names {
        ($($t:ident => $n:expr),* $(,)?) => {
            $(if a.is::<go_image::$t>() { return $n; })*
        };
    }
    names!(
        RGBA => "*image.RGBA",
        RGBA64 => "*image.RGBA64",
        NRGBA => "*image.NRGBA",
        NRGBA64 => "*image.NRGBA64",
        Alpha => "*image.Alpha",
        Alpha16 => "*image.Alpha16",
        Gray => "*image.Gray",
        Gray16 => "*image.Gray16",
        CMYK => "*image.CMYK",
        Paletted => "*image.Paletted",
        YCbCr => "*image.YCbCr",
        NYCbCrA => "*image.NYCbCrA",
    );
    "image.Image"
}

/// Go: `image.Config` (width, height; color model omitted).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ImageDims {
    pub width: i64,
    pub height: i64,
}

/// Go: `images.Format` (`JPEG = 1` ... `WEBP = 6`; Go's zero value is `Option<Format>::None`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Format {
    Jpeg = 1,
    Png = 2,
    Gif = 3,
    Tiff = 4,
    Bmp = 5,
    Webp = 6,
}

impl Format {
    /// RequiresDefaultQuality returns if the default quality needs to be applied to images of
    /// this format.
    // Go: resources/images/image.go:RequiresDefaultQuality
    pub fn requires_default_quality(self) -> bool {
        matches!(self, Format::Jpeg | Format::Webp)
    }

    /// SupportsTransparency reports whether it supports transparency in any form.
    // Go: resources/images/image.go:SupportsTransparency
    pub fn supports_transparency(self) -> bool {
        !matches!(self, Format::Jpeg)
    }

    /// DefaultExtension returns the default file extension of this format, starting with a
    /// dot. For example: .jpg for JPEG
    // Go: resources/images/image.go:DefaultExtension
    pub fn default_extension(self) -> String {
        self.media_type().first_suffix.full_suffix
    }

    /// MediaType returns the media type of this image, e.g. image/jpeg for JPEG
    // Go: resources/images/image.go:MediaType
    pub fn media_type(self) -> MediaType {
        let b = nh_media::media::builtin::builtin();
        match self {
            Format::Jpeg => b.jpeg_type.clone(),
            Format::Png => b.png_type.clone(),
            Format::Gif => b.gif_type.clone(),
            Format::Tiff => b.tiff_type.clone(),
            Format::Bmp => b.bmp_type.clone(),
            Format::Webp => b.webp_type.clone(),
        }
    }

    /// Go: `(f Format) ToImageMetaImageFormatFormat()` (`-1` for GIF/BMP).
    // Go: resources/images/image.go:ToImageMetaImageFormatFormat
    pub fn to_image_meta_image_format(self) -> exif::ImageFormat {
        match self {
            Format::Jpeg => exif::ImageFormat::Jpeg,
            Format::Png => exif::ImageFormat::Png,
            Format::Tiff => exif::ImageFormat::Tiff,
            Format::Webp => exif::ImageFormat::WebP,
            _ => exif::ImageFormat::Unknown,
        }
    }
}

/// Go: `images.Spec` — something that can reopen the encoded source bytes.
pub trait Spec: Send + Sync {
    fn read_seek_closer(&self) -> Result<Box<dyn nh_common::hugio::ReadSeekCloser>>;
}

/// Go: `images.ImageSource` (`DecodeImage()` + `Key()`) — the overlay source (the watermark),
/// decoded from its ENCODED bytes (PNG round trip), never from in-memory pixels.
pub trait ImageSource: Send + Sync {
    fn decode_image(&self) -> Result<GoImage>;
    fn key(&self) -> String;
}

/// Go: `imageConfig` (`config`, `configInit sync.Once`, `configLoaded`), shared by copies of an
/// [`Image`] like Go's pointer.
#[derive(Default)]
pub(crate) struct ImageConfigState {
    /// The config once the `sync.Once` ran (Go's zero config after an error).
    once: OnceLock<ImageDims>,
    /// `configLoaded`: the config taken from a decoded image.
    loaded: Option<ImageDims>,
}

/// Go: `images.Image` (format + processor + lazily decoded config).
#[derive(Clone)]
pub struct Image {
    pub format: Format,
    pub proc: Arc<ImageProcessor>,
    /// `None` is Go's nil `Spec` (after `WithImage`).
    pub spec: Option<Arc<dyn Spec>>,
    pub(crate) config: Arc<ImageConfigState>,
}

impl Image {
    // Go: resources/images/image.go:NewImage
    pub fn new(
        f: Format,
        proc: Arc<ImageProcessor>,
        img: Option<GoImage>,
        s: Option<Arc<dyn Spec>>,
    ) -> Image {
        Image {
            format: f,
            proc,
            spec: s,
            config: Arc::new(ImageConfigState {
                once: OnceLock::new(),
                loaded: img.map(|img| image_config_from_image(&img)),
            }),
        }
    }

    /// Go: `Image.EncodeTo(conf, img, w)`.
    // Go: resources/images/image.go:EncodeTo
    pub fn encode_to(&self, conf: &ImageConfig, img: &GoImage, w: &mut Vec<u8>) -> Result<()> {
        encode_to(conf, img, w)
    }

    /// Height returns i's height (Go ignores the error of the lazy config load).
    // Go: resources/images/image.go:Height
    pub fn height(&self) -> i64 {
        let _ = self.init_config_lazy();
        self.config.once.get().map(|c| c.height).unwrap_or(0)
    }

    /// Width returns i's width.
    // Go: resources/images/image.go:Width
    pub fn width(&self) -> i64 {
        let _ = self.init_config_lazy();
        self.config.once.get().map(|c| c.width).unwrap_or(0)
    }

    // Go: resources/images/image.go:WithImage
    pub fn with_image(&self, img: &GoImage) -> Image {
        Image {
            format: self.format,
            proc: self.proc.clone(),
            spec: None,
            config: Arc::new(ImageConfigState {
                once: OnceLock::new(),
                loaded: Some(image_config_from_image(img)),
            }),
        }
    }

    // Go: resources/images/image.go:WithSpec
    pub fn with_spec(&self, s: Arc<dyn Spec>) -> Image {
        Image {
            format: self.format,
            proc: self.proc.clone(),
            spec: Some(s),
            config: Arc::new(ImageConfigState::default()),
        }
    }

    /// Go: `InitConfig(r)` reads the image config from the given reader (`image.DecodeConfig`),
    /// inside the same `sync.Once` as the lazy load: only the first call decodes, and only
    /// that call can return an error.
    // Go: resources/images/image.go:InitConfig
    pub fn init_config(&self, r: &mut dyn Read) -> Result<()> {
        let mut err = None;
        self.config.once.get_or_init(|| match decode_config(r) {
            Ok(c) => c,
            Err(e) => {
                err = Some(e);
                ImageDims::default()
            }
        });
        match err {
            Some(e) => Err(e),
            None => Ok(()),
        }
    }

    /// Go: `initConfig()`: loads the config from the spec once (`failed to load image
    /// config: %w`; only the call that ran the `sync.Once` sees the error).
    // Go: resources/images/image.go:initConfig
    pub fn init_config_lazy(&self) -> Result<()> {
        let mut err = None;
        self.config.once.get_or_init(|| {
            if let Some(c) = self.config.loaded {
                return c;
            }
            let Some(spec) = &self.spec else {
                // Go dereferences the nil Spec.
                err = Some(Error::new(
                    "runtime error: invalid memory address or nil pointer dereference",
                ));
                return ImageDims::default();
            };
            let mut f = match spec.read_seek_closer() {
                Ok(f) => f,
                Err(e) => {
                    err = Some(e);
                    return ImageDims::default();
                }
            };
            match decode_config(&mut f) {
                Ok(c) => c,
                Err(e) => {
                    err = Some(e);
                    ImageDims::default()
                }
            }
        });
        match err {
            Some(e) => Err(e.wrap("failed to load image config")),
            None => Ok(()),
        }
    }
}

/// Go `image.Decode` over the formats neohugo registers: `image/jpeg` and `image/png` are
/// ported; GIF, WebP (`golang.org/x/image/webp`), TIFF and BMP are recognised by their magic
/// and fail with an explicit unsupported error.
pub fn decode(r: &mut dyn Read) -> Result<GoImage> {
    register_formats();
    match go_image::decode(r) {
        Ok((img, _)) => Ok(GoImage::new(img)),
        Err(e) => Err(Error::new(e.to_string())),
    }
}

/// Go `image.DecodeConfig` (see [`decode`] for the formats).
pub fn decode_config(r: &mut dyn Read) -> Result<ImageDims> {
    register_formats();
    match go_image::decode_config(r) {
        Ok((c, _)) => Ok(ImageDims {
            width: c.width,
            height: c.height,
        }),
        Err(e) => Err(Error::new(e.to_string())),
    }
}

fn unsupported_format_error(name: &str) -> go_image::BoxError {
    format!("neohugo-rs: decoding {name} images is not supported").into()
}

fn register_formats() {
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        go_image::jpeg::register();
        go_png::register();
        // Go registers these through image/gif, golang.org/x/image/{webp,tiff,bmp}.
        go_image::register_format(
            "gif",
            b"GIF8?a",
            |_| Err(unsupported_format_error("gif")),
            |_| Err(unsupported_format_error("gif")),
        );
        go_image::register_format(
            "webp",
            b"RIFF????WEBPVP8",
            |_| Err(unsupported_format_error("webp")),
            |_| Err(unsupported_format_error("webp")),
        );
        for magic in [b"II*\x00".as_slice(), b"MM\x00*".as_slice()] {
            go_image::register_format(
                "tiff",
                magic,
                |_| Err(unsupported_format_error("tiff")),
                |_| Err(unsupported_format_error("tiff")),
            );
        }
        go_image::register_format(
            "bmp",
            b"BM????\x00\x00\x00\x00",
            |_| Err(unsupported_format_error("bmp")),
            |_| Err(unsupported_format_error("bmp")),
        );
    });
}

/// `Image.EncodeTo` without the receiver (Go never reads it).
// Go: resources/images/image.go:EncodeTo
pub fn encode_to(conf: &ImageConfig, img: &GoImage, w: &mut dyn Write) -> Result<()> {
    let Some(target) = conf.target_format else {
        return Err(Error::new("format not supported"));
    };
    match target {
        Format::Jpeg => {
            let quality = conf.quality;
            let opts = go_image::jpeg::Options { quality };

            if let Some(nrgba) = img.downcast_ref::<NRGBA>()
                && nrgba.opaque()
            {
                // The same Pix/Stride/Rect as an *image.RGBA (the fast rgbaToYCbCr path).
                let rgba = RGBA {
                    pix: nrgba.pix.clone(),
                    stride: nrgba.stride,
                    rect: nrgba.rect,
                };
                return go_image::jpeg::encode(w, &rgba, Some(&opts))
                    .map_err(|e| Error::new(e.to_string()));
            }
            go_image::jpeg::encode(w, img.image(), Some(&opts))
                .map_err(|e| Error::new(e.to_string()))
        }
        Format::Png => {
            let encoder = go_png::Encoder {
                compression_level: go_png::DEFAULT_COMPRESSION,
                buffer_pool: None,
            };
            encoder
                .encode(w, img.image())
                .map_err(|e| Error::new(e.to_string()))
        }
        // (A `Giphy` source cannot exist: GIF decoding is not ported.)
        Format::Gif => crate::gif::encode(w, img.image()),
        Format::Tiff => Err(Error::new("neohugo-rs: TIFF encoding is not supported")),
        Format::Bmp => Err(Error::new("neohugo-rs: BMP encoding is not supported")),
        Format::Webp => crate::webp::encode(
            w,
            img,
            libwebp_sys::EncodingOptions {
                quality: conf.quality,
                encoding_preset: libwebp_sys::EncodingPreset(conf.hint),
                use_sharp_yuv: true,
            },
        ),
    }
}

/// Go: `images.ImageProcessor`.
pub struct ImageProcessor {
    pub cfg: Arc<ConfigNamespace<ImagingConfig, ImagingConfigInternal>>,
    pub(crate) exif_decoder: exif::Decoder,
}

impl ImageProcessor {
    // Go: resources/images/image.go:NewImageProcessor
    pub fn new(
        cfg: Arc<ConfigNamespace<ImagingConfig, ImagingConfigInternal>>,
    ) -> Result<Arc<ImageProcessor>> {
        let e = &cfg.config.imaging.exif;
        let exif_decoder = exif::Decoder::new(&[
            exif::DecoderOption::WithDateDisabled(e.disable_date),
            exif::DecoderOption::WithLatLongDisabled(e.disable_lat_long),
            exif::DecoderOption::ExcludeFields(e.exclude_fields.clone()),
            exif::DecoderOption::IncludeFields(e.include_fields.clone()),
        ])?;

        Ok(Arc::new(ImageProcessor { cfg, exif_decoder }))
    }

    /// Filename is only used for logging.
    // Go: resources/images/image.go:DecodeExif
    pub fn decode_exif(
        &self,
        filename: &str,
        format: exif::ImageFormat,
        r: &mut dyn exif::ReadSeek,
    ) -> Result<ExifInfo> {
        self.exif_decoder.decode(filename, format, r)
    }

    /// Go: `FiltersFromConfig(src, conf)` — resize -> `gift.Resize(W, H, Box)`, etc.
    // Go: resources/images/image.go:FiltersFromConfig
    pub fn filters_from_config(
        &self,
        src: &GoImage,
        conf: &ImageConfig,
    ) -> Result<Vec<GiftFilter>> {
        let mut filters: Vec<GiftFilter> = Vec::new();

        if conf.rotate != 0 {
            // Apply any rotation before any resize.
            filters.push(GiftFilter::Gift(gift::rotate(
                conf.rotate as f32,
                go_image::color::Color::Alpha16(go_image::color::TRANSPARENT),
                gift::NEAREST_NEIGHBOR_INTERPOLATION,
            )));
        }

        let resampling = || -> Result<&'static dyn gift::Resampling> {
            gift::hugo_resampling::image_filter(&conf.filter).ok_or_else(|| {
                Error::new("runtime error: invalid memory address or nil pointer dereference")
            })
        };
        let anchor = gift::Anchor(conf.anchor);

        match conf.action.as_str() {
            "resize" => {
                filters.push(GiftFilter::Gift(gift::resize(
                    conf.width,
                    conf.height,
                    resampling()?,
                )));
            }
            "crop" => {
                if conf.anchor == crate::smartcrop::SMART_CROP_ANCHOR {
                    let bounds = self.smart_crop(src, conf.width, conf.height, resampling()?)?;

                    // First crop using the bounds returned by smartCrop.
                    filters.push(GiftFilter::Gift(gift::crop(bounds)));
                    // Then center crop the image to get an image the desired size without
                    // resizing.
                    filters.push(GiftFilter::Gift(gift::crop_to_size(
                        conf.width,
                        conf.height,
                        gift::CENTER_ANCHOR,
                    )));
                } else {
                    filters.push(GiftFilter::Gift(gift::crop_to_size(
                        conf.width,
                        conf.height,
                        anchor,
                    )));
                }
            }
            "fill" => {
                if conf.anchor == crate::smartcrop::SMART_CROP_ANCHOR {
                    let bounds = self.smart_crop(src, conf.width, conf.height, resampling()?)?;

                    // First crop it, then resize it.
                    filters.push(GiftFilter::Gift(gift::crop(bounds)));
                    filters.push(GiftFilter::Gift(gift::resize(
                        conf.width,
                        conf.height,
                        resampling()?,
                    )));
                } else {
                    filters.push(GiftFilter::Gift(gift::resize_to_fill(
                        conf.width,
                        conf.height,
                        resampling()?,
                        anchor,
                    )));
                }
            }
            "fit" => {
                filters.push(GiftFilter::Gift(gift::resize_to_fit(
                    conf.width,
                    conf.height,
                    resampling()?,
                )));
            }
            _ => {}
        }
        Ok(filters)
    }

    // Go: resources/images/image.go:ApplyFiltersFromConfig
    pub fn apply_filters_from_config(&self, src: &GoImage, conf: &ImageConfig) -> Result<GoImage> {
        let filters = self.filters_from_config(src, conf)?;

        if filters.is_empty() {
            return Ok(self.resolve_src(src, conf.target_format));
        }

        self.do_filter(src, conf.target_format, &filters)
    }

    // Go: resources/images/image.go:Filter
    pub fn filter(&self, src: &GoImage, filters: &[GiftFilter]) -> Result<GoImage> {
        self.do_filter(src, None, filters)
    }

    /// Go returns the first frame of a (possibly animated) GIF here; GIF decoding is not
    /// supported by the port, so no `Giphy` source can exist.
    // Go: resources/images/image.go:resolveSrc
    pub(crate) fn resolve_src(&self, src: &GoImage, _target_format: Option<Format>) -> GoImage {
        src.clone()
    }

    /// Go: `doFilter` — destination type by source type (RGBA->RGBA, NRGBA->NRGBA, Gray->Gray,
    /// everything else -> NRGBA), then `gift.Draw`. (The `Giphy` branch cannot be reached: GIF
    /// decoding is not supported.)
    // Go: resources/images/image.go:doFilter
    pub(crate) fn do_filter(
        &self,
        src: &GoImage,
        _target_format: Option<Format>,
        filters: &[GiftFilter],
    ) -> Result<GoImage> {
        let mut gfilters: Vec<Arc<dyn gift::Filter>> = Vec::with_capacity(filters.len());
        for f in filters {
            gfilters.push(f.to_gift()?);
        }
        let filter = gift::new(gfilters);

        // The panics Go's padding filter raises inside Draw, checked up front (the bounds each
        // filter of the chain receives).
        let mut b = src.bounds();
        for (f, g) in filters.iter().zip(&filter.filters) {
            if let GiftFilter::Padding(p) = f {
                p.check(b)?;
            }
            b = g.bounds(b);
        }

        let bounds = filter.bounds(src.bounds());

        let s = src.image();
        let dst: Box<dyn go_image::Image> = if s.is::<RGBA>() {
            let mut dst = RGBA::new(bounds);
            filter.draw(&mut dst, s);
            Box::new(dst)
        } else if s.is::<NRGBA>() {
            let mut dst = NRGBA::new(bounds);
            filter.draw(&mut dst, s);
            Box::new(dst)
        } else if s.is::<Gray>() {
            let mut dst = Gray::new(bounds);
            filter.draw(&mut dst, s);
            Box::new(dst)
        } else {
            let mut dst = NRGBA::new(bounds);
            filter.draw(&mut dst, s);
            Box::new(dst)
        };

        Ok(GoImage::new(dst))
    }
}

/// A gift filter (Go `gift.Filter`): built-ins from the gift crate and Hugo's filters that
/// implement `gift::Filter` directly (mask, opacity, padding), or the Hugo filters that need
/// work before drawing (the overlay decodes its source) or that Go never draws (process,
/// auto-orient: `panic("not supported")`).
#[derive(Clone)]
pub enum GiftFilter {
    Gift(Arc<dyn gift::Filter>),
    Overlay(Arc<crate::overlay::OverlayFilter>),
    Process(Arc<crate::process::ProcessFilter>),
    Mask(Arc<crate::mask::MaskFilter>),
    Padding(Arc<crate::padding::PaddingFilter>),
    AutoOrient(Arc<crate::auto_orient::AutoOrientFilter>),
}

impl GiftFilter {
    /// The filter as gift draws it. Sources are decoded here (Go decodes them inside `Draw`
    /// and panics on an error; the port returns the error).
    pub(crate) fn to_gift(&self) -> Result<Arc<dyn gift::Filter>> {
        match self {
            GiftFilter::Gift(f) => Ok(f.clone()),
            GiftFilter::Padding(p) => Ok(p.clone()),
            GiftFilter::Overlay(o) => Ok(Arc::new(o.prepare()?)),
            GiftFilter::Mask(m) => Ok(Arc::new(m.prepare()?)),
            // Go: processFilter.Draw / autoOrientFilter.Draw panic("not supported").
            GiftFilter::Process(_) | GiftFilter::AutoOrient(_) => Err(Error::new("not supported")),
        }
    }
}

/// Go: `images.GetDefaultImageConfig(defaults)`.
// Go: resources/images/image.go:GetDefaultImageConfig
pub fn get_default_image_config(
    defaults: &ConfigNamespace<ImagingConfig, ImagingConfigInternal>,
) -> ImageConfig {
    crate::config::get_default_image_config(defaults)
}

// Go: resources/images/image.go:imageConfigFromImage
fn image_config_from_image(img: &GoImage) -> ImageDims {
    let b = img.bounds();
    ImageDims {
        width: b.max.x,
        height: b.max.y,
    }
}

/// UnwrapFilter unwraps the given filter if it is a filter wrapper.
// Go: resources/images/image.go:UnwrapFilter
pub fn unwrap_filter(f: &FilterObject) -> &GiftFilter {
    &f.filter
}

/// Go: `images.ToFilters(in any) []gift.Filter` — from template values (`images.filter`
/// objects, `[]gift.Filter`/`[]images.filter` lists of them). Go panics with
/// `"%T is not an image filter"` for anything else; the port returns that as an error.
// Go: resources/images/image.go:ToFilters
pub fn to_filters(v: &go_value::Value) -> Result<Vec<FilterObject>> {
    let not_filter = || Error::new(format!("{} is not an image filter", v.go_type_name()));
    match v {
        go_value::Value::Object(_) => match v.downcast::<FilterObject>() {
            Some(f) => Ok(vec![f.clone()]),
            None => Err(not_filter()),
        },
        go_value::Value::List(l)
            if matches!(&*l.ty.go_name(), "[]gift.Filter" | "[]images.filter") =>
        {
            let mut out = Vec::with_capacity(l.items.len());
            for item in &l.items {
                match item.downcast::<FilterObject>() {
                    Some(f) => out.push(f.clone()),
                    // A nil gift.Filter in a []gift.Filter.
                    None => return Err(not_filter()),
                }
            }
            Ok(out)
        }
        _ => Err(not_filter()),
    }
}

/// IsOpaque returns false if the image has alpha channel and there is at least 1 pixel that
/// is not (fully) opaque.
// Go: resources/images/image.go:IsOpaque
pub fn is_opaque(img: &GoImage) -> bool {
    img.image().try_opaque().unwrap_or(false)
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: resources/images/image.go (475 lines; 19/25 funcs executed)
//   types: Image, ImageProcessor, Spec, Format, imageConfig, ImageSource, Giphy
// OK L44-57: NewImage(f Format, proc *ImageProcessor, img image.Image, s Spec) *Image
// OK L66-115: (i *Image) EncodeTo(conf ImageConfig, img image.Image, w io.Writer) error
// OK L118-122: (i *Image) Height() int
// OK L125-129: (i *Image) Width() int
// OK L131-139: (i Image) WithImage(img image.Image) *Image
// OK L141-145: (i Image) WithSpec(s Spec) *Image
// OK L148-154: (i *Image) InitConfig(r io.Reader) error
// OK L156-179: (i *Image) initConfig() error
// OK L181-198: NewImageProcessor(warnl logg.LevelLogger, cfg *config.ConfigNamespace[ImagingConfig, ImagingConfigInternal]) (*ImageProcessor, error)
// OK L206-208: (p *ImageProcessor) DecodeExif(filename string, format imagemeta.ImageFormat, r io.Reader) (*exif.ExifInfo, error)
// OK L210-256: (p *ImageProcessor) FiltersFromConfig(src image.Image, conf ImageConfig) ([]gift.Filter, error)
// OK L258-274: (p *ImageProcessor) ApplyFiltersFromConfig(src image.Image, conf ImageConfig) (image.Image, error)
// OK L276-278: (p *ImageProcessor) Filter(src image.Image, filters ...gift.Filter) (image.Image, error)
// OK L280-288: (p *ImageProcessor) resolveSrc(src image.Image, targetFormat Format) image.Image
// OK L290-331: (p *ImageProcessor) doFilter(src image.Image, targetFormat Format, filters ...gift.Filter) (image.Image, error)
// OK L333-342: GetDefaultImageConfig(defaults *config.ConfigNamespace[ImagingConfig, ImagingConfigInternal]) ImageConfig
// OK L361-374: (f Format) ToImageMetaImageFormatFormat() imagemeta.ImageFormat
// OK L378-380: (f Format) RequiresDefaultQuality() bool
// OK L383-385: (f Format) SupportsTransparency() bool
// OK L389-391: (f Format) DefaultExtension() string
// OK L394-411: (f Format) MediaType() media.Type
// OK L419-425: imageConfigFromImage(img image.Image) image.Config
// OK L428-433: UnwrapFilter(in gift.Filter) gift.Filter
// OK L436-451: ToFilters(in any) []gift.Filter
// OK L455-463: IsOpaque(img image.Image) bool
// ---------------------------------------------------------------------------
