//! Port of `resources/image.go`.
//!
//! Owner: Wave B task T14 (resources-core).

//! Go `resources/image.go`: `imageResource` — Resize/Filter/... with the `_hu_` naming formula
//! `p1 + "_hu_" + HashStringHex(incomingID, root hash, conf.Key, imagingConfig.SourceHash) + ext`
//! and the Filter key `HashString(gfilters)` (specs/images.md §3-4).
//!
//! DECODE FROM ENCODED BYTES (images.md §3.1, §3.5, §9.10): every `Resize`/`Filter` decodes the
//! ENCODED bytes of its parent (Go `DecodeImage()` reads the file cache entry of an intermediate:
//! a JPEG/PNG round trip), and `images.Overlay` decodes the watermark's encoded PNG. The Rust
//! port never reads `resources/_gen`, so `ImageCache` keeps the encoded bytes of every processed
//! image in memory (keyed like Go's file cache) and `decode_image` decodes those bytes. Never
//! pass decoded pixels from one step to the next: the results differ.

use std::borrow::Cow;
use std::sync::{Arc, OnceLock};

use go_image::draw::{self, Drawer};
use go_value::{GoString, HostCtx, Map, MapType, Object, Value};
use nh_common::Result;
use nh_common::herrors::Error;
use nh_images::config::ImageConfig;
use nh_images::exif::ExifInfo;
use nh_images::image::{Format, GiftFilter, GoImage, Image};
use nh_images::process::ImageProcessSpecProvider;
use nh_resource::internal::resourcepaths::ResourcePaths;

use crate::resource::GenericResource;
use crate::transform::{ResourceAdapter, TransformationUpdate};

/// Go: `imageMeta` (`Exif *exif.ExifInfo`); `None` = nil.
type ImageMeta = Option<Arc<ExifInfo>>;

/// Go: `resources.imageResource`.
pub struct ImageResource {
    pub image: Image,
    /// When a image is processed in a chain, this holds the reference to the original (first).
    /// `None` = this is the root (Go `ir.root = ir`).
    pub(crate) root: Option<Arc<ImageResource>>,
    pub base: Arc<GenericResource>,
    /// Go `metaInit` + `meta` + `metaInitErr`.
    meta: OnceLock<std::result::Result<ImageMeta, String>>,
}

/// The parts of a new `imageResource` that are still being set up (Go mutates the clone
/// through its `baseResource` before handing it out).
pub(crate) struct ImageResourceParts {
    pub(crate) image: Image,
    pub(crate) root: Arc<ImageResource>,
    pub(crate) base: GenericResource,
}

impl ImageResourceParts {
    pub(crate) fn build(self) -> ImageResource {
        ImageResource {
            image: self.image,
            root: Some(self.root),
            base: Arc::new(self.base),
            meta: OnceLock::new(),
        }
    }
}

/// `%T` of the error Go wraps: `&os.PathError{Op: conf.Action, Path: i.TargetPath(), Err: err}`.
fn path_error(op: &str, path: &str, err: Error) -> Error {
    err.wrap(format!("{op} {path}"))
}

impl ImageResource {
    /// A root image resource (Go: `ir := &imageResource{...}; ir.root = ir`).
    pub(crate) fn new_root(image: Image, base: Arc<GenericResource>) -> ImageResource {
        ImageResource {
            image,
            root: None,
            base,
            meta: OnceLock::new(),
        }
    }

    /// An image resource from its parts (Go's struct literal).
    pub(crate) fn from_parts(
        image: Image,
        root: Arc<ImageResource>,
        base: Arc<GenericResource>,
    ) -> ImageResource {
        ImageResource {
            image,
            root: Some(root),
            base,
            meta: OnceLock::new(),
        }
    }

    /// Go `i.root` (the resource itself for a root).
    pub(crate) fn root(self: &Arc<Self>) -> Arc<ImageResource> {
        match &self.root {
            Some(r) => r.clone(),
            None => self.clone(),
        }
    }

