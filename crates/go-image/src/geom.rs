//! Port of Go 1.27.1 `image/geom.go`.
//!
//! Go `int` arithmetic wraps on overflow; the Rust port uses `wrapping_*`
//! operations so that extreme coordinates behave exactly like Go instead of
//! panicking in debug builds.

use std::any::Any;
use std::fmt;

use crate::color::{self, Color, Model};
use crate::image::{Image, RGBA64Image};

/// A Point is an X, Y coordinate pair. The axes increase right and down.
///
/// Go: image/geom.go:Point
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Point {
    pub x: i64,
    pub y: i64,
}

impl fmt::Display for Point {
    // Go: image/geom.go:Point.String
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({},{})", self.x, self.y)
    }
}

impl Point {
    // Go: image/geom.go:Point.Add
    pub fn add(self, q: Point) -> Point {
        Point {
            x: self.x.wrapping_add(q.x),
            y: self.y.wrapping_add(q.y),
        }
    }

    // Go: image/geom.go:Point.Sub
    pub fn sub(self, q: Point) -> Point {
        Point {
            x: self.x.wrapping_sub(q.x),
            y: self.y.wrapping_sub(q.y),
        }
    }

    // Go: image/geom.go:Point.Mul
    pub fn mul(self, k: i64) -> Point {
        Point {
            x: self.x.wrapping_mul(k),
            y: self.y.wrapping_mul(k),
        }
    }

    /// Go: image/geom.go:Point.Div. Division by zero panics, as in Go.
    pub fn div(self, k: i64) -> Point {
        Point {
            x: self.x.wrapping_div(k),
            y: self.y.wrapping_div(k),
        }
    }

    /// In reports whether p is in r.
    ///
    /// Go: image/geom.go:Point.In
    pub fn in_(self, r: Rectangle) -> bool {
        r.min.x <= self.x && self.x < r.max.x && r.min.y <= self.y && self.y < r.max.y
    }

    /// Mod returns the point q in r such that p.X-q.X is a multiple of r's
    /// width and p.Y-q.Y is a multiple of r's height.
    ///
    /// Go: image/geom.go:Point.Mod
    pub fn mod_(self, r: Rectangle) -> Point {
        let (w, h) = (r.dx(), r.dy());
        let mut p = self.sub(r.min);
        p.x = p.x.wrapping_rem(w);
        if p.x < 0 {
            p.x = p.x.wrapping_add(w);
        }
        p.y = p.y.wrapping_rem(h);
        if p.y < 0 {
            p.y = p.y.wrapping_add(h);
        }
        p.add(r.min)
    }

    // Go: image/geom.go:Point.Eq
    pub fn eq(self, q: Point) -> bool {
        self == q
    }
}

/// ZP is the zero [`Point`].
///
/// Go: image/geom.go:ZP
pub const ZP: Point = Point { x: 0, y: 0 };

/// Pt is shorthand for Point{X, Y}.
///
/// Go: image/geom.go:Pt
pub const fn pt(x: i64, y: i64) -> Point {
    Point { x, y }
}

/// A Rectangle contains the points with Min.X <= X < Max.X, Min.Y <= Y < Max.Y.
///
/// Go: image/geom.go:Rectangle
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Rectangle {
    pub min: Point,
    pub max: Point,
}

impl fmt::Display for Rectangle {
    // Go: image/geom.go:Rectangle.String
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}-{}", self.min, self.max)
    }
}

impl Rectangle {
    // Go: image/geom.go:Rectangle.Dx
    pub fn dx(&self) -> i64 {
        self.max.x.wrapping_sub(self.min.x)
    }

    // Go: image/geom.go:Rectangle.Dy
    pub fn dy(&self) -> i64 {
        self.max.y.wrapping_sub(self.min.y)
    }

    // Go: image/geom.go:Rectangle.Size
    pub fn size(&self) -> Point {
        Point {
            x: self.max.x.wrapping_sub(self.min.x),
            y: self.max.y.wrapping_sub(self.min.y),
        }
    }

