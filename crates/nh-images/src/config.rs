//! Port of `resources/images/config.go`.
//!
//! Owner: Wave B task T10 (images).

//! Go `resources/images/config.go`: `[imaging]` decoding and `DecodeImageConfig` (spec strings
//! like `"600x480 webp"`). `ImageConfig.key` = `hashing.HashStringHex(options)` names processed files.

use std::sync::LazyLock;

use go_image::color::Color as GoColor;
use go_value::{GoString, Map, MapType, Value};
use nh_common::{Error, Result};
use nh_config::decode::FieldRef;
use nh_config::decode_struct;
use nh_config::namespace::ConfigNamespace;

use crate::color::hex_string_to_color_go;
use crate::image::Format;
use crate::smartcrop::{SMART_CROP_ANCHOR, SMART_CROP_IDENTIFIER, SMART_CROP_VERSION_NUMBER};

pub const ACTION_RESIZE: &str = "resize";
pub const ACTION_CROP: &str = "crop";
pub const ACTION_FIT: &str = "fit";
pub const ACTION_FILL: &str = "fill";

/// Go: `images.Actions`.
// Go: resources/images/config.go:Actions
pub fn is_action(s: &str) -> bool {
    matches!(s, ACTION_RESIZE | ACTION_CROP | ACTION_FIT | ACTION_FILL)
}

/// Go: `webpoptions.EncodingPreset` (0 default ... 2 photo ...).
pub type EncodingPreset = i64;

/// Go: `gift.Anchor` (smart = 1000).
pub type Anchor = i64;

/// Go: `images.imageFormats` (keyed by extension with the dot).
// Go: resources/images/config.go:imageFormats
fn image_formats(ext: &str) -> Option<Format> {
    Some(match ext {
        ".jpg" | ".jpeg" | ".jpe" | ".jif" | ".jfif" => Format::Jpeg,
        ".png" => Format::Png,
        ".tif" | ".tiff" => Format::Tiff,
        ".bmp" => Format::Bmp,
        ".gif" => Format::Gif,
        ".webp" => Format::Webp,
        _ => return None,
    })
}

/// Go: `imageFormatsVersions` (all 0).
fn image_formats_versions(f: Format) -> Option<i64> {
    match f {
        Format::Png | Format::Webp | Format::Gif => Some(0),
        _ => None,
    }
}

/// Go: `mainImageVersionNumber`.
const MAIN_IMAGE_VERSION_NUMBER: i64 = 0;

/// Go: `anchorPositions` (lower-cased names).
// Go: resources/images/config.go:anchorPositions
pub fn anchor_position(name: &str) -> Option<Anchor> {
    Some(match name {
        "center" => gift::CENTER_ANCHOR.0,
        "topleft" => gift::TOP_LEFT_ANCHOR.0,
        "top" => gift::TOP_ANCHOR.0,
        "topright" => gift::TOP_RIGHT_ANCHOR.0,
        "left" => gift::LEFT_ANCHOR.0,
        "right" => gift::RIGHT_ANCHOR.0,
        "bottomleft" => gift::BOTTOM_LEFT_ANCHOR.0,
        "bottom" => gift::BOTTOM_ANCHOR.0,
        "bottomright" => gift::BOTTOM_RIGHT_ANCHOR.0,
        SMART_CROP_IDENTIFIER => SMART_CROP_ANCHOR,
        _ => return None,
    })
}

/// Go: `hints` (only relevant for WebP).
// Go: resources/images/config.go:hints
pub fn hint(name: &str) -> Option<EncodingPreset> {
    Some(match name {
        "picture" => 1,
        "photo" => 2,
        "drawing" => 3,
        "icon" => 4,
        "text" => 5,
        _ => return None,
    })
}

/// Go: `imageFilters`: whether `name` (lower-case) is a resampling filter. The filter itself is
/// `gift::hugo_resampling::image_filter(name)`.
// Go: resources/images/config.go:imageFilters
pub fn is_image_filter(name: &str) -> bool {
    gift::hugo_resampling::image_filter(name).is_some()
}

/// Go: `webpoptions.EncodingPresetPhoto`.
const ENCODING_PRESET_PHOTO: EncodingPreset = 2;

// Go: resources/images/config.go:ImageFormatFromExt
/// Go: `images.ImageFormatFromExt(ext)`.
pub fn image_format_from_ext(ext: &str) -> Option<Format> {
    image_formats(ext)
}

