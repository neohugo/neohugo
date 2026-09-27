//! Ports of gift v1.2.1's own `*_test.go` files (one module per Go file;
//! `utils_test.go` is in `src/utils.rs`, `TestGolden` in `tests/go_golden.rs`).
//!
//! The tables were converted mechanically from the Go sources (struct fields
//! and values in the same order), the loops by hand. Every exact-pixel
//! expectation holds under both Go float semantics: the Go tests pass with
//! go1.27.1 on linux/amd64 (no FMA) and on linux/arm64 under qemu-aarch64
//! (the golden build's fusion), so none of these tables depends on an FMA
//! site. The only FMA-dependent Go test is `TestGolden` (see PORTING.md).
//!
//! Differences from the Go tests (none weakens a check):
//! * A Go pixel getter/setter keeps a pointer to an image the test then
//!   mutates; Rust borrows forbid that, so the port creates the getter or
//!   setter after each mutation (they hold no state besides the converted
//!   palette, which is the same each time).
//! * `TestSubImage` checks the aliasing through `get_sub_image`'s callback.
//! * `TestParallelize` cannot vary `GOMAXPROCS` (utils.rs).

mod colors_test;
mod convolution_test;
mod effects_test;
mod gift_test;
mod pixels_test;
mod rank_test;
mod resize_test;
mod transform_test;

use go_image::{Gray, NRGBA, Rectangle};

/// Go: utils_test.go:checkBoundsAndPix
pub(super) fn check_bounds_and_pix(b1: Rectangle, b2: Rectangle, pix1: &[u8], pix2: &[u8]) -> bool {
    if !b1.eq(b2) {
        return false;
    }
    if pix1 != pix2 {
        return false;
    }
    true
}

/// `src := image.NewGray(r); src.Pix = pix`
pub(super) fn gray(r: Rectangle, pix: &[u8]) -> Gray {
    let mut m = Gray::new(r);
    m.pix = pix.to_vec();
    m
}

/// `src := image.NewNRGBA(r); src.Pix = pix`
pub(super) fn nrgba(r: Rectangle, pix: &[u8]) -> NRGBA {
    let mut m = NRGBA::new(r);
    m.pix = pix.to_vec();
    m
}
