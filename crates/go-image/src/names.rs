//! Port of Go 1.27.1 `image/names.go`.

use crate::color::{self, Color, Model};
use crate::geom::{Point, Rectangle};
use crate::image::impl_image_traits;

/// Uniform is an infinite-sized [`crate::Image`] of uniform color.
/// It implements the color.Color, color.Model, and Image interfaces
/// (the colour via [`Uniform::rgba`], the model via [`Model::Uniform`]).
///
/// Go: image/names.go:Uniform
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Uniform {
    pub c: Color,
}

impl Uniform {
    /// NewUniform returns a new [`Uniform`] image of the given color.
    ///
    /// Go: image/names.go:NewUniform
    pub fn new(c: Color) -> Uniform {
        Uniform { c }
    }

    // Go: image/names.go:Uniform.RGBA
    pub fn rgba(&self) -> (u32, u32, u32, u32) {
        self.c.rgba()
    }

    // Go: image/names.go:Uniform.ColorModel
    pub fn color_model(&self) -> Model {
        Model::Uniform(self.c)
    }

    // Go: image/names.go:Uniform.Convert
    pub fn convert(&self, _c: Color) -> Color {
        self.c
    }

    // Go: image/names.go:Uniform.Bounds
    pub fn bounds(&self) -> Rectangle {
        Rectangle {
            min: Point {
                x: -1_000_000_000,
                y: -1_000_000_000,
            },
            max: Point {
                x: 1_000_000_000,
                y: 1_000_000_000,
            },
        }
    }

    // Go: image/names.go:Uniform.At
    pub fn at(&self, _x: i64, _y: i64) -> Color {
        self.c
    }

    // Go: image/names.go:Uniform.RGBA64At
    pub fn rgba64_at(&self, _x: i64, _y: i64) -> color::RGBA64 {
        let (r, g, b, a) = self.c.rgba();
        color::RGBA64 {
            r: r as u16,
            g: g as u16,
            b: b as u16,
            a: a as u16,
        }
    }

    /// Opaque scans the entire image and reports whether it is fully opaque.
    ///
    /// Go: image/names.go:Uniform.Opaque
    pub fn opaque(&self) -> bool {
        let (_, _, _, a) = self.c.rgba();
        a == 0xffff
    }
}
impl_image_traits!(Uniform);

// Go: image/names.go:Black, White, Transparent, Opaque
/// Black is an opaque black uniform image.
pub const BLACK: Uniform = Uniform {
    c: Color::Gray16(color::BLACK),
};
/// White is an opaque white uniform image.
pub const WHITE: Uniform = Uniform {
    c: Color::Gray16(color::WHITE),
};
/// Transparent is a fully transparent uniform image.
pub const TRANSPARENT: Uniform = Uniform {
    c: Color::Alpha16(color::TRANSPARENT),
};
/// Opaque is a fully opaque uniform image.
pub const OPAQUE: Uniform = Uniform {
    c: Color::Alpha16(color::OPAQUE),
};