// Go: resources/images/config.go:ImageFormatFromMediaSubType
/// Go: `images.ImageFormatFromMediaSubType(sub)` (`jpeg`, `png`, `tiff`, `bmp`, `gif`, `webp`).
pub fn image_format_from_media_sub_type(sub: &str) -> Option<Format> {
    let b = nh_media::media::builtin::builtin();
    let pairs = [
        (&b.jpeg_type, Format::Jpeg),
        (&b.png_type, Format::Png),
        (&b.tiff_type, Format::Tiff),
        (&b.bmp_type, Format::Bmp),
        (&b.gif_type, Format::Gif),
        (&b.webp_type, Format::Webp),
    ];
    pairs
        .into_iter()
        .find(|(t, _)| t.sub_type == sub)
        .map(|(_, f)| f)
}

pub const DEFAULT_JPEG_QUALITY: i64 = 75;
pub const DEFAULT_RESAMPLE_FILTER: &str = "box";
pub const DEFAULT_BG_COLOR: &str = "#ffffff";
pub const DEFAULT_HINT: &str = "photo";

/// Go: `defaultImaging`.
pub fn default_imaging() -> Map {
    let mut m = Map::new(MapType::StringAny);
    m.insert("resampleFilter", Value::string(DEFAULT_RESAMPLE_FILTER));
    m.insert("bgColor", Value::string(DEFAULT_BG_COLOR));
    m.insert("hint", Value::string(DEFAULT_HINT));
    m.insert("quality", Value::int(DEFAULT_JPEG_QUALITY));
    m
}

static DEFAULT_IMAGE_CONFIG: LazyLock<ConfigNamespace<ImagingConfig, ImagingConfigInternal>> =
    LazyLock::new(|| match decode_config(&default_imaging()) {
        Ok(c) => c,
        Err(e) => panic!("{}", e.message()),
    });

/// Go: `defaultImageConfig`, set by the package `init`.
// Go: resources/images/config.go:init
pub fn default_image_config() -> &'static ConfigNamespace<ImagingConfig, ImagingConfigInternal> {
    &DEFAULT_IMAGE_CONFIG
}

/// Go: `images.ImagingConfig` (user config).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ImagingConfig {
    /// Default image quality setting (1-100). Only used for JPEG images.
    pub quality: i64,
    /// Resample filter to use in resize operations.
    pub resample_filter: String,
    /// Hint about what type of image this is (WebP only; see `DecodeImageConfig`).
    pub hint: String,
    /// The anchor to use in Fill. Default is "smart", i.e. Smart Crop.
    pub anchor: String,
    /// Default color used in fill operations (e.g. "fff" for white).
    pub bg_color: String,
    pub exif: ExifConfig,
}

decode_struct!(ImagingConfig, "images.ImagingConfig", |s| vec![
    FieldRef::new("Quality", &mut s.quality),
    FieldRef::new("ResampleFilter", &mut s.resample_filter),
    FieldRef::new("Hint", &mut s.hint),
    FieldRef::new("Anchor", &mut s.anchor),
    FieldRef::new("BgColor", &mut s.bg_color),
    FieldRef::new("Exif", &mut s.exif),
]);

/// Go: `images.ExifConfig`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ExifConfig {
    pub include_fields: String,
    pub exclude_fields: String,
    pub disable_date: bool,
    pub disable_lat_long: bool,
}

decode_struct!(ExifConfig, "images.ExifConfig", |s| vec![
    FieldRef::new("IncludeFields", &mut s.include_fields),
    FieldRef::new("ExcludeFields", &mut s.exclude_fields),
    FieldRef::new("DisableDate", &mut s.disable_date),
    FieldRef::new("DisableLatLong", &mut s.disable_lat_long),
]);

/// Go: `images.ImagingConfigInternal`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ImagingConfigInternal {
    /// Go `color.Color` (`color.White`/`color.Black` are `color.Gray16`, others `color.RGBA`);
    /// `None` is Go's nil.
    pub bg_color: Option<GoColor>,
    /// Never set by `DecodeConfig` (always 0, as in Go).
    pub hint: EncodingPreset,
    /// Resampling filter name, lower case (`"box"`); `""` is Go's nil.
    pub resample_filter: String,
    pub anchor: Anchor,
    pub imaging: ImagingConfig,
}

