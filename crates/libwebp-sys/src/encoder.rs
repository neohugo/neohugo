//! Port of the Go half of
//! `github.com/bep/gowebp@v0.3.0/internal/libwebp/a__encoder.go`.
//!
//! The C half (the cgo preamble) lives verbatim in `csrc/gowebp_encoder.c`.

use std::io::Write;
use std::os::raw::{c_float, c_int};
use std::sync::Once;

use crate::Error;
use crate::ffi;
use crate::image::{Image, PixView, Rectangle};
use crate::webpoptions::EncodingOptions;

// Go: a__encoder.go:Encode
/// Encodes `src` into `w` considering the options in `o`.
///
/// Go converts any `src` that isn't one of `*image.RGBA`, `*image.NRGBA`
/// or `*image.Gray` to `*image.NRGBA` using `draw.Draw` first
/// (`ConvertToNRGBA`). This crate has no Go image model, so that conversion
/// is the caller's job: convert with `image/draw` semantics
/// (`draw.Draw(dst, dst.Bounds(), src, src.Bounds().Min, draw.Src)` into a
/// fresh NRGBA of `src.Bounds()`) and pass [`Image::Nrgba`].
pub fn encode<W: Write + ?Sized>(
    w: &mut W,
    src: &Image<'_>,
    o: EncodingOptions,
) -> Result<(), Error> {
    let mut config = encoding_options_to_c_config(o)?;

    // bounds = src.Bounds()
    let bounds = src.bounds();
    let mut size: usize = 0;

    // The C side reads `width x height` pixels starting at &Pix[0]. Go
    // indexes Pix[0] (panics on an empty Pix) and hands the pointer to C
    // without any bounds check; we refuse inputs whose buffer the C code
    // would overrun instead of reading out of bounds.
    ensure_dsp_initialized();

    let output: *mut u8 = match src {
        Image::Rgba(v) | Image::Nrgba(v) => {
            // *image.RGBA is imported as if it were non-premultiplied, exactly
            // like gowebp does (encodeNRGBA for both).
            let (width, height, stride) = c_dims(&bounds, v);
            check_pix_rgba(v, width, height, stride)?;
            unsafe {
                ffi::gowebp_encodeNRGBA(
                    &mut config,
                    v.pix.as_ptr(),
                    width,
                    height,
                    stride,
                    &mut size,
                )
            }
        }
        Image::Gray(v) => {
            let (width, height, stride) = c_dims(&bounds, v);
            check_pix_gray(v, width, height, stride)?;
            // encodeGray takes a non-const `uint8_t*`. libwebp does not write
            // through it (no alpha plane, so WebPCleanupTransparentArea
            // returns early), but hand C a private copy so the Rust borrow
            // stays immutable.
            let mut y = v.pix.to_vec();
            unsafe {
                ffi::gowebp_encodeGray(
                    &mut config,
                    y.as_mut_ptr(),
                    width,
                    height,
                    stride,
                    &mut size,
                )
            }
        }
    };

    if output.is_null() || size == 0 {
        if !output.is_null() {
            // Go returns here before its deferred C.free (a leak); free it.
            unsafe { ffi::gowebp_free(output.cast()) };
        }
        return Err(Error::Encode);
    }

    // defer C.free(unsafe.Pointer(output))
    struct FreeOnDrop(*mut u8);
    impl Drop for FreeOnDrop {
        fn drop(&mut self) {
            unsafe { ffi::gowebp_free(self.0.cast()) };
        }
    }
    let guard = FreeOnDrop(output);

    // _, err = w.Write(output[0:size])
    let bytes = unsafe { std::slice::from_raw_parts(guard.0, size) };
    w.write_all(bytes).map_err(Error::Io)?;
    Ok(())
}

/// `C.int(bounds.Max.X)`, `C.int(bounds.Max.Y)`, `C.int(v.Stride)`: Go
/// truncates `int` to `int32`.
fn c_dims(bounds: &Rectangle, v: &PixView<'_>) -> (c_int, c_int, c_int) {
    (
        bounds.max.x as c_int,
        bounds.max.y as c_int,
        v.stride as c_int,
    )
}