    /// Go: `Exif()` — the Exif data of the root image (`None` = nil).
    // Go: resources/image.go:(*imageResource).Exif
    pub fn exif(self: &Arc<Self>) -> Result<Option<Arc<ExifInfo>>> {
        self.root().get_exif()
    }

    /// Go: `getExif()`. Go reads or writes the metadata through the images file cache; COLD-CACHE
    /// RULE: the port always decodes (Go's create path). Go panics when the source cannot be
    /// opened (`metadata init failed: …`); the port returns that message.
    // Go: resources/image.go:(*imageResource).getExif
    fn get_exif(&self) -> Result<Option<Arc<ExifInfo>>> {
        let m = self.meta.get_or_init(|| {
            let mf = self.image.format.to_image_meta_image_format();
            if mf == nh_images::exif::ImageFormat::Unknown {
                // No Exif support for this format.
                return Ok(None);
            }

            let mut f = match self.base.read_seek_closer() {
                Ok(f) => f,
                Err(e) => return Err(e.message().to_string()),
            };

            let filename = self.base.get_resource_paths().path();
            match self.base.spec.imaging.decode_exif(&filename, mf, &mut f) {
                Ok(x) => Ok(Some(Arc::new(x))),
                Err(_) => {
                    self.base.spec.logger.warnf(format!(
                        "Unable to decode Exif metadata from image: {}",
                        self.base.key()
                    ));
                    Ok(None)
                }
            }
        });

        match m {
            Ok(m) => Ok(m.clone()),
            Err(e) => Err(Error::new(format!("metadata init failed: {e}"))),
        }
    }

    /// Colors returns a slice of the most dominant colors in an image using a simple histogram
    /// method (`github.com/marekm4/color-extractor`, not ported: explicit error).
    // Go: resources/image.go:(*imageResource).Colors
    pub fn colors(&self) -> Result<Vec<nh_images::color::Color>> {
        Err(Error::feature_not_available(
            "neohugo-rs: image Colors (color-extractor) is not supported",
        ))
    }

    // Go: resources/image.go:(*imageResource).targetPath
    pub fn target_path(&self) -> String {
        self.base.target_path()
    }

    /// Clone is for internal use.
    // Go: resources/image.go:(*imageResource).Clone
    pub fn clone_image(self: &Arc<Self>) -> ImageResource {
        let gr = Arc::new(self.base.clone_resource());
        ImageResource {
            root: Some(self.root()),
            image: self.image.with_spec(gr.clone()),
            base: gr,
            meta: OnceLock::new(),
        }
    }

    // Go: resources/image.go:(*imageResource).cloneTo
    pub(crate) fn clone_to(self: &Arc<Self>, target_path: &str) -> ImageResource {
        let gr = Arc::new(self.base.clone_to(target_path));
        ImageResource {
            root: Some(self.root()),
            image: self.image.with_spec(gr.clone()),
            base: gr,
            meta: OnceLock::new(),
        }
    }

    // Go: resources/image.go:(*imageResource).cloneWithUpdates
    pub(crate) fn clone_with_updates(
        self: &Arc<Self>,
        u: &TransformationUpdate,
    ) -> Result<ImageResource> {
        let base = Arc::new(self.base.clone_with_updates(u)?);

        let img = if u.is_content_changed() {
            self.image.with_spec(base.clone())
        } else {
            self.image.clone()
        };

        Ok(ImageResource {
            root: Some(self.root()),
            image: img,
            base,
            meta: OnceLock::new(),
        })
    }

    /// Process processes the image with the given spec. The spec can contain an optional
    /// action, one of "resize", "crop", "fit" or "fill".
    // Go: resources/image.go:(*imageResource).Process
    pub fn process(self: &Arc<Self>, spec: &str) -> Result<Arc<ResourceAdapter>> {
        self.process_action_spec("", spec)
    }