    // Go: image/geom.go:Rectangle.Add
    pub fn add(&self, p: Point) -> Rectangle {
        Rectangle {
            min: Point {
                x: self.min.x.wrapping_add(p.x),
                y: self.min.y.wrapping_add(p.y),
            },
            max: Point {
                x: self.max.x.wrapping_add(p.x),
                y: self.max.y.wrapping_add(p.y),
            },
        }
    }

    // Go: image/geom.go:Rectangle.Sub
    pub fn sub(&self, p: Point) -> Rectangle {
        Rectangle {
            min: Point {
                x: self.min.x.wrapping_sub(p.x),
                y: self.min.y.wrapping_sub(p.y),
            },
            max: Point {
                x: self.max.x.wrapping_sub(p.x),
                y: self.max.y.wrapping_sub(p.y),
            },
        }
    }

    // Go: image/geom.go:Rectangle.Inset
    pub fn inset(&self, n: i64) -> Rectangle {
        let mut r = *self;
        if r.dx() < n.wrapping_mul(2) {
            r.min.x = r.min.x.wrapping_add(r.max.x).wrapping_div(2);
            r.max.x = r.min.x;
        } else {
            r.min.x = r.min.x.wrapping_add(n);
            r.max.x = r.max.x.wrapping_sub(n);
        }
        if r.dy() < n.wrapping_mul(2) {
            r.min.y = r.min.y.wrapping_add(r.max.y).wrapping_div(2);
            r.max.y = r.min.y;
        } else {
            r.min.y = r.min.y.wrapping_add(n);
            r.max.y = r.max.y.wrapping_sub(n);
        }
        r
    }

    /// Intersect returns the largest rectangle contained by both r and s. If
    /// the two rectangles do not overlap then the zero rectangle will be
    /// returned.
    ///
    /// Go: image/geom.go:Rectangle.Intersect
    pub fn intersect(&self, s: Rectangle) -> Rectangle {
        let mut r = *self;
        if r.min.x < s.min.x {
            r.min.x = s.min.x;
        }
        if r.min.y < s.min.y {
            r.min.y = s.min.y;
        }
        if r.max.x > s.max.x {
            r.max.x = s.max.x;
        }
        if r.max.y > s.max.y {
            r.max.y = s.max.y;
        }
        if r.empty() {
            return Rectangle::default();
        }
        r
    }

    // Go: image/geom.go:Rectangle.Union
    pub fn union(&self, s: Rectangle) -> Rectangle {
        let mut r = *self;
        if r.empty() {
            return s;
        }
        if s.empty() {
            return r;
        }
        if r.min.x > s.min.x {
            r.min.x = s.min.x;
        }
        if r.min.y > s.min.y {
            r.min.y = s.min.y;
        }
        if r.max.x < s.max.x {
            r.max.x = s.max.x;
        }
        if r.max.y < s.max.y {
            r.max.y = s.max.y;
        }
        r
    }

    // Go: image/geom.go:Rectangle.Empty
    pub fn empty(&self) -> bool {
        self.min.x >= self.max.x || self.min.y >= self.max.y
    }

    /// Eq reports whether r and s contain the same set of points. All empty
    /// rectangles are considered equal.
    ///
    /// Go: image/geom.go:Rectangle.Eq
    pub fn eq(&self, s: Rectangle) -> bool {
        *self == s || self.empty() && s.empty()
    }

    // Go: image/geom.go:Rectangle.Overlaps
    pub fn overlaps(&self, s: Rectangle) -> bool {
        !self.empty()
            && !s.empty()
            && self.min.x < s.max.x
            && s.min.x < self.max.x
            && self.min.y < s.max.y
            && s.min.y < self.max.y
    }

    // Go: image/geom.go:Rectangle.In
    pub fn in_(&self, s: Rectangle) -> bool {
        if self.empty() {
            return true;
        }
        s.min.x <= self.min.x
            && self.max.x <= s.max.x
            && s.min.y <= self.min.y
            && self.max.y <= s.max.y
    }