/// `WEBP_MAX_ALLOCABLE_MEMORY` (`src/utils/utils.h`, 64-bit targets).
const WEBP_MAX_ALLOCABLE_MEMORY: u64 = 1 << 34;
/// `WEBP_ALIGN_CST` (`src/utils/utils.h`).
const WEBP_ALIGN_CST: u64 = 31;
/// `WEBP_MAX_DIMENSION` (`src/webp/encode.h`).
const WEBP_MAX_DIMENSION: c_int = 16383;

/// Memory-safety guard for the RGBA/NRGBA pointer handed to C (see
/// `encode`). It follows the order of the checks in `encodeNRGBA` ->
/// `WebPPictureImportRGBA` -> `Import` (`src/enc/picture_csp_enc.c`), so that
/// every input on which libwebp fails *before* reading the pixels still goes
/// to C (and gives Go's `"failed to encode"`); only inputs on which libwebp
/// would read outside `pix` (an out-of-bounds read in Go) are refused.
fn check_pix_rgba(
    v: &PixView<'_>,
    width: c_int,
    height: c_int,
    stride: c_int,
) -> Result<(), Error> {
    if v.pix.is_empty() {
        // Go: `&v.Pix[0]` panics with "index out of range [0] with length 0".
        return Err(Error::EmptyPix);
    }
    // Import: `if (abs(rgb_stride) < 4 * width) return 0;` in C `int`
    // arithmetic (clang/arm64: abs(INT_MIN) == INT_MIN, 4 * width wraps).
    if stride.wrapping_abs() < 4i32.wrapping_mul(width) {
        return Ok(());
    }
    // WebPPictureAlloc -> WebPPictureAllocARGB -> WebPValidatePicture.
    if width <= 0 || height <= 0 {
        return Ok(());
    }
    // WebPSafeMalloc(width * height + WEBP_ALIGN_CST, sizeof(uint32_t)) fails
    // (CheckSizeArgumentsOverflow) before any pixel is read.
    let nmemb = width as u64 * height as u64 + WEBP_ALIGN_CST;
    if 4 > WEBP_MAX_ALLOCABLE_MEMORY / nmemb {
        return Ok(());
    }
    // Import then reads `width * 4` bytes at `rgba + y * stride` for every
    // row y < height (WebPEncode's dimension check comes after the import).
    rows_fit(v.pix.len(), width as i64 * 4, height, stride)
}

/// Memory-safety guard for the Gray pointer handed to C, following
/// `encodeGray` -> `WebPEncode` (`src/enc/webp_enc.c`): libwebp validates the
/// picture (positive dimensions, at most `WEBP_MAX_DIMENSION`) before it
/// reads the luma plane.
fn check_pix_gray(
    v: &PixView<'_>,
    width: c_int,
    height: c_int,
    stride: c_int,
) -> Result<(), Error> {
    if v.pix.is_empty() {
        // Go: `&v.Pix[0]` panics with "index out of range [0] with length 0".
        return Err(Error::EmptyPix);
    }
    if width <= 0 || height <= 0 || width > WEBP_MAX_DIMENSION || height > WEBP_MAX_DIMENSION {
        // The chroma malloc or WebPValidatePicture / the dimension check in
        // WebPEncode fails first.
        return Ok(());
    }
    // The encoder reads `width` bytes at `y + row * y_stride` for every row.
    rows_fit(v.pix.len(), width as i64, height, stride)
}

/// Whether the rows `[r * stride, r * stride + row_bytes)` for `r < height`
/// (height > 0) all lie inside a buffer of `len` bytes. A negative stride is
/// fine for a single row (C only reads row 0).
fn rows_fit(len: usize, row_bytes: i64, height: c_int, stride: c_int) -> Result<(), Error> {
    let last = (height as i128 - 1) * stride as i128;
    let lo = last.min(0);
    let hi = last.max(0) + row_bytes as i128;
    if lo < 0 || hi > len as i128 {
        return Err(Error::PixOutOfRange);
    }
    Ok(())
}