    /// Resize resizes the image to the specified width and height using the specified
    /// resampling filter and returns the transformed image. If one of width or height is 0,
    /// the image aspect ratio is preserved.
    // Go: resources/image.go:(*imageResource).Resize
    pub fn resize(self: &Arc<Self>, spec: &str) -> Result<Arc<ResourceAdapter>> {
        self.process_action_spec(nh_images::config::ACTION_RESIZE, spec)
    }

    /// Crop the image to the specified dimensions without resizing using the given anchor
    /// point.
    // Go: resources/image.go:(*imageResource).Crop
    pub fn crop(self: &Arc<Self>, spec: &str) -> Result<Arc<ResourceAdapter>> {
        self.process_action_spec(nh_images::config::ACTION_CROP, spec)
    }

    /// Fit scales down the image using the specified resample filter to fit the specified
    /// maximum width and height.
    // Go: resources/image.go:(*imageResource).Fit
    pub fn fit(self: &Arc<Self>, spec: &str) -> Result<Arc<ResourceAdapter>> {
        self.process_action_spec(nh_images::config::ACTION_FIT, spec)
    }

    /// Fill scales the image to the smallest possible size that will cover the specified
    /// dimensions, crops the resized image to the specified dimensions using the given anchor
    /// point.
    // Go: resources/image.go:(*imageResource).Fill
    pub fn fill(self: &Arc<Self>, spec: &str) -> Result<Arc<ResourceAdapter>> {
        self.process_action_spec(nh_images::config::ACTION_FILL, spec)
    }

    // Go: resources/image.go:(*imageResource).Filter
    pub fn filter(self: &Arc<Self>, filters: &[Value]) -> Result<Arc<ResourceAdapter>> {
        let mut gfilters: Vec<nh_images::filters::FilterObject> = Vec::new();

        for f in filters {
            gfilters.extend(nh_images::image::to_filters(f)?);
        }

        let mut options: Vec<String> = Vec::new();

        for f in &gfilters {
            if let GiftFilter::Process(p) = nh_images::image::unwrap_filter(f) {
                options.extend(split_fields(p.image_process_spec()));
            }
        }

        let mut conf_main = nh_images::config::decode_image_config(
            &options,
            &self.image.proc.cfg,
            self.image.format,
        )?;

        conf_main.action = "filter".to_string();
        conf_main.key = nh_images::filters::filters_key(&gfilters);

        let this = self.clone();
        self.do_with_image_config(conf_main, &move |src: &GoImage| -> Result<GoImage> {
            let mut filters: Vec<GiftFilter> = Vec::new();
            for f in &gfilters {
                let f = nh_images::image::unwrap_filter(f);
                match f {
                    GiftFilter::Process(p) => {
                        let options = split_fields(p.image_process_spec());
                        let conf = nh_images::config::decode_image_config(
                            &options,
                            &this.image.proc.cfg,
                            this.image.format,
                        )?;
                        let p_filters = this.image.proc.filters_from_config(src, &conf)?;
                        filters.extend(p_filters);
                    }
                    GiftFilter::AutoOrient(o) => {
                        use nh_images::auto_orient::ImageFilterFromOrientationProvider;
                        let exif = this.exif()?;
                        if let Some(tf) = o.auto_orient(exif.as_deref()) {
                            filters.push(GiftFilter::Gift(tf));
                        }
                    }
                    other => filters.push(other.clone()),
                }
            }
            this.image.proc.filter(src, &filters)
        })
    }

    // Go: resources/image.go:(*imageResource).processActionSpec
    fn process_action_spec(
        self: &Arc<Self>,
        action: &str,
        spec: &str,
    ) -> Result<Arc<ResourceAdapter>> {
        let mut options = vec![action.to_string()];
        let lower = go_unicode::strings::to_lower(spec.as_bytes()).into_owned();
        options.extend(split_fields(&String::from_utf8_lossy(&lower)));
        self.process_options(&options)
    }

