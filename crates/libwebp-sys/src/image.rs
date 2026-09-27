//! Minimal stand-in for the Go image types gowebp switches on
//! (`*image.RGBA`, `*image.NRGBA`, `*image.Gray`).
//!
//! Only the fields gowebp reads are modelled: `Pix`, `Stride` and `Rect`.

/// Go `image.Point` (`int` coordinates).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Point {
    pub x: i64,
    pub y: i64,
}

/// Go `image.Rectangle`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Rectangle {
    pub min: Point,
    pub max: Point,
}

impl Rectangle {
    /// Go `image.Rect(x0, y0, x1, y1)` (canonicalised like Go).
    pub fn new(x0: i64, y0: i64, x1: i64, y1: i64) -> Rectangle {
        let (x0, x1) = if x0 > x1 { (x1, x0) } else { (x0, x1) };
        let (y0, y1) = if y0 > y1 { (y1, y0) } else { (y0, y1) };
        Rectangle {
            min: Point { x: x0, y: y0 },
            max: Point { x: x1, y: y1 },
        }
    }
}

/// The `Pix`/`Stride`/`Rect` triple of a Go image. `pix` must start at the
/// pixel at `rect.min` (as Go's `Pix` does, also for sub-images).
#[derive(Clone, Copy, Debug)]
pub struct PixView<'a> {
    pub pix: &'a [u8],
    pub stride: i64,
    pub rect: Rectangle,
}

/// The image kinds gowebp encodes directly.
#[derive(Clone, Copy, Debug)]
pub enum Image<'a> {
    /// `*image.RGBA` (premultiplied; gowebp imports it as if it were not).
    Rgba(PixView<'a>),
    /// `*image.NRGBA`.
    Nrgba(PixView<'a>),
    /// `*image.Gray`.
    Gray(PixView<'a>),
}

impl<'a> Image<'a> {
    pub fn view(&self) -> &PixView<'a> {
        match self {
            Image::Rgba(v) | Image::Nrgba(v) | Image::Gray(v) => v,
        }
    }

    /// Go `src.Bounds()`.
    pub fn bounds(&self) -> Rectangle {
        self.view().rect
    }
}
