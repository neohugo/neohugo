//! Raw FFI declarations for the parts of libwebp 1.3.2 (`src/webp/encode.h`)
//! and the gowebp cgo preamble (`csrc/gowebp_encoder.c`) that the wrapper
//! uses.
//!
//! Only what gowebp touches is declared. `WebPConfig` is mirrored with
//! `#[repr(C)]`; its layout is checked against the C compiler's view by the
//! `config_layout_matches_c` test.

use std::os::raw::{c_float, c_int, c_uint, c_void};

/// `WEBP_ENCODER_ABI_VERSION` from `src/webp/encode.h` (libwebp 1.3.2).
pub const WEBP_ENCODER_ABI_VERSION: c_int = 0x020f;

/// C `WebPPreset` enum. cgo maps an enum without negative enumerators to
/// `uint32`, so Go's `C.WebPPreset(o.EncodingPreset)` is a truncating
/// int -> uint32 conversion.
pub type WebPPreset = c_uint;

/// C `WebPImageHint` enum.
pub type WebPImageHint = c_uint;

/// Mirror of `struct WebPConfig` (`src/webp/encode.h:92-148`).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
#[allow(non_snake_case)]
pub struct WebPConfig {
    pub lossless: c_int,
    pub quality: c_float,
    pub method: c_int,
    pub image_hint: WebPImageHint,
    pub target_size: c_int,
    pub target_PSNR: c_float,
    pub segments: c_int,
    pub sns_strength: c_int,
    pub filter_strength: c_int,
    pub filter_sharpness: c_int,
    pub filter_type: c_int,
    pub autofilter: c_int,
    pub alpha_compression: c_int,
    pub alpha_filtering: c_int,
    pub alpha_quality: c_int,
    pub pass: c_int,
    pub show_compressed: c_int,
    pub preprocessing: c_int,
    pub partitions: c_int,
    pub partition_limit: c_int,
    pub emulate_jpeg_size: c_int,
    pub thread_level: c_int,
    pub low_memory: c_int,
    pub near_lossless: c_int,
    pub exact: c_int,
    pub use_delta_palette: c_int,
    pub use_sharp_yuv: c_int,
    pub qmin: c_int,
    pub qmax: c_int,
}

unsafe extern "C" {
    /// `WebPConfigInitInternal` (`src/enc/config_enc.c`). The header's
    /// `static inline WebPConfigPreset(cfg, preset, q)` is
    /// `WebPConfigInitInternal(cfg, preset, q, WEBP_ENCODER_ABI_VERSION)`.
    pub fn WebPConfigInitInternal(
        config: *mut WebPConfig,
        preset: WebPPreset,
        quality: c_float,
        version: c_int,
    ) -> c_int;

    pub fn WebPConfigLosslessPreset(config: *mut WebPConfig, level: c_int) -> c_int;

    pub fn WebPValidateConfig(config: *const WebPConfig) -> c_int;

    pub fn WebPGetEncoderVersion() -> c_int;

    /// `encodeNRGBA` from the a__encoder.go preamble.
    pub fn gowebp_encodeNRGBA(
        config: *mut WebPConfig,
        rgba: *const u8,
        width: c_int,
        height: c_int,
        stride: c_int,
        output_size: *mut usize,
    ) -> *mut u8;

    /// `encodeGray` from the a__encoder.go preamble.
    pub fn gowebp_encodeGray(
        config: *mut WebPConfig,
        y: *mut u8,
        width: c_int,
        height: c_int,
        stride: c_int,
        output_size: *mut usize,
    ) -> *mut u8;

    /// `free(3)`; Go uses `C.free` on the encoder output.
    pub fn gowebp_free(p: *mut c_void);

    pub fn gowebp_sizeof_config() -> usize;
    pub fn gowebp_offsetof_config_use_sharp_yuv() -> usize;
    pub fn gowebp_offsetof_config_qmax() -> usize;
}

/// Go: `C.WebPConfigPreset(cfg, preset, quality)` (the static inline helper
/// from `encode.h`).
///
/// # Safety
/// `config` must point to a valid, writable `WebPConfig`.
#[allow(non_snake_case)]
pub unsafe fn WebPConfigPreset(
    config: *mut WebPConfig,
    preset: WebPPreset,
    quality: c_float,
) -> c_int {
    unsafe { WebPConfigInitInternal(config, preset, quality, WEBP_ENCODER_ABI_VERSION) }
}