    // Go: resources/image.go:(*imageResource).processOptions
    pub(crate) fn process_options(
        self: &Arc<Self>,
        options: &[String],
    ) -> Result<Arc<ResourceAdapter>> {
        let conf = nh_images::config::decode_image_config(
            options,
            &self.image.proc.cfg,
            self.image.format,
        )?;

        let this = self.clone();
        let conf2 = conf.clone();
        let img = self.do_with_image_config(conf.clone(), &move |src: &GoImage| {
            this.image.proc.apply_filters_from_config(src, &conf2)
        })?;

        if conf.action == nh_images::config::ACTION_FILL {
            // (Go: `conf.Anchor == SmartCropAnchor && img.Width() == 0 || img.Height() == 0`.)
            let w = img.width()?;
            let h = img.height()?;
            if conf.anchor == nh_images::smartcrop::SMART_CROP_ANCHOR && w == 0 || h == 0 {
                // See https://github.com/gohugoio/hugo/issues/7955
                // Smartcrop fails silently in some rare cases.
                // Fall back to a center fill.
                let conf = conf.reanchor(gift::CENTER_ANCHOR.0);
                let this = self.clone();
                let conf2 = conf.clone();
                return self.do_with_image_config(conf, &move |src: &GoImage| {
                    this.image.proc.apply_filters_from_config(src, &conf2)
                });
            }
        }

        Ok(img)
    }

    /// Go: `doWithImageConfig(conf, f)` via the ImageCache (keyed by target path; cold path
    /// only). (Go serialises the creation with a one-worker semaphore, `imageProcSem`; that has
    /// no effect on the bytes and the port does not hold a lock while computing.)
    // Go: resources/image.go:(*imageResource).doWithImageConfig
    pub(crate) fn do_with_image_config(
        self: &Arc<Self>,
        conf: ImageConfig,
        f: &dyn Fn(&GoImage) -> Result<GoImage>,
    ) -> Result<Arc<ResourceAdapter>> {
        let spec = self.base.spec.clone();
        spec.image_cache.get_or_create(self, &conf, &|| {
            let src = self
                .decode_image()
                .map_err(|e| path_error(&conf.action, &self.base.target_path(), e))?;

            let mut converted =
                f(&src).map_err(|e| path_error(&conf.action, &self.base.target_path(), e))?;

            let has_alpha = !nh_images::image::is_opaque(&converted);
            let target_supports_transparency = conf
                .target_format
                .map(|t| t.supports_transparency())
                .unwrap_or(true);
            let mut should_fill = conf.bg_color.is_some() && has_alpha;
            should_fill = should_fill || (!target_supports_transparency && has_alpha);
            let mut bg_color: Option<go_image::color::Color> = None;

            if should_fill {
                bg_color = conf.bg_color.or(self.image.proc.cfg.config.bg_color);
                let bounds = converted.bounds();
                let mut tmp = go_image::RGBA::new(bounds);
                let tb = go_image::Image::bounds(&tmp);
                let uniform = go_image::Uniform::new(bg_color.unwrap_or(
                    go_image::color::Color::Alpha16(go_image::color::TRANSPARENT),
                ));
                draw::draw(
                    &mut tmp,
                    tb,
                    &uniform,
                    go_image::Point::default(),
                    draw::Op::Src,
                );
                draw::draw(&mut tmp, tb, converted.image(), bounds.min, draw::Op::Over);
                converted = GoImage::new(Box::new(tmp));
            }

            if conf.target_format == Some(Format::Png) {
                // Apply the colour palette from the source
                if let Some(paletted) = src.downcast_ref::<go_image::Paletted>() {
                    let mut palette = paletted.palette.clone();
                    if let Some(bg) = bg_color {
                        if palette.0.len() < 256 {
                            palette = nh_images::color::add_color_to_palette(bg, palette);
                        } else {
                            nh_images::color::replace_color_in_palette(bg, &mut palette);
                        }
                    }
                    let bounds = converted.bounds();
                    let mut tmp = go_image::Paletted::new(bounds, palette);
                    let tb = tmp.rect;
                    draw::FLOYD_STEINBERG.draw(&mut tmp, tb, converted.image(), bounds.min);
                    converted = GoImage::new(Box::new(tmp));
                }
            }

            let target_format = conf.target()?;
            let mut ci = self.clone_parts(Some(&converted));
            let target_path =
                self.rel_target_path_from_config(&conf, &self.image.proc.cfg.source_hash);
            ci.base.set_target_path(target_path);
            ci.image.format = target_format;
            ci.base.set_media_type(target_format.media_type());

            Ok((ci, converted))
        })
    }

