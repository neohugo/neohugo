//! Vendored libwebp 1.3.2, compiled exactly like
//! `github.com/bep/gowebp@v0.3.0` compiles it, plus a port of the gowebp Go
//! wrapper (`libwebp/encode.go`, `libwebp/webpoptions/options.go`,
//! `internal/libwebp/a__encoder.go`).
//!
//! Hugo (`resources/images/image.go` EncodeTo, WEBP case) calls
//! `libwebp.Encode(w, img, EncodingOptions{Quality: conf.Quality,
//! EncodingPreset: EncodingPreset(conf.Hint), UseSharpYuv: true})`; the Rust
//! equivalent is [`encode`] (or [`encode_to_vec`]).
//!
//! gowebp has no decoder (Hugo decodes WebP with `golang.org/x/image/webp`).

pub mod encoder;
pub mod ffi;
pub mod image;
pub mod webpoptions;

use std::fmt;

pub use image::{Image, PixView, Point, Rectangle};
pub use webpoptions::{EncodingOptions, EncodingPreset};

/// Errors returned by [`encode`]. The `Display` strings of the Go errors are
/// the Go messages.
#[derive(Debug)]
pub enum Error {
    /// Go: `errors.New("failed to init encoder config")`.
    InitConfig,
    /// Go: `errors.New("failed to init lossless preset")`.
    LosslessPreset,
    /// Go: `errors.New("failed to validate config")`.
    ValidateConfig,
    /// Go: `errors.New("failed to encode")`.
    Encode,
    /// Go panics (`&v.Pix[0]` on an empty `Pix`).
    EmptyPix,
    /// The pixel buffer is smaller than what libwebp would read (Go would
    /// read out of bounds).
    PixOutOfRange,
    /// Error from the destination writer (Go: `w.Write` error).
    Io(std::io::Error),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::InitConfig => f.write_str("failed to init encoder config"),
            Error::LosslessPreset => f.write_str("failed to init lossless preset"),
            Error::ValidateConfig => f.write_str("failed to validate config"),
            Error::Encode => f.write_str("failed to encode"),
            Error::EmptyPix => f.write_str("runtime error: index out of range [0] with length 0"),
            Error::PixOutOfRange => {
                f.write_str("webp: pixel buffer too small for the image bounds")
            }
            Error::Io(e) => fmt::Display::fmt(e, f),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Io(e) => Some(e),
            _ => None,
        }
    }
}

// Go: libwebp/encode.go:Encode
/// Encodes `src` as Webp into `w` using the options in `o`.
///
/// See [`encoder::encode`] for the handling of image kinds other than
/// RGBA/NRGBA/Gray.
pub fn encode<W: std::io::Write + ?Sized>(
    w: &mut W,
    src: &Image<'_>,
    o: EncodingOptions,
) -> Result<(), Error> {
    encoder::encode(w, src, o)
}

/// Convenience wrapper around [`encode`] returning the WebP bytes.
pub fn encode_to_vec(src: &Image<'_>, o: EncodingOptions) -> Result<Vec<u8>, Error> {
    let mut out = Vec::new();
    encode(&mut out, src, o)?;
    Ok(out)
}

/// `WebPGetEncoderVersion()` (0x010302 for libwebp 1.3.2).
pub fn encoder_version() -> i32 {
    unsafe { ffi::WebPGetEncoderVersion() }
}