/// Go: `images.ImageConfig` — one processing request.
#[derive(Clone, Debug, PartialEq)]
pub struct ImageConfig {
    /// The target format (defaults to the source format). `None` is Go's zero `Format`, which
    /// only [`get_default_image_config`] returns.
    pub target_format: Option<Format>,
    pub action: String,
    /// `hashing.HashStringHex(options)`; `hashing.HashString(gfilters)` for Filter.
    pub key: String,
    pub quality: i64,
    pub(crate) quality_set_for_image: bool,
    pub rotate: i64,
    /// Go `color.Color`; `None` is nil.
    pub bg_color: Option<GoColor>,
    pub hint: EncodingPreset,
    pub width: i64,
    pub height: i64,
    /// Resampling filter name (a key of Go's `imageFilters`); `""` is Go's nil.
    pub filter: String,
    pub anchor: Anchor,
}

impl ImageConfig {
    /// Whether the quality was set in this spec (`q75`).
    pub fn quality_set_for_image(&self) -> bool {
        self.quality_set_for_image
    }

    /// The target format; Go's zero `Format` gives `"format not supported"` at encode time.
    pub fn target(&self) -> Result<Format> {
        self.target_format
            .ok_or_else(|| Error::new("format not supported"))
    }

    // Go: resources/images/config.go:Reanchor
    pub fn reanchor(&self, a: Anchor) -> ImageConfig {
        let mut cfg = self.clone();
        cfg.anchor = a;
        cfg.key = nh_common::hashing::hash_string_hex(&[
            Value::string(cfg.key.as_str()),
            Value::string("reanchor"),
            // gift.Anchor is a named int.
            Value::int(a),
        ]);
        cfg
    }
}

impl ImagingConfigInternal {
    // Go: resources/images/config.go:Compile
    pub fn compile(&mut self, external_cfg: &ImagingConfig) -> Result<()> {
        self.bg_color = Some(hex_string_to_color_go(external_cfg.bg_color.as_bytes())?);

        if !external_cfg.anchor.is_empty() {
            match anchor_position(&external_cfg.anchor) {
                Some(anchor) => self.anchor = anchor,
                None => {
                    return Err(Error::new(format!(
                        "invalid anchor value {} in imaging config",
                        go_q_int(self.anchor)
                    )));
                }
            }
        }

        if !is_image_filter(&external_cfg.resample_filter) {
            return Err(Error::new(format!(
                "{} is not a valid resample filter",
                go_q_nil_resampling()
            )));
        }
        self.resample_filter = external_cfg.resample_filter.clone();

        Ok(())
    }
}

impl ImagingConfig {
    // Go: resources/images/config.go:(*ImagingConfig).init
    fn init(&mut self) -> Result<()> {
        if self.quality < 0 || self.quality > 100 {
            return Err(Error::new(
                "image quality must be a number between 1 and 100",
            ));
        }

        self.bg_color = go_lower(go_unicode::strings::trim_prefix(
            self.bg_color.as_bytes(),
            b"#",
        ));
        self.anchor = go_lower(self.anchor.as_bytes());
        self.resample_filter = go_lower(self.resample_filter.as_bytes());
        self.hint = go_lower(self.hint.as_bytes());

        if self.anchor.is_empty() {
            self.anchor = SMART_CROP_IDENTIFIER.to_string();
        }

        if go_unicode::strings::trim_space(self.exif.include_fields.as_bytes()).is_empty()
            && go_unicode::strings::trim_space(self.exif.exclude_fields.as_bytes()).is_empty()
        {
            // Don't change this for no good reason. Please don't.
            self.exif.exclude_fields = "GPS|Exif|Exposure[M|P|B]|Contrast|Resolution|Sharp|JPEG|Metering|Sensing|Saturation|ColorSpace|Flash|WhiteBalance".to_string();
        }

        Ok(())
    }
}

/// `strings.ToLower` over a Go string held in a Rust `String`.
fn go_lower(s: &[u8]) -> String {
    String::from_utf8_lossy(&go_unicode::strings::to_lower(s)).into_owned()
}

/// Go `fmt.Sprintf("%q", anchor)` for a `gift.Anchor` (an int: a quoted rune literal).
fn go_q_int(i: i64) -> String {
    String::from_utf8_lossy(&go_fmt::sprintf("%q", &[Value::int(i)])).into_owned()
}

/// Go `fmt.Sprintf("%q", filter)` for a nil `gift.Resampling` interface.
fn go_q_nil_resampling() -> &'static str {
    "%!q(<nil>)"
}