    /// Go: `DecodeImage()` — decode the ENCODED bytes of this (possibly processed) image.
    /// (GIF sources are decoded with `gif.DecodeAll` in Go; GIF decoding is not ported, the
    /// decoder returns its explicit error.)
    // Go: resources/image.go:(*imageResource).DecodeImage
    pub fn decode_image(&self) -> Result<GoImage> {
        let mut f = self
            .base
            .read_seek_closer()
            .map_err(|e| e.wrap("failed to open image for decode"))?;
        nh_images::image::decode(&mut f)
    }

    /// Go: `clone(img)` — the clone's parts, to be completed by the caller.
    // Go: resources/image.go:(*imageResource).clone
    pub(crate) fn clone_parts(self: &Arc<Self>, img: Option<&GoImage>) -> ImageResourceParts {
        let spec = self.base.clone_resource();

        let image = match img {
            Some(img) => self.image.with_image(img),
            // Go: WithSpec(spec) with the clone as the spec; the port reads the clone's source
            // once it is final (the caller builds the resource).
            None => self.image.with_spec(Arc::new(spec.clone_resource())),
        };

        ImageResourceParts {
            image,
            root: self.root(),
            base: spec,
        }
    }

    // Go: resources/image.go:(*imageResource).getImageMetaCacheTargetPath
    pub fn get_image_meta_cache_target_path(&self) -> String {
        // Increment to invalidate the meta cache
        // Last increment: v0.130.0 when change to the new imagemeta library for Exif.
        const IMAGE_META_VERSION_NUMBER: i64 = 2;

        let cfg_hash = &self.image.proc.cfg.source_hash;
        let mut df = self.base.get_resource_paths();
        let (p1, _) = nh_common::paths::path::file_and_ext(&df.file);
        let h = self.base.hash();
        let id_str = nh_common::hashing::hash_string_hex(&[
            Value::Uint(h, go_value::UintKind::Uint64),
            Value::int64(self.base.size()),
            Value::int(IMAGE_META_VERSION_NUMBER),
            Value::string(cfg_hash.as_str()),
        ]);
        df.file = format!("{p1}_{id_str}.json");
        df.target_path()
    }

    // Go: resources/image.go:(*imageResource).relTargetPathFromConfig
    pub(crate) fn rel_target_path_from_config(
        &self,
        conf: &ImageConfig,
        imaging_config_source_hash: &str,
    ) -> ResourcePaths {
        rel_target_path_for_hash(
            &self.base.get_resource_paths(),
            self.image.format,
            conf,
            self.base.hash(),
            imaging_config_source_hash,
        )
    }

    pub fn width(&self) -> i64 {
        self.image.width()
    }

    pub fn height(&self) -> i64 {
        self.image.height()
    }
}

/// The `_hu_` target path of a processed image (Go `relTargetPathFromConfig` with the root
/// source hash `hash` given explicitly, for the key vectors of specs/images.md §4.5).
// Go: resources/image.go:(*imageResource).relTargetPathFromConfig
pub fn rel_target_path_for_hash(
    paths: &ResourcePaths,
    format: Format,
    conf: &ImageConfig,
    hash: u64,
    imaging_config_source_hash: &str,
) -> ResourcePaths {
    let (mut p1, mut p2) = nh_common::paths::path::file_and_ext(&paths.file);
    if conf.target_format != Some(format) {
        p2 = conf
            .target_format
            .map(|t| t.default_extension())
            .unwrap_or_default();
    }

    // Do not change.
    const IMAGE_HASH_PREFIX: &str = "_hu_";

    let mut incoming_id = String::new();
    if let Some(hu_idx) = p1.rfind(IMAGE_HASH_PREFIX) {
        incoming_id = p1[hu_idx + IMAGE_HASH_PREFIX.len()..].to_string();
        p1.truncate(hu_idx);
    }

    let hash = nh_common::hashing::hash_string_hex(&[
        Value::string(incoming_id),
        Value::Uint(hash, go_value::UintKind::Uint64),
        Value::string(conf.key.as_str()),
        Value::string(imaging_config_source_hash),
    ]);
    let mut rp = paths.clone();
    rp.file = format!("{p1}{IMAGE_HASH_PREFIX}{hash}{p2}");

    rp
}