// Go: a__encoder.go:ConvertToNRGBA is not ported (needs image/draw); see
// `encode` and PORTING.md.

// Go: a__encoder.go:encodingOptionsToCConfig
pub fn encoding_options_to_c_config(o: EncodingOptions) -> Result<ffi::WebPConfig, Error> {
    // cfg := &C.WebPConfig{} (zeroed)
    let mut cfg = ffi::WebPConfig::default();
    // quality := C.float(o.Quality)
    let quality = o.quality as c_float;

    // C.WebPPreset(o.EncodingPreset): Go int -> uint32 (truncating).
    if unsafe { ffi::WebPConfigPreset(&mut cfg, o.encoding_preset.0 as ffi::WebPPreset, quality) }
        == 0
    {
        return Err(Error::InitConfig);
    }

    if quality == 0.0 {
        // Activate the lossless compression mode with the desired efficiency level
        // between 0 (fastest, lowest compression) and 9 (slower, best compression).
        // A good default level is '6', providing a fair tradeoff between compression
        // speed and final compressed size.
        if unsafe { ffi::WebPConfigLosslessPreset(&mut cfg, 6) } == 0 {
            return Err(Error::LosslessPreset);
        }
    }

    cfg.use_sharp_yuv = bool_to_c_int(o.use_sharp_yuv);

    if unsafe { ffi::WebPValidateConfig(&cfg) } == 0 {
        return Err(Error::ValidateConfig);
    }

    Ok(cfg)
}

// Go: a__encoder.go:boolToCInt
fn bool_to_c_int(b: bool) -> c_int {
    let mut result = 0;

    if b {
        result = 1;
    }

    result
}

/// libwebp is compiled without `WEBP_USE_THREAD`, so its lazy DSP
/// initialisers (`WEBP_DSP_INIT_FUNC`) are unsynchronised: the first
/// concurrent encodes race on the function-pointer tables (Go/Hugo has the
/// same race). Run one tiny encode through every path used by gowebp under a
/// `Once` so that all tables are settled before callers run in parallel. The
/// initialisers are idempotent, so this cannot change any output.
fn ensure_dsp_initialized() {
    static INIT: Once = Once::new();
    INIT.call_once(|| {
        let mut alpha = vec![0u8; 16 * 16 * 4];
        let mut opaque = vec![0u8; 16 * 16 * 4];
        for (i, (a, o)) in alpha
            .as_chunks_mut::<4>()
            .0
            .iter_mut()
            .zip(opaque.as_chunks_mut::<4>().0.iter_mut())
            .enumerate()
        {
            a[0] = (i * 7) as u8;
            a[1] = (i * 13) as u8;
            a[2] = (i * 29) as u8;
            a[3] = if i % 3 == 0 { 0 } else { (i * 17) as u8 };
            o[..3].copy_from_slice(&a[..3]);
            o[3] = 0xff;
        }
        let gray = vec![0x80u8; 16 * 16];
        let opts = [(75, 2, true), (75, 0, false), (0, 0, false), (0, 2, true)];
        for &(quality, preset, sharp) in &opts {
            let o = EncodingOptions {
                quality,
                encoding_preset: crate::webpoptions::EncodingPreset(preset),
                use_sharp_yuv: sharp,
            };
            let Ok(mut cfg) = encoding_options_to_c_config(o) else {
                continue;
            };
            for pix in [&alpha[..], &opaque[..]] {
                let mut size = 0usize;
                let out = unsafe {
                    ffi::gowebp_encodeNRGBA(&mut cfg, pix.as_ptr(), 16, 16, 64, &mut size)
                };
                if !out.is_null() {
                    unsafe { ffi::gowebp_free(out.cast()) };
                }
            }
            let mut y = gray.clone();
            let mut size = 0usize;
            let out =
                unsafe { ffi::gowebp_encodeGray(&mut cfg, y.as_mut_ptr(), 16, 16, 16, &mut size) };
            if !out.is_null() {
                unsafe { ffi::gowebp_free(out.cast()) };
            }
        }
    });
}
