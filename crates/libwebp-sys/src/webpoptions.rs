//! Port of `github.com/bep/gowebp@v0.3.0/libwebp/webpoptions/options.go`.

/// Go: `type EncodingPreset int`.
///
/// Kept as an open integer newtype (not a closed Rust enum) because Go
/// converts arbitrary ints (Hugo: `webpoptions.EncodingPreset(conf.Hint)`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct EncodingPreset(pub i64);

impl EncodingPreset {
    pub const DEFAULT: EncodingPreset = EncodingPreset(0);
    pub const PICTURE: EncodingPreset = EncodingPreset(1);
    pub const PHOTO: EncodingPreset = EncodingPreset(2);
    pub const DRAWING: EncodingPreset = EncodingPreset(3);
    pub const ICON: EncodingPreset = EncodingPreset(4);
    pub const TEXT: EncodingPreset = EncodingPreset(5);
}

/// Go: `EncodingPresetDefault` .. `EncodingPresetText`.
pub const ENCODING_PRESET_DEFAULT: EncodingPreset = EncodingPreset::DEFAULT;
pub const ENCODING_PRESET_PICTURE: EncodingPreset = EncodingPreset::PICTURE;
pub const ENCODING_PRESET_PHOTO: EncodingPreset = EncodingPreset::PHOTO;
pub const ENCODING_PRESET_DRAWING: EncodingPreset = EncodingPreset::DRAWING;
pub const ENCODING_PRESET_ICON: EncodingPreset = EncodingPreset::ICON;
pub const ENCODING_PRESET_TEXT: EncodingPreset = EncodingPreset::TEXT;

/// Go: `webpoptions.EncodingOptions`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct EncodingOptions {
    /// Quality is a number between 0 and 100. Set to 0 for lossless.
    /// (Go `int`.)
    pub quality: i64,

    /// The encoding preset to use.
    pub encoding_preset: EncodingPreset,

    /// Use sharp (and slow) RGB->YUV conversion.
    pub use_sharp_yuv: bool,
}
