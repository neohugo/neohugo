//! Decoding sources and encoding results.

use std::io::Cursor;

use image::codecs::bmp::BmpEncoder;
use image::codecs::gif::GifEncoder;
use image::codecs::jpeg::JpegEncoder;
use image::codecs::png::PngEncoder;
use image::codecs::tiff::TiffEncoder;
use image::{ColorType, ExtendedColorType, ImageEncoder, RgbaImage};

use crate::error::ImageError;
use crate::format::ImageFormat;
use crate::pixels::flatten;
use crate::plan::{Encode, Size};
use crate::spec::Hint;

/// Decoded pixels, and whether the source was greyscale (kept greyscale when possible).
pub(crate) struct Decoded {
    pub image: RgbaImage,
    pub gray: bool,
}

/// The size and format of an encoded image, from its header.
pub(crate) fn probe(bytes: &[u8], what: &str) -> Result<(Size, ImageFormat), ImageError> {
    let decode_err = |source| ImageError::Decode {
        what: what.to_owned(),
        source,
    };
    let reader = image::ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()
        .map_err(|e| decode_err(image::ImageError::IoError(e)))?;
    let format = reader
        .format()
        .and_then(ImageFormat::from_codec)
        .ok_or_else(|| {
            decode_err(image::ImageError::Unsupported(
                image::error::UnsupportedError::from_format_and_kind(
                    image::error::ImageFormatHint::Unknown,
                    image::error::UnsupportedErrorKind::Format(
                        image::error::ImageFormatHint::Unknown,
                    ),
                ),
            ))
        })?;
    let size = reader.into_dimensions().map_err(decode_err)?;
    Ok((size, format))
}

/// Decodes an image (the first frame of an animation).
pub(crate) fn decode(bytes: &[u8], what: &str) -> Result<Decoded, ImageError> {
    let img = image::ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()
        .map_err(|e| ImageError::Decode {
            what: what.to_owned(),
            source: image::ImageError::IoError(e),
        })?
        .decode()
        .map_err(|source| ImageError::Decode {
            what: what.to_owned(),
            source,
        })?;
    let gray = matches!(img.color(), ColorType::L8 | ColorType::L16);
    Ok(Decoded {
        image: img.into_rgba8(),
        gray,
    })
}

fn is_opaque(img: &RgbaImage) -> bool {
    img.pixels().all(|p| p.0[3] == 255)
}

fn is_gray(img: &RgbaImage) -> bool {
    img.pixels().all(|p| p.0[0] == p.0[1] && p.0[1] == p.0[2])
}

/// The channel layout written: RGBA when some pixel is transparent, else RGB, or luma for a
/// greyscale source whose result is still grey.
fn packed(img: &RgbaImage, gray_source: bool) -> (Vec<u8>, ExtendedColorType) {
    if !is_opaque(img) {
        return (img.as_raw().clone(), ExtendedColorType::Rgba8);
    }
    if gray_source && is_gray(img) {
        return (
            img.pixels().map(|p| p.0[0]).collect(),
            ExtendedColorType::L8,
        );
    }
    let rgb = img
        .pixels()
        .flat_map(|p| [p.0[0], p.0[1], p.0[2]])
        .collect();
    (rgb, ExtendedColorType::Rgb8)
}

/// The libwebp settings of a hint (libwebp's presets) at `quality`, with sharp YUV.
fn webp_config(quality: u8, hint: Hint) -> Result<webp::WebPConfig, String> {
    let mut c = webp::WebPConfig::new().map_err(|()| "libwebp config version mismatch")?;
    c.quality = f32::from(quality);
    let (sns, sharpness, strength) = match hint {
        Hint::Picture => (80, 4, 35),
        Hint::Photo => (80, 3, 30),
        Hint::Drawing => (25, 6, 10),
        Hint::Icon | Hint::Text => (0, c.filter_sharpness, 0),
    };
    c.sns_strength = sns;
    c.filter_sharpness = sharpness;
    c.filter_strength = strength;
    // Bit 2: dithering, for photos only.
    if hint == Hint::Photo {
        c.preprocessing |= 2;
    } else {
        c.preprocessing &= !2;
    }
    if hint == Hint::Text {
        c.segments = 2;
    }
    c.use_sharp_yuv = 1;
    Ok(c)
}