/// Go: `images.DecodeConfig(in)` -> `ConfigNamespace` whose `source_hash` is the hash of the RAW
/// `[imaging]` map INCLUDING the `_merge` keys (`4bf645f71319dd1d` for seeksnack).
///
/// Go's `buildConfig` merges the defaults into the input map itself when it is a
/// `map[string]interface {}` or `maps.Params` (`maps.ToStringMapE` returns it as is), so the
/// `SourceStructure` of the namespace is the input with the defaults merged in; the port
/// returns that merged map.
// Go: resources/images/config.go:DecodeConfig
pub fn decode_config(input: &Map) -> Result<ConfigNamespace<ImagingConfig, ImagingConfigInternal>> {
    let config_source = Value::map(input.clone());

    let build_config = |v: &Value| -> Result<(ImagingConfigInternal, Option<Value>)> {
        let mut m = nh_common::maps::maps::to_string_map_e(v)?;
        // Merge in the defaults.
        nh_common::maps::maps::merge_shallow(&mut m, &default_imaging());

        // The merged map is the input map in Go (same map), with its original type.
        let ext = match v {
            Value::Map(orig) if matches!(orig.ty, MapType::Params | MapType::StringAny) => Some(
                Value::map(Map::with_entries(orig.ty.clone(), m.entries.clone())),
            ),
            _ => None,
        };

        let mut i = ImagingConfigInternal::default();
        nh_config::decode::decode_into(&Value::map(m), &mut i.imaging)?;

        i.imaging.init()?;

        i.bg_color = Some(hex_string_to_color_go(i.imaging.bg_color.as_bytes())?);

        if !i.imaging.anchor.is_empty() {
            match anchor_position(&i.imaging.anchor) {
                Some(anchor) => i.anchor = anchor,
                None => {
                    return Err(Error::new(format!(
                        "invalid anchor value {} in imaging config",
                        go_q_int(i.anchor)
                    )));
                }
            }
        }

        if !is_image_filter(&i.imaging.resample_filter) {
            return Err(Error::new(format!(
                "{} is not a valid resample filter",
                go_q_nil_resampling()
            )));
        }

        i.resample_filter = i.imaging.resample_filter.clone();

        Ok((i, ext))
    };

    nh_config::namespace::decode_namespace(&config_source, build_config)
        .map_err(|e| e.wrap("failed to decode media types"))
}

/// `strconv.Atoi` with Go's error text.
fn atoi(s: &str) -> Result<i64> {
    go_strconv::atoi(s).map_err(|e| Error::new(e.to_string()))
}

