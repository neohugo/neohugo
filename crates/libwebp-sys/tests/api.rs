use libwebp_sys::{
    EncodingOptions, EncodingPreset, Error, Image, PixView, Point, Rectangle, encode_to_vec, ffi,
};

#[test]
fn config_layout_matches_c() {
    unsafe {
        assert_eq!(
            std::mem::size_of::<ffi::WebPConfig>(),
            ffi::gowebp_sizeof_config()
        );
        assert_eq!(
            std::mem::offset_of!(ffi::WebPConfig, use_sharp_yuv),
            ffi::gowebp_offsetof_config_use_sharp_yuv()
        );
        assert_eq!(
            std::mem::offset_of!(ffi::WebPConfig, qmax),
            ffi::gowebp_offsetof_config_qmax()
        );
    }
}

#[test]
fn encoder_version_is_1_3_2() {
    assert_eq!(libwebp_sys::encoder_version(), 0x010302);
}

#[test]
fn photo_preset_config_matches_spec() {
    // specs/images.md §6.4: quality 75, method 4, sns 80, filter_strength 30,
    // sharpness 3, filter_type 1, segments 4, pass 1, preprocessing 2.
    let cfg = libwebp_sys::encoder::encoding_options_to_c_config(EncodingOptions {
        quality: 75,
        encoding_preset: EncodingPreset::PHOTO,
        use_sharp_yuv: true,
    })
    .unwrap();
    assert_eq!(cfg.quality, 75.0);
    assert_eq!(cfg.method, 4);
    assert_eq!(cfg.sns_strength, 80);
    assert_eq!(cfg.filter_strength, 30);
    assert_eq!(cfg.filter_sharpness, 3);
    assert_eq!(cfg.filter_type, 1);
    assert_eq!(cfg.segments, 4);
    assert_eq!(cfg.pass, 1);
    assert_eq!(cfg.preprocessing, 2);
    assert_eq!(cfg.use_sharp_yuv, 1);
    assert_eq!(cfg.lossless, 0);
    assert_eq!(cfg.thread_level, 0);
}

#[test]
fn short_buffer_is_rejected_not_read() {
    let pix = [0u8; 16];
    let img = Image::Nrgba(PixView {
        pix: &pix,
        stride: 40,
        rect: Rectangle {
            min: Point::default(),
            max: Point { x: 10, y: 10 },
        },
    });
    let o = EncodingOptions {
        quality: 75,
        encoding_preset: EncodingPreset::PHOTO,
        use_sharp_yuv: true,
    };
    assert!(matches!(encode_to_vec(&img, o), Err(Error::PixOutOfRange)));
    let img = Image::Gray(PixView {
        pix: &pix,
        stride: 10,
        rect: Rectangle {
            min: Point::default(),
            max: Point { x: 10, y: 10 },
        },
    });
    assert!(matches!(encode_to_vec(&img, o), Err(Error::PixOutOfRange)));
}

#[test]
fn writer_errors_are_returned() {
    struct Fail;
    impl std::io::Write for Fail {
        fn write(&mut self, _: &[u8]) -> std::io::Result<usize> {
            Err(std::io::Error::other("boom"))
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let pix = [200u8; 4 * 4 * 4];
    let img = Image::Nrgba(PixView {
        pix: &pix,
        stride: 16,
        rect: Rectangle::new(0, 0, 4, 4),
    });
    let o = EncodingOptions {
        quality: 75,
        encoding_preset: EncodingPreset::PHOTO,
        use_sharp_yuv: true,
    };
    let err = libwebp_sys::encode(&mut Fail, &img, o).unwrap_err();
    assert_eq!(err.to_string(), "boom");
}

/// Red-team regression. gowebp imports (reads) an RGBA/NRGBA picture before
/// `WebPEncode` rejects dimensions above 16383, so for such a picture with a
/// short `Pix` (in Go: a sub-image near the end of its parent) Go returns
/// "failed to encode" whatever the out-of-bounds bytes are. The Rust port
/// returned `PixOutOfRange`.
#[test]
fn oversize_picture_with_short_pix_fails_like_go() {
    let o = EncodingOptions {
        quality: 75,
        encoding_preset: EncodingPreset::PHOTO,
        use_sharp_yuv: true,
    };
    let pix = vec![7u8; 79999];
    for img in [
        Image::Nrgba(PixView {
            pix: &pix,
            stride: 80000,
            rect: Rectangle::new(0, 0, 20000, 1),
        }),
        Image::Rgba(PixView {
            pix: &pix[..9],
            stride: 8,
            rect: Rectangle::new(0, 0, 2, 16384),
        }),
    ] {
        let err = encode_to_vec(&img, o).unwrap_err();
        assert!(matches!(err, Error::Encode), "{err:?}");
        assert_eq!(err.to_string(), "failed to encode");
    }
    // Within WEBP_MAX_DIMENSION a short Pix is still refused (Go would
    // encode bytes read out of bounds).
    let img = Image::Nrgba(PixView {
        pix: &pix[..16383 * 4 - 1],
        stride: 16383 * 4,
        rect: Rectangle::new(0, 0, 16383, 1),
    });
    assert!(matches!(encode_to_vec(&img, o), Err(Error::PixOutOfRange)));
}