/// Encodes `img` as the plan says: flattened onto the background when the format cannot
/// store transparency or a background was asked for.
pub(crate) fn encode(
    img: RgbaImage,
    gray_source: bool,
    enc: &Encode,
) -> Result<Vec<u8>, ImageError> {
    let encode_err = |reason: String| ImageError::Encode {
        format: enc.format,
        reason,
    };
    let background = match enc.background {
        Some(bg) => Some(bg),
        None if !enc.format.supports_transparency() => Some(enc.default_background),
        None => None,
    };
    let img = match background {
        Some(mut bg) if !is_opaque(&img) => {
            if !enc.format.supports_transparency() {
                bg.0[3] = 255;
            }
            flatten(&img, bg)
        }
        _ => img,
    };
    let (w, h) = img.dimensions();
    let mut out = Vec::new();
    match enc.format {
        ImageFormat::Jpeg => {
            let (bytes, layout) = packed(&img, gray_source);
            JpegEncoder::new_with_quality(&mut out, enc.quality)
                .write_image(&bytes, w, h, layout)
                .map_err(|e| encode_err(e.to_string()))?;
        }
        ImageFormat::Png => {
            let (bytes, layout) = packed(&img, gray_source);
            PngEncoder::new(&mut out)
                .write_image(&bytes, w, h, layout)
                .map_err(|e| encode_err(e.to_string()))?;
        }
        ImageFormat::Gif => {
            let mut e = GifEncoder::new_with_speed(&mut out, 10);
            e.encode_frame(image::Frame::new(img))
                .map_err(|e| encode_err(e.to_string()))?;
        }
        ImageFormat::Tiff => {
            let (bytes, layout) = packed(&img, gray_source);
            TiffEncoder::new(Cursor::new(&mut out))
                .write_image(&bytes, w, h, layout)
                .map_err(|e| encode_err(e.to_string()))?;
        }
        ImageFormat::Bmp => {
            let (bytes, layout) = packed(&img, gray_source);
            BmpEncoder::new(&mut out)
                .write_image(&bytes, w, h, layout)
                .map_err(|e| encode_err(e.to_string()))?;
        }
        ImageFormat::Webp => {
            let config = webp_config(enc.quality, enc.hint).map_err(encode_err)?;
            let memory = if is_opaque(&img) {
                let rgb: Vec<u8> = img
                    .pixels()
                    .flat_map(|p| [p.0[0], p.0[1], p.0[2]])
                    .collect();
                webp::Encoder::from_rgb(&rgb, w, h).encode_advanced(&config)
            } else {
                webp::Encoder::from_rgba(img.as_raw(), w, h).encode_advanced(&config)
            }
            .map_err(|e| encode_err(format!("{e:?}")))?;
            out.extend_from_slice(&memory);
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn webp_presets_match_libwebp() {
        let photo = webp_config(75, Hint::Photo).expect("config");
        assert!((photo.quality - 75.0).abs() < f32::EPSILON);
        assert_eq!(photo.lossless, 0);
        assert_eq!(photo.use_sharp_yuv, 1);
        assert_eq!(
            (
                photo.sns_strength,
                photo.filter_sharpness,
                photo.filter_strength
            ),
            (80, 3, 30)
        );
        assert_eq!(photo.preprocessing & 2, 2);
        let text = webp_config(90, Hint::Text).expect("config");
        assert_eq!(
            (text.sns_strength, text.filter_strength, text.segments),
            (0, 0, 2)
        );
        assert_eq!(text.preprocessing & 2, 0);
        let drawing = webp_config(50, Hint::Drawing).expect("config");
        assert_eq!(
            (
                drawing.sns_strength,
                drawing.filter_sharpness,
                drawing.filter_strength
            ),
            (25, 6, 10)
        );
    }
}