    // Go: image/geom.go:Rectangle.Canon
    pub fn canon(&self) -> Rectangle {
        let mut r = *self;
        if r.max.x < r.min.x {
            std::mem::swap(&mut r.min.x, &mut r.max.x);
        }
        if r.max.y < r.min.y {
            std::mem::swap(&mut r.min.y, &mut r.max.y);
        }
        r
    }

    // Go: image/geom.go:Rectangle.At
    pub fn at(&self, x: i64, y: i64) -> Color {
        if (Point { x, y }).in_(*self) {
            return Color::Alpha16(color::OPAQUE);
        }
        Color::Alpha16(color::TRANSPARENT)
    }

    // Go: image/geom.go:Rectangle.RGBA64At
    pub fn rgba64_at(&self, x: i64, y: i64) -> color::RGBA64 {
        if (Point { x, y }).in_(*self) {
            return color::RGBA64 {
                r: 0xffff,
                g: 0xffff,
                b: 0xffff,
                a: 0xffff,
            };
        }
        color::RGBA64::default()
    }

    // Go: image/geom.go:Rectangle.Bounds
    pub fn bounds(&self) -> Rectangle {
        *self
    }

    // Go: image/geom.go:Rectangle.ColorModel
    pub fn color_model(&self) -> Model {
        Model::Alpha16
    }
}

impl Image for Rectangle {
    fn color_model(&self) -> Model {
        Rectangle::color_model(self)
    }
    fn bounds(&self) -> Rectangle {
        *self
    }
    fn at(&self, x: i64, y: i64) -> Color {
        Rectangle::at(self, x, y)
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn into_any(self: Box<Self>) -> Box<dyn Any> {
        self
    }
    fn as_rgba64_image(&self) -> Option<&dyn RGBA64Image> {
        Some(self)
    }
}

impl RGBA64Image for Rectangle {
    fn rgba64_at(&self, x: i64, y: i64) -> color::RGBA64 {
        Rectangle::rgba64_at(self, x, y)
    }
}

/// ZR is the zero [`Rectangle`].
///
/// Go: image/geom.go:ZR
pub const ZR: Rectangle = Rectangle { min: ZP, max: ZP };

/// Rect is shorthand for Rectangle{Pt(x0, y0), Pt(x1, y1)}. The returned
/// rectangle has minimum and maximum coordinates swapped if necessary so that
/// it is well-formed.
///
/// Go: image/geom.go:Rect
pub fn rect(mut x0: i64, mut y0: i64, mut x1: i64, mut y1: i64) -> Rectangle {
    if x0 > x1 {
        std::mem::swap(&mut x0, &mut x1);
    }
    if y0 > y1 {
        std::mem::swap(&mut y0, &mut y1);
    }
    Rectangle {
        min: Point { x: x0, y: y0 },
        max: Point { x: x1, y: y1 },
    }
}

/// mul3NonNeg returns (x * y * z), unless at least one argument is negative or
/// if the computation overflows the int type, in which case it returns -1.
///
/// Go: image/geom.go:mul3NonNeg
pub(crate) fn mul3_non_neg(x: i64, y: i64, z: i64) -> i64 {
    if (x < 0) || (y < 0) || (z < 0) {
        return -1;
    }
    let p = (x as u64 as u128) * (y as u64 as u128);
    let (hi, lo) = ((p >> 64) as u64, p as u64);
    if hi != 0 {
        return -1;
    }
    let p = (lo as u128) * (z as u64 as u128);
    let (hi, lo) = ((p >> 64) as u64, p as u64);
    if hi != 0 {
        return -1;
    }
    let a = lo as i64;
    if (a < 0) || (a as u64 != lo) {
        return -1;
    }
    a
}

/// add2NonNeg returns (x + y), unless at least one argument is negative or if
/// the computation overflows the int type, in which case it returns -1.
///
/// Go: image/geom.go:add2NonNeg
pub(crate) fn add2_non_neg(x: i64, y: i64) -> i64 {
    if (x < 0) || (y < 0) {
        return -1;
    }
    let a = x.wrapping_add(y);
    if a < 0 {
        return -1;
    }
    a
}
