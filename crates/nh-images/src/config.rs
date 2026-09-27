//! Port of `resources/images/config.go`.
//!
//! Owner: Wave B task T10 (images).


//! Go `resources/images/config.go`: `[imaging]` decoding and `DecodeImageConfig` (spec strings
//! like `"600x480 webp"`). `ImageConfig.key` = `hashing.HashStringHex(options)` names processed files.

use go_value::Map;
use nh_common::Result;
use nh_config::namespace::ConfigNamespace;

use crate::image::Format;

pub const ACTION_RESIZE: &str = "resize";
pub const ACTION_CROP: &str = "crop";
pub const ACTION_FIT: &str = "fit";
pub const ACTION_FILL: &str = "fill";

/// Go: `webpoptions.EncodingPreset` (0 default ... 2 photo ...).
pub type EncodingPreset = i64;

/// Go: `gift.Anchor` (smart = 1000).
pub type Anchor = i64;

/// Go: `images.ImagingConfig` (user config).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ImagingConfig {
    pub quality: i64,
    pub resample_filter: String,
    pub hint: String,
    pub anchor: String,
    pub bg_color: String,
    pub exif: ExifConfig,
}

/// Go: `images.ExifConfig`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ExifConfig {
    pub include_fields: String,
    pub exclude_fields: String,
    pub disable_date: bool,
    pub disable_lat_long: bool,
}

/// Go: `images.ImagingConfigInternal`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ImagingConfigInternal {
    /// RGBA.
    pub bg_color: [u8; 4],
    pub hint: EncodingPreset,
    /// Resampling filter name ("box").
    pub resample_filter: String,
    pub anchor: Anchor,
    pub imaging: ImagingConfig,
}

/// Go: `images.ImageConfig` — one processing request.
#[derive(Clone, Debug, PartialEq)]
pub struct ImageConfig {
    /// The target format (defaults to the source format).
    pub target_format: Format,
    pub action: String,
    /// `hashing.HashStringHex(options)`; `hashing.HashString(gfilters)` for Filter.
    pub key: String,
    pub quality: i64,
    pub(crate) quality_set_for_image: bool,
    pub rotate: i64,
    pub bg_color: Option<[u8; 4]>,
    pub hint: EncodingPreset,
    pub width: i64,
    pub height: i64,
    pub filter: String,
    pub anchor: Anchor,
}

/// Go: `images.DecodeConfig(in)` -> `ConfigNamespace` whose `source_hash` is the hash of the RAW
/// `[imaging]` map INCLUDING the `_merge` keys (`4bf645f71319dd1d` for seeksnack).
// Go: resources/images/config.go:DecodeConfig
pub fn decode_config(input: &Map) -> Result<ConfigNamespace<ImagingConfig, ImagingConfigInternal>> {
    todo!()
}

/// Go: `images.DecodeImageConfig(options, defaults, sourceFormat)`.
// Go: resources/images/config.go:DecodeImageConfig
pub fn decode_image_config(options: &[String], defaults: &ConfigNamespace<ImagingConfig, ImagingConfigInternal>, source_format: Format) -> Result<ImageConfig> {
    todo!()
}

/// Go: `images.ImageFormatFromExt(ext)`.
// Go: resources/images/config.go:ImageFormatFromExt
pub fn image_format_from_ext(ext: &str) -> Option<Format> {
    todo!()
}

/// Go: `images.ImageFormatFromMediaSubType(sub)`.
// Go: resources/images/config.go:ImageFormatFromMediaSubType
pub fn image_format_from_media_sub_type(sub: &str) -> Option<Format> {
    todo!()
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: resources/images/config.go (490 lines; 6/8 funcs executed)
//   types: ImageConfig, ImagingConfigInternal, ImagingConfig, ExifConfig
// EX L125-128: ImageFormatFromExt(ext string) (Format, bool)
// EX L130-133: ImageFormatFromMediaSubType(sub string) (Format, bool)
// EX L153-159: init()
// EX L161-211: DecodeConfig(in map[string]any) (*config.ConfigNamespace[ImagingConfig, ImagingConfigInternal], error)
// EX L213-344: DecodeImageConfig(options []string, defaults *config.ConfigNamespace[ImagingConfig, ImagingConfigInternal], sourceFormat Format) (ImageConfig, error)
//    L385-389: (cfg ImageConfig) Reanchor(a gift.Anchor) ImageConfig
//    L400-422: (i *ImagingConfigInternal) Compile(externalCfg *ImagingConfig) error
// EX L448-468: (cfg *ImagingConfig) init() error
// ---------------------------------------------------------------------------