/// Go `strings.Fields(s)`.
fn split_fields(s: &str) -> Vec<String> {
    go_unicode::strings::fields(s.as_bytes())
        .into_iter()
        .map(|f| String::from_utf8_lossy(f).into_owned())
        .collect()
}

/// Go: `*exif.ExifInfo` as a template value (fields `Lat`, `Long`, `Date`, `Tags`).
pub struct ExifInfoObject(pub Arc<ExifInfo>);

impl Object for ExifInfoObject {
    fn type_name(&self) -> Cow<'_, str> {
        Cow::Borrowed("*exif.ExifInfo")
    }
    fn kind(&self) -> go_value::Kind {
        go_value::Kind::Ptr
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
            "Lat" => Some(Value::Float(self.0.lat, go_value::FloatKind::F64)),
            "Long" => Some(Value::Float(self.0.long, go_value::FloatKind::F64)),
            "Date" => Some(Value::Time(self.0.date.clone())),
            "Tags" => {
                let mut m = Map::new(MapType::Named(Arc::from("exif.Tags")));
                for (k, v) in &self.0.tags {
                    m.entries.insert(GoString::from(k.as_str()), v.clone());
                }
                Some(Value::map(m))
            }
            _ => None,
        }
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: resources/image.go (480 lines; 8/21 funcs executed)
//   types: imageResource, imageMeta, giphy
// OK L78-80: (i *imageResource) Exif() *exif.ExifInfo
// OK L82-143: (i *imageResource) getExif() *exif.ExifInfo (cold: always decoded)
// OK L147-161: (i *imageResource) Colors() ([]images.Color, error) (STUB: color-extractor not ported)
// OK L163-165: (i *imageResource) targetPath() string
// OK L168-175: (i *imageResource) Clone() resource.Resource
// OK L177-184: (i *imageResource) cloneTo(targetPath string) resource.Resource
// OK L186-205: (i *imageResource) cloneWithUpdates(u *transformationUpdate) (baseResource, error)
// OK L211-213: (i *imageResource) Process(spec string) (images.ImageResource, error)
// OK L218-220: (i *imageResource) Resize(spec string) (images.ImageResource, error)
// OK L224-226: (i *imageResource) Crop(spec string) (images.ImageResource, error)
// OK L230-232: (i *imageResource) Fit(spec string) (images.ImageResource, error)
// OK L237-239: (i *imageResource) Fill(spec string) (images.ImageResource, error)
// OK L241-293: (i *imageResource) Filter(filters ...any) (images.ImageResource, error)
// OK L295-298: (i *imageResource) processActionSpec(action, spec string) (images.ImageResource, error)
// OK L300-326: (i *imageResource) processOptions(options []string) (images.ImageResource, error)
// OK L337-397: (i *imageResource) doWithImageConfig(conf images.ImageConfig, f func(src image.Image) (image.Image, error)) (images.ImageResource, error)
// OK L404-406: (g *giphy) GIF() *gif.GIF (GIF decoding not ported: no giphy)
// OK L410-426: (i *imageResource) DecodeImage() (image.Image, error)
// OK L428-443: (i *imageResource) clone(img image.Image) *imageResource
// OK L445-457: (i *imageResource) getImageMetaCacheTargetPath() string
// OK L459-480: (i *imageResource) relTargetPathFromConfig(conf images.ImageConfig, imagingConfigSourceHash string) internal.ResourcePaths
// ---------------------------------------------------------------------------
