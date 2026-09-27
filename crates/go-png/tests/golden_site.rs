//! The 11 PNGs of the golden seeksnack build (go1.27.1, darwin/arm64,
//! `png.Encoder{CompressionLevel: png.DefaultCompression}`), decoded and
//! re-encoded: the bytes must be identical. The expected bytes are the
//! golden files themselves, so this does not depend on the platform the
//! oracle fixtures were generated on. The images are 128x128 to 600x480;
//! the larger ones span several flate blocks and go through the float
//! `EstimatedBits` Huffman-table reuse decision (which, for these images,
//! amd64 Go happens to make the same way as the arm64 golden binary).

mod common;

use common::*;
use go_png::{DEFAULT_COMPRESSION, Encoder};

#[test]
fn golden_pngs_reencode_identically() {
    let names = [
        "berli-jucker-foods-ltd.berli-jucker-plc_hu_9745137e13631ab1.png",
        "berli-jucker-foods-ltd.berli-jucker-plc_hu_efcb259769ef42b1.png",
        "classic-foods-inc_hu_81b2e2c96349026f.png",
        "classic-foods-inc_hu_c0871b21a075dd2d.png",
        "cpram_hu_d59b22a35402c9d8.png",
        "cpram_hu_e33fe8783af6a9de.png",
        "frito-lay_hu_8f37120362e997d1.png",
        "frito-lay_hu_a9d9121cc2f28a58.png",
        "ja-yubari_hu_34432aa8e3d37ac4.png",
        "ja-yubari_hu_facc1c70de9e27f9.png",
        "mstile-70x70_hu_80634bc5fec9785.png",
    ];
    let mut idats = 0;
    for name in names {
        let data = read_fixture(&format!("golden/{name}"));
        go_png::register();
        let (m, format) =
            go_image::decode(&mut &data[..]).unwrap_or_else(|e| panic!("{name}: {e}"));
        assert_eq!(format, "png");
        let mut out = Vec::new();
        Encoder {
            compression_level: DEFAULT_COMPRESSION,
            ..Default::default()
        }
        .encode(&mut out, &*m)
        .unwrap();
        assert!(
            out == data,
            "{name}: re-encoding differs from the golden file"
        );
        idats += data.windows(4).filter(|w| w == b"IDAT").count();
    }
    // Several files are split into 32 KiB IDAT chunks by the bufio.Writer.
    assert!(idats > names.len());
}