/// Go: `images.DecodeImageConfig(options, defaults, sourceFormat)`.
///
/// Go cleans `options` in place (the caller's slice is modified); the port works on a copy.
// Go: resources/images/config.go:DecodeImageConfig
pub fn decode_image_config(
    options: &[String],
    defaults: &ConfigNamespace<ImagingConfig, ImagingConfigInternal>,
    source_format: Format,
) -> Result<ImageConfig> {
    let mut c = get_default_image_config(defaults);

    // Make to lower case, trim space and remove any empty strings.
    let mut options: Vec<String> = options
        .iter()
        .filter_map(|s| {
            let s = go_unicode::strings::trim_space(s.as_bytes());
            if s.is_empty() {
                None
            } else {
                Some(go_lower(s))
            }
        })
        .collect();

    for part in &options {
        let pb = part.as_bytes();
        if is_action(part) {
            c.action = part.clone();
        } else if let Some(pos) = anchor_position(part) {
            c.anchor = pos;
        } else if is_image_filter(part) {
            c.filter = part.clone();
        } else if let Some(h) = hint(part) {
            c.hint = h;
        } else if pb[0] == b'#' {
            c.bg_color = Some(hex_string_to_color_go(&pb[1..])?);
        } else if pb[0] == b'q' {
            c.quality = atoi(&part[1..])?;
            if c.quality < 1 || c.quality > 100 {
                return Err(Error::new("quality ranges from 1 to 100 inclusive"));
            }
            c.quality_set_for_image = true;
        } else if pb[0] == b'r' {
            c.rotate = atoi(&part[1..])?;
        } else if part.contains('x') {
            let width_height: Vec<&str> = part.split('x').collect();
            if width_height.len() <= 2 {
                let first = width_height[0];
                if !first.is_empty() {
                    c.width = atoi(first)?;
                }

                if width_height.len() == 2 {
                    let second = width_height[1];
                    if !second.is_empty() {
                        c.height = atoi(second)?;
                    }
                }
            } else {
                return Err(Error::new("invalid image dimensions"));
            }
        } else if let Some(f) = image_format_from_ext(&format!(".{part}")) {
            c.target_format = Some(f);
        }
    }

    match c.action.as_str() {
        ACTION_CROP | ACTION_FILL | ACTION_FIT => {
            if c.width == 0 || c.height == 0 {
                return Err(Error::new("must provide Width and Height"));
            }
        }
        ACTION_RESIZE => {
            if c.width == 0 && c.height == 0 {
                return Err(Error::new("must provide Width or Height"));
            }
        }
        _ => {
            if c.width != 0 || c.height != 0 {
                return Err(Error::new(
                    "width or height are not supported for this action",
                ));
            }
        }
    }

    if !c.action.is_empty() && c.filter.is_empty() {
        c.filter = defaults.config.resample_filter.clone();
    }

    if c.hint == 0 {
        c.hint = ENCODING_PRESET_PHOTO;
    }

    if !c.action.is_empty() && c.anchor == -1 {
        c.anchor = defaults.config.anchor;
    }

    // default to the source format
    let target = *c.target_format.get_or_insert(source_format);

    if c.quality <= 0 && target.requires_default_quality() {
        // We need a quality setting for all JPEGs and WEBPs.
        c.quality = defaults.config.imaging.quality;
    }

    if c.bg_color.is_none()
        && target != source_format
        && source_format.supports_transparency()
        && !target.supports_transparency()
    {
        c.bg_color = defaults.config.bg_color;
    }

    if MAIN_IMAGE_VERSION_NUMBER > 0 {
        options.push(MAIN_IMAGE_VERSION_NUMBER.to_string());
    }

    if let Some(v) = image_formats_versions(source_format)
        && v > 0
    {
        options.push(v.to_string());
    }

    if SMART_CROP_VERSION_NUMBER > 0 && c.anchor == SMART_CROP_ANCHOR {
        options.push(SMART_CROP_VERSION_NUMBER.to_string());
    }

    c.key = options_key(&options);

    Ok(c)
}

/// `hashing.HashStringHex(options)` of a `[]string`.
pub(crate) fn options_key(options: &[String]) -> String {
    let items: Vec<Value> = options
        .iter()
        .map(|s| Value::String(GoString::from(s.as_str())))
        .collect();
    nh_common::hashing::hash_string_hex(&[Value::list(go_value::SliceType::String, items)])
}

/// Go: `images.GetDefaultImageConfig(defaults)` (`nil` defaults = the package defaults).
// Go: resources/images/image.go:GetDefaultImageConfig
pub(crate) fn default_image_config_for(
    defaults: Option<&ConfigNamespace<ImagingConfig, ImagingConfigInternal>>,
) -> ImageConfig {
    let defaults = defaults.unwrap_or_else(|| default_image_config());
    ImageConfig {
        target_format: None,
        action: String::new(),
        key: String::new(),
        // The real values start at 0.
        anchor: -1,
        hint: defaults.config.hint,
        quality: defaults.config.imaging.quality,
        quality_set_for_image: false,
        rotate: 0,
        bg_color: None,
        width: 0,
        height: 0,
        filter: String::new(),
    }
}

/// Go: `images.GetDefaultImageConfig(defaults)`.
pub fn get_default_image_config(
    defaults: &ConfigNamespace<ImagingConfig, ImagingConfigInternal>,
) -> ImageConfig {
    default_image_config_for(Some(defaults))
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: resources/images/config.go (490 lines; 6/8 funcs executed)
//   types: ImageConfig, ImagingConfigInternal, ImagingConfig, ExifConfig
// OK L125-128: ImageFormatFromExt(ext string) (Format, bool)
// OK L130-133: ImageFormatFromMediaSubType(sub string) (Format, bool)
// OK L153-159: init()
// OK L161-211: DecodeConfig(in map[string]any) (*config.ConfigNamespace[ImagingConfig, ImagingConfigInternal], error)
// OK L213-344: DecodeImageConfig(options []string, defaults *config.ConfigNamespace[ImagingConfig, ImagingConfigInternal], sourceFormat Format) (ImageConfig, error)
// OK L385-389: (cfg ImageConfig) Reanchor(a gift.Anchor) ImageConfig
// OK L400-422: (i *ImagingConfigInternal) Compile(externalCfg *ImagingConfig) error
// OK L448-468: (cfg *ImagingConfig) init() error
// ---------------------------------------------------------------------------
